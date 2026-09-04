mod kline;
pub use kline::{Kline, KlineData};

mod index_kline;
pub use index_kline::{IndexKline, IndexKlineData};

mod xdxr;
pub use xdxr::*;

mod quotes;
pub use quotes::{QuoteData, SecurityQuotes};

mod security_list;
pub use security_list::{SecurityList, SecurityListData};

mod minute_time;
pub use minute_time::{MinuteTime, MinuteTimeData};

mod transaction;
pub use transaction::{Transaction, TransactionData};

mod finance_info;
pub use finance_info::{FinanceInfo, FinanceInfoData};

mod history_minute_time;
pub use history_minute_time::HistoryMinuteTime;

mod history_transaction;
pub use history_transaction::HistoryTransaction;

mod block_info;
pub use block_info::{
    BlockInfoChunk, BlockInfoMeta, BlockRecord, get_block_info, parse_block_content,
};

mod company_info;
pub use company_info::{CompanyInfoCategory, CompanyInfoCategoryItem, CompanyInfoContent};

mod client;
pub use client::Client;

/// 根据股票代码推断市场代码（对应 mootdx 的 `get_stock_market`）。
///
/// - `6` 开头 → 沪市（`1`），如 600xxx、688xxx
/// - `0` / `3` 开头 → 深市（`0`），如 000xxx、300xxx
/// - 其他（4/8 开头的北交所等）暂不支持，返回 `None`
///
/// ## 示例
/// ```
/// use rustdx_complete::tcp::stock::market_of;
///
/// assert_eq!(market_of("600519"), Some(1));
/// assert_eq!(market_of("300750"), Some(0));
/// assert_eq!(market_of("830000"), None);
/// ```
pub fn market_of(code: &str) -> Option<u16> {
    match code.as_bytes().first()? {
        b'6' => Some(1),
        b'0' | b'3' => Some(0),
        _ => None,
    }
}

/// 获取市场全部证券列表（自动按 1000 条分页聚合）。
///
/// 对应 mootdx 的 `stocks(market)`。
///
/// ## 示例
/// ```ignore
/// use rustdx_complete::tcp::{Tcp, stock::stocks};
///
/// let mut tcp = Tcp::new()?;
/// let all = stocks(&mut tcp, 0)?; // 深市全部证券
/// println!("深市共 {} 只证券", all.len());
/// ```
pub fn stocks(tcp: &mut crate::tcp::Tcp, market: u16) -> crate::Result<Vec<SecurityListData>> {
    use crate::tcp::Tdx;
    let count = {
        let mut sc = crate::tcp::basic::SecurityCount::new(market);
        *sc.recv_parsed(tcp)?
    };
    let mut all = Vec::with_capacity(count as usize);
    let mut start = 0u16;
    while (start as usize) < count as usize {
        let mut list = SecurityList::new(market, start);
        list.recv_parsed(tcp)?;
        let n = list.result().len();
        if n == 0 {
            break;
        }
        all.extend(list.result().iter().cloned());
        start += n as u16;
    }
    Ok(all)
}

mod industry_mapping;
pub use industry_mapping::{get_industry_info, get_industry_name, get_province_name};

mod concept_mapping;
pub use concept_mapping::{ConceptStock, get_concept_info, get_concept_names, get_concept_stocks};

// 数据验证模块
pub mod validator;
pub use validator::{
    DataLocation, Validatable, ValidationLevel, ValidationResult, detect_anomalies,
    validate_finance_consistency, validate_kline_continuity,
};
