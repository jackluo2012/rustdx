//! 服务器地址管理与连通性检测。对应于 pytdx/mootdx 的 `consts.py` / `best_ip`。
//!
//! 服务器列表主要来自 [mootdx](https://github.com/mootdx/mootdx) `consts.py` 的
//! `HQ_HOSTS`（云服务器，持续维护），并保留原先实测可用的电信主站。
//!
//! [`Tcp::new`](crate::tcp::Tcp::new) 会按列表顺序尝试建立连接（故障转移），
//! 直到某台服务器连接成功。也可用 [`check_alive`] 探测全部服务器后
//! 指定最快的一台：`Tcp::with_config`。

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use crate::tcp::Tcp;

/// 行情服务器列表（按优先级排序）。
pub static STOCK_IP: [SocketAddr; 40] = [
    // 原列表中实测长期可用的电信主站
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(115, 238, 56, 198)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(180, 153, 18, 170)), 7709),
    // mootdx HQ_HOSTS（深圳双线主站）
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(110, 41, 147, 114)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 129, 13, 54)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(120, 24, 149, 49)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(47, 113, 94, 204)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 129, 174, 169)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(110, 41, 154, 219)), 7709),
    // 上海双线主站
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(124, 70, 176, 52)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(47, 100, 236, 28)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(101, 133, 214, 242)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(47, 116, 21, 80)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(47, 116, 105, 28)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(124, 70, 199, 56)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(106, 14, 201, 131)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(106, 14, 190, 242)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(121, 36, 225, 169)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(123, 60, 70, 228)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(123, 60, 73, 44)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(124, 70, 133, 119)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(124, 71, 187, 72)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(124, 71, 187, 122)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(123, 60, 84, 66)), 7709),
    // 北京双线主站
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(121, 36, 54, 217)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(121, 36, 81, 195)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(123, 249, 15, 60)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(124, 70, 75, 113)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(120, 46, 186, 223)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(124, 70, 22, 210)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(139, 9, 133, 247)), 7709),
    // 广州双线主站
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(124, 71, 85, 110)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(139, 9, 51, 18)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(139, 159, 239, 163)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(124, 71, 9, 153)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(116, 205, 163, 254)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(116, 205, 171, 132)), 7709),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(116, 205, 183, 150)), 7709),
    // 其他
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(119, 97, 185, 59)), 7709), // 武汉电信主站
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(47, 107, 64, 168)), 7709), // 深圳双线主站
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(47, 107, 228, 47)), 7719), // 深圳双线主站（7719 端口）
];

/// 依次探测服务器列表，返回可以建立 TCP 连接的地址（保持列表顺序）。
///
/// 对应 mootdx 的 `check_server`。结果可用于挑选可用的服务器，
/// 传给 [`Tcp::with_config`](crate::tcp::Tcp::with_config)。
///
/// ## 示例
/// ```no_run
/// use rustdx_complete::tcp::{ip, Tcp, TcpConfig};
/// use std::time::Duration;
///
/// let alive = ip::check_alive(Duration::from_secs(3));
/// if let Some(fastest) = alive.first() {
///     let config = TcpConfig { timeout: Duration::from_secs(5), ip: Some(*fastest) };
///     let mut tcp = Tcp::with_config(&config).unwrap();
/// }
/// ```
pub fn check_alive(timeout: Duration) -> Vec<SocketAddr> {
    let mut alive = Vec::with_capacity(STOCK_IP.len());
    for addr in STOCK_IP.iter() {
        if tcp_connect_ok(addr, timeout) {
            alive.push(*addr);
        }
    }
    alive
}

/// 探测一台服务器是否可以在超时时间内建立 TCP 连接。
pub fn tcp_connect_ok(addr: &SocketAddr, timeout: Duration) -> bool {
    let start = Instant::now();
    std::net::TcpStream::connect_timeout(addr, timeout).is_ok() && start.elapsed() <= timeout
}

/// 协议级探测：TCP 连接、协议握手、心跳全部成功才算可用。
///
/// 比 [`check_alive`] 慢（部分服务器 TCP 可连但不响应行情协议），
/// 但结果可靠。所有服务器并发探测，总耗时约为最慢单台的耗时。
/// 结果保持列表顺序。
pub fn check_alive_protocol(timeout: Duration) -> Vec<SocketAddr> {
    use crate::tcp::TcpConfig;
    use std::thread;

    thread::scope(|s| {
        let handles: Vec<_> = STOCK_IP
            .iter()
            .enumerate()
            .map(|(i, addr)| {
                s.spawn(move || {
                    let ok = Tcp::with_config(&TcpConfig {
                        timeout,
                        ip: Some(*addr),
                    })
                    .and_then(|mut tcp| tcp.heartbeat())
                    .is_ok();
                    (i, ok.then_some(*addr))
                })
            })
            .collect();
        let mut results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        results.sort_by_key(|(i, _)| *i);
        results.into_iter().filter_map(|(_, a)| a).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::{STOCK_IP, check_alive};
    use crate::tcp::tcpstream_ip;
    use std::time::Duration;

    #[test]
    fn check_all_stock_ips() {
        if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
            println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
            return;
        }
        // 服务器随时间会有增减，要求绝大多数可用（2026-09 实测 40/40 可连）
        let alive = check_alive(Duration::from_secs(3));
        println!(
            "✅ 检测到 {} 个可用服务器 (总共 {} 个):",
            alive.len(),
            STOCK_IP.len()
        );
        for addr in &alive {
            println!("  - {addr}");
        }
        assert!(
            alive.len() >= 10,
            "可用服务器数量不足: 至少需要 10 个，当前有 {} 个。可用: {alive:?}",
            alive.len()
        );
    }

    #[test]
    fn first_server_is_connectable() {
        if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
            println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
            return;
        }
        // 列表首位服务器必须可用——它是 `Tcp::new()` 无故障转移时的首选
        assert!(
            tcpstream_ip(&STOCK_IP[0]).is_ok(),
            "首选服务器 {} 不可用",
            STOCK_IP[0]
        );
    }
}
