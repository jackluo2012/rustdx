# 🎉 rustdx-complete v0.6.0 发布指南

## ✅ 已完成的准备工作

1. **✓ 版本更新**
   - Cargo.toml: 0.5.0 → 0.6.0
   - rustdx-cmd/Cargo.toml: 同步更新到 0.6.0

2. **✓ CHANGELOG.md**
   - 添加 v0.6.0 更新日志
   - 详细记录所有重要修复

3. **✓ 代码修复**
   - 中文编码问题
   - 服务器连接问题
   - 内存安全问题
   - 示例代码问题

4. **✓ 文档更新**
   - README.md 添加更新日志
   - 创建 FIXES.md 详细修复记录
   - 创建 RELEASE.md 发布指南

5. **✓ 测试验证**
   - 所有库测试通过（32 tests）
   - 所有示例程序运行正常

## 📦 发布信息

```
包名: rustdx-complete
版本: 0.6.0
大小: 58KB
文件: 41个
```

## 🚀 发布步骤

### 方法 1：使用发布脚本（推荐）

```bash
./publish.sh
```

脚本会自动：
- 检查登录状态
- 显示发布信息
- 确认发布
- 执行发布命令

### 方法 2：手动发布

```bash
# 1. 检查登录状态
cargo search rustdx-complete

# 2. 确认包内容
cargo package --list --allow-dirty | head -20

# 3. 发布
cargo publish --allow-dirty
```

## ⚠️ 重要提示

### 首次发布

如果这是您第一次发布 `rustdx-complete`：

1. **访问 crates.io**: https://crates.io/settings
2. **关联 GitHub 账号**（推荐）
   - 使用 GitHub 关联更安全
   - 无需管理 API token
3. **或使用 API token**:
   ```bash
   cargo login <your-api-token>
   ```

### 发布前检查

- [x] 版本号已更新（0.6.0）
- [x] CHANGELOG.md 已更新
- [x] 所有测试通过
- [x] 文档已更新
- [x] 包大小合理（58KB）
- [ ] **已登录 crates.io**
- [ ] **确认包内容正确**

### 发布后不可修改

- ⚠️ **一旦发布，无法删除或修改**
- ✅ 如有问题，只能发布新版本（0.6.1, 0.7.0）
- ✅ 可以 yank（撤回），但包仍可下载

## 📝 发布后工作

### 1. 验证发布

```bash
# 访问包页面
https://crates.io/crates/rustdx-complete

# 或使用 cargo search
cargo search rustdx-complete
```

### 2. 创建 GitHub Release

```bash
# 创建标签
git tag -a v0.6.0 -m "Release v0.6.0 - Bug 修复和重要改进"
git push origin v0.6.0

# 然后在 GitHub 上创建 Release
# https://github.com/jackluo2012/rustdx/releases/new
```

Release 标题：`Release v0.6.0 - Bug 修复和重要改进`

Release 描述：
```markdown
## 🎉 重要更新

### 🔧 重要修复
1. 修复中文编码显示问题（GBK → UTF-8）
2. 修复服务器连接无数据的问题
3. 修复内存安全问题（移除 unsafe 代码）
4. 修复示例代码编译错误

### 📝 完整更新日志
详见 CHANGELOG.md
```

### 3. 更新 README.md（可选）

如果需要更新版本徽章：
```markdown
[<img alt="crates.io" src="https://img.shields.io/crates/v/rustdx-complete?style=flat&color=fc8d62&logo=rust&label=rustdx-complete" height="20">](https://crates.io/crates/rustdx-complete)
```

### 4. 通知用户

- GitHub Discussions
- 项目 Issues
- 其他社区渠道

## 🔍 故障排查

### 问题 1：未登录错误

```
error: no such crate as `rustdx-complete`
```

**解决**：
```bash
# 访问 https://crates.io/settings
# 关联 GitHub 账号或获取 API token
cargo login <token>
```

### 问题 2：版本冲突

```
error: crate `rustdx-complete` version `0.6.0` is already uploaded
```

**解决**：更新到新版本（0.6.1 或 0.7.0）

### 问题 3：包太大

```
error: package too large
```

**解决**：
- 检查 `exclude` 字段
- 移除不必要的文件
- 当前包 58KB，远低于限制（300KB）

## 📚 相关资源

- [crates.io 发布文档](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [语义化版本](https://semver.org/)
- [包管理最佳实践](https://doc.rust-lang.org/cargo/reference/publishing.html)

## ✅ 检查清单

发布前最后确认：

- [ ] 已阅读本文档
- [ ] 版本号正确（0.6.0）
- [ ] CHANGELOG.md 已更新
- [ ] 所有测试通过
- [ ] 已登录 crates.io
- [ ] 了解发布后不可修改
- [ ] 准备好创建 GitHub Release

---

## 🎯 快速开始（3步发布）

```bash
# 1. 登录（首次）
cargo login <token>

# 2. 运行脚本
./publish.sh

# 3. 验证
cargo search rustdx-complete
```

**祝发布顺利！** 🚀
