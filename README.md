# rustdx

[<img alt="github" src="https://img.shields.io/github/license/zjp-CN/rustdx?color=blue" height="20">](https://github.com/zjp-CN/rustdx)
[<img alt="github" src="https://img.shields.io/github/issues/zjp-CN/rustdx?color=db2043" height="20">](https://github.com/zjp-CN/rustdx/issues)
[<img alt="crates.io" src="https://img.shields.io/crates/v/rustdx-complete?style=flat&color=fc8d62&logo=rust&label=rustdx-complete" height="20">](https://crates.io/crates/rustdx-complete)
[<img alt="crates.io" src="https://img.shields.io/crates/v/rustdx-complete/0.6.2?style=flat&color=green&logo=rust&logoColor=white&label=v0.6.2" height="20">](https://crates.io/crates/rustdx-complete)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-rustdx-66c2a5?style=flat&labelColor=555555&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/rustdx)
[<img alt="crates.io" src="https://img.shields.io/crates/v/rustdx-cmd?style=flat&color=fc8d62&logo=rust&label=rustdx-cmd" height="20">](https://crates.io/crates/rustdx-cmd)
[<img alt="build status" src="https://github.com/zjp-CN/rustdx/workflows/Release%20CI/badge.svg" height="20">](https://github.com/zjp-CN/rustdx/actions)

[![](https://img.shields.io/crates/d/rustdx.svg?label=downloads+rustdx&style=social)](https://crates.io/crates/rustdx)
[![](https://img.shields.io/crates/dv/rustdx.svg?label=downloads@latest+rustdx&style=social)](https://crates.io/crates/rustdx)
[![](https://img.shields.io/crates/d/rustdx-cmd.svg?label=downloads+rustdx-cmd&style=social)](https://crates.io/crates/rustdx-cmd)
[![](https://img.shields.io/crates/dv/rustdx-cmd.svg?label=downloads@latest+rustdx-cmd&style=social)](https://crates.io/crates/rustdx-cmd)

受 [pytdx](https://pypi.org/project/pytdx/1.28) 启发的 A 股数据获取工具，包含：
1. 一个 Rust 通用库 [rustdx-complete](https://crates.io/crates/rustdx-complete)；
2. 一个命令行工具 [rustdx-cmd](https://crates.io/crates/rustdx-cmd)。

## 📝 最新更新 (v0.6.4 - 2025-12-31)

> **重要功能更新**: 补充完整五档买卖盘数据，完全对标通达信实时行情协议 ✅

### 🎉 v0.6.4 重要更新

**补充 SecurityQuotes 完整五档买卖盘字段**
- ✅ 新增 bid2-5, ask2-5（买二到买五、卖二到卖五价格）
- ✅ 新增 bid2_vol-5_vol, ask2_vol-5_vol（买二到买五、卖二到卖五成交量）
- ✅ 完全对标通达信实时行情数据结构
- ✅ 与 pytdx 的 get_security_quotes 功能一致

**新增字段示例**:
```rust
pub struct QuoteData {
    // ...原有字段
    pub bid1: f64,   // 买一价
    pub ask1: f64,   // 卖一价
    pub bid1_vol: f64,  // 买一量
    pub ask1_vol: f64,  // 卖一量
    // ✨ 新增五档买卖盘（共16个新字段）
    pub bid2: f64, pub ask2: f64, pub bid2_vol: f64, pub ask2_vol: f64,
    pub bid3: f64, pub ask3: f64, pub bid3_vol: f64, pub ask3_vol: f64,
    pub bid4: f64, pub ask4: f64, pub bid4_vol: f64, pub ask4_vol: f64,
    pub bid5: f64, pub ask5: f64, pub bid5_vol: f64, pub ask5_vol: f64,
}
```

### 📝 v0.6.2 文档修复（历史版本）

### 🔧 v0.6.1 重要修复（历史版本）

**修复 SecurityQuotes 数据不完整问题**
- ✅ 调整边界检查从 100 字节改为 70 字节
- ✅ 修复单只股票解析失败的问题（0/1 → 1/1）
- ✅ 修复多只股票数据丢失的问题（3/4 → 4/4）
- ✅ 所有 14 个示例程序测试通过（100%）

**修复 SecurityQuotes 数据不完整问题**
- ✅ 调整边界检查从 100 字节改为 70 字节
- ✅ 修复单只股票解析失败的问题（0/1 → 1/1）
- ✅ 修复多只股票数据丢失的问题（3/4 → 4/4）
- ✅ 所有 14 个示例程序测试通过（100%）

### 📦 安装

```bash
# Cargo.toml
[dependencies]
rustdx-complete = "0.6.4"
```

---

## 📝 历史版本 (v0.6.0 - 2025-12-30)

**1. 修复中文编码显示问题**
- ✅ 修复 GBK 编码的中文数据显示为乱码的问题
- ✅ 股票名称、指数名称等中文数据现在能正确显示
- ✅ 使用 `encoding_rs` 库进行 GBK → UTF-8 编码转换

**2. 修复服务器连接问题**
- ✅ 优化服务器 IP 顺序，将可用的服务器移到前面
- ✅ 默认服务器 `115.238.56.198:7709` 现在能正常返回数据

**3. 修复内存安全问题**
- ✅ 移除所有 `unsafe` 的 `get_unchecked` 操作
- ✅ 添加数据边界检查，防止 panic
- ✅ 所有解析函数现在都能安全处理不完整数据

**4. 修复示例代码**
- ✅ 更新所有示例代码使用正确的 crate 名称 `rustdx_complete`
- ✅ 所有 12 个示例程序现在都能正常编译和运行

### 📦 安装

```bash
# Cargo.toml
[dependencies]
rustdx-complete = "0.6.4"
```

或使用 cargo add：

```bash
cargo add rustdx-complete
```

## rustdx 库使用

rustdx 是一个功能完整的 A 股数据获取库，完全对标 pytdx 的核心功能。

### 功能特性

| 功能 | rustdx 模块 | pytdx 对应 | 说明 |
|------|------------|-----------|------|
| 日K线 | `Kline` | `get_security_bars` | 支持多种周期（日/周/月/分钟） |
| 除权数据 | `Xdxr` | `get_xdxr` | 股票除权除息信息 |
| 实时行情 | `SecurityQuotes` | `get_security_quotes` | 股票和指数实时快照 |
| 股票列表 | `SecurityList` | `get_security_list` | 获取所有股票代码 |
| 分时数据 | `MinuteTime` | `get_minute_time_data` | 当日分时成交数据 |
| 逐笔成交 | `Transaction` | `get_transaction_data` | tick-level 成交数据 |
| 财务信息 | `FinanceInfo` | `get_finance_info` | 32个财务基本面数据 |
| 指数行情 | `SecurityQuotes` | `get_index_quotes` | 上证指数、深证成指等 |

### 安装

```toml
[dependencies]
rustdx-complete = "0.6.4"
```

### 使用示例

#### 获取股票实时行情

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::SecurityQuotes;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut tcp = Tcp::new()?;

    // 获取多只股票的实时行情
    let mut quotes = SecurityQuotes::new(vec![
        (0, "000001"),  // 平安银行（深市）
        (1, "600000"),  // 浦发银行（沪市）
    ]);

    quotes.recv_parsed(&mut tcp)?;

    for quote in quotes.result() {
        println!("{}: 当前价: {}", quote.code, quote.price);
    }

    Ok(())
}
```

#### 获取指数行情

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::SecurityQuotes;

let mut tcp = Tcp::new()?;

// 获取主要指数行情
let mut quotes = SecurityQuotes::new(vec![
    (1, "000001"),  // 上证指数
    (0, "399001"),  // 深证成指
    (1, "000300"),  // 沪深300
]);

quotes.recv_parsed(&mut tcp)?;

for quote in quotes.result() {
    println!("{}: {} (涨跌: {}%)", quote.code, quote.price, quote.change_percent);
}
```

#### 获取日线数据

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::Kline;

let mut tcp = Tcp::new()?;
let mut kline = Kline::new(1, "600000", 9, 0, 10); // 沪市、浦发银行、日线、从0开始获取10条

kline.recv_parsed(&mut tcp)?;

for bar in kline.result() {
    println!("{} : 开({}) 高({}) 低({}) 收({})",
        bar.dt, bar.open, bar.high, bar.low, bar.close);
}
```

#### 获取财务信息

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::FinanceInfo;

let mut tcp = Tcp::new()?;
let mut finance = FinanceInfo::new(0, "000001"); // 深市、平安银行

finance.recv_parsed(&mut tcp)?;

let info = &finance.result()[0];
println!("股票代码: {}", info.code);
println!("总股本: {:.0} 股", info.zongguben);
println!("净资产: {:.0} 元", info.jingzichan);
println!("净利润: {:.0} 元", info.jinglirun);
```

#### 获取分时数据

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::MinuteTime;

let mut tcp = Tcp::new()?;
let mut minute = MinuteTime::new(0, "000001"); // 深市、平安银行

minute.recv_parsed(&mut tcp)?;

for (i, data) in minute.result().iter().take(10).enumerate() {
    println!("{} : 价格={} 成交量={}", i + 1, data.price, data.vol);
}
```

#### 获取逐笔成交

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::Transaction;

let mut tcp = Tcp::new()?;
let mut transaction = Transaction::new(0, "000001", 0, 20); // 深市、平安银行、从第0条开始获取20笔

transaction.recv_parsed(&mut tcp)?;

for data in transaction.result().iter().take(5) { // 只打印前5笔
    println!("{} : 价格={} 成交量={} 买卖方向={}",
        data.time, data.price, data.vol, data.buyorsell);
}
```

### 市场代码说明

- `0` = 深市（深圳证券交易所）
- `1` = 沪市（上海证券交易所）

### 超时设置

默认 TCP 超时时间为 5 秒。如果网络环境较差，可以调整 `src/tcp/mod.rs` 中的 `TIMEOUT` 常量。

### 完整示例程序

项目 `examples/` 目录下提供了完整的使用示例：

- `test_security_quotes.rs` - 股票和指数行情
- `test_kline.rs` - K线数据
- `test_finance_info.rs` - 财务信息
- `test_minute_time.rs` - 分时数据
- `test_transaction.rs` - 逐笔成交
- `test_security_list.rs` - 股票列表

运行示例：
```bash
cargo run --example test_security_quotes
```

### 快速开始

#### 1. 创建新项目

```bash
cargo new my_stock_app
cd my_stock_app
```

#### 2. 添加依赖

在 `Cargo.toml` 中添加：

```toml
[dependencies]
rustdx-complete = "0.6.4"
```

或使用 cargo add：

```bash
cargo add rustdx-complete
```

#### 3. 编写代码

在 `src/main.rs` 中：

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::SecurityQuotes;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 连接到通达信服务器
    let mut tcp = Tcp::new()?;

    // 获取股票行情
    let mut quotes = SecurityQuotes::new(vec![
        (0, "000001"),  // 平安银行
        (1, "600000"),  // 浦发银行
    ]);

    quotes.recv_parsed(&mut tcp)?;

    // 打印结果
    for quote in quotes.result() {
        println!(
            "{}: 当前价={}, 涨跌幅={}%",
            quote.code, quote.price, quote.change_percent
        );
    }

    Ok(())
}
```

#### 4. 运行

```bash
cargo run
```

### API 文档

完整的 API 文档请访问：
- **docs.rs**: https://docs.rs/rustdx-complete
- **crates.io**: https://crates.io/crates/rustdx-complete

### 详细使用示例

#### 股票实时行情

获取多只股票的实时快照数据：

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::SecurityQuotes;

let mut tcp = Tcp::new()?;
let mut quotes = SecurityQuotes::new(vec![
    (0, "000001"),  // 平安银行（深市）
    (1, "600000"),  // 浦发银行（沪市）
    (1, "600036"),  // 招商银行（沪市）
]);

quotes.recv_parsed(&mut tcp)?;

for quote in quotes.result() {
    println!("股票代码: {}", quote.code);
    println!("当前价: {:.2}", quote.price);
    println!("昨收: {:.2}", quote.preclose);
    println!("今开: {:.2}", quote.open);
    println!("最高: {:.2}", quote.high);
    println!("最低: {:.2}", quote.low);
    println!("成交量: {:.0}", quote.vol);
    println!("成交额: {:.0}", quote.amount);
    println!("买一: {:.2} × {:.0}", quote.bid1, quote.bid1_vol);
    println!("卖一: {:.2} × {:.0}", quote.ask1, quote.ask1_vol);
    println!("买二: {:.2} × {:.0}", quote.bid2, quote.bid2_vol);
    println!("卖二: {:.2} × {:.0}", quote.ask2, quote.ask2_vol);
    println!("买三: {:.2} × {:.0}", quote.bid3, quote.bid3_vol);
    println!("卖三: {:.2} × {:.0}", quote.ask3, quote.ask3_vol);
    println!("买四: {:.2} × {:.0}", quote.bid4, quote.bid4_vol);
    println!("卖四: {:.2} × {:.0}", quote.ask4, quote.ask4_vol);
    println!("买五: {:.2} × {:.0}", quote.bid5, quote.bid5_vol);
    println!("卖五: {:.2} × {:.0}", quote.ask5, quote.ask5_vol);
    println!("涨跌幅: {:.2}%", quote.change_percent);
    println!();
}
```

**注意事项**：
- 建议一次查询不超过 80 只股票
- 市场代码：0=深市，1=沪市
- 数据为实时快照，包括五档买卖盘

#### 指数行情

获取主要指数的实时行情：

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::SecurityQuotes;

let mut tcp = Tcp::new()?;

// 获取主要指数
let mut quotes = SecurityQuotes::new(vec![
    (1, "000001"),  // 上证指数
    (0, "399001"),  // 深证成指
    (1, "000300"),  // 沪深300
    (0, "399006"),  // 创业板指
]);

quotes.recv_parsed(&mut tcp)?;

println!("📊 主要指数行情：");
for quote in quotes.result() {
    println!(
        "{}: {:.2} ({:+.2}%)",
        quote.code, quote.price, quote.change_percent
    );
}
```

**常用指数代码**：
- 上证指数：`000001` (market=1)
- 深证成指：`399001` (market=0)
- 沪深300：`000300` (market=1)
- 创业板指：`399006` (market=0)
- 中证500：`000905` (market=1)
- 科创50：`000688` (market=1)

#### K线数据

获取日K线、周K线、月K线等：

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::Kline;

let mut tcp = Tcp::new()?;

// Kline参数：market, code, category, start, count
// category: 9=日K(新)
let mut kline = Kline::new(1, "600000", 9, 0, 10);  // 沪市、浦发银行、日线、从0开始获取10条

kline.recv_parsed(&mut tcp)?;

println!("浦发银行最近10日K线：");
for bar in kline.result() {
    println!(
        "{:?}: 开={:.2} 高={:.2} 低={:.2} 收={:.2} 量={:.0}",
        bar.dt, bar.open, bar.high, bar.low, bar.close, bar.vol
    );
}
```

**K线周期说明**：
- `category = 9`: 日K线（推荐）
- `category = 5`: 5分钟K线
- `category = 6`: 15分钟K线
- `category = 7`: 30分钟K线
- `category = 8`: 1小时K线
- `category = 10`: 周K线
- `category = 11`: 月K线

#### 财务信息

获取股票的财务基本面数据：

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::FinanceInfo;

let mut tcp = Tcp::new()?;
let mut finance = FinanceInfo::new(0, "000001");  // 平安银行

finance.recv_parsed(&mut tcp)?;

let info = &finance.result()[0];

println!("📊 {} 财务数据：", info.code);
println!("上市日期: {}", info.ipo_date);
println!("总股本: {:.0} 股 ({:.2} 亿股)", info.zongguben, info.zongguben / 1_0000_0000.0);
println!("流通股: {:.0} 股 ({:.2} 亿股)", info.liutongguben, info.liutongguben / 1_0000_0000.0);
println!("总资产: {:.2} 亿元", info.zongzichan / 1_0000_0000.0);
println!("净资产: {:.2} 亿元", info.jingzichan / 1_0000_0000.0);
println!("净利润: {:.2} 亿元", info.jinglirun / 1_0000_0000.0);
println!("主营收入: {:.2} 亿元", info.zhuyingshouru / 1_0000_0000.0);
```

**财务字段说明**：
- `zongguben`: 总股本（股）
- `liutongguben`: 流通股本（股）
- `zongzichan`: 总资产（元）
- `jingzichan`: 净资产（元）
- `jinglirun`: 净利润（元）
- `zhuyingshouru`: 主营收入（元）
- `jingyingxianjinliu`: 经营现金流（元）

#### 分时数据

获取当日分时成交数据（240个数据点）：

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::MinuteTime;

let mut tcp = Tcp::new()?;
let mut minute = MinuteTime::new(0, "000001");  // 平安银行

minute.recv_parsed(&mut tcp)?;

println!("平安银行分时数据（前10条）：");
for (i, data) in minute.result().iter().take(10).enumerate() {
    println!(
        "{}: 价格={:.2} 成交量={:.0}",
        i + 1, data.price, data.vol
    );
}

println!("...");
println!("总计: {} 条数据", minute.result().len());
```

**数据说明**：
- 每个交易日产生 240 条分时数据
- 时间范围：9:30-15:00
- 时间格式：HH:MM
- 成交量单位：手

#### 逐笔成交

获取 tick 级别的逐笔成交数据：

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::Transaction;

let mut tcp = Tcp::new()?;
let mut transaction = Transaction::new(0, "000001", 0, 20);  // 从第0条开始获取20笔

transaction.recv_parsed(&mut tcp)?;

println!("平安银行逐笔成交（最近5笔）：");
for data in transaction.result().iter().take(5) {
    let direction = match data.buyorsell {
        0 => "买入",
        1 => "卖出",
        _ => "中性",
    };

    println!(
        "{}: 价格={:.2} 数量={:.0}手 方向={}",
        data.time, data.price, data.vol, direction
    );
}

println!("...");
// 安全地获取最后一笔成交
if let Some(last) = transaction.result().last() {
    println!("最新成交序号: {}", last.num);
}
```

**买卖方向说明**：
- `0`: 买入（主动买）
- `1`: 卖出（主动卖）
- `8`: 中性（未知）

#### 股票列表

获取所有股票代码和名称：

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::SecurityList;

let mut tcp = Tcp::new()?;

// 第一次获取：从0开始，获取1000只股票
let mut list = SecurityList::new(0, 0);  // market=0(深市), start=0

list.recv_parsed(&mut tcp)?;

println!("股票列表（前10只）：");
for (i, stock) in list.result().iter().take(10).enumerate() {
    println!(
        "{}: 代码={}, 名称={}",
        i + 1, stock.code, stock.name
    );
}

println!("...");
println!("本批次获取: {} 只股票", list.result().len());
```

**分页说明**：
- 每次最多获取 1000 只股票
- 参数1：`market` (0=深市, 1=沪市)
- 参数2：`start` (0, 1000, 2000, ...) - 起始位置
- 示例：`SecurityList::new(0, 0)` - 深市从0开始获取
- 示例：`SecurityList::new(0, 1000)` - 深市从1000开始获取

### 错误处理

所有 TCP 连接和数据获取都可能失败，建议使用错误处理：

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::SecurityQuotes;

fn main() {
    // 尝试连接
    let mut tcp = match Tcp::new() {
        Ok(tcp) => {
            println!("✅ 连接成功");
            tcp
        }
        Err(e) => {
            eprintln!("❌ 连接失败: {}", e);
            // 尝试其他服务器或退出
            return;
        }
    };

    // 获取数据
    let mut quotes = SecurityQuotes::new(vec![(0, "000001")]);

    match quotes.recv_parsed(&mut tcp) {
        Ok(_) => {
            // 处理数据
            for quote in quotes.result() {
                println!("{}: {}", quote.code, quote.price);
            }
        }
        Err(e) => {
            eprintln!("❌ 获取数据失败: {}", e);
        }
    }
}
```

### 性能优化建议

1. **复用 TCP 连接**
```rust
let mut tcp = Tcp::new()?;

// 获取多种数据
let mut quotes = SecurityQuotes::new(vec![(0, "000001")]);
quotes.recv_parsed(&mut tcp)?;

let mut kline = Kline::new(0, "000001", 9, 0, 10);
kline.recv_parsed(&mut tcp)?;
```

2. **批量查询**
```rust
// 一次查询多只股票，而不是多次查询单只股票
let mut quotes = SecurityQuotes::new(vec![
    (0, "000001"), (0, "000002"), (0, "000003"),
    // ... 最多80只
]);
```

3. **使用 release 模式**
```bash
cargo run --release  # 性能提升明显
```

### 常见问题

#### Q: 为什么连接超时？
A: 默认超时时间为 5 秒。如果网络环境较差，可以修改 `src/tcp/mod.rs` 中的 `TIMEOUT` 常量。

#### Q: 数据更新频率？
A: 实时行情数据来自通达信服务器，交易时间内实时更新。

#### Q: 支持港股和美股吗？
A: 目前仅支持 A 股（沪深两市）。

#### Q: 如何获取历史数据？
A: 使用 `Kline` 模块获取历史 K 线数据，或使用 `rustdx-cmd` 工具解析通达信数据文件。

#### Q: 数据准确吗？
A: 数据来自通达信官方服务器，经过验证准确可靠。

### 示例程序

项目 `examples/` 目录包含完整的示例程序：

| 示例程序 | 功能描述 |
|---------|---------|
| `test_security_quotes.rs` | 股票和指数实时行情 |
| `test_finance_info.rs` | 财务信息查询 |
| `test_transaction.rs` | 逐笔成交数据 |
| `test_minute_time.rs` | 分时数据 |
| `test_security_list.rs` | 股票列表 |
| `test_index_quotes.rs` | 指数行情 |

运行示例：
```bash
cargo run --example test_security_quotes
cargo run --example test_finance_info
cargo run --example test_transaction
```

### 贡献指南

欢迎贡献代码、报告问题或提出建议！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

### 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

### 致谢

- [pytdx](https://pypi.org/project/pytdx/) - Python 版本的通达信接口
- 通达信 - 提供数据服务器

---

命令行工具（统计数据基于笔者的单核 CPU Ubuntu 系统 release build，以实际速度为准）：
1. 解析所有最新股票列表的历史 A 股数据（包含复权数据）不到 30s ，解析后的 csv 大小 1G 多；
2. 将解析后的 csv 数据插入到 ClickHouse （20s，表 268 M） 或 MongoDB （7 分钟，表超过 700 M）；
3. 东财日线增量更新（包括复权），2s 更新完。

关于复权：
1. 使用涨跌幅复权算法，无需修改（重算）历史复权信息；
2. 只计算收盘价前复权，其他价格复权只需基于收盘价和相对价格即可计算出来（这在 ClickHouse 中很快）。

具体文档待补充。

## rustdx-cmd

### 安装

使用以下一种方式即可：

1. 下载 [已编译的 release 版本](https://github.com/zjp-CN/rustdx/releases/latest)

2. cargo install：
```console
cargo install rustdx-cmd
```

3. cargo build：
```console
$ git clone https://github.com/zjp-CN/rustdx.git
$ cd rustdx
$ cargo build -p rustdx-cmd --release # 编译（二进制在 target/release 下）
$ cargo install --path rustdx-cmd     # 安装（二进制在全局 .cargo/bin 下）
```

### 子命令

- day：解析通达信 day 文件，具体查看帮助 `rustdx day --help`、`rustdx day -h o -h l`。
- east：获取东方财富当日 A 股数据，具体查看帮助 `rustdx east --help`。

### 完整使用例子

准备好 day 文件、gbbq 文件和 ClickHouse 数据库：

p.s. 请勿使用本项目 `assets/` 中的 gbbq 文件，因为那对你来说是过时的。

> 注意：
>
> 此工具的主要目的就是快速补齐历史日线数据，但**没有**校验交易日数据连续或者清空数据库的功能。
>
> 因没有每天记录日线导致日线不完整（或者其他原因导致数据有问题），请**重新**解析和存储所有历史数据。
>
> 重新存储数据之前，使用以下 sql 命令（以 ClickHouse 为例）删除历史数据：
>
> ```sql
> TRUNCATE TABLE rustdx.factor;
> ```
>
> 如果发现历史数据不正确，请提交 [issue](https://github.com/zjp-CN/rustdx/issues)。

```console
# 解析所有最新股票的历史日线数据，且计算复权数据
$ rustdx day /vdb/tmp/tdx/sh/ /vdb/tmp/tdx/sz/ -l official -g ../assets/gbbq -t rustdx.factor
# 写入 ClickHouse 数据库
$ clickhouse-client --query "INSERT INTO rustdx.factor FORMAT CSVWithNames" < stocks.csv

# 有了历史日线数据之后，每个交易日收盘之后，更新当天数据
$ rustdx east -p factor.csv -t rustdx.factor
# 写入 ClickHouse 数据库
$ clickhouse-client --query "INSERT INTO rustdx.factor FORMAT CSVWithNames" < eastmoney.csv
```

其中 factor.csv 来自数据库中，前一天的复权数据，ClickHouse 的导出命令：
```sql
SELECT
    yesterday() AS date,
    code,
    last_value(close) AS close,
    last_value(factor) AS factor
FROM rustdx.factor
GROUP BY code
INTO OUTFILE 'factor.csv'
FORMAT CSVWithNames;
```

---

或者：
```console
# 解析所有最新股票的历史日线数据，且计算复权数据，写入 ClickHouse 数据库
$ rustdx day /vdb/tmp/tdx/sh/ /vdb/tmp/tdx/sz/ -l official -g ../assets/gbbq -o clickhouse -t rustdx.factor

# 有了历史日线数据之后，每个交易日收盘之后，更新当天数据
$ rustdx east -p clickhouse -o clickhouse -t rustdx.factor
```

## CHANGELOG

[更新记录](https://github.com/zjp-CN/rustdx/blob/main/CHANGELOG.md)

## 使用示例

### 计算任何周期的涨跌幅

```sql
SELECT
    code,
    toYYYYMM(date) AS m, -- 这里以月周期为例
    ((LAST_VALUE(factor) / FIRST_VALUE(factor)) * FIRST_VALUE(close)) / FIRST_VALUE(preclose) AS mgrowth
FROM rustdx.factor -- 命令行参数中所写入的表名，假设你按照我上面给的命令行示例运行，那么原始数据在这个表
GROUP BY code, m   -- 按照月聚合
ORDER BY code ASC, m DESC;
```

为什么 `mgrowth` 是那样计算，见 [涨跌幅复权与前复权](https://zjp-cn.github.io/posts/qfq/)。

### 计算前复权价格

注意，上面计算涨幅时没有计算前复权价格，但大部分情况下必须知道前复权价格来计算价格相关的指标。

那么可以每日数据成功入库之后，运行一次以下脚本，注意：
* 这基于最新价来计算所有股票的所有历史前复权价格（在我的单核机器上需要 11 秒）
* 每次运行脚本会把之前的计算结果清空
* 前复权的结果在 `rustdx.qfq` 这个表（只有股票代码和价格）

```sql
-- 计算前复权价格
DROP TABLE IF EXISTS rustdx.qfq_x; -- 临时表
CREATE TABLE rustdx.qfq_x (
    code FixedString(6),
    x    Float64,
    PRIMARY KEY(code)
) ENGINE = MergeTree  AS 
WITH
qfq AS (
    SELECT code, LAST_VALUE(close) / LAST_VALUE(factor) AS qfq_multi
    FROM rustdx.factor
    GROUP BY code
    ORDER BY code
)
SELECT * FROM qfq;

DROP TABLE IF EXISTS rustdx.qfq; -- 前复权价格
CREATE TABLE rustdx.qfq (
    date  Date,
    code  FixedString(6),
    close Float64,
    open  Float64,
    high  Float64,
    low   Float64,
    PRIMARY KEY(date, code)
) ENGINE = MergeTree AS
WITH
qfq_x AS (SELECT * FROM rustdx.qfq_x),
fct AS (
    SELECT date, code, open/close AS open, high/close AS high, low/close AS low, factor
    FROM rustdx.factor
),
raw AS (
    SELECT *
    FROM fct
    LEFT JOIN qfq_x ON qfq_x.code = fct.code
)
SELECT date, code, factor*x AS close, open*close AS open, high*close AS high, low*close AS low
FROM raw
ORDER BY date, code
```
