use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::SecurityQuotes;

fn main() {
    println!("🔍 调试：查看服务器返回的原始数据\n");

    // 创建TCP连接
    println!("1️⃣  连接到服务器...");
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

    // 获取服务器的引用
    let (stream, _buffer, recv) = tcp.get_ref();
    println!("📊 连接信息:");
    println!("   对端地址: {:?}", stream.peer_addr());
    println!("   初始接收缓冲区[16B]: {:02X?}\n", recv);

    // 尝试获取股票行情
    println!("2️⃣  发送股票行情请求...");
    let mut quotes = SecurityQuotes::new(vec![(0, "000001")]);

    // 查看发送的数据
    println!("   发送数据: {:02X?}", quotes.send());
    println!("   发送长度: {} 字节\n", quotes.send().len());

    // 手动发送并接收
    match tcp.send_recv(quotes.send()) {
        Ok((sent, recv_len)) => {
            println!("   发送 {} 字节，接收 {} 字节", sent, recv_len);
            let recv_data = tcp.get_ref_recv();
            println!("   接收缓冲区[16B]: {:02X?}", recv_data);

            // 解析响应头
            if recv_len >= 16 {
                println!("\n   📋 响应头解析:");
                println!("      字节0-11 (标识): {:02X?}", &recv_data[0..12]);
                let deflate_size = u16::from_le_bytes([recv_data[12], recv_data[13]]);
                let inflate_size = u16::from_le_bytes([recv_data[14], recv_data[15]]);
                println!("      字节12-13 (压缩后长度): {} (0x{:04X})", deflate_size, deflate_size);
                println!("      字节14-15 (解压后长度): {} (0x{:04X})", inflate_size, inflate_size);

                if deflate_size == 0 {
                    println!("\n   ⚠️  服务器返回空数据（压缩长度为0）");
                    println!("   可能原因:");
                    println!("      1. 当前不是交易时间");
                    println!("      2. 服务器不支持该请求");
                    println!("      3. 网络问题导致数据不完整");
                }
            }
        }
        Err(e) => {
            println!("   ❌ 发送/接收失败: {}", e);
        }
    }

    println!("\n3️⃣  测试其他服务器...");
    use rustdx_complete::tcp::ip::STOCK_IP;

    for (i, ip) in STOCK_IP.iter().take(5).enumerate() {
        println!("\n   尝试服务器 #{}: {}...", i + 1, ip);
        match Tcp::new_with_ip(ip) {
            Ok(mut tcp2) => {
                println!("      ✅ 连接成功");

                match tcp2.send_recv(quotes.send()) {
                    Ok((sent, recv_len)) => {
                        let recv_data = tcp2.get_ref_recv();
                        if recv_len >= 16 {
                            let deflate_size = u16::from_le_bytes([recv_data[12], recv_data[13]]);
                            let inflate_size = u16::from_le_bytes([recv_data[14], recv_data[15]]);
                            println!("      响应: 压缩={}B, 解压={}B", deflate_size, inflate_size);

                            if deflate_size > 0 {
                                println!("      ✅✅✅ 这个服务器返回了数据！");
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        println!("      ❌ 请求失败: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("      ❌ 连接失败: {}", e);
            }
        }
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("调试完成");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}
