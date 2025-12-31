#!/usr/bin/env rustx
/**
测试获取股票行业和省份信息
*/
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::{FinanceInfo, get_industry_name, get_province_name, get_industry_info};

fn main() {
    println!("🚀 测试股票行业和省份信息获取功能\n");

    // 创建TCP连接
    println!("1️⃣  连接到通达信服务器...");
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

    // 测试多个不同行业的股票
    let test_stocks = vec![
        (0, "000001", "平安银行"),  // 银行
        (0, "000002", "万科A"),     // 房地产
        (1, "600000", "浦发银行"),  // 银行
        (1, "600036", "招商银行"),  // 银行
        (0, "000858", "五粮液"),    // 食品饮料
        (1, "600519", "贵州茅台"),  // 食品饮料
        (0, "300750", "宁德时代"),  // 锂电池
        (1, "601318", "中国平安"),  // 保险
    ];

    println!("2️⃣  获取测试股票的行业和省份信息:\n");
    println!("   {:<10} {:<12} {:<12} {:<12} {:<10}", "股票", "名称", "行业", "省份", "行业大类");
    println!("   {}", "-".repeat(70));

    for (market, code, name) in test_stocks {
        let mut finance = FinanceInfo::new(market, code);
        match finance.recv_parsed(&mut tcp) {
            Ok(_) => {
                if finance.result().len() > 0 {
                    let info = &finance.result()[0];
                    let industry_name = get_industry_name(info.industry);
                    let province_name = get_province_name(info.province);
                    let (_, _, category) = get_industry_info(info.industry);
                    println!(
                        "   {:<10} {:<12} {:<12} {:<12} {:<10}",
                        code, name, industry_name, province_name, category
                    );
                } else {
                    println!("   {:<10} {:<12} ❌ 无数据", code, name);
                }
            }
            Err(e) => {
                println!("   {:<10} {:<12} ❌ 获取失败: {}", code, name, e);
            }
        }
    }

    println!("\n3️⃣  使用示例:");
    println!("   ```rust");
    println!("   // 示例代码");
    println!("   let industry = get_industry_name(info.industry);");
    println!("   let province = get_province_name(info.province);");
    println!("   ```");

    println!("\n✅ 测试完成！");
    println!("\n💡 提示:");
    println!("   - 行业代码已自动映射到行业名称，无需手动处理");
    println!("   - 省份代码也已自动映射到省份名称");
    println!("   - 支持获取行业大类信息（金融、消费、科技等）");
    println!("   - 映射表位于 `src/tcp/stock/industry_mapping.rs`");
}
