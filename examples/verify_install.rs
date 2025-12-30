#!/usr/bin/env rustx
/**
验证 rustdx-complete v0.6.0 安装
*/
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::SecurityList;

fn main() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("    rustdx-complete v0.6.0 安装验证");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("📦 包信息:");
    println!("   版本: {}", env!("CARGO_PKG_VERSION"));
    println!("   名称: {}", env!("CARGO_PKG_NAME"));
    println!();

    println!("🔍 测试 1: 连接服务器...");
    match Tcp::new() {
        Ok(mut tcp) => {
            println!("   ✅ 连接成功\n");

            println!("🔍 测试 2: 获取股票列表（中文显示）...");
            let mut list = SecurityList::new(1, 0); // 沪市

            match list.recv_parsed(&mut tcp) {
                Ok(_) => {
                    if list.result().len() > 0 {
                        println!("   ✅ 获取成功\n");

                        println!("📊 前 5 只股票（验证中文编码）:");
                        for (i, stock) in list.result().iter().take(5).enumerate() {
                            println!("   {}. {:6} - {}", i + 1, stock.code, stock.name);
                        }
                        println!();

                        // 验证中文显示
                        let first_stock = &list.result()[0];
                        if !first_stock.name.is_empty() {
                            let has_chinese = first_stock.name.chars().any(|c| c >= '\u{4E00}' && c <= '\u{9FFF}');
                            if has_chinese {
                                println!("✅ 中文编码: 正常");
                                println!("✅ 所有测试通过！\n");
                                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                                println!("    安装验证成功 - rustdx-complete v0.6.0 可正常使用");
                                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                            } else {
                                println!("⚠️  中文编码: 可能未正确显示");
                            }
                        } else {
                            println!("⚠️  未获取到股票名称");
                        }
                    } else {
                        println!("   ❌ 获取失败: 返回数据为空");
                    }
                }
                Err(e) => {
                    println!("   ❌ 获取失败: {}\n", e);
                    println!("   可能原因:");
                    println!("      - 非交易时间");
                    println!("      - 网络问题");
                    println!("      - 服务器临时不可用");
                }
            }
        }
        Err(e) => {
            println!("   ❌ 连接失败: {}", e);
            println!();
            println!("   请检查:");
            println!("      1. 网络连接是否正常");
            println!("      2. 防火墙是否允许连接");
            println!("      3. crates.io 是否已正确安装");
        }
    }

    println!();
    println!("📚 更多示例:");
    println!("   cargo run --example test_security_quotes");
    println!("   cargo run --example test_finance_info");
    println!("   cargo run --example test_chinese_encoding");
    println!();
}
