//! 高层客户端（对标 mootdx 的 `StdQuotes`）。
//!
//! 封装连接管理与各协议请求，方法与 mootdx 的 API 一一对应，
//! 全部返回 owned 数据，开箱即用：
//!
//! ```ignore
//! use rustdx_complete::tcp::stock::Client;
//!
//! let mut client = Client::new()?; // 内置服务器故障转移
//! let quotes = client.quotes(&[(0, "000001"), (1, "600519")])?;
//! for q in quotes {
//!     println!("{}: {:.2} ({:+.2}%)", q.code, q.price, q.change_percent);
//! }
//! ```

use super::{
    BlockRecord, CompanyInfoCategory, CompanyInfoCategoryItem, CompanyInfoContent, FinanceInfoData,
    HistoryMinuteTime, HistoryTransaction, IndexKline, IndexKlineData, Kline, KlineData,
    MinuteTimeData, QuoteData, SecurityListData, TransactionData, Xdxr, XdxrData,
};
use crate::pool::ConnectionPool;
use crate::tcp::helper::DateTime;
use crate::tcp::{Tcp, TcpConfig, Tdx};
use chrono::Datelike;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// 高层客户端：持有连接，提供与 mootdx 对齐的语义化 API。
#[derive(Debug)]
pub struct Client {
    /// 底层连接（需要更细粒度控制时可直接使用）。
    pub tcp: Tcp,
}

/// 批量拉取（`k_batch`/`bars_batch`）的任务队列：`(输入序号, market, code)`。
type BatchQueue<'a> = Arc<Mutex<VecDeque<(usize, u16, &'a str)>>>;
/// 批量拉取的结果槽：按输入序号存放各股票的拉取结果。
type BatchResults<'a> = Arc<Mutex<Vec<Option<std::io::Result<Vec<KlineData<'a>>>>>>>;

impl Client {
    /// 连接服务器（内置故障转移），等价于 `Tcp::new()`。
    pub fn new() -> std::io::Result<Self> {
        Ok(Self { tcp: Tcp::new()? })
    }

    /// 以指定配置连接服务器。
    pub fn with_config(config: &TcpConfig) -> std::io::Result<Self> {
        Ok(Self {
            tcp: Tcp::with_config(config)?,
        })
    }

    /// Connect to an ordered pool with server rotation on reconnect.
    pub fn with_servers(
        config: &TcpConfig,
        servers: &[std::net::SocketAddr],
    ) -> std::io::Result<Self> {
        Ok(Self {
            tcp: Tcp::with_servers(config, servers)?,
        })
    }

    /// Connect with one shared absolute deadline for the complete server pool.
    pub fn with_servers_until(
        config: &TcpConfig,
        servers: &[std::net::SocketAddr],
        deadline: std::time::Instant,
    ) -> std::io::Result<Self> {
        Ok(Self {
            tcp: Tcp::with_servers_until(config, servers, deadline)?,
        })
    }

    /// 重连（见 [`Tcp::reconnect`]）。
    pub fn reconnect(&mut self) -> std::io::Result<()> {
        self.tcp.reconnect()
    }

    /// 心跳（见 [`Tcp::heartbeat`]）。
    pub fn heartbeat(&mut self) -> std::io::Result<u16> {
        self.tcp.heartbeat()
    }

    /// 请求失败时自动重连重试（见 [`Tcp::retry`]）。
    pub fn retry<T>(
        &mut self,
        f: impl FnMut(&mut Tcp) -> std::io::Result<T>,
        attempts: usize,
    ) -> std::io::Result<T> {
        self.tcp.retry(f, attempts)
    }

    /// 实时行情快照（mootdx `quotes`）。返回 owned 数据。
    ///
    /// [`TcpConfig::recheck_empty`](crate::tcp::TcpConfig::recheck_empty)
    /// 开启时空结果二次确认（等 300ms → 重连 → 重拉一次）——盘中高频轮询
    /// 场景下服务器会静默返回空帧，二次确认可消除抖动误判。
    ///
    /// 注意：非交易时段 quotes 本就为空，业务层应按时段判断后再调用，
    /// 否则每次都会多付一次重查代价。
    pub fn quotes(&mut self, stocks: &[(u16, &str)]) -> std::io::Result<Vec<QuoteData>> {
        let run = |tcp: &mut Tcp| {
            let mut quotes = SecurityQuotesRef::new(stocks.to_vec());
            quotes.recv_parsed(tcp)?;
            Ok(quotes.result().to_vec())
        };
        let result = if self.tcp.config().recheck_empty {
            recheck_empty(&mut self.tcp, run)
        } else {
            run(&mut self.tcp)
        };
        result.map_err(|e| ctx_err(e, format_args!("Client::quotes(n={})", stocks.len())))
    }

    /// 股票K线（mootdx `bars`）。
    ///
    /// 单页请求（最多 `count` 根）；解析层保证按时间**升序**返回。
    /// 区间回补请用 [`Client::bars_range`]（自动翻页 + 窗口过滤）。
    pub fn bars<'a>(
        &mut self,
        market: u16,
        code: &'a str,
        category: u16,
        start: u16,
        count: u16,
    ) -> std::io::Result<Vec<super::KlineData<'a>>> {
        let mut kline = Kline::new(market, code, category, start, count);
        kline.recv_parsed(&mut self.tcp).map_err(|e| {
            ctx_err(
                e,
                format_args!("Client::bars(market={market}, code={code}, category={category})"),
            )
        })?;
        Ok(kline.result().to_vec())
    }

    /// 按日期区间拉取任意周期 K 线（自动翻页，时间升序，去重）。
    ///
    /// 与 [`Client::k`](Self::k)（仅日线）不同，本方法支持全部 K 线种类
    /// （`category`：0=5m、1=15m、2=30m、3=1h、…、9=日线），适用于
    /// 分钟线等短周期数据的**区间回补**。
    ///
    /// 内置防护（均来自全市场回补实测）：
    /// - 自动翻页直至覆盖 `begin` 或历史尽头（页序无关，末端统一升序）；
    /// - 翻页偏移可能重叠 → 按时间戳去重；
    /// - 分钟线历史深度耗尽时服务器返回**乱码日期**（实测 2004/2035 年）→
    ///   遇不可信年份即终止，且最终按 `[begin, end]` 窗口过滤兜底；
    /// - [`TcpConfig::recheck_empty`](crate::tcp::TcpConfig::recheck_empty)
    ///   开启时空结果二次确认（防服务器静默限流）。
    ///
    /// # 参数
    ///
    /// - `begin`/`end`: 日期区间（YYYYMMDD，闭区间），`None` 表示不设限。
    pub fn bars_range<'a>(
        &mut self,
        market: u16,
        code: &'a str,
        category: u16,
        begin: Option<u32>,
        end: Option<u32>,
    ) -> std::io::Result<Vec<super::KlineData<'a>>> {
        let ctx =
            format_args!("Client::bars_range(market={market}, code={code}, category={category})");
        fetch_bars_range(&mut self.tcp, market, code, category, begin, end)
            .map_err(|e| ctx_err(e, ctx))
    }

    /// 指数K线（mootdx `index`/`index_bars`），含涨跌家数。
    ///
    /// 股票 K 线方法（[`Client::bars`] / [`Client::k`] 等）对指数代码也能返回正确的
    /// 价格与成交量，但不含涨跌家数；需要涨跌家数时走本方法（区间语义请用
    /// [`Client::index_bars_range`]）。
    pub fn index_bars<'a>(
        &mut self,
        market: u16,
        code: &'a str,
        category: u16,
        start: u16,
        count: u16,
    ) -> std::io::Result<Vec<super::IndexKlineData<'a>>> {
        let mut kline = IndexKline::new(market, code, category, start, count);
        kline.recv_parsed(&mut self.tcp).map_err(|e| {
            ctx_err(
                e,
                format_args!("Client::index_bars(market={market}, code={code})"),
            )
        })?;
        Ok(kline.result().to_vec())
    }

    /// 指数K线区间回补：[`Client::index_bars`] 的自动翻页版，
    /// 与 [`Client::bars_range`](Self::bars_range) 完全对称（0x052d 指数专用命令）。
    ///
    /// - 自动翻页直至覆盖 `begin` 或历史尽头（页序无关，末端统一升序）；
    /// - 翻页偏移可能重叠 → 按时间戳去重；
    /// - 最终按 `[begin, end]` 窗口过滤。
    ///
    /// # 参数
    ///
    /// - `begin`/`end`: 日期区间（YYYYMMDD，闭区间），`None` 表示不设限。
    pub fn index_bars_range<'a>(
        &mut self,
        market: u16,
        code: &'a str,
        category: u16,
        begin: Option<u32>,
        end: Option<u32>,
    ) -> std::io::Result<Vec<super::IndexKlineData<'a>>> {
        let ctx = format_args!(
            "Client::index_bars_range(market={market}, code={code}, category={category})"
        );
        fetch_index_bars_range(&mut self.tcp, market, code, category, begin, end)
            .map_err(|e| ctx_err(e, ctx))
    }

    /// 按日期区间拉取日K线（mootdx `k`）。
    ///
    /// 自动翻页直到覆盖 `begin`，并过滤出 `[begin, end]`（格式 YYYYMMDD）的数据。
    /// `begin`/`end` 传 `None` 表示不设下/上限。
    ///
    /// [`TcpConfig::recheck_empty`](crate::tcp::TcpConfig::recheck_empty)
    /// 开启时空结果二次确认（防服务器静默限流误判为停牌真空）。
    pub fn k<'a>(
        &mut self,
        market: u16,
        code: &'a str,
        begin: Option<u32>,
        end: Option<u32>,
    ) -> std::io::Result<Vec<super::KlineData<'a>>> {
        let run = |tcp: &mut Tcp| fetch_k(tcp, market, code, begin, end);
        let result = if self.tcp.config().recheck_empty {
            recheck_empty(&mut self.tcp, run)
        } else {
            run(&mut self.tcp)
        };
        result.map_err(|e| ctx_err(e, format_args!("Client::k(market={market}, code={code})")))
    }

    /// 批量拉取多只股票日K（内部连接池并行，不占用当前连接）。
    ///
    /// # 参数
    ///
    /// - `stocks`: `(market, code)` 列表（market: 0=深市、1=沪市）；
    /// - `begin`/`end`: 日期区间（YYYYMMDD），`None` 不设限；
    /// - `max_parallel`: 并行连接数上限；传 `0` 按 CPU 核数自动。
    ///   建议 4~16——服务器有连接数限制，过大会触发拒连。
    ///
    /// # 返回
    ///
    /// 按输入顺序返回；单只股票拉取失败不影响其他股票
    /// （失败信息在对应 [`BatchKline::result`] 中）。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use rustdx_complete::tcp::stock::Client;
    ///
    /// let client = Client::new()?;
    /// let rows = client.k_batch(&[(1, "600000"), (0, "000001"), (1, "600519")],
    ///                           Some(20260101), None, 4)?;
    /// for row in rows {
    ///     println!("{}: {} 根", row.code, row.result?.len());
    /// }
    /// ```
    pub fn k_batch<'a>(
        &self,
        stocks: &[(u16, &'a str)],
        begin: Option<u32>,
        end: Option<u32>,
        max_parallel: usize,
    ) -> std::io::Result<Vec<BatchKline<'a>>> {
        let results = run_batch(stocks, max_parallel, |tcp, market, code| {
            fetch_k(tcp, market, code, begin, end)
                .map_err(|e| ctx_err(e, format_args!("k_batch(market={market}, code={code})")))
        })?;
        Ok(stocks
            .iter()
            .zip(results)
            .map(|(&(market, code), result)| BatchKline {
                market,
                code,
                result,
            })
            .collect())
    }

    /// 批量拉取多只股票任意周期 K 线区间（内部连接池并行，不占用当前连接）。
    ///
    /// 分钟线等短周期数据的**全市场回补**配套：每只股票走 [`Client::bars_range`]
    /// 同一套防护（自动翻页、去重、乱码年份终止、窗口过滤、升序），多连接并行。
    ///
    /// # 参数
    ///
    /// - `stocks`: `(market, code)` 列表（market: 0=深市、1=沪市）；
    /// - `category`: K 线种类（0=5m、1=15m、…、9=日线）；
    /// - `begin`/`end`: 日期区间（YYYYMMDD，闭区间），`None` 不设限；
    /// - `max_parallel`: 并行连接数上限；传 `0` 按 CPU 核数自动。
    ///   建议 4~16——服务器有连接数限制，过大会触发拒连。
    ///
    /// 按输入顺序返回；单只失败不影响其他（失败信息在对应 [`BatchBars::result`] 中）。
    pub fn bars_batch<'a>(
        &self,
        stocks: &[(u16, &'a str)],
        category: u16,
        begin: Option<u32>,
        end: Option<u32>,
        max_parallel: usize,
    ) -> std::io::Result<Vec<BatchBars<'a>>> {
        let results = run_batch(stocks, max_parallel, |tcp, market, code| {
            fetch_bars_range(tcp, market, code, category, begin, end).map_err(|e| {
                ctx_err(
                    e,
                    format_args!("bars_batch(market={market}, code={code}, category={category})"),
                )
            })
        })?;
        Ok(stocks
            .iter()
            .zip(results)
            .map(|(&(market, code), result)| BatchBars {
                market,
                code,
                result,
            })
            .collect())
    }

    /// 按日期区间拉取**复权**日K线（前复权/后复权，本地计算）。
    ///
    /// 通达信服务器返回的 K 线均为不复权数据（实测请求中的复权字段实为
    /// 「取样步长」，见 `tests/adj_field_probe.rs`），因此本方法在本地用
    /// 除权除息事件（[`Client::xdxr`]）计算复权价：算法与通达信/akshare
    /// 一致，除权参考价 `(preclose*10 - fh + pg*pgj) / (10 + pg + sg)`，
    /// 除权日价格按 `preclose_adj / preclose` 缩放，逐日累积。
    ///
    /// 复权只调整 OHLC 价格；`vol`/`amount` 保持原始值。
    ///
    /// # 参数
    ///
    /// - `adj`: 前复权（[`Adj::Qfq`]，最新价 = 实际价）或后复权
    ///   （[`Adj::Hfq`]，上市首日价 = 实际价）；
    /// - `begin`/`end`: 日期区间（YYYYMMDD），`None` 不设限。
    pub fn k_adjusted<'a>(
        &mut self,
        market: u16,
        code: &'a str,
        adj: Adj,
        begin: Option<u32>,
        end: Option<u32>,
    ) -> std::io::Result<Vec<super::KlineData<'a>>> {
        // 复权因子需从上市日起连续累积，先全量拉取再在本地过滤区间
        let all = fetch_k(&mut self.tcp, market, code, None, None).map_err(|e| {
            ctx_err(
                e,
                format_args!("Client::k_adjusted(market={market}, code={code})"),
            )
        })?;
        if all.is_empty() {
            return Ok(all);
        }
        let xdxrs = self.xdxr(market, code)?;
        let multipliers = adjusted_multipliers(&all, &xdxrs, adj);

        let out: Vec<super::KlineData<'a>> = all
            .into_iter()
            .zip(multipliers)
            .filter(|(bar, _)| {
                let d = DateTime::to_u32(bar.dt);
                begin.is_none_or(|b| d >= b) && end.is_none_or(|e| d <= e)
            })
            .map(|(mut bar, m)| {
                bar.open *= m;
                bar.high *= m;
                bar.low *= m;
                bar.close *= m;
                bar
            })
            .collect();
        Ok(out)
    }

    /// 当日分时（mootdx `minute`）。
    ///
    /// 2026 新协议已直接支持（响应含 market+code 头与盘口快照，数据块定位
    /// 见 [`super::MinuteTime`] 文档）。若某台服务器解析仍为空（异常响应），
    /// 本方法**自动回退**：「今日历史分时接口」（[`HistoryMinuteTime`] 查询
    /// 当天）协议稳定，作为兜底数据源。
    ///
    /// 两个接口返回相同的数据结构（每分钟一个价格/成交量点）。
    ///
    /// 09:25 之前会返回 1 个占位点（价格为昨收、成交量 0）；09:25~09:30 这个点是
    /// 开盘价与竞价成交量；第一分钟（09:31）的成交量包含竞价成交量。
    pub fn minute(&mut self, market: u16, code: &str) -> std::io::Result<Vec<MinuteTimeData>> {
        let ctx = format_args!("Client::minute(market={market}, code={code})");
        let mut mt = super::MinuteTime::new(market, code);
        mt.recv_parsed(&mut self.tcp).map_err(|e| ctx_err(e, ctx))?;
        if !mt.result().is_empty() {
            return Ok(mt.result().to_vec());
        }

        // 回退：当日分时接口不可用 → 今日历史分时接口（协议稳定）
        let mut hmt = HistoryMinuteTime::new(market, code, today_yyyymmdd());
        hmt.recv_parsed(&mut self.tcp)
            .map_err(|e| ctx_err(e, format_args!("{ctx}（回退到今日历史分时）")))?;
        Ok(hmt.result().to_vec())
    }

    /// 历史分时（mootdx `minutes`）。
    pub fn history_minute(
        &mut self,
        market: u16,
        code: &str,
        date: u32,
    ) -> std::io::Result<Vec<MinuteTimeData>> {
        let mut mt = HistoryMinuteTime::new(market, code, date);
        mt.recv_parsed(&mut self.tcp).map_err(|e| {
            ctx_err(
                e,
                format_args!("Client::history_minute(market={market}, code={code}, date={date})"),
            )
        })?;
        Ok(mt.result().to_vec())
    }

    /// 当日逐笔成交（mootdx `transaction`）。
    pub fn transaction(
        &mut self,
        market: u16,
        code: &str,
        start: u16,
        count: u16,
    ) -> std::io::Result<Vec<TransactionData>> {
        let mut tx = super::Transaction::new(market, code, start, count);
        tx.recv_parsed(&mut self.tcp).map_err(|e| {
            ctx_err(
                e,
                format_args!("Client::transaction(market={market}, code={code})"),
            )
        })?;
        Ok(tx.result().to_vec())
    }

    /// 当日全部逐笔成交：自动翻页，按时间升序。
    ///
    /// 单次请求最多 [`Transaction::MAX_COUNT`](super::Transaction::MAX_COUNT) 笔，
    /// 本方法从最新一页往前取到开盘。
    pub fn transaction_all(
        &mut self,
        market: u16,
        code: &str,
    ) -> std::io::Result<Vec<TransactionData>> {
        let step = super::Transaction::MAX_COUNT;
        collect_ticks(step, |start| self.transaction(market, code, start, step))
    }

    /// 某日全部历史逐笔成交：自动翻页，按时间升序。`date` 格式 YYYYMMDD。
    pub fn history_transaction_all(
        &mut self,
        market: u16,
        code: &str,
        date: u32,
    ) -> std::io::Result<Vec<TransactionData>> {
        let step = HistoryTransaction::MAX_COUNT;
        collect_ticks(step, |start| {
            self.history_transaction(market, code, start, step, date)
        })
    }

    /// 历史逐笔成交（mootdx `transactions`）。
    pub fn history_transaction(
        &mut self,
        market: u16,
        code: &str,
        start: u16,
        count: u16,
        date: u32,
    ) -> std::io::Result<Vec<TransactionData>> {
        let mut tx = HistoryTransaction::new(market, code, start, count, date);
        tx.recv_parsed(&mut self.tcp).map_err(|e| {
            ctx_err(
                e,
                format_args!("Client::history_transaction(market={market}, code={code})"),
            )
        })?;
        Ok(tx.result().to_vec())
    }

    /// 财务信息（mootdx `finance`）。
    pub fn finance(&mut self, market: u16, code: &str) -> std::io::Result<FinanceInfoData> {
        let mut fin = super::FinanceInfo::new(market as u8, code);
        fin.recv_parsed(&mut self.tcp).map_err(|e| {
            ctx_err(
                e,
                format_args!("Client::finance(market={market}, code={code})"),
            )
        })?;
        Ok(fin.result().first().cloned().unwrap_or_default())
    }

    /// 除权除息信息（mootdx `xdxr`）。
    pub fn xdxr(&mut self, market: u16, code: &str) -> std::io::Result<Vec<super::XdxrData>> {
        let mut xdxr = Xdxr::new(market, code);
        xdxr.recv_parsed(&mut self.tcp).map_err(|e| {
            ctx_err(
                e,
                format_args!("Client::xdxr(market={market}, code={code})"),
            )
        })?;
        Ok(xdxr.result().to_vec())
    }

    /// 完整的板块数据：概念、风格、指数三类，成分不截断，带板块指数代码与最近
    /// 调整日期（见 [`super::get_blocks`]）。做题材请用这个，而不是 [`Client::block`]。
    pub fn blocks(&mut self) -> crate::Result<Vec<super::Block>> {
        super::get_blocks(&mut self.tcp)
            .map_err(|e| crate::Error::Io(std::io::Error::other(format!("Client::blocks: {e}"))))
    }

    /// 板块文件（mootdx `block`），如 `"block_gn.dat"` 概念板块。
    ///
    /// **每个板块最多返回 400 只成分股**（文件格式的上限，热门题材会被截断），
    /// 完整成分请用 [`Client::blocks`]。
    pub fn block(&mut self, block_file: &str) -> crate::Result<Vec<BlockRecord>> {
        super::get_block_info(&mut self.tcp, block_file).map_err(|e| {
            crate::Error::Io(std::io::Error::other(format!(
                "Client::block({block_file}): {e}"
            )))
        })
    }

    /// 证券数量（mootdx `stock_count`）。
    pub fn stock_count(&mut self, market: u16) -> std::io::Result<u16> {
        let mut sc = super::super::basic::SecurityCount::new(market);
        let c = sc
            .recv_parsed(&mut self.tcp)
            .map_err(|e| ctx_err(e, format_args!("Client::stock_count(market={market})")))?;
        Ok(*c)
    }

    /// 全部证券列表（mootdx `stocks`），自动分页聚合。
    ///
    /// 仅沪深（market 0/1）。北交所（market 2）服务器不提供列表，立即返回
    /// `Unsupported` 错误。
    pub fn stocks(&mut self, market: u16) -> crate::Result<Vec<SecurityListData>> {
        super::stocks(&mut self.tcp, market).map_err(|e| {
            crate::Error::Io(std::io::Error::other(format!(
                "Client::stocks(market={market}): {e}"
            )))
        })
    }

    /// F10 公司资料全部栏目（mootdx `F10`），返回 `(栏目名, 内容)`。
    pub fn f10(&mut self, market: u16, code: &str) -> std::io::Result<Vec<(String, String)>> {
        let ctx = format_args!("Client::f10(market={market}, code={code})");
        let mut cat = CompanyInfoCategory::new(market, code);
        cat.recv_parsed(&mut self.tcp)
            .map_err(|e| ctx_err(e, ctx))?;
        let mut result = Vec::with_capacity(cat.result().len());
        for item in cat.result() {
            let mut content =
                CompanyInfoContent::new(market, code, &item.filename, item.start, item.length);
            content
                .recv_parsed(&mut self.tcp)
                .map_err(|e| ctx_err(e, format_args!("{ctx}（栏目 {}）", item.name)))?;
            result.push((item.name.clone(), std::mem::take(&mut content.data)));
        }
        Ok(result)
    }

    /// F10 公司资料栏目目录（mootdx `F10C`）。
    pub fn f10_categories(
        &mut self,
        market: u16,
        code: &str,
    ) -> std::io::Result<Vec<CompanyInfoCategoryItem>> {
        let mut cat = CompanyInfoCategory::new(market, code);
        cat.recv_parsed(&mut self.tcp).map_err(|e| {
            ctx_err(
                e,
                format_args!("Client::f10_categories(market={market}, code={code})"),
            )
        })?;
        Ok(cat.result().to_vec())
    }
}

/// 逐笔翻页：`start = 0` 是最新一页，越往后越早；每页内部按时间升序。
/// 取到不足一页为止，再把各页按「旧页在前」拼起来。
fn collect_ticks(
    step: u16,
    mut fetch_page: impl FnMut(u16) -> std::io::Result<Vec<TransactionData>>,
) -> std::io::Result<Vec<TransactionData>> {
    let mut pages = Vec::new();
    let mut start = 0u16;
    loop {
        let page = fetch_page(start)?;
        let full = page.len() >= step as usize;
        pages.push(page);
        // 不足一页即到开盘；u16 偏移用尽也停止
        let Some(next) = start.checked_add(step).filter(|_| full) else {
            break;
        };
        start = next;
    }
    Ok(pages.into_iter().rev().flatten().collect())
}

/// 今日日期（**北京时间**），格式 YYYYMMDD（用于当日分时回退到今日历史分时）。
///
/// 交易日以东八区为准，不能用本机时区：机器在其它时区时「今天」会差一天。
fn today_yyyymmdd() -> u32 {
    let beijing = chrono::FixedOffset::east_opt(8 * 3600).expect("东八区偏移合法");
    let today = chrono::Utc::now().with_timezone(&beijing).date_naive();
    (today.year() * 10000 + today.month() as i32 * 100 + today.day() as i32) as u32
}

/// 给 IO 错误附加接口上下文（保留错误 kind，便于调用方按 kind 分支处理）。
fn ctx_err(e: std::io::Error, ctx: std::fmt::Arguments<'_>) -> std::io::Error {
    std::io::Error::new(e.kind(), format!("{ctx}: {e}"))
}

/// 批量拉取通用执行器：内部连接池并行，对每只股票调用 `fetch`，
/// 按输入顺序返回各自结果（`k_batch`/`bars_batch` 共用）。
///
/// - `max_parallel == 0` 按 CPU 核数自动；
/// - 单只失败不影响其他；worker 建连失败时，未被处理的股票结果为
///   `worker 未能处理该股票（连接建立失败）`。
fn run_batch<'a, F>(
    stocks: &[(u16, &'a str)],
    max_parallel: usize,
    fetch: F,
) -> std::io::Result<Vec<std::io::Result<Vec<KlineData<'a>>>>>
where
    F: Fn(&mut Tcp, u16, &'a str) -> std::io::Result<Vec<KlineData<'a>>> + Sync,
{
    if stocks.is_empty() {
        return Ok(Vec::new());
    }
    let workers = match max_parallel {
        0 => std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4),
        n => n,
    }
    .min(stocks.len())
    .max(1);

    let pool = ConnectionPool::new(workers)?;
    let queue: BatchQueue<'a> = Arc::new(Mutex::new(
        stocks
            .iter()
            .enumerate()
            .map(|(i, &(m, c))| (i, m, c))
            .collect(),
    ));
    let results: BatchResults<'a> = Arc::new(Mutex::new((0..stocks.len()).map(|_| None).collect()));

    // 固定 worker 数 = 连接池大小：每个 worker 持一条连接处理队列，
    // 避免反复建连/归还的锁竞争，也不会触发连接池饱和错误。
    let fetch = &fetch;
    std::thread::scope(|s| {
        for _ in 0..workers {
            let queue = Arc::clone(&queue);
            let results = Arc::clone(&results);
            let pool = pool.clone();
            s.spawn(move || {
                let Ok(mut conn) = pool.get_connection() else {
                    return;
                };
                loop {
                    let item = queue.lock().unwrap().pop_front();
                    let Some((idx, market, code)) = item else {
                        break;
                    };
                    let r = conn.execute(|tcp| fetch(tcp, market, code));
                    results.lock().unwrap()[idx] = Some(r);
                }
            });
        }
    });

    let mut results = results.lock().unwrap();
    Ok(results
        .iter_mut()
        .map(|slot| {
            slot.take().unwrap_or_else(|| {
                Err(std::io::Error::other(
                    "worker 未能处理该股票（连接建立失败）",
                ))
            })
        })
        .collect())
}

/// 避免与 facade 文档中的 `SecurityQuotes` 混淆的内部别名。
use super::SecurityQuotes as SecurityQuotesRef;

/// 空结果二次确认：TDX 服务端在高频请求下会**静默返回空响应**（不报错），
/// 与停牌/退市股「窗口内本就无 K 线」的真空无法从返回值区分。
/// 结果为空时：等待 300ms 并重连，重拉一次，仍空才返回空。
fn recheck_empty<T>(
    tcp: &mut Tcp,
    mut fetch: impl FnMut(&mut Tcp) -> std::io::Result<Vec<T>>,
) -> std::io::Result<Vec<T>> {
    let first = fetch(tcp)?;
    if !first.is_empty() {
        return Ok(first);
    }
    tcp.sleep_with_deadline(std::time::Duration::from_millis(300))?;
    tcp.reconnect()?;
    fetch(tcp)
}

/// 区间翻页页大小（单页上限，与 pytdx/mootdx 约定一致）。
const BARS_PAGE: u16 = 800;
/// 区间翻页页数上限：分钟线历史深度有限（约近年），超深分页服务器会返回
/// 乱码日期，封顶兜底（800 × 40 = 3.2 万根，约 660 个交易日的 5 分钟线）。
const MAX_BARS_PAGES: usize = 40;

/// 区间翻页通用实现：自动翻页、去重、窗口过滤、升序。
///
/// 股票（0x052c）与指数（0x052d）K 线区间回补共用同一套约定，仅「拉取一页」
/// 的请求类型与元素类型不同，由调用方以闭包注入：
///
/// - `fetch_page(start)`: 拉取偏移 `start` 起的一页（至多 [`BARS_PAGE`] 根）；
/// - `dt`: 取元素时间戳。
///
/// 不依赖 [`Tcp`]，判停/去重/过滤逻辑可离线单测。
fn fetch_paged_range<T>(
    mut fetch_page: impl FnMut(u16) -> std::io::Result<Vec<T>>,
    dt: impl Fn(&T) -> DateTime,
    begin: Option<u32>,
    end: Option<u32>,
) -> std::io::Result<Vec<T>> {
    let mut all: Vec<T> = Vec::new();
    let mut start = 0u16;
    let mut pages = 0usize;
    loop {
        let bars = fetch_page(start)?;
        let n = bars.len();
        // 页内最旧一根（顺序无关；乱码日期页由下方年份校验拦截）
        let oldest = bars.iter().map(&dt).min();
        all.extend(bars);
        pages += 1;
        if n < BARS_PAGE as usize {
            break; // 历史尽头
        }
        let Some(oldest) = oldest else {
            break;
        };
        if !(2000..=2070).contains(&oldest.year) {
            break; // 服务器历史深度耗尽，开始返回乱码日期（实测 2004/2035 年）
        }
        if let Some(begin) = begin {
            // 页内最旧数据已早于 begin，停止翻页（min 判停，顺序无关）
            if DateTime::to_u32(oldest) < begin {
                break;
            }
        }
        if pages >= MAX_BARS_PAGES {
            break;
        }
        start = start.saturating_add(BARS_PAGE);
        if start == 0 {
            break; // u16 溢出，服务器历史已耗尽
        }
    }

    // 升序 + 翻页偏移重叠去重（排序后重复时间戳相邻）+ 窗口过滤（丢弃乱码日期页残留）
    all.sort_by_key(&dt);
    all.dedup_by_key(|b| dt(b));
    all.retain(|bar| {
        let d = DateTime::to_u32(dt(bar));
        begin.is_none_or(|b| d >= b) && end.is_none_or(|e| d <= e)
    });
    Ok(all)
}

/// 从给定连接按日期区间拉取任意周期 K 线：自动翻页、去重、窗口过滤、升序。
///
/// 与 `Client::bars_range` 共用逻辑（`Client::k` 的日K专用版见 [`fetch_k`]）。
fn fetch_bars_range<'a>(
    tcp: &mut Tcp,
    market: u16,
    code: &'a str,
    category: u16,
    begin: Option<u32>,
    end: Option<u32>,
) -> std::io::Result<Vec<KlineData<'a>>> {
    fetch_paged_range(
        |start| {
            let mut kline = Kline::new(market, code, category, start, BARS_PAGE);
            kline.recv_parsed(tcp)?;
            Ok(kline.result().to_vec())
        },
        |b| b.dt,
        begin,
        end,
    )
}

/// 从给定连接按日期区间拉取指数K线：自动翻页、去重、窗口过滤、升序。
///
/// 与 [`fetch_bars_range`]（股票 0x052c）对称，命令为 0x052d 指数专用；
/// 翻页判停/去重/乱码年份终止/窗口过滤同一套约定。
fn fetch_index_bars_range<'a>(
    tcp: &mut Tcp,
    market: u16,
    code: &'a str,
    category: u16,
    begin: Option<u32>,
    end: Option<u32>,
) -> std::io::Result<Vec<IndexKlineData<'a>>> {
    fetch_paged_range(
        |start| {
            let mut kline = IndexKline::new(market, code, category, start, BARS_PAGE);
            kline.recv_parsed(tcp)?;
            Ok(kline.result().to_vec())
        },
        |b| b.dt,
        begin,
        end,
    )
}

/// 除权参考价（`category == 1` 除权除息事件）：
/// `(前收盘 × 10 − 每10股派现 + 配股价 × 每10股配股) ÷ (10 + 每10股送转 + 每10股配股)`，
/// **四舍五入到分**（与交易所/行情软件的除权日昨收一致）。
///
/// 分母为 0（无任何变动）或结果非法时原样返回 `preclose`。
/// 除权日 `prev_close` 应传本函数结果，随后即可做涨跌停判定
/// （见 [`crate::limit`]）与日线 `prev_close` 链构建。
pub fn ex_right_reference(preclose: f64, ev: &XdxrData) -> f64 {
    let numer = preclose * 10.0 - ev.fh_qltp as f64 + ev.pgj_qzgb as f64 * ev.pg_hzgb as f64;
    let denom = 10.0 + ev.sg_hltp as f64 + ev.pg_hzgb as f64;
    if denom > 0.0 {
        let adj = numer / denom;
        if adj.is_finite() && adj > 0.0 {
            return (adj * 100.0).round() / 100.0;
        }
    }
    preclose
}

/// 从给定连接拉取单只股票日K：自动翻页、日期区间过滤、按日期升序排序。
///
/// 与 `Client::k` 共用同一套逻辑，供 `k_batch` 在连接池连接上复用。
fn fetch_k<'a>(
    tcp: &mut Tcp,
    market: u16,
    code: &'a str,
    begin: Option<u32>,
    end: Option<u32>,
) -> std::io::Result<Vec<KlineData<'a>>> {
    const PAGE: u16 = 800;
    const CATEGORY_DAY: u16 = 9;

    let mut all = Vec::new();
    let mut start = 0u16;
    loop {
        let mut kline = Kline::new(market, code, CATEGORY_DAY, start, PAGE);
        kline.recv_parsed(tcp)?;
        let bars = kline.result().to_vec();
        let n = bars.len();
        // 页内顺序不再假设（2026-09 实测部分服务器返回顺序与早前相反，
        // 猜测与「当日分时协议变更」同期调整），统一在末端排序兜底。
        all.extend(bars);
        if n < PAGE as usize {
            break; // 历史尽头
        }
        if let Some(begin) = begin {
            // 页内最旧数据已早于 begin，停止翻页（min 而非 first，顺序无关）
            if let Some(oldest) = all.iter().map(|b| DateTime::to_u32(b.dt)).min()
                && oldest < begin
            {
                break;
            }
        }
        start = start.saturating_add(PAGE);
        if start == 0 {
            break; // u16 溢出，服务器历史已耗尽
        }
    }

    all.retain(|bar| {
        let d = DateTime::to_u32(bar.dt);
        begin.is_none_or(|b| d >= b) && end.is_none_or(|e| d <= e)
    });
    // 契约：按日期升序（文档承诺「统一按升序存放」）；翻页偏移可能重叠，
    // 排序后重复日期相邻，一并去重（与 bars_range 一致）
    all.sort_by_key(|bar| bar.dt);
    all.dedup_by_key(|bar| bar.dt);
    Ok(all)
}

/// 批量 K 线单项结果。
///
/// `result` 为该股票独立的结果：`Ok` 含按日期升序的日 K 数据，
/// `Err` 为该股票拉取失败（不影响其他股票）。
#[derive(Debug)]
pub struct BatchKline<'a> {
    /// 市场代码（0=深市、1=沪市）
    pub market: u16,
    /// 6 位股票代码
    pub code: &'a str,
    /// 该股票拉取结果
    pub result: std::io::Result<Vec<KlineData<'a>>>,
}

/// [`Client::bars_batch`] 单项结果（结构同 [`BatchKline`]，任意周期）。
#[derive(Debug)]
pub struct BatchBars<'a> {
    /// 市场代码（0=深市、1=沪市）
    pub market: u16,
    /// 6 位股票代码
    pub code: &'a str,
    /// 该股票拉取结果（按时间升序）
    pub result: std::io::Result<Vec<KlineData<'a>>>,
}

/// 复权方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adj {
    /// 前复权：以最新价为基准，历史价按比例缩放（最新价 = 实际价）。
    Qfq,
    /// 后复权：以上市首日为基准，后续价格按比例放大（首日价 = 实际价）。
    Hfq,
}

/// 计算每根 K 线的复权乘数（基于收盘价序列与除权除息事件）。
///
/// 算法（与通达信/akshare 一致）：
/// - 除权参考价 `preclose_adj = (preclose*10 - fh + pg*pgj) / (10 + pg + sg)`
///   （`fh`=每 10 股派现、`sg`=每 10 股送转、`pg`=每 10 股配股、`pgj`=配股价）；
/// - 除权日价格缩放乘数 `scale *= preclose_adj / preclose`，逐日累积；
///   `preclose` 始终为**除权日前一交易日实际收盘**（停牌顺延到下一交易日处理）；
/// - 后复权乘数 `= 1/scale`（除权后价格放大回除权前量纲，首日价不变）；
/// - 前复权乘数 `= scale[last]/scale`（以最新价为基准，最新价不变）。
///
/// 只使用 `category == 1`（除权除息）事件；增发/回购/转配等不改变价格基准。
fn adjusted_multipliers(days: &[KlineData<'_>], xdxrs: &[XdxrData], adj: Adj) -> Vec<f64> {
    // 首根 K 线当日及之前的除权事件没有「前一交易日收盘」可用，且对序列内
    // 各根的相对价格没有影响（首根已是除权后价格），必须跳过——否则会拿首根
    // 自己的收盘价当前收盘，给整条序列乘上一个错误的常数。
    let first_date = days.first().map(|d| DateTime::to_u32(d.dt)).unwrap_or(0);
    let mut events: Vec<(u32, f32, f32, f32, f32)> = xdxrs
        .iter()
        .filter(|x| x.category == 1 && x.date > first_date)
        .map(|x| (x.date, x.fh_qltp, x.sg_hltp, x.pg_hzgb, x.pgj_qzgb))
        .collect();
    events.sort_by_key(|e| e.0);
    let mut ev = events.iter().peekable();

    let mut scales: Vec<f64> = Vec::with_capacity(days.len());
    let mut scale = 1.0f64;
    let mut preclose = days.first().map(|d| d.close).unwrap_or(0.0);
    for d in days {
        let date = DateTime::to_u32(d.dt);
        // 应用所有日期 ≤ 当前交易日且尚未应用的除权事件（含停牌场景：
        // 除权日无 K 线时顺延到下一个交易日）
        while let Some(&&(edate, fh, sg, pg, pgj)) = ev.peek() {
            if edate > date {
                break;
            }
            let event = XdxrData {
                market: 0,
                code: String::new(),
                date: edate,
                category: 1,
                fh_qltp: fh,
                pgj_qzgb: pgj,
                sg_hltp: sg,
                pg_hzgb: pg,
            };
            let preclose_adj = ex_right_reference(preclose, &event);
            if preclose > 0.0 {
                scale *= preclose_adj / preclose;
            }
            ev.next();
        }
        scales.push(scale);
        preclose = d.close;
    }

    let last = *scales.last().unwrap_or(&1.0);
    scales
        .into_iter()
        .map(|s| match adj {
            Adj::Hfq => 1.0 / s,
            Adj::Qfq => last / s,
        })
        .map(|m| if m.is_finite() { m } else { 1.0 })
        .collect()
}

#[cfg(test)]
mod tests {

    #[test]
    fn k_range_retain_logic() {
        // 直接测试日期过滤逻辑所依赖的 to_u32 排序
        use crate::tcp::helper::DateTime;
        let d = DateTime {
            year: 2026,
            month: 9,
            day: 1,
            hour: 15,
            minute: 0,
        };
        assert_eq!(d.to_u32(), 20260901);
    }

    #[test]
    fn adjusted_multipliers_no_xdxr() {
        use super::Adj;
        use super::adjusted_multipliers;
        // 无除权：scale 恒 1 → 前/后复权乘数均为 1
        let days = vec![kd(20260101, 10.0), kd(20260102, 11.0), kd(20260105, 12.0)];
        assert_eq!(
            adjusted_multipliers(&days, &[], Adj::Qfq),
            vec![1.0, 1.0, 1.0]
        );
        assert_eq!(
            adjusted_multipliers(&days, &[], Adj::Hfq),
            vec![1.0, 1.0, 1.0]
        );
    }

    #[test]
    fn adjusted_multipliers_split() {
        use super::Adj;
        use super::adjusted_multipliers;
        // 10 送 10：除权参考价 = 前收/2 → scale 减半
        let days = vec![
            kd(20260105, 20.0),
            kd(20260106, 10.0), // 除权日，前收 20 → 参考价 10（10送10）
            kd(20260107, 10.5),
        ];
        let xdxrs = vec![xdxr(20260106, 0.0, 10.0, 0.0, 0.0)]; // 每 10 股送 10
        // 后复权：除权后价格放大回除权前量纲
        let m = adjusted_multipliers(&days, &xdxrs, Adj::Hfq);
        assert!((m[0] - 1.0).abs() < 1e-9);
        assert!((m[1] - 2.0).abs() < 1e-9, "除权日乘数应为 2，实际 {m:?}");
        assert!((m[2] - 2.0).abs() < 1e-9);
        // 前复权：最新价不变，历史价按比例缩小
        let m = adjusted_multipliers(&days, &xdxrs, Adj::Qfq);
        assert!((m[2] - 1.0).abs() < 1e-9);
        assert!((m[0] - 0.5).abs() < 1e-9, "除权前乘数应为 0.5，实际 {m:?}");
    }

    #[test]
    fn adjusted_multipliers_cash_and_rights() {
        use super::Adj;
        use super::adjusted_multipliers;
        // 每 10 股派 5 元 + 每 10 股配 3 股（配股价 2 元）：
        // 参考价 = (20*10 - 5 + 3*2) / (10 + 3) = 201/13 ≈ 15.4615 → 四舍五入 15.46
        let days = vec![kd(20260105, 20.0), kd(20260106, 15.0), kd(20260107, 15.5)];
        let xdxrs = vec![xdxr(20260106, 5.0, 0.0, 3.0, 2.0)];
        let m = adjusted_multipliers(&days, &xdxrs, Adj::Hfq);
        assert!((m[1] - (20.0 / 15.46)).abs() < 1e-9, "实际 {m:?}");
    }

    /// 除权参考价与交易所口径一致（四舍五入到分）。
    #[test]
    fn ex_right_reference_rounds_to_fen() {
        use super::ex_right_reference;
        // (20*10 - 5 + 3*2) / 13 = 201/13 = 15.4615… → 15.46
        let ev = xdxr(20260106, 5.0, 0.0, 3.0, 2.0);
        assert_eq!(ex_right_reference(20.0, &ev), 15.46);
        // 无变动（分母为 0）→ 原样返回
        let none = xdxr(20260106, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(ex_right_reference(20.0, &none), 20.0);
    }

    #[test]
    fn adjusted_multipliers_suspended_xdxr_day() {
        use super::Adj;
        use super::adjusted_multipliers;
        // 除权日为停牌日（无 K 线）：事件顺延到下一交易日处理，
        // 仍用除权前最后一个交易日收盘作为 preclose
        let days = vec![kd(20260102, 20.0), kd(20260106, 10.0)];
        let xdxrs = vec![xdxr(20260105, 0.0, 10.0, 0.0, 0.0)]; // 01-05 除权，01-06 才恢复交易
        let m = adjusted_multipliers(&days, &xdxrs, Adj::Hfq);
        assert!((m[0] - 1.0).abs() < 1e-9);
        assert!(
            (m[1] - 2.0).abs() < 1e-9,
            "停牌顺延后乘数应为 2，实际 {m:?}"
        );
    }

    fn kd(date: u32, close: f64) -> super::KlineData<'static> {
        let (y, m, d) = (date / 10000, date / 100 % 100, date % 100);
        super::KlineData {
            dt: crate::tcp::helper::DateTime {
                year: y as u16,
                month: m as u16,
                day: d as u16,
                hour: 15,
                minute: 0,
            },
            code: "",
            open: close,
            close,
            high: close,
            low: close,
            vol: 0.0,
            amount: 0.0,
        }
    }

    fn xdxr(date: u32, fh: f32, sg: f32, pg: f32, pgj: f32) -> super::XdxrData {
        super::XdxrData {
            market: 1,
            code: "600000".into(),
            date,
            category: 1,
            fh_qltp: fh,
            pgj_qzgb: pgj,
            sg_hltp: sg,
            pg_hzgb: pg,
        }
    }

    /// 逐笔翻页：从最新页往前取，拼接后按时间升序；不足一页即停。
    #[test]
    fn collect_ticks_orders_pages_oldest_first() {
        use super::{TransactionData, collect_ticks};
        let tick = |t: &str| TransactionData {
            time: t.into(),
            ..Default::default()
        };
        // 共 5 笔，步长 2：start=0 → 最新两笔，start=2 → 中间两笔，start=4 → 最早一笔
        let mut starts = vec![];
        let all = collect_ticks(2, |start| {
            starts.push(start);
            Ok(match start {
                0 => vec![tick("14:58"), tick("14:59")],
                2 => vec![tick("10:00"), tick("11:00")],
                4 => vec![tick("09:30")],
                _ => vec![],
            })
        })
        .unwrap();
        assert_eq!(starts, [0, 2, 4]);
        let times: Vec<_> = all.iter().map(|t| t.time.as_str()).collect();
        assert_eq!(times, ["09:30", "10:00", "11:00", "14:58", "14:59"]);

        // 恰好整页时多取一页空页后停止
        let mut calls = 0;
        let all = collect_ticks(2, |start| {
            calls += 1;
            Ok(if start == 0 {
                vec![tick("a"), tick("b")]
            } else {
                vec![]
            })
        })
        .unwrap();
        assert_eq!((all.len(), calls), (2, 2));
    }

    /// 首根 K 线当日及之前的除权事件不参与复权：后复权首日价 = 实际价。
    #[test]
    fn adjusted_multipliers_ignores_events_up_to_first_bar() {
        use super::Adj;
        use super::adjusted_multipliers;
        let days = vec![kd(20260105, 20.0), kd(20260106, 21.0)];
        // 一个在首根之前、一个恰在首根当日
        let xdxrs = vec![
            xdxr(20250601, 0.0, 10.0, 0.0, 0.0),
            xdxr(20260105, 5.0, 0.0, 0.0, 0.0),
        ];
        assert_eq!(
            adjusted_multipliers(&days, &xdxrs, Adj::Hfq),
            vec![1.0, 1.0]
        );
        assert_eq!(
            adjusted_multipliers(&days, &xdxrs, Adj::Qfq),
            vec![1.0, 1.0]
        );
    }

    /// 回退辅助函数：今日日期必须为合法 8 位 YYYYMMDD，且为北京时间的日期。
    #[test]
    fn today_yyyymmdd_format() {
        use super::today_yyyymmdd;
        let v = today_yyyymmdd();
        assert!((20000101..=20991231).contains(&v), "非法日期: {v}");
        let beijing = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
        let today = chrono::Utc::now().with_timezone(&beijing).date_naive();
        let expected = {
            use chrono::Datelike;
            (today.year() * 10000 + today.month() as i32 * 100 + today.day() as i32) as u32
        };
        assert_eq!(v, expected);
    }
    /// 生成同一天内 `n` 根互不重复的分钟级时间戳（**降序**，模拟服务器页内乱序）。
    fn minute_page(date: u32, n: usize) -> Vec<crate::tcp::helper::DateTime> {
        let (y, m, d) = (date / 10000, date / 100 % 100, date % 100);
        (0..n)
            .rev()
            .map(|i| crate::tcp::helper::DateTime {
                year: y as u16,
                month: m as u16,
                day: d as u16,
                hour: (i / 60) as u16,
                minute: (i % 60) as u16,
            })
            .collect()
    }

    /// 用假分页数据驱动泛型翻页函数，返回（结果, 实际请求过的 start 偏移）。
    fn run_paged(
        pages: Vec<Vec<crate::tcp::helper::DateTime>>,
        begin: Option<u32>,
        end: Option<u32>,
    ) -> (Vec<crate::tcp::helper::DateTime>, Vec<u16>) {
        let mut starts = Vec::new();
        let out = super::fetch_paged_range(
            |start| {
                starts.push(start);
                let idx = (start / super::BARS_PAGE) as usize;
                Ok(pages.get(idx).cloned().unwrap_or_default())
            },
            |d| *d,
            begin,
            end,
        )
        .unwrap();
        (out, starts)
    }

    /// 重叠页去重 + 末端统一升序 + 不足一页即历史尽头。
    #[test]
    fn paged_range_dedup_and_ascending() {
        let full = super::BARS_PAGE as usize;
        // 第 2 页前半与第 1 页重叠（翻页偏移重叠），后半为更早一天
        let mut page1 = minute_page(20260105, full / 2);
        page1.extend(minute_page(20260102, full / 2));
        let pages = vec![
            minute_page(20260105, full),
            page1,
            minute_page(20260101, 10),
        ];
        let (out, starts) = run_paged(pages, None, None);
        assert_eq!(starts, vec![0, 800, 1600]);
        assert_eq!(out.len(), full + full / 2 + 10);
        assert!(out.windows(2).all(|w| w[0] < w[1]), "应严格升序且无重复");
        assert_eq!(out.first().unwrap().to_u32(), 20260101);
        assert_eq!(out.last().unwrap().to_u32(), 20260105);
    }

    /// `[begin, end]` 闭区间窗口过滤。
    #[test]
    fn paged_range_window_filter() {
        let full = super::BARS_PAGE as usize;
        let pages = vec![
            minute_page(20260105, full),
            minute_page(20260102, full),
            minute_page(20260101, 10),
        ];
        let (out, starts) = run_paged(pages.clone(), Some(20260102), Some(20260102));
        // 第 2 页最旧 == begin（未早于 begin）→ 继续翻到不足一页的第 3 页
        assert_eq!(starts, vec![0, 800, 1600]);
        assert_eq!(out.len(), full);
        assert!(out.iter().all(|d| d.to_u32() == 20260102));

        let (out, _) = run_paged(pages, None, Some(20260101));
        assert_eq!(out.len(), 10);
    }

    /// 乱码年份页：终止翻页，残留由窗口过滤丢弃。
    #[test]
    fn paged_range_garbage_year_stops() {
        let full = super::BARS_PAGE as usize;
        let pages = vec![
            minute_page(20260105, full),
            minute_page(20990101, full), // 历史深度耗尽后的乱码日期
            minute_page(20260101, full), // 不应再被请求
        ];
        let (out, starts) = run_paged(pages.clone(), None, Some(20261231));
        assert_eq!(starts, vec![0, 800]);
        assert_eq!(out.len(), full);
        assert!(out.iter().all(|d| d.to_u32() == 20260105));

        // 年份早于 2000 同样视为乱码
        let pages = vec![minute_page(20260105, full), minute_page(19040101, full)];
        let (out, starts) = run_paged(pages, Some(20260101), None);
        assert_eq!(starts, vec![0, 800]);
        assert_eq!(out.len(), full);
    }

    /// 页内最旧一根早于 `begin` 时停止翻页。
    #[test]
    fn paged_range_stops_when_oldest_before_begin() {
        let full = super::BARS_PAGE as usize;
        let pages = vec![
            minute_page(20260105, full),
            minute_page(20251230, full),
            minute_page(20251229, full), // 不应再被请求
        ];
        let (out, starts) = run_paged(pages, Some(20260101), None);
        assert_eq!(starts, vec![0, 800]);
        assert_eq!(out.len(), full);
        assert!(out.iter().all(|d| d.to_u32() == 20260105));
    }

    /// 页数上限兜底 + 拉取错误原样上抛。
    #[test]
    fn paged_range_page_cap_and_error() {
        let full = super::BARS_PAGE as usize;
        let mut calls = 0usize;
        let out = super::fetch_paged_range(
            |_start| {
                calls += 1;
                // 每页一个不同的日期，永远满页
                Ok(minute_page(20260101 + calls as u32, full))
            },
            |d| *d,
            None,
            None,
        )
        .unwrap();
        assert_eq!(calls, super::MAX_BARS_PAGES);
        assert_eq!(out.len(), full * super::MAX_BARS_PAGES);

        let err = super::fetch_paged_range(
            |start| {
                if start == 0 {
                    Ok(minute_page(20260105, full))
                } else {
                    Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "boom"))
                }
            },
            |d: &crate::tcp::helper::DateTime| *d,
            None,
            None,
        )
        .unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
    }
}
