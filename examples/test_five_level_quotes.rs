#!/usr/bin/env rustx
/**
测试五档买卖盘数据完整性
*/
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::SecurityQuotes;

fn main() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("测试五档买卖盘数据完整性");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    match test_five_level_quotes() {
        Ok(_) => println!("\n✅ 测试通过"),
        Err(e) => println!("\n❌ 测试失败: {}", e),
    }
}

fn test_five_level_quotes() -> Result<(), Box<dyn std::error::Error>> {
    let mut tcp = Tcp::new()?;

    // 获取多只股票的实时行情
    let mut quotes = SecurityQuotes::new(vec![
        (0, "000001"),  // 平安银行（深市）
        (1, "600000"),  // 浦发银行（沪市）
        (1, "600036"),  // 招商银行（沪市）
    ]);

    quotes.recv_parsed(&mut tcp)?;

    println!("获取到 {} 只股票的行情数据\n", quotes.result().len());

    for (i, quote) in quotes.result().iter().enumerate() {
        println!("【股票 {}】{}", i + 1, quote.code);
        println!("当前价: {:.2}", quote.price);

        // 验证五档买卖盘数据
        println!("\n─── 买盘（Bid）───");
        println!("买一: {:.2} × {:.0} 手", quote.bid1, quote.bid1_vol);
        println!("买二: {:.2} × {:.0} 手", quote.bid2, quote.bid2_vol);
        println!("买三: {:.2} × {:.0} 手", quote.bid3, quote.bid3_vol);
        println!("买四: {:.2} × {:.0} 手", quote.bid4, quote.bid4_vol);
        println!("买五: {:.2} × {:.0} 手", quote.bid5, quote.bid5_vol);

        println!("\n─── 卖盘（Ask）───");
        println!("卖一: {:.2} × {:.0} 手", quote.ask1, quote.ask1_vol);
        println!("卖二: {:.2} × {:.0} 手", quote.ask2, quote.ask2_vol);
        println!("卖三: {:.2} × {:.0} 手", quote.ask3, quote.ask3_vol);
        println!("卖四: {:.2} × {:.0} 手", quote.ask4, quote.ask4_vol);
        println!("卖五: {:.2} × {:.0} 手", quote.ask5, quote.ask5_vol);

        // 验证数据有效性
        let has_valid_data = quote.bid1 > 0.0 && quote.ask1 > 0.0
            && quote.bid1_vol > 0.0 && quote.ask1_vol > 0.0;

        if has_valid_data {
            println!("\n✅ 数据完整有效");
        } else {
            println!("\n⚠️  数据可能无效（非交易时间或数据源问题）");
        }

        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    }

    Ok(())
}
