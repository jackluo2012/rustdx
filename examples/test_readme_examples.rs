#!/usr/bin/env rustx
/**
测试 README 文档中的所有示例代码
*/
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::{SecurityQuotes, Kline, FinanceInfo, MinuteTime, Transaction, SecurityList};

fn main() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("测试 README 文档示例代码");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 测试 1: 获取股票实时行情
    println!("1️⃣  测试：获取股票实时行情");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    match test_security_quotes() {
        Ok(_) => println!("✅ 通过\n"),
        Err(e) => println!("❌ 失败: {}\n", e),
    }

    // 测试 2: 获取指数行情
    println!("2️⃣  测试：获取指数行情");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    match test_index_quotes() {
        Ok(_) => println!("✅ 通过\n"),
        Err(e) => println!("❌ 失败: {}\n", e),
    }

    // 测试 3: 获取 K 线数据
    println!("3️⃣  测试：获取 K 线数据");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    match test_kline() {
        Ok(_) => println!("✅ 通过\n"),
        Err(e) => println!("❌ 失败: {}\n", e),
    }

    // 测试 4: 获取财务信息
    println!("4️⃣  测试：获取财务信息");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    match test_finance_info() {
        Ok(_) => println!("✅ 通过\n"),
        Err(e) => println!("❌ 失败: {}\n", e),
    }

    // 测试 5: 获取分时数据
    println!("5️⃣  测试：获取分时数据");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    match test_minute_time() {
        Ok(_) => println!("✅ 通过\n"),
        Err(e) => println!("❌ 失败: {}\n", e),
    }

    // 测试 6: 获取逐笔成交
    println!("6️⃣  测试：获取逐笔成交");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    match test_transaction() {
        Ok(_) => println!("✅ 通过\n"),
        Err(e) => println!("❌ 失败: {}\n", e),
    }

    // 测试 7: 获取股票列表
    println!("7️⃣  测试：获取股票列表");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    match test_security_list() {
        Ok(_) => println!("✅ 通过\n"),
        Err(e) => println!("❌ 失败: {}\n", e),
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("测试完成");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

fn test_security_quotes() -> Result<(), Box<dyn std::error::Error>> {
    let mut tcp = Tcp::new()?;

    // 获取多只股票的实时行情
    let mut quotes = SecurityQuotes::new(vec![
        (0, "000001"),  // 平安银行（深市）
        (1, "600000"),  // 浦发银行（沪市）
    ]);

    quotes.recv_parsed(&mut tcp)?;

    for quote in quotes.result() {
        // ⚠️ 问题 1: quote.name 字段为空！
        println!("{}: '{}' - 当前价: {}", quote.code, quote.name, quote.price);
    }

    Ok(())
}

fn test_index_quotes() -> Result<(), Box<dyn std::error::Error>> {
    let mut tcp = Tcp::new()?;

    // 获取主要指数行情
    let mut quotes = SecurityQuotes::new(vec![
        (1, "000001"),  // 上证指数
        (0, "399001"),  // 深证成指
        (1, "000300"),  // 沪深300
    ]);

    quotes.recv_parsed(&mut tcp)?;

    for quote in quotes.result() {
        println!("{}: {} (涨跌: {}%)", quote.code, quote.price, quote.change_percent);
    }

    Ok(())
}

fn test_kline() -> Result<(), Box<dyn std::error::Error>> {
    let mut tcp = Tcp::new()?;
    let mut kline = Kline::new(1, "600000", 9, 0, 10);

    kline.recv_parsed(&mut tcp)?;

    for bar in kline.result() {
        println!("{:?} : 开({}) 高({}) 低({}) 收({})",
            bar.dt, bar.open, bar.high, bar.low, bar.close);
    }

    Ok(())
}

fn test_finance_info() -> Result<(), Box<dyn std::error::Error>> {
    let mut tcp = Tcp::new()?;
    let mut finance = FinanceInfo::new(0, "000001");

    finance.recv_parsed(&mut tcp)?;

    let info = &finance.result()[0];
    println!("股票代码: {}", info.code);
    println!("总股本: {:.0} 股", info.zongguben);
    println!("净资产: {:.0} 元", info.jingzichan);
    println!("净利润: {:.0} 元", info.jinglirun);

    Ok(())
}

fn test_minute_time() -> Result<(), Box<dyn std::error::Error>> {
    let mut tcp = Tcp::new()?;
    let mut minute = MinuteTime::new(0, "000001");

    minute.recv_parsed(&mut tcp)?;

    for (i, data) in minute.result().iter().take(10).enumerate() {
        println!("{} : 价格={} 成交量={}", i + 1, data.price, data.vol);
    }

    Ok(())
}

fn test_transaction() -> Result<(), Box<dyn std::error::Error>> {
    let mut tcp = Tcp::new()?;
    let mut transaction = Transaction::new(0, "000001", 0, 20);

    transaction.recv_parsed(&mut tcp)?;

    for data in transaction.result().iter().take(5) {
        println!("价格={} 成交量={} 买卖方向={}",
            data.price, data.vol, data.buyorsell);
    }

    // 安全地获取最后一笔成交
    if let Some(last) = transaction.result().last() {
        println!("最新成交序号: {}", last.num);
    }

    Ok(())
}

fn test_security_list() -> Result<(), Box<dyn std::error::Error>> {
    let mut tcp = Tcp::new()?;

    // 第一次获取：从0开始，获取1000只股票
    let mut list = SecurityList::new(0, 0);

    list.recv_parsed(&mut tcp)?;

    println!("股票列表（前5只）：");
    for (i, stock) in list.result().iter().take(5).enumerate() {
        println!("{}: 代码={}, 名称={}",
            i + 1, stock.code, stock.name);
    }

    println!("本批次获取: {} 只股票", list.result().len());

    Ok(())
}
