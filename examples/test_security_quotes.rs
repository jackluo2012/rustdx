#!/usr/bin/env rustx
/**
测试SecurityQuotes功能，获取实时股票行情
*/
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::SecurityQuotes;

fn run_tests(tcp: &mut Tcp) {
    // 测试单只股票
    println!("2️⃣  测试获取单只股票行情 (000001 平安银行)...");
    let mut quotes = SecurityQuotes::new(vec![(0, "000001")]);
    match quotes.recv_parsed(tcp) {
        Ok(_) => {
            println!("   ✅ 获取成功\n");
            for quote in quotes.result() {
                println!("   📊 股票信息:");
                println!("      代码: {}", quote.code);
                println!("      当前价: {:.2}", quote.price);
                println!("      昨收: {:.2}", quote.last_close);
                println!("      今开: {:.2}", quote.open);
                println!("      最高: {:.2}", quote.high);
                println!("      最低: {:.2}", quote.low);
                println!("      成交量: {:.2}手", quote.vol);
                println!("      成交额: {:.2}元", quote.amount);
                println!("      涨跌幅: {:.2}%", quote.change_percent);
                println!("      买一: {:.2} ({:.2}手)", quote.bid1, quote.bid1_vol);
                println!("      卖一: {:.2} ({:.2}手)", quote.ask1, quote.ask1_vol);
            }
        }
        Err(e) => {
            println!("   ❌ 获取失败: {}\n", e);
        }
    }

    println!("\n3️⃣  测试获取多只股票行情...");
    let stocks = vec![
        (0, "000001"),  // 平安银行
        (0, "000002"),  // 万科A
        (1, "600000"),  // 浦发银行
        (1, "600519"),  // 贵州茅台
    ];
    let mut quotes = SecurityQuotes::new(stocks);
    match quotes.recv_parsed(tcp) {
        Ok(_) => {
            println!("   ✅ 获取成功\n");
            println!("   📊 股票行情列表:");
            for quote in quotes.result() {
                println!("      {}: {:.2}元 ({:.2}%)",
                    quote.code,
                    quote.price,
                    quote.change_percent
                );
            }
        }
        Err(e) => {
            println!("   ❌ 获取失败: {}\n", e);
        }
    }
}

fn main() {
    println!("🚀 测试SecurityQuotes功能\n");

    // Tcp::new() 内置服务器列表故障转移（依次尝试直到连接成功）
    println!("1️⃣  连接到通达信服务器...");

    match Tcp::new() {
        Ok(mut tcp) => {
            println!("   ✅ 连接成功\n");
            run_tests(&mut tcp);
        }
        Err(e) => {
            println!("   ❌ 所有服务器连接失败: {e}");
            return;
        }
    }

    println!("\n✅ 测试完成！");
}
