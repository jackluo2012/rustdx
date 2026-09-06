//! 协议探测：K 线请求 22-24 字节是否为服务器端复权开关。
//!
//! 通达信 GET_SECURITY_BARS 请求体在 `category`(20-22) 与 `start`(24-26) 之间有
//! 2 字节固定值（模板为 0x0001）。部分协议文档称其为「复权标志」。本测试对
//! sh600000 最近 3 根日 K 分别以 0x0000 / 0x0001 / 0x0002 请求并逐字节对比响应：
//! 响应完全一致 → 服务器端无复权能力（或忽略该字段），复权必须在本地做；
//! 响应不同 → 服务器支持复权，后续 `k()` 可暴露该参数。

use rustdx_complete::tcp::Tdx;
use rustdx_complete::tcp::{self, Tcp};
use rustdx_complete::tcp::stock::Kline;

#[test]
fn probe_kline_adj_field() -> std::io::Result<()> {
    if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
        println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
        return Ok(());
    }

    let mut tcp = Tcp::new()?;

    // 三种字段值下解析出的日 K 数据
    let mut rows = Vec::new();
    for field in [0x0000u16, 0x0001, 0x0002, 0x0100] {
        let mut k = Kline::new(1, "600000", 9, 0, 3);
        k.send[22..24].copy_from_slice(&field.to_le_bytes());
        let raw = tcp::send_recv(&mut tcp, &k.send, "probe-adj")?.0;
        k.recv_parsed(&mut tcp)?;
        println!(
            "field=0x{field:04x} raw[{}]: {}",
            raw.len(),
            raw.iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
        let s: Vec<String> = k
            .result()
            .iter()
            .map(|b| format!("{:?}/c{:.2}", b.dt, b.close))
            .collect();
        println!("  -> parsed {} 根: {}", s.len(), s.join("  "));
        rows.push(s);
    }

    // 字段值 0x0001 与 0x0000/0x0002 的数据是否一致
    println!("0x0001 vs 0x0000 价格一致: {}", rows[1] == rows[0]);
    println!("0x0001 vs 0x0002 价格一致: {}", rows[1] == rows[2]);
    Ok(())
}
