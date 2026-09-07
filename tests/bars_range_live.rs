//! `Client::bars_range` / `Client::k` 升序与区间契约实盘验证。
//!
//! 契约（源自全市场分钟线回补实测的踩坑）：
//! - 升序：服务端返回顺序不保证，解析层统一排序；
//! - 去重：翻页偏移可能重叠；
//! - 窗口：`[begin, end]` 闭区间（YYYYMMDD）；
//! - 乱码年份（2004/2035）不得出现在窗口内结果中。
use rustdx_complete::tcp::stock::{market_of, Client};

fn date_of(dt: &rustdx_complete::tcp::helper::DateTime) -> u32 {
    dt.to_u32()
}

#[test]
fn bars_range_5m_ordered_deduped_in_window() -> std::io::Result<()> {
    if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
        println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
        return Ok(());
    }

    let mut client = Client::new()?;
    let bars = client.bars_range(0, "000001", 0, Some(20260801), Some(20260907))?;
    assert!(!bars.is_empty(), "5 分钟线窗口不应为空");

    // 升序 + 去重（相邻严格递增）
    for w in bars.windows(2) {
        assert!(w[0].dt < w[1].dt, "时间序列必须严格升序且去重");
    }
    // 窗口过滤 + 年份合理（乱码日期不得残留）
    for b in &bars {
        let d = date_of(&b.dt);
        assert!((20260801..=20260907).contains(&d), "越窗日期: {d}");
    }
    // 5 分钟线全部落在交易时段（09:30~11:30 / 13:00~15:00）
    for b in &bars {
        let in_session = (b.dt.hour == 9 && b.dt.minute >= 30)
            || (b.dt.hour == 10)
            || (b.dt.hour == 11 && b.dt.minute <= 30)
            || (b.dt.hour == 13)
            || (b.dt.hour == 14)
            || (b.dt.hour == 15 && b.dt.minute == 0);
        assert!(
            in_session,
            "越出交易时段: {:02}:{:02}",
            b.dt.hour, b.dt.minute
        );
    }
    println!("5m 窗口: {} 根, {} ~ {}", bars.len(), date_of(&bars[0].dt), date_of(&bars[bars.len() - 1].dt));
    Ok(())
}

#[test]
fn k_day_ascending_contract() -> std::io::Result<()> {
    if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
        println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
        return Ok(());
    }

    let mut client = Client::new()?;
    let bars = client.k(1, "600519", Some(20260601), Some(20260907))?;
    assert!(!bars.is_empty(), "日K窗口不应为空");
    for w in bars.windows(2) {
        assert!(w[0].dt < w[1].dt, "日K必须严格升序");
    }
    Ok(())
}

#[test]
fn bars_single_page_ascending() -> std::io::Result<()> {
    if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
        println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
        return Ok(());
    }

    let mut client = Client::new()?;
    let bars = client.bars(0, "000001", 0, 0, 100)?;
    assert!(!bars.is_empty());
    for w in bars.windows(2) {
        assert!(w[0].dt < w[1].dt, "bars 单页也必须升序（契约在解析层落实）");
    }
    Ok(())
}

/// recheck_empty 开启：正常股结果与非开启一致（重试路径不被触发）。
#[test]
fn recheck_empty_config_no_regression() -> std::io::Result<()> {
    if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
        println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
        return Ok(());
    }

    let config = rustdx_complete::tcp::TcpConfig {
        recheck_empty: true,
        retry_delay_ms: 200,
        ..Default::default()
    };
    let mut client = Client::with_config(&config)?;
    let bars = client.k(0, "000001", Some(20260801), Some(20260907))?;
    assert!(!bars.is_empty(), "开启 recheck_empty 后正常股不应为空");
    for w in bars.windows(2) {
        assert!(w[0].dt < w[1].dt);
    }
    let _ = market_of("000001"); // market_of 与 limit::board_of 前缀判定保持可用
    Ok(())
}
