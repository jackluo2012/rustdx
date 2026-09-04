#!/usr/bin/env rustx
/**
调试版：查看指数响应数据的实际大小
*/
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::SecurityQuotes;
use std::net::SocketAddr;

fn main() {
    println!("🔍 调试指数行情数据包大小\n");

    use rustdx_complete::tcp::TcpConfig;
    let addr: SocketAddr = "115.238.56.198:7709".parse().unwrap();
    match Tcp::with_config(&TcpConfig { timeout: std::time::Duration::from_secs(5), ip: Some(addr) }) {
        Ok(mut tcp) => {
            println!("✅ 连接成功\n");

            // 测试普通股票
            println!("1️⃣  测试普通股票(000001平安银行)...");
            let mut quotes = SecurityQuotes::new(vec![(0, "000001")]);
            match quotes.recv(&mut tcp) {
                Ok(_) => {
                    println!("   响应包大小: {} 字节\n", quotes.response.len());
                }
                Err(e) => {
                    println!("   ❌ 失败: {}\n", e);
                }
            }

            // 测试上证指数
            println!("2️⃣  测试上证指数(000001)...");
            let mut quotes = SecurityQuotes::new(vec![(1, "000001")]);
            match quotes.recv(&mut tcp) {
                Ok(_) => {
                    println!("   响应包大小: {} 字节\n", quotes.response.len());
                }
                Err(e) => {
                    println!("   ❌ 失败: {}\n", e);
                }
            }

            // 测试深证成指
            println!("3️⃣  测试深证成指(399001)...");
            let mut quotes = SecurityQuotes::new(vec![(0, "399001")]);
            match quotes.recv(&mut tcp) {
                Ok(_) => {
                    println!("   响应包大小: {} 字节\n", quotes.response.len());
                }
                Err(e) => {
                    println!("   ❌ 失败: {}\n", e);
                }
            }
        }
        Err(e) => {
            println!("❌ 连接失败: {}", e);
        }
    }
}
