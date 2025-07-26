#!/bin/bash

# A3Mailer 文档清理脚本
# 清理冗余的中间过渡 MD 文件，保留重要文档

echo "🧹 开始清理 A3Mailer 项目文档..."

# 创建 docs 目录（如果不存在）
mkdir -p docs/archive

# 定义要保留的核心文档
KEEP_FILES=(
    "README.md"
    "README_CN.md"
    "A3MAILER_中文说明书.md"
    "CHANGELOG.md"
    "CONTRIBUTING.md"
    "SECURITY.md"
    "UPGRADING.md"
    "ROADMAP.md"
)

# 定义要归档的文档（移动到 docs/archive）
ARCHIVE_FILES=(
    "A3MAILER_PROJECT_SUMMARY.md"
    "AI_WEB3_FEATURES.md"
    "DEVELOPMENT_RULES.md"
    "FINAL_PRODUCTION_CODE_SUMMARY.md"
    "FINAL_PROJECT_REPORT.md"
    "GITHUB_UPLOAD_GUIDE.md"
    "MISSING_FEATURES_ANALYSIS.md"
    "PLACEHOLDER_CRATES_SUMMARY.md"
    "PRODUCTION_CODE_COMPLETION_REPORT.md"
    "PRODUCTION_COMPLETION_SUMMARY.md"
    "PRODUCTION_ENHANCEMENT_SUMMARY.md"
    "PROJECT_RENAME_SUMMARY.md"
    "RELEASE_NOTES_TEMPLATE.md"
    "RUST_BACKEND_TESTING_SUMMARY.md"
    "SECURITY_PROCESS.md"
    "ULTIMATE_PRODUCTION_COMPLETION.md"
)

echo "📁 归档中间过渡文档到 docs/archive..."

# 移动文档到归档目录
for file in "${ARCHIVE_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo "  📄 归档: $file"
        mv "$file" "docs/archive/"
    fi
done

echo "✅ 文档清理完成！"
echo ""
echo "📋 保留的核心文档:"
for file in "${KEEP_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo "  ✓ $file"
    fi
done

echo ""
echo "📦 归档的文档数量: ${#ARCHIVE_FILES[@]}"
echo "📁 归档位置: docs/archive/"

echo ""
echo "🎯 当前项目根目录的 MD 文件:"
ls -1 *.md 2>/dev/null | wc -l | xargs echo "  总数:"
ls -1 *.md 2>/dev/null || echo "  无 MD 文件"
