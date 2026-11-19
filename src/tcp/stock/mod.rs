mod kline;
pub use kline::{Kline, KlineData};

mod index_kline;
pub use index_kline::{IndexKline, IndexKlineData};

mod xdxr;
pub use xdxr::*;

mod quotes;
pub use quotes::{AuctionMatch, QuoteData, SecurityQuotes};

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
    Block, BlockInfoChunk, BlockInfoMeta, BlockKind, BlockRecord, download_file, get_block_info,
    get_blocks, parse_block_content, parse_blocks,
};

mod company_info;
pub use company_info::{CompanyInfoCategory, CompanyInfoCategoryItem, CompanyInfoContent};

mod client;
pub use client::{Adj, BatchBars, BatchKline, Client, ex_right_reference};

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

/// 证券的报价精度类别：决定行情、分时、逐笔里价格整数的换算除数。
///
/// 协议里这些价格都是整数，但单位随品种不同。只有 K 线统一以千分之一元为
/// 单位、不受影响；行情快照、分时、逐笔若一律按「分」换算，基金会放大 10 倍、
/// 债券放大 100 倍（逐笔为 10 倍）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceKind {
    /// 股票、指数、深市 B 股、北交所：两位小数。
    Stock,
    /// 基金（ETF / LOF / REIT / 货币基金）与沪市 B 股：三位小数。
    Fund,
    /// 债券（可转债、国债、逆回购）。
    Bond,
}

impl PriceKind {
    /// 行情快照、当日/历史分时的价格除数。
    pub(crate) fn quote_divisor(self) -> f64 {
        match self {
            Self::Stock => 100.0,
            Self::Fund => 1000.0,
            Self::Bond => 10000.0,
        }
    }

    /// 当日/历史逐笔的价格除数（债券在逐笔里比行情少一位）。
    pub(crate) fn tick_divisor(self) -> f64 {
        match self {
            Self::Stock => 100.0,
            Self::Fund | Self::Bond => 1000.0,
        }
    }
}

/// 按市场与代码前缀判定报价精度类别。
///
/// 规则来自 2026-11-18 盘中实测（以 K 线收盘价为基准逐类比对行情/分时/逐笔）：
///
/// | 市场 | 前缀 | 类别 |
/// |---|---|---|
/// | 沪 (1) | `5`（50x/51x/58x 基金、ETF、REIT）、`900`（B 股） | [`PriceKind::Fund`] |
/// | 沪 (1) | `1`（110/113/118 可转债等）、`20`（204 逆回购）、`01`/`02`（国债） | [`PriceKind::Bond`] |
/// | 深 (0) | `15`/`16`/`18`（ETF、LOF、封闭式基金） | [`PriceKind::Fund`] |
/// | 深 (0) | `10`/`11`/`12`/`13`（国债、可转债、逆回购） | [`PriceKind::Bond`] |
/// | 其余 | 股票、指数（沪 `000`/`880`、深 `399`）、深市 B 股 `200`、北交所 | [`PriceKind::Stock`] |
///
/// 实测覆盖：沪 `018`/`019`、`110`/`111`/`113`/`118`/`132`、`204`、`50x`~`58x`、`900`；
/// 深 `117`/`123`/`127`/`128`/`131`、`158`/`159`、`160`~`169`、`180`/`181`。
/// 同族里当日无成交、未能比对的前缀（如沪 `010`/`020`/`12x`、深 `10x`/`110`~`116`）
/// 按所在族归类。
///
/// ## 示例
/// ```
/// use rustdx_complete::tcp::stock::{PriceKind, price_kind};
///
/// assert_eq!(price_kind(1, "600519"), PriceKind::Stock);
/// assert_eq!(price_kind(1, "510300"), PriceKind::Fund);
/// assert_eq!(price_kind(0, "159915"), PriceKind::Fund);
/// assert_eq!(price_kind(1, "113042"), PriceKind::Bond);
/// assert_eq!(price_kind(1, "000001"), PriceKind::Stock); // 上证指数
/// assert_eq!(price_kind(0, "200012"), PriceKind::Stock); // 深市 B 股
/// ```
pub fn price_kind(market: u16, code: &str) -> PriceKind {
    let starts = |prefixes: &[&str]| prefixes.iter().any(|p| code.starts_with(p));
    match market {
        1 if starts(&["5", "900"]) => PriceKind::Fund,
        1 if starts(&["1", "20", "01", "02"]) => PriceKind::Bond,
        0 if starts(&["15", "16", "18"]) => PriceKind::Fund,
        0 if starts(&["10", "11", "12", "13"]) => PriceKind::Bond,
        _ => PriceKind::Stock,
    }
}

/// 获取市场全部证券列表（自动按 1000 条分页聚合）。
///
/// 对应 mootdx 的 `stocks(market)`。仅支持沪深（market 0/1）；北交所返回
/// [`std::io::ErrorKind::Unsupported`]。
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
    // 北交所（market 2）：数量接口有回应，但列表请求服务器不回应（2026-11-18 实测
    // 4 台服务器一致），与其等到超时再重试，不如立即说明。
    if market == 2 {
        return Err(crate::Error::Io(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "行情服务器不提供北交所（market=2）的证券列表",
        )));
    }
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
#[allow(deprecated)]
pub use concept_mapping::{ConceptStock, get_concept_info, get_concept_names, get_concept_stocks};

// 数据验证模块
pub mod validator;
pub use validator::{
    DataLocation, Validatable, ValidationLevel, ValidationResult, detect_anomalies,
    validate_finance_consistency, validate_kline_continuity,
};
