use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::ip::STOCK_IP;
use rustdx_complete::tcp::stock::SecurityList;
use std::io::Result;

fn main() -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("         TCP连接测试程序");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("📡 正在连接到通达信服务器...");
    println!("   超时设置: 5秒");
    println!("   服务器: {}", STOCK_IP[0]);
    
    let mut tcp = Tcp::new()?;
    println!("✅ 连接成功！\n");
    
    println!("📊 正在获取证券列表...");
    let mut list = SecurityList::new(0, 0); // 深市，从0开始
    let result = list.recv_parsed(&mut tcp);

    match result {
        Ok(data) => {
            println!("✅ 获取成功！\n");
            println!("返回数据统计:");
            println!("  - 总条数: {}", data.len());
            
            if !data.is_empty() {
                println!("\n前5条证券信息：");
                for (i, item) in data.iter().take(5).enumerate() {
                    println!("  {}. 代码: {}, 名称: {}", i+1, item.code, item.name);
                }
            }
        }
        Err(e) => {
            println!("❌ 获取失败: {}", e);
            return Err(e);
        }
    }
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("         测试完成");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    Ok(())
}
