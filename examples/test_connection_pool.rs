//! 连接池示例
//!
//! 展示如何使用 rustdx 的 TCP 连接池
//!
//! 运行方式：
//! ```bash
//! cargo run --example test_connection_pool
//! ```

use rustdx_complete::builder::KlineBuilder;
use rustdx_complete::pool::ConnectionPool;
use rustdx_complete::tcp::Tdx;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏊 rustdx TCP 连接池示例\n");

    // ========================================
    // 示例 1: 基础连接池使用
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 1: 基础连接池使用");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("创建连接池（最大3个连接）：\n");
    let pool = ConnectionPool::new(3)?;

    println!("let pool = ConnectionPool::new(3)?;\n");

    // 查看初始状态
    let stats = pool.stats();
    println!("✅ 连接池创建成功：");
    println!("   最大连接数: {}", stats.max_size);
    println!("   当前连接数: {}", stats.total);
    println!("   活跃连接: {}", stats.active);
    println!("   空闲连接: {}", stats.idle);

    // ========================================
    // 示例 2: 获取和归还连接
    // ========================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 2: 获取和归还连接");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("获取连接并查看状态：\n");

    // 获取第一个连接
    {
        println!("let mut conn1 = pool.get_connection()?;");
        let mut _conn1 = pool.get_connection()?;

        let stats = pool.stats();
        println!("\n获取第1个连接后：");
        println!("   总连接数: {}", stats.total);
        println!("   活跃连接: {}", stats.active);
        println!("   空闲连接: {}", stats.idle);

        // 连接在离开作用域时自动归还
        println!("\nconn1 离开作用域，连接自动归还...");
    }

    // 等待连接归还
    std::thread::sleep(std::time::Duration::from_millis(100));

    let stats = pool.stats();
    println!("\n连接归还后：");
    println!("   活跃连接: {}", stats.active);
    println!("   空闲连接: {}", stats.idle);

    // ========================================
    // 示例 3: 连接复用
    // ========================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 3: 连接复用");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("演示连接复用（不会创建新连接）：\n");

    // 第一次获取连接
    {
        let _conn1 = pool.get_connection()?;
        let stats = pool.stats();
        println!("第1次获取连接：总连接数 = {}", stats.total);

        // 连接归还
    }

    std::thread::sleep(std::time::Duration::from_millis(100));

    // 第二次获取连接（应该复用之前的连接）
    {
        let _conn2 = pool.get_connection()?;
        let stats = pool.stats();
        println!("第2次获取连接：总连接数 = {} (复用)", stats.total);
    }

    // ========================================
    // 示例 4: 连接池限制
    // ========================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 4: 连接池限制");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 创建新的连接池用于此示例
    let pool_limit = ConnectionPool::new(3)?;

    println!("演示连接池的最大连接数限制：\n");

    let _conn1 = pool_limit.get_connection()?;
    let _conn2 = pool_limit.get_connection()?;
    let _conn3 = pool_limit.get_connection()?;

    let stats = pool_limit.stats();
    println!("已获取 3 个连接（达到最大值）：");
    println!("   最大连接数: {}", stats.max_size);
    println!("   当前连接数: {}", stats.total);

    // 尝试获取第4个连接（应该失败）
    println!("\n尝试获取第4个连接...");
    match pool_limit.get_connection() {
        Ok(_) => println!("   ❌ 不应该成功"),
        Err(e) => println!("   ✅ 正确拒绝: {}", e),
    }

    // ========================================
    // 示例 5: 使用连接执行查询
    // ========================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 5: 使用连接池执行查询");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("⚡ 连接到通达信服务器...\n");

    // 使用原始连接池

    // 使用连接池获取数据
    let mut kline = KlineBuilder::daily("600000", 5)?;

    println!("📊 获取浦发银行(600000) 最近5天数据...\n");

    match pool.get_connection() {
        Ok(mut conn) => match kline.recv_parsed(conn.get_mut()) {
            Ok(_) => {
                println!("✅ 数据获取成功\n");

                let data = kline.result();
                println!("获取到 {} 条数据：\n", data.len());

                for (i, bar) in data.iter().enumerate() {
                    println!(
                        "  {}. {:04}-{:02}-{:02} | 收:{:.2} 量:{:.0}万",
                        i + 1,
                        bar.dt.year,
                        bar.dt.month,
                        bar.dt.day,
                        bar.close,
                        bar.vol / 10000.0
                    );
                }
            }
            Err(e) => {
                println!("❌ 数据获取失败: {}", e);
            }
        },
        Err(e) => {
            println!("❌ 获取连接失败: {}", e);
            println!("\n💡 提示：请确保网络连接正常，通达信服务器可访问");
            return Ok(());
        }
    }

    // ========================================
    // 示例 6: 并发查询（使用多个连接）
    // ========================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 6: 并发查询");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("使用连接池并发查询多只股票：\n");

    // 创建新的连接池用于并发示例
    let pool2 = ConnectionPool::new(3)?;

    println!("⚡ 并发查询3只股票的数据...\n");

    // 使用 execute 方法执行查询
    let codes = vec!["600000", "000001", "300001"];

    for code in codes {
        let mut kline = match KlineBuilder::daily(code, 1) {
            Ok(k) => k,
            Err(e) => {
                println!("❌ 构建{}查询失败: {}", code, e);
                continue;
            }
        };

        match pool2.get_connection() {
            Ok(mut conn) => match kline.recv_parsed(conn.get_mut()) {
                Ok(_) => {
                    let data = kline.result();
                    if !data.is_empty() {
                        let bar = &data[0];
                        println!(
                            "✅ {} | {:04}-{:02}-{:02} | 收:{:.2}",
                            code, bar.dt.year, bar.dt.month, bar.dt.day, bar.close
                        );
                    }
                }
                Err(_) => {
                    println!("⚠️  {} 数据获取失败", code);
                }
            },
            Err(_) => {
                println!("⚠️  {} 获取连接失败（池已满）", code);
            }
        }
    }

    // ========================================
    // 示例 7: 连接池统计
    // ========================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 7: 连接池统计");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let stats = pool2.stats();
    println!("连接池状态统计：\n");
    println!("   总连接数: {} / {}", stats.total, stats.max_size);
    println!("   活跃连接: {}", stats.active);
    println!("   空闲连接: {}", stats.idle);
    println!(
        "   利用率: {:.1}%",
        (stats.total as f64 / stats.max_size as f64) * 100.0
    );

    // ========================================
    // 示例 8: 自定义配置
    // ========================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 8: 自定义连接池配置");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("使用自定义配置创建连接池：\n");
    println!("use rustdx_complete::pool::{{ConnectionPool, PoolConfig}};");
    println!("\nlet config = PoolConfig {{");
    println!("    max_size: 5,");
    println!("    max_idle: 300,        // 5分钟空闲时间");
    println!("    max_lifetime: 1800,   // 30分钟最大生命周期");
    println!("    ..Default::default()");
    println!("}};");
    println!("\nlet pool = ConnectionPool::with_config(config)?;\n");

    // 关闭所有连接池
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ 连接池示例完成！");

    println!("\n💡 使用建议：");
    println!("  • 连接池减少TCP握手开销，提升性能");
    println!("  • 设置合适的max_size（通常3-5个）");
    println!("  • 连接自动归还，无需手动管理");
    println!("  • 适合批量查询或高频场景");
    println!("  • 使用execute()方法简化代码");
    println!("  • 监控stats()了解连接池状态");

    // 清理
    pool.close();
    pool2.close();

    Ok(())
}
