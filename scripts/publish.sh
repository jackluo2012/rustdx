#!/bin/bash
# rustdx-complete v0.6.0 发布脚本

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "    🚀 发布 rustdx-complete v0.6.0 到 crates.io"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 检查是否已登录
echo "📋 检查 crates.io 登录状态..."
if cargo search rustdx-complete >/dev/null 2>&1; then
    echo "   ✅ 已登录 crates.io"
else
    echo "   ⚠️  未登录或网络问题"
    echo ""
    echo "请先登录："
    echo "  1. 访问 https://crates.io/settings"
    echo "  2. 关联你的 GitHub 账号"
    echo "  3. 或运行: cargo login <your-api-token>"
    echo ""
    exit 1
fi

echo ""
echo "📦 发布信息："
echo "   - 包名: rustdx-complete"
echo "   - 版本: 0.6.0"
echo "   - 大小: 58KB"
echo ""

# 确认发布
read -p "确认发布？(yes/no): " confirm
if [ "$confirm" != "yes" ]; then
    echo "❌ 发布已取消"
    exit 1
fi

echo ""
echo "📤 开始发布..."
echo ""

# 发布（允许未提交的文件）
cargo publish --allow-dirty

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "    ✅ 发布完成！"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "🔗 包地址: https://crates.io/crates/rustdx-complete"
echo ""
echo "📝 后续步骤："
echo "   1. 访问上面的链接验证发布"
echo "   2. 创建 GitHub Release (tag: v0.6.0)"
echo "   3. 更新 README.md 中的徽章（如果需要）"
echo ""
