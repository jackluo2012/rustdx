//! A 股涨跌停规则：板块判定、涨跌停价、封板/炸板判型、连板高度。
//!
//! 纯函数、无 IO：调用方负责给出正确输入——原始不复权价格、正确的昨收盘
//! （除权日用 [`ex_right_reference`](crate::tcp::stock::ex_right_reference)）。
//!
//! 涨跌停价规则：`前收盘 × (1 ± 幅度)`，**四舍五入到分**（与交易所/行情软件
//! 一致）。新股上市首日等特殊涨跌幅不在本模块，调用方应跳过首日
//! （如日线 `prev_close == 0` 的首行）。
//!
//! ## 示例
//! ```ignore
//! use rustdx_complete::limit::{self, LimitStatus};
//!
//! let board = limit::board_of("000428").unwrap();
//! let up = limit::limit_up_price(4.26, board, false).unwrap(); // 4.69
//! assert_eq!(
//!     limit::limit_status(4.69, 4.69, up),
//!     Some(LimitStatus::Sealed)
//! );
//! ```

/// 个股所属板块（按代码前缀判定，决定涨跌幅）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Board {
    /// 沪深主板（60x / 000·001·002·003）±10%
    Main,
    /// 创业板（300·301·302）±20%
    ChiNext,
    /// 科创板（688·689）±20%
    Star,
    /// 北交所（43·82·83·87·88·92 开头）±30%
    Beijing,
}

impl Board {
    /// 含 ST 修正的涨跌停幅度（%）。
    ///
    /// 主板 ST ±5%；创业板/科创板 ST 仍 ±20%；北交所无 ST ±5% 规则。
    /// 新股上市首日等特殊规则不在本模块。
    pub fn limit_pct_for(self, st: bool) -> f64 {
        match self {
            Board::Main if st => 5.0,
            Board::Main => 10.0,
            Board::ChiNext | Board::Star => 20.0,
            Board::Beijing => 30.0,
        }
    }

    /// 落库/日志用小写串。
    pub fn as_str(self) -> &'static str {
        match self {
            Board::Main => "main",
            Board::ChiNext => "chinext",
            Board::Star => "star",
            Board::Beijing => "beijing",
        }
    }
}

/// 按代码前缀判定板块。代码应为 6 位数字；无法识别（如指数、基金、债券）
/// 返回 `None`。
pub fn board_of(code: &str) -> Option<Board> {
    let b = code.as_bytes();
    if b.len() != 6 || !b.iter().all(u8::is_ascii_digit) {
        return None;
    }
    match b {
        // 沪市主板 60x（含 601/603/605）
        [b'6', b'0', ..] => Some(Board::Main),
        // 科创板 688/689
        [b'6', b'8', b'8' | b'9', ..] => Some(Board::Star),
        // 深市主板 000/001/002/003（002 中小板已并入主板）
        [b'0', b'0'..=b'3', ..] => Some(Board::Main),
        // 创业板 300/301/302
        [b'3', b'0', b'0'..=b'2', ..] => Some(Board::ChiNext),
        // 北交所 43/82/83/87/88/92
        [b'4', b'3', ..]
        | [b'8', b'2' | b'3' | b'7' | b'8', ..]
        | [b'9', b'2', ..] => Some(Board::Beijing),
        _ => None,
    }
}

/// ST 股名识别：名称含 "ST"（含 *ST）即按 ST 幅度处理。
/// 未股改 "S" 前缀股已基本消亡，不单独建模。
pub fn is_st_name(name: &str) -> bool {
    name.contains("ST")
}

/// 四舍五入到分。
fn round_fen(price: f64) -> f64 {
    (price * 100.0).round() / 100.0
}

/// 按分比较价格相等（涨跌停判定必须免疫浮点误差）。
fn same_fen(a: f64, b: f64) -> bool {
    (a * 100.0).round() == (b * 100.0).round()
}

/// 涨停价（正常交易日）。
///
/// `prev_close` 非正或非有限时返回 `None`（除权日昨收应传除权参考价，
/// 见 [`crate::tcp::stock::ex_right_reference`]）。
pub fn limit_up_price(prev_close: f64, board: Board, st: bool) -> Option<f64> {
    if !prev_close.is_finite() || prev_close <= 0.0 {
        return None;
    }
    Some(round_fen(prev_close * (1.0 + board.limit_pct_for(st) / 100.0)))
}

/// 跌停价（正常交易日）。非法输入同 [`limit_up_price`]。
pub fn limit_down_price(prev_close: f64, board: Board, st: bool) -> Option<f64> {
    if !prev_close.is_finite() || prev_close <= 0.0 {
        return None;
    }
    Some(round_fen(prev_close * (1.0 - board.limit_pct_for(st) / 100.0)))
}

/// 个股当日相对涨停价的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitStatus {
    /// 封板：收盘价 = 涨停价（含一字板）。
    Sealed,
    /// 炸板：盘中触板（最高价达涨停价）但收盘未封住。
    Blown,
    /// 未触板。
    Untouched,
}

impl LimitStatus {
    /// 落库/日志用小写串。
    pub fn as_str(self) -> &'static str {
        match self {
            LimitStatus::Sealed => "sealed",
            LimitStatus::Blown => "blown",
            LimitStatus::Untouched => "untouched",
        }
    }
}

/// 判定某日相对涨停价的状态。
///
/// - 收盘 = 涨停价 → [`LimitStatus::Sealed`]；
/// - 最高价达涨停价但收盘未封住 → [`LimitStatus::Blown`]；
/// - 否则 → [`LimitStatus::Untouched`]。
///
/// `prev_close` 非法时返回 `None`（调用方应跳过该日，如上市首日）。
pub fn limit_status(close: f64, high: f64, prev_close: f64, board: Board, st: bool) -> Option<LimitStatus> {
    let up = limit_up_price(prev_close, board, st)?;
    if same_fen(close, up) {
        Some(LimitStatus::Sealed)
    } else if same_fen(high, up) || high > up {
        Some(LimitStatus::Blown)
    } else {
        Some(LimitStatus::Untouched)
    }
}

/// 连板高度：`statuses` 按时间升序，从最近一日往前数连续
/// [`LimitStatus::Sealed`] 的天数。空序列或最近一日未封板 → 0。
pub fn limit_up_streak(statuses: &[LimitStatus]) -> u32 {
    statuses
        .iter()
        .rev()
        .take_while(|s| **s == LimitStatus::Sealed)
        .count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_of_by_prefix() {
        assert_eq!(board_of("600519"), Some(Board::Main)); // 沪主板
        assert_eq!(board_of("605111"), Some(Board::Main));
        assert_eq!(board_of("000001"), Some(Board::Main)); // 深主板
        assert_eq!(board_of("002594"), Some(Board::Main)); // 原中小板
        assert_eq!(board_of("300750"), Some(Board::ChiNext));
        assert_eq!(board_of("301269"), Some(Board::ChiNext));
        assert_eq!(board_of("688981"), Some(Board::Star));
        assert_eq!(board_of("689009"), Some(Board::Star));
        assert_eq!(board_of("832566"), Some(Board::Beijing));
        assert_eq!(board_of("430047"), Some(Board::Beijing));
        assert_eq!(board_of("920002"), Some(Board::Beijing));
        // 非股票（指数/基金/未知前缀/非法代码）
        assert_eq!(board_of("510300"), None); // ETF
        assert_eq!(board_of("999999"), None); // 指数
        assert_eq!(board_of("60005"), None); // 位数不足
        assert_eq!(board_of("000001."), None); // 含非数字
        assert_eq!(board_of(""), None);
    }

    #[test]
    fn limit_prices_main_10pct() {
        // 实测样本：000428 昨收 4.26 → 涨停 4.69（4.686 四舍五入到分）
        let up = limit_up_price(4.26, Board::Main, false).unwrap();
        assert_eq!(up, 4.69);
        assert_eq!(limit_down_price(4.26, Board::Main, false).unwrap(), 3.83);
    }

    #[test]
    fn limit_prices_by_board_and_st() {
        // 主板 ST ±5%：10.03 → 10.53（10.5315 → 10.53）
        assert_eq!(limit_up_price(10.03, Board::Main, true).unwrap(), 10.53);
        // 创业板/科创板 ±20%（ST 同幅度）
        assert_eq!(limit_up_price(10.00, Board::ChiNext, false).unwrap(), 12.0);
        assert_eq!(limit_up_price(10.00, Board::ChiNext, true).unwrap(), 12.0);
        assert_eq!(limit_up_price(50.13, Board::Star, false).unwrap(), 60.16);
        // 北交所 ±30%
        assert_eq!(limit_up_price(10.00, Board::Beijing, false).unwrap(), 13.0);
    }

    #[test]
    fn limit_prices_invalid_preclose() {
        assert_eq!(limit_up_price(0.0, Board::Main, false), None); // 上市首日
        assert_eq!(limit_up_price(-1.0, Board::Main, false), None);
        assert_eq!(limit_up_price(f64::NAN, Board::Main, false), None);
    }

    #[test]
    fn limit_status_classification() {
        let _up = limit_up_price(4.26, Board::Main, false).unwrap(); // 4.69
        // 收盘 = 涨停 → 封板（浮点免疫：4.69 vs 4.689999）
        assert_eq!(limit_status(4.69, 4.69, 4.26, Board::Main, false), Some(LimitStatus::Sealed));
        // 盘中触板（high = 涨停）但收盘未封住 → 炸板
        assert_eq!(limit_status(4.60, 4.69, 4.26, Board::Main, false), Some(LimitStatus::Blown));
        // 未触板
        assert_eq!(limit_status(4.60, 4.65, 4.26, Board::Main, false), Some(LimitStatus::Untouched));
        // 首日 prev_close = 0 → None
        assert_eq!(limit_status(4.69, 4.69, 0.0, Board::Main, false), None);
    }

    #[test]
    fn limit_up_streak_counts_recent_seals() {
        let s = [
            LimitStatus::Sealed,
            LimitStatus::Sealed,
            LimitStatus::Blown,
            LimitStatus::Sealed,
            LimitStatus::Sealed,
            LimitStatus::Sealed,
        ];
        assert_eq!(limit_up_streak(&s), 3); // 最近连续 3 板
        assert_eq!(limit_up_streak(&[]), 0);
        assert_eq!(limit_up_streak(&[LimitStatus::Blown]), 0);
    }

    #[test]
    fn is_st_name_detection() {
        assert!(is_st_name("ST易联众"));
        assert!(is_st_name("*ST左江"));
        assert!(is_st_name("STO panasonic")); // 含 ST 即判
        assert!(!is_st_name("平安银行"));
    }

    #[test]
    fn board_as_str_roundtrip_meaning() {
        assert_eq!(Board::Main.as_str(), "main");
        assert_eq!(Board::ChiNext.as_str(), "chinext");
        assert_eq!(Board::Star.as_str(), "star");
        assert_eq!(Board::Beijing.as_str(), "beijing");
    }
}
