use rustdx_complete::tcp::stock::SecurityQuotes;
/**
 * rustdx-complete 连接测试示例
 *
 * 演示如何正确连接到通达信公共服务器
 *
 * 重要提示:
 * - 不需要在本地运行通达信服务
 * - 直接连接到远程服务器 (115.238.56.198:7709)
 * - 内置多个备用服务器自动切换
 */
use rustdx_complete::tcp::{Tcp, Tdx};
use std::net::SocketAddr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 rustdx-complete 连接测试\n");

    // ============================================================
    // 方法1: 使用默认连接（推荐）
    // ============================================================
    println!("方法1: 使用默认连接");
    println!("连接到默认服务器: 115.238.56.198:7709\n");

    match Tcp::new() {
        Ok(mut tcp) => {
            println!("✅ 连接成功！\n");

            // 查询股票数据
            let mut quotes = SecurityQuotes::new(vec![
                (0, "000001"), // 平安银行
                (1, "600000"), // 浦发银行
            ]);

            match quotes.recv_parsed(&mut tcp) {
                Ok(_) => {
                    println!("📊 股票行情:");
                    for quote in quotes.result() {
                        println!(
                            "  {}: {:.2}元 ({:+.2}%)",
                            quote.code, quote.price, quote.change_percent
                        );
                    }
                    println!("\n✅ 数据获取成功！\n");
                }
                Err(e) => {
                    println!("❌ 获取数据失败: {}\n", e);
                }
            }
        }
        Err(e) => {
            println!("❌ 连接失败: {}", e);
            println!("尝试方法2...\n");
        }
    }

    // ============================================================
    // 方法2: 指定服务器地址
    // ============================================================
    println!("方法2: 指定服务器地址");

    let server_addrs = vec![
        "114.80.149.19:7709",
        "114.80.149.22:7709",
        "218.108.47.69:7709",
    ];

    let mut connected = false;

    for addr_str in server_addrs {
        println!("尝试连接到 {}...", addr_str);

        match addr_str.parse::<SocketAddr>() {
            Ok(addr) => {
                match Tcp::with_config(&rustdx_complete::tcp::TcpConfig {
                    timeout: std::time::Duration::from_secs(5),
                    ip: Some(addr),
                }) {
                    Ok(mut tcp) => {
                        println!("✅ 连接成功！\n");

                        // 简单测试
                        let mut quotes = SecurityQuotes::new(vec![(0, "000001")]);
                        if quotes.recv_parsed(&mut tcp).is_ok() {
                            println!("✅ 数据查询正常\n");
                            connected = true;
                            break;
                        }
                    }
                    Err(e) => {
                        println!("❌ 失败: {}\n", e);
                    }
                }
            }
            Err(e) => {
                println!("❌ 无效地址: {}\n", e);
            }
        }
    }

    if connected {
        println!("✅ 找到可用服务器\n");
    } else {
        println!("⚠️  所有服务器连接失败\n");
        println!("可能的原因:");
        println!("  1. 网络连接问题");
        println!("  2. 防火墙阻止");
        println!("  3. 需要配置代理\n");
        println!("解决方案:");
        println!("  1. 运行诊断脚本: ./scripts/diagnose_network.sh");
        println!("  2. 查看文档: NETWORK_CONNECTION_GUIDE.md");
        println!("  3. 检查网络: ping 115.238.56.198\n");
    }

    // ============================================================
    // 方法3: 自动尝试所有服务器
    // ============================================================
    println!("方法3: 自动尝试所有内置服务器");

    use rustdx_complete::tcp::ip::STOCK_IP;

    let mut working_servers = Vec::new();

    for (i, addr) in STOCK_IP.iter().enumerate().take(10) {
        if i % 3 == 0 {
            println!("正在扫描... {}/10", i + 1);
        }

        if rustdx_complete::tcp::Tcp::with_config(&rustdx_complete::tcp::TcpConfig {
            timeout: std::time::Duration::from_secs(5),
            ip: Some(*addr),
        })
        .is_ok()
        {
            working_servers.push(*addr);
        }
    }

    println!("\n找到 {} 个可用服务器:", working_servers.len());
    for (i, addr) in working_servers.iter().enumerate() {
        println!("  {}. {}", i + 1, addr);
    }

    if working_servers.is_empty() {
        println!("\n❌ 未找到可用服务器");
        println!("请检查网络连接或防火墙设置\n");
    } else {
        println!("\n✅ rustdx-complete 可以正常使用！\n");
    }

    Ok(())
}
