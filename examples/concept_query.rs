#!/usr/bin/env rustx
/**
概念股查询示例
*/
use rustdx_complete::tcp::stock::get_concept_stocks;

fn main() {
    println!("🚀 概念股查询示例\n");

    // 查询新能源汽车概念的成分股
    if let Some(stocks) = get_concept_stocks("新能源汽车") {
        println!("📗 新能源汽车概念成分股（前20只）:");
        println!("   {:<10} {:<12}", "代码", "名称");
        println!("   {}", "-".repeat(30));

        for stock in stocks.iter().take(20) {
            println!("   {:<10} {:<12}", stock.code, stock.name);
        }
    }

    println!("\n✅ 查询完成！");
}
