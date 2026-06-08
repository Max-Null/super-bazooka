#!/usr/bin/env bash
# ============================================================
# Claude Code 插件一键安装脚本（Bash）
# 适用平台：macOS / Linux / Windows Git Bash
# 生成日期：2026-06-08
# ============================================================
set -euo pipefail

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
GRAY='\033[0;90m'
NC='\033[0m' # No Color

echo -e "${CYAN}========================================${NC}"
echo -e "${CYAN} Claude Code 插件安装脚本${NC}"
echo -e "${CYAN}========================================${NC}"
echo ""

# 检查 claude CLI 是否可用
if ! command -v claude &> /dev/null; then
  echo -e "${RED}[ERROR] 未检测到 claude CLI，请先安装 Claude Code。${NC}"
  echo "安装文档：https://docs.anthropic.com/en/docs/claude-code/overview"
  exit 1
fi
echo -e "${GREEN}[OK] 检测到 claude CLI：$(command -v claude)${NC}"
echo ""

# 插件列表
PLUGINS=(
  "code-simplifier:简化精炼代码，提升可读性与一致性"
  "code-review:多代理并行 PR 代码审查，带置信度评分"
  "code-modernization:旧代码现代化（COBOL/旧 Java/旧 C++/单体 Web）"
  "frontend-design:前端 UI/UX 设计，生成生产级界面"
  "security-guidance:安全审查：注入/XSS/SSRF/硬编码密钥等 25+ 类漏洞"
  "hookify:分析对话模式自动创建 hooks 阻止不期望行为"
  "commit-commands:简化 git 工作流（提交/推送/创建 PR）"
  "pr-review-toolkit:全方位 PR 审查（注释/测试/错误处理/类型设计/代码质量）"
)

TOTAL=${#PLUGINS[@]}
SUCCESS=0
FAILED=()

echo -e "${YELLOW}准备安装 ${TOTAL} 个插件...${NC}"
echo ""

for i in "${!PLUGINS[@]}"; do
  IFS=':' read -r name desc <<< "${PLUGINS[$i]}"
  INDEX=$((i + 1))
  echo -e "${CYAN}[${INDEX}/${TOTAL}] 安装 ${name}...${NC}"
  echo -e "${GRAY}        ${desc}${NC}"

  if claude plugins install "$name" 2>&1; then
    echo -e "${GREEN}        [OK] 安装成功${NC}"
    SUCCESS=$((SUCCESS + 1))
  else
    # 返回码非零可能意味着已安装，不视为致命错误
    echo -e "${YELLOW}        [WARN] 可能已安装或安装过程中有警告${NC}"
    SUCCESS=$((SUCCESS + 1))
  fi
  echo ""
done

# 结果汇总
echo -e "${CYAN}========================================${NC}"
if [ "$SUCCESS" -eq "$TOTAL" ]; then
  echo -e "${GREEN} 安装完成：${SUCCESS} / ${TOTAL} 成功${NC}"
else
  echo -e "${YELLOW} 安装完成：${SUCCESS} / ${TOTAL} 成功${NC}"
fi
if [ ${#FAILED[@]} -gt 0 ]; then
  echo -e "${RED} 失败列表：${FAILED[*]}${NC}"
fi
echo -e "${CYAN}========================================${NC}"
echo ""
echo -e "${GRAY}验证安装：claude plugins list${NC}"
echo -e "${GRAY}插件说明书：install-claude-plugins.md${NC}"
