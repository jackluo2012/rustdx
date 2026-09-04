#!/usr/bin/env rustx
/**
测试Transaction功能，获取股票成交明细
*/
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::Transaction;

fn main() {
    println!("🚀 测试Transaction功能\n");

    // 创建TCP连接，尝试多个服务器
    println!("1️⃣  连接到通达信服务器...");

    // 首先尝试默认连接
    match Tcp::new() {
        Ok(mut tcp) => {
            println!("   ✅ 连接成功\n");
            test_transaction(&mut tcp);
        }
        Err(e) => {
            println!("   ❌ 默认连接失败: {}，尝试其他服务器...", e);

            // 尝试其他服务器IP
            use rustdx_complete::tcp::ip::STOCK_IP;
            let mut last_error = e.to_string();
            let mut connected = false;

            for (i, ip) in STOCK_IP.iter().enumerate().take(5) {
                println!("\n   尝试服务器 #{}: {}...", i + 1, ip);
                match Tcp::new_with_ip(ip) {
                    Ok(mut tcp) => {
                        println!("   ✅ 连接成功\n");
                        test_transaction(&mut tcp);
                        connected = true;
                        break;
                    }
                    Err(e) => {
                        last_error = format!("{} (服务器#{})", e, i + 1);
                        println!("   ❌ 失败: {}", e);
                    }
                }
            }

            if !connected {
                println!("\n   ❌ 所有服务器连接失败");
                println!("   最后错误: {}\n", last_error);
                return;
            }
        }
    }

    println!("\n✅ 测试完成！");
}

fn test_transaction(tcp: &mut Tcp) {
    // 测试深市股票成交明细
    println!("2️⃣  测试获取000001平安银行的成交明细（前20笔）...");
    let mut transaction = Transaction::new(0, "000001", 0, 20);

    match transaction.recv_parsed(tcp) {
        Ok(_) => {
            println!("   ✅ 获取成功\n");
            println!("   📊 返回数量: {} 笔成交\n", transaction.result().len());

            if !transaction.result().is_empty() {
                println!("   前20笔成交:");
                println!("      时间      价格     成交量   编号   买卖");
                println!("      {}", "-".repeat(47));
                for data in transaction.result().iter().take(20) {
                    let buyorsell_text = match data.buyorsell {
                        0 => "买",
                        1 => "卖",
                        8 => "中性",
                        _ => "未知",
                    };
                    println!("      {} {:>7.2} {:>8} {:>6} {}",
                        data.time, data.price, data.vol, data.num, buyorsell_text);
                }

                // 统计买卖方向
                let buy_count = transaction.result().iter()
                    .filter(|d| d.buyorsell == 0).count();
                let sell_count = transaction.result().iter()
                    .filter(|d| d.buyorsell == 1).count();
                let neutral_count = transaction.result().iter()
                    .filter(|d| d.buyorsell == 8).count();

                println!("\n   📈 统计信息:");
                println!("      买入: {} 笔", buy_count);
                println!("      卖出: {} 笔", sell_count);
                println!("      中性: {} 笔", neutral_count);
            }
        }
        Err(e) => {
            println!("   ❌ 获取失败: {}\n", e);
        }
    }

    // 测试分页获取成交明细
    println!("\n3️⃣  测试分页获取000001的成交明细（start=20, count=20）...");
    let mut transaction = Transaction::new(0, "000001", 20, 20);

    match transaction.recv_parsed(tcp) {
        Ok(_) => {
            println!("   ✅ 获取成功\n");
            println!("   📊 返回数量: {} 笔成交\n", transaction.result().len());

            if !transaction.result().is_empty() {
                println!("   前10笔成交:");
                println!("      时间      价格     成交量   编号   买卖");
                println!("      {}", "-".repeat(47));
                for data in transaction.result().iter().take(10) {
                    let buyorsell_text = match data.buyorsell {
                        0 => "买",
                        1 => "卖",
                        8 => "中性",
                        _ => "未知",
                    };
                    println!("      {} {:>7.2} {:>8} {:>6} {}",
                        data.time, data.price, data.vol, data.num, buyorsell_text);
                }
            }
        }
        Err(e) => {
            println!("   ❌ 获取失败: {}\n", e);
        }
    }

    // 测试沪市股票成交明细
    println!("\n4️⃣  测试获取600000浦发银行的成交明细（前20笔）...");
    let mut transaction = Transaction::new(1, "600000", 0, 20);

    match transaction.recv_parsed(tcp) {
        Ok(_) => {
            println!("   ✅ 获取成功\n");
            println!("   📊 返回数量: {} 笔成交\n", transaction.result().len());

            if !transaction.result().is_empty() {
                println!("   前20笔成交:");
                println!("      时间      价格     成交量   编号   买卖");
                println!("      {}", "-".repeat(47));
                for data in transaction.result().iter().take(20) {
                    let buyorsell_text = match data.buyorsell {
                        0 => "买",
                        1 => "卖",
                        8 => "中性",
                        _ => "未知",
                    };
                    println!("      {} {:>7.2} {:>8} {:>6} {}",
                        data.time, data.price, data.vol, data.num, buyorsell_text);
                }
            }
        }
        Err(e) => {
            println!("   ❌ 获取失败: {}\n", e);
        }
    }
}
