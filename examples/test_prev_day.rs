
fn main() {
    let dates = vec![
        20250103i64, // 周五
        20250106i64, // 周一  
        20250107i64, // 周二
    ];
    
    for date in dates {
        println!("{}:", date);
        println!("  is_work_day: {}", trade_date_a::is_work_day(date));
        println!("  prev_trading_day: {}", trade_date_a::get_trade_day_offset(date, 1));
    }
}
