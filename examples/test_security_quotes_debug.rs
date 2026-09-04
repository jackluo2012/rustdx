#!/usr/bin/env rustx
use rustdx_complete::tcp::stock::SecurityQuotes;
/**
测试SecurityQuotes功能，并打印调试信息
*/
use rustdx_complete::tcp::{Tcp, Tdx};

fn main() {
    println!("🚀 测试SecurityQuotes功能（调试模式）\n");

    // 创建TCP连接
    println!("1️⃣  连接到通达信服务器...");
    let mut tcp = match Tcp::new() {
        Ok(t) => {
            println!("   ✅ 连接成功\n");
            t
        }
        Err(e) => {
            println!("   ❌ 连接失败: {e}");
            return;
        }
    };

    test_quotes(&mut tcp);
}

fn test_quotes(tcp: &mut Tcp) {
    println!("2️⃣  测试获取单只股票行情 (000001 平安银行)...");
    let mut quotes = SecurityQuotes::new(vec![(0, "000001")]);

    // 打印请求包
    println!("   📤 请求包信息:");
    let send_bytes = quotes.send();
    println!("      长度: {} 字节", send_bytes.len());
    println!("      前30字节（hex）:");
    for i in (0..send_bytes.len().min(30)).step_by(8) {
        let end = (i + 8).min(send_bytes.len());
        let hex_str: String = send_bytes[i..end]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");
        println!("         字节 {:2}-{:2}: {}", i, end - 1, hex_str);
    }

    match quotes.recv_parsed(tcp) {
        Ok(_) => {
            println!("   ✅ 获取成功\n");
            for quote in quotes.result() {
                println!("   📊 股票信息:");
                println!("      代码: {}", quote.code);
                println!("      当前价: {:.2}", quote.price);
                println!("      涨跌幅: {:.2}%", quote.change_percent);
            }
        }
        Err(e) => {
            println!("   ❌ 获取失败: {}\n", e);
        }
    }

    println!("\n✅ 测试完成！");
}
