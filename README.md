# cc-gui — Claude Code Desktop GUI

> Tauri 2 + Vue 3 桌面应用，为 [Claude Code CLI](https://docs.anthropic.com/en/docs/claude-code) 提供图形化界面。
>
> 支持 DeepSeek API 代理后端，兼容 Windows / macOS / Linux。

---

## 系统架构

```mermaid
flowchart TB
    subgraph FRONTEND["🖥️ Vue 3 前端"]
        direction LR
        UI["ChatPanel<br/>MessageBubble<br/>InputBar<br/>FilePanel<br/>SettingsPanel"]
        STORES["Pinia Stores<br/>chat · session · settings"]
        BRIDGE["tauri-bridge.ts<br/>IPC 封装层"]
        UI --> STORES --> BRIDGE
    end

    subgraph RUST["⚙️ Rust 后端 (Tauri 2)"]
        direction LR
        CMDS["lib.rs<br/>28 Tauri Commands"]
        PROCESS["process.rs<br/>三线程进程管理"]
        SESSION["session.rs<br/>会话管理"]
        DB["db.rs<br/>SQLite"]
        PROTOCOL["protocol.rs<br/>NDJSON 解析"]
        CMDS --> PROCESS
        CMDS --> SESSION
        CMDS --> PROTOCOL
        SESSION --> DB
    end

    subgraph CLI["🤖 Claude Code CLI"]
        CLAUDE["claude --print<br/>--output-format stream-json<br/>--include-partial-messages"]
    end

    subgraph EXTERNAL["🌐 外部服务"]
        API["DeepSeek API<br/>/v1/chat/completions"]
    end

    FRONTEND <-->|"Tauri IPC<br/>invoke() ↔ emit()"| RUST
    PROCESS -->|"spawn subprocess<br/>stdin/stdout/stderr"| CLI
    CLI -->|"HTTP stream-json"| API
    SESSION -->|"reqwest<br/>连接测试"| API
```

---

## 核心业务流程

### 用户消息完整生命周期

```mermaid
sequenceDiagram
    actor User as 👤 用户
    participant Input as InputBar.vue
    participant Store as Pinia chatStore
    participant Bridge as tauri-bridge.ts
    participant Tauri as Tauri IPC
    participant Rust as Rust lib.rs
    participant PM as process.rs
    participant CLI as Claude CLI
    participant Proto as protocol.rs
    participant Stream as useStreamProcessor

    User->>Input: 输入消息 + 设置模式/参数
    Input->>Store: chatStore.sendMessage(msg, opts)
    Store->>Bridge: sendMessage(sessionId, message, planMode, ...)
    Bridge->>Tauri: invoke("send_message", params)
    Tauri->>Rust: send_message command

    Rust->>Rust: 保存 user message → SQLite
    Rust->>Rust: auto_title_from_first_message()
    Rust->>Rust: sync_permission_settings()
    Rust->>PM: spawn_claude_session(params)

    PM->>PM: find_claude() 定位 CLI 可执行文件
    PM->>PM: 构建 CLI 参数<br/>--print --stream-json --verbose<br/>--include-partial-messages<br/>--permission-mode --effort
    PM->>CLI: spawn 子进程 (stdin/stdout/stderr)

    par 三线程并行
        CLI-->>PM: wait() 进程退出
    and
        loop 流式输出
            CLI-->>PM: stdout NDJSON lines
            PM-->>Proto: BufReader::lines
            Proto-->>Proto: StreamLine::parse()
            Proto-->>Proto: to_frontend_event()
            Proto-->>Tauri: emit("stream-event")
        end
    and
        CLI-->>PM: stderr lines
        PM-->>Tauri: emit("stream-error")
    end

    Tauri-->>Stream: listen("stream-event")
    Stream->>Store: appendText / appendThinking / addToolUse
    Store-->>User: 实时渲染消息气泡

    CLI-->>PM: process exit
    PM-->>Tauri: emit("process-exited")
    Stream->>Store: finishAssistantMessage()
    Store->>Bridge: saveMessage() → SQLite
    Stream-->>User: 🔔 桌面通知
```

---

## 三线程进程模型

```mermaid
flowchart TB
    SPAWN["spawn_claude_session()"]
    CHILD["Claude CLI 子进程<br/>pid + stdin + stdout + stderr"]

    SPAWN --> CHILD

    CHILD --> STDIN["stdin → StdinManager<br/>注册到 HashMap<br/>供后续权限响应写入"]
    CHILD --> STDOUT["stdout → BufReader"]
    CHILD --> STDERR["stderr → BufReader"]

    subgraph T1["🧵 Thread 1: Waiter"]
        WAIT["owns child process"]
        SELECT["tokio::select!"]
        KILL["kill_tx → child.kill()"]
        EXIT["child.wait() → exit status"]
        WAIT --> SELECT
        SELECT -->|"kill signal"| KILL
        SELECT -->|"natural exit"| EXIT
        KILL --> NOTIFY["exit_notify.notify_waiters()"]
        EXIT --> NOTIFY
        NOTIFY --> EMIT1["emit('process-exited')"]
    end

    subgraph T2["🧵 Thread 2: Stdout Reader"]
        READ["BufReader::lines()"]
        PARSE["StreamLine::parse()"]
        ROUTE["to_frontend_event()"]
        EMIT2["emit('stream-event')"]
        READ --> PARSE --> ROUTE --> EMIT2
    end

    subgraph T3["🧵 Thread 3: Stderr Reader"]
        READ_E["BufReader::lines()"]
        EMIT3["emit('stream-error')"]
        READ_E --> EMIT3
    end

    STDOUT --> T2
    STDERR --> T3
```

---

## NDJSON 协议解析

```mermaid
flowchart LR
    RAW["raw NDJSON line<br/>from CLI stdout"] --> PARSE{"serde_json::<br/>from_str()"}

    PARSE -->|"成功"| TAG{"inner['type']"}

    TAG -->|"system/init"| SID["capture_session_id()<br/>→ emit('session-created')"]
    TAG -->|"assistant"| ASST["提取 text + thinking<br/>+ tool_use blocks<br/>→ StreamFrontendEvent"]
    TAG -->|"user"| USER["提取 tool_result<br/>→ StreamFrontendEvent"]
    TAG -->|"result"| RESULT["提取 duration_ms<br/>+ cost_usd + usage<br/>→ is_final = true"]
    TAG -->|"stream_event"| DELTA["content_block_delta<br/>→ text_delta / input_json_delta"]
    TAG -->|"control_request"| CTRL["权限请求<br/>→ tool_name + tool_input"]

    PARSE -->|"失败"| ERR["emit('stream-error')"]

    SID --> EMIT["Tauri emit →<br/>useStreamProcessor.ts"]
    ASST --> EMIT
    USER --> EMIT
    RESULT --> EMIT
    DELTA --> EMIT
    CTRL --> EMIT
    ERR --> EMIT
```

---

## 前端事件处理流水线

```mermaid
flowchart TD
    LISTEN["useStreamProcessor.startListening()"]

    LISTEN --> EVENTS

    subgraph EVENTS["5 个 Tauri 事件监听器"]
        SE["stream-event<br/>主数据流"]
        SD["stream-debug<br/>原始 stdout"]
        SERR["stream-error<br/>stderr 错误"]
        SC["session-created<br/>CLI 会话 UUID"]
        PE["process-exited<br/>进程退出"]
    end

    SE --> SWITCH{"data.type ?"}

    SWITCH -->|"assistant"| APPEND["chatStore.appendText()<br/>chatStore.appendThinking()<br/>chatStore.addToolUse()"]
    SWITCH -->|"control_request"| CTRL["chatStore.addControlRequest()"]
    SWITCH -->|"result / done"| FINISH["saveMessage() → SQLite<br/>chatStore.finishAssistantMessage()<br/>notifyComplete()"]
    SWITCH -->|"error"| ERRF["chatStore.appendText('⚠️')<br/>chatStore.finishAssistantMessage()"]

    SD --> DEBUG["useDebugLog.add()"]
    SERR --> DEBUG
    SC --> LINK["sessionStore.setClaudeSessionId()<br/>storeClaudeSession() → Rust"]
    PE --> CHECK{"success ?"}
    CHECK -->|"false + processing"| FINISH2["chatStore.finishAssistantMessage()"]
    CHECK -->|"true"| DEBUG2["useDebugLog.add()"]
```

---

## 会话生命周期

```mermaid
stateDiagram-v2
    [*] --> Idle: createSession()
    Idle --> Running: sendMessage()
    Running --> Running: stream-event<br/>增量渲染 token
    Running --> Completed: result event<br/>is_final = true
    Running --> Error: stream-error<br/>或 process exit != 0
    Running --> Stopped: stopSession()<br/>用户主动停止
    Completed --> Running: sendMessage()<br/>同一会话继续对话
    Error --> Running: sendMessage()<br/>重试
    Stopped --> Running: sendMessage()<br/>继续
    Completed --> [*]: deleteSession()
    Error --> [*]: deleteSession()
    Stopped --> [*]: deleteSession()
    Idle --> [*]: deleteSession()

    note right of Running
        CLI 进程存活
        stdin 可写入 (权限响应)
        --resume 续接可用
    end note

    note right of Completed
        duration_ms, cost_usd,
        input_tokens, output_tokens
        已写入消息 JSON blob
    end note
```

---

## 权限模式映射

```mermaid
flowchart LR
    subgraph UI["🎛️ 前端 UI"]
        MODE["ModeBar.vue<br/>用户选择权限模式"]
    end

    subgraph MAP["映射逻辑"]
        SW{"plan_mode ?"}
        SW -->|"true"| PLAN["CLI --permission-mode plan"]
        SW -->|"false"| AUTO{"auto_mode ?"}
        AUTO -->|"true"| A1["① 写 settings.json<br/>permissions.defaultMode = 'auto'<br/>② CLI --permission-mode auto"]
        AUTO -->|"false"| OTHER["CLI --permission-mode<br/>{default | acceptEdits |<br/>bypassPermissions | dontAsk}"]
    end

    MODE --> SW
```

---

## 数据库 Schema

```mermaid
erDiagram
    sessions {
        TEXT id PK "UUID v4"
        TEXT title "首条消息截取 50 字符"
        TEXT cli_session_id "CLI 返回的会话 UUID，用于 --resume"
        TEXT cwd "项目工作目录"
        TEXT model "模型名称"
        TEXT status "idle | running | completed | error"
        TEXT created_at "datetime('now')"
        TEXT updated_at "datetime('now')"
    }

    messages {
        TEXT id PK "role-timestamp (u/a/s)"
        TEXT session_id FK "关联 sessions.id，CASCADE 删除"
        TEXT role "user | assistant | system"
        TEXT content "完整 stream-json JSON blob"
        TEXT token_usage "{} 或 token 统计 JSON"
        TEXT created_at "datetime('now')"
    }

    approved_scenarios {
        TEXT tool_name PK "工具名如 Bash"
        TEXT pattern PK "匹配模式，默认 *"
        TEXT created_at "datetime('now')"
    }

    settings {
        TEXT key PK
        TEXT value "任意字符串值"
    }

    sessions ||--o{ messages : "1:N CASCADE"
```

---

## 技术栈

| 层 | 技术 |
|---|------|
| **桌面框架** | [Tauri 2](https://v2.tauri.app/) |
| **前端** | [Vue 3](https://vuejs.org/) + [TypeScript](https://www.typescriptlang.org/) |
| **状态管理** | [Pinia](https://pinia.vuejs.org/) |
| **样式** | [Tailwind CSS 4](https://tailwindcss.com/) + [DaisyUI 5](https://daisyui.com/) |
| **路由** | [Vue Router 4](https://router.vuejs.org/) |
| **编辑器** | [CodeMirror 6](https://codemirror.net/) |
| **代码高亮** | [highlight.js](https://highlightjs.org/) |
| **图表渲染** | [Mermaid](https://mermaid.js.org/) |
| **国际化** | [vue-i18n](https://vue-i18n.intlify.dev/) |
| **后端** | [Rust](https://www.rust-lang.org/) (Tauri) + [tokio](https://tokio.rs/) |
| **数据库** | [SQLite](https://www.sqlite.org/) (rusqlite bundled, WAL mode) |
| **HTTP** | [reqwest](https://docs.rs/reqwest/) (rustls-tls) |
| **测试** | [Vitest](https://vitest.dev/) + [Playwright](https://playwright.dev/) + cargo test |

---

## 功能特性

- 🖥️ **完整 GUI 交互** — 聊天面板、消息气泡、Markdown 渲染、代码高亮、Mermaid 图表
- 🧠 **三线程流式处理** — Waiter / Stdout Reader / Stderr，实时增量 token 渲染
- 🔍 **命令面板** — Ctrl+K 快速搜索，6 大分组 40+ 命令，拼音首字母匹配，最近使用追踪
- 📊 **上下文用量弹窗** — 对标 `/context`，分类明细 tokens 用量 + 中英双文标注
- 📁 **文件面板** — 文件树浏览、代码预览、Diff 对比
- 💬 **会话管理** — 创建/删除/重命名/续接(`--resume`)，SQLite 持久化
- ⚙️ **设置面板** — API Key / Base URL / Model 配置 + 连接测试
- 🛡️ **权限控制** — 6 种权限模式 UI 切换，自动同步 `~/.claude/settings.json`
- 🌐 **i18n 国际化** — 中文 / 英文双语界面
- 🎨 **亮/暗双主题** — CSS 变量驱动，弹窗、面板、代码块全适配
- 🔔 **桌面通知** — 助手完成时系统通知（含耗时和 token 统计）
- ⚡ **Ultracode 支持** — 多代理自动编排

---

## 快速开始

### 前置要求

- [Node.js](https://nodejs.org/) ≥ 18
- [Rust](https://www.rust-lang.org/tools/install) ≥ 1.70
- [Claude Code CLI](https://www.npmjs.com/package/@anthropic-ai/claude-code) (npm 全局安装)
- Windows: [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- Linux: `libwebkit2gtk-4.1-dev` 等 Tauri 系统依赖

### 安装

```bash
git clone <repo-url> cc-gui
cd cc-gui
npm install
```

### 开发

```bash
# 方式 1：仅启动前端（浏览器调试）
npm run dev

# 方式 2：启动完整桌面应用
npm run dev:tauri
```

### 构建

```bash
# 生产构建
npm run build:tauri

# Windows MSI 安装包
npm run build:tauri:msi

# Windows NSIS 安装包
npm run build:tauri:nsis
```

构建产物位于 `src-tauri/target/release/bundle/`。

---

## 项目结构

```
cc-gui/
├── src/                       # Vue 3 前端源码
│   ├── components/
│   │   ├── chat/              # 聊天面板、消息气泡、输入栏、模式栏
│   │   ├── layout/            # 主布局容器
│   │   ├── session/           # 会话侧边栏
│   │   ├── files/             # 文件面板、文件树、预览、Diff
│   │   ├── settings/          # 设置面板
│   │   └── shared/            # Markdown渲染、Mermaid、命令面板、错误边界
│   ├── composables/           # useStreamProcessor, useFilePreview 等
│   ├── stores/                # Pinia 状态管理
│   ├── lib/                   # 工具函数、Tauri 桥接、测试 mock
│   ├── locales/               # 中英文语言包
│   └── assets/                # 样式
├── src-tauri/                 # Rust 后端
│   ├── src/
│   │   ├── main.rs            # 程序入口
│   │   ├── lib.rs             # 28 个 Tauri IPC 命令
│   │   ├── process.rs         # 三线程进程管理
│   │   ├── protocol.rs        # NDJSON 协议解析
│   │   ├── session.rs         # 会话管理 + API 测试
│   │   └── db.rs              # SQLite 数据库
│   └── tests/                 # Rust 集成测试
├── e2e/                       # Playwright E2E 测试
├── docs/                      # 分析文档
├── scripts/                   # 辅助脚本
├── CLAUDE.md                  # Claude Code 项目指令
└── package.json
```

---

## 命令参考

| 命令 | 说明 |
|------|------|
| `npm run dev` | 启动 Vite 开发服务器 |
| `npm run dev:tauri` | 启动 Tauri 桌面应用 |
| `npm run build` | 类型检查 + 前端构建 |
| `npm run build:tauri` | 生产构建 |
| `npm run build:tauri:msi` | 仅 MSI 安装包 |
| `npm run build:tauri:nsis` | 仅 NSIS 安装包 |
| `npm run test` | vitest 单元测试 |
| `npm run test:e2e` | Playwright E2E 测试 |
| `npm run test:e2e:smoke` | E2E 冒烟测试 |
| `npm run test:e2e:real` | 真实 CLI 输出测试 |
| `npm run test:rust` | Rust 单元测试 |
| `npm run test:rust:real` | Rust 集成测试 |
| `npm run test:all` | 全部测试 |
| `npm run test:quick` | 快速测试（跳过 E2E） |

---

## 配置

在应用设置面板中配置以下参数：

| 项 | 说明 | 示例 |
|----|------|------|
| **API Key** | DeepSeek API 密钥 | `sk-xxxx` |
| **Base URL** | API 端点 | `https://api.deepseek.com` |
| **Model** | 模型名称 | `deepseek-v4-pro` |

也可通过环境变量配置（优先）：

| 环境变量 | 说明 |
|----------|------|
| `ANTHROPIC_AUTH_TOKEN` | API 密钥 |
| `ANTHROPIC_BASE_URL` | API 基础 URL |
| `ANTHROPIC_MODEL` | 默认模型 |
| `CLAUDE_CODE_EFFORT_LEVEL` | 推理力度 (low/medium/high/max) |

权限模式会自动同步到 `~/.claude/settings.json` 的 `permissions.defaultMode`。

---

## 开发时间线

> 项目从可行性分析到功能完善，历时 5 天（2026-06-07 ~ 2026-06-11）。

### Day 0 · 2026-06-07（可行性分析）

| 事项 | 说明 |
|------|------|
| 📋 项目可行性分析 | [SOLO-FEASIBILITY-ANALYSIS.md](docs/SOLO-FEASIBILITY-ANALYSIS.md) — 技术栈验证、CLI 兼容性验证、架构对标准备 |
| 🏗️ 项目骨架搭建 | Tauri 2 + Vue 3 + Vite + TailwindCSS 4 + DaisyUI 5 |
| ⚙️ Rust 后端初始化 | `main.rs` / `lib.rs` / `db.rs` / `process.rs` / `protocol.rs` / `session.rs` 全部模块搭建 |
| 🔄 三线程进程模型 | Waiter / Stdout Reader / Stderr，对标 TOKENICODE 架构 |
| 📡 NDJSON 协议解析 | system / assistant / user / result 四种事件类型解析 |
| 💬 聊天 UI 基础 | ChatPanel / MessageBubble / InputBar / ModeBar 组件 |
| 🎨 代码高亮 | MarkdownRenderer + highlight.js 13 种语言 |
| ⚙️ 设置面板 | API Key / Base URL / Model 配置 + 连接测试 |
| 🌐 i18n | 中/英文双语 |
| 🗄️ SQLite | 4 表 schema（sessions, messages, approved_scenarios, settings）|
| 🧪 测试基础设施 | vitest + Playwright + cargo test |

**完成度: ~60%**

### Day 1 · 2026-06-08（初始发布）

| 提交 | 说明 |
|------|------|
| `b811869` | 🎉 **Initial commit** — 项目首次提交，完整骨架代码 |
| `406264e` | 🧹 移除未引用的截图，添加测试产物到 `.gitignore` |
| `f0c1f24` | 📁 **完善文件面板** — 文件树浏览、代码预览、Diff 查看器、面包屑导航、右键菜单 |

| 新增文件 | 说明 |
|----------|------|
| `FileTree.vue` | 递归目录树（展开/折叠、右键菜单）|
| `FilePreview.vue` | CodeMirror 6 只读预览（12 种语言） |
| `DiffViewer.vue` | 代码差异对比（三色标记） |
| `FileTree.test.ts` | 文件树组件测试 |
| `SessionSidebar.test.ts` | 会话侧边栏测试 |
| `chat.test.ts` | 聊天状态管理测试 |

### Day 2 · 2026-06-09（插件集成分析）

| 提交 | 说明 |
|------|------|
| `76e5d87` | 🔌 **Claude 插件分析** — 深入分析 36 个官方插件，安装 14 个核心插件 |

| 成果 | 说明 |
|------|------|
| [CLAUDE-PLUGINS-ANALYSIS.md](docs/CLAUDE-PLUGINS-ANALYSIS.md) | 36 个插件的完整分类和使用指南 |
| `install-claude-plugins.sh` | 一键安装 14 个推荐插件的脚本 |
| `search-results.md` | 代码库搜索结构文档 |
| CI/CD 配置 | `.github/workflows/ci.yml` + `release.yml` |

**安装的 14 个插件**:
`code-simplifier` · `code-review` · `commit-commands` · `pr-review-toolkit` · `feature-dev` · `plugin-dev` · `hookify` · `claude-md-management` · `frontend-design` · `claude-code-setup` · `mcp-tunnels` · `code-modernization` · `update-config` · `verify`

### Day 3 · 2026-06-10（Phase 6 完善打磨）

| 提交 | 说明 |
|------|------|
| `c79cae9` | ✨ **Phase 6 全面打磨** — 功能完善、UI 优化、测试增强 |

| 模块 | 改进内容 |
|------|----------|
| 🎨 **主题系统** | 亮色/暗色主题切换，CSS 变量统一管理 |
| 💬 **ChatPanel** | 消息操作（编辑/重发/复制/删除）、Debug 面板、JSON 导出 |
| 📝 **InputBar** | 工具栏（附件/文件/清除上下文/模式切换按钮）、自动 resize |
| 🫧 **MessageBubble** | 头像区分、思考折叠动画、工具调用卡片、操作按钮 |
| 🔔 **通知** | 桌面通知（含耗时 & token 统计） |
| 🧭 **CommandPalette** | Ctrl+K 命令面板（8 个核心操作） |
| ⚡ **Ultracode** | 多代理自动编排支持 |
| 🛡️ **权限控制** | 6 种模式 UI 切换 + `~/.claude/settings.json` 自动同步 |
| 🧪 **测试** | `MessageBubble.test.ts`（169 行）、`settings.test.ts`、组件测试完善 |

### Day 4 · 2026-06-11（文档与深度集成）🔄 进行中

| 提交 | 说明 |
|------|------|
| `fafe3bf` | 📚 **文档完善** — 移除废弃的 DeepSeek 代理脚本，新增完整项目文档 |

| 进行中 | 说明 |
|--------|------|
| 🔧 **CommandPalette 升级** | 40+ 命令，6 大分类（对标 Claude Code CLI `/` 命令 + VS Code Palette） |
| 📊 **ContextIndicator** | 上下文窗口用量可视化 |
| 🔌 **Stream 增强** | useStreamProcessor 扩展：事件回放、系统通知分级 |
| 🎨 **主题增强** | Light 主题完善（50+ 行 CSS 变量） |
| 🌐 **i18n 补全** | 91 行新翻译条目（中/英） |
| 🏪 **Settings 扩展** | Token 使用量持久化、上下文窗口配置 |
| 📖 **开发时间线** | 本节的建立 |

---

## 许可

MIT

---

## 相关链接

- [Claude Code CLI 文档](https://docs.anthropic.com/en/docs/claude-code)
- [Tauri 2 文档](https://v2.tauri.app/)
- [Vue 3 文档](https://vuejs.org/)
