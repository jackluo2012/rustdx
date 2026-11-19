# shortmind-os 交接清单（rustdx 1.14.0 → 1.16.0）

> 写给正在维护 shortmind-os 的开发者 / AI。
> 来源：2026-11-17 ~ 11-19 对 rustdx 的实盘核对，以及对 shortmind-os 本地工作区
> （`f990b41`，比 origin/main 落后 7 个提交，那 7 个提交不涉及 rustdx 用法）的只读审阅。
> **本清单没有改动 shortmind-os 的任何文件。** 文中的 shortmind 行号以该提交为准，
> 改之前请重新定位。

## 0. 先读这个：三条认知更正

shortmind 的文档和代码里有三处"事实"已经不成立，下面多项改动都源于此。

| 原来的认识 | 实际情况 | 证据 |
|---|---|---|
| 「TDX 重查询族自 2026-09-10 起被风控，40 台全军覆没」（`DATA_SOURCE_BOUNDARY.md` 二c） | 不是风控。握手时发送的第三个包（pytdx `setup_cmd3`）会让服务端掐掉 K 线/行情/逐笔的权限，只回 2 字节 `20 03`。**rustdx 1.12.0 起握手不再发它，K 线已恢复** | 绕过代理、用独立 Python 重放、变换 12 组参数结果相同；4 台服务器上去掉该包立即恢复。11-18 压测：9 分钟 3.2 万请求、单台 32 连接每秒近 600 请求，零异常 |
| 「名义 ST 的个别股票实际按 10% 带宽交易」（`limit_status_auto` 的由来） | **2026-07-06 起沪深主板全部 ST 的涨跌幅由 ±5% 调整为 ±10%** | 11-19 实盘 141 只主板 ST 中 17 只的最高/最低价超出 ±5%，最大 +10.09% / -10.00%；历史日线上主板 ST 收于 ±5% 档的次数 6 月 679 次、8 月 31 次 |
| 「0x052c 对指数代码返回空数组」（`DATA_SOURCE_BOUNDARY.md` 一.3） | 服务器按指数格式回应（每条多 4 字节）。旧版按股票格式解析会错位；**1.14.0 起自动识别**，`k`/`bars`/`bars_range` 对指数返回正确结果 | 5 个指数的日线与 5 分钟线，经股票接口与指数接口逐项一致 |

## 1. 必须改（影响数据正确性）

### 1.1 主板 ST 涨跌幅：5% → 按日期 10%

**现状**

- `crates/domain/src/limit.rs` 的 `pct_for`：`Board::Main if st => 5`。
- `crates/domain/src/stock.rs:29` 的 `limit_pct_for` 同样是 5（测试 `:156` 断言 5.0）。
- 为解释"有些 ST 按 10% 交易"，加了**四处**同语义的"双档回退"：
  `domain::limit::limit_status_auto`、`market-engine/src/limit_up.rs:102-106`、
  `limit_up.rs:228-232`、`apps/worker/src/plans.rs:94-96`。
- 直接调 `limit_up_price` / `limit_down_price` 的地方没有回退：
  `realtime.rs:246`、`:993`、`:1395`，`review.rs:122`、`:443`、`:446`、`:860`、`:1062`。

**后果**

- 双档回退只在最高价触及 10% 档时才生效。停在 +5% 的主板 ST 仍被判为**封板**。
  实例（11-19）：*ST雅博 昨收 1.09，现价 1.14、最高 1.18，真实涨停价 1.20。
- 没有回退的调用点（实时涨停判定、跌停判定、复盘）对主板 ST 一律按 5% 算，全错。
- 跌停方向没有任何回退。

**改法**

rustdx 1.16.0 的 `limit` 模块已是整数分实现（见 2.1），并提供带日期的版本：

```rust
use rustdx_complete::limit::{self, Board, MAIN_ST_10PCT_SINCE};

limit::limit_up_price_on(prev_close, board, st, date);   // date: YYYYMMDD
limit::limit_down_price_on(prev_close, board, st, date);
limit::limit_status_on(close, high, prev_close, board, st, date);
board.limit_pct_on(st, date);
// 不带 _on 的版本按现行规则（主板 ST 10%），用于盘中
```

建议：`domain::limit` 保留业务类型（`DailyBar`、`LimitStatus::BoardBreak` 及其落库字符串、
`LadderTier`），把"算价"委托给上面的函数；判型用**当日日期**调 `_on` 版本。
然后删掉四处双档回退——有了正确的幅度表它们不再需要，留着反而会掩盖问题。

**需要重算的历史数据**

2026-07-06 及之后，主板 ST 相关的：涨停明细、连板高度、梯队、情绪判型、依赖它们的快照与简报。
7 月 6 日之前的不受影响（那时确实是 5%）。

**验收**

- 单测：`limit_up_price_on(10.03, Main, true, 20260703) == 10.53`、`(…, 20260706) == 11.03`。
- 用 11-19 的数据回放：主板 ST 无一只的最高价超过计算出的涨停价。
- `rg -n 'limit_status_auto' crates apps` 为空。

### 1.2 板块成分：`block_gn.dat` 每板块最多 400 只

**现状**

`crates/market-engine/src/ingest.rs:274` 的 `blocks()` 读 `block_gn.dat` / `block_zs.dat` /
`block_fg.dat`，写入 `stock_blocks`。

**后果**

这三个文件是固定槽位格式，每个板块最多 400 只。269 个概念里 28 个被截断，共丢失 5766 条成分；
保留的是**按代码排序的前 400 只**，所以丢的主要是沪市股票：

| 概念 | 完整成分 | `stock_blocks` 里现在有 | 其中沪市 |
|---|---|---|---|
| 人工智能 | 1077（沪 376） | 400 | 0 |
| 芯片 | 940（沪 402） | 400 | 0 |
| DeepSeek | 775（沪 326） | 400 | 0 |
| 锂电池 | 619（沪 239） | 400 | 48 |
| 商业航天 | 535（沪 208） | 400 | 97 |
| 低空经济 | 508（沪 182） | 400 | 96 |

受影响：`realtime::aggregate_sectors` 的板块脉冲、`first_theme` 题材归属、`review.rs:153` 的概念查询、
inspector 的 `theme_cross` / `concept_gap`。

另外 `blocks()` 里用 `market_of(&r.code)` 从代码推市场，会丢掉北交所成分。

**改法**

rustdx 1.15.0 新增 `Client::blocks()`，读服务器上的 `infoharbor_block.dat`：

```rust
use rustdx_complete::tcp::stock::{Block, BlockKind};

let blocks: Vec<Block> = client.blocks()?;   // 约 2.3 秒、760KB
// Block { kind: Concept|Style|Index, name, index_code: Option<String> /* 880xxx */,
//         created: Option<u32>, updated: Option<u32>, stocks: Vec<(u16 /*market*/, String)> }
```

- `BlockKind::Concept / Style / Index` 对应原来的 `gn / fg / zs`。
- 成分带市场，不要再用 `market_of` 推。
- `index_code` 是板块指数代码（沪市 880xxx），可直接取板块指数行情/K 线；建议落库，
  用它做关联比用名字可靠（板块名最长 4 个汉字，"人形机器人"在源头就是"人形机器"）。
- 任一板块成分数与文件头声明不符时返回错误，不会把下载残缺的成分当真。

**需要确认的一点**

`rg '\.blocks\(\)'` 只在 `apps/worker/src/init.rs:40` 和一个 example 里找到调用。
如果板块确实只在 `init` 时同步一次，那么 `stock_blocks` 自初始化后就没更新过；
而通达信几乎每个交易日都在调整成分（11 月以来 55 个概念有调整）。
建议并入每日流水线，盘前或盘后取一次即可（成分一天最多变一次）。

**已知限制**：通达信新增概念很慢（2026 年至今只新增 AI营销、MLCC概念 两个），
这份数据抓不到刚发酵的新题材。这是数据源的边界，不是 bug。

**验收**

- `SELECT count(*) FROM stock_blocks WHERE block_type='gn' AND block_name='人工智能'` 约 1077（随日变动）。
- 同一查询加 `AND market = 1` 不为 0。
- 成分数大于 400 的概念有 27 个左右（截断时这个数是 0）。注意完整数据里本来就有 1 个概念
  恰好 400 只，所以不能用"没有恰为 400 的"来判断。

### 1.3 集合竞价阶段的 TDX 行情

**现状**

`crates/market-engine/src/realtime.rs:952-957`：

```rust
// 竞价期 price 即虚拟撮合价（与 auction_ticks 采集口径一致）
let ref_px = if q.open > 0.0 { q.open } else { q.price };
```

**实际情况**

"price 即虚拟撮合价"对腾讯源成立，**对 TDX 不成立**。11-19 早盘 09:11~09:34 连续采样的结论：

| 时段 | `price` / `open` / `vol` | 撮合信息在哪 |
|---|---|---|
| 09:15 之前 | 全 0 | 无 |
| 09:15~09:25 | 全 0 | 买一价 = 卖一价 = 撮合价；买一量 = 卖一量 = 撮合量；买二量 / 卖二量 = 买方 / 卖方未匹配量 |
| 09:25 起 | 开盘价 / 开盘价 / 竞价成交量 | 五档恢复正常 |

而 `quality.rs:222` 把 `price <= 0` 视为关键字段缺失、行情不可用。两者叠加：
**TDX 作为实时源时，竞价窗口内所有行情都不可用，竞价异动告警不会触发，
竞价过程数据是空的。** 这与 `DATA_SOURCE_BOUNDARY.md` #12③ 是同一类问题（窗口与字段语义错配），只是换了数据源。

**改法**

rustdx 1.14.1 新增：

```rust
if let Some(a) = quote.auction() {
    // a.price 撮合价, a.volume 撮合量(手), a.unmatched_buy / a.unmatched_sell 未匹配量(手)
}
```

仅在"现价为 0 且买一价 = 卖一价 > 0 且买一量 = 卖一量"时返回 `Some`。
建议在 `RealtimeQuote::from_quote` 这一层做归一：竞价态时把撮合价填进 `price`
（与腾讯源口径对齐），撮合量与未匹配量另存；这样下游不需要区分数据源。

北交所竞价阶段是普通的买一卖一报价，`auction()` 返回 `None`。

**验收**：用 TDX 源跑一次 09:15~09:25，竞价异动告警能触发，`auction_ticks` 有非零价格。

### 1.4 升级依赖

`Cargo.toml:80`：`rustdx-complete = "1.14.0"` → `"1.16.0"`，
`cargo update -p rustdx-complete --precise 1.16.0`。

**升级后编译/行为上要处理的**

| 版本 | 变化 | shortmind 需要做的 |
|---|---|---|
| 1.14.1 | 开盘前/竞价阶段 `change_percent` 不再是 -100%（现价为 0 时给 0） | shortmind 自己算涨幅，不受影响；见 3.1 |
| 1.14.1 | 零成交量返回精确的 0（此前是 `5.9e-39`） | `b.vol.max(0.0) as u64` 结果不变，无需处理 |
| 1.14.1 | 债券行情时间可解出；基金时间精度改为百分之一秒 | 只用股票的话无影响 |
| 1.15.0 | `get_concept_stocks` 等标记 deprecated | shortmind 未使用，无影响 |
| 1.15.0 | 主板 ST 现行幅度改为 10% | 只用了 `is_st_name`，无直接影响；见 1.1 |
| 1.16.0 | **`QuoteData` 新增 `market` 字段** | 用结构体字面量构造 `QuoteData` 的测试需要补字段；`quote_sources.rs:44` "同码有歧义时保留未知" 可以改为直接取 `q.market` |
| 1.16.0 | 涨跌停价整数分 | 见 1.1 |

## 2. 建议改（消除重复 / 降低成本）

### 2.1 `domain::limit` 的整数分实现可以退役

`DATA_SOURCE_BOUNDARY.md` #17 记录了 rustdx 浮点舍入偏差，shortmind 因此自写了整数分版本。
rustdx 1.16.0 已改为同一公式 `(昨收分 × (100 ± 幅度) + 50) / 100`，并有穷举测试
（0.01~999.99 元 × 三档幅度 × 涨跌两个方向）。两边现在算法相同，建议合并到 rustdx 一处，
理由见 1.1。#17 可以更新为"已下沉"。

### 2.2 关闭 `recheck_empty`

`ingest.rs:47`、`:95`、`:570` 三处 `recheck_empty: true`。
它针对的"静默空响应"根因是握手包（已修），异常响应现在会直接报错并由 `auto_reconnect`
换服务器重试。继续开着的代价：每只停牌股、每个本来就没有 K 线的区间，多等 300ms 并重连一次。
`ingest.rs:10-11` 的边界表格与 `:520` 的注释同步更新。

rustdx 没有给这个字段加 `#[deprecated]`，就是为了不在这三处产生警告。

### 2.3 更新文档

- `DATA_SOURCE_BOUNDARY.md` 一.3：改为"1.14.0 起股票 K 线接口对指数自动识别；需要涨跌家数时用 `index_bars`"。
- 同文 #1"TDX 静默空结果"：K 线与行情的异常响应现在是错误而非空；`rows == 0` 告警仍有价值，保留。
- 同文 二c：标题与首段改为"握手包问题（已于 rustdx 1.12.0 修复）"；备用通道保留，见 4。
- 同文 #17：见 2.1。
- `Cargo.toml:76-79` 的版本注释。
- `TDX_REALTIME_FIX.md`：如果其中有基于"风控"的推断，一并更正。

### 2.4 K 线成交量口径（1.14.0 已变，确认下游）

`Cargo.toml` 注释显示 shortmind 已适配"股票分钟线成交量归一为手"。请再确认这两点：

- **指数与债券的日线成交量放大了 100 倍**（协议里是百手，现统一为手）。`ingest.rs:243`
  `volume: b.vol as u64` 写的是指数日线。如果 `kline_day` 里 1.14.0 之前写入的指数行没有重算，
  同一张表里新旧口径会差 100 倍。
- **指数分钟线没有成交量**，`vol` 字段是成交额的百分之一。`ingest.rs:382-383` 的注释显示已处理。

## 3. 需要 shortmind 侧确认（我没看透）

### 3.1 `aggregate_sectors` 是否把停牌股按 -100% 计入

`realtime.rs:538-541`：只过滤了 `last_close <= 0`，随后 `pct = price / last_close - 1`。
如果传进来的 `quotes` 没有预先用 `usable` 筛过，现价为 0 的证券（开盘前的全部、盘中的停牌股）
会以 -100% 计入板块等权平均。`realtime.rs:1391` 的另一处是同时判了 `price <= 0` 的。

### 3.2 盘前占位 K 线

09:25 之前，TDX 的 K 线与分时接口就已返回"今天"那一根：价格全是昨收、成交量 0。
收盘流水线 15:35 才跑，不受影响；如果有盘中拉当日 K 线的路径，按 `vol == 0.0` 过滤
（1.14.1 之前零成交量不是精确的 0，这个判断过滤不掉）。

### 3.3 ST 识别

`is_st_name` 靠证券名称里有没有 "ST"。摘帽 / 戴帽当天名称是否及时更新，两边都没有验证过。

## 4. 不建议改（现状更合理）

- **交易日历以交易所官方为准**：`calendar_sync.rs` 从深交所接口滚动同步，rustdx 的静态日历只做兜底，
  这个分工是对的。注意 rustdx 内置节假日只覆盖 2024~2026，范围外 `is_trading_day` 只按周几判断
  （2027 年元旦、国庆都返回 true）。`init.rs:81`、`:107` 与 `calendar_sync.rs:95` 在比对时可改用
  1.16.0 的 `TradingCalendar::try_is_trading_day`，它在范围外返回 `None`，避免把"没有数据"当成"是交易日"去比。
- **新浪 / 东财 / 腾讯 / 本地客户端文件这几条备用通道**：当初为"风控"而建，但作为多源冗余仍有价值。
  主通道可以回到 TDX，备用通道不必拆。
- **落库字符串、梯队结构、首题材归属、prev_close 链**：业务契约，留在 shortmind。

## 5. 边界判定法的一处补充

`DATA_SOURCE_BOUNDARY.md` 一的三问保持不变，建议在第 2 问加一句区分：

> 交易所规则本身（涨跌幅是多少、哪天生效、怎么舍入、哪些品种是三位小数报价）是**市场事实**，
> 归数据源库；规则判定之后的含义（落库字符串、梯队、情绪周期）是**业务语义**，留业务层。

这次的两个问题都落在这条线上：涨跌停幅度表与舍入规则在两边各有一份，一边修了另一边没跟上。

## 6. 数据源本身给不了的（核对过，不必再试）

龙虎榜、真正的逐笔成交与逐笔委托（这里的"逐笔"是约 3 秒一次的分笔汇总）、十档盘口、资金流向；
行情约 3 秒一跳；北交所证券列表（服务器不回应，`Client::stocks(2)` 立即返回 `Unsupported`）；
行业名称（`tdxhy.cfg` 只有行业代码）。

## 附：rustdx 1.11.2 → 1.16.0 的完整变更

见本仓库 `CHANGELOG.md`。每个版本的条目都写了实测依据与未验证的部分。
