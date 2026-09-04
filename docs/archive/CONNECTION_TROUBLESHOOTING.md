# 连接问题快速排查

## ⚠️ 常见误解

**错误理解**:
```
❌ "需要在本地运行通达信服务（端口 2222）"
❌ "rustdx-complete 依赖本地 TCP 服务"
```

**正确理解**:
```
✅ rustdx-complete 连接远程通达信服务器
✅ 默认服务器: 115.238.56.198:7709
✅ 不需要本地任何服务
```

---

## 🔍 快速诊断

### 1分钟快速检查

```bash
# 检查网络连接
ping 115.238.56.198

# 测试 TCP 连接
nc -zv 115.238.56.198 7709

# 运行自动诊断
./scripts/diagnose_network.sh
```

---

## 💡 常见错误和解决方案

### 错误 1: "Connection refused"

**完整错误信息**:
```
Error: Connection refused (os error 111)
或
通达信服务未运行 - nc -zv 127.0.0.1 2222 返回 "Connection refused"
```

**原因**: 你在测试本地端口 2222，但 rustdx-connect 不使用本地端口！

**解决方案**:
```rust
// ❌ 错误：不需要本地服务
// let tcp = Tcp::connect_to_local_2222()?;

// ✅ 正确：直接连接远程服务器
let mut tcp = Tcp::new()?;  // 自动连接到 115.238.56.198:7709
```

---

### 错误 2: "TimedOut"

**完整错误信息**:
```
Error: TimedOut
或
Error: Connection timed out
```

**原因**: 网络问题或防火墙阻止

**解决方案**:
```rust
use rustdx_complete::tcp::ip::STOCK_IP;

// 尝试多个服务器
for addr in STOCK_IP.iter().take(5) {
    match Tcp::new_with_ip(addr) {
        Ok(mut tcp) => {
            println!("✅ 连接成功: {}", addr);
            // 使用这个连接
            break;
        }
        Err(e) => {
            println!("⚠️  {} 失败: {}", addr, e);
        }
    }
}
```

或者运行诊断脚本：
```bash
./scripts/diagnose_network.sh
```

---

### 错误 3: "No route to host"

**完整错误信息**:
```
Error: No route to host (os error 113)
```

**原因**: 网络不可达或被防火墙阻止

**解决方案**:
1. 检查网络连接
2. 检查防火墙设置
3. 如果在公司网络，联系网络管理员
4. 尝试使用 VPN 或代理

---

### 错误 4: "Permission denied"

**完整错误信息**:
```
Error: Permission denied (os error 13)
```

**原因**: 防火墙阻止出站连接

**解决方案**:
```bash
# Ubuntu/Debian (ufw)
sudo ufw allow out 7709/tcp

# CentOS/RHEL (firewalld)
sudo firewall-cmd --add-port=7709/tcp --permanent
sudo firewall-cmd --reload

# 或者临时禁用防火墙测试（不推荐）
sudo ufw disable
```

---

## 🧪 测试连接

### 方法 1: 使用示例程序

```bash
cargo run --example test_connection
```

预期输出：
```
🚀 rustdx-complete 连接测试

方法1: 使用默认连接
连接到默认服务器: 115.238.56.198:7709

✅ 连接成功！

📊 股票行情:
  000001 平安银行: 11.45元 (+0.00%)

✅ 数据获取成功！
```

### 方法 2: 使用 cargo test

```bash
cargo test check_all_stock_ips -- --nocapture
```

预期输出：
```
✅ 检测到 15 个可用服务器 (总共 19 个):
  - 115.238.56.198:7709
  - 114.80.149.19:7709
  - 114.80.149.22:7709
  ...
```

### 方法 3: 简单代码测试

创建 `test.rs`:
```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::SecurityQuotes;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("连接到通达信服务器...");

    match Tcp::new() {
        Ok(mut tcp) => {
            println!("✅ 连接成功！");

            let mut quotes = SecurityQuotes::new(vec![(0, "000001")]);
            quotes.recv_parsed(&mut tcp)?;

            for quote in quotes.result() {
                println!("股票: {} 价格: {}", quote.code, quote.price);
            }

            Ok(())
        }
        Err(e) => {
            println!("❌ 连接失败: {}", e);
            Err(e.into())
        }
    }
}
```

运行：
```bash
cargo run --example test
```

---

## 📋 检查清单

在使用 rustdx-complete 前，请确认：

- [ ] 有互联网连接
- [ ] 防火墙允许 TCP 出站连接（端口 7709）
- [ ] 不在公司网络限制中
- [ ] 未设置阻止连接的代理
- [ ] rustdx-complete 版本 >= 0.6.6

---

## 🆘 仍然无法连接？

### 收集诊断信息

```bash
# 1. 运行诊断脚本
./scripts/diagnose_network.sh > diagnose_output.txt 2>&1

# 2. 测试网络连接
ping -c 5 115.238.56.198 > ping_output.txt 2>&1

# 3. 测试 TCP 连接
nc -zv 115.238.56.198 7709 > nc_output.txt 2>&1

# 4. 运行 Rust 测试
cargo test check_all_stock_ips -- --nocapture > rust_test.txt 2>&1
```

### 提交 Issue

将以上诊断信息提交到：
[https://github.com/jackluo2012/rustdx/issues](https://github.com/jackluo2012/rustdx/issues)

---

## 📚 相关文档

- [网络连接完整指南](NETWORK_CONNECTION_GUIDE.md)
- [项目 README](README.md)
- [示例程序](examples/)

---

**记住**: rustdx-connect 是连接到**远程服务器**，不需要本地服务！
