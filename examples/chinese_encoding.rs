#!/usr/bin/env rustx
use rustdx_complete::tcp::stock::SecurityList;
/**
测试中文编码是否正确显示
*/
use rustdx_complete::tcp::{Tcp, Tdx};

fn main() {
    println!("🔍 测试中文编码显示\n");

    // 创建TCP连接
    println!("1️⃣  连接到服务器...");
    let mut tcp = match Tcp::new() {
        Ok(tcp) => {
            println!("   ✅ 连接成功\n");
            tcp
        }
        Err(e) => {
            println!("   ❌ 连接失败: {}\n", e);
            return;
        }
    };

    // 获取股票列表（包含中文名称）
    println!("2️⃣  获取深市股票列表（测试中文名称）...");
    let mut list = SecurityList::new(0, 0);

    match list.recv_parsed(&mut tcp) {
        Ok(_) => {
            println!("   ✅ 获取成功\n");
            println!("   📊 前20只股票的名称：");

            for (i, stock) in list.result().iter().take(20).enumerate() {
                println!("      {:2}. {:6} - {}", i + 1, stock.code, stock.name);
            }
        }
        Err(e) => {
            println!("   ❌ 获取失败: {}\n", e);
        }
    }

    // 测试沪市股票
    println!("\n3️⃣  获取沪市股票列表（测试中文名称）...");
    let mut list = SecurityList::new(1, 0);

    match list.recv_parsed(&mut tcp) {
        Ok(_) => {
            println!("   ✅ 获取成功\n");
            println!("   📊 前20只股票的名称：");

            for (i, stock) in list.result().iter().take(20).enumerate() {
                println!("      {:2}. {:6} - {}", i + 1, stock.code, stock.name);
            }
        }
        Err(e) => {
            println!("   ❌ 获取失败: {}\n", e);
        }
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("测试完成");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}
