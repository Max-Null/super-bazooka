# 前端-Store

> Pinia 状态管理 — chat（消息流/工具调用/控制请求）、session（会话列表/CRUD）、settings（API 配置/UI 偏好/字号/主题/语言）。

## 功能说明

- chat store：消息状态管理（流式追加、去重、编辑、回滚、Markdown 导出）、工具调用追踪（Agent/Task 使用统计）、控制请求（工具审批）
- session store：会话 CRUD 状态、活跃会话切换、CLI 会话 UUID 映射
- settings store：API 配置（读写 `~/.claude/settings.json`）+ UI 偏好（localStorage 持久化），双向自动同步

## Store 数据流

```mermaid
graph TD
    subgraph "chat store"
        Messages[messages: Message[]]
        CurrentMsg[currentAssistantMsg]
        IsProcessing[isProcessing]
        PendingCR[pendingControlRequest]
        UsedAgents[usedAgents: Set]
    end

    subgraph "session store"
        Sessions[sessions: Session[]]
        ActiveId[activeSessionId]
    end

    subgraph "settings store"
        ApiConf[apiKey/baseUrl/model]
        UiConf[planMode/autoMode/permissionMode/effort/ponytailMode]
        Appearance[theme/locale/fontSize]
    end

    subgraph "持久化"
        SQLite[(SQLite via Rust)]
        LS[localStorage]
        SettingsJSON["~/.claude/settings.json"]
    end

    Messages --> SQLite
    Sessions --> SQLite
    ApiConf --> SettingsJSON
    UiConf --> LS
    Appearance --> LS
```

## 公开 API

| 类型 | 名称 | 说明 | 文件 |
|------|------|------|------|
| store | useChatStore | messages / currentAssistantMsg / isProcessing / pendingControlRequest / usedAgents + addUserMessage / startAssistantMessage / appendText / appendThinking / addToolUse / addControlRequest / resolveControlRequest / finishAssistantMessage / clearMessages / loadMessages / updateMessage / truncateFromIndex / truncateAfterMessage / exportMarkdown | src/stores/chat.ts |
| store | useSessionStore | sessions / activeSessionId + loadSessions / createSession / setActiveSession / renameSession / deleteSession / setClaudeSessionId / getActiveClaudeSessionId | src/stores/session.ts |
| store | useSettingsStore | apiKey / baseUrl / model / planMode / autoMode / permissionMode / effort / ponytailMode / theme / locale / fontSize / claudePath / resolvedClaudePath | src/stores/settings.ts |

## 配置属性

### `localStorage.*`

| 配置键 | 类型 | 默认值 | 必填 | 说明 |
|--------|------|--------|------|------|
| `localStorage.cc-gui-ui-settings` | `object` | `{ planMode: false, autoMode: true, permissionMode: "bypassPermissions", effort: "high", ponytailMode: "full", theme: "dark", locale: "zh", fontSize: "medium", claudePath: "" }` | 否 | UI 偏好持久化键 |

## 代码示例

### chat store — 消息流追加

```typescript
// stores/chat.ts
export const useChatStore = defineStore("chat", () => {
  const messages = ref<Message[]>([]);
  const currentAssistantMsg = ref<Message | null>(null);

  function startAssistantMessage(): string {
    const msg: Message = { id: genId(), role: "assistant", content: "", thinking: "", toolUses: [], timestamp: Date.now(), isStreaming: true };
    currentAssistantMsg.value = msg;
    messages.value.push(msg);
    return msg.id;
  }

  function appendText(text: string) {
    if (currentAssistantMsg.value) currentAssistantMsg.value.content += text;
  }

  function addToolUse(tool: ToolUse) {
    if (currentAssistantMsg.value) currentAssistantMsg.value.toolUses.push(tool);
    // 追踪 Agent/Task 使用
    if ((tool.name === "Agent" || tool.name === "Task") && tool.input) {
      const agentType = tool.input.subagent_type;
      if (agentType) usedAgents.value = new Set([...usedAgents.value, agentType]);
    }
  }

  function finishAssistantMessage(durationMs?, inputTokens?, outputTokens?, costUSD?) {
    if (currentAssistantMsg.value) {
      currentAssistantMsg.value.isStreaming = false;
      currentAssistantMsg.value.durationMs = durationMs;
      currentAssistantMsg.value.inputTokens = inputTokens;
      currentAssistantMsg.value.outputTokens = outputTokens;
      currentAssistantMsg.value.costUSD = costUSD;
    }
    currentAssistantMsg.value = null;
    isProcessing.value = false;
  }

  return { messages, currentAssistantMsg, isProcessing, usedAgents, addUserMessage, startAssistantMessage, appendText, addToolUse, finishAssistantMessage, clearMessages, exportMarkdown };
});
```

### settings store — 双向配置同步

```typescript
// stores/settings.ts
// 启动时从 ~/.claude/settings.json 加载
getClaudeSettings().then(s => {
  apiKey.value = s.api_key;
  baseUrl.value = s.base_url;
  model.value = s.model;
});

// 变更时自动写回（debounce via watch）
watch([apiKey, baseUrl, model, effort, planMode, autoMode, permissionMode],
  ([k, u, m, e]) => setClaudeSettings(k, u, m, e, resolvePermissionMode())
);

// UI 偏好 → localStorage
watch([planMode, autoMode, permissionMode, effort, theme, locale, fontSize],
  () => localStorage.setItem(STORAGE_KEY, JSON.stringify(uiState))
);
```

## 依赖说明

### 内部依赖

| 模块 | 说明 |
|------|------|
| `前端-Lib` | tauri-bridge（所有 IPC 调用） |

### 外部依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| `vue` | ^3.5.35 | ref / watch |
| `pinia` | ^3.0.4 | defineStore |

<!-- @generated v0.5.1 -->
<!-- @baseline commit=f67115370991f3521ab8aece00f990d651886eac generated=2026-06-26T12:00:00+08:00 -->
