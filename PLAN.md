# cc-gui — Claude Code Desktop GUI

> Tauri 2 + Vue 3 构建的 Claude Code 桌面 GUI，DeepSeek 后端，对标 TOKENICODE + CodePilot

---

## 技术栈

| 层 | 技术 |
|----|------|
| 桌面框架 | Tauri 2 |
| 前端 | Vue 3 + TypeScript + Pinia + vue-i18n + Vue Router |
| CSS | Tailwind CSS 4 + daisyUI 5 |
| 后端 | Rust (tokio, serde, rusqlite, reqwest, dirs, uuid) |
| 持久化 | SQLite (rusqlite bundled mode) |
| 测试 | vitest + @vue/test-utils + Playwright + cargo test |
| 代码高亮 | highlight.js |

---

## 目录结构

```
cc-gui/
├── src/                              # Vue 3 前端
│   ├── main.ts                       # 入口
│   ├── App.vue                       # 根组件
│   ├── router/index.ts               # 路由 (/chat, /settings)
│   ├── assets/main.css               # CSS 变量 + Tailwind + daisyUI
│   ├── stores/
│   │   ├── chat.ts                   # 消息流、流处理、control_request、Token 统计
│   │   ├── session.ts                # 会话 CRUD（对接 Rust SQLite）
│   │   └── settings.ts               # API 配置、主题、语言、权限模式、effort
│   ├── components/
│   │   ├── layout/AppShell.vue       # Navbar + 可折叠侧边栏 + Ctrl+K 命令面板
│   │   ├── session/SessionSidebar.vue # 会话列表、新建、删除、重命名
│   │   ├── chat/
│   │   │   ├── ChatPanel.vue         # 消息区、欢迎页、Permission bar、Debug、导出
│   │   │   ├── InputBar.vue          # SVG 图标按钮 + auto-resize textarea
│   │   │   ├── MessageBubble.vue     # 头像、思考折叠、工具卡片、Markdown、编辑/重发
│   │   │   ├── ModeBar.vue           # 权限模式 + 思考深度切换
│   │   │   └── ThinkingIndicator.vue # 思考中动画指示器
│   │   ├── settings/SettingsPanel.vue # API 配置、连接测试、批准场景管理
│   │   ├── files/
│   │   │   ├── FilePanel.vue         # 文件浏览器（可折叠、拖拽宽度、面包屑）
│   │   │   ├── FileTree.vue          # 递归目录树（展开/折叠、右键菜单）
│   │   │   ├── FilePreview.vue       # CodeMirror 6 只读预览 (12 种语言)
│   │   │   └── DiffViewer.vue        # 代码 Diff 查看器（三色标记）
│   │   └── shared/
│   │       ├── MarkdownRenderer.vue  # Markdown → HTML + highlight.js + Mermaid
│   │       ├── MermaidRenderer.vue   # Mermaid 图表渲染 (SVG)
│   │       └── CommandPalette.vue    # Ctrl+K 命令面板 (8 个操作)
│   ├── composables/
│   │   ├── useStreamProcessor.ts     # Tauri 事件监听 (stream + control_request)
│   │   ├── useHighlight.ts           # highlight.js 封装 (15 种语言)
│   │   └── useDebugLog.ts            # Debug 面板
│   └── lib/
│       ├── tauri-bridge.ts           # IPC 调用 + 类型定义
│       └── tauri-mock.ts             # E2E 测试 Tauri API mock
│
├── src-tauri/                        # Rust 后端
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                   # 入口
│       ├── lib.rs                    # 17 Tauri commands
│       ├── process.rs                # ProcessManager + StdinManager + 三线程 spawn
│       ├── protocol.rs               # NDJSON 解析 + capture_session_id
│       ├── session.rs                # SQLite CRUD + approved_scenarios
│       └── db.rs                     # SQLite schema + migration
│   └── tests/
│       ├── protocol_test.rs          # 9 tests
│       ├── integration_test.rs       # spawn 真 claude
│       └── resume_test.rs            # --resume 多轮上下文
│
├── e2e/                              # Playwright E2E
│   ├── chat-flow.spec.ts             # 交互流程
│   ├── visual.spec.ts                # 截图对比
│   └── real-stream.spec.ts           # 真 stream-json 回放
│
├── scripts/
│   ├── dev.sh / dev.bat / dev.ps1    # 一键启动
│   └── record_claude_output.sh       # 录制真 claude 输出
│
├── .claude/settings.local.json       # PostToolUse hook: 编辑后跑 vitest
├── package.json
├── vite.config.ts / vite.config.e2e.ts
├── vitest.config.ts
├── playwright.config.ts
└── PLAN.md
```

---

## 通信协议

Claude Code CLI 每行一个 NDJSON 事件：

| 事件 | 用途 | 处理 |
|------|------|------|
| `system/init` | session_id, 工具列表, 模型 | Rust → emit `session-created` → 前端存储 |
| `system/thinking_tokens` | 思考进度 | 跳过 |
| `assistant` | content[text, thinking, tool_use] | Rust → emit `stream-event` → 前端渲染 |
| `user` | tool_result | emit |
| `result` | 耗时, Token, 费用 | emit → 前端 token 统计 + 消息持久化 |
| `control_request` | 权限请求 | Rust → emit → Permission bar → stdin 回复 |

CLI 启动参数：

```bash
claude \
  --print \
  --output-format stream-json \
  --verbose \
  --permission-mode <plan|acceptEdits|bypassPermissions|default> \
  --effort <low|medium|high|xhigh> \
  --max-turns 10 \
  --resume <session_uuid> \
  --add-dir ~/.claude \
  "message"
```

---

## 权限模式

| UI 模式 | CLI `--permission-mode` | `settings.json` 操作 |
|---------|------------------------|---------------------|
| 计划模式 | `plan` | — |
| 自动模式 | `default` | 写 `permissions.defaultMode: "auto"`；切出时还原 `"default"` |
| 编辑前询问 | `default` | — |
| 编辑自动 | `acceptEdits` | — |
| 完全放行 | `bypassPermissions` | — |

`auto` 不是 CLI flag，而是 `~/.claude/settings.json` 中 `permissions.defaultMode` 的值。双向同步由 `process.rs::sync_permission_settings()` 实现。

SCP 权限交互：
```
CLI → control_request → Rust → Tauri event → Permission bar → Allow/Deny
→ invoke("send_stdin") → StdinManager → CLI stdin → 继续执行
```

自动批准：`approved_scenarios` 表 (tool_name, pattern)，匹配的工具自动放行。

---

## Effort Level

| UI 选项 | 映射 |
|---------|------|
| low / medium / high / xhigh | `--effort <value>` 直接传递 |
| max | `--effort xhigh`（目前最高可用档位） |
| ultracode | `--effort xhigh` + `--settings '{"ultracode":true}'` + Workflow 编排 |

---

## 会话生命周期

```
用户发消息
    ↓ invoke("send_message", { sessionId, message, planMode, autoMode, ... })
Rust sync_permission_settings()   ← 自动模式写 settings.json
    ↓ spawn_claude_session() — 三线程
    ├─ Waiter: owns child, tokio::select! { wait │ kill_rx }
    ├─ Stdout Reader: BufReader→NDJSON→emit events
    └─ Stderr Reader: emit errors
    ↓
前端 useStreamProcessor:
  ├─ stream-event → Pinia chat store
  ├─ session-created → 存储 UUID + 回写 Rust + 前端 session
  ├─ control_request → Permission bar
  ├─ process-exited → 结束处理
  └─ result/done → save message to SQLite + token 统计
    ↓
用户继续输入 → 循环（第二条自动 --resume）
```

---

## SQLite Schema

```sql
CREATE TABLE sessions (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL DEFAULT 'New Chat',
  cli_session_id TEXT,
  cwd TEXT NOT NULL,
  model TEXT,
  status TEXT DEFAULT 'idle',
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE messages (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
  role TEXT CHECK(role IN ('user','assistant','system')),
  content TEXT NOT NULL,          -- JSON blob: text + thinking + toolUses + tokens
  token_usage TEXT,
  created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);

CREATE TABLE approved_scenarios (
  tool_name TEXT NOT NULL,
  pattern TEXT NOT NULL,
  created_at TEXT DEFAULT (datetime('now')),
  PRIMARY KEY (tool_name, pattern)
);
```

---

## 测试体系

```
Vitest     132 tests   9 files   stores + 组件 + FileTree + edit/resend/export + SessionSidebar + theme + drag-drop
Playwright  16 tests   3 files   E2E 交互 + 截图 + 真 stream 回放
Cargo       12 tests   3 files   协议解析 + 集成 + 多轮
─────────────────────────────────────────
Total      148 tests

新增模块（Phase 3）:
- Rust: `list_dir`, `read_file_content`, `get_workspace_root`, `reveal_in_explorer` (4 commands)
- Vue: FilePanel, FileTree, FilePreview, DiffViewer (4 components)
- Dependencies: codemirror 6, @codemirror/lang-*, @codemirror/theme-one-dark, diff
```

| 命令 | 内容 | 耗时 |
|------|------|------|
| `npm test` | vitest（PostToolUse hook 自动触发） | 3s |
| `npm run test:e2e` | Playwright 全量 | 15s |
| `npm run test:rust` | cargo test | 10s |
| `npm run test:e2e:real` | 录制 claude → 前端回放 | 15s |
| `npm run test:rust:real` | spawn claude ×2 → 验证 --resume | 10s |
| `npm run test:all` | 全部 | 30s |

---

## 分阶段进度

### Phase 1：脚手架 + 最小可用 ✅

**原计划：**
- [x] `npm create tauri-app` 初始化
- [x] Vue 3 + Tailwind + Pinia + vue-i18n 配置
- [x] AppShell 单窗口布局
- [x] 前端消息气泡 + 输入框 + Markdown 渲染
- [x] 前端 stores（chat, session, settings）+ 单元测试
- [x] Rust 壳（main.rs, lib.rs, protocol.rs, session.rs）
- [x] Rust spawn_claude() + stdout 转发前端
- [x] 前端解析 stream-json，渲染 thinking / text
- [x] DeepSeek API 配置 + 连接测试

**实际额外完成：**
- [x] daisyUI 5 + 自定义 CSS 变量设计系统（`--bg-root`, `--accent`, `--amber` 等）
- [x] highlight.js 代码高亮（15 种语言）
- [x] 三线程 spawn 模型（Waiter + Stdout Reader + Stderr Reader）
- [x] NDJSON 解析 + Tauri event emit + capture_session_id
- [x] Windows 兼容：`find_claude()` + `creation_flags` + `--add-dir`
- [x] E2E 测试框架：vitest + Playwright + Tauri mock + record/replay
- [x] 一键启动脚本（dev.bat / dev.ps1 / dev.sh）

### Phase 2：工具可视化 + 多轮对话 ✅

**原计划：**
- [x] 工具调用卡片（Bash/Read/Write/Edit 可折叠展示）
- [x] 工具结果带语法高亮（复用 MarkdownRenderer）
- [x] 思考过程（thinking）默认折叠
- [x] `--resume` 多轮对话持久化：Rust SQLite 存 cli_session_id
- [x] SQLite 初始化 + migration（参考 CodePilot db.ts）
- [x] 会话标题自动生成（首条消息截取）

**实际额外完成：**
- [x] 多会话管理：SessionSidebar 侧边栏（新建、删除、重命名、当前高亮）
- [x] 5 种权限模式：plan / auto / default / acceptEdits / bypassPermissions
- [x] SCP 权限交互：control_request → Permission bar → Allow/Deny → stdin 回复
- [x] 自动批准：approved_scenarios 表 + SettingsPanel 管理 UI
- [x] ModeBar 组件（权限模式 + effort 切换）
- [x] Effort 映射：low/medium/high/xhigh/max/ultracode
- [x] auto 模式双向 settings.json 同步（sync_permission_settings）
- [x] SettingsPanel：API 配置、连接测试、批准场景管理
- [x] Vue Router 路由（/chat, /settings）
- [x] Thinking 计时器 + Token 统计
- [x] 消息持久化：saveMessage JSON blob（text + thinking + toolUses + tokens）
- [x] loadMessages 从 SQLite 恢复完整会话
- [x] PostToolUse hook：每次编辑后自动跑 vitest
- [x] E2E 真 stream-json 回放测试（record_claude_output.sh → real-stream.spec.ts）
- [x] 真实多轮上下文集成测试（resume_test.rs）
- [x] 测试体系：95 vitest + 16 playwright + 12 cargo = 123 tests

### Phase 3：文件集成 ✅ 已完成

- [x] 文件浏览器面板（可折叠，抽屉式拉手 `FILES` 标签）
- [x] 目录树（文件类型图标、文件大小、文件夹优先排序）
- [x] 面包屑导航（分段可点击、返回上一级 ⌂、回到工作目录根 ⌂）
- [x] 工作目录显示（header 等宽字体 + 强调色、点击打开文件面板）
- [x] CodeMirror 6 文件预览（只读、one-dark 主题、12 种语言）
- [x] 代码 Diff 查看器（增/删/不变三色标记、行号、diff 库）
- [x] Rust file ops（`list_dir`, `read_file_content`, `get_workspace_root`, `reveal_in_explorer`）
- [x] 右键上下文菜单（Reveal in Explorer / Copy Path / Copy Name）
- [x] 深层目录递归展开（▶ 展开/▼ 折叠、缩进竖线、加载动画）
- [x] 拖拽调整面板宽度（细条手柄、200px ~ 600px）

### Phase 4：体验优化 ✅ 已完成

- [x] Mermaid 图表渲染（flowchart, sequence, gantt, ER, pie 等全部类型）
- [x] Token 用量 + 费用 UI（⏱s ↑in ↓out $cost 显示在每条 assistant 消息底部）
- [x] 命令面板 (Ctrl+K) — 搜索过滤、键盘导航、模式切换、会话操作
- [x] 消息编辑/重发功能 — 内联编辑 textarea、Save & Resend 按钮、截断后续消息、单按钮 Resend
- [x] 会话导出 — Markdown 格式导出（含 thinking、tool_use、token 统计）、blob URL 下载
- [x] 用户消息操作按钮：复制、编辑(✏️)、重发(↻)

### Phase 5：打包发布 ✅ 已完成

- [x] Windows .msi (11 MB) — WiX Toolset，自动下载依赖
- [x] Windows .exe (7.5 MB) — NSIS 安装包，单文件安装
- [x] Tauri bundle 配置 — NSIS（中英文）、窗口最小尺寸 800×600
- [x] CI/CD — GitHub Actions: `ci.yml` (PR 测试+编译检查) + `release.yml` (tag 推送 → Windows 构建)
- [x] npm scripts — `build:tauri`, `build:tauri:msi`, `build:tauri:nsis`
- [ ] macOS .dmg / Linux .AppImage — 待定
- [ ] 自动更新 / 代码签名 — 待定

### Phase 6：完善打磨 ❌

#### A. 功能增强

##### A1. 会话搜索 ✅
- **目标**：SessionSidebar 顶部增加搜索输入框，按标题实时过滤会话列表
- **实现**：
  - `SessionSidebar.vue`：搜索 input + `filteredSessions` computed
  - 输入时实时过滤，匹配标题（大小写不敏感），无匹配显示 "No matching sessions"
  - 清除按钮一键重置搜索
  - 不影响新建/删除/重命名等现有操作
- **文件**：`src/components/session/SessionSidebar.vue`，`src/components/session/SessionSidebar.test.ts`(新)
- **测试**：搜索过滤、空状态显示、清除按钮、恢复全部列表（7 tests）

##### A2. 主题切换 ✅
- **目标**：支持深色/浅色模式切换，默认 dark
- **实现**：
  - `src/assets/theme-light.css`(新)：`[data-theme="light"]` CSS 变量覆写
  - `src/App.vue`：根元素绑定 `data-theme` 属性，watch 动态切换
  - `AppShell.vue`：header 新增 sun/moon 图标切换按钮
  - `CommandPalette.vue`：新增 theme-dark / theme-light 操作
  - `settings.ts`：theme 持久化 localStorage
- **文件**：`src/assets/theme-light.css`(新), `src/App.vue`, `src/components/layout/AppShell.vue`, `src/components/shared/CommandPalette.vue`
- **测试**：theme toggle、localStorage 持久化（2 tests）

##### A3. 全局快捷键 ✅
- **目标**：快捷键全局可用（无需打开命令面板）
- **绑定**：`Ctrl+N` 新建 · `Ctrl+B` 侧边栏 · `Ctrl+E` 文件面板 · `Ctrl+,` 设置
- **实现**：`AppShell.vue` 全局 keydown listener，input/textarea 聚焦时跳过
- **文件**：`src/components/layout/AppShell.vue`
- **注意**：Ctrl+K 仍由 CommandPalette 独立处理，不与全局冲突

##### A4. 拖拽文件到输入框 ✅
- **目标**：从系统拖拽文件到输入框，自动插入文件路径
- **实现**：`InputBar.vue` → @dragover/@dragleave/@drop，虚线高亮边框，支持多文件
- **文件**：`src/components/chat/InputBar.vue`
- **测试**：拖拽高亮、文件路径插入（3 tests）

##### A5. InputBar 工具栏 + 命令菜单增强 ✅（对标 VS Code 扩展）
- **目标**：在 InputBar 上方新增工具栏，左侧放置附加文件按钮 + 命令菜单按钮，对齐 VS Code 扩展体验
- **布局**：
  ```
  ┌──────────────────────────────────────────────────────────────────────┐
  │ [📎] [☰] [模式: 计划模式 ▼] [努力: high ▼]    [▂▃▅▇ 45.6K/200K] │
  └──────────────────────────────────────────────────────────────────────┘
  ```
- **工具栏按钮（从左到右）**：
  | 按钮 | 功能 |
  |------|------|
  | 📎 附加文件 | 打开文件选择器，将选中文件路径插入输入框（支持多个） |
  | ☰ 命令菜单 | 打开 Ctrl+K 命令面板（与快捷键等价），含更多操作 |
  | 权限模式选择器 | 下拉切换（计划模式/自动模式/编辑前询问/编辑自动/完全放行），显示当前模式名 |
  | Effort 选择器 | 下拉切换思考深度（low/medium/high/xhigh/max/ultracode），显示当前值 |
  | 上下文指示器 | 右侧显示 token 用量（见 A6） |
- **注意**：权限模式选择器从现有 `ModeBar.vue` 提取并移到工具栏，原 ModeBar 移除（功能已被工具栏 + CommandPalette 覆盖）
- **命令面板扩展**（☰ 按钮或 Ctrl+K 打开）：
  - 新增：attach-file（附加文件）、show-usage（查看用量）、compact（压缩上下文）
  - 保留现有 8 个操作（new-session, toggle-sidebar, toggle-files, settings, 4 个权限模式）
- **实现**：
  - `InputBarToolbar.vue`(新)：工具栏（📎Attach + ☰Menu + 权限模式▼ + 努力▼）
  - `useCommandPalette.ts`(新)：轻量事件总线，解耦菜单按钮 ↔ CommandPalette
  - `ChatPanel.vue`：ModeBar 替换为 InputBarToolbar，handleAttachFile
  - `CommandPalette.vue`：新增 attach-file/show-usage/compact + watch 总线打开
  - `AppShell.vue`：处理新增命令
  - ModeBar.vue 保留但不渲染（后续移除）
- **文件**：`InputBarToolbar.vue`(新), `useCommandPalette.ts`(新), `ChatPanel.vue`, `CommandPalette.vue`, `AppShell.vue`

##### A6. 上下文指示器 ✅
- **目标**：工具栏右侧显示 context window 使用情况
- **实现**：`ContextIndicator.vue`(新) → 分段进度条 + 百分比，模型容量映射，绿→黄→红阈值
- **文件**：`src/components/chat/ContextIndicator.vue`(新), `InputBarToolbar.vue`

#### B. 质量加固

##### B1. Rust unwrap 消除 ✅
- 5 处 unwrap/expect 已消除或改为 proper error：
  - `process.rs:203`：`serde_json::to_string_pretty().unwrap()` → `map_err`
  - `db.rs:14`：`data_dir().expect()` → `unwrap_or_else` fallback to temp dir
  - `db.rs:26`：`create_dir_all().expect()` → `map_err` → SqliteResult
  - `lib.rs:343`：`Db::new().expect()` → `unwrap_or_else` with eprintln + exit
  - `lib.rs:380`：Tauri builder expect 保留（入口点必须成功）

##### B2. 按需加载 ✅
- MermaidRenderer → `defineAsyncComponent(() => import(...))`

##### B3. 前端错误边界 ✅
- `ErrorBoundary.vue`(新)：`onErrorCaptured` + 重试按钮
- ChatPanel / FilePanel / SettingsPanel 已包裹

#### C. 体验打磨

##### C1. 消息过渡动画 ✅
- Vue `<TransitionGroup name="msg">` + CSS slide-up/fade-in 动画

##### C2. 系统通知 ✅
- Web Notification API：AI 回答完成时弹出通知（耗时 + token）

##### C3. 会话统计信息 ✅
- Rust list_sessions 聚合 JSON content 中的 token/cost
- SessionSidebar 每个会话显示 token 总量 + 费用

### Phase 6：完善打磨 ✅ 已完成（2026-06-10）

全部 10 项已完成，详见各分节。

### 后续 · 2026-06-11（文档与深度集成）🔄 进行中

- [ ] CommandPalette 升级至 40+ 命令、6 大分类
- [ ] ContextIndicator 上下文窗口用量可视化
- [ ] useStreamProcessor 增强（事件回放、通知分级）
- [ ] Light 主题完善
- [ ] i18n 补全（91 行新条目）
- [ ] Settings Token 持久化

> 📖 完整开发时间线见 [README.md § 开发时间线](README.md#开发时间线)

#### 实施顺序

```
第一轮（功能增强）：
  A1 会话搜索 → A2 主题切换 → A3 全局快捷键 → A4 拖拽文件
  → A5 命令菜单增强 → A6 上下文指示器

第二轮（质量加固）：
  B1 Rust unwrap → B2 代码分包 → B3 错误边界

第三轮（体验打磨）：
  C1 消息动画 → C2 系统通知 → C3 会话统计
```

---

## 注意事项

### 模型名称
- `--model` 只传 Anthropic 模型名（如 `claude-sonnet-4-6`）
- 前端 settings 中的模型名（如 `deepseek-v4-pro[1M]`）仅存储参考，不传给 CLI
- CLI 通过 `~/.claude/settings.json` 自行配置模型

### 工作目录
- `detect_project_root()` 自动检测项目根目录
- 子进程通过 `.current_dir()` 设置正确的工作目录
- 不传 `--cwd` flag（仅 `claude agents` 子命令支持）

### Windows 兼容
- `find_claude()` 自动查找 `%APPDATA%/npm/claude.cmd`
- spawn 时 `creation_flags = 0x08000000` 防止控制台闪烁
- `--add-dir ~/.claude` 允许 CLI 读取用户全局配置

---

## 关键设计决策

1. **裸 CLI 而非 SDK**：Tauri Rust 直接 spawn，无需 Node sidecar
2. **SQLite 而非 localStorage**：Rust 端可靠，不受前端刷新影响
3. **auto 模式**：settings.json 而非 CLI flag（与 VS Code 扩展一致）
4. **process.rs 合并 stream/cli 模块**：避免过早拆分
5. **BypassModeMap 已移除**：权限切换通过 sync_permission_settings() 写 settings.json

---

## 参考项目

| 来源 | 借鉴 |
|------|------|
| TOKENICODE `lib.rs:1665` | 三线程进程模型 |
| TOKENICODE `claude_process.rs` | ProcessManager, StdinManager |
| TOKENICODE `protocol.rs` | control_request/control_response |
| CodePilot `db.ts` | SQLite schema |
| CodePilot `permission-registry.ts` | 自动批准逻辑 |
