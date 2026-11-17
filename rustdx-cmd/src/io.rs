use crate::cmd::DayCmd;
use eyre::{Result, anyhow, eyre};
use rustdx_cli::fetch_code::StockList;
use rustdx_complete::file::{
    day::fq::Day,
    gbbq::{Factor, Gbbq},
};
use std::{
    fs::{self, File},
    io::{self, Write},
    path::Path,
    process::Command,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::Instant,
};

const BUFFER_SIZE: usize = 32 * (1 << 20); // 32M

/// 并发解析目录下的 day 文件并写入 CSV（利用多核优势）。
///
/// - 解析阶段：多个工作线程（默认逻辑核心数）并行读取并解析 `.day` 文件；
/// - 写入阶段：主线程从 channel 顺序接收解析结果并写入 CSV，避免并发写文件。
///
/// `parse` 负责把单个文件解析成 `Vec<T>`，`T` 为可序列化的日线类型。
fn run_par<F, T>(cmd: &DayCmd, wtr: &mut csv::Writer<File>, parse: F) -> Result<()>
where
    F: Fn(&str, &Path) -> eyre::Result<Vec<T>> + Send + Sync,
    T: serde::Serialize + Send,
{
    let start = Instant::now();
    let hm = cmd.stocklist();
    let mut total_files = 0usize;
    let mut total_rows = 0usize;
    for dir in &cmd.path {
        // 收集 (完整代码, 文件路径)，代码含市场前缀（如 `sz000001`、`sz200b07`）
        let work: Vec<(String, std::path::PathBuf)> = filter_file(dir)?
            .filter_map(|f| {
                let s = f.to_str().unwrap();
                let (b, code) = cmd.filter_ec(s);
                filter(b, f.as_path(), hm.as_ref(), dir)
                    .unwrap_or(false)
                    .then_some((code, f))
            })
            .take(
                cmd.amount
                    .unwrap_or_else(|| filter_file(dir).map(|it| it.count()).unwrap_or(0)),
            )
            .collect();
        let n = work.len();
        info!("dir: {dir:?} day 文件数量：{n}");
        let take = cmd.amount.unwrap_or(n);

        let count = AtomicUsize::new(0); // 成功解析的文件数
        let next = AtomicUsize::new(0); // 任务队列：原子取下一个文件
        let workers = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let (tx, rx) = mpsc::channel::<Vec<T>>();

        // 所有 worker 共享的引用：原子计数器与任务列表
        let count = &count;
        let next = &next;
        let work = &work;
        let parse = &parse;

        thread::scope(|s| {
            for _ in 0..workers {
                let tx = tx.clone();
                s.spawn(move || {
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= work.len() {
                            break;
                        }
                        let (code, src) = &work[i];
                        if let Ok(v) = parse(code, src) {
                            count.fetch_add(1, Ordering::Relaxed);
                            // 发送失败说明主线程已退出，直接结束
                            let _ = tx.send(v);
                        }
                    }
                });
            }
            drop(tx); // 关闭发送端，主线程的迭代才能结束
            for v in rx {
                total_rows += v.len();
                for t in v {
                    wtr.serialize(t)?;
                }
            }
            Ok::<(), eyre::Report>(())
        })?;

        total_files += count.load(Ordering::Relaxed);
        print(dir, count.load(Ordering::Relaxed), take);
    }
    wtr.flush()?;

    // 解析后校验报告
    let elapsed = start.elapsed();
    let out_size = fs::metadata(&cmd.output)
        .map(|m| m.len() as f64 / (1024.0 * 1024.0))
        .unwrap_or(0.0);
    info!(
        "解析完成 → 文件 {} 个 / 行 {} 行 / 输出 {:.1} MiB / 耗时 {:.1}s",
        total_files,
        total_rows,
        out_size,
        elapsed.as_secs_f64()
    );
    Ok(())
}

/// 并发解析并输出 CSV（无复权）。
pub fn run_csv(cmd: &DayCmd) -> Result<()> {
    let file = File::create(&cmd.output)?;
    let mut wtr = csv::WriterBuilder::new()
        .buffer_capacity(BUFFER_SIZE)
        .from_writer(file);
    run_par(cmd, &mut wtr, |code, src| {
        Ok(rustdx_complete::file::day::Day::from_file_into_vec(
            code, src,
        )?)
    })
}

/// 并发解析并输出 CSV（复权，基于上市日）。
pub fn run_csv_fq(cmd: &DayCmd) -> Result<()> {
    // 股本变迁
    let mut bytes = fs::read(cmd.gbbq.as_ref().unwrap())?;
    let gbbq = Gbbq::filter_hashmap(Gbbq::iter(&mut bytes[4..]));

    let file = File::create(&cmd.output)?;
    let mut wtr = csv::WriterBuilder::new()
        .buffer_capacity(BUFFER_SIZE)
        .from_writer(file);
    run_par(cmd, &mut wtr, |code, src| {
        Ok(Day::new(code, src, gbbq.get(code).map(Vec::as_slice))?)
    })
}

/// 并发解析并输出 CSV（复权，基于前一日的因子）。
pub fn run_csv_fq_previous(cmd: &DayCmd) -> Result<()> {
    // 股本变迁
    let mut bytes = fs::read(cmd.gbbq.as_ref().unwrap())?;
    let gbbq = Gbbq::filter_hashmap(Gbbq::iter(&mut bytes[4..]));

    // 前收
    let previous = previous_csv_table(&cmd.previous, &cmd.table, cmd.keep_factor)?;

    let file = File::create(&cmd.output)?;
    let mut wtr = csv::WriterBuilder::new()
        .buffer_capacity(BUFFER_SIZE)
        .from_writer(file);
    run_par(cmd, &mut wtr, |code, src| {
        Ok(Day::concat(
            code,
            src,
            // 无分红数据并不意味着无复权数据
            gbbq.get(code).map(Vec::as_slice),
            previous.get(code),
        )?)
    })
}

/// 筛选 day 文件
#[rustfmt::skip]
fn filter_file(dir: &Path) -> Result<impl Iterator<Item = std::path:: PathBuf>> {
    Ok(dir.read_dir()?
          .filter_map(|e| e.map(|f| f.path()).ok())
          .filter(|p| p.extension().map(|s| s == "day").unwrap_or_default()))
}

/// 筛选存在于股票列表的股票
#[inline]
fn filter(b: bool, src: &Path, hm: Option<&StockList>, dir: &Path) -> Option<bool> {
    let src = src.strip_prefix(dir).ok()?.to_str()?.strip_suffix(".day")?;
    Some(b && hm.map(|m| m.contains(src)).unwrap_or(true))
}

fn print(dir: &Path, count: usize, take: usize) {
    if count == 0 && take != 0 {
        error!("{dir:?} 目录下无 `.day` 文件符合要求");
    } else if take == 0 {
        error!("请输入大于 0 的文件数量");
    } else {
        info!("{dir:?}\t已完成：{count}");
    }
}

/// 把 `库名.表名` 拆成 `(库名, 表名)`（表名不带前导 `.`）。
///
/// 库名、表名会被直接拼进 ClickHouse SQL，所以只允许 `[A-Za-z0-9_]`，
/// 且必须恰好含一个 `.`（多个 `.` 时表名部分含非法字符，同样报错）。
fn database_table(table: &str) -> Result<(&str, &str)> {
    let valid =
        |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
    match table.split_once('.') {
        Some((db, tb)) if valid(db) && valid(tb) => Ok((db, tb)),
        _ => Err(eyre!(
            "表名 {table:?} 不合法：应为 `库名.表名` 格式，且库名、表名只能包含字母、数字、下划线"
        )),
    }
}

#[test]
fn test_database_table() {
    // 合法
    assert_eq!(database_table("rustdx.tmp").unwrap(), ("rustdx", "tmp"));
    assert_eq!(database_table("db_1.Day_2").unwrap(), ("db_1", "Day_2"));
    // 无点、库名或表名为空
    for s in ["", "rustdx", ".tmp", "rustdx.", "."] {
        assert!(database_table(s).is_err(), "{s:?}");
    }
    // 多个点：不支持，报错
    assert!(database_table("a.b.c").is_err());
    // 非法字符（含 SQL 注入、空白、反引号、非 ASCII）
    for s in [
        "rustdx.tmp; DROP TABLE x",
        "rust dx.tmp",
        "rustdx.`tmp`",
        "rustdx.tmp-1",
        "库.表",
    ] {
        assert!(database_table(s).is_err(), "{s:?}");
    }
    let msg = database_table("tmp").unwrap_err().to_string();
    assert!(msg.contains("库名.表名"), "{msg}");
}

/// ClickHouse HTTP 连接配置（环境变量，约定与生态一致）。
#[derive(Debug, Clone)]
struct ClickHouseConfig {
    url: String,
    user: String,
    password: String,
}

impl ClickHouseConfig {
    fn from_env() -> Self {
        Self {
            url: std::env::var("CLICKHOUSE_URL").unwrap_or_else(|_| "http://127.0.0.1:8123".into()),
            user: std::env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "default".into()),
            password: std::env::var("CLICKHOUSE_PASSWORD").unwrap_or_default(),
        }
    }

    /// 给请求加上认证 header（ClickHouse 原生 X-ClickHouse-User/Key），并关闭
    /// 「4xx/5xx 即报错」：ClickHouse 的错误原因在响应体里，需要读出来。
    fn request(&self, url: &str) -> ureq::RequestBuilder<ureq::typestate::WithBody> {
        let mut req = ureq::post(url).config().http_status_as_error(false).build();
        if self.user != "default" || !self.password.is_empty() {
            req = req.header("X-ClickHouse-User", &self.user);
        }
        if !self.password.is_empty() {
            req = req.header("X-ClickHouse-Key", &self.password);
        }
        req
    }

    /// 检查响应状态并返回响应体；失败时带上 ClickHouse 给出的原因。
    fn check(mut resp: ureq::http::Response<ureq::Body>) -> Result<String> {
        let status = resp.status().as_u16();
        let body = resp.body_mut().read_to_string().unwrap_or_default();
        if (200..300).contains(&status) {
            return Ok(body);
        }
        let body = body.trim();
        // 鉴权失败实测多因 cwd 不在部署目录、CLICKHOUSE_* env 缺失
        let hint = if matches!(status, 401 | 403) {
            "（鉴权失败：CLICKHOUSE_URL/CLICKHOUSE_USER/CLICKHOUSE_PASSWORD \
             环境变量须与部署 .env 一致，在部署目录下执行）"
        } else {
            ""
        };
        Err(eyre!("ClickHouse HTTP {status}{hint}: {body}"))
    }

    /// 执行 SQL（DDL/DML，body 即语句）。
    fn exec(&self, sql: &str) -> Result<String> {
        let resp = self
            .request(&self.url)
            .header("Content-Type", "text/plain; charset=utf-8")
            .send(sql.as_bytes())
            .map_err(|e| eyre!("{e}"))?;
        Self::check(resp)
    }

    /// 数据流导入（CSV 文件 → `INSERT INTO <table> FORMAT CSVWithNames`）。
    fn insert_csv(&self, table: &str, csv: &std::path::Path) -> Result<()> {
        let mut file = std::fs::File::open(csv)?;
        let len = file.metadata()?.len();
        let sql = urlencode(&format!("INSERT INTO {table} FORMAT CSVWithNames"));
        let resp = self
            .request(&format!("{}/?query={sql}", self.url))
            .send(ureq::SendBody::from_reader(&mut file))
            .map_err(|e| eyre!("{e}"))?;
        Self::check(resp)?;
        debug!("clickhouse 导入完成：{table}（{len} 字节 CSV）");
        // 输出数据边界：缓存解压目录可能滞后于最新交易日（实测 09-10 命中
        // 09-06 的旧缓存，包行数相近极易误判为新包）——一眼确认新鲜度。
        // 仅为诊断信息：查询失败不能让已经成功的导入以失败收场。
        match self.exec(&format!(
            "SELECT concat(toString(count()), ' 行，数据截至 ', toString(max(date))) FROM {table}"
        )) {
            Ok(info) => info!("day_file 数据边界: {}", info.trim()),
            Err(e) => warn!("导入已完成，但查询数据边界失败: {e}"),
        }
        Ok(())
    }
}

/// URL 查询参数转义（导入 SQL 仅含表名/FORMAT 关键字，字符集有限）。
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// ClickHouse HTTP 执行 SQL（建库建表等 DDL）。
fn ch_execute(cfg: &ClickHouseConfig, sql: &str) -> Result<()> {
    cfg.exec(sql).map(|_| ())
}

pub fn setup_clickhouse(fq: bool, table: &str) -> Result<()> {
    let (database_name, _) = database_table(table)?;
    let cfg = ClickHouseConfig::from_env();
    let create_database = format!("CREATE DATABASE IF NOT EXISTS {database_name}");
    ch_execute(&cfg, &create_database)?;
    #[rustfmt::skip]
    let create_table = if fq {
        format!("
            CREATE TABLE IF NOT EXISTS {table}
            (
                `date` Date CODEC(DoubleDelta),
                `code` String,
                `open` Float64,
                `high` Float64,
                `low` Float64,
                `close` Float64,
                `amount` Float64,
                `vol` Float64,
                `preclose` Float64,
                `factor` Float64
            )
            ENGINE = ReplacingMergeTree()
            ORDER BY (date, code)
        ")
    } else {
        format!("
            CREATE TABLE IF NOT EXISTS {table}
            (
                `date` Date CODEC(DoubleDelta),
                `code` String,
                `open` Float64,
                `high` Float64,
                `low` Float64,
                `close` Float64,
                `amount` Float64,
                `vol` Float64
            )
            ENGINE = ReplacingMergeTree()
            ORDER BY (date, code)
        ")
    }; // PARTITION BY 部分可能需要去掉
    ch_execute(&cfg, &create_table)
}

pub fn insert_clickhouse(output: &impl AsRef<Path>, table: &str, keep: bool) -> Result<()> {
    database_table(table)?; // 表名会拼进 SQL，先校验
    let cfg = ClickHouseConfig::from_env();
    cfg.insert_csv(table, output.as_ref())?;
    info!("成功插入数据到 clickhouse 数据库（{table}）");
    keep_csv(output, keep)?;
    Ok(())
}

/// 需要本机 ClickHouse 服务与日线 clickhouse csv 文件（缺任一则跳过）
#[test]
fn test_insert_clickhouse() -> Result<()> {
    if std::env::var_os("RUSTDX_SKIP_INTEGRATION_TESTS").is_some() {
        eprintln!("⚠️  跳过：已设置 RUSTDX_SKIP_INTEGRATION_TESTS");
        return Ok(());
    }
    // 写入走 HTTP：连不上 ClickHouse 服务时跳过，而不是让 cargo test 失败
    if let Err(e) = ClickHouseConfig::from_env().exec("SELECT 1") {
        eprintln!("⚠️  跳过：无法连接 ClickHouse（{e}）");
        return Ok(());
    }
    if !Path::new("clickhouse").exists() {
        eprintln!("⚠️  跳过：当前目录下没有日线 csv 文件 `clickhouse`");
        return Ok(());
    }
    setup_clickhouse(true, "rustdx.tmp")?;
    insert_clickhouse(&"clickhouse", "rustdx.tmp", true)
}

type Previous = Result<std::collections::HashMap<String, Factor>>;

pub fn previous_csv_table(
    path: &Option<std::path::PathBuf>,
    table: &str,
    keep_factor: bool,
) -> Previous {
    if let Some(Some(path)) = path.as_ref().map(|p| p.to_str()) {
        if path == "clickhouse" {
            clickhouse_factor_csv(table, keep_factor)
        } else {
            previous_csv(path, keep_factor)
        }
    } else {
        Err(anyhow!("请检查 gbbq 路径"))
    }
}

/// 读取前收盘价（前 factor ）数据。
///
/// 注意：`code` 列需为**含市场前缀的完整代码**（如 `sh600000`），
/// 与 day 命令输出的 code 格式一致，否则无法匹配。
pub fn previous_csv(p: impl AsRef<Path>, keep_factor: bool) -> Previous {
    let path = p.as_ref();
    let prev = csv::Reader::from_reader(File::open(path)?)
        .deserialize::<Factor>()
        .filter_map(|f| f.ok())
        .map(|f| (f.code.clone(), f))
        .collect();
    if !keep_factor {
        fs::remove_file(path)?;
    }
    Ok(prev)
}

/// 获取当前最新 factor（走 ClickHouse HTTP，与写入共用 `CLICKHOUSE_*` 配置）
fn clickhouse_factor_csv(table: &str, keep_factor: bool) -> Previous {
    database_table(table)?; // 表名会拼进 SQL，先校验
    let query = format!(
        "\
WITH 
  df AS (
  SELECT
    code,
  arrayLast(
      x->true, 
      arraySort(x->x.1, groupArray((
        date, close, factor
      )))
    ) AS t
  FROM
    {table}
  GROUP BY
    code
  )
SELECT code, t.1 AS date, t.2 AS close, t.3 AS factor FROM df
FORMAT CSVWithNames"
    );
    debug!("clickhouse 查询 factor：{query:?}");
    let body = ClickHouseConfig::from_env().exec(&query)?;
    // 落盘后沿用 previous_csv，保持 keep_factor（是否保留 factor.csv）的语义
    fs::write("factor.csv", body)?;
    previous_csv("factor.csv", keep_factor)
}

/// TODO: 与数据库有关的，把库名、表名可配置
pub fn run_mongodb(cmd: &DayCmd) -> Result<()> {
    // TODO:排查为什么 date 列无法变成 date 类型 date.date(2006-01-02)
    // 先校验表名，避免解析完 day 文件才发现参数不合法
    let (database_name, table_name) = database_table(&cmd.table)?;
    cmd.run_csv()?;
    let args = [
        "--db",
        database_name,
        "--collection",
        table_name,
        "--type=csv",
        "--file",
        &cmd.output,
        "--columnsHaveTypes",
        "--fields=code.string()",
    ];
    let output = Command::new("mongoimport")
        .args(args)
        .output()
        .map_err(|e| {
            eyre!("无法运行 mongoimport（需安装 MongoDB Database Tools 并加入 PATH）: {e}")
        })?;
    check_output("mongoimport", output)?;
    keep_csv(&cmd.output, cmd.keep_csv)?;
    Ok(())
}

/// 转发子进程的输出；子进程失败时返回带退出码的错误。
fn check_output(program: &str, output: std::process::Output) -> Result<()> {
    io::stdout().write_all(&output.stdout)?;
    io::stderr().write_all(&output.stderr)?;
    if output.status.success() {
        return Ok(());
    }
    match output.status.code() {
        Some(code) => Err(eyre!("{program} 执行失败，退出码 {code}")),
        None => Err(eyre!("{program} 被信号终止（无退出码）")),
    }
}

fn keep_csv(fname: &impl AsRef<Path>, keep: bool) -> io::Result<()> {
    if keep {
        fs::rename(fname, fname.as_ref().with_extension("csv"))
    } else {
        fs::remove_file(fname)
    }
}

/// 读取本地 xls(x) 文件
pub fn read_xlsx(path: &str, col: usize, prefix: &str) -> Option<StockList> {
    use calamine::{Data, Reader, open_workbook_auto};
    let mut workbook = open_workbook_auto(path).ok()?;
    let format_ = |x: &str| format!("{}{}", crate::cmd::auto_prefix(prefix, x), x);
    // 每个单元格被解析的类型可能会不一样，所以把股票代码统一转化成字符型
    if let Some(Ok(range)) = workbook.worksheet_range_at(0) {
        Some(
            range
                .rows()
                .skip(1)
                // 列越界、空单元格或其他类型（日期、布尔、错误值等）的行直接跳过
                .filter_map(|r| match r.get(col)? {
                    Data::Int(x) => Some(format_(&x.to_string())),
                    Data::Float(x) => Some(format_(&(*x as i64).to_string())),
                    Data::String(x) => Some(format_(x)),
                    _ => None,
                })
                .collect(),
        )
    } else {
        None
    }
}
