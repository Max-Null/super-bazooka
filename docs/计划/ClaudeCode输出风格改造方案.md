# Claude Code 输出风格改造方案

## 文档信息

| 项目 | 内容 |
|------|------|
| 文档标题 | Claude Code 输出风格改造方案 |
| 适用项目 | `cc-gui` |
| 文档类型 | 方案设计 |
| 目标版本 | Phase 1 |
| 编写日期 | 2026-06-20 |
| 编写者 | AI 助手（GPT-5.4） |
| 文档状态 | 已修订（v2，2026-06-20） |

## 1. 改造目标

本方案的目标是将当前项目的 assistant 输出方式，从“聊天气泡式聚合消息”改造为接近 Claude Code 的“执行日志流式输出”。

目标效果包括：

- assistant 输出不再以单个大气泡为主，而是以多个按时间顺序排列的执行块构成
- 思考过程显示为 `Thought for Xs` 类型的折叠块
- 工具调用显示为独立工具卡片，包含 `IN / OUT`
- 最终解释性文本显示为单独的文本块
- 一轮 assistant 输出完整持久化为结构化 block 序列

## 2. 当前问题

当前实现的核心问题在于：assistant 的所有输出都被聚合到单条消息结构中，导致数据层与展示层都无法自然承载 Claude Code 风格的分段输出。

现状特征：

- `thinking` 是单个字符串
- `toolUses` 是工具调用列表，但不是独立渲染块
- `content` 是最终聚合文本
- `MessageBubble.vue` 负责 assistant 的大部分展示逻辑
- `useStreamProcessor.ts` 负责把流式事件折叠写进单一 `currentAssistantMsg`

这样会带来以下限制：

- 无法自然表达“先 thought，再 tool，再文本”的顺序
- 无法把工具输入输出做成标准化卡片
- 难以实现与 Claude Code 一致的日志式阅读体验
- 后续持久化和重放也会持续依赖聚合字段

## 3. 前置发现：Rust 侧已有 ContentBlock 结构

> **关键发现（2026-06-20 方案审查）**：`src-tauri/src/protocol.rs` 中已经定义了 `ContentBlock` 枚举（`Text` / `Thinking` / `ToolUse`），与目标 `ExecutionBlock` 模型高度对应。当前瓶颈是 `StreamFrontendEvent.to_frontend_event()` 方法将 blocks **拍平为聚合字段**（`text: String`、`thinking: String`、`tool_use: Option<Vec<Value>>`），丢失了顺序信息和 block 边界。

这意味着方案原定的「Phase 1 前端映射重建 blocks → Phase 2 Rust 协议结构化」顺序是**兜圈子**——前端从拍平数据重建 blocks，然后 Phase 2 再删掉映射逻辑。

**修正策略**：Phase 1 直接修改 `StreamFrontendEvent` 结构，使其携带 blocks 序列而非聚合字段。工作量更小，且自然对齐后续协议演进。详见下方「Phase 1 开发任务列表」章节。

## 4. 核心设计原则

本方案采用以下设计原则：

1. assistant 输出以 `ExecutionBlock` 为核心，而不是 `content/thinking/toolUses`
2. 前端渲染围绕 block 构建，而不是继续扩展 `MessageBubble`
3. 持久化结构直接保存 block 序列，不考虑兼容旧数据
4. 用户消息保持简单结构，assistant 消息完全结构化
5. Rust 侧 `StreamFrontendEvent` 直接携带 blocks，前端不做重建映射

## 5. 数据结构定义

### 5.1 基础类型

```ts
export type MessageRole = "user" | "assistant";

export type ExecutionBlockKind =
  | "thought"
  | "tool"
  | "text"
  | "notice"
  | "permission";

export type ExecutionBlockStatus =
  | "streaming"
  | "done"
  | "error";
```

### 5.2 Block 模型

```ts
export interface BaseBlock {
  id: string;
  kind: ExecutionBlockKind;
  createdAt: number;
  status: ExecutionBlockStatus;
}

export interface ThoughtBlock extends BaseBlock {
  kind: "thought";
  summary: string;       // e.g. "Thought for 3s"
  content: string;
  durationMs?: number;
  collapsed: boolean;
}

export interface ToolBlock extends BaseBlock {
  kind: "tool";
  toolName: string;      // PowerShell / Bash / Grep / Read
  title: string;         // e.g. "Create docs subdirectories"
  input: string;
  output: string;
  /** Tool result returned by CLI (user-role tool_result message).
   *
   *  截断策略（在 ExecutionToolBlock 组件中实施）：
   *  - result 字段独立截断，不与 output 合并计算长度
   *  - 默认展示前 2000 字符，超出部分折叠
   *  - 折叠区提供 "Show full output (N chars)" 展开按钮
   *  - result 为纯文本时按行截断（优先保留完整行），代码块按字符截断
   *  - output（stdout/stderr）不截断——通常较短；如需截断，在上层
   *    AssistantExecutionView 中限制整个 ToolBlock 最大高度 */
  result?: string;
  /** Whether tool result was an error */
  resultIsError?: boolean;
  stream: "stdout" | "stderr" | "mixed";
  durationMs?: number;
  exitCode?: number | null;
  /** Whether this tool requires user approval before execution.
   *  When true, the block is rendered with an Approve/Deny UI inline. */
  requiresApproval?: boolean;
  collapsed: boolean;
}

export interface TextBlock extends BaseBlock {
  kind: "text";
  content: string;
}

export interface NoticeBlock extends BaseBlock {
  kind: "notice";
  level: "info" | "success" | "warning" | "error";
  content: string;
}

export type PermissionResolution =
  | "pending"    // 等待用户审批
  | "approved"   // 用户批准
  | "denied"     // 用户拒绝
  | "expired"    // 审批超时（CLI 侧已跳过）
  | "error";     // 审批过程出错

export interface PermissionRequestBlock extends BaseBlock {
  kind: "permission";
  toolName: string;
  toolInput: string;
  resolution: PermissionResolution;
}

export type ExecutionBlock =
  | ThoughtBlock
  | ToolBlock
  | TextBlock
  | NoticeBlock
  | PermissionRequestBlock;
```

### 5.3 消息结构

```ts
export interface UserMessagePayload {
  text: string;
  attachments: Array<{
    name: string;
    path: string;
  }>;
}

export interface AssistantMessagePayload {
  blocks: ExecutionBlock[];
  durationMs?: number;
  inputTokens?: number;
  outputTokens?: number;
  costUSD?: number;
}

export interface ChatMessage {
  id: string;
  role: MessageRole;
  timestamp: number;
  isStreaming: boolean;
  payload: UserMessagePayload | AssistantMessagePayload;
}
```

### 5.4 示例

#### 用户消息

```json
{
  "id": "u-001",
  "role": "user",
  "timestamp": 1781887000000,
  "isStreaming": false,
  "payload": {
    "text": "把 docs 目录整理一下",
    "attachments": []
  }
}
```

#### assistant 消息

```json
{
  "id": "a-001",
  "role": "assistant",
  "timestamp": 1781887001000,
  "isStreaming": true,
  "payload": {
    "blocks": [
      {
        "id": "b1",
        "kind": "thought",
        "createdAt": 1781887001200,
        "status": "done",
        "summary": "Thought for 3s",
        "content": "我先分析一下目录结构...",
        "durationMs": 3000,
        "collapsed": true
      },
      {
        "id": "b2",
        "kind": "tool",
        "createdAt": 1781887004300,
        "status": "done",
        "toolName": "PowerShell",
        "title": "Create docs subdirectories",
        "input": "New-Item -ItemType Directory ...",
        "output": "目录: H:\\\\MaxNull\\\\WorkStation\\\\cc-gui\\\\docs",
        "stream": "stdout",
        "durationMs": 120,
        "exitCode": 0,
        "collapsed": false
      },
      {
        "id": "b3",
        "kind": "text",
        "createdAt": 1781887004500,
        "status": "done",
        "content": "测试通过了，之前是 vitest 赝态问题。"
      }
    ],
    "durationMs": 3200,
    "inputTokens": 123,
    "outputTokens": 456,
    "costUSD": 0.0123
  }
}
```

## 6. 事件协议表

为了让前端可以稳定组装 `ExecutionBlock`，后端需要向前端发出更明确的执行事件。

### 6.1 事件列表

| 事件类型 | 触发时机 | 关键字段 | 前端动作 |
|------|------|------|------|
| `thought_start` | 开始思考 | `session_id`, `message_id`, `block_id` | 创建 `ThoughtBlock` |
| `thought_delta` | 思考内容增量到达 | `block_id`, `text` | 追加到 `ThoughtBlock.content` |
| `thought_end` | 思考结束 | `block_id`, `duration_ms` | 更新 `summary/status/durationMs` |
| `tool_start` | 开始工具调用 | `block_id`, `tool_name`, `title`, `input` | 创建 `ToolBlock` |
| `tool_input` | 工具输入补充（input_json_delta 增量） | `block_id`, `text` | 追加到 `ToolBlock.input` |
| `tool_result` | 工具执行结果返回（单次完整事件，CLI 以 user/tool_result 消息返回） | `block_id`, `content`, `is_error` | 写入 `ToolBlock.result`，前端截断长输出 |
| `tool_end` | 工具调用结束 | `block_id`, `success`, `duration_ms`, `exit_code` | 更新 `ToolBlock.status` |
| `permission_request` | CLI 请求用户审批工具调用 | `block_id`, `tool_name`, `tool_input` | 创建 `PermissionRequestBlock`，渲染 Approve/Deny |
| `permission_resolved` | 用户完成审批 | `block_id`, `resolution` | 更新 `PermissionRequestBlock.resolution` |
| `text_start` | 开始生成最终文本 | `block_id` | 创建 `TextBlock` |
| `text_delta` | 文本增量到达 | `block_id`, `text` | 追加到 `TextBlock.content` |
| `text_end` | 文本完成 | `block_id` | 更新 `TextBlock.status` |
| `notice` | 系统说明/成功/警告/错误 | `level`, `content` | 创建 `NoticeBlock` |
| `session_done` | 当前一轮 assistant 完成 | `duration_ms`, `input_tokens`, `output_tokens`, `cost_usd` | 标记消息结束 |
| `session_error` | 当前一轮异常中止 | `error` | 追加错误块并结束消息 |

### 6.2 建议事件类型定义

```ts
export type ExecutionEvent =
  | {
      type: "thought_start";
      session_id: string;
      message_id: string;
      block_id: string;
    }
  | {
      type: "thought_delta";
      session_id: string;
      message_id: string;
      block_id: string;
      text: string;
    }
  | {
      type: "thought_end";
      session_id: string;
      message_id: string;
      block_id: string;
      duration_ms?: number;
    }
  | {
      type: "tool_start";
      session_id: string;
      message_id: string;
      block_id: string;
      tool_name: string;
      title: string;
      input?: string;
    }
  | {
      type: "tool_input";
      session_id: string;
      message_id: string;
      block_id: string;
      text: string;
    }
  | {
      type: "tool_end";
      session_id: string;
      message_id: string;
      block_id: string;
      success: boolean;
      duration_ms?: number;
      exit_code?: number | null;
    }
  | {
      type: "tool_result";
      session_id: string;
      message_id: string;
      block_id: string;
      content: string;
      is_error: boolean;
    }
  | {
      type: "permission_request";
      session_id: string;
      message_id: string;
      block_id: string;
      tool_name: string;
      tool_input: string;
    }
  | {
      type: "permission_resolved";
      session_id: string;
      message_id: string;
      block_id: string;
      resolution: PermissionResolution;
    }
  | {
      type: "text_start";
      session_id: string;
      message_id: string;
      block_id: string;
    }
  | {
      type: "text_delta";
      session_id: string;
      message_id: string;
      block_id: string;
      text: string;
    }
  | {
      type: "text_end";
      session_id: string;
      message_id: string;
      block_id: string;
    }
  | {
      type: "notice";
      session_id: string;
      message_id: string;
      block_id: string;
      level: "info" | "success" | "warning" | "error";
      content: string;
    }
  | {
      type: "session_done";
      session_id: string;
      message_id: string;
      duration_ms?: number;
      input_tokens?: number;
      output_tokens?: number;
      cost_usd?: number;
    }
  | {
      type: "session_error";
      session_id: string;
      message_id: string;
      error: string;
    };
```

## 7. 组件拆分清单

assistant 输出改造后，不再继续依赖当前的 `MessageBubble.vue` 作为主渲染容器，而是拆分为更清晰的执行块组件。

### 7.1 建议组件结构

| 组件 | 职责 | 输入 | 输出 |
|------|------|------|------|
| `ChatMessageItem.vue` | 顶层消息分发器 | `message` | 根据角色选择渲染器 |
| `UserMessageBubble.vue` | 用户消息展示 | `UserMessagePayload` | 附件点击事件 |
| `AssistantExecutionView.vue` | assistant 执行流容器，内含 footer 统计（时长/token/cost） | `AssistantMessagePayload` | block 级交互 |
| `ExecutionThoughtBlock.vue` | 思考块 | `ThoughtBlock` | 展开/折叠 |
| `ExecutionToolBlock.vue` | 工具块（含 IN/OUT/result，长输出截断） | `ToolBlock` | 展开/折叠、复制、显示完整输出 |
| `ExecutionTextBlock.vue` | 文本块 | `TextBlock` | 无 |
| `ExecutionNoticeBlock.vue` | 通知块 | `NoticeBlock` | 无 |
| `ExecutionPermissionBlock.vue` | 权限审批块（内联 Approve/Deny） | `PermissionRequestBlock` | approve / deny |

### 7.2 建议目录

```text
src/components/chat/
  ChatMessageItem.vue
  UserMessageBubble.vue
  AssistantExecutionView.vue    # 内含 footer 统计，不拆独立组件
  ExecutionThoughtBlock.vue
  ExecutionToolBlock.vue
  ExecutionTextBlock.vue
  ExecutionNoticeBlock.vue
  ExecutionPermissionBlock.vue
```

### 7.3 各组件说明

#### `UserMessageBubble.vue`

- 保留当前用户消息的轻量气泡样式
- 显示文本与附件
- 不参与 assistant block 渲染逻辑

#### `AssistantExecutionView.vue`

- 遍历 `payload.blocks`
- 负责按顺序渲染每个 block
- 在尾部直接渲染时长、token、cost 统计（不拆独立 `ExecutionFooterStats` 组件——ponytail：减少文件数）

#### `ExecutionThoughtBlock.vue`

- 默认折叠
- 标题行显示：`Thought for Xs`
- 展开后显示完整思考文本

#### `ExecutionToolBlock.vue`

- 标题行显示：工具名、操作摘要、状态、用时
- 内容显示：`IN`（输入）、`OUT`（输出）、`RESULT`（工具执行结果）
- 长输出截断：result/output 超过 ~2000 字符默认折叠，提供"Show full output"展开按钮
- 复制按钮：IN / OUT / RESULT 各行独立复制

#### `ExecutionTextBlock.vue`

- 负责展示最终 markdown 文本
- 直接复用现有 `MarkdownRenderer.vue`

#### `ExecutionNoticeBlock.vue`

- 展示简单说明、成功消息、警告、错误
- 样式根据 `level` 区分

#### `ExecutionPermissionBlock.vue`

- 内联渲染权限审批 UI（替代当前 ChatPanel 顶部的独立审批栏）
- 显示工具名和输入预览
- 提供 Approve / Deny 按钮（对应 `resolution: "pending"` 态）
- 5 种 resolution 状态渲染：
  - `pending` → 显示 Approve / Deny 按钮
  - `approved` → 绿色 ✓ Approved，折叠
  - `denied` → 红色 ✗ Denied，折叠
  - `expired` → 灰色 ⏱ Expired（CLI 已跳过），折叠
  - `error` → 黄色 ⚠ Error，显示错误信息

## 8. Phase 1 开发任务列表

Phase 1 的目标是完成消息模型、渲染层、事件协议的同步切换，形成一个可运行、可持久化的 Claude Code 风格输出 MVP。

> **策略修正**：基于第 3 节的前置发现，Phase 1 将直接修改 Rust 侧 `StreamFrontendEvent` 以携带 blocks 序列（原方案 Phase 2 内容前移），前端不再做拍平→重建的兜圈子映射。

### 8.1 Phase 1 范围

- 重构 Rust `StreamFrontendEvent` 结构，使其携带 blocks 序列而非聚合字段
- 重构前端消息数据结构（`ExecutionBlock` 模型）
- 建立 assistant block 渲染组件（6 个新组件）
- 调整持久化格式为新结构
- 适配 `exportMarkdown` 导出
- 不考虑旧数据兼容

### 8.2 任务拆分

#### 任务 1：重构 Rust 事件结构（原 Phase 2 前移）

改动文件：

- `src-tauri/src/protocol.rs`

具体工作：

- 新增 `FrontendBlock` 类型（对应 `ContentBlock`，增加 `block_id` + `status` + `created_at`）：
  ```rust
  pub struct FrontendBlock {
      pub block_id: String,
      pub kind: String,        // "thought" | "tool" | "text" | "notice" | "permission"
      pub created_at: u64,
      pub status: String,      // "streaming" | "done" | "error"
      pub content: Value,      // block-type-specific payload
  }
  ```
- 修改 `StreamFrontendEvent`：将 `text/thinking/tool_use` 聚合字段替换为 `blocks: Vec<FrontendBlock>`
- 保留 `control_request`、`is_final`、`duration_ms`、`input_tokens`、`output_tokens`、`cost_usd`、`error` 字段
- 重写 `to_frontend_event()`：不再拍平 `ContentBlock`，而是映射为 `FrontendBlock` 序列
- 映射规则：
  - `ContentBlock::Thinking` → `FrontendBlock { kind: "thought", ... }`
  - `ContentBlock::ToolUse` → `FrontendBlock { kind: "tool", ... }`
  - `ContentBlock::Text` → `FrontendBlock { kind: "text", ... }`
  - `control_request` 事件 → `FrontendBlock { kind: "permission", ... }`

#### 任务 2：重构前端消息数据模型

改动文件：

- `src/stores/chat.ts`
- `src/lib/tauri-bridge.ts`（更新 `StreamEvent` 接口）

具体工作：

- 引入 `ExecutionBlock` 相关类型（与第 5 节定义对齐）
- 将 `Message` 改为 `payload` 驱动结构（`UserMessagePayload | AssistantMessagePayload`）
- assistant 不再依赖 `content/thinking/toolUses`
- 新增以下 store 方法：
  ```ts
  startAssistantExecution(messageId?: string)
  upsertBlock(block: ExecutionBlock)         // 按 block_id 新增或更新
  finishBlock(blockId: string, status?: ExecutionBlockStatus)
  appendTextDelta(blockId: string, text: string)
  appendThoughtDelta(blockId: string, text: string)
  appendToolInputDelta(blockId: string, json: string)
  setToolResult(blockId: string, result: string, isError: boolean)
  setPermissionResolution(blockId: string, resolution: PermissionResolution)
  finishAssistantExecution(stats: {...})
  ```
- 更新 `loadMessages()`：按新 `{ blocks: [...], ... }` JSON 格式解析
- 更新 `exportMarkdown()`：遍历 blocks 序列生成导出文本
- 更新 `StreamEvent` TypeScript 接口：匹配新的 Rust `StreamFrontendEvent`（`blocks: FrontendBlock[]` 替代 `text/thinking/tool_use`）

#### 任务 3：搭建 assistant block 组件

改动文件：

- 新建 `AssistantExecutionView.vue`
- 新建 `ExecutionThoughtBlock.vue`
- 新建 `ExecutionToolBlock.vue`
- 新建 `ExecutionTextBlock.vue`
- 新建 `ExecutionNoticeBlock.vue`
- 新建 `ExecutionPermissionBlock.vue`

具体工作：

- 先用 mock 数据验证静态布局
- 完成折叠、代码块、标题行、状态点等基础交互
- `ExecutionToolBlock`：支持 IN / OUT / RESULT 三段展示，~2000 字符截断 + "Show full output"
- `ExecutionPermissionBlock`：内联 Approve / Deny 按钮

#### 任务 4：改造消息渲染入口

改动文件：

- `src/components/chat/ChatPanel.vue`

具体工作：

- 替换当前 assistant 消息统一走 `MessageBubble.vue` 的逻辑
- 改为：
  - `role === "user"` → `UserMessageBubble.vue`
  - `role === "assistant"` → `AssistantExecutionView.vue`
- 移除 ChatPanel 顶部的独立 `control_request` 审批栏（改为内联 `ExecutionPermissionBlock`）

#### 任务 5：改造流式组装逻辑

改动文件：

- `src/composables/useStreamProcessor.ts`

具体工作：

- 停止直接写 `appendText / appendThinking / addToolUse`
- 改为消费 Rust 侧发出的 blocks 序列事件：
  - `StreamFrontendEvent.blocks` 直接驱动 `upsertBlock()` / `finishBlock()`
  - `result` 事件触发 `finishAssistantExecution()`
  - `control_request` → `upsertBlock({ kind: "permission", ... })`
- 前端不再做事件→block 的映射推理（映射已在 Rust 侧 `to_frontend_event()` 完成）

#### 任务 6：改造持久化结构

改动文件：

- `src-tauri/src/lib.rs`
- `src-tauri/src/session.rs`

具体工作：

- 用户消息统一保存：
  ```json
  { "text": "...", "attachments": [...] }
  ```
- assistant 消息统一保存：
  ```json
  { "blocks": [...], "durationMs": 1234, "inputTokens": 10, "outputTokens": 20, "costUSD": 0.001 }
  ```
- 不提供旧格式 fallback

#### 任务 7：补基础测试

改动文件：

- `src/stores/chat.test.ts`
- assistant 相关新组件测试文件
- `src/composables/useStreamProcessor.test.ts`
- `src-tauri/tests/`（Rust 集成测试）

具体工作：

- 验证 block upsert / finish 行为
- 验证不同 block 类型渲染
- 验证 Rust `to_frontend_event()` blocks 映射正确性
- 验证 `exportMarkdown()` 新格式导出

## 9. Phase 1 验收标准

完成 Phase 1 后，系统需要满足以下条件：

- assistant 输出不再是传统聊天气泡样式
- 页面可稳定展示：
  - thought block（折叠/展开）
  - tool block（IN / OUT / RESULT，长输出截断）
  - text block（markdown 渲染）
  - notice block（按 level 区分样式）
  - permission block（内联 Approve/Deny）
- `tool block` 支持 `IN / OUT / RESULT`
- `permission block` 替代了 ChatPanel 独立审批栏
- `text block` 支持 markdown 渲染
- assistant 一轮输出可完整保存并重新加载
- `exportMarkdown()` 按新 blocks 格式导出
- 新结构在前端单测 + Rust 集成测试中通过

## 10. Phase 1 非目标

以下内容不在本阶段范围内：

- 不兼容旧数据库消息结构
- 不要求完全复刻 Claude Code 的所有视觉细节（如虚拟滚动、状态图标动画）
- 不处理多会话同时流式回放的高级优化
- ~~不要求 Rust 侧输出 block 级协议~~（已前移：任务 1 直接改造 `StreamFrontendEvent`）

## 11. 风险评估

本次改造属于消息模型、渲染方式、持久化结构三层联动调整，虽然目标清晰，但仍存在以下主要风险。

### 11.1 数据结构切换风险

- 风险描述：assistant 消息从聚合字段改为 block 结构后，前端 store、渲染组件、持久化读写都需要同步切换，任何一层遗漏都可能导致消息无法正常显示或保存。
- 影响范围：`src/stores/chat.ts`、`src/components/chat/`、`src-tauri/src/session.rs`
- 缓解措施：
  - 先完成类型定义与 store 改造，再接入 UI
  - 对 `append / finish / persist / load` 建立单元测试
  - 在 Phase 1 中优先保证单轮输出稳定，不同时推进过多交互细节

### 11.2 流式事件映射风险（已降低）

- 风险描述：原方案 Phase 1 在前端做事件→block 映射，存在 block 边界不清的问题。**已通过策略修正解决**：映射逻辑前移到 Rust 侧 `to_frontend_event()`，直接利用已有的 `ContentBlock` 结构。
- 影响范围：`src-tauri/src/protocol.rs`、`src/composables/useStreamProcessor.ts`
- 缓解措施：
  - Rust 侧直接从 `ContentBlock` 映射为 `FrontendBlock`，不经过拍平→重建
  - block 边界由 `content_block_start` / `content_block_stop` 事件明确
  - 前端只消费 blocks，不做推断

### 11.3 持久化不可兼容风险

- 风险描述：本方案明确不兼容旧数据，切换后原有 assistant 历史消息可能无法按新结构直接读取。
- 影响范围：本地 SQLite 历史会话
- 缓解措施：
  - 在实施前明确这是结构切换版本
  - 若上线前需要保留历史，可单独增加一次性迁移脚本；不在本方案范围内
  - 在文档和版本说明中明确“旧历史不保证可用”

### 11.4 UI 复杂度上升风险

- 风险描述：assistant 渲染从单组件扩展为多个 block 组件后，样式、折叠状态、代码块滚动和长输出性能都会比当前复杂。
- 影响范围：`src/components/chat/`
- 缓解措施：
  - Phase 1 先实现最小可用交互，不一次性堆叠所有视觉细节
  - 复用现有 `MarkdownRenderer.vue`
  - 对长输出区域统一使用独立容器和滚动约束

### 11.5 协议演进风险（已消除）

- 风险描述：~~Phase 1 允许前端做事件映射，后续 Rust 协议设计不一致可能导致前端再次重构。~~ **已通过策略修正消除**：Phase 1 直接完成 Rust 侧 `StreamFrontendEvent` 改造，后续只需扩展 block 类型，无需重构渲染模型。

## 12. 推荐实施顺序

建议按以下顺序推进：

```text
1. 重构 Rust StreamFrontendEvent（任务 1 — 协议层，先定数据格式）
2. 重构前端消息数据模型（任务 2 — 类型 + store）
3. 新建 assistant block 组件（任务 3 — 6 个组件）
4. 替换消息渲染入口（任务 4 — ChatPanel 改造）
5. 改造 useStreamProcessor（任务 5 — 消费 blocks 事件）
6. 改造持久化与加载（任务 6 — 前后端保存/加载）
7. 补齐测试（任务 7）
8. 视觉细节打磨
```

## 13. 里程碑

建议将本次改造拆分为 4 个里程碑，每个里程碑都对应可检查的交付结果。

### M1. 协议与模型切换完成

- 目标：完成 Rust `StreamFrontendEvent` 改造 + 前端 `ExecutionBlock` / `ChatMessage.payload` 定义
- 交付物：
  - `protocol.rs` 中 `FrontendBlock` + 新 `StreamFrontendEvent` 结构
  - `chat.ts` 类型与 store API 改造完成
  - 基础单元测试可运行
- 完成标志：
  - 前后端通过新的 blocks 事件格式通信
  - 前端可以用 mock 数据驱动新的 assistant message 结构

### M2. 新渲染链路可用

- 目标：assistant 输出正式切到 block 渲染
- 交付物：
  - `AssistantExecutionView.vue`（含 footer 统计）
  - `ExecutionThoughtBlock.vue`
  - `ExecutionToolBlock.vue`
  - `ExecutionTextBlock.vue`
  - `ExecutionNoticeBlock.vue`
  - `ExecutionPermissionBlock.vue`
- 完成标志：
  - 页面中 assistant 不再通过旧气泡聚合字段展示
  - 6 种 block 均可静态渲染

### M3. 流式输出贯通

- 目标：Rust 侧直接输出 blocks，前端消费并实时更新
- 交付物：
  - `protocol.rs::to_frontend_event()` 重写完成
  - `useStreamProcessor.ts` 新消费逻辑
  - 对应测试用例
- 完成标志：
  - 一轮 assistant 输出可以按 `thought -> tool -> permission -> text/notice` 的形式连续渲染

### M4. 持久化与回放完成

- 目标：将新结构写入数据库，并支持重新加载和导出
- 交付物：
  - Rust 侧消息保存结构更新
  - 前端加载解析逻辑更新
  - `exportMarkdown()` 适配 blocks 格式
  - 基础回归测试
- 完成标志：
  - 刷新页面后，assistant block 序列可以稳定恢复
  - Markdown 导出格式正确

## 14. 后续 Phase

本方案当前只定义 Phase 1 的 MVP 改造。Phase 1 完成后，建议继续按以下阶段推进。

### Phase 2：协议细化与边界事件

> **注意**：Phase 1 任务 1 已将 Rust `StreamFrontendEvent` 的 blocks 载体改造前移。Phase 2 的目标是进一步细化事件粒度——从「完整 blocks 数组一次性推送」演进为「block 粒度的增量事件」，使前端能实时展示 block 的流式生成过程。

- 目标：在 `process.rs` 中按 block 生命周期发射细粒度事件
- 主要工作：
  - 在 `process.rs` 的 Stdout Reader 中，为每个 `content_block_start` / `content_block_delta` / `content_block_stop` 发射独立事件
  - 事件类型对齐 Phase 1 已定义的事件协议表（第 6.1 节）：
    - `thought_start` / `thought_delta` / `thought_end`
    - `tool_start` / `tool_input` / `tool_result` / `tool_end`
    - `text_start` / `text_delta` / `text_end`
    - `permission_request` / `permission_resolved`
  - 前端 `useStreamProcessor.ts` 从「消费 blocks 数组」切换到「消费增量事件」模式
  - Phase 1 的 blocks 数组格式保留作为 `session_done` 时的完整快照（用于持久化）
- 改动文件：
  - `src-tauri/src/process.rs`：Stdout Reader 发射细粒度 Tauri 事件
  - `src-tauri/src/protocol.rs`：可能需要中间事件类型
  - `src/composables/useStreamProcessor.ts`：增量消费逻辑
- 预期收益：
  - block 逐个出现而非一次性渲染，视觉效果更接近 Claude Code
  - 前端状态机更精确（block 级 streaming 状态）
  - 长工具调用时用户可以看到输入参数实时生成

### Phase 3：体验增强

- 目标：进一步贴近 Claude Code 的阅读体验和交互细节
- 主要工作：
  - 工具块折叠状态记忆
  - 复制 `IN / OUT`
  - 长输出虚拟滚动或懒渲染
  - 更精细的时间轴与状态图标
- 预期收益：
  - 提升可读性
  - 提升长会话的使用体验

### Phase 4：检索与调试能力增强

- 目标：让结构化 block 不仅可看，还能被搜索、过滤和分析
- 主要工作：
  - 按 block 类型过滤
  - 搜索工具调用、输出内容和 notice
  - 为错误块和 stderr 输出增加快速定位入口
- 预期收益：
  - 输出流从“展示层能力”提升到“诊断层能力”

### Phase 5：历史迁移与数据治理

- 目标：若产品需要长期稳定演进，再处理旧数据迁移与结构治理
- 主要工作：
  - 设计旧消息到新 block 结构的迁移脚本
  - 为消息 schema 增加版本字段
  - 建立数据校验与修复工具
- 说明：
  - 本 Phase 明确不在当前方案范围内，仅作为未来演进方向保留

## 15. 结论

本方案建议直接放弃当前 assistant 消息的聚合展示方式，转向以 `ExecutionBlock` 为中心的结构化执行流模型。

这样做的好处是：

- 数据结构与目标展示形式一致
- 组件边界更清晰
- 后续持久化、回放、搜索、折叠控制更自然
- 更容易实现与 Claude Code 接近的输出体验

Phase 1 完成后，项目即可具备基础的 Claude Code 风格执行日志输出能力；后续再通过协议细化、体验增强与检索能力建设，逐步把输出系统从”可展示”推进到”可分析、可回放、可扩展”。

---

## 16. 修改记录

### 2026-06-20 — 方案审查修正（v1 → v2）

基于实际代码审查（`protocol.rs`、`chat.ts`、`useStreamProcessor.ts`、`tauri-bridge.ts`），对原方案进行以下修正：

| # | 修正项 | 修改前 | 修改后 | 原因 | 影响 |
|---|--------|--------|--------|------|------|
| 1 | **新增前置发现（第 3 节）** | 无 | 指出 Rust 侧已有 `ContentBlock` 枚举，`StreamFrontendEvent` 拍平了 blocks | 发现后修正 Phase 1 策略，避免前端兜圈子重建 | §3（新增）; §8 任务列表 |
| 2 | **Phase 1 纳入 Rust 改造** | Phase 1 只做前端映射，Phase 2 才改 Rust | Phase 1 任务 1 直接改造 `StreamFrontendEvent` 携带 blocks | 跳过”拍平→重建”的浪费，工作量更小 | `protocol.rs`; §8 任务 1 |
| 3 | **新增 `PermissionRequestBlock`** | 无，`control_request` 不在 block 模型中 | 新增 `kind: “permission”` block + 事件 `permission_request` / `permission_resolved`；resolution 明确 5 态枚举 | 权限审批是执行流一等公民，应内联渲染 | §5.1-5.2 类型; §6.1-6.2 事件协议; §7.1 组件表 |
| 4 | **`ToolBlock` 增加 `result` 字段 + 截断策略** | 只有 `input` / `output` | 新增 `result`、`resultIsError`、`requiresApproval`；写死 result 独立截断 2000 字符 + “Show full output” | CLI 工具结果以独立 `user/tool_result` 消息返回；不写死策略后续只实现 happy path | §5.2 ToolBlock 类型; §7.3 ExecutionToolBlock |
| 5 | **删除 `tool_output` 增量事件** | 假设工具输出是流式增量 | 改为单次 `tool_result` 事件 | CLI 实际行为：工具结果一次性完整返回 | §6.1-6.2 事件协议 |
| 6 | **删除 `ExecutionTimeline.vue`** | 列为可选组件 | 删除 | 7 个任务中零引用——ponytail：不存在的需求不写代码 | §7.1-7.2 组件清单 |
| 7 | **合并 `ExecutionFooterStats.vue`** | 独立组件 | 合并到 `AssistantExecutionView.vue` 内部 | ponytail：减少文件数，统计区逻辑简单不配独立文件 | §7.1-7.3 组件清单 |
| 8 | **新增 `ExecutionPermissionBlock.vue`** | 无 | 内联审批 UI，替代 ChatPanel 独立审批栏 | 对应 PermissionRequestBlock 模型 | §7.1-7.3 组件清单 |
| 9 | **`exportMarkdown()` 纳入任务** | 未提及 | 任务 2 中加入适配 | 改为 blocks 后旧导出逻辑编译报错 | §8 任务 2; `chat.ts` |
| 10 | **细化 Phase 2** | 一段话概述 | 详细说明 block 粒度增量事件、改动文件、Phase 1 格式保留策略 | Phase 2 设计影响 Phase 1 的 `StreamFrontendEvent` 结构（需保留 blocks 数组用于持久化快照） | §14; `process.rs`; `protocol.rs` |
| 11 | **更新风险 10.2** | “流式事件映射风险” | 风险降低——映射前移到 Rust 侧 | 直接利用已有 `ContentBlock` 结构 | §11 |
| 12 | **删除风险 10.5** | “后续协议演进风险” | 标记已消除 | Phase 1 已完成 Rust 协议改造 | §11 |
| 13 | **更新实施顺序** | 模型→组件→渲染→映射→持久化 | 协议→模型→组件→渲染→消费→持久化 | Rust 协议先定数据格式 | §12 |
| 14 | **更新里程碑 M1-M4** | 不含 Rust/权限/导出 | 新增 Rust 协议、PermissionBlock、exportMarkdown | 与更新后的任务对齐 | §13 |
| 15 | **设计原则第 5 条更新** | “前后端通过更明确的执行事件协议协作” | “Rust 侧 StreamFrontendEvent 直接携带 blocks，前端不做重建映射” | 更精确地描述修正后的策略 | §4 |
| 16 | **`StreamFrontendEvent` 保留字段** | 未说明 | 明确保留 `control_request`、`is_final`、`duration_ms`、`input_tokens`、`output_tokens`、`cost_usd`、`error` | 避免误删仍在使用的重要字段 | `protocol.rs`; §8 任务 1 |

### 修改统计

- 新增内容：~80 行（第 3 节前置发现 + Phase 2 细化 + 修改记录）
- 修改内容：~150 行（任务列表、事件协议、组件清单、风险、里程碑、实施顺序）
- 删除内容：~20 行（`ExecutionTimeline.vue`、`ExecutionFooterStats.vue`、`tool_output` 事件）
- 净增：~60 行

### 未改动内容

以下章节保持原方案不变：
- 第 1 节：改造目标
- 第 2 节：当前问题
- 第 4.1 节：基础类型（仅新增 `”permission”` 枚举值）
- 第 4.3 节：消息结构
- 第 4.4 节：示例（后续实施时根据实际 block 结构更新）
- Phase 3/4/5：保持概述级别，不对 Phase 1 构成反向依赖
