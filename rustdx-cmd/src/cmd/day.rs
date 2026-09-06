use argh::FromArgs;
use eyre::Result;
use rustdx_cli::fetch_code;
use rustdx_cli::fetch_code::StockList;

/// 例子：`rustdx day /vdb/tmp/tdx/sh/ /vdb/tmp/tdx/sz/ -l official -g ../assets/gbbq`。
///
/// **无参数模式**：直接运行 `rustdx day`（不带任何目录）时，自动从通达信官网
/// 下载日线完整包 <https://data.tdx.com.cn/vipdoc/hsjday.zip>，解压后解析；
/// 下载失败会交互询问备选下载地址（URL）或本地 zip 文件路径。
#[derive(FromArgs, PartialEq, Debug, Clone)]
#[argh(subcommand, name = "day")]
pub struct DayCmd {
    /// 可选。指定一个或多个含 *.day 文件的文件夹路径；也可传下载地址（URL）
    /// 或本地 zip 文件路径（自动下载/解压后解析）。不指定时自动下载官网完整包。
    #[argh(positional)]
    pub path: Vec<std::path::PathBuf>,

    /// 可选。解析后的输出方式。`rustdx day -h o` 查看详细使用说明。
    /// 默认值为 stocks.csv，表示输出 csv 格式，且保存到当前目录的 stocks.csv 文件中。
    #[argh(option, short = 'o', default = "String::from(\"stocks.csv\")")]
    pub output: String,

    /// 可选。指定时，表示保存 csv 文件。只针对非 csv output 有效。
    #[argh(switch, short = 'k', long = "keep-csv")]
    pub keep_csv: bool,

    /// 可选。指定时，表示保存 factor.csv 文件。
    #[argh(switch)]
    pub keep_factor: bool,

    /// 可选。指定复权数据（csv 文件路径）。如果没有指定这个参数，则不会计算复权。
    #[argh(option, short = 'g')]
    pub gbbq: Option<std::path::PathBuf>,

    /// 可选。提供前一日复权数据（csv 文件路径）。在指定了复权时，除非从上市日开始解析，
    /// 否则必须指定前一日复权数据。因为前复权数据必须是日期连贯的、基于上市日的。
    /// 【注意】复权数据尚不支持 `-o clickhouse`。
    #[argh(option, short = 'p')]
    pub previous: Option<std::path::PathBuf>,

    /// 可选。指定 6 位代码来解析股票。`rustdx day -h l` 查看详细使用说明。
    #[argh(option, short = 'l')]
    pub stocklist: Option<String>,

    /// 可选。指定交易所或者代码开头的文件。使用 `rustdx day -h e`
    /// 查看详细使用说明。
    #[argh(option, short = 'e')]
    pub exchange: Option<String>,

    /// 可选。`day -e sh -l xlsx_path.xlsx -x 0`
    #[argh(option, short = 'x', long = "xlsx-col")]
    pub xlsx_col: Option<usize>,

    /// 可选。匹配 6 位代码的前几位。比如：
    /// `-e sz -c 0000` 等价于 `-l sz0000开头的股票` 即 `sz0000*.day`
    #[argh(option, short = 'c')]
    pub code: Option<String>,

    /// 可选。指定解析文件的数量。如果指定多个路径，则为每个路径下待解析的文件数量。
    #[argh(option, short = 'n')]
    pub amount: Option<usize>,

    /// 可选。显示详细的使用说明。
    #[argh(option, short = 'h')]
    description: Vec<String>,

    /// 可选。指定表名称，默认为 `rustdx.tmp`。
    #[argh(option, short = 't', default = "String::from(\"rustdx.tmp\")")]
    pub table: String,
}

impl DayCmd {
    pub fn run(&self) -> Result<()> {
        // 无参数模式：自动下载官网日线完整包 → 解压 → 用 lday 目录继续解析。
        // 若传入的是 URL 或本地 zip 路径，也走同一套下载/解压流程。
        if let Some(dirs) = self.resolve_source_dirs()? {
            let cmd = DayCmd {
                path: dirs,
                ..self.clone()
            };
            return cmd.run();
        }

        match self.output.as_str() {
            "clickhouse" => self.run_clickhouse(),
            x if x.ends_with("csv") => self.run_csv(),
            "mongodb" => crate::io::run_mongodb(self),
            _ => todo!(),
        }
    }

    /// 解析来源：
    /// - path 为空 → 自动下载官网完整包；
    /// - 单个 path 是 URL（http/https）→ 下载该地址；
    /// - 单个 path 是 `.zip` 文件 → 解压本地 zip；
    /// - 否则（目录列表）→ 返回 None，走原有解析逻辑。
    ///
    /// 返回 `Some(dirs)` 表示需要换成这些 lday 目录继续；`None` 表示直接用原 path。
    fn resolve_source_dirs(&self) -> Result<Option<Vec<std::path::PathBuf>>> {
        use crate::download;
        if self.path.is_empty() {
            log::info!(
                "未指定目录，自动从 {} 下载通达信日线完整包 ...",
                download::DEFAULT_HSJDAY_URL
            );
            return Ok(Some(download::prepare_hsjday_dirs()?));
        }

        if self.path.len() == 1 {
            let p = &self.path[0];
            let s = p.to_string_lossy();
            if s.starts_with("http://") || s.starts_with("https://") {
                log::info!("按指定地址下载日线完整包: {s}");
                let dirs = download::prepare_hsjday_dirs_from(&s)?;
                return Ok(Some(dirs));
            }
            if s.ends_with(".zip") && p.is_file() {
                log::info!("解压本地 zip: {}", p.display());
                let dirs = download::prepare_hsjday_dirs_from_zip(p)?;
                return Ok(Some(dirs));
            }
        }
        Ok(None)
    }

    pub fn run_csv(&self) -> Result<()> {
        if self.gbbq.is_some() {
            if self.previous.is_some() {
                crate::io::run_csv_fq_previous(self)
            } else {
                crate::io::run_csv_fq(self)
            }
        } else {
            crate::io::run_csv(self)
        }
    }

    /// clickhouse-client --query "INSERT INTO table FORMAT CSVWithNames" < clickhouse[.csv]
    pub fn run_clickhouse(&self) -> Result<()> {
        crate::io::setup_clickhouse(self.gbbq.is_some(), &self.table)?;
        self.run_csv()?;
        crate::io::insert_clickhouse(&self.output, &self.table, self.keep_csv)
    }

    pub fn help_info(&self) -> &Self {
        for arg in &self.description {
            match arg.as_str() {
                "output" | "o" => println!("{DAYCMD_OUTPUT}"),
                "stocklist" | "l" => println!("{DAYCMD_STOCKLIST}"),
                "exchange" | "e" => println!("{DAYCMD_EXCHANGE}"),
                _ => println!(
                    "请查询以下参数：output stocklist exchange 或者它们的简写 o l \
                               e；\n使用 `-h e -h l` 的形式查询多个参数的使用方法"
                ),
            }
        }
        self
    }

    /// 匹配 `.day` 之前的内容：比如 `sz000001`
    pub fn stocklist(&self) -> Option<fetch_code::StockList> {
        use crate::io::read_xlsx;
        use fetch_code::get_offical_stocks;
        match (
            self.stocklist.as_deref(),
            self.exchange.as_deref(),
            self.xlsx_col,
        ) {
            (Some("official"), _, _) => get_offical_stocks("official").ok(),
            (Some("sse"), _, _) => get_offical_stocks("sse").ok(),
            (Some("szse"), _, _) => get_offical_stocks("szse").ok(),
            (Some(ex), Some(prefix), _) if ex.len() == 6 || ex.contains(',') => {
                self.parse_list(prefix)
            }
            (Some(ex), Some("sz"), _) => read_xlsx(ex, 4, "sz"),
            (Some(ex), Some("sh"), _) => read_xlsx(ex, 0, "sh"),
            (Some(ex), None, Some(n)) => read_xlsx(ex, n, ""),
            (Some(ex), Some(prefix), Some(n)) => read_xlsx(ex, n, prefix),
            _ => self.parse_list(""),
        }
    }

    /// 筛选 sz/sh 交易所和股票代码的开头，并把代码转换为
    /// **含市场前缀的完整字符串**（如 `sz000001`、`sz200b07`）。
    /// 当 -e 为 auto 时，自动匹配 6 开头的股票为 sh，否则为 sz。
    ///
    /// 不再把代码转成 `u32`：既支持带字母的特殊品种代码（如深市板块指数
    /// `200b07`，`u32` 无法表示），也让输出 code 保留市场前缀从而避免
    /// 合并多市场（sh/sz/bj）时的代码混叠。
    pub fn filter_ec(&self, fname: &str) -> (bool, String) {
        let len = fname.len();
        let code = &fname[len - 10..len - 4]; // 6 位代码，可含字母
        let ex_f = &fname[len - 12..len - 10]; // 市场前缀 sh/sz/bj
        let match_ex = |ex: &str| ex == ex_f || ex == "auto";
        let full = format!("{ex_f}{code}");
        (
            self.exchange.as_deref().map(match_ex).unwrap_or(true)
                && self
                    .code
                    .as_ref()
                    .map(|s| code.starts_with(s))
                    .unwrap_or(true),
            full,
        )
    }

    fn parse_list(&self, p: &str) -> Option<StockList> {
        let prefix = |x: &str| format!("{}{}", auto_prefix(p, x), x);
        self.stocklist
            .as_ref()
            .map(|s| s.split(',').map(prefix).collect())
    }
}

#[inline]
pub fn auto_prefix<'a>(prefix: &'a str, code: &'a str) -> &'a str {
    if prefix == "auto" && &code[0..1] == "6" {
        "sh"
    } else if prefix == "auto" {
        "sz"
    } else {
        prefix
    }
}

#[rustfmt::skip]
const DAYCMD_EXCHANGE: &str = "--exchange 或 -e ：
指定 day 文件的代码开头，一般搭配 `-l` 使用：
 * `sz`
 * `sh`

参考 `rustdx day -h l`
";

// 【todo】如果提供 txt 文件路径，则读取里面的六位代码数据。使用 `\\n` 分隔。
// 【todo】如果提供数据库路径，则使用数据库的股票代码。
#[rustfmt::skip]
const DAYCMD_STOCKLIST: &str = "--stocklist 或 -l ：
匹配 `.day` 之前的内容：比如 `sz000001`。具体用法：
 * `-l official` 从上交所和深交所官网获取最新的 A 股、科创板、创业板股票代码列表
 * `-l sse` 从上交所官网获取 A 股、科创板股票代码列表
 * `-l szse` 从深交所官网获取 A 股、创业板股票代码列表
 * `-l xlsx 或 xls 文件路径`，常和 `-e`（6 位代码的前缀） `-x` （xlsx 文件第几列）一起使用，见下面的例子
 * `-l 逗号分隔的 6 位代码` 指定固定几个股票，见下面的例子

`-l excel_path.xls[x] -e sz` 从本地路径获取深交所官网下载的代码列表
  （或者第 4 (E) 列 6 位股票代码的 excel，代码开头会自动添 `sz`）
`-l excel_path.xls[x] -e sh` 从本地路径获取上交所官网下载的代码列表
  （或者第 0 (A) 列 6 位股票代码的 excel，代码开头会自动添 `sh` ）
* 或者更一般地：`-l excel_path.xls[x] -x n [-e prefix]` 表示
  识别 excel_path.xls[x] 文件第 n 列股票代码，如果需要添加前缀，则指定 -e，
  如果不需要添加前缀，则不需要 -e

`-l sz000001,sh688001` 逗号分隔的带 sh/sz 标识的代码字符串
`-l 000001,000002 -e sz` 等价于 `-l sz000001,sz000002`
`-l 688001,688002 -e sh` 等价于 `-l sh688001,sh688002`
* 或者更一般地： `-l 688001,688002 -e xx` 等价于 `-l xx688001,xx688002`

【注意】由于该参数是可选的，这意味着没有指定 `-l` 时，会解析所提供文件夹下所有 day 文件。
        如果你无法确保该文件夹下的数据完全是你需要的，请指定 `-l` 参数。
        比如：通达信官网 (https://www.tdx.com.cn/article/alldata.html) 下载的
        “上证所有证券日线” 和 “上证所有证券日线” 数据包含许多除股票之外的证券数据。
        建议使用 `-l official`。
";

#[rustfmt::skip]
const DAYCMD_OUTPUT: &str = "--output 或 -o ：
解析后的输出方式：
`-o csv_path.csv` 保存成 csv 格式，默认值为 stocks.csv，表示当前目录的 stocks.csv 文件
`-o clickhouse` 保存成 csv 格式，并把 csv 的数据插入到 clickhouse 数据库
`-o mongodb` 保存成 csv 格式，并把 csv 的数据插入到 mongodb 数据库

注意：
1. 成功插入到 clickhouse 或 mongodb 数据库之后，默认会删除掉解析的 stocks.csv 文件。
   如果需要保存这个文件，使用 `-k` 参数：`-o clickhouse -k` 或 `-o mongodb -k`。
2. clickhouse 数据库必须先建表再插入数据，因此本工具会提前建表。
3. 支持 `-g xx [-p xx]` 和 `-o clickhouse` 并存。即 
   `rustdx day day_file_path -o clickhouse -g gbbq_path [-p csv_path]`
   表示解析并插入复权数据到 clickhouse。
";
