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
    HistoryMinuteTime, HistoryTransaction, IndexKline, Kline, MinuteTimeData, QuoteData,
    SecurityListData, TransactionData, Xdxr,
};
use crate::tcp::helper::DateTime;
use crate::tcp::{Tcp, TcpConfig, Tdx};
use chrono::Datelike;

/// 高层客户端：持有连接，提供与 mootdx 对齐的语义化 API。
#[derive(Debug)]
pub struct Client {
    /// 底层连接（需要更细粒度控制时可直接使用）。
    pub tcp: Tcp,
}

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
        quotes.recv_parsed(&mut self.tcp)?;
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
        kline.recv_parsed(&mut self.tcp)?;
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
        kline.recv_parsed(&mut self.tcp)?;
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
        const PAGE: u16 = 800;
        const CATEGORY_DAY: u16 = 9;

        let mut all = Vec::new();
        let mut start = 0u16;
        loop {
            let bars = self.bars(market, code, CATEGORY_DAY, start, PAGE)?;
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
        let mut mt = super::MinuteTime::new(market, code);
        mt.recv_parsed(&mut self.tcp)?;
        if !mt.result().is_empty() {
            return Ok(mt.result().to_vec());
        }

        // 回退：当日分时接口不可用 → 今日历史分时接口（协议稳定）
        let mut hmt = HistoryMinuteTime::new(market, code, today_yyyymmdd());
        hmt.recv_parsed(&mut self.tcp)?;
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
        mt.recv_parsed(&mut self.tcp)?;
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
        tx.recv_parsed(&mut self.tcp)?;
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
        tx.recv_parsed(&mut self.tcp)?;
        Ok(tx.result().to_vec())
    }

    /// 财务信息（mootdx `finance`）。
    pub fn finance(&mut self, market: u16, code: &str) -> std::io::Result<FinanceInfoData> {
        let mut fin = super::FinanceInfo::new(market as u8, code);
        fin.recv_parsed(&mut self.tcp)?;
        Ok(fin.result().first().cloned().unwrap_or_default())
    }

    /// 除权除息信息（mootdx `xdxr`）。
    pub fn xdxr(&mut self, market: u16, code: &str) -> std::io::Result<Vec<super::XdxrData>> {
        let mut xdxr = Xdxr::new(market, code);
        xdxr.recv_parsed(&mut self.tcp)?;
        Ok(xdxr.result().to_vec())
    }

    /// 板块文件（mootdx `block`），如 `"block_gn.dat"` 概念板块。
    pub fn block(&mut self, block_file: &str) -> crate::Result<Vec<BlockRecord>> {
        super::get_block_info(&mut self.tcp, block_file)
    }

    /// 证券数量（mootdx `stock_count`）。
    pub fn stock_count(&mut self, market: u16) -> std::io::Result<u16> {
        let mut sc = super::super::basic::SecurityCount::new(market);
        Ok(*sc.recv_parsed(&mut self.tcp)?)
    }

    /// 全部证券列表（mootdx `stocks`），自动分页聚合。
    pub fn stocks(&mut self, market: u16) -> crate::Result<Vec<SecurityListData>> {
        super::stocks(&mut self.tcp, market)
    }

    /// F10 公司资料全部栏目（mootdx `F10`），返回 `(栏目名, 内容)`。
    pub fn f10(&mut self, market: u16, code: &str) -> std::io::Result<Vec<(String, String)>> {
        let mut cat = CompanyInfoCategory::new(market, code);
        cat.recv_parsed(&mut self.tcp)?;
        let mut result = Vec::with_capacity(cat.result().len());
        for item in cat.result() {
            let mut content =
                CompanyInfoContent::new(market, code, &item.filename, item.start, item.length);
            content.recv_parsed(&mut self.tcp)?;
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
        cat.recv_parsed(&mut self.tcp)?;
        Ok(cat.result().to_vec())
    }
}

/// 今日日期，格式 YYYYMMDD（用于当日分时回退到今日历史分时）。
fn today_yyyymmdd() -> u32 {
    let today = chrono::Local::now().date_naive();
    (today.year() * 10000 + today.month() as i32 * 100 + today.day() as i32) as u32
}

/// 避免与 facade 文档中的 `SecurityQuotes` 混淆的内部别名。
use super::SecurityQuotes as SecurityQuotesRef;

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

    /// 回退辅助函数：今日日期必须为合法 8 位 YYYYMMDD，且与 chrono 本地日期一致。
    #[test]
    fn today_yyyymmdd_format() {
        use super::today_yyyymmdd;
        let v = today_yyyymmdd();
        assert!(v >= 20000101 && v <= 20991231, "非法日期: {v}");
        let today = chrono::Local::now().date_naive();
        let expected = {
            use chrono::Datelike;
            (today.year() * 10000 + today.month() as i32 * 100 + today.day() as i32) as u32
        };
        assert_eq!(v, expected);
    }
}
