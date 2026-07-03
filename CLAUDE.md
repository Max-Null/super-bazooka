# cc-gui CLAUDE.md

> Tauri 2 + Vue 3 + TypeScript 桌面应用 — 为 Claude Code CLI 提供图形化桌面界面。

## 架构入口

```
src/stores/{chat,session,settings}.ts    # Pinia 全局状态
src/lib/tauri-bridge.ts                  # 所有 invoke() 封装
src/composables/useStreamProcessor.ts    # CC 流事件 → store
src/components/chat/ChatPanel.vue        # 主聊天面板 + 审批
src/components/settings/SettingsPanel.vue # 设置面板
src-tauri/src/{lib,process,protocol,session,db}.rs  # Rust 后端
```

完整架构树、功能索引、设计决策 → [docs/知识/](docs/知识/)

## 核心数据流

```
用户输入 → sendMessage(IPC) → spawn_claude_session(process.rs)
  → CC 子进程(stdin: NDJSON 用户消息, stdout: stream-json)
    → protocol.rs 解析 → Tauri emit("stream-event")
      → useStreamProcessor.ts → Pinia store → Vue 渲染
```

Settings: 前端 watcher → `set_claude_settings`(lib.rs) → 写入 `~/.claude/settings.json` env 区（ANTHROPIC_AUTH_TOKEN/API_KEY/BASE_URL/MODEL）。发送消息前 `sync_permission_settings` 写 `permissions.defaultMode`。

## 🔴 最高优先级规则

### 禁止手搓轮子

**写任何新代码前按序检查：**
1. Grep 搜索已有实现
2. 检查 `src/composables/` 有无对应 `useXxx()`
3. 检查 `src/lib/utils.ts` / `tauri-bridge.ts`
4. 检查同类 `.vue` 组件是否已有相同逻辑

常见违规：组件间复制粘贴逻辑、手写 `invoke()` 不走 bridge、新写已有 util 函数。

### 组件与数据流
- Props ↓ Events ↑，**禁止 `defineModel`**
- 所有 Tauri 调用走 `tauri-bridge.ts`，不在组件中直接 `invoke()`
- 全局状态用 Pinia stores

### i18n 硬性要求
所有面向用户的字符串必须中英双语，新增文案同时添加到 `zh.json` 和 `en.json`。

### Rust 硬性要求
- 禁止 `panic`/`unwrap`/`expect`（`run()` 入口和 `#[cfg(test)]` 除外）
- 返回 `Result<T, String>`
- 禁止 `unsafe`

### 动手前必做
1. Grep 已有实现，检查 composables/lib/同类组件
2. **修改 `src-tauri/` 下任何 .rs 文件前** → `Read docs/知识/设计决策参考.md`（31 条决策，改核心流程时逐条对照）
3. 写 UI 前 → Grep 同类组件，复制其模板和 class 模式。颜色用 CSS 变量，禁止硬编码色值

## 构建与测试

```bash
npm run dev:tauri              # 开发运行
npm run build:tauri            # 生产构建
npm run test                   # vitest 单元测试
npm run test:e2e               # Playwright E2E（mock 模式）
npm run test:rust              # Rust 测试
npm run test:quick             # 快速测试（跳过 e2e）
```

## 关键设计决策（细节见 docs/知识/设计决策参考.md）

1. **三线程进程模型** — Waiter + Stdout Reader + Stderr，NDJSON 双工（`--input-format stream-json --output-format stream-json`）
2. **SQLite 持久化** — WAL 模式，4 表（sessions, messages, settings, approved_scenarios）
3. **不传 `--model` 给 CLI** — CC 从 `settings.json` env 区读取模型配置
4. **权限在 spawn 前同步** — `sync_permission_settings()` 写 `permissions.defaultMode`
5. **用户消息只由 Rust 保存** — 前端不重复写，避免历史回显双份

## 阶段性完成必做

1. 补充测试（检查 `.test.ts`）
2. 更新 CLAUDE.md / docs/（如有设计变更）
3. Git commit（Conventional Commits: `feat(模块):` / `fix(模块):`）

## 颜色系统

全部颜色通过 CSS 变量管理（`src/assets/main.css`），禁止硬编码色值。

| 变量 | 用途 | 暗色 | 亮色 |
|------|------|------|------|
| `--accent` | 主色调（按钮/链接/焦点/思考中状态） | `#06d6a0` | `#059669` |
| `--accent-dim` | 主色调暗色（边框/分隔线） | `#048a66` | `#047857` |
| `--accent-glow` | 主色调发光（选中背景） | `rgba(6,214,160,.10)` | `rgba(5,150,105,.08)` |
| `--amber` | 琥珀色（思考块边框/图标） | `#f5a623` | `#d97706` |
| `--coral` | 珊瑚色（错误/已停止/橙点） | `#ff5e5b` | `#dc2626` |
| `--violet` | 紫罗兰（工具名/MCP） | `#a78bfa` | `#7c3aed` |
| `--blue` | 蓝色（未读蓝点） | `#3b82f6` | `#2563eb` |
| `--bg-*` | 背景层级（root/surface/elevated/hover/active） | 暗灰系 | 浅灰系 |
| `--border-*` | 边框（dim/default/bright） | | |
| `--text-*` | 文字（muted/secondary/primary/bright） | | |

## Git 提交规范

`feat|fix|refactor|chore(模块): 描述` — 模块: chat, files, session, settings, rust, test, docs, ui, i18n
