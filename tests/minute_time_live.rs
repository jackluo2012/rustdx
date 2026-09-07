//! 当日分时实盘验证（2026 新协议解析）。
//!
//! - 与「今日历史分时」（协议稳定）交叉对照：点数与末点价格应一致；
//! - 多服务器解析结果应一致。
//!
//! 非交易日/集合竞价前无当日分时（返回空），此时跳过对照断言。
use rustdx_complete::tcp::stock::{Client, HistoryMinuteTime};
use rustdx_complete::tcp::{TcpConfig, Tdx};
use std::time::Duration;

fn skip_if_no_live(data: &[(f64, i32)]) -> bool {
    if data.is_empty() {
        println!("⚠️  当前无当日分时（非交易日或未开盘），跳过对照断言");
        return true;
    }
    false
}

/// Client::minute（新协议解析）与今日历史分时（旧协议，稳定）逐项对照。
#[test]
fn minute_time_live_matches_history() -> std::io::Result<()> {
    if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
        println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
        return Ok(());
    }

    let mut client = Client::new()?;
    for (market, code) in [(0u16, "000001"), (1, "600000")] {
        let live = client.minute(market, code)?;
        if skip_if_no_live(
            &live.iter().map(|d| (d.price, d.vol)).collect::<Vec<_>>(),
        ) {
            return Ok(());
        }

        let today: u32 = {
            use chrono::Datelike;
            let d = chrono::Local::now().date_naive();
            (d.year() as u32) * 10000 + d.month() * 100 + d.day()
        };
        let mut hmt = HistoryMinuteTime::new(market, code, today);
        hmt.recv_parsed(&mut client.tcp)?;
        let hist = hmt.result();

        assert!(!hist.is_empty(), "{code}: 今日历史分时为空，无法对照");
        assert_eq!(
            live.len(),
            hist.len(),
            "{code}: 当日分时与历史分时点数不一致"
        );
        // 盘中两次请求间隔数秒，允许末点价格微小差异
        let diff = (live[live.len() - 1].price - hist[hist.len() - 1].price).abs();
        assert!(
            diff <= 0.02,
            "{code}: 末点价格不一致: live={:.2} hist={:.2}",
            live[live.len() - 1].price,
            hist[hist.len() - 1].price
        );
        println!("{code}: {}/{} 点对照一致 ✓", live.len(), hist.len());
    }
    Ok(())
}

/// 多台不同服务器的当日分时解析结果应一致（点数与末点价格）。
#[test]
fn minute_time_live_multi_server() -> std::io::Result<()> {
    if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
        println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
        return Ok(());
    }

    let mut results = Vec::new();
    for index in [0usize, 2, 8] {
        let config = TcpConfig::with_index(index, Duration::from_secs(5));
        let mut client = match Client::with_config(&config) {
            Ok(c) => c,
            Err(e) => {
                println!("服务器 #{index} 连接失败（跳过）: {e}");
                continue;
            }
        };
        let data = client.minute(0, "000001")?;
        if skip_if_no_live(&data.iter().map(|d| (d.price, d.vol)).collect::<Vec<_>>()) {
            return Ok(());
        }
        let last = data[data.len() - 1].price;
        println!("服务器 #{index}: {} 点, 末点 {last:.2}", data.len());
        results.push((index, data.len(), last));
    }
    assert!(results.len() >= 2, "可用服务器不足 2 台，无法对照");
    for w in results.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        assert_eq!(a.1, b.1, "服务器 #{} 与 #{} 点数不一致", a.0, b.0);
        assert!(
            (a.2 - b.2).abs() <= 0.02,
            "服务器 #{} 与 #{} 末点价格不一致: {:.2} vs {:.2}",
            a.0,
            b.0,
            a.2,
            b.2
        );
    }
    Ok(())
}
