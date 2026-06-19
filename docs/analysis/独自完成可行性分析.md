# cc-gui 独自完成可行性分析

> 2026-06-07 | 基于项目当前状态 + 工具链验证

---

## 1. 项目概述

**cc-gui** — Claude Code Desktop GUI，基于 **Tauri 2 + Vue 3 + TypeScript** 的桌面应用。

核心架构：前端通过 Tauri IPC 调用 Rust 后端 → Rust 后端 spawn Claude Code CLI 进程 → NDJSON 流式输出 → 解析为前端事件 → Vue 3 实时渲染聊天界面。

---

## 2. 当前完成度 ~60%

### 2.1 已完成 ✅

| 模块 | 文件 | 说明 |
|------|------|------|
| 项目骨架 | — | Tauri 2 + Vue 3 + Vite + TailwindCSS + DaisyUI |
| Rust 进程管理 | `process.rs` | 三线程模型（Waiter / Stdout Reader / Stderr） |
| NDJSON 协议解析 | `protocol.rs` | system/assistant/user/result 事件 |
| 前端流式处理 | `useStreamProcessor.ts` | 5 种 Tauri 事件监听 → Pinia 实时更新 |
| 聊天 UI | `ChatPanel.vue` | 消息列表、自动滚屏、处理中指示器 |
| 消息气泡 | `MessageBubble.vue` | Markdown 渲染、思考折叠、工具卡片 |
| 代码高亮 | `MarkdownRenderer.vue` + `useHighlight.ts` | highlight.js 13 种语言 |
| 输入栏 | `InputBar.vue` | Enter 发送、Shift+Enter 换行、自动 resize |
| 设置面板 | `SettingsPanel.vue` | API 配置、连接测试、快速预设 |
| i18n | `locales/` | 中/英文 |
| 调试面板 | `useDebugLog.ts` | 200 行日志、折叠开关 |
| E2E 测试 | `e2e/` | 12 个 Playwright 测试（mock 模式） |
| 单元测试 | `*.test.ts` | 6 个测试文件 (InputBar, MessageBubble, MarkdownRenderer, 3 stores) |
| Tauri Mock | `tauri-mock.ts` | 完整 mock，浏览器内运行 e2e |
| `--resume` 支持 | `lib.rs` + `session.rs` | 会话续接 |

### 2.2 未完成 ❌

| 功能 | 优先级 | 说明 |
|------|--------|------|
| **会话持久化** | 🔴 高 | 仅内存 HashMap。注释 "Phase 2+: SQLite via rusqlite" |
| **会话侧边栏 UI** | 🔴 高 | `session.ts` 有数据但无渲染组件 |
| **Router 路由** | 🔴 高 | `router/` 目录为空，设置面板无导航入口 |
| **设置面板入口** | 🔴 高 | `SettingsPanel.vue` 存在但无法从主界面访问 |
| **工具结果回显** | 🟡 中 | Tool use 卡片已渲染但 tool_result 未回填 |
| **StdinManager 集成** | 🟡 中 | 结构体已定义但 `_stdin` 被忽略 |
| **BypassMode 接入** | 🟡 中 | `BypassModeMap` 未接前端/Tauri command |
| **多会话管理** | 🟢 低 | 后端支持但前端仅单会话 |
| **权限控制开关** | 🟢 低 | `--dangerously-skip-permissions` 硬编码 |

---

## 3. 工具链验证 ✅

| 工具 | 版本 | 状态 |
|------|------|------|
| `claude` CLI | 2.1.167 | ✅ 已安装，PATH 可用 |
| `cargo` / `rustc` | 1.96.1 | ✅ 已安装 |
| `node` / `npm` | — | ✅ package.json scripts 可运行 |
| Tauric CLI | `@tauri-apps/cli` 2.11 | ✅ 在 devDependencies |
| NDJSON 协议 | — | ✅ 已用真实 CLI 输出验证 parser 兼容 |

**关键发现：**用 `claude --print --output-format stream-json "say hi"` 捕获的 NDJSON 输出与 `protocol.rs` 的 `StreamLine::parse()` / `to_frontend_event()` 完全兼容。system/init、assistant (text + thinking)、result 事件均正确解析。

---

## 4. 能否独自完成？最终判断

### ✅ 技术层面：可以

所有工具链就位，真实 CLI 可用。我能：
- 编译 Rust 后端 (`cargo build`)
- 运行单元测试 (`cargo test`, `vitest`)
- 用真实 NDJSON 输出验证协议解析器
- 实现缺失功能（SQLite、侧边栏、Router 等）
- 运行 e2e 测试验证前端

### ⚠️ 产品层面：需要你拍板

以下决策无法由 AI 自行判断：

| # | 决策 | 选项 |
|---|------|------|
| 1 | 会话持久化方案 | SQLite vs JSON 文件 vs 前端 IndexedDB |
| 2 | 会话侧边栏布局 | 左侧面板 vs 可折叠抽屉 vs 顶部 Tab |
| 3 | 设置入口位置 | 侧边栏图标 vs 导航按钮 vs 快捷键 |
| 4 | stdin 交互模式 | 弹出确认框 vs 内联按钮 vs 自动批准 |
| 5 | Tauri 构建配置 | debug build 足够还是需要 release 签名？ |

### ⚠️ 实操层面：GUI 验证需要你

- Rust 编译后可运行 `cargo test` 验证后端逻辑（自动）
- 前端可用 Playwright mock 模式验证（自动）
- 但 **真实 GUI 窗口（`tauri dev`）需要你肉眼确认** UI 渲染、主题、交互手感

---

## 5. 可独立完成的工作量

| 工作 | 预估 |
|------|------|
| SQLite 会话持久化 (`rusqlite`) | 2-3h |
| 会话侧边栏 UI 组件 | 2-3h |
| Router + 设置面板导航 | 1-2h |
| 工具结果回填到 tool_use 卡片 | 1h |
| StdinManager 接线 | 2-3h |
| BypassModeMap 接入 | 1h |
| 补充 Rust 侧单元测试 | 1-2h |
| `--resume` 端到端测试 | 1h |
| 代码清理 + 文档 | 1h |
| **总计** | **~13-17h** |

---

## 6. 结论

**修正后判断：技术上可以独自完成剩余 40%，但需要你确认 5 个产品决策后开始。** 最终 GUI 验收仍需你肉眼看一次。

如果你拍板上述决策，我现在就可以开始实现。
