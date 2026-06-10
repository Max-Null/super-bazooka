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
│   │   ├── chat/              # ChatPanel, MessageBubble, InputBar, ModeBar, ThinkingIndicator, ContextIndicator
│   │   ├── layout/            # AppShell (主布局容器)
│   │   ├── session/           # SessionSidebar (会话侧边栏)
│   │   ├── files/             # FilePanel, FileTree, FilePreview, DiffViewer
│   │   ├── settings/          # SettingsPanel (API配置/连接测试)
│   │   └── shared/            # MarkdownRenderer, MermaidRenderer, CommandPalette, ErrorBoundary, FilePreviewModal
│   ├── composables/           # useStreamProcessor, useFilePreview, useCommandPalette, useDebugLog, useHighlight
│   ├── stores/                # session.ts, chat.ts, settings.ts (Pinia)
│   ├── lib/                   # utils.ts, tauri-bridge.ts, tauri-mock.ts
│   ├── locales/               # zh.json, en.json (vue-i18n)
│   ├── router/                # Vue Router 路由配置
│   └── assets/                # main.css, theme-light.css
├── src-tauri/                 # Rust 后端 (Tauri 2)
│   ├── src/
│   │   ├── main.rs            # 程序入口
│   │   ├── lib.rs             # 28 个 Tauri commands + AppState
│   │   ├── process.rs         # 三线程进程模型 (Waiter / Stdout Reader / Stderr)
│   │   ├── protocol.rs        # NDJSON stream-json 事件解析
│   │   ├── session.rs         # 会话 CRUD + API 连接测试 + 批准场景
│   │   └── db.rs              # SQLite 初始化与迁移
│   └── tests/                 # Rust 集成测试
├── e2e/                       # Playwright E2E 测试 (12个用例)
├── docs/                      # 分析文档
└── scripts/                   # 构建/测试辅助脚本
```

### 核心数据流

```
User Input (InputBar.vue)
  → sendMessage (Tauri IPC command, lib.rs)
    → sync_permission_settings → spawn_claude_session (process.rs)
      → Claude Code CLI (subprocess, stream-json + --include-partial-messages)
        → NDJSON lines → BufReader (process.rs)
          → StreamLine::parse → to_frontend_event (protocol.rs)
            → app_handle.emit("stream-event") (Tauri event)
              → useStreamProcessor.ts listen → Pinia store → Vue 3 reactive render
```

### Rust 模块职责

| 模块 | 职责 |
|------|------|
| `main.rs` | 程序入口，初始化 DB + 启动 Tauri |
| `lib.rs` | 28 个 Tauri commands：会话 CRUD、消息管理、文件操作、进程管理 |
| `process.rs` | 三线程模型 spawn Claude CLI：Waiter（进程生命周期）、Stdout Reader（NDJSON 解析→事件发射）、Stderr Reader（错误转发） |
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

## 工程规范

> 继承自全局 CLAUDE.md，以下为 cc-gui 项目特定补充。

### 0. 动手前必做
1. **先查复用**: Grep 搜索是否已有类似组件/composable/util，优先复用而非重写
2. **先查 skill**: 检查可用 skill（frontend-design 等）
3. **先看文档**: 以官方文档为准，不猜 API 行为
4. **写完后对照 spec 逐条验证，再回复用户**
5. **多步骤操作前**：检查 `docs/` 目录是否有相关计划文档，将实施计划写入文件再执行

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
