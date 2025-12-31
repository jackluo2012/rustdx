#!/usr/bin/env rustx
/**
综合示例：结合通达信行业和东方财富概念板块信息

这个示例展示了如何：
1. 通过通达信获取股票的行业和省份信息
2. 通过概念板块查询股票所属的热门概念
3. 结合两种信息进行股票分析
*/
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::{FinanceInfo, get_industry_name, get_province_name, get_concept_stocks};

fn main() {
    println!("🚀 综合示例：通达信行业 + 东方财富概念板块\n");

    // 1. 获取股票的通达信行业信息
    println!("1️⃣  通过通达信获取股票行业信息:\n");
    println!("   {:<12} {:<10} {:<12} {:<10}", "股票", "名称", "行业", "省份");
    println!("   {}", "-".repeat(50));

    let test_stocks = vec![
        (0, "000001", "平安银行"),
        (1, "600519", "贵州茅台"),
        (0, "300750", "宁德时代"),
    ];

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

    for (market, code, name) in test_stocks {
        let mut finance = FinanceInfo::new(market, code);
        match finance.recv_parsed(&mut tcp) {
            Ok(_) => {
                if finance.result().len() > 0 {
                    let info = &finance.result()[0];
                    let industry = get_industry_name(info.industry);
                    let province = get_province_name(info.province);
                    println!("   {:<12} {:<10} {:<12} {:<10}", code, name, industry, province);
                }
            }
            Err(_) => {}
        }
    }

    // 2. 查询热门概念的成分股
    println!("\n2️⃣  通过东方财富查询热门概念板块:\n");

    let hot_concepts = vec!["新能源汽车", "锂电池", "芯片", "人工智能"];

    for concept in hot_concepts {
        if let Some(stocks) = get_concept_stocks(concept) {
            println!("   📗 {} 概念 (显示前5只):", concept);
            println!("   {:<12} {:<12}", "代码", "名称");
            println!("   {}", "-".repeat(30));

            for stock in stocks.iter().take(5) {
                println!("   {:<12} {:<12}", stock.code, stock.name);
            }
            println!();
        }
    }

    // 3. 综合使用场景
    println!("3️⃣  综合使用场景：\n");
    println!("   💡 典型应用场景:");
    println!("      - 筛选某个行业的龙头股（如：银行业）");
    println!("      - 查找某个热门概念的成分股（如：新能源汽车）");
    println!("      - 结合行业和概念进行板块轮动分析");
    println!("      - 按省份筛选本地股票");

    println!("\n4️⃣  数据来源说明:\n");
    println!("   📊 通达信数据:");
    println!("      - 来源: rustdx (通达信行情接口)");
    println!("      - 内容: 实时行情、财务信息、行业分类");
    println!("      - 特点: 实时、准确、官方标准");

    println!("\n   📈 东方财富概念板块:");
    println!("      - 来源: AKShare (东方财富)");
    println!("      - 内容: 热门概念、题材板块");
    println!("      - 特点: 市场热点、主题投资");

    println!("\n✅ 综合示例完成！\n");

    println!("💡 使用建议:");
    println!("   1. 通达信行业分类用于基本面分析");
    println!("   2. 东方财富概念板块用于市场热点追踪");
    println!("   3. 两者结合可以更全面地分析股票特征");
}
