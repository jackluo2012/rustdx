//! 技术指标计算库
//!
//! 提供常用的技术分析指标计算，包括：
//! - 移动平均线（SMA, EMA）
//! - MACD（指数平滑异同移动平均线）
//! - RSI（相对强弱指标）
//! - 布林带（Bollinger Bands）
//! - KDJ（随机指标）
//!
//! # 设计原则
//!
//! - 纯 Rust 实现，性能优异
//! - 类型安全，编译期检查
//! - 零外部依赖
//! - 与 K线数据无缝集成
//!
//! # 使用示例
//!
//! ```rust
//! use rustdx_complete::indicators::{sma, ema, macd, rsi};
//!
//! let closes = vec![10.0, 10.5, 11.0, 10.8, 11.2];
//!
//! // 计算简单移动平均
//! let sma_3 = sma(&closes, 3);
//!
//! // 计算指数移动平均
//! let ema_3 = ema(&closes, 3);
//!
//! // 计算 MACD
//! let macd_result = macd(&closes, 12, 26, 9);
//!
//! // 计算 RSI
//! let rsi_14 = rsi(&closes, 14);
//! ```

// ============================================================================
// 移动平均线
// ============================================================================

/// 计算简单移动平均线 (Simple Moving Average)
///
/// # 参数
///
/// - `data`: 价格数据序列
/// - `period`: 周期（如 5, 10, 20, 60）
///
/// # 返回
///
/// 返回一个向量，前 `period-1` 个元素为 `None`（数据不足），
/// 从第 `period` 个元素开始为计算出的 SMA 值
///
/// # 示例
///
/// ```rust
/// use rustdx_complete::indicators::sma;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let result = sma(&data, 3);
/// assert_eq!(result, vec![None, None, Some(2.0), Some(3.0), Some(4.0)]);
/// ```
///
/// # 数学公式
///
/// ```text
/// SMA = (P1 + P2 + ... + Pn) / n
///
/// 其中：
/// - P1, P2, ..., Pn 是最近 n 个周期的价格
/// - n 是周期数
/// ```
pub fn sma(data: &[f64], period: usize) -> Vec<Option<f64>> {
    if data.is_empty() {
        return vec![];
    }

    if period < 2 {
        return vec![None; data.len()];
    }

    let mut result = Vec::with_capacity(data.len());

    // 如果数据长度小于 period，所有位置都是 None
    if data.len() < period {
        return vec![None; data.len()];
    }

    // 前面的数据不足，返回 None
    for _ in 0..period - 1 {
        result.push(None);
    }

    // 计算移动平均
    for window in data.windows(period) {
        let sum: f64 = window.iter().sum();
        let avg = sum / period as f64;
        result.push(Some(avg));
    }

    result
}

/// 计算指数移动平均线 (Exponential Moving Average)
///
/// # 参数
///
/// - `data`: 价格数据序列
/// - `period`: 周期（如 12, 26）
///
/// # 返回
///
/// 返回一个向量，第一个元素为初始值，后续为计算出的 EMA 值
///
/// # 示例
///
/// ```rust
/// use rustdx_complete::indicators::ema;
///
/// let data = vec![22.27, 22.19, 22.08, 22.17, 22.18];
/// let result = ema(&data, 5);
/// ```
///
/// # 数学公式
///
/// ```text
/// EMA(today) = Price(today) × k + EMA(yesterday) × (1 − k)
///
/// 其中：
/// - k = 2 / (n + 1) 是平滑系数
/// - n 是周期数
/// - 第一个 EMA 值通常使用 SMA 作为初始值
/// ```
pub fn ema(data: &[f64], period: usize) -> Vec<f64> {
    if data.is_empty() {
        return vec![];
    }

    if period < 1 {
        return vec![0.0; data.len()];
    }

    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut emas = Vec::with_capacity(data.len());

    // 第一个 EMA 值使用第一个价格
    let mut prev_ema = data[0];
    emas.push(prev_ema);

    // 后续 EMA 值使用递推公式
    for &price in &data[1..] {
        let ema = (price - prev_ema) * multiplier + prev_ema;
        emas.push(ema);
        prev_ema = ema;
    }

    emas
}

// ============================================================================
// MACD
// ============================================================================

/// MACD 指标结果
#[derive(Debug, Clone)]
pub struct MacdResult {
    /// MACD 线（快线 - 慢线）
    pub macd: Vec<Option<f64>>,
    /// 信号线（MACD 的 EMA）
    pub signal: Vec<Option<f64>>,
    /// 柱状图（MACD - Signal）
    pub histogram: Vec<Option<f64>>,
}

/// 计算 MACD 指标 (Moving Average Convergence Divergence)
///
/// # 参数
///
/// - `data`: 价格数据序列（通常是收盘价）
/// - `fast_period`: 快线周期（默认 12）
/// - `slow_period`: 慢线周期（默认 26）
/// - `signal_period`: 信号线周期（默认 9）
///
/// # 返回
///
/// 返回包含 MACD 线、信号线和柱状图的结构体
///
/// # 示例
///
/// ```rust
/// use rustdx_complete::indicators::macd;
///
/// let closes = vec![10.0, 10.5, 11.0, 10.8, 11.2, 11.5, 11.3, 11.8];
/// let result = macd(&closes, 12, 26, 9);
///
/// println!("MACD: {:?}", result.macd);
/// println!("Signal: {:?}", result.signal);
/// println!("Histogram: {:?}", result.histogram);
/// ```
///
/// # 数学公式
///
/// ```text
/// MACD Line = EMA(fast) − EMA(slow)
/// Signal Line = EMA(MACD Line, signal_period)
/// Histogram = MACD Line − Signal Line
///
/// 其中：
/// - EMA(fast) 是快速 EMA（通常 12 日）
/// - EMA(slow) 是慢速 EMA（通常 26 日）
/// - Signal 是 MACD 线的 EMA（通常 9 日）
/// ```
///
/// # 交易信号
///
/// - **金叉**：MACD 线上穿信号线 → 买入信号
/// - **死叉**：MACD 线下穿信号线 → 卖出信号
/// - **柱状图 > 0**：多头市场
/// - **柱状图 < 0**：空头市场
pub fn macd(
    data: &[f64],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> MacdResult {
    // 计算 EMA
    let ema_fast = ema(data, fast_period);
    let ema_slow = ema(data, slow_period);

    // MACD 线 = 快线 - 慢线
    let macd_line: Vec<Option<f64>> = ema_fast
        .iter()
        .zip(ema_slow.iter())
        .map(|(f, s)| Some(f - s))
        .collect();

    // 信号线 = MACD 线的 EMA
    let macd_values: Vec<f64> = macd_line.iter().filter_map(|&v| v).collect();
    let signal_ema = ema(&macd_values, signal_period);

    // 构造信号线（前面数据不足的部分为 None）
    let mut signal_line = vec![None; macd_line.len().saturating_sub(signal_ema.len())];
    signal_line.extend(signal_ema.iter().map(|&v| Some(v)));

    // 柱状图 = MACD - Signal
    let histogram: Vec<Option<f64>> = macd_line
        .iter()
        .zip(signal_line.iter())
        .map(|(m, s)| match (m, s) {
            (Some(macd_val), Some(signal_val)) => Some(macd_val - signal_val),
            _ => None,
        })
        .collect();

    MacdResult {
        macd: macd_line,
        signal: signal_line,
        histogram,
    }
}

// ============================================================================
// RSI
// ============================================================================

/// 计算相对强弱指标 (Relative Strength Index)
///
/// # 参数
///
/// - `data`: 价格数据序列（通常是收盘价）
/// - `period`: 周期（默认 14）
///
/// # 返回
///
/// 返回 RSI 值向量，范围 0-100，前面不足的数据为 None
///
/// # 示例
///
/// ```rust
/// use rustdx_complete::indicators::rsi;
///
/// let closes = vec![10.0, 10.5, 11.0, 10.8, 11.2, 11.5, 11.3, 11.8, 12.0, 11.9];
/// let result = rsi(&closes, 14);
/// ```
///
/// # 数学公式
///
/// ```text
/// RSI = 100 − (100 / (1 + RS))
///
/// 其中：
/// - RS = 平均涨幅 / 平均跌幅
/// - 平均涨幅 = 最近 N 周期内涨幅之和 / N
/// - 平均跌幅 = 最近 N 周期内跌幅之和 / N
/// ```
///
/// # 交易信号
///
/// - **RSI > 70**：超买，可能回调 → 考虑卖出
/// - **RSI < 30**：超卖，可能反弹 → 考虑买入
/// - **RSI = 50**：中性区域
/// - **背离**：价格创新高但 RSI 未创新高 → 顶背离（看跌）
pub fn rsi(data: &[f64], period: usize) -> Vec<Option<f64>> {
    if data.len() < period + 1 {
        return vec![None; data.len()];
    }

    let mut result = vec![None; period]; // 前面数据不足

    for window in data.windows(period + 1) {
        let mut gains = 0.0;
        let mut losses = 0.0;

        // 计算累计涨幅和跌幅
        for i in 1..=period {
            let change = window[i] - window[i - 1];
            if change > 0.0 {
                gains += change;
            } else {
                losses += change.abs();
            }
        }

        // 计算平均涨幅和跌幅
        let avg_gain = gains / period as f64;
        let avg_loss = losses / period as f64;

        // 计算 RSI
        let rsi_value = if avg_loss == 0.0 {
            100.0 // 没有跌幅，RSI = 100
        } else {
            let rs = avg_gain / avg_loss;
            100.0 - (100.0 / (1.0 + rs))
        };

        result.push(Some(rsi_value));
    }

    result
}

// ============================================================================
// 布林带
// ============================================================================

/// 布林带指标结果
#[derive(Debug, Clone)]
pub struct BollingerBandsResult {
    /// 上轨
    pub upper: Vec<Option<f64>>,
    /// 中轨（SMA）
    pub middle: Vec<Option<f64>>,
    /// 下轨
    pub lower: Vec<Option<f64>>,
}

/// 计算布林带 (Bollinger Bands)
///
/// # 参数
///
/// - `data`: 价格数据序列（通常是收盘价）
/// - `period`: 周期（默认 20）
/// - `std_dev`: 标准差倍数（默认 2.0）
///
/// # 返回
///
/// 返回包含上轨、中轨、下轨的结构体
///
/// # 示例
///
/// ```rust
/// use rustdx_complete::indicators::bollinger_bands;
///
/// let closes = vec![10.0, 10.5, 11.0, 10.8, 11.2];
/// let result = bollinger_bands(&closes, 20, 2.0);
///
/// println!("上轨: {:?}", result.upper);
/// println!("中轨: {:?}", result.middle);
/// println!("下轨: {:?}", result.lower);
/// ```
///
/// # 数学公式
///
/// ```text
/// 中轨 = SMA(period)
/// 上轨 = 中轨 + (std_dev × 标准差)
/// 下轨 = 中轨 − (std_dev × 标准差)
///
/// 其中：
/// - std_dev 通常是 2（即 2 倍标准差）
/// ```
///
/// # 交易信号
///
/// - **价格触及上轨**：可能超买 → 考虑卖出
/// - **价格触及下轨**：可能超卖 → 考虑买入
/// - **布林带收口**：波动率降低，即将突破
/// - **布林带开口**：波动率增加，趋势开始
pub fn bollinger_bands(
    data: &[f64],
    period: usize,
    std_dev_multiplier: f64,
) -> BollingerBandsResult {
    if data.is_empty() {
        return BollingerBandsResult {
            upper: vec![],
            middle: vec![],
            lower: vec![],
        };
    }

    // 计算中轨（SMA）
    let sma_values = sma(data, period);

    // 计算上下轨
    let mut upper_band = Vec::new();
    let mut lower_band = Vec::new();

    for (i, _) in data.iter().enumerate() {
        if period < 2 || i + 1 < period {
            // 数据不足或周期无效
            upper_band.push(None);
            lower_band.push(None);
        } else {
            // 计算窗口内的标准差（使用 windows 方法更安全）
            let start = i + 1 - period;
            let window = &data[start..=i];
            let mean: f64 = window.iter().sum::<f64>() / period as f64;
            let variance = window.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / period as f64;
            let std = variance.sqrt();

            // 上轨 = 中轨 + k × 标准差
            // 下轨 = 中轨 - k × 标准差
            if i < sma_values.len() {
                if let Some(middle_val) = sma_values[i] {
                    upper_band.push(Some(middle_val + std_dev_multiplier * std));
                    lower_band.push(Some(middle_val - std_dev_multiplier * std));
                } else {
                    upper_band.push(None);
                    lower_band.push(None);
                }
            } else {
                upper_band.push(None);
                lower_band.push(None);
            }
        }
    }

    BollingerBandsResult {
        upper: upper_band,
        middle: sma_values,
        lower: lower_band,
    }
}

// ============================================================================
// KDJ 随机指标
// ============================================================================

/// KDJ 指标结果
#[derive(Debug, Clone)]
pub struct KdjResult {
    /// K 值（快速确认线）
    pub k: Vec<Option<f64>>,
    /// D 值（慢速确认线）
    pub d: Vec<Option<f64>>,
    /// J 值（提前反应线）
    pub j: Vec<Option<f64>>,
}

/// 计算随机指标 (KDJ - Stochastic Oscillator)
///
/// # 参数
///
/// - `high`: 最高价序列
/// - `low`: 最低价序列
/// - `close`: 收盘价序列
/// - `k_period`: K 线周期（默认 9）
/// - `d_period`: D 线周期（默认 3）
/// - `j_period`: J 线倍数（默认 3）
///
/// # 返回
///
/// 返回包含 K、D、J 三条线的结构体
///
/// # 示例
///
/// ```rust
/// use rustdx_complete::indicators::kdj;
///
/// let highs = vec![11.0, 11.2, 11.5, 11.3, 11.8];
/// let lows = vec![10.0, 10.5, 10.8, 10.6, 11.0];
/// let closes = vec![10.5, 11.0, 11.2, 11.1, 11.5];
///
/// let result = kdj(&highs, &lows, &closes, 9, 3, 3);
/// ```
///
/// # 数学公式
///
/// ```text
/// RSV = (收盘价 − 最低价) / (最高价 − 最低价) × 100
/// K = SMA(RSV, k_period)  // 通常为 3 日 SMA
/// D = SMA(K, d_period)    // 通常为 3 日 SMA
/// J = 3 × K − 2 × D
/// ```
///
/// # 交易信号
///
/// - **K > D**：上升趋势 → 持有或买入
/// - **K < D**：下降趋势 → 考虑卖出
/// - **K < 20 且 D < 20**：超卖区 → 买入信号
/// - **K > 80 且 D > 80**：超买区 → 卖出信号
/// - **J > 100**：严重超买 → 强烈卖出信号
/// - **J < 0**：严重超卖 → 强烈买入信号
pub fn kdj(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    k_period: usize,
    d_period: usize,
    j_multiplier: usize,
) -> KdjResult {
    let len = close.len();

    if len == 0 {
        return KdjResult {
            k: vec![],
            d: vec![],
            j: vec![],
        };
    }

    if len != high.len() || len != low.len() {
        return KdjResult {
            k: vec![None; len],
            d: vec![None; len],
            j: vec![None; len],
        };
    }

    // 计算 RSV (Raw Stochastic Value)
    let mut rsv_values = Vec::new();
    for i in 0..len {
        if k_period < 2 || i + 1 < k_period {
            rsv_values.push(None);
        } else {
            let start = i + 1 - k_period;
            let window_high = &high[start..=i];
            let window_low = &low[start..=i];

            let highest_high = window_high.iter().fold(f64::NAN, |a, &b| a.max(b));
            let lowest_low = window_low.iter().fold(f64::NAN, |a, &b| a.min(b));

            if highest_high - lowest_low > 0.0 {
                let rsv = (close[i] - lowest_low) / (highest_high - lowest_low) * 100.0;
                rsv_values.push(Some(rsv));
            } else {
                rsv_values.push(Some(50.0)); // 最高价 = 最低价时，RSV = 50
            }
        }
    }

    // 计算 K 值（RSV 的移动平均）
    let k_values: Vec<f64> = rsv_values
        .iter()
        .filter_map(|&v| v)
        .collect();

    let k_ema = if k_values.is_empty() {
        vec![0.0; len]
    } else {
        ema(&k_values, d_period)
    };

    let mut k_line = vec![None; rsv_values.len().saturating_sub(k_ema.len())];
    k_line.extend(k_ema.iter().map(|&v| Some(v)));

    // 计算 D 值（K 的移动平均）
    let d_values: Vec<f64> = k_line.iter().filter_map(|&v| v).collect();
    let d_ema = if d_values.is_empty() {
        vec![0.0]
    } else {
        ema(&d_values, d_period)
    };

    let mut d_line = vec![None; k_line.len().saturating_sub(d_ema.len())];
    d_line.extend(d_ema.iter().map(|&v| Some(v)));

    // 计算 J 值
    let mut j_line = Vec::new();
    for (k_val, d_val) in k_line.iter().zip(d_line.iter()) {
        match (k_val, d_val) {
            (Some(k), Some(d)) => {
                let j = j_multiplier as f64 * k - (j_multiplier as f64 - 1.0) * d;
                j_line.push(Some(j));
            }
            _ => {
                j_line.push(None);
            }
        }
    }

    KdjResult {
        k: k_line,
        d: d_line,
        j: j_line,
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sma() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&data, 3);

        assert_eq!(result.len(), 5);
        assert_eq!(result[0], None);
        assert_eq!(result[1], None);
        assert_eq!(result[2], Some(2.0));
        assert_eq!(result[3], Some(3.0));
        assert_eq!(result[4], Some(4.0));
    }

    #[test]
    fn test_ema() {
        let data = vec![22.27, 22.19, 22.08, 22.17, 22.18];
        let result = ema(&data, 5);

        assert_eq!(result.len(), 5);
        assert!((result[0] - 22.27).abs() < 0.01); // 第一个值是第一个价格
        // 后续值应该是递归计算的 EMA
        assert!(result[1..].iter().all(|&v| v > 0.0));
    }

    #[test]
    fn test_macd() {
        let data = vec![
            10.0, 10.5, 11.0, 10.8, 11.2, 11.5, 11.3, 11.8,
            12.0, 11.9, 12.2, 12.5, 12.3, 12.8, 13.0,
        ];

        let result = macd(&data, 12, 26, 9);

        // 检查返回值结构
        assert_eq!(result.macd.len(), data.len());
        assert_eq!(result.signal.len(), data.len());
        assert_eq!(result.histogram.len(), data.len());

        // 检查是否有值
        let has_values = result.macd.iter().any(|&v| v.is_some());
        assert!(has_values);
    }

    #[test]
    fn test_rsi() {
        let data = vec![
            10.0, 10.5, 11.0, 10.8, 11.2, 11.5, 11.3, 11.8,
            12.0, 11.9, 12.2, 12.5, 12.3, 12.8, 13.0,
        ];

        let period = 14;
        let result = rsi(&data, period);

        assert_eq!(result.len(), data.len());

        // 前面应该有 None
        assert_eq!(result[..period].iter().filter(|&&v| v.is_none()).count(), period);

        // 后面应该有值
        let has_values = result[period..].iter().any(|&v| v.is_some());
        assert!(has_values);

        // RSI 应该在 0-100 范围内
        for value in result.iter().filter_map(|&v| v) {
            assert!((0.0..=100.0).contains(&value));
        }
    }

    #[test]
    fn test_rsi_extreme_cases() {
        // 所有价格上涨
        let rising: Vec<f64> = (0..20).map(|i| 10.0 + i as f64).collect();
        let result = rsi(&rising, 14);
        if let Some(&Some(value)) = result.last() {
            assert_eq!(value, 100.0); // 应该接近 100
        }

        // 所有价格下跌
        let falling: Vec<f64> = (0..20).map(|i| 30.0 - i as f64).collect();
        let result = rsi(&falling, 14);
        if let Some(&Some(value)) = result.last() {
            assert!(value < 50.0); // 应该小于 50
        }
    }

    #[test]
    fn test_bollinger_bands() {
        let data = vec![
            10.0, 10.5, 11.0, 10.8, 11.2, 11.5, 11.3, 11.8,
            12.0, 11.9, 12.2, 12.5, 12.3, 12.8, 13.0,
        ];

        let result = bollinger_bands(&data, 5, 2.0);

        // 检查结构
        assert_eq!(result.upper.len(), data.len());
        assert_eq!(result.middle.len(), data.len());
        assert_eq!(result.lower.len(), data.len());

        // 检查是否有值
        let has_values = result.upper.iter().any(|&v| v.is_some());
        assert!(has_values);

        // 检查上轨 >= 中轨 >= 下轨
        for i in 0..data.len() {
            if let (Some(u), Some(m), Some(l)) = (result.upper[i], result.middle[i], result.lower[i]) {
                assert!(u >= m);
                assert!(m >= l);
            }
        }
    }

    #[test]
    fn test_kdj() {
        let highs: Vec<f64> = (0..20).map(|i| 10.0 + i as f64 * 0.1 + 0.5).collect();
        let lows: Vec<f64> = (0..20).map(|i| 10.0 + i as f64 * 0.1 - 0.5).collect();
        let closes: Vec<f64> = (0..20).map(|i| 10.0 + i as f64 * 0.1).collect();

        let result = kdj(&highs, &lows, &closes, 9, 3, 3);

        // 检查结构
        assert_eq!(result.k.len(), closes.len());
        assert_eq!(result.d.len(), closes.len());
        assert_eq!(result.j.len(), closes.len());

        // 检查是否有值
        let has_values = result.k.iter().any(|&v| v.is_some());
        assert!(has_values);

        // KDJ 应该在 0-100 范围内（J 可能超出）
        for value in result.k.iter().filter_map(|&v| v) {
            assert!((0.0..=100.0).contains(&value));
        }
        for value in result.d.iter().filter_map(|&v| v) {
            assert!((0.0..=100.0).contains(&value));
        }
    }

    #[test]
    fn test_empty_data() {
        let data: Vec<f64> = vec![];

        assert_eq!(sma(&data, 5), vec![None; 0]);
        assert_eq!(ema(&data, 5), vec![]);
        assert_eq!(rsi(&data, 14), vec![None; 0]);
    }

    #[test]
    fn test_single_value() {
        let data = vec![10.0];

        let sma_result = sma(&data, 5);
        // SMA 需要至少 period 个数据才能开始计算
        // 对于单个值和 period=5，返回 None
        assert_eq!(sma_result.len(), 1);
        assert_eq!(sma_result[0], None);

        let ema_result = ema(&data, 5);
        // EMA 对于单个值直接返回该值
        assert_eq!(ema_result.len(), 1);
        assert_eq!(ema_result[0], 10.0);
    }
}
