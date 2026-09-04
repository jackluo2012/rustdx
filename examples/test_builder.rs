//! Builder 模式示例
//!
//! 展示如何使用 rustdx 的 Builder 模式 API
//!
//! 运行方式：
//! ```bash
//! cargo run --example test_builder
//! ```

use rustdx_complete::builder::KlineBuilder;
use rustdx_complete::tcp::{Tcp, Tdx};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔨 rustdx Builder 模式示例\n");

    // ========================================
    // 示例 1: 基础 Builder 模式
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 1: 基础 Builder 模式");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("使用 Builder 模式构建K线查询：\n");
    println!("let kline = KlineBuilder::new()");
    println!("    .code(\"600000\")");
    println!("    .category(9)     // 日线");
    println!("    .count(100)      // 100条");
    println!("    .build()?;\n");

    let kline1 = KlineBuilder::new()
        .code("600000")
        .category(9)
        .count(100)
        .build()?;

    println!("✅ 构建成功：");
    println!("   股票代码: {}", kline1.code);
    println!(
        "   市场: {} ({}{})",
        kline1.market,
        if kline1.market == 1 {
            "上海"
        } else {
            "深圳"
        },
        if kline1.code.starts_with('6') {
            "主板"
        } else {
            ""
        }
    );
    println!(
        "   K线类型: {} ({})",
        kline1.category,
        match kline1.category {
            0 => "5分钟",
            1 => "15分钟",
            2 => "30分钟",
            3 => "1小时",
            9 => "日线",
            _ => "其他",
        }
    );
    println!("   起始位置: {}", kline1.start);
    println!("   数量: {}", kline1.count);

    // ========================================
    // 示例 2: 使用默认值
    // ========================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 2: 使用默认值");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("只设置股票代码，其他参数使用默认值：\n");
    println!("let kline = KlineBuilder::new()");
    println!("    .code(\"000001\")");
    println!("    .build()?;\n");

    let kline2 = KlineBuilder::new().code("000001").build()?;

    println!("✅ 构建成功（自动使用默认值）：");
    println!("   股票代码: {}", kline2.code);
    println!(
        "   市场: {} (自动识别: {})",
        kline2.market,
        if kline2.market == 1 {
            "上海"
        } else {
            "深圳"
        }
    );
    println!("   K线类型: {} (默认: 日线)", kline2.category);
    println!("   起始位置: {} (默认: 0)", kline2.start);
    println!("   数量: {} (默认: 100)", kline2.count);

    // ========================================
    // 示例 3: 便捷方法
    // ========================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 3: 便捷方法");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("使用便捷方法快速构建不同周期的K线：\n");

    // 日线
    let daily = KlineBuilder::daily("600000", 200)?;
    println!("KlineBuilder::daily(\"600000\", 200)");
    println!("  → {} 日线，{} 条\n", daily.code, daily.count);

    // 60分钟
    let min60 = KlineBuilder::min60("600000", 100)?;
    println!("KlineBuilder::min60(\"600000\", 100)");
    println!("  → {} 60分钟线，{} 条\n", min60.code, min60.count);

    // 15分钟
    let min15 = KlineBuilder::min15("600000", 100)?;
    println!("KlineBuilder::min15(\"600000\", 100)");
    println!("  → {} 15分钟线，{} 条\n", min15.code, min15.count);

    // 5分钟
    let min5 = KlineBuilder::min5("600000", 100)?;
    println!("KlineBuilder::min5(\"600000\", 100)");
    println!("  → {} 5分钟线，{} 条\n", min5.code, min5.count);

    // 周线
    let weekly = KlineBuilder::weekly("600000", 50)?;
    println!("KlineBuilder::weekly(\"600000\", 50)");
    println!("  → {} 周线，{} 条\n", weekly.code, weekly.count);

    // ========================================
    // 示例 4: 链式调用
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 4: 流畅的链式调用");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("Builder 模式支持流畅的链式调用：\n");

    let kline4 = KlineBuilder::new()
        .code("300001") // 创业板
        .market(0) // 深圳
        .category(9) // 日线
        .start(10) // 从第10条开始
        .count(50) // 获取50条
        .build()?;

    println!("✅ 构建成功：");
    println!("   股票代码: {}", kline4.code);
    println!("   市场: {}", kline4.market);
    println!("   K线类型: {}", kline4.category);
    println!("   起始: {}, 数量: {}", kline4.start, kline4.count);

    // ========================================
    // 示例 5: 错误处理
    // ========================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 5: 编译期类型安全与运行时验证");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("Builder 模式提供参数验证：\n");

    // 缺少必填参数
    println!("1. 缺少股票代码：");
    match KlineBuilder::new().build() {
        Ok(_) => println!("   ❌ 不应该成功"),
        Err(e) => println!("   ✅ 正确捕获错误: {}", e),
    }

    // 参数超出范围
    println!("\n2. count 超过限制（>800）：");
    match KlineBuilder::new().code("600000").count(801).build() {
        Ok(_) => println!("   ❌ 不应该成功"),
        Err(e) => println!("   ✅ 正确捕获错误: {}", e),
    }

    // ========================================
    // 示例 6: 结合 TCP 连接使用
    // ========================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 6: 结合 TCP 连接获取真实数据");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("连接到通达信服务器并获取数据：\n");

    println!("⚡ 连接到通达信服务器...");
    let mut tcp = match Tcp::new() {
        Ok(t) => {
            println!("✅ 连接成功\n");
            t
        }
        Err(e) => {
            println!("❌ 连接失败: {}\n", e);
            println!("跳过数据获取示例");
            println!("\n💡 提示：请确保网络连接正常，通达信服务器可访问");
            return Ok(());
        }
    };

    // 使用 Builder 构建 K线查询
    let mut kline = KlineBuilder::daily("600000", 10)?;

    println!("📊 获取浦发银行(600000) 最近10天日线数据...");
    match kline.recv_parsed(&mut tcp) {
        Ok(_) => {
            println!("✅ 数据获取成功\n");

            let data = kline.result();
            println!("共获取 {} 条数据：\n", data.len());

            // 显示最近5条
            for (i, bar) in data.iter().rev().take(5).enumerate() {
                println!(
                    "  {}. {:04}-{:02}-{:02} | 开:{:.2} 高:{:.2} 低:{:.2} 收:{:.2} 量:{:.0}",
                    i + 1,
                    bar.dt.year,
                    bar.dt.month,
                    bar.dt.day,
                    bar.open,
                    bar.high,
                    bar.low,
                    bar.close,
                    bar.vol / 10000.0 // 转换为万手
                );
            }
        }
        Err(e) => {
            println!("❌ 数据获取失败: {}", e);
        }
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Builder 模式示例完成！");

    println!("\n💡 使用建议：");
    println!("  • 使用 Builder 模式可以更清晰地表达查询意图");
    println!("  • 链式调用使代码更简洁、更易读");
    println!("  • 自动市场识别，减少手动设置");
    println!("  • 编译期类型安全 + 运行时参数验证");
    println!("  • 便捷方法适合常用场景（日线、分时等）");

    Ok(())
}
