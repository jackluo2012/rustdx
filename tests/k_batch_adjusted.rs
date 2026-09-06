//! 集成测试：`Client::k_batch`（连接池并行）与 `Client::k_adjusted`（本地复权）。
//!
//! 均连接真实通达信服务器。用 `RUSTDX_SKIP_INTEGRATION_TESTS=1` 可跳过。

use rustdx_complete::tcp::helper::DateTime;
use rustdx_complete::tcp::stock::{Adj, Client};
use std::collections::HashSet;

/// k_batch 结果与逐只 k() 完全一致（日期 + 收盘价），且顺序保持输入顺序。
#[test]
fn tcp_k_batch_matches_k() -> std::io::Result<()> {
    if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
        println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
        return Ok(());
    }

    let stocks = [(1u16, "600000"), (0, "000001"), (1, "600519"), (0, "000858")];
    let client = Client::new()?;
    let batch = client.k_batch(&stocks, Some(20250101), None, 4)?;
    assert_eq!(batch.len(), stocks.len(), "返回数量应等于输入数量");

    let mut c = Client::new()?;
    for (row, &(m, code)) in batch.iter().zip(&stocks) {
        let d = row.result.as_ref().expect("单只拉取应成功");
        let single = c.k(m, code, Some(20250101), None)?;
        assert_eq!(d.len(), single.len(), "{code} 根数不一致");
        for (a, b) in d.iter().zip(&single) {
            assert_eq!(
                DateTime::to_u32(a.dt.clone()),
                DateTime::to_u32(b.dt.clone()),
                "{code} 日期不一致"
            );
            assert!((a.close - b.close).abs() < 1e-9, "{code} 收盘价不一致");
        }
        println!("{code}: {} 根 ✓", d.len());
    }
    Ok(())
}

/// k_batch 输入顺序返回 + 空列表安全。
#[test]
fn tcp_k_batch_empty_and_order() -> std::io::Result<()> {
    if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
        println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
        return Ok(());
    }
    let client = Client::new()?;
    assert!(client.k_batch(&[], None, None, 4)?.is_empty());
    let rows = client.k_batch(&[(1, "600000"), (1, "600519")], Some(20260101), None, 2)?;
    assert_eq!(rows[0].code, "600000");
    assert_eq!(rows[1].code, "600519");
    Ok(())
}

/// 复权验证（sh600519 贵州茅台，上市以来多次分红除权）：
/// 1. 前复权最新收盘 = 原始最新收盘（以最新为基准）；
/// 2. 后复权首日收盘 = 原始首日收盘（以上市为基准）；
/// 3. 原始序列中除权日开盘相对前收明显跳空，前复权后跳空被消除（收益率连续）。
#[test]
fn tcp_k_adjusted_qfq_hfq() -> std::io::Result<()> {
    if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
        println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
        return Ok(());
    }

    let mut c = Client::new()?;
    let raw = c.k(1, "600519", None, None)?;
    assert!(!raw.is_empty(), "茅台日K为空");
    let qfq = c.k_adjusted(1, "600519", Adj::Qfq, None, None)?;
    let hfq = c.k_adjusted(1, "600519", Adj::Hfq, None, None)?;
    assert_eq!(qfq.len(), raw.len());
    assert_eq!(hfq.len(), raw.len());

    // 1. 前复权最新价 = 实际最新价
    let last = raw.last().unwrap();
    let qlast = qfq.last().unwrap();
    assert!(
        (qlast.close - last.close).abs() < 1e-6,
        "前复权最新价 {:.2} != 原始 {:.2}",
        qlast.close,
        last.close
    );
    // 2. 后复权首日价 = 实际首日价
    let first = raw.first().unwrap();
    let hfirst = hfq.first().unwrap();
    assert!(
        (hfirst.close - first.close).abs() < 1e-6,
        "后复权首日价 {:.2} != 原始 {:.2}",
        hfirst.close,
        first.close
    );

    // 3. 除权日跳空消除：复权后除权日开盘相对前收的"跳空"应等于
    //    真实日内波动（开盘相对除权参考价的涨跌），即回到正常范围（<5%）；
    //    原始序列中除权日开盘相对前收会因除权参考价大幅跳空（远超 5%）。
    let exdiv: HashSet<u32> = c
        .xdxr(1, "600519")?
        .iter()
        .filter(|x| x.category == 1)
        .map(|x| x.date)
        .collect();
    let mut checked = 0;
    let mut removed = 0;
    for i in 1..raw.len() {
        let d = DateTime::to_u32(raw[i].dt.clone());
        if !exdiv.contains(&d) {
            continue;
        }
        // 原始序列：除权日开盘相对前收的跳空
        let gap_raw = raw[i].open / raw[i - 1].close - 1.0;
        // 前复权序列：同日的跳空（应为真实日内波动，而非除权跳空）
        let gap_q = qfq[i].open / qfq[i - 1].close - 1.0;
        println!(
            "除权日 {d}: 原始跳空 {:.4} → 前复权 {:.4}",
            gap_raw, gap_q
        );
        checked += 1;
        if gap_raw.abs() > 0.01 {
            assert!(
                gap_q.abs() < 0.05,
                "除权日 {d} 复权后跳空 {gap_q:.4} 超出正常日内波动（原始 {gap_raw:.4}）"
            );
            removed += 1;
        }
    }
    assert!(checked >= 5, "茅台除权事件异常少: {checked}");
    assert!(removed >= 2, "未发现可验证的除权跳空: {removed}");
    println!("验证除权日 {checked} 个，消除跳空 {removed} 个 ✓");
    Ok(())
}

/// 复权区间过滤：k_adjusted 的 begin/end 与 k() 一致。
#[test]
fn tcp_k_adjusted_range() -> std::io::Result<()> {
    if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
        println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
        return Ok(());
    }
    let mut c = Client::new()?;
    let q = c.k_adjusted(1, "600519", Adj::Qfq, Some(20240101), Some(20241231))?;
    assert!(!q.is_empty());
    for bar in &q {
        let d = DateTime::to_u32(bar.dt.clone());
        assert!((20240101..=20241231).contains(&d));
    }
    println!("2024 年 qfq {} 根 ✓", q.len());
    Ok(())
}
