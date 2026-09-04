//! 缓存功能示例
//!
//! 展示如何使用 rustdx 的智能缓存层
//!
//! 运行方式：
//! ```bash
//! cargo run --example test_cache
//! ```

use rustdx_complete::cache::Cache;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("💾 rustdx 智能缓存层示例\n");

    // ========================================
    // 示例 1: 基础内存缓存
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 1: 基础内存缓存");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 创建内存缓存（5分钟TTL）
    let cache = Cache::memory(Duration::from_secs(300));

    // 手动设置和获取
    println!("\n手动缓存模式:");
    cache.set("stock:600000", "浦发银行数据".as_bytes());

    match cache.get("stock:600000") {
        Some(data) => {
            println!("✅ 缓存命中: {} 字节", data.len());
        }
        None => {
            println!("❌ 缓存未命中");
        }
    }

    // ========================================
    // 示例 2: get_or_fetch 模式（推荐）
    // ========================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 2: get_or_fetch 模式（推荐）");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 使用 get_or_fetch 模式
    let cache = Cache::memory(Duration::from_secs(300));
    let mut fetch_count = 0;

    // 第一次调用：缓存未命中
    println!("\n第一次获取（缓存未命中）:");
    let data1: Result<Vec<u8>, Box<dyn std::error::Error>> =
        cache.get_or_fetch("kline:1:600000:9", || {
            fetch_count += 1;
            println!("  → 从服务器获取数据...");
            Ok(vec![1, 2, 3, 4, 5])
        });

    println!("  结果: {:?}", data1);
    println!("  总获取次数: {}", fetch_count);

    // 第二次调用：缓存命中
    println!("\n第二次获取（缓存命中）:");
    let data2: Result<Vec<u8>, Box<dyn std::error::Error>> =
        cache.get_or_fetch("kline:1:600000:9", || {
            fetch_count += 1;
            println!("  → 从服务器获取数据...");
            Ok(vec![1, 2, 3, 4, 5])
        });

    println!("  结果: {:?}", data2);
    println!("  总获取次数: {} (未增加，因为缓存命中)", fetch_count);

    // ========================================
    // 示例 3: 缓存过期
    // ========================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 3: 缓存过期");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let cache = Cache::memory(Duration::from_millis(50)); // 50ms TTL

    cache.set("temp_key", "临时数据".as_bytes());

    println!("设置缓存（50ms TTL）");

    // 立即获取
    match cache.get("temp_key") {
        Some(_) => println!("✅ 缓存命中（未过期）"),
        None => println!("❌ 缓存未命中"),
    }

    // 等待过期
    println!("等待 60ms...");
    std::thread::sleep(Duration::from_millis(60));

    // 过期后获取
    match cache.get("temp_key") {
        Some(_) => println!("✅ 缓存命中"),
        None => println!("❌ 缓存未命中（已过期）"),
    }

    // ========================================
    // 示例 4: 缓存统计
    // ========================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 4: 缓存统计");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let cache = Cache::memory(Duration::from_secs(300));

    cache.set("key1", b"data1");
    cache.set("key2", b"data2");
    cache.set("key3", b"data3");

    println!("\n缓存键数量: {}", cache.backend().len());
    println!("TTL 设置: {:?}", cache.ttl());

    // 清空缓存
    cache.clear();
    println!("清空后数量: {}", cache.backend().len());

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ 缓存示例完成！");

    println!("\n💡 使用建议：");
    println!("  • 实时行情数据：推荐 3-5 分钟 TTL");
    println!("  • 日线数据：推荐 1 小时 TTL");
    println!("  • 财务数据：推荐 1 天 TTL");
    println!("  • 使用 get_or_fetch 模式简化代码");
    println!("  • 开发环境使用 MemoryCache，生产环境使用 FileCache");

    Ok(())
}
