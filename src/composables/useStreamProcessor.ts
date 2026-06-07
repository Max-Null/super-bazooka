import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useChatStore, type ToolUse } from "@/stores/chat";
import { useSessionStore } from "@/stores/session";
import { useDebugLog } from "@/composables/useDebugLog";
import { storeClaudeSession, saveMessage, type StreamEvent, type ProcessExitedEvent } from "@/lib/tauri-bridge";

let unlisten: UnlistenFn | null = null;
let unlistenDebug: UnlistenFn | null = null;
let unlistenError: UnlistenFn | null = null;
let unlistenSession: UnlistenFn | null = null;
let unlistenProcessExit: UnlistenFn | null = null;

interface SessionCreatedPayload {
  ourId: string;
  claudeSessionId: string;
}

export function useStreamProcessor() {
  const chat = useChatStore();
  const session = useSessionStore();
  const debugLog = useDebugLog();

  async function startListening() {
    if (unlisten) return;

    unlisten = await listen<StreamEvent>("stream-event", (event) => {
      const data = event.payload;
      debugLog.add(`📨 event: ${data.type} | text=${data.text.slice(0,50)} | thinking=${data.thinking.slice(0,50)} | final=${data.is_final}`);

      switch (data.type) {
        case "assistant":
          if (data.text) {
            chat.appendText(data.text);
          }
          if (data.thinking) {
            chat.appendThinking(data.thinking);
          }
          if (data.tool_use) {
            for (const tu of data.tool_use) {
              chat.addToolUse({
                id: tu.id,
                name: tu.name,
                input: tu.input,
              });
            }
          }
          break;

        case "control_request":
          if (data.control_request) {
            chat.addControlRequest(data.control_request);
          }
          break;

        case "result":
        case "done": {
          const msg = chat.currentAssistantMsg;
          if (msg) {
            // Save full message as JSON blob: content + thinking + toolUses + stats
            const fullContent = JSON.stringify({
              text: msg.content,
              thinking: msg.thinking,
              toolUses: msg.toolUses,
              durationMs: data.duration_ms,
              inputTokens: data.input_tokens,
              outputTokens: data.output_tokens,
              costUSD: data.cost_usd,
            });
            saveMessage(msg.id, session.activeSessionId, "assistant", fullContent, "{}").catch(() => {});
          }
          chat.finishAssistantMessage(
            data.duration_ms,
            data.input_tokens,
            data.output_tokens,
            data.cost_usd
          );
          break;
        }

        case "error":
          chat.appendText(`\n\n> ⚠️ ${data.error || "Unknown error"}`);
          chat.finishAssistantMessage();
          break;

        default:
          debugLog.add(`📨 unknown type: ${data.type}`);
          break;
      }
    });

    // Debug: raw stdout lines from claude
    unlistenDebug = await listen<string>("stream-debug", (event) => {
      const raw = event.payload;
      // Try to parse and pretty-print JSON
      try {
        const parsed = JSON.parse(raw);
        debugLog.add(`📤 raw: type=${parsed.type || '?'} keys=${Object.keys(parsed).join(',')}`);
      } catch {
        debugLog.add(`📤 raw: ${raw.slice(0, 200)}`);
      }
    });

    // Stderr output from claude
    unlistenError = await listen<string>("stream-error", (event) => {
      debugLog.add(`⚠️ stderr: ${event.payload.slice(0, 300)}`);
    });

    // Session created: store on both frontend (Pinia) and backend (Rust)
    unlistenSession = await listen<SessionCreatedPayload>("session-created", (event) => {
      const { ourId, claudeSessionId } = event.payload;
      session.setClaudeSessionId(ourId, claudeSessionId);
      storeClaudeSession(ourId, claudeSessionId); // → Rust SessionManager
      debugLog.add(`🔗 session: ${ourId} → claude:${claudeSessionId}`);
    });

    // Process exited: update processing state
    unlistenProcessExit = await listen<ProcessExitedEvent>("process-exited", (event) => {
      const { session_id, exit_code, success } = event.payload;
      debugLog.add(`🏁 process exited: ${session_id} code=${exit_code} ok=${success}`);
      if (!success && chat.isProcessing) {
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
