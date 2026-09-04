# RustDX 全面测试套件

本目录包含 RustDX 项目的开盘日全面测试套件。

## 📋 测试内容

### 核心功能测试 (8个API模块)

1. **SecurityQuotes** - 实时行情查询
   - 单只股票查询
   - 批量股票查询(50只)
   - 指数行情查询

2. **Kline** - K线数据获取
   - 日K线
   - 周K线

3. **FinanceInfo** - 财务信息
   - 总股本、净资产等32个字段

4. **MinuteTime** - 分时数据
   - 240个分时数据点

5. **Transaction** - 逐笔成交
   - tick级别成交数据

6. **SecurityList** - 股票列表
   - 深市和沪市股票

7. **五档买卖盘** - 完整性验证
   - 买一到买五、卖一到卖五

8. **v0.6.6新功能**
   - 行业分类映射
   - 概念板块查询

## 🚀 快速开始

### 方式1: 使用测试脚本(推荐)

```bash
# 运行所有测试
./scripts/run_comprehensive_test.sh

# 快速测试(仅核心功能)
./scripts/run_comprehensive_test.sh quick

# 生成测试报告
./scripts/run_comprehensive_test.sh report
```

### 方式2: 直接使用cargo

```bash
# 运行所有测试
cargo test --test comprehensive_test -- --live --nocapture

# 运行单个测试
cargo test test_01_security_quotes_single -- --live --nocapture

# 运行特定类别的测试
cargo test --test comprehensive_test test_0 -- --live --nocapture  # 核心功能
cargo test --test comprehensive_test test_09 -- --live --nocapture # 五档买卖盘
cargo test --test comprehensive_test test_1 -- --live --nocapture  # v0.6.6新功能
```

## 📊 测试报告

测试完成后，报告会保存在 `reports/` 目录：

```bash
ls -lh reports/
# 2026-01-05-comprehensive-test-report.md
```

报告包含：
- 执行摘要
- 详细测试结果
- 性能指标
- 发现的问题
- 优化建议

## 🎯 测试覆盖

| 测试类别 | 测试数量 | 状态 |
|---------|---------|------|
| 核心功能 | 8个模块 | ✅ |
| 实时数据验证 | 5个验证点 | ✅ |
| 五档买卖盘 | 完整性验证 | ✅ |
| 新功能 | 2个功能 | ✅ |
| 性能测试 | 4个指标 | ✅ |

## ⚡ 性能基准

- **单只股票查询**: <50ms
- **批量50只查询**: <200ms (实际: ~42ms) ✨
- **K线查询**: <50ms
- **财务信息**: <100ms

## 📝 环境要求

- Rust 1.x 或更高版本
- 网络连接(连接通达信服务器)
- Linux/WSL2/macOS/Windows

## 🔧 故障排除

### 测试跳过

如果看到消息"⚠️ 跳过集成测试"，需要设置环境变量：

```bash
export RUSTDX_LIVE_TEST=1
# 或
RUSTDX_LIVE_TEST=1 cargo test --test comprehensive_test -- --live --nocapture
```

### 网络问题

如果连接失败，检查：
1. 网络连接是否正常
2. 防火墙是否阻止连接
3. 通达信服务器是否可访问

## 📚 相关文档

- [详细测试报告](../reports/2026-01-05-comprehensive-test-report.md)
- [项目README](../README.md)
- [CHANGELOG](../CHANGELOG.md)

## 🤝 贡献

欢迎贡献测试用例！请遵循以下步骤：

1. 在 `tests/comprehensive_test.rs` 中添加测试函数
2. 使用 `#[test]` 标记
3. 添加测试结果统计
4. 更新此README

## 📄 许可证

MIT License - 与项目主许可证相同
