# cc-gui CLAUDE.md

> Claude Code Desktop GUI — Tauri 2 + Vue 3 + TypeScript 桌面应用
>
> 为 Claude Code CLI 提供图形化桌面界面，支持 DeepSeek API 代理后端。

---

## 项目架构

```
cc-gui/
├── src/                       # Vue 3 前端
│   ├── components/
│   │   ├── chat/              # ChatPanel, MessageBubble, InputBar, InputBarToolbar, ModeBar, ThinkingIndicator, ContextIndicator
│   │   ├── layout/            # AppShell (主布局容器，含全局键盘快捷键)
│   │   ├── session/           # SessionSidebar (会话侧边栏)
│   │   ├── files/             # FilePanel, FileTree, FilePreview, DiffViewer
│   │   ├── settings/          # SettingsPanel (API配置/连接测试/主题语言)
│   │   └── shared/            # ManagePanel(8合1管理), MarkdownRenderer, MermaidRenderer, CommandPalette, ModalShell, ContextUsageModal, ErrorBoundary, FilePreviewModal
│   ├── composables/           # useStreamProcessor, useCommandRegistry, useNewSession, useFilePreview, useCommandPalette(含 ChatCommandBus), useDebugLog, useHighlight
│   ├── stores/                # session.ts, chat.ts, settings.ts (Pinia)
│   ├── lib/                   # utils.ts, tauri-bridge.ts, tauri-mock.ts, pinyin.ts
│   ├── locales/               # zh.json, en.json (vue-i18n)
│   ├── router/                # Vue Router 路由配置
│   └── assets/                # main.css（含亮色主题，不用再单独引入 theme-light.css）
├── src-tauri/                 # Rust 后端 (Tauri 2)
│   ├── src/
│   │   ├── main.rs            # 程序入口
│   │   ├── lib.rs             # 23 个 Tauri commands + AppState
│   │   ├── process.rs         # 三线程进程模型 (Waiter / Stdout Reader / Stderr)
│   │   ├── protocol.rs        # NDJSON stream-json 事件解析
│   │   ├── session.rs         # 会话 CRUD + API 连接测试 + 批准场景
│   │   └── db.rs              # SQLite 初始化与迁移
│   └── tests/                 # Rust 集成测试
├── e2e/                       # Playwright E2E 测试 (12个用例)
├── docs/                      # 项目文档（文件名用中文）
│   ├── 知识/                  # 自举指南、架构穿透等参考文档
│   └── 计划/                  # 实施计划
└── scripts/                   # 构建/测试辅助脚本
```

### 核心数据流

```
User Input (InputBar.vue)
  → sendMessage (Tauri IPC command, lib.rs)
    → sync_permission_settings → spawn_claude_session (process.rs)
      → Claude Code CLI (subprocess, stream-json + --input-format stream-json)
        │  ← stdin: {"type":"user","message":{...}} + set_permission_mode
        │  → stdout: NDJSON lines → BufReader (process.rs)
        │       → StreamLine::parse → to_frontend_event (protocol.rs)
        │         → app_handle.emit("stream-event") (Tauri event)
        │           → useStreamProcessor.ts listen → Pinia store → Vue 3 reactive render
        │
        ├─ control_request(can_use_tool/hook_callback)
        │    → chat.pendingControlRequest → 审批栏/问答弹窗
        │    → 用户操作 → control_response → stdin (StdinManager)
        │
        └─ control_request(interrupt)
             ← 用户点停止 → stdin → CC 优雅退出 → 3s 超时 kill
```

### 审批交互流程

```
CC stdout: control_request
  → protocol.rs: 提取 request_id, tool_name, tool_input
    → StreamFrontendEvent → Tauri emit("stream-event")
      → useStreamProcessor.ts → chat.addControlRequest()
        → ChatPanel.vue 渲染:
          ├─ tool_name === "AskUserQuestion" → 问答弹窗 (radio/checkbox/Other)
          └─ 其他                                 → 审批栏 (允许/拒绝)
            → handleAllow/handleDeny/submitAnswers
              → sendStdin(control_response)
                → Rust StdinManager.send() → CC stdin
```

### 功能索引（排查问题时先看这里）

> 当用户报告某个功能异常时，此表可直接定位到实现文件和数据源，避免全项目 Grep。

| 功能 | 前端入口 | 核心逻辑 | 后端命令 | 数据源/配置 |
|------|----------|----------|----------|-------------|
| 聊天消息 | `ChatPanel.vue` | `useStreamProcessor.ts` 监听 `stream-event` → `chat` store | `send_message`, `send_stdin`, `stop_session`, `list_messages`, `save_message` | SQLite `messages` 表 |
| 流事件处理 | `useStreamProcessor.ts` | 注册/注销 Tauri 事件监听，去重，保存消息，**分段思考计时** | `spawn_claude_session` (`process.rs`) | Claude CLI `stream-json` stdout |
| **工具审批交互** | `ChatPanel.vue` 审批栏 | 收到 `control_request(can_use_tool)` 弹出审批栏，允许/拒绝 → `control_response` 写入 stdin | `send_stdin` (`lib.rs`) → `StdinManager.send()` | CC CLI stdin NDJSON |
| **AskUserQuestion 问答** | `ChatPanel.vue` 问答弹窗 | `tool_name === "AskUserQuestion"` 时弹 radio/checkbox/Other 问答弹窗，提交 answers | 同上 | CC CLI stdin NDJSON |
| 会话管理 | `SessionSidebar.vue` | `session` store → `tauri-bridge.ts` | `create_session`, `list_sessions`, `delete_session`, `rename_session`, `get_session`, `store_claude_session` | SQLite `sessions` 表 |
| **MCP 管理** | `ManagePanel.vue` (tab `"mcp"`) | `loadMCP()` 扫描 `settings.json` + `.mcp.json`；`extractMcpServers()` 解析配置；`connectedMcpServers` 追踪运行时连接 | `get_claude_dir`, `read_file_content`, `get_workspace_root` | `~/.claude/settings.json`, `.mcp.json`, CLI 运行时 `system/init` 事件 |
| 插件管理 | `ManagePanel.vue` (tab `"plugins"`) | `loadJSON("enabledPlugins")` 读取启停状态 | `get_claude_dir`, `read_file_content`, `write_file` | `~/.claude/settings.json` |
| 技能/Agent/Hooks | `ManagePanel.vue` (tabs) | 扫描 `~/.claude/` 对应子目录 | `get_claude_dir`, `list_dir`, `read_file_content`, `write_file` | `~/.claude/skills/`, `agents/`, `settings.json` hooks |
| Memory | `ManagePanel.vue` (tab `"memory"`) | `loadMemory()` 递归扫描目录树 | `get_claude_dir`, `list_dir`, `read_file_content` | `~/.claude/memory/` |
| 权限管理 | `ManagePanel.vue` (tab `"permissions"`) + `ChatPanel.vue` 审批栏 | `sync_permission_settings()` 写 `settings.json`；审批场景 CRUD | `add_approved_scenario`, `remove_approved_scenario`, `list_approved_scenarios` | `~/.claude/settings.json`, SQLite `approved_scenarios` 表 |
| 输出样式 | `ManagePanel.vue` (tab `"styles"`) | `loadJSON("outputStyles")` | `get_claude_dir`, `read_file_content`, `write_file` | `~/.claude/settings.json` |
| 命令面板 | `CommandPalette.vue` | `useCommandPaletteBus`(打开), `useCommandRegistry`(动态注册), `pinyin.ts`(拼音搜索) | 无 | 内置命令 + `localStorage` 最近使用 |
| Token 监控 | `ContextIndicator.vue` + `ContextUsageModal.vue` | `chat` store 中的 token 统计字段 | 无 | `stream-event` 中的 `input_tokens`/`output_tokens` |
| 文件浏览 | `FilePanel.vue` + `FileTree.vue` | 动态加载目录树，右键菜单 | `list_dir`, `read_file_content`, `get_workspace_root`, `reveal_in_explorer` | 本地文件系统 |
| 文件预览 | `FilePreviewModal.vue` + `FilePreview.vue` | 自动检测类型（图片/code/md/文本/二进制） | `read_file_content`, `read_file_base64` | 本地文件 |
| Diff 对比 | `DiffViewer.vue` | `diff` 库行级差异计算 | 无 | 传入的 `oldStr` / `newStr` |
| API 设置 | `SettingsPanel.vue` | `settings` store → `localStorage`；连接测试 | `connect_llm` | `localStorage` (`cc-gui-settings`) |
| 主题/语言 | `SettingsPanel.vue` + `AppShell.vue` | `settings` store，`data-theme` 属性 | 无 | `localStorage` |

### Rust 模块职责

| 模块 | 职责 |
|------|------|
| `main.rs` | 程序入口，初始化 DB + 启动 Tauri |
| `lib.rs` | 23 个 Tauri commands：会话 CRUD、消息管理、文件操作、进程管理 |
| `process.rs` | 三线程模型 spawn Claude CLI：Waiter（进程生命周期）、Stdout Reader（NDJSON 解析→事件发射，含 `control_request` 处理）、Stderr Reader（错误转发）。**启动后通过 stdin 写入用户消息 NDJSON + `set_permission_mode` 控制请求** |
| `protocol.rs` | `StreamLine`：解析 NDJSON → `StreamFrontendEvent`；支持 system/assistant/user/result/control_request/stream_event |
| `session.rs` | `SessionManager`：SQLite 背书的会话 CRUD + 消息持久化 + DeepSeek API 连接测试 + BypassMode 批准场景 |
| `db.rs` | `Db`：SQLite 初始化 + WAL 模式 + 4 张表（sessions, messages, settings, approved_scenarios）+ 3 个索引 |

### Vue 组件通信模式

- **全局状态**: Pinia stores (`chat`, `session`, `settings`) — 跨组件共享
- **数据流**: `defineProps` ↓ + `defineEmits` ↑ — 禁止 `defineModel`
- **Tauri 桥接**: `lib/tauri-bridge.ts` — 封装所有 `invoke()` 调用
- **测试 Mock**: `lib/tauri-mock.ts` — 完整 Tauri API mock，浏览器内运行 e2e

---

## 构建与测试命令

### 开发

```bash
npm run dev           # 仅启动 Vite 前端 (http://localhost:1420)
npm run dev:tauri     # 启动 Tauri 桌面应用（前端 + Rust 后端）
```

### 构建

```bash
npm run build         # TypeScript 类型检查 + Vite 构建
npm run build:tauri   # Tauri 生产构建（Windows: .msi + .exe installer）
npm run build:tauri:msi   # 仅 MSI 安装包
npm run build:tauri:nsis  # 仅 NSIS 安装包
```

### 测试

```bash
npm run test              # vitest 单元测试（前端）
npm run test:e2e          # Playwright E2E 测试（mock 模式）
npm run test:e2e:smoke    # E2E 冒烟测试（chat flow + visual）
npm run test:e2e:real     # 真实 Claude CLI 输出的 E2E 测试
npm run test:rust         # Rust 单元测试
npm run test:rust:real    # Rust 真实集成测试（--ignored）
npm run test:all          # 全部测试（vitest + e2e + rust）
npm run test:quick        # 快速测试（vitest + rust，跳过 e2e）
```

---

## 🔴 硬性规则：禁止手搓轮子（NO REINVENTION）

> **这是本项目的最高优先级规则。违反此规则是项目中最严重的工程错误。**

### 写任何新代码前，必须执行以下检查（按顺序）：

1. **Grep 搜索已有实现**：写新函数/组件/composable/hook 之前，先用 Grep 在整个 `src/` 目录搜索功能关键词。项目里大概率已有类似逻辑。
2. **检查 composables 目录**：`src/composables/` 下是否已有能复用的 `useXxx()`。
3. **检查 lib 目录**：`src/lib/utils.ts` 或 `src/lib/tauri-bridge.ts` 是否已有对应工具函数。
4. **检查同类组件**：其他 `.vue` 组件是否已有相同的交互逻辑（按钮行为、数据处理、API 调用模式）。

### 常见违规场景（每次必须自查）：

| 违规行为 | 正确做法 |
|----------|----------|
| 在组件 B 里重写组件 A 已有的函数 | 提取为 composable，两处导入同一个 |
| 手写 `invoke()` 而不是用 `tauri-bridge.ts` 封装 | 一律通过 bridge 调用 |
| 复制粘贴另一个组件的 `<script>` 逻辑 | 提取共享逻辑到 composable 或 utils |
| 新写一个已有同类功能的 util 函数 | Grep 搜索 → 复用现有实现 |

### 为什么这么严格？

每次都手搓轮子 → 同一逻辑散落多处 → 改 A 处漏 B 处 → Bug 积累 → 无人能读懂全部代码 → 项目变屎山。

**一个 bug 的修复方式如果是"让它和那边已经正确的代码一样"，说明你一开始就该复用，而不是重写。**

---

## 🔵 UI 风格一致性：写 UI 前先看同类界面

> 类比"禁止手搓轮子"的 UI 版本。一致性是一种审美——优秀的 UI 不是每个组件独自漂亮，而是整体像一个团队做的。

### 写任何 UI 前，必须执行以下检查：

1. **看同类组件的模板**：这个按钮/标签/列表在项目中其他组件里怎么写的？直接用同样的 class 和 style 模式。
2. **看 ModalShell 用法**：需要弹窗时，先看 `ModalShell.vue` 提供了哪些插槽（`#header` / `#footer` / default）。不要自己在子组件里用 flex hack 模拟三段式布局。
3. **看颜色变量**：全部用 CSS 变量（`var(--accent)` / `var(--text-muted)` / `var(--border-dim)` / `var(--bg-hover)` 等），禁止硬编码色值或 inline style 写死颜色。

### 常见 UI 违规：

| 违规行为 | 正确做法 |
|----------|----------|
| 按钮放在文字行内（flex items-center gap-2） | 操作按钮独立一行，全宽、虚线边框、`border-color: var(--border-dim)` |
| 自己写 flex-col + overflow hack 固定顶底 | 用 ModalShell 的 `#header` / `#footer` 插槽 |
| `<span>` 标签不可点击 | 用 `<button>` + `hover:underline` + `var(--accent)` 色 |
| 新增 UI 不参考已有同类界面 | Grep 搜索同类组件 → 复制其模板结构和 class 模式 |
| 修改 ModalShell 默认行为（如加 `bodyScroll` prop） | 优先用插槽解决问题，不要为单个使用场景加全局 prop |

---

## 工程规范

> 继承自全局 CLAUDE.md，以下为 cc-gui 项目特定补充。

### 0. 动手前必做
1. **先查复用**: Grep 搜索是否已有类似组件/composable/util，优先复用而非重写
2. **先查 skill**: 检查可用 skill（frontend-design 等）
3. **先看文档**: 以官方文档为准，不猜 API 行为
4. **写完后对照 spec 逐条验证，再回复用户**
5. **多步骤操作前**：检查 `docs/` 目录是否有相关计划文档，将实施计划写入文件再执行

### 0. 阶段性完成后必做（无需用户提醒）

每完成一个功能/修复的阶段性工作后，**自动**执行以下收尾：

1. **补充测试**：新增/修改的功能是否有单元测试覆盖？检查相关 `.test.ts` 文件
2. **更新 CLAUDE.md**：设计决策、数据流、功能索引是否有变化？同步更新
3. **更新 docs/ 计划**：如果此前写了实施计划，标记已完成项
4. **git commit**：`git add -A && git commit -m "..."`（Conventional Commits 格式）

> 用户不需要每次打字提醒。这是自动执行的流程。

### 版本号管理

项目版本号**以 `package.json` 为准**，通过 Vite `define` 注入 `__APP_VERSION__` 到 UI（设置页 about 显示）。

**发版时需同步更新三处**（保持三者一致）：

| 文件 | 字段 | 说明 |
|------|------|------|
| `package.json` | `version` | **源头** — Vite 构建时注入 UI |
| `src-tauri/tauri.conf.json` | `version` | 安装包版本号（MSI/NSIS） |
| `src-tauri/Cargo.toml` | `version` | Rust 二进制版本号 |

```bash
# 发版前执行
grep '"version"' package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml
```

### 1. Vue 3 组件规范
- **单文件组件**: 每个 `.vue` 文件只负责一件事，超过 300 行考虑拆分
- **Props down, Events up**: 数据单向流动，子组件不修改 props
- **Composables 复用**: 有状态逻辑抽成 `useXxx()`，无状态工具函数放 `lib/utils.ts`
- **defineModel 禁止**: 用 `defineProps` + `defineEmits` 保持数据流清晰
- **模板简洁**: 复杂表达式提取为 computed
- **Tauri 调用**: 统一通过 `lib/tauri-bridge.ts` 封装，不在组件中直接 `invoke()`

### 2. TypeScript 规范
- **strict 模式**: 所有类型标注完整，不用 `any`
- **interface 优先**: 对象类型用 `interface`，联合类型用 `type`
- **错误处理**: Promise 调用必须 `.catch()` 或 try/catch
- **测试 mock**: `tauri-mock.ts` 覆盖所有用到的 Tauri API

### 3. Rust 后端规范
- **禁止 panic**: 不允许 `.unwrap()` / `.expect()` 在生产代码中（`run()` 入口和 `#[cfg(test)]` 除外）
- **Result<T, String>**: 所有 Tauri commands 返回 Result，错误信息清晰
- **模块组织**: 一个文件一个职责（process / protocol / session / db / lib）
- **导入顺序**: std → 第三方 → crate 内部
- **禁止 unsafe**: 除非有明确的 Windows 平台兼容需求

### 4. 项目特定约定
- **文档文件名用中文**：`docs/` 下新增的文件一律用中文命名（如 `架构穿透文档.md`），不用英文
- **会话管理**: 前端 ourId (UUID v4) ↔ 后端 claude_session_id (CLI 返回)，映射关系存储在 `sessions.cli_session_id`
- **NDJSON 协议**: 遵守 Claude Code CLI 的 stream-json 输出格式，始终使用 `--include-partial-messages` 获取增量 token
- **权限模式映射**: `auto` → 写 settings.json `permissions.defaultMode`；其他 → CLI `--permission-mode` 标志
- **i18n**: 所有面向用户的字符串必须中英双语，新增文案同时添加到 `zh.json` 和 `en.json`
- **测试**: 核心逻辑有 vitest 覆盖，关键交互有 E2E 覆盖，新增功能必须补测试

### 5. Git 提交规范
- **Conventional Commits**: `feat(模块): 描述`、`fix(模块): 描述`
- **模块标签**: chat, files, session, settings, rust, test, docs

---

## 关键设计决策

1. **三线程进程模型**: 参考 TOKENICODE 架构，Waiter 管理进程生命周期 + Stdout Reader 解析流 + Stderr 转发错误
2. **SQLite 持久化**: rusqlite bundled 模式，WAL journal，4 表 schema
3. **DeepSeek 兼容**: 前端设置面板配置 API Key/Base URL/Model，连接测试走 DeepSeek `/v1/chat/completions`
4. **不传 --model 给 CLI**: Claude CLI 使用自己的 `~/.claude/settings.json` 配置模型；GUI 的 model 设置存储在 session 记录中供参考
5. **权限同步**: `sync_permission_settings()` 在每次 spawn 前将 auto/default 模式写入 `~/.claude/settings.json`
6. **Mock 优先测试**: `tauri-mock.ts` 提供完整 mock，Playwright 测试在浏览器中运行无需 Tauri
7. **用户消息单写**: 用户消息只由 Rust 后端 `send_message` 保存，前端不再重复保存，避免历史回显双份
8. **亮色主题 CSS 顺序**: `[data-theme="light"]` 必须在 `:root` 之后，禁止 `@import` 独立亮色 CSS 文件（会被提升到 `:root` 前导致变量被覆盖）
9. **ModalShell 统一样式**: 所有弹窗通过 `ModalShell.vue` 外壳实现，颜色用 CSS 变量，不写 inline style 或硬编码色值
10. **拼音搜索**: 命令面板支持拼音首字母搜索，`lib/pinyin.ts` 覆盖 3755 个常用汉字
11. **ManagePanel 8合1管理**: `ManagePanel.vue` 统一管理 8 个 Tab（plugins/mcp/skills/agents/hooks/memory/permissions/styles），通过 Tab 切换时清空数据并重新加载，避免 8 个独立弹窗
12. **MCP 配置多源扫描**: `loadMCP()` 扫描 `~/.claude.json`（官方用户级 MCP 配置）→ `~/.claude/.mcp.json` → 项目根 `.mcp.json` → **已启用**插件的 `.mcp.json`（先读 `enabledPlugins` 过滤）→ 运行时连接状态。描述由 AI 生成并缓存到 `item_descriptions` 表。注意：`~/.claude/settings.json` 的 `mcpServers` 字段**会被 CLI 静默忽略**，不要扫它
13. **文件写入安全域**: Rust `write_file` 命令只允许写入 `~/.claude/` 子树，拒绝越界路径
14. **可扩展命令注册**: `useCommandRegistry` 允许各组件动态注册命令面板命令，`CommandPalette.vue` 启动时收集所有注册命令
15. **流事件去重**: `useStreamProcessor` 对 `stream-event` 按 sessionId + content 前缀去重，防止 CLI 重复输出
16. **OpenAI 兼容端点 URL 规范化**: 所有调 `/v1/chat/completions`（翻译、描述生成、连接测试）的地方必须先 `trim_end_matches("/anthropic").trim_end_matches("/v1")` 再拼接 URL。因为用户可能配置 `ANTHROPIC_BASE_URL=https://api.deepseek.com/anthropic`（Anthropic 格式），直接拼 `/v1/chat/completions` 会 404。Rust 端统一用 `fn openai_base(base_url)` 处理。
17. **翻译和描述生成模型**: 翻译、MCP 描述生成等轻量 API 调用使用 `CLAUDE_CODE_SUBAGENT_MODEL` 环境变量（fallback `deepseek-chat`），不要硬编码模型名。Rust 端统一用 `fn subagent_model()` 读取。
18. **ModalShell 三段式布局**: header（`#header` 插槽 + 关闭按钮）/ body（default slot，`overflow-y-auto`）/ footer（`#footer` 插槽，`v-if="$slots.footer"`）。子组件用插槽填充各段，不要自己在 default slot 里造 flex 三段式。
19. **Skills/Agents 管理多源扫描**: `loadSkills()` 和 `loadAgents()` 同时扫描自定义目录（`~/.claude/skills/` / `agents/`）和已启用插件的缓存目录（`~/.claude/plugins/cache/<mkt>/<plug>/<ver>/`），按来源分组显示。Skills 点击直接执行（`/skill-name`）而非打开编辑。
20. **Agent 使用状态追踪**: chat store 维护 `usedAgents: Set<string>`，`addToolUse()` 检测 Agent/Task 工具调用并提取 `subagent_type`。ManagePanel 的 Agent Tab 根据 Set 标记绿点（用过）或灰点（未用），用过的排前面。`clearMessages()` 时重置。
21. **Hook 管理 schema 适配**: ManagePanel 的 Hooks Tab 适配 CC 新版 Hook schema（`事件 → [matcher → {hooks: [{type, command}]}]`），不再假设扁平结构。
22. **SessionStart Hook 行为规则注入**: 全局 CLAUDE.md 的行为规则移至 `~/.claude/hooks/behavioral-rules.md`，由 PowerShell 脚本在 SessionStart 时注入。Hook 注入无 "may not be relevant" 免责，CLAUDE.md 保留骨架版用于 `/compact` 后存活。
23. **Ponytail 精简模式 GUI**: 工具栏新增 Ponytail 下拉（同权限/深度样式），选中直接发送 `/ponytail <mode>`。Setting 面板同思考深度样式下拉设默认值。状态持久化到 localStorage。
24. **字号设置**: 支持小/中/大（14/18/22px），通过 `data-font-size` 属性 + `html { font-size }` CSS 实现全局 rem 缩放。ModalShell 宽度用 rem 适配。ManagePanel 内 `text-[*px]` 全部转为 `text-[*rem]` 以跟随字号。
25. **NDJSON 双工协议**: CC 子进程以 `--output-format stream-json --input-format stream-json --permission-prompt-tool stdio` 启动（不带 `--print`，两者互斥）。用户消息通过 stdin 写入 NDJSON `{"type":"user","message":{...}}`。初始化后发 `set_permission_mode` 控制请求告知 CC 权限模式。
26. **control_request/response 审批流**: CC 通过 stdout 发 `control_request(can_use_tool/hook_callback)` → `protocol.rs` 提取 `request_id`、`tool_name`、`tool_input` → 前端 `ChatPanel.vue` 渲染审批栏（通用工具）或问答弹窗（`tool_name === "AskUserQuestion"`）→ 用户操作后构造 `control_response(subtype:"success",request_id,response:{behavior,updatedInput/message})` 写入 stdin。`hook_callback` 不自动回复，走统一的用户审批流程。
27. **AskUserQuestion 问答弹窗**: 识别 `tool_name === "AskUserQuestion"` 时不显示通用审批栏，弹出 `ModalShell` 内 radio/checkbox 选项组 + Other 自由输入。答案格式 `updatedInput:{questions,answers}`，answers key 精确匹配 question 原文，value 为 label 或 label[]。
28. **interrupt 优雅停止**: 用户点击停止→先发 `control_request(subtype:"interrupt")` 到 stdin，等 3 秒让 CC 优雅退出，超时才 `pm.kill()`。
29. **思考计时分段**: 每段 tool_use 前独立计时（`thinkingDurationMs`），总耗时 = 各段求和。去掉了实时 `setInterval` 计时器，不受审批暂停影响。
30. **工具名 i18n**: `tools.Bash`→`命令行`、`tools.Write`→`写入文件` 等，`zh.json`/`en.json` 双份。`ChatPanel.vue` 和 `MessageBubble.vue` 共用 `toolLabel()` 映射。
