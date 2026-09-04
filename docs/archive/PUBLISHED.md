# 🎉 rustdx-complete v0.6.0 发布成功！

## ✅ 发布信息

- **版本**: 0.6.0
- **发布日期**: 2025-12-30
- **crates.io**: https://crates.io/crates/rustdx-complete
- **文档**: https://docs.rs/rustdx-complete
- **包大小**: 58KB
- **状态**: ✅ 已成功发布

## 📦 安装方式

### 方式 1: 使用最新版本

```toml
[dependencies]
rustdx-complete = "0.6"
```

### 方式 2: 使用确切版本

```toml
[dependencies]
rustdx-complete = "=0.6.0"
```

### 方式 3: 使用 cargo add

```bash
cargo add rustdx-complete
```

## 🚀 快速开始

### 1. 创建新项目

```bash
cargo new my_stock_app
cd my_stock_app
```

### 2. 添加依赖

```bash
cargo add rustdx-complete
```

### 3. 编写代码

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::SecurityQuotes;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 连接到通达信服务器
    let mut tcp = Tcp::new()?;

    // 获取股票行情
    let mut quotes = SecurityQuotes::new(vec![
        (0, "000001"),  // 平安银行
        (1, "600000"),  // 浦发银行
    ]);

    quotes.recv_parsed(&mut tcp)?;

    // 打印结果
    for quote in quotes.result() {
        println!(
            "{}: 当前价={}, 涨跌幅={}%",
            quote.code, quote.price, quote.change_percent
        );
    }

    Ok(())
}
```

### 4. 运行

```bash
cargo run
```

## ✨ v0.6.0 新功能

### 1. 中文编码修复

**修复前**（乱码）:
```
395001 �������
395002 ����¹�
```

**修复后**（正确显示）:
```
395001 主板Ａ股
395002 主板Ｂ股
```

### 2. 服务器连接优化

**修复前**:
```
⚠️  行情数据长度不足: 0 字节
返回数据: 0 只股票
```

**修复后**:
```
✅ 获取成功
返回数据: 3 只股票
000001 : 11.48元 (0.09%)
000002 : 4.62元 (0.00%)
600000 : 12.39元 (0.41%)
```

### 3. 内存安全提升

- ✅ 移除所有 `unsafe` 代码
- ✅ 添加边界检查
- ✅ 优雅处理错误数据

## 📊 功能对比

| 功能 | v0.5.0 | v0.6.0 | 改进 |
|------|--------|--------|------|
| 中文显示 | ❌ 乱码 | ✅ 正常 | 完全修复 |
| 数据获取 | ❌ 0条 | ✅ 正常 | 完全修复 |
| 内存安全 | ⚠️  unsafe | ✅ 安全 | 完全修复 |
| 示例代码 | ❌ 编译错误 | ✅ 正常 | 完全修复 |
| 错误处理 | ⚠️  panic | ✅ 优雅降级 | 显著改进 |

## 🧪 测试验证

所有示例程序测试通过：

```bash
# 股票行情
cargo run --example test_security_quotes
✅ 获取成功
返回数据: 3 只股票

# 指数行情
cargo run --example test_index_quotes
✅ 获取成功
返回数据: 2 个指数

# 财务信息
cargo run --example test_finance_info
✅ 获取成功
返回数据: 完整财务数据

# 股票列表（含中文）
cargo run --example test_chinese_encoding
✅ 获取成功
前10只股票：
  1. 395001 - 主板Ａ股
  2. 395002 - 主板Ｂ股
  3. 395004 - 创业板
  ...
```

## 📈 升级指南

### 从 v0.5.0 升级到 v0.6.0

**完全向后兼容，无需修改代码！**

只需更新版本号：

```toml
[dependencies]
rustdx-complete = "0.6"  # 从 "0.5" 更新到 "0.6"
```

然后运行：

```bash
cargo update
cargo build
```

## 🔗 相关链接

- **crates.io**: https://crates.io/crates/rustdx-complete
- **文档**: https://docs.rs/rustdx-complete
- **GitHub**: https://github.com/jackluo2012/rustdx
- **Release**: https://github.com/jackluo2012/rustdx/releases/tag/v0.6.0

## 📝 完整更新日志

详见 [CHANGELOG.md](CHANGELOG.md#v060---2025-12-30)

## 🙏 致谢

感谢所有用户的使用和反馈！

如有问题，请：
- 提交 [GitHub Issue](https://github.com/jackluo2012/rustdx/issues)
- 查看 [完整文档](https://docs.rs/rustdx-complete)

---

**享受使用 rustdx-complete v0.6.0！** 🚀🎉
