# Agent 使用状态追踪

> 在 Agent 管理 Tab 中标记当前会话已使用的 Agent（绿点），未使用的保持灰点，已用的排前面。
> 仿照 CC `/context` 命令的 Agent 列表设计。

---

## 现状

- `ManagePanel.vue` 的 Agent Tab 已能列出所有 Agent（自定义 + 插件提供）
- 但所有 Agent 统一绿点（`enabled: true`），无法区分哪些在当前会话中被使用过
- `/context` 命令能显示当前加载的 Agent 列表，但 ContextUsageModal 中的数据是硬编码的

## 目标

| Agent 状态 | 圆点颜色 | 排序 |
|-----------|---------|------|
| 当前会话使用过 | 🟢 绿色 | 前 |
| 当前会话未使用 | ⚪ 灰色 | 后 |
| 不在会话中（ManagePanel 独立打开） | ⚪ 灰色 | — |

---

## 实施步骤

### 步骤 1：验证 tool_use 事件中的 agent 调用格式

**验证方法**：在 cc-gui 中发送一条消息，要求使用 agent（如 code-review），然后查看调试日志面板中 stream-event 的 tool_use 数据。

**验证项**：
- [ ] tool_use.name 是 `Agent` 还是 `Task`？
- [ ] tool_use.input 中 agent 类型字段名是 `subagent_type` 吗？
- [ ] 确认数据示例

→ 校验：拿到实际的 tool_use JSON 示例。

### 步骤 2：chat store 添加 usedAgents

**文件**：`src/stores/chat.ts`

- 新增 `usedAgents: Ref<Set<string>>`
- `addToolUse()` 中检测 agent 调用并写入 Set
- 切换会话或清空时重置

→ 校验：TypeScript 编译通过。

### 步骤 3：ManagePanel Agent Tab 读取 usedAgents

**文件**：`src/components/shared/ManagePanel.vue`

- `loadAgents()` 或渲染阶段读取 `chatStore.usedAgents`
- Agent 在 Set 中 → `enabled: true`（绿点），不在 → `enabled: false`（灰点）
- 排序：绿点在前
- 无活跃会话时全部灰点

→ 校验：编译通过 + 实际运行验证。

### 步骤 4：Workflow/后台 agent 的已知限制

Workflow 派出的 agent 不会被主 stream 追踪 → 文档化此限制。

→ 校验：记录到项目文档。

---

## 漏洞与应对

| 漏洞 | 等级 | 应对 |
|------|------|------|
| tool_use name 不确定是 `Agent` 还是 `Task` | 🔴 阻塞 | 步骤 1 实测确认，两种都支持 |
| Workflow/后台 agent 不产生主 stream 事件 | 🟡 已知限制 | 文档化，暂不处理 |
| 切换会话时 usedAgents 残留 | 🟡 可修 | 会话切换或 `clearMessages` 时重置 Set |

---

## 文件变更清单

| 文件 | 改动 |
|------|------|
| `src/stores/chat.ts` | + usedAgents Set, addToolUse 中检测, reset 时清空 |
| `src/components/shared/ManagePanel.vue` | loadAgents 读 usedAgents, 排序 + enabled 标记 |

---

> 创建日期：2026-06-24
> 状态：✅ 已实现

## 实现记录

### 步骤 1 ✅
- tool_use.name 为 `"Agent"`（实测确认）
- 代码同时兼容 `Agent` 和 `Task`

### 步骤 2 ✅ — chat store
- `usedAgents: Ref<Set<string>>` 追踪已用 agent
- `addToolUse()` 检测 Agent/Task 调用，提取 `subagent_type`
- `clearMessages()` 时重置

### 步骤 3 ✅ — ManagePanel Agent Tab
- 导入 chatStore
- `loadAgents()` 末尾排序：usedAgents 中的排前面 → `enabled: true`（绿点）
- 未使用的 → `enabled: undefined`（灰点）
- `usedAgents.size === 0` 时保持全部绿点（默认行为）

### 步骤 4 ✅ — 已知限制
- Workflow/后台 agent 不产生主 stream 事件 → 无法追踪
