use crate::cmd::DayCmd;
use eyre::{Result, anyhow};
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
    let hm = cmd.stocklist();
    for dir in &cmd.path {
        // 收集 (完整代码, 文件路径)，代码含市场前缀（如 `sz000001`、`sz200b07`）
        let work: Vec<(String, std::path::PathBuf)> = filter_file(dir)?
            .filter_map(|f| {
                let s = f.to_str().unwrap();
                let (b, code) = cmd.filter_ec(s);
                filter(b, f.as_path(), hm.as_ref(), dir).unwrap_or(false)
                    .then_some((code, f))
            })
            .take(cmd.amount.unwrap_or_else(|| filter_file(dir).map(|it| it.count()).unwrap_or(0)))
            .collect();
        let n = work.len();
        info!("dir: {dir:?} day 文件数量：{n}");
        let take = cmd.amount.unwrap_or(n);

        let count = AtomicUsize::new(0); // 成功解析的文件数
        let next = AtomicUsize::new(0); // 任务队列：原子取下一个文件
        let workers = thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        let (tx, rx) = mpsc::channel::<Vec<T>>();

        // 所有 worker 共享的引用：原子计数器与任务列表
        let count = &count;
        let next = &next;
        let work = &work;
        let parse = &parse;

        thread::scope(|s| {
            for _ in 0..workers {
                let tx = tx.clone();
                s.spawn(move || loop {
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
                });
            }
            drop(tx); // 关闭发送端，主线程的迭代才能结束
            for v in rx {
                for t in v {
                    wtr.serialize(t)?;
                }
            }
            Ok::<(), eyre::Report>(())
        })?;

        print(dir, count.load(Ordering::Relaxed), take);
    }
    wtr.flush().map_err(|e| e.into())
}

/// 并发解析并输出 CSV（无复权）。
pub fn run_csv(cmd: &DayCmd) -> Result<()> {
    let file = File::create(&cmd.output)?;
    let mut wtr = csv::WriterBuilder::new()
        .buffer_capacity(BUFFER_SIZE)
        .from_writer(file);
    run_par(cmd, &mut wtr, |code, src| {
        Ok(rustdx_complete::file::day::Day::from_file_into_vec(code, src)?)
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

fn database_table(table: &str) -> (&str, &str) {
    let pos = table.find('.').unwrap();
    table.split_at(pos) // (database_name, table_name)
}

pub fn setup_clickhouse(fq: bool, table: &str) -> Result<()> {
    let create_database = format!("CREATE DATABASE IF NOT EXISTS {}", database_table(table).0);
    let output = Command::new("clickhouse-client")
        .args(["--query", &create_database])
        .output()?;
    check_output(output);
    #[rustfmt::skip]
    let create_table = if fq {
        format!("
            CREATE TABLE IF NOT EXISTS {table}
            (
                `date` Date CODEC(DoubleDelta),
                `code` String,
                `open` Float32,
                `high` Float32,
                `low` Float32,
                `close` Float32,
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
                `open` Float32,
                `high` Float32,
                `low` Float32,
                `close` Float32,
                `amount` Float64,
                `vol` Float64
            )
            ENGINE = ReplacingMergeTree()
            ORDER BY (date, code)
        ")
    }; // PARTITION BY 部分可能需要去掉
    let output = Command::new("clickhouse-client")
        .args(["--query", &create_table])
        .output()?;
    check_output(output);
    Ok(())
}

pub fn insert_clickhouse(output: &impl AsRef<Path>, table: &str, keep: bool) -> Result<()> {
    use std::process::{Command, Stdio};
    let query = format!("INSERT INTO {table} FORMAT CSVWithNames");
    let result = Command::new("clickhouse-client")
        .args(["--query", &query])
        .stdin(Stdio::from(File::open(output)?))
        .output()?;
    if result.status.success() {
        info!("成功插入数据到 clickhouse 数据库");
        debug!(
            "clickhouse 返回结果：{}",
            String::from_utf8_lossy(&result.stdout)
        );
    } else {
        error!(
            "插入数据到 clickhouse 数据库时遇到：{}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    keep_csv(output, keep)?;
    Ok(())
}

/// 需要日线 clickhouse csv 文件
#[test]
fn test_insert_clickhouse() -> Result<()> {
    // 本地未安装 clickhouse-client 时跳过（该测试依赖本机 ClickHouse 服务）
    if std::process::Command::new("clickhouse-client")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("⚠️  跳过：未安装 clickhouse-client");
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

/// 获取当前最新 factor
fn clickhouse_factor_csv(table: &str, keep_factor: bool) -> Previous {
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
INTO OUTFILE 'factor.csv'
FORMAT CSVWithNames;"
    );
    let args = ["--query", &query];
    let output = Command::new("clickhouse-client").args(args).output()?;
    info!("clickhouse-client --query {query:?}");
    check_output(output);
    previous_csv("factor.csv", keep_factor)
}

/// TODO: 与数据库有关的，把库名、表名可配置
pub fn run_mongodb(cmd: &DayCmd) -> Result<()> {
    cmd.run_csv()?;
    // TODO:排查为什么 date 列无法变成 date 类型 date.date(2006-01-02)
    let (database_name, table_name) = database_table(&cmd.table);
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
    let output = Command::new("mongoimport").args(args).output()?;
    check_output(output);
    keep_csv(&cmd.output, cmd.keep_csv)?;
    Ok(())
}

fn check_output(output: std::process::Output) {
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();
    assert!(output.status.success());
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
                .map(|r| match &r[col] {
                    Data::Int(x) => format_(&x.to_string()),
                    Data::Float(x) => format_(&(*x as i64).to_string()),
                    Data::String(x) => format_(x),
                    _ => unreachable!(),
                })
                .collect(),
        )
    } else {
        None
    }
}
