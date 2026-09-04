#!/usr/bin/env rustx
/**
测试指数行情功能
使用SecurityQuotes获取上证指数、深证成指等实时行情数据
*/
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::SecurityQuotes;

fn main() {
    println!("🚀 测试指数行情功能\n");

    // Tcp::new() 内置服务器列表故障转移（依次尝试直到连接成功）
    println!("1️⃣  连接到通达信服务器...");

    match Tcp::new() {
        Ok(mut tcp) => {
            println!("   ✅ 连接成功\n");
            test_index_quotes(&mut tcp);
        }
        Err(e) => {
            println!("   ❌ 所有服务器连接失败: {e}");
            return;
        }
    }

    println!("\n✅ 测试完成！");
}

fn test_index_quotes(tcp: &mut Tcp) {
    // 测试上证指数
    println!("2️⃣  测试获取上证指数(000001)行情...");
    println!("{}", "=".repeat(60));

    let mut quotes = SecurityQuotes::new(vec![(1, "000001")]); // 上证指数：market=1, code=000001

    match quotes.recv_parsed(tcp) {
        Ok(_) => {
            if !quotes.result().is_empty() {
                let quote = &quotes.result()[0];
                println!("   📊 上证指数行情:");
                println!("      代码: {}", quote.code);
                println!("      当前价: {:.2}", quote.price);
                println!("      昨收: {:.2}", quote.last_close);
                println!("      今开: {:.2}", quote.open);
                println!("      最高: {:.2}", quote.high);
                println!("      最低: {:.2}", quote.low);
                println!("      成交量: {:.0} 手", quote.vol);
                println!("      成交额: {:.0} 元", quote.amount);
            }
        }
        Err(e) => {
            println!("   ❌ 获取失败: {}", e);
        }
    }

    // 测试深证成指
    println!("\n3️⃣  测试获取深证成指(399001)行情...");
    println!("{}", "=".repeat(60));

    let mut quotes = SecurityQuotes::new(vec![(0, "399001")]); // 深证成指：market=0, code=399001

    match quotes.recv_parsed(tcp) {
        Ok(_) => {
            if !quotes.result().is_empty() {
                let quote = &quotes.result()[0];
                println!("   📊 深证成指行情:");
                println!("      代码: {}", quote.code);
                println!("      当前价: {:.2}", quote.price);
                println!("      昨收: {:.2}", quote.last_close);
                println!("      今开: {:.2}", quote.open);
                println!("      最高: {:.2}", quote.high);
                println!("      最低: {:.2}", quote.low);
            }
        }
        Err(e) => {
            println!("   ❌ 获取失败: {}", e);
        }
    }

    // 测试同时获取多个指数
    println!("\n4️⃣  测试同时获取多个指数行情...");
    println!("{}", "=".repeat(60));

    let mut quotes = SecurityQuotes::new(vec![
        (1, "000001"), // 上证指数
        (0, "399001"), // 深证成指
        (1, "000300"), // 沪深300
    ]);

    match quotes.recv_parsed(tcp) {
        Ok(_) => {
            println!("   获取到 {} 个指数的行情数据:\n", quotes.result().len());

            for (i, quote) in quotes.result().iter().enumerate() {
                println!("   指数 #{}:", i + 1);
                println!("      代码: {}", quote.code);
                println!("      当前价: {:.2}", quote.price);
                println!("      涨跌: {:.2}%", quote.change_percent);
                println!();
            }
        }
        Err(e) => {
            println!("   ❌ 获取失败: {}", e);
        }
    }
}
