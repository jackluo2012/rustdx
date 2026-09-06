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
    HistoryMinuteTime, HistoryTransaction, IndexKline, Kline, KlineData, MinuteTimeData, QuoteData,
    SecurityListData, TransactionData, Xdxr, XdxrData,
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

/// `k_batch` 的任务队列：`(输入序号, market, code)`。
type BatchQueue<'a> = Arc<Mutex<VecDeque<(usize, u16, &'a str)>>>;
/// `k_batch` 的结果槽：按输入序号存放各股票的拉取结果。
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
    pub fn quotes(&mut self, stocks: &[(u16, &str)]) -> std::io::Result<Vec<QuoteData>> {
        let mut quotes = SecurityQuotesRef::new(stocks.to_vec());
        quotes
            .recv_parsed(&mut self.tcp)
            .map_err(|e| ctx_err(e, format_args!("Client::quotes(n={})", stocks.len())))?;
        Ok(quotes.result().to_vec())
    }

    /// 股票K线（mootdx `bars`）。
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

    /// 指数K线（mootdx `index`/`index_bars`），含涨跌家数。
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

    /// 按日期区间拉取日K线（mootdx `k`）。
    ///
    /// 自动翻页直到覆盖 `begin`，并过滤出 `[begin, end]`（格式 YYYYMMDD）的数据。
    /// `begin`/`end` 传 `None` 表示不设下/上限。
    pub fn k<'a>(
        &mut self,
        market: u16,
        code: &'a str,
        begin: Option<u32>,
        end: Option<u32>,
    ) -> std::io::Result<Vec<super::KlineData<'a>>> {
        fetch_k(&mut self.tcp, market, code, begin, end)
            .map_err(|e| ctx_err(e, format_args!("Client::k(market={market}, code={code})")))
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
        if stocks.is_empty() {
            return Ok(Vec::new());
        }
        let workers = match max_parallel {
            0 => std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4),
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
        let results: BatchResults<'a> =
            Arc::new(Mutex::new((0..stocks.len()).map(|_| None).collect()));

        // 固定 worker 数 = 连接池大小：每个 worker 持一条连接处理队列，
        // 避免反复建连/归还的锁竞争，也不会触发连接池饱和错误。
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
                        let r = conn.execute(|tcp| {
                            fetch_k(tcp, market, code, begin, end).map_err(|e| {
                                ctx_err(e, format_args!("k_batch(market={market}, code={code})"))
                            })
                        });
                        results.lock().unwrap()[idx] = Some(r);
                    }
                });
            }
        });

        let mut out = Vec::with_capacity(stocks.len());
        for (i, &(market, code)) in stocks.iter().enumerate() {
            let result = results.lock().unwrap()[i].take().unwrap_or_else(|| {
                Err(std::io::Error::other("worker 未能处理该股票（连接建立失败）"))
            });
            out.push(BatchKline {
                market,
                code,
                result,
            });
        }
        Ok(out)
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
                let d = DateTime::to_u32(bar.dt.clone());
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
    /// ⚠️ 部分服务器 2026 起分时响应格式已变（数据前新增 market+code 头、编码改变），
    /// 旧协议解析异常会返回空数据。本方法内置**自动回退**：当日分时接口解析为空
    /// （协议不匹配）时，改用「今日历史分时接口」（[`HistoryMinuteTime`] 查询当天）
    /// 取数——历史分时接口协议稳定、实测 240 点完整，这是 pytdx/mootdx 生态的
    /// 同类解决方案（参见 pytdx#148、xmtdx）。
    ///
    /// 当日分时与今日历史分时返回相同的数据结构（每分钟一个价格/成交量点）。
    pub fn minute(&mut self, market: u16, code: &str) -> std::io::Result<Vec<MinuteTimeData>> {
        let ctx = format_args!("Client::minute(market={market}, code={code})");
        let mut mt = super::MinuteTime::new(market, code);
        mt.recv_parsed(&mut self.tcp)
            .map_err(|e| ctx_err(e, ctx))?;
        if !mt.result().is_empty() {
            return Ok(mt.result().to_vec());
        }

        // 回退：当日分时接口不可用 → 今日历史分时接口（协议稳定）
        let mut hmt = HistoryMinuteTime::new(market, code, today_yyyymmdd());
        hmt.recv_parsed(&mut self.tcp).map_err(|e| {
            ctx_err(e, format_args!("{ctx}（回退到今日历史分时）"))
        })?;
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
            ctx_err(e, format_args!("Client::xdxr(market={market}, code={code})"))
        })?;
        Ok(xdxr.result().to_vec())
    }

    /// 板块文件（mootdx `block`），如 `"block_gn.dat"` 概念板块。
    pub fn block(&mut self, block_file: &str) -> crate::Result<Vec<BlockRecord>> {
        super::get_block_info(&mut self.tcp, block_file).map_err(|e| {
            crate::Error::Io(std::io::Error::other(format!("Client::block({block_file}): {e}")))
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
    pub fn stocks(&mut self, market: u16) -> crate::Result<Vec<SecurityListData>> {
        super::stocks(&mut self.tcp, market).map_err(|e| {
            crate::Error::Io(std::io::Error::other(format!("Client::stocks(market={market}): {e}")))
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
            content.recv_parsed(&mut self.tcp).map_err(|e| {
                ctx_err(e, format_args!("{ctx}（栏目 {}）", item.name))
            })?;
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

/// 今日日期，格式 YYYYMMDD（用于当日分时回退到今日历史分时）。
fn today_yyyymmdd() -> u32 {
    let today = chrono::Local::now().date_naive();
    (today.year() * 10000 + today.month() as i32 * 100 + today.day() as i32) as u32
}

/// 给 IO 错误附加接口上下文（保留错误 kind，便于调用方按 kind 分支处理）。
fn ctx_err(e: std::io::Error, ctx: std::fmt::Arguments<'_>) -> std::io::Error {
    std::io::Error::new(e.kind(), format!("{ctx}: {e}"))
}

/// 避免与 facade 文档中的 `SecurityQuotes` 混淆的内部别名。
use super::SecurityQuotes as SecurityQuotesRef;

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
            if let Some(oldest) = all
                .iter()
                .map(|b| DateTime::to_u32(b.dt.clone()))
                .min()
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
        let d = DateTime::to_u32(bar.dt.clone());
        begin.is_none_or(|b| d >= b) && end.is_none_or(|e| d <= e)
    });
    // 契约：按日期升序（文档承诺「统一按升序存放」）
    all.sort_by_key(|bar| DateTime::to_u32(bar.dt.clone()));
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
    let mut events: Vec<(u32, f32, f32, f32, f32)> = xdxrs
        .iter()
        .filter(|x| x.category == 1)
        .map(|x| (x.date, x.fh_qltp, x.sg_hltp, x.pg_hzgb, x.pgj_qzgb))
        .collect();
    events.sort_by_key(|e| e.0);
    let mut ev = events.iter().peekable();

    let mut scales: Vec<f64> = Vec::with_capacity(days.len());
    let mut scale = 1.0f64;
    let mut preclose = days.first().map(|d| d.close).unwrap_or(0.0);
    for d in days {
        let date = DateTime::to_u32(d.dt.clone());
        // 应用所有日期 ≤ 当前交易日且尚未应用的除权事件（含停牌场景：
        // 除权日无 K 线时顺延到下一个交易日）
        while let Some(&&(edate, fh, sg, pg, pgj)) = ev.peek() {
            if edate > date {
                break;
            }
            let preclose_adj = (preclose * 10.0 - fh as f64 + pg as f64 * pgj as f64)
                / (10.0 + pg as f64 + sg as f64);
            if preclose_adj > 0.0 && preclose > 0.0 {
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
        use super::adjusted_multipliers;
        use super::Adj;
        // 无除权：scale 恒 1 → 前/后复权乘数均为 1
        let days = vec![
            kd(20260101, 10.0),
            kd(20260102, 11.0),
            kd(20260105, 12.0),
        ];
        assert_eq!(adjusted_multipliers(&days, &[], Adj::Qfq), vec![1.0, 1.0, 1.0]);
        assert_eq!(adjusted_multipliers(&days, &[], Adj::Hfq), vec![1.0, 1.0, 1.0]);
    }

    #[test]
    fn adjusted_multipliers_split() {
        use super::adjusted_multipliers;
        use super::Adj;
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
        use super::adjusted_multipliers;
        use super::Adj;
        // 每 10 股派 5 元 + 每 10 股配 3 股（配股价 2 元）：
        // 参考价 = (20*10 - 5 + 3*2) / (10 + 3) = 201/13 ≈ 15.46
        let days = vec![kd(20260105, 20.0), kd(20260106, 15.0), kd(20260107, 15.5)];
        let xdxrs = vec![xdxr(20260106, 5.0, 0.0, 3.0, 2.0)];
        let m = adjusted_multipliers(&days, &xdxrs, Adj::Hfq);
        assert!(
            (m[1] - (13.0 * 20.0 / 201.0)).abs() < 1e-9,
            "实际 {m:?}"
        );
    }

    #[test]
    fn adjusted_multipliers_suspended_xdxr_day() {
        use super::adjusted_multipliers;
        use super::Adj;
        // 除权日为停牌日（无 K 线）：事件顺延到下一交易日处理，
        // 仍用除权前最后一个交易日收盘作为 preclose
        let days = vec![kd(20260102, 20.0), kd(20260106, 10.0)];
        let xdxrs = vec![xdxr(20260105, 0.0, 10.0, 0.0, 0.0)]; // 01-05 除权，01-06 才恢复交易
        let m = adjusted_multipliers(&days, &xdxrs, Adj::Hfq);
        assert!((m[0] - 1.0).abs() < 1e-9);
        assert!((m[1] - 2.0).abs() < 1e-9, "停牌顺延后乘数应为 2，实际 {m:?}");
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

    /// 回退辅助函数：今日日期必须为合法 8 位 YYYYMMDD，且与 chrono 本地日期一致。
    #[test]
    fn today_yyyymmdd_format() {
        use super::today_yyyymmdd;
        let v = today_yyyymmdd();
        assert!((20000101..=20991231).contains(&v), "非法日期: {v}");
        let today = chrono::Local::now().date_naive();
        let expected = {
            use chrono::Datelike;
            (today.year() * 10000 + today.month() as i32 * 100 + today.day() as i32) as u32
        };
        assert_eq!(v, expected);
    }
}
