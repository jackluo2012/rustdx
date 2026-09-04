//! 交易日历示例
//!
//! 展示如何使用 rustdx 的中国A股交易日历功能
//!
//! 运行方式：
//! ```bash
//! cargo run --example test_trading_calendar
//! ```

use chrono::{Datelike, NaiveDate, NaiveDateTime};
use rustdx_complete::calendar::TradingCalendar;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📅 rustdx 中国A股交易日历示例\n");

    // ========================================
    // 示例 1: 判断是否是交易日
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 1: 判断是否是交易日");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let dates = vec![
        NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(), // 周三（元旦）
        NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(), // 周四
        NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(), // 周五
        NaiveDate::from_ymd_opt(2025, 1, 4).unwrap(), // 周六
        NaiveDate::from_ymd_opt(2025, 1, 5).unwrap(), // 周日
        NaiveDate::from_ymd_opt(2025, 1, 6).unwrap(), // 周一
    ];

    println!("2025年1月1日-1月6日交易日情况：\n");
    for date in &dates {
        let is_trading = TradingCalendar::is_trading_day(date);
        let weekday = format!("{:?}", date.weekday());
        let status = if is_trading {
            "✅ 是交易日"
        } else {
            "❌ 非交易日"
        };
        println!(
            "  {:04}-{:02}-{:02} ({}) {}",
            date.year(),
            date.month(),
            date.day(),
            weekday,
            status
        );
    }

    // ========================================
    // 示例 2: 判断是否是交易时间
    // ========================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 2: 判断是否是交易时间");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let times = vec![
        "2025-01-06 09:00:00", // 交易前
        "2025-01-06 10:00:00", // 交易中
        "2025-01-06 12:00:00", // 午休时间
        "2025-01-06 14:00:00", // 交易中
        "2025-01-06 16:00:00", // 交易后
        "2025-01-04 10:00:00", // 周六
    ];

    println!("A股交易时间：9:30-15:00\n");
    for time_str in &times {
        let dt = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S").unwrap();
        let is_trading_time = TradingCalendar::is_trading_time(&dt);
        let status = if is_trading_time {
            "✅ 交易中"
        } else {
            "❌ 非交易时间"
        };
        println!("  {} - {}", time_str, status);
    }

    // ========================================
    // 示例 3: 获取前一个交易日
    // ========================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 3: 获取前一个交易日");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let date = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap(); // 周一
    let prev = TradingCalendar::previous_trading_day(&date);
    println!(
        "{} 的前一个交易日: {:04}-{:02}-{:02}\n",
        date,
        prev.year(),
        prev.month(),
        prev.day()
    );

    // ========================================
    // 示例 4: 获取后一个交易日
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 4: 获取后一个交易日");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let date = NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(); // 周五
    let next = TradingCalendar::next_trading_day(&date);
    println!(
        "{} 的后一个交易日: {:04}-{:02}-{:02}\n",
        date,
        next.year(),
        next.month(),
        next.day()
    );

    // ========================================
    // 示例 5: 获取最近N个交易日
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 5: 获取最近N个交易日");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let start = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap();
    let trading_days = TradingCalendar::get_trading_days(&start, 10);

    println!("从 {} 开始的10个交易日：\n", start);
    for (i, date) in trading_days.iter().enumerate() {
        println!(
            "  {}. {:04}-{:02}-{:02} ({:?})",
            i + 1,
            date.year(),
            date.month(),
            date.day(),
            date.weekday()
        );
    }

    // ========================================
    // 示例 6: 计算两个日期之间的交易日数量
    // ========================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 6: 计算交易日数量");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
    let count = TradingCalendar::count_trading_days(&start, &end);

    println!("2025年1月交易日统计：");
    println!("  自然日: {} 天", end - start);
    println!("  交易日: {} 天\n", count);

    // ========================================
    // 示例 7: 获取当前交易日
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 7: 获取当前交易日");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let today = TradingCalendar::current_trading_day();
    println!(
        "当前交易日: {:04}-{:02}-{:02}\n",
        today.year(),
        today.month(),
        today.day()
    );

    // ========================================
    // 示例 8: 获取市场交易日
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 8: 获取市场交易日（考虑交易时间）");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let market_day = TradingCalendar::market_trading_day();
    println!(
        "市场交易日: {:04}-{:02}-{:02}",
        market_day.year(),
        market_day.month(),
        market_day.day()
    );
    println!("说明：如果今天在交易时间前，返回上一个交易日；否则返回今天",);

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ 交易日历示例完成！");

    println!("\n💡 使用建议：");
    println!("  • 判断交易日时，使用 `is_trading_day`");
    println!("  • 获取历史数据时，使用 `get_trading_days_before`");
    println!("  • 计算收益率时，使用 `count_trading_days` 获取实际交易日数");
    println!("  • 所有节假日和调休都自动处理，无需手动判断");
    println!("  • 数据定期更新，依赖 trade_date_a crate");

    Ok(())
}
