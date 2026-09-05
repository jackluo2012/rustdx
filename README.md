# rustdx

[<img alt="github" src="https://img.shields.io/github/license/jackluo2012/rustdx?color=blue" height="20">](https://github.com/jackluo2012/rustdx)
[<img alt="github" src="https://img.shields.io/github/issues/jackluo2012/rustdx?color=db2043" height="20">](https://github.com/jackluo2012/rustdx/issues)
[<img alt="crates.io" src="https://img.shields.io/crates/v/rustdx-complete?style=flat&color=fc8d62&logo=rust&label=rustdx-complete" height="20">](https://crates.io/crates/rustdx-complete)
[<img alt="crates.io" src="https://img.shields.io/crates/v/rustdx-complete/1.2.0?style=flat&color=green&logo=rust&logoColor=white&label=v1.2.0" height="20">](https://crates.io/crates/rustdx-complete)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-rustdx-66c2a5?style=flat&labelColor=555555&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/rustdx-complete)
[<img alt="crates.io" src="https://img.shields.io/crates/v/rustdx-cli?style=flat&color=fc8d62&logo=rust&label=rustdx-cli" height="20">](https://crates.io/crates/rustdx-cli)

受 [pytdx](https://pypi.org/project/pytdx/) / [mootdx](https://github.com/mootdx/mootdx) 启发的 A 股数据获取工具（Rust 实现）：

1. 一个 Rust 通用库 [rustdx-complete](https://crates.io/crates/rustdx-complete)：通达信行情协议 + 本地数据文件解析 + 技术指标；
2. 一个命令行工具 [rustdx-cli](https://crates.io/crates/rustdx-cli)：解析通达信 day 文件、东财日线增量更新、ClickHouse 写入。

## ✨ 特性

- **行情协议全覆盖**：实时行情（含五档盘口）、K线、指数K线、当日/历史分时、当日/历史逐笔、财务信息、除权除息、F10 公司资料、板块文件（概念/指数/风格）
- **连接可靠性**：40 台服务器协议级自动故障转移、心跳保活、失败自动重连重试、超时可配置
- **数据正确性**：全部协议按 pytdx 源码逐字节对照并经交易时段实盘验证；解析异常时返回空数据而非垃圾数据
- **本地 day 文件并发解析**：rustdx-cli 多线程并行解析（线程数=逻辑核心数），并支持带字母的特殊品种代码（如深市板块指数 `sz200b07`）
- **技术指标**：SMA/EMA/MACD/RSI/布林带/KDJ
- **辅助能力**：交易日历、智能缓存、连接池、Builder 模式 API、数据验证
- **197 个测试**（含真实抓包字节的回归测试），clippy 零警告

## 📦 安装

```toml
[dependencies]
rustdx-complete = "1.2.0"
```

或 `cargo add rustdx-complete`。

## 🚀 快速开始（Client 高层 API）

```rust
use rustdx_complete::tcp::stock::{market_of, Client};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 连接（内置 40 台服务器故障转移、心跳与重试）
    let mut client = Client::new()?;

    // 实时行情（代码自动推断市场）
    let code = "600519";
    let quotes = client.quotes(&[(market_of(code).unwrap(), code)])?;
    for q in &quotes {
        println!("{}: {:.2} (昨收 {:.2}, {:+.2}%)", q.code, q.price, q.last_close, q.change_percent);
    }

    // 按日期区间拉取日K线
    let bars = client.k(1, "600519", Some(20260801), Some(20260904))?;
    for b in &bars {
        println!("{}-{:02}-{:02} 收{:.2}", b.dt.year, b.dt.month, b.dt.day, b.close);
    }

    // 概念板块（服务器真实数据，269 个板块）
    let blocks = client.block("block_gn.dat")?;
    let lidian: Vec<_> = blocks.iter().filter(|r| r.blockname == "锂电池").collect();
    println!("锂电池板块 {} 只成分股", lidian.len());

    Ok(())
}
```

## 📖 功能对照（rustdx ↔ mootdx/pytdx）

| mootdx 方法 | rustdx API | 说明 |
|------|------|------|
| `quotes` | `Client::quotes` / `SecurityQuotes` | 实时行情快照，含五档盘口、内外盘、服务器时间 |
| `bars` | `Client::bars` / `Kline` | 股票K线（5m/15m/30m/1h/日/周/月…，category 0-11） |
| `index` / `index_bars` | `Client::index_bars` / `IndexKline` | 指数K线，含上涨/下跌家数 |
| `k` | `Client::k` | 按日期区间拉日K线，自动翻页 |
| `minute` | `Client::minute` / `MinuteTime` | 当日分时 ⚠️ 见已知问题 |
| `minutes` | `Client::history_minute` / `HistoryMinuteTime` | 历史分时 |
| `transaction` | `Client::transaction` / `Transaction` | 当日逐笔成交 |
| `transactions` | `Client::history_transaction` / `HistoryTransaction` | 历史逐笔成交 |
| `finance` | `Client::finance` / `FinanceInfo` | 35 个财务字段 |
| `xdxr` | `Client::xdxr` / `Xdxr` | 除权除息/送配股等股本变迁 |
| `F10C` / `F10` | `Client::f10_categories` / `Client::f10` | 公司资料栏目与内容 |
| `block` | `Client::block` / `get_block_info` | 板块文件（block_gn/zs/fg.dat） |
| `stocks` / `stock_count` | `Client::stocks` / `stock_count` | 全量证券列表（自动分页） |
| `bestip` / `check_server` | `tcp::ip::check_alive` / `check_alive_protocol` | 服务器连通性探测（TCP/协议级） |
| heartbeat / auto_retry | `Tcp::heartbeat` / `Tcp::retry` | 心跳保活、失败自动重连重试 |

> 不计划支持：mootdx `ExtQuotes` 扩展市场（官方标注已失效）。

## 🔧 底层 API（细粒度控制）

每个协议对应一个请求结构体，实现统一的 `Tdx` trait（`send` / `recv` / `parse` / `recv_parsed`）：

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::SecurityQuotes;

let mut tcp = Tcp::new()?; // 故障转移 + 握手
let mut quotes = SecurityQuotes::new(vec![(0, "000001"), (1, "600000")]);
quotes.recv_parsed(&mut tcp)?;
for q in quotes.result() {
    println!("{}: 买一 {:.2}×{:.0} 卖一 {:.2}×{:.0}",
        q.code, q.bid1, q.bid1_vol, q.ask1, q.ask1_vol);
}
```

连接层能力：

```rust
use rustdx_complete::tcp::{ip, Tcp, TcpConfig};
use std::time::Duration;

// 指定服务器与超时
let cfg = TcpConfig { timeout: Duration::from_secs(5), ip: Some(ip::STOCK_IP[0]) };
let mut tcp = Tcp::with_config(&cfg)?;

// 心跳 / 重连 / 自动重试
tcp.heartbeat()?;
tcp.reconnect()?;
```

## 📊 附加模块

- `indicators`：SMA/EMA/MACD/RSI/布林带/KDJ，与K线数据无缝衔接
- `calendar`：A股交易日历（法定节假日，数据源 trade_date_a）
- `cache`：内存/文件缓存（TTL 过期）
- `pool`：TCP 连接池
- `builder`：KlineBuilder 链式 API
- `tcp::stock::validator`：K线连续性/财务一致性验证、异常值检测
- `file`：通达信本地文件解析（.day 日线、.lc1/.lc5 分钟线、gbbq 股本变迁）

## ⚠️ 已知问题（诚实声明）

1. **当日分时**（`MinuteTime`）：2026 年起部分通达信服务器变更了分时响应格式，
   pytdx/mootdx 同样无法解析。rustdx 内置防御性校验，解析异常时返回**空数据**
   （不会输出垃圾数据）；历史分时不受影响（实测 240 点完整）。
   逆向进展见 [pytdx #148](https://github.com/rainx/pytdx/issues/148)。
2. **历史逐笔的买卖方向**：实测除 0=买、1=卖、2=中性外还会出现 5、8 等值
   （疑似集合竞价标记），服务器语义未公开，请谨慎使用该字段。
3. **东财接口**（rustdx-cli 的 `east` 命令）：在代理/VPN（fake-IP DNS）环境下会被
   断开，需要直连网络。

## 🖥 rustdx-cli 命令行

```console
$ cargo install rustdx-cli

# 解析通达信 day 文件（含复权），写入 ClickHouse
$ rustdx day /path/tdx/sh/ /path/tdx/sz/ -l official -g gbbq -o clickhouse -t rustdx.factor

# 每个交易日收盘后，用东财数据增量更新（需直连网络）
$ rustdx east -p clickhouse -o clickhouse -t rustdx.factor
```

**day 命令输出说明**（v1.2.0 起）：
- **code 列含市场前缀**：`sh600000` / `sz000001` / `sz200b07`，合并多市场（sh/sz/bj）
  输出时不会发生同代码混叠（例如 `sh000001` 上证指数与 `sz000001` 平安银行可区分）；
- **支持带字母代码**：深市板块指数等特殊品种（如 `sz200b07`）不再被过滤跳过；
- **并发解析**：多线程并行（线程数 = 逻辑核心数），单核串行 → 并发后全市场
  日线（12,394 个 `.day`）解析由约 3 分钟降至约 1 分 48 秒。

> ⚠️ 破坏性变更：v1.1.x 输出 code 为 6 位裸数字，v1.2.0 起带市场前缀；
> 复权 `-p` 的前一日 factor CSV 的 code 列也需为带前缀格式。

历史日线统计（上游数据，单核 release build）：解析全部 A 股历史 < 30s；
东财日线增量更新约 2s。涨跌幅复权算法无需重算历史复权信息，
详见[涨跌幅复权与前复权](https://zjp-cn.github.io/posts/qfq/)。

## 🧪 测试

```console
$ cargo test --workspace   # 197 个测试（含实盘网络验证）
$ cargo clippy --workspace --all-targets   # 零警告
```

协议解析的正确性通过「真实抓包字节 → 期望结构」回归测试保障（与 pytdx
同连接抓包逐字段对照），协议再变化时能第一时间发现。

## 📝 CHANGELOG

[更新记录](CHANGELOG.md)

## 🙏 致谢

- [pytdx](https://github.com/rainx/pytdx) / [tdxpy](https://github.com/mootdx/tdxpy)：协议逆向参考
- [mootdx](https://github.com/mootdx/mootdx)：服务器列表维护与 API 设计参考
- [zjp-CN/rustdx](https://github.com/zjp-CN/rustdx)：本项目 fork 的上游（本地文件解析与复权计算）
- 通达信：提供行情服务

## 许可证

MIT，见 [LICENSE](LICENSE)。
