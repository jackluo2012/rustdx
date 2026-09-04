# rustdx-complete 网络连接说明

## ⚠️ 重要澄清

### rustdx-complete 不需要本地通达信服务器！

**常见误解**：
- ❌ 错误：需要在本地运行通达信服务（端口 2222）
- ❌ 错误：需要配置本地 TCP 服务
- ✅ 正确：rustdx-connect 直接连接**通达信公共服务器**

---

## 🌐 工作原理

### 连接流程

```
你的应用
    ↓
rustdx-complete (TCP 客户端)
    ↓
通达信公共服务器 (115.238.56.198:7709)
    ↓
返回股票数据
```

### 服务器列表

rustdx-complete 内置了 19 个通达信公共服务器（位于 `src/tcp/ip.rs`）：

| 服务器地址 | 端口 | 状态 |
|-----------|------|------|
| 115.238.56.198 | 7709 | ✅ 推荐（默认） |
| 114.80.149.19 | 7709 | ✅ 可用 |
| 114.80.149.22 | 7709 | ✅ 可用 |
| 39.100.68.59 | 7709 | ⚠️ 可能不返回数据 |
| ... | 7709 | - |

**完整列表**: 见 `src/tcp/ip.rs` 中的 `STOCK_IP` 数组

---

## 🚀 正确使用方式

### 基础使用

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::SecurityQuotes;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建连接（自动连接到远程服务器）
    let mut tcp = Tcp::new()?;  // 连接到 115.238.56.198:7709

    // 2. 查询股票数据
    let mut quotes = SecurityQuotes::new(vec![
        (0, "000001"),  // 平安银行
        (1, "600000"),  // 浦发银行
    ]);

    quotes.recv_parsed(&mut tcp)?;

    // 3. 打印结果
    for quote in quotes.result() {
        println!("{}: {}", quote.code, quote.price);
    }

    Ok(())
}
```

### 使用备用服务器

如果默认服务器不可用，可以指定其他服务器：

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use std::net::SocketAddr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 方法1: 使用内置服务器的索引
    let mut tcp = Tcp::new_with_ip(&rustdx_complete::tcp::ip::STOCK_IP[1])?;

    // 方法2: 自定义服务器地址
    let addr: SocketAddr = "114.80.149.19:7709".parse()?;
    let mut tcp = Tcp::new_with_ip(&addr)?;

    Ok(())
}
```

---

## 🔧 故障排查

### 问题 1: 连接超时

**现象**：
```
Error: TimedOut
```

**原因**：
- 网络连接问题
- 防火墙阻止
- 服务器暂时不可用

**解决方案**：
```rust
use rustdx_complete::tcp::ip::STOCK_IP;

// 尝试多个服务器
for (i, addr) in STOCK_IP.iter().enumerate().take(5) {
    println!("尝试服务器 #{}: {}", i + 1, addr);
    match Tcp::new_with_ip(addr) {
        Ok(mut tcp) => {
            println!("✅ 连接成功！");
            // 使用这个连接
            return Ok(());
        }
        Err(e) => {
            println!("❌ 失败: {}", e);
        }
    }
}
```

### 问题 2: 无法解析域名

**现象**：
```
Error: TryAddrError
```

**解决方案**：
- 使用 IP 地址而不是域名
- 检查 DNS 设置
- 尝试其他服务器

### 问题 3: 返回空数据

**现象**：连接成功，但数据为空

**原因**：
- 服务器可能临时不返回数据（如 39.100.68.59）
- 非交易时间

**解决方案**：
```rust
// 使用已知可用的服务器
let addr: SocketAddr = "115.238.56.198:7709".parse()?;
let mut tcp = Tcp::new_with_ip(&addr)?;
```

---

## 📊 网络要求

### 必需条件

✅ **互联网连接**（连接到通达信服务器）
✅ **TCP 协议**（端口 7709）
✅ **防火墙允许出站连接**

### 不需要

❌ 本地通达信软件
❌ 本地 TCP 服务器
❌ 端口 2222 或其他本地端口
❌ VPN（除非你的网络限制访问）

---

## 🧪 测试连接

### 方法 1: 使用 cargo test

```bash
# 测试所有内置服务器
cargo test check_all_stock_ips -- --nocapture
```

输出示例：
```
✅ 检测到 15 个可用服务器 (总共 19 个):
  - 115.238.56.198:7709
  - 114.80.149.19:7709
  - 114.80.149.22:7709
  ...
```

### 方法 2: 使用 netcat

```bash
# 测试连接到通达信服务器（不是本地！）
nc -zv 115.238.56.198 7709

# 成功输出：
# Connection to 115.238.56.198 7709 port [tcp/*] succeeded!
```

### 方法 3: 使用示例程序

```bash
cargo run --example test_security_quotes
```

---

## 🔐 安全说明

### 数据来源

- **服务器**: 通达信公共服务器
- **协议**: 自定义 TCP 协议（与 pytdx 相同）
- **数据**: 公开的股票行情数据
- **安全**: 无需认证，只读数据

### 隐私

- ❌ 不收集个人信息
- ❌ 不上传数据
- ✅ 仅下载公开行情数据

---

## 📝 常见问题 (FAQ)

### Q: 为什么我看到"端口 2222"的错误？

**A**: 这是一个误解。rustdx-complete 不使用端口 2222。我们连接的是远程服务器的 7709 端口。

### Q: 我需要安装通达信软件吗？

**A**: 不需要。rustdx-complete 是独立的 Rust 库。

### Q: 可以在服务器环境使用吗？

**A**: 可以！只要有互联网连接即可。

### Q: 为什么有多个服务器？

**A**:
- 负载均衡
- 高可用性
- 地理分布（不同地区连接速度不同）

### Q: 如何选择最快的服务器？

**A**:
```rust
use rustdx_complete::tcp::ip::STOCK_IP;

let mut fastest = None;
let mut min_duration = std::time::Duration::from_secs(999);

for addr in STOCK_IP.iter().take(5) {
    let start = std::time::Instant::now();
    if let Ok(_) = Tcp::new_with_ip(addr) {
        let duration = start.elapsed();
        if duration < min_duration {
            min_duration = duration;
            fastest = Some(*addr);
        }
    }
}

if let Some(addr) = fastest {
    println!("最快的服务器: {} ({:?})", addr, min_duration);
}
```

---

## 🆘 获取帮助

如果仍然遇到连接问题：

1. **检查网络**: `ping 115.238.56.198`
2. **检查防火墙**: 确保允许 TCP 出站连接
3. **查看日志**: 设置 `RUST_LOG=debug` 环境变量
4. **提交 Issue**: [https://github.com/jackluo2012/rustdx/issues](https://github.com/jackluo2012/rustdx/issues)

---

## 📚 相关资源

- [项目 README](../README.md)
- [pytdx (Python版本)](https://pypi.org/project/pytdx/)
- [通达信官网](http://www.tdx.com.cn/)

---

**最后更新**: 2026-01-05
**版本**: v0.6.6
