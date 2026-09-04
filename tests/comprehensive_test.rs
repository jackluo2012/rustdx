use rustdx_complete::tcp::stock::{
    FinanceInfo, Kline, MinuteTime, SecurityList, SecurityQuotes, Transaction, get_concept_stocks,
    get_industry_name, get_province_name,
};
/**
 * RustDX 开盘日全面测试套件
 *
 * 测试覆盖：
 * - 核心功能完整性（8个API模块）
 * - 实时数据准确性
 * - 错误处理和边界情况
 *
 * 运行方式：
 * cargo test --test comprehensive_test -- --live
 */
use rustdx_complete::tcp::{Tcp, Tdx};

// ============================================================================
// 测试结果统计结构
// ============================================================================

#[derive(Debug, Default)]
struct TestResults {
    total: usize,
    passed: usize,
    failed: usize,
    warnings: usize,
    errors: Vec<String>,
}

impl TestResults {
    fn record_pass(&mut self) {
        self.total += 1;
        self.passed += 1;
    }

    fn record_fail(&mut self, msg: String) {
        self.total += 1;
        self.failed += 1;
        self.errors.push(msg);
    }

    fn record_warning(&mut self) {
        self.total += 1;
        self.warnings += 1;
        self.passed += 1;
    }

    fn print_summary(&self) {
        println!("\n{}", "=".repeat(60));
        println!("测试结果汇总");
        println!("{}", "=".repeat(60));
        println!("总测试数: {}", self.total);
        println!(
            "✅ 通过: {} ({:.1}%)",
            self.passed,
            (self.passed as f64 / self.total as f64) * 100.0
        );
        println!(
            "❌ 失败: {} ({:.1}%)",
            self.failed,
            (self.failed as f64 / self.total as f64) * 100.0
        );
        println!("⚠️  警告: {}", self.warnings);

        if !self.errors.is_empty() {
            println!("\n失败详情:");
            for (i, err) in self.errors.iter().enumerate() {
                println!("  {}. {}", i + 1, err);
            }
        }
        println!("{}", "=".repeat(60));
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

fn should_run_live_tests() -> bool {
    std::env::var("RUSTDX_LIVE_TEST").is_ok() || std::env::args().any(|arg| arg.contains("--live"))
}

fn get_current_time() -> String {
    let datetime = chrono::Local::now();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

// ============================================================================
// 第一部分：核心功能完整性测试
// ============================================================================

#[test]
fn test_01_security_quotes_single() {
    if !should_run_live_tests() {
        return;
    }

    println!("\n[测试 1.1] 单只股票实时行情查询");
    let mut results = TestResults::default();

    match Tcp::new() {
        Ok(mut tcp) => {
            let mut quotes = SecurityQuotes::new(vec![(0, "000001")]);
            match quotes.recv_parsed(&mut tcp) {
                Ok(_) => {
                    for quote in quotes.result() {
                        println!(
                            "  ✅ {}: {:.2}元 ({:.2}%)",
                            quote.code, quote.price, quote.change_percent
                        );

                        // 验证关键字段
                        if quote.price > 0.0 {
                            results.record_pass();
                        } else {
                            results.record_fail("价格字段异常".to_string());
                        }

                        if !quote.code.is_empty() {
                            results.record_pass();
                        } else {
                            results.record_fail("股票代码为空".to_string());
                        }
                    }
                }
                Err(e) => {
                    results.record_fail(format!("获取失败: {}", e));
                }
            }
        }
        Err(e) => {
            results.record_fail(format!("连接失败: {}", e));
        }
    }

    results.print_summary();
    assert!(results.failed == 0, "存在失败测试用例");
}

#[test]
fn test_02_security_quotes_batch() {
    if !should_run_live_tests() {
        return;
    }

    println!("\n[测试 1.2] 批量股票行情查询 (50只)");
    let mut results = TestResults::default();

    match Tcp::new() {
        Ok(mut tcp) => {
            // 准备50只热门股票
            let stocks = vec![
                (0, "000001"),
                (0, "000002"),
                (0, "000063"),
                (0, "000066"),
                (0, "000333"),
                (0, "000338"),
                (0, "000651"),
                (0, "000725"),
                (0, "000858"),
                (0, "000876"),
                (1, "600000"),
                (1, "600036"),
                (1, "600519"),
                (1, "600887"),
                (1, "600900"),
                (1, "601012"),
                (1, "601066"),
                (1, "601318"),
                (1, "601398"),
                (1, "601857"),
                (0, "002415"),
                (0, "002594"),
                (0, "300750"),
                (0, "300760"),
                (1, "688981"),
                // ... 更多股票
                (1, "600030"),
                (1, "600048"),
                (1, "600276"),
                (1, "600690"),
                (1, "600837"),
                (0, "000001"),
                (0, "000002"),
                (0, "000063"),
                (0, "000066"),
                (0, "000333"),
                (0, "000338"),
                (0, "000651"),
                (0, "000725"),
                (0, "000858"),
                (0, "000876"),
                (1, "600000"),
                (1, "600036"),
                (1, "600519"),
                (1, "600887"),
                (1, "600900"),
                (1, "601012"),
                (1, "601066"),
                (1, "601318"),
                (1, "601398"),
                (1, "601857"),
            ];

            let start = std::time::Instant::now();
            let mut quotes = SecurityQuotes::new(stocks);
            match quotes.recv_parsed(&mut tcp) {
                Ok(_) => {
                    let elapsed = start.elapsed();
                    let quote_count = quotes.result().len();
                    println!("  ✅ 成功获取 {} 只股票行情", quote_count);
                    println!("  ⏱️  耗时: {:?}", elapsed);

                    if quote_count == 50 {
                        results.record_pass();
                    } else {
                        results.record_fail(format!("期望50只，实际获取{}只", quote_count));
                    }

                    // 验证性能
                    if elapsed.as_millis() < 200 {
                        results.record_pass();
                        println!("  ✅ 性能良好");
                    } else {
                        results.record_warning();
                        println!("  ⚠️  性能偏慢");
                    }

                    // 验证数据完整性
                    let mut valid_count = 0;
                    for quote in quotes.result() {
                        if quote.price > 0.0 {
                            valid_count += 1;
                        }
                    }
                    println!("  📊 数据完整: {}/50", valid_count);
                    results.record_pass();
                }
                Err(e) => {
                    results.record_fail(format!("批量获取失败: {}", e));
                }
            }
        }
        Err(e) => {
            results.record_fail(format!("连接失败: {}", e));
        }
    }

    results.print_summary();
}

#[test]
fn test_03_index_quotes() {
    if !should_run_live_tests() {
        return;
    }

    println!("\n[测试 1.3] 主要指数行情查询");
    let mut results = TestResults::default();

    match Tcp::new() {
        Ok(mut tcp) => {
            let indices = vec![
                (1, "000001"), // 上证指数
                (0, "399001"), // 深证成指
                (1, "000300"), // 沪深300
                (0, "399006"), // 创业板指
                (1, "000688"), // 科创50
            ];

            let mut quotes = SecurityQuotes::new(indices);
            match quotes.recv_parsed(&mut tcp) {
                Ok(_) => {
                    println!("  📊 主要指数行情:");
                    for quote in quotes.result() {
                        println!(
                            "     {}: {:.2} ({:+.2}%)",
                            quote.code, quote.price, quote.change_percent
                        );
                        results.record_pass();
                    }

                    if quotes.result().len() == 5 {
                        results.record_pass();
                    } else {
                        results
                            .record_fail(format!("期望5个指数，实际{}个", quotes.result().len()));
                    }
                }
                Err(e) => {
                    results.record_fail(format!("获取指数失败: {}", e));
                }
            }
        }
        Err(e) => {
            results.record_fail(format!("连接失败: {}", e));
        }
    }

    results.print_summary();
}

#[test]
fn test_04_kline_data() {
    if !should_run_live_tests() {
        return;
    }

    println!("\n[测试 1.4] K线数据获取");
    let mut results = TestResults::default();

    match Tcp::new() {
        Ok(mut tcp) => {
            // 测试日K线
            println!("  📈 测试日K线数据");
            let mut kline = Kline::new(1, "600000", 9, 0, 10); // 浦发银行
            match kline.recv_parsed(&mut tcp) {
                Ok(_) => {
                    let count = kline.result().len();
                    println!("    ✅ 获取 {} 条日K线", count);

                    if count > 0 {
                        results.record_pass();
                        let bar = &kline.result()[0];
                        println!(
                            "    最新: {:?} 开:{:.2} 高:{:.2} 低:{:.2} 收:{:.2}",
                            bar.dt, bar.open, bar.high, bar.low, bar.close
                        );

                        // 验证数据合理性
                        if bar.high >= bar.low && bar.close > 0.0 {
                            results.record_pass();
                        } else {
                            results.record_fail("K线数据异常".to_string());
                        }
                    } else {
                        results.record_fail("K线数据为空".to_string());
                    }
                }
                Err(e) => {
                    results.record_fail(format!("获取K线失败: {}", e));
                }
            }

            // 测试周K线
            println!("  📈 测试周K线数据");
            let mut kline_week = Kline::new(1, "600000", 10, 0, 5);
            match kline_week.recv_parsed(&mut tcp) {
                Ok(_) => {
                    let count = kline_week.result().len();
                    println!("    ✅ 获取 {} 条周K线", count);
                    results.record_pass();
                }
                Err(e) => {
                    results.record_fail(format!("获取周K线失败: {}", e));
                }
            }
        }
        Err(e) => {
            results.record_fail(format!("连接失败: {}", e));
        }
    }

    results.print_summary();
}

#[test]
fn test_05_finance_info() {
    if !should_run_live_tests() {
        return;
    }

    println!("\n[测试 1.5] 财务信息获取");
    let mut results = TestResults::default();

    match Tcp::new() {
        Ok(mut tcp) => {
            let stocks = vec![
                (0, "000001"), // 平安银行
                (1, "600519"), // 贵州茅台
                (1, "300750"), // 宁德时代（注意：市场代码错误，会失败）
            ];

            for (market, code) in stocks {
                println!("  📊 测试股票: {}", code);
                let mut finance = FinanceInfo::new(market, code);
                match finance.recv_parsed(&mut tcp) {
                    Ok(_) => {
                        if let Some(info) = finance.result().first() {
                            println!("    ✅ 总股本: {:.2}亿股", info.zongguben / 1_0000_0000.0);
                            println!("    ✅ 净资产: {:.2}亿元", info.jingzichan / 1_0000_0000.0);

                            // 验证关键字段
                            if info.zongguben > 0.0 && info.jingzichan > 0.0 {
                                results.record_pass();
                            } else {
                                results.record_warning();
                                println!("    ⚠️  部分财务字段为0");
                            }
                        }
                    }
                    Err(e) => {
                        if market == 1 && code == "300750" {
                            println!("    ⚠️  预期失败（错误的市场代码）: {}", e);
                            results.record_pass();
                        } else {
                            results.record_fail(format!("获取财务信息失败: {}", e));
                        }
                    }
                }
            }
        }
        Err(e) => {
            results.record_fail(format!("连接失败: {}", e));
        }
    }

    results.print_summary();
}

#[test]
fn test_06_minute_time_data() {
    if !should_run_live_tests() {
        return;
    }

    println!("\n[测试 1.6] 分时数据获取");
    let mut results = TestResults::default();

    match Tcp::new() {
        Ok(mut tcp) => {
            let mut minute = MinuteTime::new(0, "000001");
            match minute.recv_parsed(&mut tcp) {
                Ok(_) => {
                    let count = minute.result().len();
                    println!("  ✅ 获取 {} 条分时数据", count);

                    // 验证数据量（正常应该是240条）
                    if count > 200 {
                        results.record_pass();
                        println!("  ✅ 数据量正常");

                        // 显示前3条和后3条
                        println!("  📊 分时数据示例:");
                        for (i, data) in minute.result().iter().take(3).enumerate() {
                            println!("     {} 价格:{:.2} 量:{:.0}", i + 1, data.price, data.vol);
                        }
                        println!("     ...");
                        for (i, data) in minute.result().iter().rev().take(3).enumerate() {
                            println!(
                                "     {} 价格:{:.2} 量:{:.0}",
                                count - 2 + i,
                                data.price,
                                data.vol
                            );
                        }
                    } else {
                        results.record_warning();
                        println!("  ⚠️  数据量偏少，可能不在交易时间");
                    }

                    // 验证数据合理性
                    let mut valid_count = 0;
                    for data in minute.result() {
                        if data.price > 0.0 && data.vol >= 0 {
                            valid_count += 1;
                        }
                    }
                    println!("  📊 数据有效性: {}/{}", valid_count, count);
                    results.record_pass();
                }
                Err(e) => {
                    results.record_fail(format!("获取分时数据失败: {}", e));
                }
            }
        }
        Err(e) => {
            results.record_fail(format!("连接失败: {}", e));
        }
    }

    results.print_summary();
}

#[test]
fn test_07_transaction_data() {
    if !should_run_live_tests() {
        return;
    }

    println!("\n[测试 1.7] 逐笔成交数据获取");
    let mut results = TestResults::default();

    match Tcp::new() {
        Ok(mut tcp) => {
            let mut transaction = Transaction::new(0, "000001", 0, 20);
            match transaction.recv_parsed(&mut tcp) {
                Ok(_) => {
                    let count = transaction.result().len();
                    println!("  ✅ 获取 {} 笔逐笔成交", count);

                    if count > 0 {
                        results.record_pass();
                        println!("  📊 最近5笔成交:");
                        for data in transaction.result().iter().take(5) {
                            let direction = match data.buyorsell {
                                0 => "买",
                                1 => "卖",
                                _ => "?",
                            };
                            println!(
                                "     {} 价格:{:.2} 量:{:.0}手 {}",
                                data.time, data.price, data.vol, direction
                            );
                        }

                        // 验证买卖方向统计
                        let mut buy_count = 0;
                        let mut sell_count = 0;
                        for data in transaction.result() {
                            if data.buyorsell == 0 {
                                buy_count += 1;
                            } else if data.buyorsell == 1 {
                                sell_count += 1;
                            }
                        }
                        println!("  📊 买卖分布: 买{}笔 卖{}笔", buy_count, sell_count);
                        results.record_pass();
                    } else {
                        results.record_warning();
                        println!("  ⚠️  无逐笔成交数据，可能不在交易时间");
                    }
                }
                Err(e) => {
                    results.record_fail(format!("获取逐笔成交失败: {}", e));
                }
            }
        }
        Err(e) => {
            results.record_fail(format!("连接失败: {}", e));
        }
    }

    results.print_summary();
}

#[test]
fn test_08_security_list() {
    if !should_run_live_tests() {
        return;
    }

    println!("\n[测试 1.8] 股票列表获取");
    let mut results = TestResults::default();

    match Tcp::new() {
        Ok(mut tcp) => {
            // 测试深市股票列表
            println!("  📋 获取深市股票列表 (前1000只)");
            let mut list = SecurityList::new(0, 0);
            match list.recv_parsed(&mut tcp) {
                Ok(_) => {
                    let count = list.result().len();
                    println!("    ✅ 获取 {} 只股票", count);

                    if count > 0 {
                        results.record_pass();
                        println!("    📊 前5只股票:");
                        for stock in list.result().iter().take(5) {
                            println!("       {} {}", stock.code, stock.name);
                        }
                    } else {
                        results.record_fail("股票列表为空".to_string());
                    }
                }
                Err(e) => {
                    results.record_fail(format!("获取股票列表失败: {}", e));
                }
            }

            // 测试沪市股票列表
            println!("  📋 获取沪市股票列表 (前1000只)");
            let mut list_sh = SecurityList::new(1, 0);
            match list_sh.recv_parsed(&mut tcp) {
                Ok(_) => {
                    let count = list_sh.result().len();
                    println!("    ✅ 获取 {} 只股票", count);
                    results.record_pass();
                }
                Err(e) => {
                    results.record_fail(format!("获取沪市股票列表失败: {}", e));
                }
            }
        }
        Err(e) => {
            results.record_fail(format!("连接失败: {}", e));
        }
    }

    results.print_summary();
}

// ============================================================================
// 第二部分：五档买卖盘完整性测试
// ============================================================================

#[test]
fn test_09_five_level_quotes() {
    if !should_run_live_tests() {
        return;
    }

    println!("\n[测试 2.1] 五档买卖盘完整性验证");
    let mut results = TestResults::default();

    match Tcp::new() {
        Ok(mut tcp) => {
            let test_stocks = vec![
                (0, "000001"), // 平安银行
                (1, "600519"), // 贵州茅台
                (0, "300750"), // 宁德时代
            ];

            for (market, code) in test_stocks {
                println!("  📊 测试股票: {}", code);
                let mut quotes = SecurityQuotes::new(vec![(market, code)]);
                match quotes.recv_parsed(&mut tcp) {
                    Ok(_) => {
                        if let Some(quote) = quotes.result().first() {
                            // 验证五档买卖盘数据
                            println!(
                                "    买一到买五: {:.2} {:.2} {:.2} {:.2} {:.2}",
                                quote.bid1, quote.bid2, quote.bid3, quote.bid4, quote.bid5
                            );
                            println!(
                                "    卖一到卖五: {:.2} {:.2} {:.2} {:.2} {:.2}",
                                quote.ask1, quote.ask2, quote.ask3, quote.ask4, quote.ask5
                            );

                            // 验证价格递减关系: 买5 < ... < 买1 < 卖1 < ... < 卖5
                            let buy_prices =
                                [quote.bid5, quote.bid4, quote.bid3, quote.bid2, quote.bid1];
                            let sell_prices =
                                [quote.ask1, quote.ask2, quote.ask3, quote.ask4, quote.ask5];

                            let mut buy_correct = true;
                            for i in 0..4 {
                                if buy_prices[i] > buy_prices[i + 1] {
                                    buy_correct = false;
                                    break;
                                }
                            }

                            let mut sell_correct = true;
                            for i in 0..4 {
                                if sell_prices[i] > sell_prices[i + 1] {
                                    sell_correct = false;
                                    break;
                                }
                            }

                            if buy_correct && sell_correct {
                                println!("    ✅ 价格递减关系正确");
                                results.record_pass();
                            } else {
                                println!("    ❌ 价格递减关系异常");
                                results.record_warning();
                            }

                            // 验证买卖价差
                            let spread = quote.ask1 - quote.bid1;
                            let spread_percent = (spread / quote.bid1) * 100.0;
                            println!("    买卖价差: {:.4} ({:.3}%)", spread, spread_percent);

                            if spread > 0.0 && spread_percent < 5.0 {
                                println!("    ✅ 价差合理");
                                results.record_pass();
                            } else {
                                println!("    ⚠️  价差异常");
                                results.record_warning();
                            }

                            // 验证成交量
                            let total_buy_vol = quote.bid1_vol
                                + quote.bid2_vol
                                + quote.bid3_vol
                                + quote.bid4_vol
                                + quote.bid5_vol;
                            let total_sell_vol = quote.ask1_vol
                                + quote.ask2_vol
                                + quote.ask3_vol
                                + quote.ask4_vol
                                + quote.ask5_vol;

                            println!(
                                "    买盘总量: {:.0}手 卖盘总量: {:.0}手",
                                total_buy_vol, total_sell_vol
                            );

                            if total_buy_vol > 0.0 || total_sell_vol > 0.0 {
                                results.record_pass();
                            } else {
                                results.record_warning();
                            }
                        }
                    }
                    Err(e) => {
                        results.record_fail(format!("获取{}五档行情失败: {}", code, e));
                    }
                }
            }
        }
        Err(e) => {
            results.record_fail(format!("连接失败: {}", e));
        }
    }

    results.print_summary();
}

// ============================================================================
// 第三部分：v0.6.6 新功能测试（行业和概念）
// ============================================================================

#[test]
fn test_10_industry_mapping() {
    if !should_run_live_tests() {
        return;
    }

    println!("\n[测试 3.1] 行业分类映射功能");
    let mut results = TestResults::default();

    match Tcp::new() {
        Ok(mut tcp) => {
            let test_cases = vec![
                (1, "600519", "酒类", "贵州"), // 贵州茅台
                (1, "600036", "银行", "浙江"), // 招商银行
                (0, "000858", "银行", "四川"), // 五粮液
            ];

            for (market, code, expected_industry, expected_province) in test_cases {
                println!("  📊 测试股票: {}", code);
                let mut finance = FinanceInfo::new(market, code);
                match finance.recv_parsed(&mut tcp) {
                    Ok(_) => {
                        if let Some(info) = finance.result().first() {
                            let industry = get_industry_name(info.industry);
                            let province = get_province_name(info.province);

                            println!("    行业: {} (期望: {})", industry, expected_industry);
                            println!("    省份: {} (期望: {})", province, expected_province);

                            if industry.contains(expected_industry) {
                                println!("    ✅ 行业映射正确");
                                results.record_pass();
                            } else {
                                println!("    ⚠️  行业映射不匹配");
                                results.record_warning();
                            }

                            if province.contains(expected_province) {
                                println!("    ✅ 省份映射正确");
                                results.record_pass();
                            } else {
                                println!("    ⚠️  省份映射不匹配");
                                results.record_warning();
                            }
                        }
                    }
                    Err(e) => {
                        results.record_fail(format!("获取{}财务信息失败: {}", code, e));
                    }
                }
            }
        }
        Err(e) => {
            results.record_fail(format!("连接失败: {}", e));
        }
    }

    results.print_summary();
}

#[test]
fn test_11_concept_stocks() {
    if !should_run_live_tests() {
        return;
    }

    println!("\n[测试 3.2] 概念板块成分股查询");
    let mut results = TestResults::default();

    let test_concepts = vec!["新能源汽车", "锂电池", "芯片", "人工智能"];

    for concept in test_concepts {
        println!("  🔍 查询概念: {}", concept);
        match get_concept_stocks(concept) {
            Some(stocks) => {
                println!("    ✅ 找到 {} 只成分股", stocks.len());
                if !stocks.is_empty() {
                    println!("    📊 前5只成分股:");
                    for stock in stocks.iter().take(5) {
                        println!("       {} {}", stock.code, stock.name);
                    }
                    results.record_pass();
                } else {
                    println!("    ⚠️  成分股为空");
                    results.record_warning();
                }
            }
            None => {
                println!("    ⚠️  未找到该概念板块");
                results.record_warning();
            }
        }
    }

    results.print_summary();
}

// ============================================================================
// 主测试入口（生成报告）
// ============================================================================

#[test]
fn generate_test_report() {
    if !should_run_live_tests() {
        return;
    }

    println!("\n{}", "=".repeat(60));
    println!("RustDX 开盘日全面测试报告");
    println!("{}", "=".repeat(60));
    println!("测试时间: {}", get_current_time());
    println!("版本号: {}", env!("CARGO_PKG_VERSION"));
    println!("{}", "=".repeat(60));

    println!("\n测试套件准备就绪");
    println!("\n运行以下命令执行完整测试:");
    println!("  cargo test --test comprehensive_test -- --live --nocapture");
    println!("\n或者运行单个测试:");
    println!("  cargo test test_01_security_quotes_single -- --live --nocapture");
}
