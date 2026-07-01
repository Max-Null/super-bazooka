import { defineStore } from "pinia";
import { ref, computed } from "vue";

export interface AttachedFile {
  name: string;
  path: string;
}

export interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
  thinking: string;
  toolUses: ToolUse[];
  timestamp: number;
  isStreaming: boolean;
  durationMs?: number;
  inputTokens?: number;
  outputTokens?: number;
  costUSD?: number;
  attachments?: AttachedFile[];
  /** 用户手动停止（非自然结束） */
  wasStopped?: boolean;
}

export interface ToolUse {
  id: string;
  name: string;
  input: Record<string, unknown>;
  result?: string;
  isError?: boolean;
  /** 该工具调用前的思考耗时（毫秒） */
  thinkingDurationMs?: number;
  /** 该工具执行耗时（毫秒），从工具调用到下一个思考/文本开始的间隔 */
  executionDurationMs?: number;
  /** 工具开始执行的时间戳（Date.now()），用于流式期间显示实时计时 */
  startedAt?: number;
}

export interface ControlRequest {
  subtype: string;
  tool_name?: string;
  tool_input: Record<string, unknown>;
  /** 控制请求的唯一 ID，响应时必须原样带回 */
  request_id?: string;
  /** Resolved: 'allow' | 'deny' | null (pending) */
  resolution?: string;
}

function genId(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 9);
}

export const useChatStore = defineStore("chat", () => {
  const messages = ref<Message[]>([]);
  const currentAssistantMsg = ref<Message | null>(null);
  const isProcessing = ref(false);
  // 审批队列：防止子 agent 并发 control_request 互相覆盖
  const pendingControlRequests = ref<ControlRequest[]>([]);
  // 兼容旧引用：队列头即当前待审批项
  const pendingControlRequest = computed(() =>
    pendingControlRequests.value.length > 0 ? pendingControlRequests.value[0] : null
  );

  // 会话消息缓存：切换会话时保留进行中的流式消息（DB 只有已完成的消息）
  const sessionCache = new Map<string, Message[]>();

  /** 将当前消息深拷贝存入缓存（切换会话前调用） */
  function saveSessionCache(sessionId: string) {
    if (!sessionId) return;
    // JSON 深拷贝，避免引用污染（ponytail: WebView2 不保证 structuredClone）
    sessionCache.set(sessionId, JSON.parse(JSON.stringify(messages.value)));
  }

  /** 从缓存恢复消息；缓存无数据则返回 null */
  function loadFromCache(sessionId: string): Message[] | null {
    return sessionCache.get(sessionId) ?? null;
  }

  /**
   * 处理后台会话的流式事件（当前活跃会话不是该 session 时调用）。
   * 将增量数据写入 sessionCache，切回时 loadFromCache 即可恢复完整状态。
   */
  // ponytail: event 用 any 避免跨模块类型依赖
  function handleBackgroundStreamEvent(sessionId: string, event: any) {
    const cached = sessionCache.get(sessionId) || [];
    let last = cached[cached.length - 1];

    if (event.type === 'assistant') {
      // 确保有进行中的 assistant 消息
      if (!last || last.role !== 'assistant' || !last.isStreaming) {
        last = {
          id: genId(), role: 'assistant', content: '', thinking: '',
          toolUses: [], timestamp: Date.now(), isStreaming: true,
        };
        cached.push(last);
      }
      // 文本去重（同 useStreamProcessor 逻辑）
      if (event.text) {
        if (last.content && event.text.startsWith(last.content)) {
          const newPart = event.text.slice(last.content.length);
          if (newPart) last.content += newPart;
        } else {
          last.content += event.text;
        }
      }
      // thinking 去重
      if (event.thinking) {
        if (last.thinking && event.thinking.startsWith(last.thinking)) {
          const newPart = event.thinking.slice(last.thinking.length);
          if (newPart) last.thinking += newPart;
        } else {
          last.thinking += event.thinking;
        }
      }
      if (event.tool_use) {
        for (const tu of event.tool_use) {
          last.toolUses.push({ id: tu.id, name: tu.name, input: tu.input || {} });
        }
      }
      if (event.input_tokens != null) last.inputTokens = event.input_tokens;
      if (event.output_tokens != null) last.outputTokens = event.output_tokens;
      if (event.cost_usd != null) last.costUSD = event.cost_usd;
    } else if (event.type === 'result' || event.type === 'done') {
      if (last && last.isStreaming) {
        last.isStreaming = false;
        last.durationMs = event.duration_ms;
        last.inputTokens = event.input_tokens ?? last.inputTokens;
        last.outputTokens = event.output_tokens ?? last.outputTokens;
        last.costUSD = event.cost_usd ?? last.costUSD;
      }
    } else if (event.type === 'error') {
      if (last && last.isStreaming) {
        last.content += `\n\n> ⚠️ ${event.error || 'Unknown error'}`;
        last.isStreaming = false;
      }
    } else if (event.type === 'token_usage') {
      if (last) {
        if (event.input_tokens != null) last.inputTokens = event.input_tokens;
        if (event.output_tokens != null) last.outputTokens = event.output_tokens;
      }
    }

    sessionCache.set(sessionId, cached);
  }

  function addUserMessage(content: string, attachments?: AttachedFile[]): string {
    const id = genId();
    messages.value.push({
      id,
      role: "user",
      content,
      thinking: "",
      toolUses: [],
      timestamp: Date.now(),
      isStreaming: false,
      attachments: attachments?.length ? attachments : undefined,
    });
    return id;
  }

  function startAssistantMessage(): string {
    const id = genId();
    const msg: Message = {
      id,
      role: "assistant",
      content: "",
      thinking: "",
      toolUses: [],
      timestamp: Date.now(),
      isStreaming: true,
    };
    currentAssistantMsg.value = msg;
    messages.value.push(msg);
    isProcessing.value = true;  // 中途发送场景：CC 完成上一轮后继续下一轮时恢复 isProcessing
    return id;
  }

  function appendText(text: string) {
    if (!currentAssistantMsg.value) startAssistantMessage();
    currentAssistantMsg.value!.content += text;
  }

  function appendThinking(thinking: string) {
    if (!currentAssistantMsg.value) startAssistantMessage();
    currentAssistantMsg.value!.thinking += thinking;
  }

  function addToolUse(tool: ToolUse) {
    if (!currentAssistantMsg.value) startAssistantMessage();
    currentAssistantMsg.value!.toolUses.push(tool);
    // 追踪 agent 使用：tool name 可能是 Agent 或 Task
    if ((tool.name === "Agent" || tool.name === "Task") && tool.input) {
      const agentType = (tool.input as any).subagent_type || (tool.input as any).agent_type;
      if (agentType && typeof agentType === "string") {
        const next = new Set(usedAgents.value);
        next.add(agentType);
        usedAgents.value = next;
      }
    }
  }

  function markStopped() {
    if (currentAssistantMsg.value) {
      currentAssistantMsg.value.wasStopped = true;
    }
  }

  function finishAssistantMessage(durationMs?: number, inputTokens?: number, outputTokens?: number, costUSD?: number) {
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

  function addControlRequest(cr: ControlRequest) {
    pendingControlRequests.value = [...pendingControlRequests.value, cr];
  }

  function resolveControlRequest(resolution: string) {
    if (pendingControlRequests.value.length > 0) {
      pendingControlRequests.value[0].resolution = resolution;
      pendingControlRequests.value = pendingControlRequests.value.slice(1);
    }
  }

  /** 当前会话中使用过的 agent 类型（如 "pr-review-toolkit:code-simplifier"） */
  const usedAgents = ref<Set<string>>(new Set());

  function clearMessages() {
    messages.value = [];
    currentAssistantMsg.value = null;
    isProcessing.value = false;
    pendingControlRequests.value = [];
    usedAgents.value = new Set();
  }

  /** Update a specific message's content (for edit) */
  function updateMessage(id: string, content: string) {
    const msg = messages.value.find(m => m.id === id);
    if (msg) msg.content = content;
  }

  /** Remove all messages after (and including) the given index. Returns the removed count. */
  function truncateFromIndex(index: number): number {
    if (index < 0 || index >= messages.value.length) return 0;
    const removed = messages.value.length - index;
    messages.value.splice(index, removed);
    if (currentAssistantMsg.value && !messages.value.includes(currentAssistantMsg.value)) {
      currentAssistantMsg.value = null;
    }
    return removed;
  }

  /** Remove all messages after the given message ID (exclusive — keeps the message itself) */
  function truncateAfterMessage(id: string): number {
    const idx = messages.value.findIndex(m => m.id === id);
    if (idx < 0) return 0;
    return truncateFromIndex(idx + 1);
  }

  /**
   * Build an export Markdown string for the current session.
   * Includes thinking blocks, tool uses, and token stats.
   */
  function exportMarkdown(sessionTitle: string): string {
    const lines: string[] = [];
    lines.push(`# ${sessionTitle}`);
    lines.push(`> Exported at ${new Date().toISOString().slice(0, 19).replace('T', ' ')}`);
    lines.push('');
    for (const msg of messages.value) {
      const roleLabel = msg.role === 'user' ? '## You' : '## Claude';
      lines.push(roleLabel);
      if (msg.thinking) {
        lines.push('');
        lines.push('> **Thinking:**');
        lines.push('> ' + msg.thinking.replace(/\n/g, '\n> '));
      }
      if (msg.toolUses.length > 0) {
        lines.push('');
        for (const tu of msg.toolUses) {
          lines.push(`> 🔧 **${tu.name}**`);
          const preview = JSON.stringify(tu.input, null, 2);
          lines.push('> ```json');
          lines.push('> ' + preview.replace(/\n/g, '\n> '));
          lines.push('> ```');
          if (tu.result) {
            const shortResult = tu.result.length > 500 ? tu.result.slice(0, 500) + '…' : tu.result;
            lines.push('> **Result:**');
            lines.push('> ```');
            lines.push('> ' + shortResult.replace(/\n/g, '\n> '));
            lines.push('> ```');
          }
        }
      }
      if (msg.content) {
        lines.push('');
        lines.push(msg.content);
      }
      if (msg.role === 'assistant' && !msg.isStreaming) {
        const stats: string[] = [];
        if (msg.durationMs) stats.push(`⏱ ${(msg.durationMs / 1000).toFixed(1)}s`);
        if (msg.inputTokens) stats.push(`↑${msg.inputTokens}`);
        if (msg.outputTokens) stats.push(`↓${msg.outputTokens}`);
        if (msg.costUSD !== undefined) stats.push(`$${msg.costUSD.toFixed(4)}`);
        if (stats.length) lines.push(`\n*${stats.join(' | ')}*`);
      }
      lines.push('');
      lines.push('---');
      lines.push('');
    }
    return lines.join('\n');
  }

  /** Restore messages from database records */
  function loadMessages(records: Array<{ id: string; role: string; content: string; created_at: string }>) {
    clearMessages();
    for (const rec of records) {
      let textContent = rec.content;
      let thinking = "";
      let toolUses: ToolUse[] = [];
      let durationMs: number | undefined;
      let inputTokens: number | undefined;
      let outputTokens: number | undefined;
      let costUSD: number | undefined;

      // Try to parse JSON for assistant messages (new format)
      let attachments: AttachedFile[] | undefined;

      // Try JSON parse for both user and assistant messages
      try {
        const parsed = JSON.parse(rec.content);
        if (parsed && typeof parsed === "object") {
          if (rec.role === "assistant") {
            textContent = parsed.text || "";
            thinking = parsed.thinking || "";
            toolUses = parsed.toolUses || [];
            durationMs = parsed.durationMs;
            inputTokens = parsed.inputTokens;
            outputTokens = parsed.outputTokens;
            costUSD = parsed.costUSD;
          } else if (rec.role === "user" && Array.isArray(parsed.attachments)) {
            textContent = parsed.text || "";
            attachments = parsed.attachments;
          }
        }
      } catch {
        // Old format: plain text, use as-is
      }

      messages.value.push({
        id: rec.id,
        role: rec.role as "user" | "assistant",
        content: textContent,
        thinking,
        toolUses,
        timestamp: new Date(rec.created_at + "Z").getTime(),
        isStreaming: false,
        durationMs,
        inputTokens,
        outputTokens,
        costUSD,
        attachments,
      });
    }
  }

  return {
    messages,
    currentAssistantMsg,
    isProcessing,
    pendingControlRequest,
    pendingControlRequests,
    addUserMessage,
    startAssistantMessage,
    appendText,
    appendThinking,
    addToolUse,
    addControlRequest,
    resolveControlRequest,
    markStopped,
    finishAssistantMessage,
    usedAgents,
    clearMessages,
    loadMessages,
    saveSessionCache,
    loadFromCache,
    handleBackgroundStreamEvent,
    sessionCache,
    updateMessage,
    truncateFromIndex,
    truncateAfterMessage,
    exportMarkdown,
  };
});
