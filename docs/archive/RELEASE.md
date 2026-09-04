# 发布到 crates.io 指南

## 📋 发布前检查清单

### ✅ 已完成项目
- [x] 修复中文编码乱码问题
- [x] 修复服务器连接问题
- [x] 修复内存安全问题
- [x] 修复示例代码
- [x] 更新文档
- [x] 所有库测试通过（32 passed）

### 📝 发布前准备

#### 1. 检查当前版本
```bash
cargo pkgid rustdx-complete
# 输出: rustdx-complete 0.5.0
```

#### 2. 建议的版本号
根据修复的重要程度，建议发布新版本：
- **选项 A（推荐）**: `0.5.1` - 补丁版本（bug 修复）
- **选项 B**: `0.6.0` - 次版本（重要改进）

我建议使用 `0.5.1`，因为这主要是 bug 修复。

## 🚀 发布步骤

### 步骤 1: 更新版本号

编辑 `Cargo.toml`:

```toml
[package]
name = "rustdx-complete"
version = "0.5.1"  # 从 0.5.0 更新到 0.5.1
```

### 步骤 2: 更新 CHANGELOG.md

在 `CHANGELOG.md` 顶部添加：

```markdown
# [0.5.1] - 2025-12-30

## 🔧 重要修复

### 修复
- 修复中文编码显示问题（GBK → UTF-8）
- 修复服务器连接无数据的问题
- 修复内存安全问题（移除 unsafe 代码）
- 修复示例代码编译错误

### 改进
- 优化服务器 IP 顺序，使用可用服务器
- 添加数据边界检查，提高稳定性
- 更新所有文档和示例代码

## [0.5.0] - 之前版本
...
```

### 步骤 3: 验证包内容

```bash
# 查看将要发布的包内容
cargo package --list

# 打包但不发布（dry-run）
cargo package --allow-dirty
```

### 步骤 4: 检查包文件大小和质量

```bash
# 查看生成的 .crate 文件
ls -lh target/package/rustdx-complete-0.5.1.cargo

# 检查包内容（排除的文件）
cargo package --list | grep -E "^examples|^tests|^assets"
# 应该不显示这些目录（因为在 Cargo.toml 中排除了）
```

### 步骤 5: 登录 crates.io（如果需要）

```bash
# 使用 GitHub 关联（推荐）
# 访问 https://crates.io/settings
# 关联你的 GitHub 账号

# 或使用 API token
cargo login [your-api-token]
```

### 步骤 6: 发布到 crates.io

```bash
# 发布到 crates.io
cargo publish
```

**注意**：
- 首次发布需要等待几分钟审核
- 之后会自动发布
- 发布后无法删除，只能发布新版本

### 步骤 7: 验证发布

```bash
# 访问你的包页面
# https://crates.io/crates/rustdx-complete

# 或使用 cargo search
cargo search rustdx-complete
```

## ⚠️ 注意事项

### 发布前必读

1. **检查 Cargo.toml 配置**:
   ```toml
   [package]
   name = "rustdx-complete"
   version = "0.5.1"
   license = "MIT"  # ✅ 许可证
   description = "..."  # ✅ 描述
   repository = "https://github.com/jackluo2012/rustdx"  # ✅ 仓库地址
   authors = ["..."]  # ✅ 作者信息
   ```

2. **exclude 字段正确**:
   ```toml
   exclude = [
       "assets", "examples", "benches", "tests", "old", "tests-integration",
       "CHANGELOG.md", "*.csv", "*.log", ".github", ".gitignore",
       "rustfmt.toml", "LICENSE"  # LICENSE 已排除，使用 MIT 即可
   ]
   ```

3. **依赖项正确**:
   - 所有依赖都有明确的版本号
   - workspace 依赖已正确配置

### 发布后不可修改

- ⚠️ 一旦发布，**无法删除或修改**
- ⚠️ 如有问题，只能发布新版本（0.5.2, 0.6.0 等）
- ✅ 可以 yank（撤回）旧版本，但包仍然可用

## 📊 版本号规范

遵循语义化版本（Semver）：

- **0.5.0 → 0.5.1**: 补丁版本（bug 修复）
- **0.5.1 → 0.6.0**: 次版本（向后兼容的新功能）
- **0.6.0 → 1.0.0**: 主版本（破坏性变更）

## 🔍 快速测试

发布前可以测试本地包：

```bash
# 在临时目录创建新项目
cd /tmp
mkdir test-rustdx
cd test-rustdx
cargo init

# 添加本地依赖
echo 'rustdx-complete = { path = "/home/jackluo/data/learn/rust/rustdx" }' >> Cargo.toml

# 测试
cargo build
```

## 📝 发布后工作

1. **创建 GitHub Release**:
   - 标签: `v0.5.1`
   - 标题: "Release v0.5.1 - Bug 修复和改进"
   - 描述: 包含 CHANGELOG 内容

2. **更新 README.md**:
   - 添加新版本说明
   - 更新下载徽章（如果需要）

3. **通知用户**:
   - GitHub Discussions
   - 项目 Issues
   - 其他社区渠道

## 🆘 常见问题

### Q: 发布失败怎么办？
A:
1. 检查错误信息
2. 确保版本号没有冲突
3. 确保网络连接正常
4. 确认 crates.io 账户权限

### Q: 如何撤回已发布的版本？
A:
```bash
cargo yank --vers 0.5.1
```
注意：已撤回的版本仍然可以下载，但不会作为默认版本。

### Q: 包太大怎么办？
A:
- 检查 `exclude` 字段
- 移除不必要的文件
- 优化文档大小

## 📚 相关资源

- [crates.io 文档](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [语义化版本](https://semver.org/)
- [Cargo 发布指南](https://doc.rust-lang.org/cargo/reference/publishing.html)
