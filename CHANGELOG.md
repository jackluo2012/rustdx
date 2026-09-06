# Changelog

## [Unreleased]

## v1.3.0 (2026-09-06)

### 🚀 rustdx-cli：day 命令自动下载（一键解析日线完整包）

- `rustdx day`（不带任何目录参数）：自动从通达信官网
  <https://data.tdx.com.cn/vipdoc/hsjday.zip> 下载日线完整包 → 解压 → 解析
- `rustdx day <url>`：从指定地址下载；`rustdx day <xx.zip>`：解压本地 zip
- 下载失败时交互询问备选下载地址（URL）或本地 zip 文件路径
- 解压支持通达信 zip 的 Windows 反斜杠条目名（归一化为 `/`），带 zip-slip
  路径穿越安全检查；下载/解压均在系统临时目录进行
- 原有目录解析模式完全不变（实测 `rustdx day <dir>` 行为一致）
- 实测：真实下载 522.9 MiB（约 84 秒）→ 解压 12,394 文件 → 解析全流程通过

### 🔄 当日分时自动回退（客户端自动生效，实盘待确认）

- `Client::minute` 当日分时接口解析为空（2026 新协议不匹配）时，自动改用
  **今日历史分时接口**（`HistoryMinuteTime` 查询当天）取数；历史分时接口协议稳定
  （2026-09-04 实测 240 点完整）。非交易日返回空数据（无当日分时）。
- 端到端（当日分时回退后拿到完整分时）需下个交易日实盘确认，协议已按
  pytdx#148 / xmtdx 同类方案实现。

## v1.2.0 (2026-09-05)

### 🚀 day 文件解析改进：带字母代码 + 市场前缀 + 并发（Phase 2b 全量数据实测驱动）

用通达信官网「沪深京日线完整包」全量实测（12,394 个 `.day`，A股/B股/指数/基金/可转债混合）
驱动出两处程序侧问题并修复：

**① 支持带字母的特殊品种代码**
- 通达信官方数据包含深市板块指数等带字母代码文件（如 `sz200b07`），旧版 `filter_ec`
  用 `u32::parse` 只认纯数字代码而被过滤跳过；现改为字符串代码，**不再跳过**（实测该文件 913 条完整解析）
- 顺带消除 gbbq 分组里的 `code.parse().unwrap()` panic 隐患（改用字符串分组）

**② 合并多市场输出不再代码混叠（破坏性变更）**
- 旧版 CSV 只输出 6 位裸代码，合并 sh/sz/bj 时 `sh000001`（上证指数）与
  `sz000001`（平安银行）混在同一 code 下；现 `Day.code` 从 `u32` 改为
  **含市场前缀的字符串**（`sh600000` / `sz000001` / `sz200b07`），输出可区分
- 受影响：`Day::code` 字段类型、`Day::from_bytes`/`from_file_into_vec` 签名、
  `Fq`/`Gbbq::filter_hashmap` key 类型、clickhouse 表 `code` 列（`FixedString(6)` → `String`）、
  复权前收 factor CSV 的 code 列格式

**③ 并发解析（利用多核优势）**
- `rustdx-cli day` 三个解析路径（普通/复权/增量复权）由单核串行改为
  `std::thread::scope` + 原子任务队列 + channel 的多线程并行（线程数 = 逻辑核心数，
  主线程顺序写 CSV）
- 全市场日线实测：约 3 分钟（串行）→ **1 分 48 秒**（并发）

**测试**
- `tests/file-day.rs` / `tests/fq.rs` 快照更新为带前缀 code（`sz000001`）
- 全量 12,394 文件解析：**12,394/12,394** 通过（旧版 12,391/12,394），
  空文件（数据源本身为空）正确无输出行

## v1.1.1 (2026-09-05)

### 🛡️ 日线 K 线解析与顺序健壮性（Phase 2a 实测驱动）

**解析防御（`Kline::parse`，两处一处修）**
- ✅ 服务器返回记录数少于请求数（历史尽头 / 长期停牌）：旧版 `debug_assert_eq!` panic、release 下越界读垃圾数据；现截断到实际可解析的记录数
- ✅ 响应体被截断（代理环境实测出现仅 2 字节包头的响应）：逐字段安全读取（价格 varint 不定长，无法按步长预算），不足一条即止，空 body 返回空数据——守住「解析异常时返回空数据而非垃圾数据」约定

**`Client::k` 顺序无关化**
- ✅ 2026-09 实测部分服务器 `bars` 返回顺序与早前相反（与「当日分时协议变更」疑同期调整），旧版 `page.reverse()` 的顺序假设失效导致升序契约被破坏；现按「页内最小日期判停 + 末端升序排序」实现，不再依赖页内顺序

**背景**：short-mind-os Phase 2a 全市场日线/分钟线回补（5222 只 × 多日并发）实测触发以上三处；fork 侧 197 测试与 clippy -D warnings 全绿。

## v1.1.0 (2026-09-04)

### 🎉 对标 mootdx 补齐核心功能 + 行情解析全面修复

**数据正确性（SecurityQuotes 行情，pytdx 同连接抓包逐字段验证）**
- ✅ 昨收/开/高/低/五档价格按 `(当前价 + 差值)/100` 还原（旧版输出差值或 0）
- ✅ 涨跌幅真实计算（旧版输出 655% 乱码），涨速字段改有符号 i16
- ✅ 成交量单位修正为手（旧版多除 100）；新增现量/外盘/内盘/服务器时间字段
- ✅ 破解服务器时间编码（HHMM + 秒×10000/60 十进制拼接）
- ✅ 真实抓包字节的回归测试，协议变化可第一时间发现

**新功能（全部经交易时段实盘验证）**
- ✅ `IndexKline` 指数K线（含上涨/下跌家数）
- ✅ `HistoryMinuteTime` 历史分时（实测 240 点完整）
- ✅ `HistoryTransaction` 历史逐笔成交
- ✅ `get_block_info` 板块文件：逆向出固定槽位布局，实测 269 个概念板块
  41147 条成分记录，**取代旧版硬编码且张冠李戴的概念数据**
- ✅ `CompanyInfoCategory/Content` F10 公司资料
- ✅ `Client` 高层 API（对标 mootdx StdQuotes）+ `k()` 日期区间拉K线
- ✅ `market_of` 代码自动推断市场、`stocks` 全量列表自动分页

**连接可靠性（对标 mootdx bestip/heartbeat/auto_retry）**
- ✅ 服务器列表更新为 mootdx 维护的 40 台（原 19 台仅 2 台存活，新列表 40/40 可连）
- ✅ 协议级故障转移（TCP 可连≠行情可用）、`TcpConfig` 超时与指定服务器
- ✅ `Tcp::heartbeat` 心跳保活、`Tcp::retry` 自动重连重试、`Tcp::reconnect` 重连
- ✅ 修复 16 字节响应头单次读取隐患、财务信息边界检查（旧版会 panic）

**Bug 修复**
- ✅ `Xdxr` 请求字节布局修复（market 写错位置导致任意股票返回 0 条，实测 80 条）
- ✅ 分时防御性解析：协议异常返回空数据而非垃圾，消除 panic 风险
- ✅ rustdx-cmd 编译修复（`day`/`east` 恢复可用）、东财请求自动重试
- ✅ `TradingCalendar::is_trading_time` 时区偏差

**依赖升级与现代化**
- ✅ lazy_static 移除（标准库 const 构造）、thiserror 2、miniz_oxide 0.9
- ✅ ureq 3、calamine 0.36、csv 1.4、tabled 0.21、subprocess 移除（std::process）
- ✅ edition 2024、移除 eprintln 副作用、统一错误处理

**文档与工程**
- ✅ README 全面重写（功能对照表、真实示例、已知问题诚实声明）
- ✅ 197 个测试全部通过、clippy 零警告
- ✅ 仓库整理：14 个过时文档归档 docs/archive/，pytdx 对照脚本入 scripts/
- ✅ CLI 更名发布：`rustdx-cmd` → `rustdx-cli`（crates.io 上 `rustdx-cmd` 归属上游 zjp-CN，改用自有命名空间发布；二进制名仍为 `rustdx`）

## v1.0.1 (2026-09-04)

### 🔧 构建与发布基线修复

**恢复模块接线（与 crates.io 1.0.0 发布物对齐）**
- ✅ `indicators` / `cache` / `error` / `calendar` / `builder` / `pool` 六个模块在 `lib.rs` 中重新声明并导出
- ✅ `tcp::stock::validator` 数据验证模块重新接线
- ✅ 新增 `trade_date_a` 依赖（交易日历数据源，升级至 2026.0）

**修复 rustdx-cmd 编译错误**
- ✅ 修正 `rustdx_complete_cmd` → `rustdx_cmd`、`rustdx` → `rustdx_complete` 的 crate 引用
- ✅ 修复 `eastmoney::get` 调用签名（补充分页参数）
- ✅ `rustdx day` / `rustdx east` 子命令恢复可用

**质量基线**
- ✅ `cargo build --workspace --all-targets` / `cargo test` / `cargo clippy -- -D warnings` 全部通过
- ✅ 清理全部编译警告与 clippy 告警（约 50 处）
- ✅ 修复 `TradingCalendar::is_trading_time` 的时区换算偏差（改用 NaiveDateTime 的时间部分）
- ✅ 三处版本号统一（Cargo.toml / rustdx-cmd / tests-integration）

## v1.0.0 (2026-01-06)

> 已发布至 crates.io（rustdx-complete 1.0.0）。包含数据验证（validator）、技术指标（indicators）、
> 智能缓存（cache）、交易日历（calendar）、Builder 模式 API、连接池（pool）、分层错误类型（error）。
> 注：此版本的完整源码此前未完整提交至 git 仓库，v1.0.1 已从 crates.io 发布物恢复对齐。

## v0.6.6 (2025-12-31)

### 🎉 重大更新 - 股票行业分类和概念板块查询功能

**新增股票行业分类映射模块 (industry_mapping)**
- ✅ 通达信行业代码自动映射到中文名称（银行、证券、酒类等）
- ✅ 省份代码自动映射（深圳、贵州、四川等）
- ✅ 行业大类分类（金融、消费、科技、材料、能源等）
- ✅ 支持主流行业 40+ 个分类
- ✅ 提供 3 个核心函数：`get_industry_name()`, `get_province_name()`, `get_industry_info()`

**新增东方财富概念板块映射模块 (concept_mapping)**
- ✅ 新能源汽车、锂电池、芯片、人工智能等 10+ 热门概念
- ✅ 每个概念提供成分股列表（前20只）
- ✅ 与通达信行业分类形成互补，全面分析股票特征
- ✅ 提供 Python 脚本 `generate_concept_mapping.py` 自动生成映射数据
- ✅ 提供 3 个核心函数：`get_concept_stocks()`, `get_concept_names()`, `get_concept_info()`

**双数据源综合应用**
- ✅ 通达信：基本面分析、行业分类、实时行情
- ✅ 东方财富：市场热点、概念板块、主题投资
- ✅ 两者结合可进行板块轮动、股票筛选、投资组合分析

**新增示例程序**
- `test_industry_info` - 行业信息查询示例（8个测试股票）
- `test_concept_query` - 概念板块查询示例
- `test_combined_info` - 双数据源综合应用示例

**新增文档**
- `INDUSTRY_MAPPING.md` - 行业分类使用指南
- `CONCEPT_STOCK.md` - 概念板块使用指南

**技术细节**
- 新增模块：2个（industry_mapping, concept_mapping）
- 新增示例程序：3个
- 新增文档：2个
- 代码行数：约 +800 行
- 所有测试通过：37/37 ✅

**使用示例**
```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::{FinanceInfo, get_industry_name, get_concept_stocks};

// 获取股票的行业信息
let mut tcp = Tcp::new()?;
let mut finance = FinanceInfo::new(1, "600519");
finance.recv_parsed(&mut tcp)?;
let info = &finance.result()[0];

println!("行业: {}", get_industry_name(info.industry));  // 酒类

// 查询热门概念板块成分股
if let Some(stocks) = get_concept_stocks("新能源汽车") {
    // 查询成分股...
}
```

### 💡 升级建议

**所有用户强烈建议升级到 v0.6.6**

本次更新新增了行业分类和概念板块查询功能，让rustdx从纯数据获取工具升级为具备基本面分析和热点追踪能力的综合平台。

---

## v0.6.4 (2025-12-31)

### 🎉 重要更新 - 完整五档买卖盘数据

**补充 SecurityQuotes 完整五档买卖盘字段**
- ✅ 新增 bid2-5, ask2-5（买二到买五、卖二到卖五价格）
- ✅ 新增 bid2_vol-5_vol, ask2_vol-5_vol（买二到买五、卖二到卖五成交量）
- ✅ 完全对标通达信实时行情数据结构
- ✅ 与 pytdx 的 get_security_quotes 功能一致

**新增字段（共20个字段）**
```rust
pub struct QuoteData {
    // 原有字段...
    /// 买一价
    pub bid1: f64,
    /// 卖一价
    pub ask1: f64,
    /// 买一量（手）
    pub bid1_vol: f64,
    /// 卖一量（手）
    pub ask1_vol: f64,
    // ✨ 新增字段
    /// 买二价
    pub bid2: f64,
    /// 卖二价
    pub ask2: f64,
    /// 买二量（手）
    pub bid2_vol: f64,
    /// 卖二量（手）
    pub ask2_vol: f64,
    /// 买三价
    pub bid3: f64,
    /// 卖三价
    pub ask3: f64,
    /// 买三量（手）
    pub bid3_vol: f64,
    /// 卖三量（手）
    pub ask3_vol: f64,
    /// 买四价
    pub bid4: f64,
    /// 卖四价
    pub ask4: f64,
    /// 买四量（手）
    pub bid4_vol: f64,
    /// 卖四量（手）
    pub ask4_vol: f64,
    /// 买五价
    pub bid5: f64,
    /// 卖五价
    pub ask5: f64,
    /// 买五量（手）
    pub bid5_vol: f64,
    /// 卖五量（手）
    pub ask5_vol: f64,
    // ...其他字段
}
```

### 📝 使用示例

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::quotes::SecurityQuotes;

let mut tcp = Tcp::new()?;
let mut quotes = SecurityQuotes::new(vec![
    (0, "000001"),  // 平安银行
    (1, "600000"),  // 浦发银行
]);

quotes.recv_parsed(&mut tcp)?;

for quote in quotes.result() {
    println!("股票: {}", quote.code);
    println!("当前价: {}", quote.price);
    println!("买一: {} ({})  卖一: {} ({})",
        quote.bid1, quote.bid1_vol, quote.ask1, quote.ask1_vol);
    println!("买二: {} ({})  卖二: {} ({})",
        quote.bid2, quote.bid2_vol, quote.ask2, quote.ask2_vol);
    println!("买三: {} ({})  卖三: {} ({})",
        quote.bid3, quote.bid3_vol, quote.ask3, quote.ask3_vol);
    println!("买四: {} ({})  卖四: {} ({})",
        quote.bid4, quote.bid4_vol, quote.ask4, quote.ask4_vol);
    println!("买五: {} ({})  卖五: {} ({})",
        quote.bid5, quote.bid5_vol, quote.ask5, quote.ask5_vol);
}
```

### 💡 技术细节

- ✅ 代码中已解析五档买卖盘数据，但未添加到 QuoteData 结构体
- ✅ 现在完全暴露所有五档买卖盘字段给用户
- ✅ 保持向后兼容，原有字段不变
- ✅ 完全对标通达信协议规范

## v0.6.3 (2025-12-31)

### 📝 文档更新

**统一 README 版本号**
- ✅ 将所有依赖版本号从 "0.6" 统一更新为 "0.6.2"
- ✅ 移除版本注释中的旧版本号引用
- ✅ 确保文档与已发布版本一致

**更新位置**
- 最新更新章节安装说明
- 历史版本章节安装说明
- 核心功能章节安装说明
- 快速开始章节安装说明

## v0.6.2 (2025-12-31)

### 📝 文档修复

**修复 README.md 中的所有代码示例错误**
- ✅ 修复版本号：0.5 → 0.6
- ✅ 修复 `MinuteTime::new()` 参数：3个 → 2个
- ✅ 修复 `Transaction::new()` 参数：3个 → 4个（添加 count 参数）
- ✅ 修复 `SecurityList::new()` 参数：1个 → 2个（添加 start 参数）
- ✅ 移除不存在的字段：`QuoteData.name`、`MinuteTimeData.time`、`SecurityListData.market`
- ✅ 修复 `DateTime` Display：使用 `{:?}` 格式化
- ✅ 修复 unwrap panic：使用安全的 `if let Some()` 模式
- ✅ 更新所有 API 参数说明和注释

**测试验证**
- ✅ 所有 7 个核心功能测试通过
- ✅ 所有 README 示例代码可运行
- ✅ 添加 `README_ISSUES.md` 详细记录所有问题

### 💡 升级建议

**所有用户强烈建议升级到 v0.6.2**

本次修复了 README 文档中的所有错误，确保用户能够正确使用库的所有功能。



## v0.6.0 (2025-12-30)

### 🎉 重要更新

这是一个重要的修复和改进版本，**强烈建议所有用户升级**！

### 🔧 重要修复

**1. 修复中文编码显示问题**
- ✅ 修复 GBK 编码的中文数据显示为乱码的问题
- ✅ 股票名称、指数名称等中文数据现在能正确显示
- ✅ 使用 `encoding_rs` 库进行 GBK → UTF-8 编码转换

**2. 修复服务器连接问题**
- ✅ 优化服务器 IP 顺序，将可用的服务器移到前面
- ✅ 默认服务器 `115.238.56.198:7709` 现在能正常返回数据
- ✅ 解决所有示例程序返回空数据的问题

**3. 修复内存安全问题**
- ✅ 移除所有 `unsafe` 的 `get_unchecked` 操作
- ✅ 添加数据边界检查，防止 panic
- ✅ 所有解析函数现在都能安全处理不完整数据

**4. 修复示例代码**
- ✅ 更新所有示例代码使用正确的 crate 名称 `rustdx_complete`
- ✅ 所有 10+ 个示例程序现在都能正常编译和运行
- ✅ 添加中文编码测试示例

### 🚀 新增功能

- 添加 GBK 解码辅助函数：`gbk_to_string()` 和 `gbk_to_string_trim_null()`
- 改进错误提示，在数据不完整时显示清晰的警告信息

### 📝 文档更新

- 更新 README.md，添加最新更新日志
- 创建 FIXES.md 详细记录所有修复
- 创建 RELEASE.md 发布指南

### ⚠️ 破坏性变更

**本次更新完全向后兼容**，不会破坏现有代码。

### 📊 测试

- 所有库测试通过（32 tests passed）
- 所有示例程序测试通过
- 中文编码显示验证通过

### 🔗 相关链接

- GitHub: https://github.com/jackluo2012/rustdx
- crates.io: https://crates.io/crates/rustdx-complete

---

## v0.5.0 (2025-12-27)

### 新增功能 (Features)

#### 核心功能模块（完全对标 pytdx）

1. **SecurityQuotes** - 实时行情数据
   - ✅ 支持股票实时行情（`get_security_quotes`）
   - ✅ 支持指数实时行情（上证指数、深证成指、沪深300等）
   - ✅ 可同时获取多只股票/指数的行情快照
   - ✅ 返回字段：当前价、今开、最高、最低、成交量、成交额、买卖五档等

2. **FinanceInfo** - 财务信息（32个财务字段）
   - ✅ 基本信息：股票代码、上市日期、更新日期、所属省份、所属行业
   - ✅ 股本结构：总股本、流通股本、国家股、法人股、B股、H股、职工股
   - ✅ 资产负债：总资产、流动资产、固定资产、无形资产、净资产
   - ✅ 利润表：主营收入、主营利润、营业利润、净利润
   - ✅ 现金流：经营现金流、总现金流
   - ✅ 对应 pytdx 的 `get_finance_info`

3. **Transaction** - 逐笔成交数据
   - ✅ tick-level 成交数据
   - ✅ 返回字段：时间、价格、成交量、成交号、买卖方向
   - ✅ 支持分页获取历史逐笔数据
   - ✅ 对应 pytdx 的 `get_transaction_data`

4. **MinuteTime** - 分时数据
   - ✅ 当日分时成交数据（240个数据点）
   - ✅ 返回字段：时间(HH:MM)、价格、成交量
   - ✅ 对应 pytdx 的 `get_minute_time_data`

5. **SecurityList** - 股票列表
   - ✅ 获取所有股票代码和名称
   - ✅ 支持分页查询（每次1000只）
   - ✅ 对应 pytdx 的 `get_security_list`

6. **IndexQuotes** - 指数行情
   - ✅ 上证指数(000001)、深证成指(399001)、沪深300(000300)等
   - ✅ 由 SecurityQuotes 模块统一支持
   - ✅ 对应 pytdx 的指数行情功能

#### 示例程序和测试

- 新增 `test_security_quotes.rs` - 股票和指数行情示例
- 新增 `test_finance_info.rs` - 财务信息示例
- 新增 `test_transaction.rs` - 逐笔成交示例
- 新增 `test_minute_time.rs` - 分时数据示例
- 新增 `test_security_list.rs` - 股票列表示例
- 新增 `test_index_quotes.rs` - 指数行情示例

### Bug 修复

#### 关键Bug修复：SecurityQuotes 发送长度问题

- **问题描述**：`send()` 方法返回整个582字节缓冲区而非实际需要长度，导致所有 SecurityQuotes 调用失败（"failed to fill whole buffer"）
- **影响范围**：影响所有使用 SecurityQuotes 的功能（股票和指数行情）
- **修复方案**：重写 `send()` 方法，只返回实际需要的字节数（22 + stocks.len() * 7）
- **验证结果**：
  - 单元测试：29/29 通过
  - 实际数据验证：股票、指数行情全部正常

### 文档更新

- README.md 新增"rustdx 库使用"章节，包含：
  - 8大核心功能对照表
  - 6个详细使用示例（股票行情、指数行情、K线、财务、分时、逐笔）
  - 市场代码说明
  - 超时设置说明
  - 完整示例程序列表

### 测试验证

- 单元测试：29/29 通过
- 功能验证：
  - 上证指数(000001): 3963.68 (+0.02%) ✅
  - 深证成指(399001): 13603.89 (+0.01%) ✅
  - 沪深300(000300): 4657.24 (+0.00%) ✅
  - 平安银行财务数据：32个字段全部获取 ✅
  - 逐笔成交数据：正常解析 ✅
  - 分时数据：240个数据点 ✅

### pytdx 功能完整性

rustdx 现已**完全实现** pytdx 的核心功能：

| 功能 | rustdx 模块 | pytdx 对应 | 状态 |
|------|------------|-----------|------|
| 日K线 | `Kline` | `get_security_bars` | ✅ |
| 除权数据 | `Xdxr` | `get_xdxr` | ✅ |
| 股票行情 | `SecurityQuotes` | `get_security_quotes` | ✅ |
| 股票列表 | `SecurityList` | `get_security_list` | ✅ |
| 分时数据 | `MinuteTime` | `get_minute_time_data` | ✅ |
| 逐笔成交 | `Transaction` | `get_transaction_data` | ✅ |
| 财务信息 | `FinanceInfo` | `get_finance_info` | ✅ |
| 指数行情 | `SecurityQuotes` | `get_index_quotes` | ✅ |

### 代码统计

- 新增模块：3个（finance_info, transaction, minute_time）
- 修改模块：2个（quotes 修复bug, mod.rs 导出）
- 新增示例程序：6个
- 新增Python验证脚本：6个
- 代码行数：+1297行

---

## v0.4.0 (2023-02-21)

rustdx-cmd：
* 移除所有异步依赖，因为请求量很少，不需要引入复杂的代码
* 移除所有不需要的代码和依赖（使用 cargo-udeps 检测到未使用的依赖）
* 移除 official 子命令

rustdx：
* 移除 html_root_url，因为 docs.rs 上的文档不需要这个属性

## v0.3.0 (2023-02-20)

- 更新依赖
- clippy fix
- 改动 features
- 命令行工具
    - 从 clickhouse 获取 factor.csv 之后自动删除它
    - 东财数据默认 6000 条

## [bin-v0.1.4](https://github.com/zjp-CN/rustdx/tree/bin-v0.1.4) (2021-12-01)

[Full Changelog](https://github.com/zjp-CN/rustdx/compare/v0.2.4.beta1...bin-v0.1.4)

**Implemented enhancements:**

- 增加版本参数 -v [\#18](https://github.com/zjp-CN/rustdx/issues/18)
- rustdx 命令默认不打印 TopLevel 结构体 [\#13](https://github.com/zjp-CN/rustdx/issues/13)

**Fixed bugs:**

- 停牌结束后复权因子错误 [\#17](https://github.com/zjp-CN/rustdx/issues/17)

**Closed issues:**

- `rustdx day -h o` 删除过时的帮助信息 [\#16](https://github.com/zjp-CN/rustdx/issues/16)
- Xdxr 内部小重构 [\#15](https://github.com/zjp-CN/rustdx/issues/15)
- `file::day::fq::Day` 文档：单位“笔”改为“股” [\#14](https://github.com/zjp-CN/rustdx/issues/14)
- 删除 serde\_type 模块，利用 `cfg_attr` 设置 serde 相关内容 [\#12](https://github.com/zjp-CN/rustdx/issues/12)
- 修正 lc 文档 [\#11](https://github.com/zjp-CN/rustdx/issues/11)

## [v0.2.4.beta1](https://github.com/zjp-CN/rustdx/tree/v0.2.4.beta1) (2021-10-13)

[Full Changelog](https://github.com/zjp-CN/rustdx/compare/v0.2.4...v0.2.4.beta1)

**Implemented enhancements:**

- 支持 day east -o clickhouse 从而无需导入命令 [\#9](https://github.com/zjp-CN/rustdx/issues/9)
- 支持 day east -p clickhouse 从而无需手动导出 factor.csv [\#8](https://github.com/zjp-CN/rustdx/issues/8)

**Fixed bugs:**

- 修复 rustdx east Unrecognized argument: -o错误 [\#10](https://github.com/zjp-CN/rustdx/issues/10)

## [v0.2.4](https://github.com/zjp-CN/rustdx/tree/v0.2.4) (2021-10-08)

[Full Changelog](https://github.com/zjp-CN/rustdx/compare/v0.2.3...v0.2.4)

**Implemented enhancements:**

- rustdx day 拓展 -o clickhouse -g xx -p xx [\#7](https://github.com/zjp-CN/rustdx/issues/7)
- ClickHouse 和前复权情况下，数据录入与导出命令 [\#2](https://github.com/zjp-CN/rustdx/issues/2)

**Fixed bugs:**

- 有些特例 \# 000001 \# 上市日因子不是 1 [\#1](https://github.com/zjp-CN/rustdx/issues/1)

## [v0.2.3](https://github.com/zjp-CN/rustdx/tree/v0.2.3) (2021-10-05)

[Full Changelog](https://github.com/zjp-CN/rustdx/compare/2e5e8de6535e215f4e77a80f2fadf814961b7af1...v0.2.3)



\* *This Changelog was automatically generated by [github_changelog_generator](https://github.com/github-changelog-generator/github-changelog-generator)*
