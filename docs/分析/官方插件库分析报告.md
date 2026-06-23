# Claude Code 官方插件库分析报告

> 仓库: [anthropics/claude-plugins-official](https://github.com/anthropics/claude-plugins-official)
> 生成日期: 2026-06-09
> ⭐ 29,646 | 👀 185

## 仓库概况

Anthropic 官方维护的高质量 Claude Code 插件目录。结构分为 `/plugins`（内部开发）和 `/external_plugins`（第三方），共 **36 个内部插件**，按功能分为 6 大类。

---

## 已安装插件及使用指南（14 个）

### 1. code-simplifier — 代码精炼

**描述**: 简化精炼代码，提升可读性与一致性，保留全部功能。

**触发方式**: 自动触发。编写完代码后，Claude 会自动调用 `code-simplifier` agent 对修改过的代码进行精炼。

**手动触发**:
```
"simplify the code I just wrote"
"refactor this function for clarity"
```

---

### 2. code-review — PR 代码审查

**描述**: 多代理并行 PR 审查。启动 4 个独立 agent 从不同角度审计变更，置信度评分 ≥80 才上报。

**命令**: `/code-review`

**审查维度**:
- Agent #1 & #2: CLAUDE.md 合规性检查
- Agent #3: 变更中的明显 bug 扫描
- Agent #4: git blame / 历史上下文分析

**用法**:
```
/code-review
```
自动跳过已关闭、draft、trivial 或已审查过的 PR。

---

### 3. code-modernization — 旧代码现代化

**描述**: 结构化旧代码现代化工作流——从评估到转换的完整链路。

**命令**（按顺序执行）:

| 命令 | 用途 | 产出 |
|------|------|------|
| `/modernize-assess <system-dir>` | 代码库清单（语言/复杂度/技术债） | `analysis/<system>/ASSESSMENT.md` |
| `/modernize-map <system-dir>` | 依赖与拓扑图 | `analysis/<system>/TOPOLOGY.html` |
| `/modernize-extract-rules <system-dir>` | 挖掘业务规则（Given/When/Then） | `analysis/<system>/BUSINESS_RULES.md` |
| `/modernize-brief <system-dir> [target-stack]` | 综合现代化方案（审批关卡） | `analysis/<system>/MODERNIZATION_BRIEF.md` |
| `/modernize-reimagine <system-dir> <vision>` | 绿场重建 | `modernized/<system>-reimagined/` |
| `/modernize-transform <system-dir> <module> <stack>` | 单模块绞杀器式重写 | `modernized/<system>/<module>/` |
| `/modernize-harden <system-dir>` | 安全加固（OWASP/CWE/CVE） | `analysis/<system>/SECURITY_FINDINGS.md` |

**专用 agent**: `legacy-analyst`, `business-rules-extractor`, `architecture-critic`, `security-auditor`, `test-engineer`

**用法示例**:
```
/modernize-assess legacy-billing
/modernize-map legacy-billing
/modernize-extract-rules legacy-billing
```

---

### 4. frontend-design — 前端设计

**描述**: 生成独特的、生产级前端界面，避免通用 AI 美学。

**触发方式**: Claude 在做前端工作时自动调用该 skill。

**手动触发**:
```
"Create a dashboard for a music streaming app"
"Build a landing page for an AI security startup"
"Design a settings panel with dark mode"
```

---

### 5. security-guidance — 安全审查（三层防护）

**描述**: 三层安全审查——模式警告 + LLM diff 审查 + Agentic commit 审查。

**三层防护**:

| 层 | 触发时机 | 机制 |
|----|----------|------|
| 1. 模式警告 | Edit/Write 时 | ~25 种危险模式的正则匹配（yaml.load, innerHTML, pickle, 硬编码密钥等） |
| 2. LLM diff 审查 | 每轮结束 | 发送 diff 给快 LLM（Opus 4.7），高危发现反馈给 Claude 修复 |
| 3. Agentic commit 审查 | git commit 时 | SDK 驱动的深度审查——跨文件追踪数据流（IDOR, auth bypass, SSRF） |

**配置**（环境变量）:

```bash
# 选择模型
SECURITY_REVIEW_MODEL=claude-opus-4-7   # 默认

# 启用/禁用层
SECURITY_GUIDANCE_DISABLE=1    # 完全关闭
ENABLE_PATTERN_RULES=0         # 关闭第 1 层
ENABLE_CODE_SECURITY_REVIEW=0  # 关闭第 2/3 层
```

**前置条件**: Python 3.8+, Claude Code CLI ≥ v2.1.144

---

### 6. hookify — 行为规则配置

**描述**: 无需编辑 hooks.json，用自然语言创建 hooks 规则。

**命令**:

| 命令 | 用途 |
|------|------|
| `/hookify <描述>` | 从自然语言描述创建规则 |
| `/hookify` | 无参数：分析对话找出不期望行为 |
| `/hookify:configure` | 交互式启用/禁用规则 |
| `/hookify:list` | 列出所有已配置规则 |
| `/hookify:help` | 获取帮助 |

**规则文件位置**: `.claude/hookify.<规则名>.local.md`（Markdown + YAML frontmatter）

**用法示例**:
```
/hookify Don't use console.log in TypeScript files
/hookify Warn me when I use rm -rf commands
/hookify
```

**特点**: 无需重启，规则在下一次工具调用时立即生效。

---

### 7. commit-commands — Git 工作流简化

**描述**: 自动化 git 操作——提交、推送、创建 PR。

**命令**:

| 命令 | 用途 |
|------|------|
| `/commit` | 分析变更 → 自动生成 commit message → 暂存 → 提交 |
| `/commit-push-pr` | 提交 + 推送 + 创建 PR（一键完成） |
| `/commit-commands:clean_gone` | 清理所有 `[gone]` 分支及关联 worktree |

**特点**:
- 自动匹配仓库的 commit message 风格
- 遵循 conventional commit 规范
- 自动跳过敏感文件（.env, credentials.json）

**用法**:
```
/commit
/commit-push-pr
/commit-commands:clean_gone
```

---

### 8. pr-review-toolkit — 全方位 PR 审查

**描述**: 6 个专业 agent 组成的 PR 审查套件。

| Agent | 专注领域 | 触发关键词 |
|-------|----------|-----------|
| `comment-analyzer` | 注释准确性、文档完整性、注释腐烂 | "Check if the comments are accurate" |
| `pr-test-analyzer` | 测试覆盖质量、边界情况 | "Are there any critical test gaps?" |
| `silent-failure-hunter` | 静默失败、错误处理不当 | "Check error handling in this PR" |
| `type-design-analyzer` | 类型设计、封装、不变量 | "Review the type design" |
| `code-reviewer` | 代码风格、最佳实践合规 | General code review |
| `code-simplifier` | 代码简化、可维护性 | "Simplify this code" |

**命令**: `/pr-review-toolkit:review-pr` — 一次触发全部 agent 的综合审查。

**用法**:
```
"Review this PR for silent failures"
"Check if the tests are thorough"
"Analyze comments for technical debt"
```

---

### 9. claude-code-setup ⭐ 新装 — 项目自动化推荐

**描述**: 扫描代码库，推荐量身定制的 Claude Code 自动化配置。

**触发方式**: 自动 skill，读操作不修改文件。

**推荐类别**:
- MCP Servers（context7 文档、Playwright 前端测试）
- Skills（Plan agent、frontend-design）
- Hooks（auto-format、auto-lint、阻止敏感文件）
- Subagents（安全、性能、可访问性审查）
- Slash Commands（/test、/pr-review、/explain）

**用法**:
```
"recommend automations for this project"
"help me set up Claude Code"
"what hooks should I use?"
```

---

### 10. claude-md-management ⭐ 新装 — CLAUDE.md 维护

**描述**: 维护和优化 CLAUDE.md 文件——审计质量、捕获会话学习。

**两个工具**:

| | `claude-md-improver` (skill) | `/revise-claude-md` (command) |
|---|---|---|
| 目的 | 保持 CLAUDE.md 与代码库一致 | 捕获会话学习 |
| 触发 | 代码库变更时 | 会话结束时 |
| 用法 | "audit my CLAUDE.md files" | `/revise-claude-md` |

**用法**:
```
"audit my CLAUDE.md files"
"check if my CLAUDE.md is up to date"
/revise-claude-md
```

---

### 11. typescript-lsp ⭐ 新装 — TypeScript 语言服务器

**描述**: 为 Claude Code 提供 TypeScript/JavaScript 代码智能——跳转定义、查找引用、错误检查。

**支持扩展名**: `.ts` `.tsx` `.js` `.jsx` `.mts` `.cts` `.mjs` `.cjs`

**前置条件**: 需全局安装 language server
```bash
npm install -g typescript-language-server typescript
```

**对 cc-gui 的价值**: 本项目是 Vue + TypeScript + Tauri 技术栈，安装后 Claude 可直接读取 TypeScript 类型信息，理解组件接口、类型定义、跳转引用。

---

### 12. feature-dev ⭐ 新装 — 功能开发工作流

**描述**: 7 阶段结构化功能开发流程——理解代码库 → 澄清需求 → 设计架构 → 质量审查。

**命令**: `/feature-dev [功能描述]`

**7 阶段流程**:
1. 代码库探索
2. 需求澄清
3. 架构设计
4. 详细设计
5. 实现
6. 质量审查（code-reviewer agent）
7. 文档更新

**专用 agent**: `code-explorer`, `code-architect`, `code-reviewer`

**用法**:
```
/feature-dev Add user authentication with OAuth
/feature-dev
```

---

### 13. plugin-dev ⭐ 新装 — 插件开发工具包

**描述**: 开发 Claude Code 插件的完整工具包。

**7 个 skill**:

| Skill | 用途 |
|-------|------|
| `hook-development` | Hooks API 与事件驱动自动化 |
| `mcp-integration` | MCP 服务器集成 |
| `plugin-structure` | 插件组织与 manifest 配置 |
| `plugin-settings` | 配置模式（.claude/plugin-name.local.md） |
| `command-development` | 创建带 frontmatter 和参数的 slash 命令 |
| `agent-development` | AI 辅助生成自主 agent |
| `skill-development` | 渐进式暴露的 skill 创建 |

**命令**: `/plugin-dev:create-plugin` — 端到端插件创建工作流（8 阶段）

**专用 agent**: `agent-creator`, `plugin-validator`, `skill-reviewer`

---

### 14. mcp-tunnels ⭐ 新装 — MCP 隧道连接

**描述**: 通过 Anthropic MCP 隧道连接私有 MCP 服务器——无入站端口、无公网暴露。

**命令**: `/create-docker-mcp-tunnel [deployment-dir]`

**流程**:
1. Preflight — 检查 Docker、OpenSSL、出站连通性
2. 创建隧道（Claude Console）— 复制域名和 token 到 `.env`
3. 证书 — OpenSSL 生成 CA 和服务器证书
4. Docker Compose 启动 tunnel proxy + sample server

**前置条件**: Docker, Docker Compose, OpenSSL

**注意**: 研究预览版，依赖 Cloudflare 传输。

---

## 插件能力速查表

| 插件 | 入口 | 类型 | 对 cc-gui 价值 |
|------|------|------|---------------|
| `code-simplifier` | 自动 / agent | agent | ⭐⭐⭐ 日常 |
| `code-review` | `/code-review` | command | ⭐⭐⭐ PR 审查 |
| `code-modernization` | `/modernize-*` | 7 commands | ⭐ 旧代码改造 |
| `frontend-design` | 自动 / skill | skill | ⭐⭐⭐ Vue 前端 |
| `security-guidance` | 自动 / hook | 3-layer hook | ⭐⭐⭐ 每次提交 |
| `hookify` | `/hookify` | command+skill | ⭐⭐ 行为规则 |
| `commit-commands` | `/commit` 等 | 3 commands | ⭐⭐⭐ 每日 git |
| `pr-review-toolkit` | `/review-pr` | command+6 agents | ⭐⭐⭐ PR 审查增强 |
| `claude-code-setup` | 自动 / skill | skill | ⭐⭐⭐ 项目初始化 |
| `claude-md-management` | skill+`/revise-claude-md` | skill+command | ⭐⭐⭐ CLAUDE.md 维护 |
| `typescript-lsp` | 自动 | LSP | ⭐⭐⭐ TS 智能感知 |
| `feature-dev` | `/feature-dev` | command+3 agents | ⭐⭐ 大功能开发 |
| `plugin-dev` | skill+command | 7 skills+command | ⭐ 插件开发 |
| `mcp-tunnels` | `/create-docker-mcp-tunnel` | command | ⭐ 私有 MCP |

---

## 仓库完整插件目录（36 个内部插件）

### 🔧 开发工作流（10 个）

| 插件 | 简介 |
|------|------|
| `code-simplifier` | 简化精炼代码，提升可读性与一致性，保留全部功能不变 |
| `code-review` | 多代理并行 PR 审查——4 个 agent 独立审计，置信度 ≥80 才上报，过滤误报 |
| `code-modernization` | 旧代码现代化全流程（assess→map→extract-rules→brief→reimagine\|transform→harden），支持 COBOL/旧 Java/旧 C++/单体 Web |
| `pr-review-toolkit` | 6 个专业 agent 组成的 PR 审查套件（注释分析/测试覆盖/静默失败/类型设计/代码质量/代码简化） |
| `commit-commands` | 简化 git 工作流——分析变更自动生成 commit message、一键提交+推送+创建 PR、清理 gone 分支 |
| `hookify` | 用自然语言创建 hooks 规则（Markdown + YAML frontmatter），无需编辑 hooks.json，无需重启即刻生效 |
| `feature-dev` | 7 阶段结构化功能开发（探索→澄清→架构设计→详细设计→实现→审查→文档），内置 3 个专业 agent |
| `agent-sdk-dev` | Claude Agent SDK 开发辅助插件 |
| `plugin-dev` | 插件开发工具包——7 个 skill 覆盖 hooks/MCP/插件结构/配置/命令/agent/skill 开发 |
| `mcp-server-dev` | MCP 服务器设计与构建指导——部署模式（remote HTTP/MCPB/local）、工具设计模式、认证、交互式 MCP 应用 |

### 🌐 语言服务器 LSP（12 个）

| 插件 | 对应语言 | 简介 |
|------|----------|------|
| `clangd-lsp` | C / C++ | 基于 clangd 的代码智能（跳转定义/查找引用/错误检查） |
| `csharp-lsp` | C# | C# 语言服务器，提供 IntelliSense 级别支持 |
| `gopls-lsp` | Go | 官方 Go 语言服务器 gopls，提供代码导航与诊断 |
| `jdtls-lsp` | Java | Eclipse JDT Language Server，Java 代码智能 |
| `kotlin-lsp` | Kotlin | Kotlin 语言服务器，支持代码补全与诊断 |
| `lua-lsp` | Lua | Lua 语言服务器，支持语法检查与代码导航 |
| `php-lsp` | PHP | PHP 语言服务器，提供静态分析与代码导航 |
| `pyright-lsp` | Python | 基于 Pyright 的 Python 类型检查与代码智能 |
| `ruby-lsp` | Ruby | Ruby LSP 服务器，支持代码补全与诊断 |
| `rust-analyzer-lsp` | Rust | rust-analyzer 语言服务器，Rust 代码智能 |
| `swift-lsp` | Swift | Swift 语言服务器，支持 SourceKit-LSP |
| `typescript-lsp` | TypeScript / JavaScript | TypeScript 语言服务器（`.ts` `.tsx` `.js` `.jsx` `.mts` `.cts` `.mjs` `.cjs`） |

### 🛠 配置与效率工具（6 个）

| 插件 | 简介 |
|------|------|
| `claude-code-setup` | 分析代码库并推荐量身定制的自动化配置（hooks / skills / MCP / 子代理 / slash 命令），只读不修改 |
| `claude-md-management` | 维护 CLAUDE.md 文件——审计代码库对齐度、捕获会话学习、保持项目记忆新鲜（skill + /revise-claude-md 命令） |
| `mcp-tunnels` | 通过 Anthropic MCP 隧道连接私有 MCP 服务器——Docker Compose 端到端，无入站端口/无公网暴露 |
| `session-report` | 会话报告生成——总结本次会话的工作内容和产出 |
| `playground` | 创建交互式 HTML 演示场——自包含单文件可视化探索器，含实时预览和复制按钮 |
| `cwc-makers` | M5Stack Cardputer 硬件入门——一键刷 UIFlow 固件、安装 Claude Buddy 应用 |

### 🎨 输出与交互风格（3 个）

| 插件 | 简介 |
|------|------|
| `frontend-design` | 前端 UI/UX 设计——生成独特的、生产级前端界面，避免通用 AI 美学，大胆的排版和配色 |
| `explanatory-output-style` | 添加教育性实现洞察——解释实现选择与代码库模式（模拟已弃用的 Explanatory 输出风格） |
| `learning-output-style` | 交互式学习模式——在关键决策点请求有意义的代码贡献（模拟未发布的 Learning 输出风格） |

### 🧠 专项能力（3 个）

| 插件 | 简介 |
|------|------|
| `math-olympiad` | 竞赛数学求解（IMO / Putnam / USAMO）——对抗性验证捕捉自验证遗漏，校准式弃权优于幻觉 |
| `security-guidance` | 三层安全防护——(1) 模式警告 25+ 危险 API，(2) LLM diff 审查，(3) Agentic commit 深度跨文件审查 |
| `skill-creator` | 创建、优化和评估 skill——含基准测试、方差分析和触发词优化 |

### 📚 参考与实验（2 个）

| 插件 | 简介 |
|------|------|
| `example-plugin` | 完整示例插件——展示 commands / agents / skills / hooks / MCP 全部扩展选项，插件开发参考 |
| `ralph-loop` | 持续自引用 AI 循环（Ralph Wiggum 技术）——用相同 prompt 在 while-true 循环中迭代至任务完成 |
