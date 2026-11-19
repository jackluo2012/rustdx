//! A 股涨跌停规则：板块判定、涨跌停价、封板/炸板判型、连板高度。
//!
//! 纯函数、无 IO：调用方负责给出正确输入——原始不复权价格、正确的昨收盘
//! （除权日用 [`ex_right_reference`](crate::tcp::stock::ex_right_reference)）。
//!
//! 涨跌停价规则：`前收盘 × (1 ± 幅度)`，**四舍五入到分**（与交易所/行情软件
//! 一致）。新股上市首日等特殊涨跌幅不在本模块，调用方应跳过首日
//! （如日线 `prev_close == 0` 的首行）。
//!
//! **主板 ST 的幅度随日期变化**：[`MAIN_ST_10PCT_SINCE`](crate::limit::MAIN_ST_10PCT_SINCE)
//! （2026-07-06）起由 ±5% 调整为
//! ±10%。不带日期的函数按现行规则计算，用于盘中；回算历史（如连板高度）时
//! 用带 `_on` 后缀的版本并传入当日日期。
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

/// 沪深主板 ST（风险警示）股票涨跌幅由 ±5% 调整为 ±10% 的生效日（YYYYMMDD）。
///
/// 2026-11-19 实盘核对：141 只主板 ST 中 17 只的当日最高/最低价超出 ±5%，
/// 最大涨跌幅为 +10.09% / -10.00%；历史日线上主板 ST 收于 ±5% 档的次数自
/// 2026 年 7 月起骤降。
pub const MAIN_ST_10PCT_SINCE: u32 = 20260706;

impl Board {
    /// 现行规则下、含 ST 修正的涨跌停幅度（%）。
    ///
    /// 主板 ±10%（ST 自 [`MAIN_ST_10PCT_SINCE`] 起同为 ±10%）；创业板/科创板
    /// ±20%（ST 同幅度）；北交所 ±30%。新股上市首日等特殊规则不在本模块。
    /// 回算历史请用 [`Board::limit_pct_on`]。
    pub fn limit_pct_for(self, st: bool) -> f64 {
        self.limit_pct_on(st, u32::MAX)
    }

    /// 新股上市后**不设涨跌幅限制**的交易日数（含上市首日）。
    ///
    /// 现行规则：主板（2023-04-10 全面注册制起）、创业板、科创板为前 5 个交易日；
    /// 北交所仅上市首日。这几天没有涨跌停价，回算连板或统计涨停时应跳过
    /// 日线的前这么多根，本模块的其它函数不知道上市日期，不会自动跳过。
    ///
    /// 注意：主板在 2023-04-10 之前上市的新股适用旧规则（首日 44%、次日起
    /// ±10%），不在此列。本值来自交易所规则；2026-11-19 核对时市场上没有上市
    /// 5 日内的新股，未能用实盘数据验证。
    pub fn ipo_free_days(self) -> u32 {
        match self {
            Board::Main | Board::ChiNext | Board::Star => 5,
            Board::Beijing => 1,
        }
    }

    /// `date`（YYYYMMDD）当日适用的涨跌停幅度（%）。
    ///
    /// 与 [`Board::limit_pct_for`] 的唯一区别：主板 ST 在
    /// [`MAIN_ST_10PCT_SINCE`] 之前是 ±5%。
    pub fn limit_pct_on(self, st: bool, date: u32) -> f64 {
        match self {
            Board::Main if st && date < MAIN_ST_10PCT_SINCE => 5.0,
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
        [b'4', b'3', ..] | [b'8', b'2' | b'3' | b'7' | b'8', ..] | [b'9', b'2', ..] => {
            Some(Board::Beijing)
        }
        _ => None,
    }
}

/// ST 股名识别：名称含 "ST"（含 *ST）即按 ST 幅度处理。
/// 未股改 "S" 前缀股已基本消亡，不单独建模。
///
/// 证券列表里的名称是**当前**名称（2026-11-19 核对：11 月 6 日、11 日摘帽的
/// 两只已是新名称，更新及时）。回算历史时拿不到当时的名称：一只现在已摘帽的
/// 主板股票，在 [`MAIN_ST_10PCT_SINCE`] 之前戴帽期间的 ±5% 涨停会被按 ±10%
/// 漏判，反之亦然。需要精确历史的调用方得自己保存名称变更记录。
pub fn is_st_name(name: &str) -> bool {
    name.contains("ST")
}

/// 按分比较价格相等（涨跌停判定必须免疫浮点误差）。
fn same_fen(a: f64, b: f64) -> bool {
    (a * 100.0).round() == (b * 100.0).round()
}

/// 现行规则下的涨停价（正常交易日）。回算历史请用 [`limit_up_price_on`]。
///
/// `prev_close` 非正或非有限时返回 `None`（除权日昨收应传除权参考价，
/// 见 [`crate::tcp::stock::ex_right_reference`]）。
pub fn limit_up_price(prev_close: f64, board: Board, st: bool) -> Option<f64> {
    limit_up_price_on(prev_close, board, st, u32::MAX)
}

/// 现行规则下的跌停价（正常交易日）。非法输入同 [`limit_up_price`]。
pub fn limit_down_price(prev_close: f64, board: Board, st: bool) -> Option<f64> {
    limit_down_price_on(prev_close, board, st, u32::MAX)
}

/// `date`（YYYYMMDD）当日的涨停价。非法输入同 [`limit_up_price`]。
pub fn limit_up_price_on(prev_close: f64, board: Board, st: bool, date: u32) -> Option<f64> {
    limit_price(prev_close, board.limit_pct_on(st, date))
}

/// `date`（YYYYMMDD）当日的跌停价。非法输入同 [`limit_up_price`]。
pub fn limit_down_price_on(prev_close: f64, board: Board, st: bool, date: u32) -> Option<f64> {
    limit_price(prev_close, -board.limit_pct_on(st, date))
}

/// `前收盘 × (1 + 幅度%)` 按交易所口径（十进制四舍五入到分）计算；前收盘非法时
/// 返回 `None`。
///
/// **全程用整数分运算**：`(昨收分 × (100 ± 幅度) + 50) / 100`。直接在浮点上乘再
/// 舍入会出错——`1.15 × 1.1` 在 f64 里是 `1.26499…`，舍入得 1.26，而交易所的
/// 涨停价是 1.27。穷举 0.01~999.99 元 × 10%/20%/30% 两个方向共约 60 万组，
/// 浮点做法有 5133 组差 1 分，会把真实的封板误判成炸板。
fn limit_price(prev_close: f64, pct: f64) -> Option<f64> {
    if !prev_close.is_finite() || prev_close <= 0.0 {
        return None;
    }
    // 昨收本身精确到分，取整只是消除 f64 的表示误差
    let prev_fen = (prev_close * 100.0).round() as i64;
    let pct = pct.round() as i64; // 幅度都是整数百分比（5/10/20/30）
    (prev_fen > 0).then(|| ((prev_fen * (100 + pct) + 50) / 100) as f64 / 100.0)
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
/// 按现行规则判定，用于盘中；回算历史请用 [`limit_status_on`]。
pub fn limit_status(
    close: f64,
    high: f64,
    prev_close: f64,
    board: Board,
    st: bool,
) -> Option<LimitStatus> {
    limit_status_on(close, high, prev_close, board, st, u32::MAX)
}

/// 按 `date`（YYYYMMDD）当日适用的涨跌幅判定状态，规则同 [`limit_status`]。
///
/// 回算连板高度时逐日调用本函数：主板 ST 在 [`MAIN_ST_10PCT_SINCE`] 前后的
/// 涨停价不同，用现行规则回算旧日期会把当时的涨停板全部漏掉。
pub fn limit_status_on(
    close: f64,
    high: f64,
    prev_close: f64,
    board: Board,
    st: bool,
    date: u32,
) -> Option<LimitStatus> {
    let up = limit_up_price_on(prev_close, board, st, date)?;
    if same_fen(close, up) {
        Some(LimitStatus::Sealed)
    } else if same_fen(high, up) || high > up {
        Some(LimitStatus::Blown)
    } else {
        Some(LimitStatus::Untouched)
    }
}

/// 盘中判定相对涨停价的状态：在 [`limit_status_on`] 的基础上要求**卖一没有挂单**
/// 才算封板。
///
/// 盘中「现价 = 涨停价」不等于封住：卖一可能还挂在涨停价上、正在被吃
/// （2026-11-19 盘中实测，32 只现价等于涨停价的股票里有 1 只卖一还有 44 手）。
/// **只在连续交易时段（09:30~14:57）使用**；收盘后请用 [`limit_status_on`]。
/// 15:05 起是盘后定价交易，行情的买一 / 卖一变成按收盘价挂出的盘后委托
/// （2026-11-19 实测：贵州茅台 15:05 卖一是收盘价 1460.21 × 9 手、买一为空），
/// 封死涨停的股票这时也可能出现卖一量，会被本函数误判成炸板。
///
/// - `price` / `high` / `prev_close` / `ask1_vol` 对应行情快照的现价、最高、昨收、
///   卖一量；
/// - 现价 = 涨停价且卖一量为 0 → [`LimitStatus::Sealed`]；
/// - 触及过涨停价但当前没封住（含现价在涨停价、卖一仍有挂单）→
///   [`LimitStatus::Blown`]；
/// - 否则 → [`LimitStatus::Untouched`]。
pub fn limit_status_live(
    price: f64,
    high: f64,
    prev_close: f64,
    ask1_vol: f64,
    board: Board,
    st: bool,
    date: u32,
) -> Option<LimitStatus> {
    let status = limit_status_on(price, high, prev_close, board, st, date)?;
    Some(if status == LimitStatus::Sealed && ask1_vol > 0.0 {
        LimitStatus::Blown
    } else {
        status
    })
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
        // 主板 ST 现行 ±10%，与普通主板股票相同
        assert_eq!(limit_up_price(10.03, Board::Main, true).unwrap(), 11.03);
        assert_eq!(limit_down_price(10.03, Board::Main, true).unwrap(), 9.03);
        // 创业板/科创板 ±20%（ST 同幅度）
        assert_eq!(limit_up_price(10.00, Board::ChiNext, false).unwrap(), 12.0);
        assert_eq!(limit_up_price(10.00, Board::ChiNext, true).unwrap(), 12.0);
        assert_eq!(limit_up_price(50.13, Board::Star, false).unwrap(), 60.16);
        // 北交所 ±30%
        assert_eq!(limit_up_price(10.00, Board::Beijing, false).unwrap(), 13.0);
    }

    /// 主板 ST 的幅度在 2026-07-06 由 ±5% 调整为 ±10%；其它板块不随日期变化。
    #[test]
    fn main_st_limit_depends_on_date() {
        let before = MAIN_ST_10PCT_SINCE - 3; // 2026-07-03，调整前最后一个交易日
        // 调整前：10.03 → 10.53（10.5315）/ 9.53（9.5285）
        assert_eq!(
            limit_up_price_on(10.03, Board::Main, true, before),
            Some(10.53)
        );
        assert_eq!(
            limit_down_price_on(10.03, Board::Main, true, before),
            Some(9.53)
        );
        // 生效日起
        assert_eq!(
            limit_up_price_on(10.03, Board::Main, true, MAIN_ST_10PCT_SINCE),
            Some(11.03)
        );
        assert_eq!(
            limit_up_price_on(10.03, Board::Main, true, 20261119),
            Some(11.03)
        );
        // 非 ST 与其它板块不受日期影响
        for date in [before, MAIN_ST_10PCT_SINCE] {
            assert_eq!(Board::Main.limit_pct_on(false, date), 10.0);
            assert_eq!(Board::ChiNext.limit_pct_on(true, date), 20.0);
            assert_eq!(Board::Star.limit_pct_on(true, date), 20.0);
            assert_eq!(Board::Beijing.limit_pct_on(true, date), 30.0);
        }
        // 2026-11-19 实盘样本：*ST雅博 昨收 1.09，现价 1.14（+5% 档）、最高 1.18。
        // 现行规则涨停价是 1.20，这不是封板；按旧的 5% 才会被误判成封板。
        assert_eq!(limit_up_price(1.09, Board::Main, true), Some(1.2));
        assert_eq!(
            limit_status(1.14, 1.18, 1.09, Board::Main, true),
            Some(LimitStatus::Untouched)
        );
        assert_eq!(
            limit_status_on(1.14, 1.14, 1.09, Board::Main, true, before),
            Some(LimitStatus::Sealed)
        );
    }

    /// 交易所口径是十进制四舍五入到分；浮点乘法后再舍入会差 1 分。
    #[test]
    fn limit_prices_use_exact_decimal_rounding() {
        // f64 路径算错的实例：1.15×1.1=1.265 → 1.27（f64 得 1.26）
        assert_eq!(limit_up_price(1.15, Board::Main, false), Some(1.27));
        assert_eq!(limit_down_price(1.15, Board::Main, false), Some(1.04));
        assert_eq!(limit_down_price(2.65, Board::Main, false), Some(2.39));
        assert_eq!(limit_down_price(9.35, Board::Main, false), Some(8.42));
        // 调整前的主板 ST 5%：4.30 → 4.52（f64 得 4.51）
        assert_eq!(
            limit_up_price_on(4.30, Board::Main, true, MAIN_ST_10PCT_SINCE - 1),
            Some(4.52)
        );
        // 穷举 0.01~999.99 元：每个结果都等于整数分的十进制四舍五入
        for (board, pct) in [
            (Board::Main, 10i64),
            (Board::ChiNext, 20),
            (Board::Beijing, 30),
        ] {
            for fen in 1..=99_999i64 {
                let prev = fen as f64 / 100.0;
                let up = limit_up_price(prev, board, false).unwrap();
                let down = limit_down_price(prev, board, false).unwrap();
                assert_eq!(
                    (up * 100.0).round() as i64,
                    (fen * (100 + pct) + 50) / 100,
                    "{prev} 涨停"
                );
                assert_eq!(
                    (down * 100.0).round() as i64,
                    (fen * (100 - pct) + 50) / 100,
                    "{prev} 跌停"
                );
            }
        }
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
        assert_eq!(
            limit_status(4.69, 4.69, 4.26, Board::Main, false),
            Some(LimitStatus::Sealed)
        );
        // 盘中触板（high = 涨停）但收盘未封住 → 炸板
        assert_eq!(
            limit_status(4.60, 4.69, 4.26, Board::Main, false),
            Some(LimitStatus::Blown)
        );
        // 未触板
        assert_eq!(
            limit_status(4.60, 4.65, 4.26, Board::Main, false),
            Some(LimitStatus::Untouched)
        );
        // 首日 prev_close = 0 → None
        assert_eq!(limit_status(4.69, 4.69, 0.0, Board::Main, false), None);
    }

    /// 盘中现价在涨停价但卖一还有挂单，不算封板。
    #[test]
    fn live_status_requires_empty_ask() {
        let live = |price, high, ask1_vol| {
            limit_status_live(price, high, 28.37, ask1_vol, Board::Main, false, 20261119)
        };
        // 昨收 28.37 → 涨停 31.21（2026-11-19 九阳股份）
        assert_eq!(live(31.21, 31.21, 0.0), Some(LimitStatus::Sealed));
        assert_eq!(live(31.21, 31.21, 44.0), Some(LimitStatus::Blown));
        assert_eq!(live(30.90, 31.21, 120.0), Some(LimitStatus::Blown));
        assert_eq!(live(30.90, 31.00, 120.0), Some(LimitStatus::Untouched));
        assert_eq!(
            limit_status_live(1.0, 1.0, 0.0, 0.0, Board::Main, false, 20261119),
            None
        );
    }

    #[test]
    fn ipo_free_days_by_board() {
        assert_eq!(Board::Main.ipo_free_days(), 5);
        assert_eq!(Board::ChiNext.ipo_free_days(), 5);
        assert_eq!(Board::Star.ipo_free_days(), 5);
        assert_eq!(Board::Beijing.ipo_free_days(), 1);
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
