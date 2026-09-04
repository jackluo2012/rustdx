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
            // 按时间正序插入（服务器返回的是从 start 往回数的数据，这里统一按升序存放）
            let mut page: Vec<_> = bars;
            page.reverse();
            all.splice(0..0, page);
            if n < PAGE as usize {
                break; // 历史尽头
            }
            if let Some(begin) = begin {
                // 最旧的一页数据已经早于 begin，停止翻页
                if let Some(oldest) = all.first()
                    && DateTime::to_u32(oldest.dt.clone()) < begin
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
        Ok(all)
    }

    /// 当日分时（mootdx `minute`）。⚠️ 部分服务器协议已变，异常时返回空。
    pub fn minute(&mut self, market: u16, code: &str) -> std::io::Result<Vec<MinuteTimeData>> {
        let mut mt = super::MinuteTime::new(market, code);
        mt.recv_parsed(&mut self.tcp)?;
        Ok(mt.result().to_vec())
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
}
