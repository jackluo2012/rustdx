#!/bin/bash
# 发布 rustdx-complete 与 rustdx-cli 到 crates.io。
#
# 用法：scripts/publish.sh            # 检查 + 确认后发布
#       scripts/publish.sh --dry-run  # 只做检查与打包演练，不上传
#
# 版本号取自各自的 Cargo.toml；crates.io 上已有同版本的包会被跳过，
# 所以只升了其中一个包的版本时也可以直接运行。
# 需要先 `cargo login`。rustdx-cli 依赖 rustdx-complete，必须后发布。

set -euo pipefail
cd "$(dirname "$0")/.."

DRY_RUN=""
[ "${1:-}" = "--dry-run" ] && DRY_RUN="--dry-run"

version_of() {
    cargo metadata --no-deps --format-version 1 |
        python3 -c "import json,sys; print(next(p['version'] for p in json.load(sys.stdin)['packages'] if p['name']=='$1'))"
}

# crates.io 上是否已有该版本
published() {
    cargo search "$1" --limit 1 2>/dev/null | grep -q "^$1 = \"$2\""
}

if [ -n "$(git status --porcelain)" ]; then
    echo "❌ 工作区有未提交的改动，请先提交" >&2
    exit 1
fi

echo "📋 发布前检查（格式 / clippy / 离线测试）..."
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
RUSTDX_SKIP_INTEGRATION_TESTS=1 cargo test --workspace --quiet

TODO=()
for crate in rustdx-complete rustdx-cli; do
    v=$(version_of "$crate")
    if published "$crate" "$v"; then
        echo "⏭️  $crate $v 已在 crates.io，跳过"
    else
        echo "📦 待发布：$crate $v"
        TODO+=("$crate")
    fi
done

if [ ${#TODO[@]} -eq 0 ]; then
    echo "没有需要发布的包（如需发布请先升版本号）"
    exit 0
fi

if [ -z "$DRY_RUN" ]; then
    read -r -p "确认发布以上包？发布后无法撤回 (yes/no): " confirm
    [ "$confirm" = "yes" ] || { echo "❌ 已取消"; exit 1; }
fi

for crate in "${TODO[@]}"; do
    cargo publish -p "$crate" $DRY_RUN
done

echo "✅ 完成。后续：打标签 v$(version_of rustdx-complete) 并推送"
