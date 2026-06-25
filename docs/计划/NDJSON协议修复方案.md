# NDJSON 交互协议修复方案

> 基于 [CCGUI Interactive Protocol Report](#) 对 cc-gui 当前实现的完整审计。
>
> 审计日期：2026-06-26
> 实施日期：2026-06-26
> **状态：P0 ✅ 完成 / P1 ✅ 完成 / P2 延后**

---

## 〇、实施结果

| 阶段 | 项 | 状态 | 验证 |
|------|----|------|------|
| P0-1 | CLI flags（`--input-format stream-json` `--permission-prompt-tool stdio` 去 `--print`） | ✅ | control_request 成功到达前端 |
| P0-2 | control_response 格式（嵌套 `response.subtype/request_id/response.behavior/updatedInput/message`） | ✅ | 允许/拒绝均成功，CC 正确响应 |
| P0-2 | 用户消息改 stdin NDJSON | ✅ | CC 正常处理 prompt |
| P0-2 | `set_permission_mode` 初始化请求 | ✅ | 权限模式同步 |
| P1-1 | hook_callback → 前端审批（非自动允许） | ✅ | 安全审查后修正 |
| P1-2 | AskUserQuestion 问答弹窗 | ✅ | radio/checkbox/Other 可用 |
| P1-3 | interrupt 优雅停止（先发控制请求再 kill） | ✅ | 3 秒超时兜底 |
| P1-4 | deny message + allow updatedInput | ✅ | 含在 P0-2 |
| 附 | 工具名 i18n（Bash→命令行等） | ✅ | 审批栏+消息气泡 |
| 附 | 思考计时重构（各段 thinkingDurationMs 求和） | ✅ | 去实时计时器 |

---

## 〇、交叉验证结论

方案制定后联网查了三个来源做交叉验证：

| 来源 | 类型 | 覆盖范围 |
|------|------|---------|
| [Claude Code Agent SDK 官方文档](https://code.claude.com/docs/en/agent-sdk/user-input) | 官方 | SDK API（`canUseTool` 回调），非 NDJSON 协议层 |
| [claude-cli-agent-protocol skill](https://raw.githubusercontent.com/NeverSight/skills_feed/refs/heads/main/data/skills-md/bohdan-shulha/skills/claude-cli-agent-protocol/SKILL.md) | 社区 | NDJSON stream-json 协议（更接近 cc-gui 场景） |
| CCGUI Interactive Protocol Report | 定制分析 | 基于 CC v2.1.181，最全面 |

### 确认的项

| 方案项 | 验证结果 | 依据 |
|--------|---------|------|
| `--permission-prompt-tool stdio` | ✅ 必须加 | 官方 SDK 文档 + 社区 skill 一致确认 |
| `--input-format stream-json` | ✅ 必须加 | 社区 skill 确认，`--output-format` 的 stdin 对应项 |
| control_response 嵌套格式 `{response:{subtype,request_id,response:{behavior,...}}}` | ✅ 正确 | 社区 skill 的 NDJSON 示例与报告一致 |
| `allow` 必须含 `updatedInput` | ✅ 正确 | 官方 SDK 文档明确要求 |
| `deny` 必须含 `message` | ✅ 正确 | 官方 SDK 文档明确要求 |
| `AskUserQuestion` 答案格式 `{questions:[...],answers:{...}}` | ✅ 正确 | 官方 SDK 文档有完整示例 |

### 修正的项

| 方案原内容 | 验证后修正 | 原因 |
|-----------|-----------|------|
| `initialize` 握手是 P0 必须 | **降为 P2**（防御性处理） | 社区 skill 明确说"无显式握手，第一条消息是 system"，`initialize` 子类型可能是 SDK 内部协议而非 CLI 协议 |
| `hook_callback` 是 P0 | **降为 P1**（观察到再修） | 社区 skill 不包含此协议。如果用户没配 hooks 就不会触发 |
| `ExitPlanMode` issue | **升级关注** | 官方 GitHub issue #39666 确认存在 bug：`control_response` for `ExitPlanMode` 会被静默忽略，进程无限挂起 |

### SDK API vs NDJSON 协议的区别

**重要**：官方文档描述的是 **Agent SDK API**（TypeScript/Python 回调），cc-gui 走的是 **CLI headless NDJSON** 模式。两者的 control_response 格式不同：

| 层级 | 格式 |
|------|------|
| **SDK API**（官方文档） | `return { behavior: "allow", updatedInput: input }` — SDK 内部处理序列化 |
| **NDJSON**（cc-gui 场景） | `{"type":"control_response","response":{"subtype":"success","request_id":"...","response":{"behavior":"allow","updatedInput":{...}}}}` — 原始 JSON 写入 stdin |

cc-gui 需要实现的是 NDJSON 格式，方案中的格式是正确的。

---

## 一、问题概述

用户反馈：CC 需要授权时 GUI 无法提供交互，会话卡死。

审计发现：17 项检查中 5 项通过、10 项缺失、2 项部分实现。根因是 cc-gui 与 Claude Code CLI 的 NDJSON 控制协议对接不完整。

---

## 二、当前状态总览

| # | 检查项 | 状态 | 严重性 |
|---|--------|------|--------|
| 1 | `--output-format stream-json` | ✅ | -- |
| 2 | `--input-format stream-json` | ❌ | **P0** |
| 3 | `--permission-prompt-tool stdio` | ❌ | **P0** |
| 4 | `--include-partial-messages` | ✅ | -- |
| 5 | 解析 `control_request` | ✅ | -- |
| 6 | 处理 `initialize` 握手（防御性） | ❌ | P2 |
| 7 | `allow`/`deny` 使用 `behavior` 字段 | ⚠️ 用了 `response` | **P0** |
| 8 | `control_response` 含 `request_id` | ❌ | **P0** |
| 9 | `control_response` 含 `subtype` | ❌ | P1 |
| 10 | `allow` 响应含 `updatedInput` | ❌ | P1 |
| 11 | `deny` 响应含 `message` | ❌ | P1 |
| 12 | AskUserQuestion 问答 UI | ❌ | P1 |
| 13 | ExitPlanMode 特殊处理 | ❌ | P2 |
| 14 | hook_callback 基础响应 | ❌ | P1 |
| 15 | 停止按钮发 interrupt | ❌ | P1 |
| 16 | stdin 写 `\n` 结尾 | ✅ | -- |
| 17 | 终止进程前优雅中断 | ❌ | P2 |

---

## 三、分阶段修复计划

### P0 — 必须修（阻塞核心交互）

#### P0-1: 补全 CLI 启动参数

**文件**: `src-tauri/src/process.rs:306-311`

**现状**:
```rust
let mut args = vec![
    "--print".to_string(),
    "--output-format".to_string(), "stream-json".to_string(),
    "--verbose".to_string(),
    "--include-partial-messages".to_string(),
];
```

**修改**: 增加 `--input-format stream-json` 和 `--permission-prompt-tool stdio`。

**影响**: 不加这两个 flag，CLI 不会以 NDJSON 格式接收 stdin 控制响应，也不会通过 stdout 发送 `can_use_tool` 审批请求。

---

#### P0-2: 修正 control_response 格式

**文件**: `src/components/chat/ChatPanel.vue:281-289`、`src/stores/chat.ts`、`src-tauri/src/protocol.rs`

**现状**（前端发给 Rust 后端）:
```json
{"type":"control_response","response":"allow"}
```

**协议规范格式**（NDJSON 层级，非 SDK API）:
```json
{
  "type": "control_response",
  "response": {
    "subtype": "success",
    "request_id": "<匹配的 request_id>",
    "response": {
      "behavior": "allow",
      "updatedInput": { ... }
    }
  }
}
```

**需要改动**:
1. `protocol.rs`: 解析 `control_request` 时**提取并传递 `request_id`** 和 `updatedInput` 到前端
2. `chat` store: 存储当前 pending 控制请求的 `request_id` + `updatedInput`
3. `ChatPanel.vue`: `handleAllow`/`handleDeny` 构造符合规范的完整 JSON

---

### P1 — 应该修（影响体验 / 特定场景阻塞）

#### P1-1: 实现 hook_callback 基础响应

**文件**: `src-tauri/src/protocol.rs`

**机制**: 用户配置了 hooks 时，CC 到对应生命周期会发送 `control_request(subtype: "hook_callback")`。GUI 必须回复否则会话阻塞。

**方案**: 后端识别 `hook_callback` 子类型时直接回复空 `control_response`（不阻塞执行），将 hook 事件转发前端展示。

```json
{
  "type": "control_response",
  "response": {
    "subtype": "success",
    "request_id": "<request_id>",
    "response": { "behavior": "allow" }
  }
}
```

后期可扩展为可阻断 hook 展示审批 UI。

---

#### P1-2: AskUserQuestion 问答 UI

**文件**: `src/components/chat/ChatPanel.vue`、新增 `src/components/chat/QuestionModal.vue`

**现状**: `can_use_tool` 中 `tool_name === "AskUserQuestion"` 也走通用审批栏，用户只能点"允许"/"拒绝"。

**方案**:
1. 识别 `tool_name === "AskUserQuestion"` → 渲染问答弹窗（而非审批栏）
2. 弹窗展示 `questions[]` 数组，每个问题：`header` 标签 + `question` 文本 + `options` 选项组
3. 单选用 radio、多选用 checkbox，均追加 "Other" 自由输入
4. 提交时构造 `answers` 对象（key 精确匹配 question 原文、value 为 label 或 label[]）
5. 如果用户选了 "Other"，用自由文本作为 answer value（不是字符串 "Other"）

**答案格式**（官方文档确认）:
```json
{
  "behavior": "allow",
  "updatedInput": {
    "questions": [ /* 原样回传 */ ],
    "answers": {
      "How should I format the output?": "Summary",
      "Which sections should I include?": ["Introduction", "Conclusion"]
    }
  }
}
```

#### P1-3: 停止按钮发 interrupt

**文件**: `src/components/chat/ChatPanel.vue:330-334`、`src-tauri/src/process.rs`

**现状**: 停止按钮直接调用 `stopSession()` → `pm.kill()` 杀进程，不走 interrupt。

**方案**:
1. 用户点击"停止"→ 先发 `control_request(subtype: "interrupt")` 到 stdin
2. 等待 3 秒（让 CC 优雅中断）
3. 超时未退出再 `pm.kill()`

#### P1-4: deny 加 message、allow 回传 updatedInput

**文件**: `src/components/chat/ChatPanel.vue`

**方案**:
- `handleDeny()` → `message: "User denied this action"`
- `handleAllow()` → 原样回传 `updatedInput`（从 store 中取）

---

### P2 — 可延后（功能完善）

| 项 | 说明 |
|----|------|
| `initialize` 握手防御性处理 | 社区 skill 说"无显式握手"，但如果 CC 发了就回复 `control_response(subtype: "success")` |
| **ExitPlanMode 计划审批面板** | ⚠️ 官方 Issue #39666: `control_response` for ExitPlanMode 会被静默忽略导致无限挂起，需测试确认后特殊处理 |
| SubagentStart/Stop 进度展示 | 长时间无响应时用户误以为卡死 |
| FileChanged 冲突提示 | 外部修改静默覆盖 |
| PermissionDenied 反馈 | 用户不知操作为何失败 |
| OAuth 认证流程 | MCP server 登录场景 |
| 其他 control_request 子类型 | set_model, get_models, set_permission_mode 等 |

---

## 四、数据流变更

修复后的完整审批流：

```
CC stdout: control_request(can_use_tool, request_id, tool_name, input)
    │
    ▼
protocol.rs: parse → 提取 request_id, tool_name, input, updatedInput
    │
    ▼
Tauri event "stream-event" → useStreamProcessor.ts
    │
    ▼
chat store: setPendingControlRequest({ request_id, tool_name, input, updatedInput })
    │
    ▼
ChatPanel.vue 渲染:
    ├─ tool_name === "AskUserQuestion" → QuestionModal
    ├─ tool_name === "ExitPlanMode"     → PlanApprovalPanel
    └─ 其他                             → 审批栏 (Allow/Deny)
    │
    ▼
用户操作 → handleAllow() / handleDeny()
    │
    ▼
sendStdin(JSON.stringify({
  type: "control_response",
  response: {
    subtype: "success",
    request_id: "<匹配>",
    response: {
      behavior: "allow" | "deny",
      updatedInput: { ... },    // allow 时回传
      message: "..."             // deny 时必填
    }
  }
}))
    │
    ▼
Rust stdin_manager.send() → stdin.write_all(data + "\n")
```

## 五、实施步骤

按依赖关系排序：

**第一次提交（解决交互卡死）**:
1. **P0-1** 补 CLI flags（独立，无依赖）
2. **P0-2** 修 control_response 格式（依赖 protocol.rs 传递 request_id → chat store → ChatPanel）

**第二次提交（完善交互体验）**:
3. **P1-1** hook_callback 基础响应（独立，后端自动回复 allow）
4. **P1-2** AskUserQuestion UI（依赖 P0-2）
5. **P1-3** interrupt 停止流程（依赖 P0-2）
6. **P1-4** deny message + updatedInput 回传（依赖 P0-2）

**P2 延后**:
- initialize 防御性处理、ExitPlanMode 面板、Subagent 进度等

P0-1 和 P0-2 可以并行修改（无相互依赖），预计一次 commit 完成。
