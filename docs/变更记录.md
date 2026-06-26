# 变更记录

## [0.1.1] — 2026-06-27

### NDJSON 交互协议 (P0/P1)

- **P0-1**: CC 子进程启动参数修正 — 添加 `--input-format stream-json` 和 `--permission-prompt-tool stdio`，移除 `--print`，消息通过 stdin NDJSON 发送
- **P0-2**: 修正 `control_response` 格式为嵌套结构 `response.subtype/request_id/response.behavior/updatedInput/message`，新增 `request_id` 往返和 `set_permission_mode` 初始化
- **P1-1**: `hook_callback` 不再自动允许，转发前端走用户审批流程
- **P1-2**: `AskUserQuestion` 问答弹窗 — radio/checkbox 选项组 + Other 自由输入，答案格式 `{questions, answers}`
- **P1-3**: interrupt 优雅停止 — 先发 `control_request(subtype: "interrupt")`，3 秒超时兜底
- **P1-4**: deny 含 `message`、allow 含 `updatedInput`（含在 P0-2）

### i18n 修复

- 修复 vue-i18n v10 消息编译器 `SyntaxError: 10` — `@` 用 `{'@...'}` 转义，`{ }` 字面量修正
- 授权弹窗 "Allow Bash?" 国际化 — `chat.allowTool: "允许使用 {tool}？"`
- 工具名翻译 — `Bash→命令行`、`Write→写入文件`、`Read→读取文件` 等，中英双份

### UI 改进

- 思考计时重构 — 每段 tool_use 前独立计时（`🧠Xs`），总耗时 = 各段求和，去除实时计时器，不受审批暂停影响
- Debug 面板复制按钮 — 与消息气泡同款 SVG 图标，鼠标移入显示
- 问答弹窗样式修复 — header 标签 `whitespace-nowrap`、去掉双滚动条

### 工程

- 版本号管理 — 从 `package.json` 注入 `__APP_VERSION__` 到 UI，三处同步（`package.json` / `tauri.conf.json` / `Cargo.toml`）
- 安全审查修复 — `hook_callback` 从自动允许改回用户审批

---

## [0.1.0] — 2026-06

### 核心功能

- Tauri 2 + Vue 3 + TypeScript 桌面应用架构
- Claude Code CLI 子进程管理 — 三线程模型（Waiter / Stdout Reader / Stderr）
- NDJSON stream-json 协议解析 — `StreamLine` + `StreamFrontendEvent`
- 会话管理 — 创建/删除/重命名/切换，SQLite 持久化
- 聊天界面 — 流式消息渲染、Markdown + Mermaid 渲染、审批栏
- 文件浏览 — FilePanel / FileTree / FilePreview / DiffViewer
- 命令面板 — 拼音搜索 + 动态命令注册
- 8 合 1 管理面板 — plugins/mcp/skills/agents/hooks/memory/permissions/styles
- API 设置 — DeepSeek API 配置 + 连接测试
- 国际化 — vue-i18n 中英双语，所有面向用户文本双份
- 主题 — 暗色/亮色/系统自动

### 权限系统

- 6 种权限模式 — `default` / `acceptEdits` / `bypassPermissions` / `plan` / `dontAsk` / `auto`
- `sync_permission_settings()` 写入 `~/.claude/settings.json`
- 批准场景 CRUD（SQLite `approved_scenarios` 表）

### 工程

- vitest 单元测试 + Playwright E2E + Rust 集成测试
- Conventional Commits 提交规范
- CLAUDE.md 项目文档 + docs/ 中文文档体系
