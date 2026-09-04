//! 技术指标计算示例
//!
//! 展示如何使用 rustdx 的技术指标计算功能
//!
//! 运行方式：
//! ```bash
//! cargo run --example test_technical_indicators
//! ```

use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::Kline;
use rustdx_complete::indicators::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📈 rustdx 技术指标计算示例\n");

    // 连接到通达信服务器
    println!("⚡ 连接到通达信服务器...");
    let mut tcp = Tcp::new()?;
    println!("✅ 连接成功\n");

    // 获取K线数据
    println!("📊 获取浦发银行(600000)的K线数据...");
    let mut kline = Kline::new(1, "600000", 9, 0, 100); // 获取最近100天的日线数据
    kline.recv_parsed(&mut tcp)?;

    let data = kline.result();
    println!("✅ 获取到 {} 条数据\n", data.len());

    // 提取收盘价
    let closes: Vec<f64> = data.iter().map(|k| k.close).collect();
    let highs: Vec<f64> = data.iter().map(|k| k.high).collect();
    let lows: Vec<f64> = data.iter().map(|k| k.low).collect();

    // ========================================
    // 示例 1: 移动平均线（SMA & EMA）
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 1: 移动平均线");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let sma_20 = sma(&closes, 20);
    let ema_12 = ema(&closes, 12);

    println!("计算 SMA(20) 和 EMA(12)：\n");

    // 显示最近5天的数据
    for ((bar, sma_val), ema_val) in data.iter()
        .zip(sma_20.iter())
        .zip(ema_12.iter())
        .rev()
        .take(5)
        
    {
        println!("  日期: {:04}-{:02}-{:02}", bar.dt.year, bar.dt.month, bar.dt.day);
        println!("    收盘价: {:.2}", bar.close);

        match sma_val {
            Some(val) => println!("    SMA(20): {:.2}", val),
            None => println!("    SMA(20): N/A (数据不足)"),
        }

        match ema_val {
            &val => println!("    EMA(12): {:.2}", val),
        }

        if let (Some(sma), &ema) = (sma_val, ema_val) {
            if ema > *sma {
                println!("    信号: 📈 上升趋势 (EMA > SMA)");
            } else if ema < *sma {
                println!("    信号: 📉 下降趋势 (EMA < SMA)");
            } else {
                println!("    信号: ➡️ 中性 (EMA = SMA)");
            }
        }

        println!();
    }

    // ========================================
    // 示例 2: MACD 指标
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 2: MACD 指标");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let macd_result = macd(&closes, 12, 26, 9);

    println!("计算 MACD(12, 26, 9)：\n");

    // 显示最近5天的 MACD 数据
    let mut macd_signals = Vec::new();
    for (((bar, macd_val), signal_val), hist_val) in data.iter()
        .zip(macd_result.macd.iter())
        .zip(macd_result.signal.iter())
        .zip(macd_result.histogram.iter())
        .rev()
        .take(5)
        
    {
        println!("  日期: {:04}-{:02}-{:02}", bar.dt.year, bar.dt.month, bar.dt.day);

        match (macd_val, signal_val) {
            (Some(m), Some(s)) => {
                println!("    MACD: {:.4}", m);
                println!("    Signal: {:.4}", s);
                println!("    Histogram: {:.4}", hist_val.unwrap_or(0.0));

                if m > s {
                    println!("    信号: 📈 金叉（MACD > Signal）→ 看涨");
                    macd_signals.push(" bullish");
                } else if m < s {
                    println!("    信号: 📉 死叉（MACD < Signal）→ 看跌");
                    macd_signals.push("bearish");
                } else {
                    println!("    信号: ➡️ 中性");
                    macd_signals.push("neutral");
                }
            }
            _ => {
                println!("    MACD: 计算中...");
            }
        }

        println!();
    }

    // ========================================
    // 示例 3: RSI 指标
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 3: RSI 相对强弱指标");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let rsi_14 = rsi(&closes, 14);

    println!("计算 RSI(14)：\n");

    // 显示最近5天的 RSI 数据
    for (bar, rsi_val) in data.iter()
        .zip(rsi_14.iter())
        .rev()
        .take(5)
        
    {
        println!("  日期: {:04}-{:02}-{:02}", bar.dt.year, bar.dt.month, bar.dt.day);

        match rsi_val {
            Some(val) => {
                println!("    RSI(14): {:.2}", val);

                if *val > 70.0 {
                    println!("    信号: ⚠️ 超买区（RSI > 70）→ 可能回调");
                } else if *val < 30.0 {
                    println!("    信号: ⚠️ 超卖区（RSI < 30）→ 可能反弹");
                } else if *val > 50.0 {
                    println!("    信号: 📊 多头市场（RSI > 50）");
                } else {
                    println!("    信号: 📊 空头市场（RSI < 50）");
                }
            }
            None => {
                println!("    RSI(14): 计算中...");
            }
        }

        println!();
    }

    // ========================================
    // 示例 4: 布林带
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 4: 布林带 (Bollinger Bands)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let bb_result = bollinger_bands(&closes, 20, 2.0);

    println!("计算布林带(20, 2σ)：\n");

    // 显示最近5天的布林带数据
    for (((bar, upper), middle), lower) in data.iter()
        .zip(bb_result.upper.iter())
        .zip(bb_result.middle.iter())
        .zip(bb_result.lower.iter())
        .rev()
        .take(5)
        
    {
        println!("  日期: {:04}-{:02}-{:02}", bar.dt.year, bar.dt.month, bar.dt.day);
        println!("    收盘价: {:.2}", bar.close);

        match (upper, middle, lower) {
            (Some(u), Some(m), Some(l)) => {
                println!("    上轨: {:.2}", u);
                println!("    中轨: {:.2}", m);
                println!("    下轨: {:.2}", l);

                // 计算价格在布林带中的位置
                let position = (bar.close - l) / (u - l) * 100.0;
                println!("    位置: {:.1}%", position);

                if bar.close >= *u {
                    println!("    信号: ⚠️ 触及上轨 → 可能超买");
                } else if bar.close <= *l {
                    println!("    信号: ⚠️ 触及下轨 → 可能超卖");
                } else if bar.close > *m {
                    println!("    信号: 📊 在中轨上方 → 多头优势");
                } else {
                    println!("    信号: 📊 在中轨下方 → 空头优势");
                }

                // 计算带宽（波动率指标）
                let bandwidth = (u - l) / m * 100.0;
                println!("    带宽: {:.2}%", bandwidth);
            }
            _ => {
                println!("    布林带: 计算中...");
            }
        }

        println!();
    }

    // ========================================
    // 示例 5: KDJ 随机指标
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 5: KDJ 随机指标");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let kdj_result = kdj(&highs, &lows, &closes, 9, 3, 3);

    println!("计算 KDJ(9, 3, 3)：\n");

    // 显示最近5天的 KDJ 数据
    for (((bar, k_val), d_val), j_val) in data.iter()
        .zip(kdj_result.k.iter())
        .zip(kdj_result.d.iter())
        .zip(kdj_result.j.iter())
        .rev()
        .take(5)
        
    {
        println!("  日期: {:04}-{:02}-{:02}", bar.dt.year, bar.dt.month, bar.dt.day);

        match (k_val, d_val, j_val) {
            (Some(k), Some(d), Some(j)) => {
                println!("    K: {:.2}", k);
                println!("    D: {:.2}", d);
                println!("    J: {:.2}", j);

                // KDJ 信号判断
                if *k < 20.0 && *d < 20.0 {
                    println!("    信号: ⚠️ 超卖区（K < 20 且 D < 20）→ 买入信号");
                } else if *k > 80.0 && *d > 80.0 {
                    println!("    信号: ⚠️ 超买区（K > 80 且 D > 80）→ 卖出信号");
                } else if *j > 100.0 {
                    println!("    信号: 🔴 严重超买（J > 100）→ 强烈卖出");
                } else if *j < 0.0 {
                    println!("    信号: 🟢 严重超卖（J < 0）→ 强烈买入");
                } else if k > d {
                    println!("    信号: 📈 K > D → 上升趋势");
                } else if k < d {
                    println!("    信号: 📉 K < D → 下降趋势");
                } else {
                    println!("    信号: ➡️ K = D → 中性");
                }
            }
            _ => {
                println!("    KDJ: 计算中...");
            }
        }

        println!();
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ 技术指标计算完成！");

    println!("\n💡 使用建议：");
    println!("  • 结合多个指标进行分析，不要依赖单一指标");
    println!("  • 关注指标的背离信号（价格创新高但指标未创新高）");
    println!("  • 在趋势行情中使用趋势指标（SMA, EMA, MACD）");
    println!("  • 在震荡行情中使用震荡指标（RSI, KDJ）");

    Ok(())
}
