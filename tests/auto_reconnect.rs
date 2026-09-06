//! 集成测试：`TcpConfig::auto_reconnect` 自动重连。
//!
//! 真实连接服务器：先成功请求一次，再人为 shutdown 底层 TCP 连接，
//! 下一次请求应触发「失败 → 自动重连 → 重发请求 → 成功」。
//! 用 `RUSTDX_SKIP_INTEGRATION_TESTS=1` 可跳过。

use rustdx_complete::tcp::stock::Client;
use rustdx_complete::tcp::TcpConfig;
use std::net::Shutdown;

#[test]
fn tcp_auto_reconnect_after_shutdown() -> std::io::Result<()> {
    if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
        println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
        return Ok(());
    }

    let config = TcpConfig {
        auto_reconnect: 2,
        ..Default::default()
    };
    let mut c = Client::with_config(&config)?;

    // 第一次请求成功
    let q1 = c.quotes(&[(1, "600000")])?;
    assert!(!q1.is_empty(), "首次行情请求应为空？");

    // 人为断开底层 TCP（服务器端随即感知连接关闭）
    c.tcp.get_ref().0.shutdown(Shutdown::Both)?;

    // 下一次请求：write 失败 → 自动重连 → 重发 → 成功
    let q2 = c.quotes(&[(1, "600000")])?;
    assert!(!q2.is_empty(), "断线后自动重连未恢复");
    println!("断线后自动重连成功: {} 条行情 ✓", q2.len());
    Ok(())
}

/// 默认 auto_reconnect=0 时，断线后请求应失败（保持旧行为）。
#[test]
fn tcp_no_auto_reconnect_fails_after_shutdown() -> std::io::Result<()> {
    if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
        println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
        return Ok(());
    }

    let mut c = Client::new()?; // 默认配置：不自动重连
    let q1 = c.quotes(&[(1, "600000")])?;
    assert!(!q1.is_empty());

    c.tcp.get_ref().0.shutdown(Shutdown::Both)?;
    let r = c.quotes(&[(1, "600000")]);
    assert!(r.is_err(), "默认配置下断线后请求应失败");
    println!("默认配置（auto_reconnect=0）断线后按预期失败 ✓");
    Ok(())
}
