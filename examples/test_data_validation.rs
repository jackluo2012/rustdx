//! 数据验证示例
//!
//! 展示如何使用 rustdx 的数据完整性验证功能
//!
//! 运行方式：
//! ```bash
//! cargo run --example test_data_validation
//! ```

use rustdx_complete::tcp::stock::{FinanceInfo, Kline, validator::*};
use rustdx_complete::tcp::{Tcp, Tdx};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 rustdx 数据完整性验证示例\n");

    // 连接到通达信服务器
    println!("⚡ 连接到通达信服务器...");
    let mut tcp = Tcp::new()?;
    println!("✅ 连接成功\n");

    // ========================================
    // 示例 1: K线数据连续性验证
    // ========================================
    println!("📊 示例 1: K线数据连续性验证");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut kline = Kline::new(1, "600000", 9, 0, 100); // 浦发银行
    kline.recv_parsed(&mut tcp)?;

    println!("股票代码: 600000");
    println!("数据条数: {}\n", kline.result().len());

    let result = validate_kline_continuity(kline.result(), "600000");

    match &result.level {
        ValidationLevel::Ok => {
            println!("✅ {}\n", result.details[0]);
        }
        ValidationLevel::Warning(msg) => {
            println!("⚠️  {}\n", msg);
            for detail in &result.details {
                println!("  - {}", detail);
            }
            println!("\n💡 建议:");
            for suggestion in &result.suggestions {
                println!("  • {}", suggestion);
            }
        }
        ValidationLevel::Error(msg) => {
            println!("❌ {}\n", msg);
            for detail in &result.details {
                println!("  - {}", detail);
            }
        }
    }

    // ========================================
    // 示例 2: 财务数据一致性验证
    // ========================================
    println!("\n📊 示例 2: 财务数据一致性验证");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut finance = FinanceInfo::new(1, "600000");
    finance.recv_parsed(&mut tcp)?;

    let finance_info = &finance.result()[0];

    println!("股票代码: {}", finance_info.code);
    println!("总股本: {:.2} 亿股", finance_info.zongguben / 1_0000_0000.0);
    println!(
        "流通股: {:.2} 亿股",
        finance_info.liutongguben / 1_0000_0000.0
    );
    println!(
        "净资产: {:.2} 亿元",
        finance_info.jingzichan / 1_0000_0000.0
    );
    println!(
        "总资产: {:.2} 亿元\n",
        finance_info.zongzichan / 1_0000_0000.0
    );

    let result = validate_finance_consistency(finance_info);

    match &result.level {
        ValidationLevel::Ok => {
            println!("✅ {}\n", result.details[0]);
        }
        ValidationLevel::Warning(msg) => {
            println!("⚠️  {}\n", msg);
            for detail in &result.details {
                println!("  - {}", detail);
            }
        }
        ValidationLevel::Error(msg) => {
            println!("❌ {}\n", msg);
            for detail in &result.details {
                println!("  - {}", detail);
            }
            println!("\n💡 建议:");
            for suggestion in &result.suggestions {
                println!("  • {}", suggestion);
            }
        }
    }

    // ========================================
    // 示例 3: 异常值检测
    // ========================================
    println!("\n📊 示例 3: 异常值检测");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    println!("股票代码: 600000");
    println!("检测阈值: 3σ (标准差)\n");

    let result = detect_anomalies(kline.result(), 3.0);

    match &result.level {
        ValidationLevel::Ok => {
            println!("✅ {}\n", result.details[0]);
        }
        ValidationLevel::Warning(msg) => {
            println!("⚠️  {}\n", msg);
            println!("发现的问题:");
            for detail in result.details.iter().take(5) {
                println!("  • {}", detail);
            }
            if result.details.len() > 5 {
                println!("  ... 还有 {} 个问题", result.details.len() - 5);
            }
            println!("\n💡 可能的原因:");
            for suggestion in &result.suggestions {
                println!("  • {}", suggestion);
            }
        }
        ValidationLevel::Error(msg) => {
            println!("❌ {}\n", msg);
        }
    }

    // ========================================
    // 示例 4: 使用 Trait 进行单条数据验证
    // ========================================
    println!("\n📊 示例 4: 单条数据验证（Trait）");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if let Some(first_bar) = kline.result().first() {
        println!(
            "验证日期: {:04}-{:02}-{:02}",
            first_bar.dt.year, first_bar.dt.month, first_bar.dt.day
        );
        println!("开盘价: {:.2}", first_bar.open);
        println!("收盘价: {:.2}", first_bar.close);
        println!("最高价: {:.2}", first_bar.high);
        println!("最低价: {:.2}\n", first_bar.low);

        let result = first_bar.validate();

        if result.is_valid() {
            println!("✅ 单条数据验证通过");
        } else {
            println!("❌ 单条数据验证失败:");
            for detail in &result.details {
                println!("  - {}", detail);
            }
        }
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ 验证完成！");

    Ok(())
}
