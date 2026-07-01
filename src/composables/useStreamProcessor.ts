import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useI18n } from "vue-i18n";
import { useChatStore, type ToolUse } from "@/stores/chat";
import { useSessionStore } from "@/stores/session";
import { useDebugLog } from "@/composables/useDebugLog";
import { useStderrLog } from "@/composables/useStderrLog";
import { storeClaudeSession, saveMessage, saveSessionDebugLog, saveSessionStderrLog, type StreamEvent, type ProcessExitedEvent } from "@/lib/tauri-bridge";
import { translateError } from "@/lib/utils";

let unlisten: UnlistenFn | null = null;
let unlistenDebug: UnlistenFn | null = null;
let unlistenError: UnlistenFn | null = null;
let unlistenSession: UnlistenFn | null = null;
let unlistenProcessExit: UnlistenFn | null = null;

function notifyComplete(durationMs?: number, inputTokens?: number, outputTokens?: number) {
  if (!("Notification" in window)) return;
  if (Notification.permission === "denied") return;
  const body = [
    durationMs ? `${(durationMs / 1000).toFixed(1)}s` : "",
    inputTokens ? `↑${inputTokens}` : "",
    outputTokens ? `↓${outputTokens}` : "",
  ].filter(Boolean).join(" · ");
  if (Notification.permission === "granted") {
    new Notification("cc-gui — Done", { body: body || undefined, silent: true });
  } else {
    Notification.requestPermission().then(p => {
      if (p === "granted") new Notification("cc-gui — Done", { body: body || undefined, silent: true });
    });
  }
}

interface SessionCreatedPayload {
  ourId: string;
  claudeSessionId: string;
  mcpServers?: string[];
}

export function useStreamProcessor() {
  const chat = useChatStore();
  const session = useSessionStore();
  const debugLog = useDebugLog();
  const stderrLog = useStderrLog();
  const { t } = useI18n();

  // 分阶段计时：思考 ↔ 工具执行
  let thinkingStart = 0;
  let toolExecStart = 0;
  /** 最后一个工具调用的引用，用于回填执行耗时 */
  let lastToolUse: ToolUse | null = null;

  function markThinkingStart() {
    if (!thinkingStart) thinkingStart = Date.now();
    // 思考开始 = 上一个工具执行结束
    if (toolExecStart && lastToolUse) {
      lastToolUse.executionDurationMs = Date.now() - toolExecStart;
      toolExecStart = 0;
      lastToolUse = null;
    }
  }
  function popThinkingDuration(): number {
    const dur = thinkingStart ? Date.now() - thinkingStart : 0;
    thinkingStart = 0;
    return dur;
  }
  function markToolExecStart(tool: ToolUse) {
    toolExecStart = Date.now();
    tool.startedAt = toolExecStart;  // 供 MessageBubble 显示实时计时
    lastToolUse = tool;
  }

  async function startListening() {
    if (unlisten) return;

    unlisten = await listen<StreamEvent>("stream-event", (event) => {
      const data = event.payload;
      debugLog.add(`📨 event: ${data.type} | sid=${data.session_id} | text=${(data.text||'').slice(0,50)} | thinking=${(data.thinking||'').slice(0,50)} | final=${data.is_final}`);

      // 事件是否属于当前活跃会话（CC 或 Zen）
      const isActive = data.session_id === session.activeSessionId
        || data.session_id === session.zenActiveId;

      // 事件属于后台会话 → 写入缓存，更新 activity 指示器
      if (data.session_id && !isActive) {
        chat.handleBackgroundStreamEvent(data.session_id, data);
        if (data.type === 'control_request') {
          session.setSessionActivity(data.session_id, 'blocked');
        } else if (data.type === 'result' || data.type === 'done') {
          // 不在 blocked 时降级为 unread
          if (session.sessionActivity[data.session_id] !== 'blocked') {
            session.setSessionActivity(data.session_id, 'unread');
          }
        } else if (data.type === 'assistant') {
          if (session.sessionActivity[data.session_id] !== 'blocked') {
            session.setSessionActivity(data.session_id, 'processing');
          }
        }
        return;
      }

      switch (data.type) {
        case "assistant":
          // 活跃会话处理中（不被 blocked 覆盖）
          if (data.session_id && session.sessionActivity[data.session_id] !== 'blocked') {
            session.setSessionActivity(data.session_id, 'processing');
          }
          if (data.text || data.thinking) markThinkingStart();
          if (data.text) {
            // 去重：当完整 assistant 事件携带的文本以已有内容开头时，
            // 说明之前已通过 text_delta 增量事件接收过，只追加新后缀。
            // 对于不发送增量事件的后端（DeepSeek），currentContent 为空，
            // 完整事件的文本会被完整使用。
            const currentContent = chat.currentAssistantMsg?.content || "";
            if (currentContent && data.text.startsWith(currentContent)) {
              const newPart = data.text.slice(currentContent.length);
              if (newPart) chat.appendText(newPart);
            } else {
              chat.appendText(data.text);
            }
          }
          if (data.thinking) {
            // 同样的去重逻辑处理 thinking 块
            const currentThinking = chat.currentAssistantMsg?.thinking || "";
            if (currentThinking && data.thinking.startsWith(currentThinking)) {
              const newPart = data.thinking.slice(currentThinking.length);
              if (newPart) chat.appendThinking(newPart);
            } else {
              chat.appendThinking(data.thinking);
            }
          }
          if (data.tool_use) {
            for (const tu of data.tool_use) {
              // 记录该工具调用前的思考耗时，然后重置计时器
              const thinkingDur = popThinkingDuration();
              const toolUse: ToolUse = {
                id: tu.id,
                name: tu.name,
                input: tu.input,
                thinkingDurationMs: thinkingDur,
              };
              chat.addToolUse(toolUse);
              markToolExecStart(toolUse);
            }
          }
          // assistant 事件携带 message.usage——实时更新 token 统计（DeepSeek 后端 result 可能不含 usage）
          if (chat.currentAssistantMsg) {
            if (data.input_tokens != null) chat.currentAssistantMsg.inputTokens = data.input_tokens;
            if (data.output_tokens != null) chat.currentAssistantMsg.outputTokens = data.output_tokens;
            if (data.cost_usd != null) chat.currentAssistantMsg.costUSD = data.cost_usd;
          }
          break;

        case "control_request":
          // 需要用户审批/问答 → 橙点（最高优先级）
          if (data.session_id) session.setSessionActivity(data.session_id, 'blocked');
          if (data.control_request) {
            const cr = data.control_request;
            debugLog.add(`  🔐 subtype=${cr.subtype} tool=${cr.tool_name} request_id=${cr.request_id}`);
            debugLog.add(`  🔐 tool_input keys: ${cr.tool_input ? Object.keys(cr.tool_input).join(',') : '(null)'}`);
            chat.addControlRequest(cr);
          }
          break;

        // message_delta 携带该轮 assistant 的最终 output_tokens
        case "token_usage":
          if (chat.currentAssistantMsg) {
            if (data.input_tokens != null) chat.currentAssistantMsg.inputTokens = data.input_tokens;
            if (data.output_tokens != null) chat.currentAssistantMsg.outputTokens = data.output_tokens;
          }
          break;

        case "result":
        case "done": {
          // 活跃会话完成 → 用户正在看，无需指示器
          if (data.session_id) session.setSessionActivity(data.session_id, null);
          const msg = chat.currentAssistantMsg;
          if (msg) {
            const targetSessionId = data.session_id || session.zenActiveId || session.activeSessionId;
            // Save full message as JSON blob: content + thinking + toolUses + stats
            const fullContent = JSON.stringify({
              text: msg.content,
              thinking: msg.thinking,
              toolUses: msg.toolUses,
              durationMs: data.duration_ms,
              // event 可能不含 token（如 DeepSeek result），fallback 到 message 对象上的值
              inputTokens: data.input_tokens ?? msg.inputTokens,
              outputTokens: data.output_tokens ?? msg.outputTokens,
              costUSD: data.cost_usd ?? msg.costUSD,
            });
            saveMessage(msg.id, targetSessionId, "assistant", fullContent, "{}").catch(() => {});
          }
          // 结算最后的思考和执行计时
          const finalThinking = popThinkingDuration(); // 最后一段思考（无后续 tool_use 触发 pop）
          if (toolExecStart && lastToolUse) {
            lastToolUse.executionDurationMs = Date.now() - toolExecStart;
            // 最终思考时间附加到最后一个工具的 thinkingDurationMs
            if (finalThinking > 0) lastToolUse.thinkingDurationMs = (lastToolUse.thinkingDurationMs || 0) + finalThinking;
            toolExecStart = 0;
            lastToolUse = null;
          } else if (finalThinking > 0 && msg?.toolUses.length) {
            // 有工具但 toolExecStart 已结算（思考在 result 前就开始了）
            const last = msg.toolUses[msg.toolUses.length - 1];
            last.thinkingDurationMs = (last.thinkingDurationMs || 0) + finalThinking;
          }
          chat.finishAssistantMessage(
            data.duration_ms,
            data.input_tokens ?? msg?.inputTokens,
            data.output_tokens ?? msg?.outputTokens,
            data.cost_usd ?? msg?.costUSD,
          );
          // 持久化 debug/stderr 日志 + 刷新侧栏统计
          const sid = data.session_id || session.zenActiveId || session.activeSessionId;
          if (sid) {
            saveSessionDebugLog(sid, JSON.stringify(debugLog.exportLines(sid))).catch(() => {});
            saveSessionStderrLog(sid, JSON.stringify(stderrLog.exportLines(sid))).catch(() => {});
            session.loadSessions().catch(() => {});  // 更新侧栏 token/cost/message_count
          }

          // Desktop notification
          notifyComplete(data.duration_ms, data.input_tokens, data.output_tokens);
          break;
        }

        case "error": {
          const { key, params } = translateError(data.error || "Unknown error");
          chat.appendText(`\n\n> ⚠️ ${t(key, params as any)}`);
          chat.finishAssistantMessage();
          break;
        }

        default:
          debugLog.add(`📨 unknown type: ${data.type}`);
          break;
      }
    });

    // Debug: raw stdout lines from claude
    unlistenDebug = await listen<string>("stream-debug", (event) => {
      const raw = event.payload;
      stderrLog.add(raw);  // 完整保留到 stderr 日志
      // 同时摘要记录到 debug 日志
      try {
        const parsed = JSON.parse(raw);
        debugLog.add(`📤 raw: type=${parsed.type || '?'} keys=${Object.keys(parsed).join(',')}`);
      } catch {
        debugLog.add(`📤 raw: ${raw.slice(0, 200)}`);
      }
    });

    // Stderr output from claude（--verbose 输出 LLM 请求详情）
    unlistenError = await listen<string>("stream-error", (event) => {
      const raw = event.payload;
      stderrLog.add(`[stderr] ${raw}`);  // 完整保留
      debugLog.add(`⚠️ stderr: ${raw.slice(0, 300)}`);  // debug 摘要
    });

    // Session created: store on both frontend (Pinia) and backend (Rust)
    unlistenSession = await listen<SessionCreatedPayload>("session-created", (event) => {
      const { ourId, claudeSessionId, mcpServers } = event.payload;
      session.setClaudeSessionId(ourId, claudeSessionId);
      storeClaudeSession(ourId, claudeSessionId); // → Rust SessionManager
      if (mcpServers) session.connectedMcpServers = [...mcpServers];
      debugLog.add(`🔗 session: ${ourId} → claude:${claudeSessionId}`);
    });

    // Process exited: 仅当是活跃会话时才更新 UI 状态
    unlistenProcessExit = await listen<ProcessExitedEvent>("process-exited", (event) => {
      const { session_id, exit_code, success } = event.payload;
      debugLog.add(`🏁 process exited: ${session_id} code=${exit_code} ok=${success}`);
      const isActiveExit = session_id === session.activeSessionId
        || session_id === session.zenActiveId;
      if (session_id && !isActiveExit) {
        // 后台会话退出：若未正常 result，强制结束缓存中的流式消息
        if (!success) {
          chat.handleBackgroundStreamEvent(session_id, { type: 'error', error: '进程异常退出' });
        }
        // 非 blocked 状态 → 标记为 unread
        if (session.sessionActivity[session_id] !== 'blocked') {
          session.setSessionActivity(session_id, 'unread');
        }
        return;
      }
      // 活跃会话退出 → 清除 activity（用户正在看）
      session.setSessionActivity(session_id, null);
      chat.isProcessing = false;
      if (!success) {
        chat.finishAssistantMessage();
      }
    });
  }

  function stopListening() {
    if (unlisten) { unlisten(); unlisten = null; }
    if (unlistenDebug) { unlistenDebug(); unlistenDebug = null; }
    if (unlistenError) { unlistenError(); unlistenError = null; }
    if (unlistenSession) { unlistenSession(); unlistenSession = null; }
    if (unlistenProcessExit) { unlistenProcessExit(); unlistenProcessExit = null; }
  }

  return { startListening, stopListening };
}
