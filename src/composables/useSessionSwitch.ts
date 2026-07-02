import { useRouter } from "vue-router";
import { useChatStore } from "@/stores/chat";
import { useSessionStore } from "@/stores/session";
import { listMessages } from "@/lib/tauri-bridge";

/**
 * 会话切换逻辑（AppShell rail + SessionSidebar 共用）。
 * 切换前缓存当前消息，切回时优先从缓存恢复（保留后台累积的流式消息），
 * 缓存未命中则从 DB 加载。
 */
export function useSessionSwitch() {
  const chat = useChatStore();
  const session = useSessionStore();
  const router = useRouter();

  async function switchTo(id: string) {
    // 保存当前会话消息到缓存
    if (session.activeSessionId) {
      chat.saveSessionCache(session.activeSessionId);
    }
    session.setActiveSession(id);
    // 只清除已完成的会话指示器，处理中的保留绿点
    if (session.sessionActivity[id] !== 'processing') {
      session.setSessionActivity(id, null);
    }

    // 优先从缓存恢复，缓存无数据则从 DB 加载
    const cached = chat.loadFromCache(id);
    if (cached) {
      chat.clearMessages();
      chat.messages.push(...cached);
      // 恢复流式状态：若最后一条消息在流式中，保持引用以继续接收事件
      const last = chat.messages[chat.messages.length - 1];
      if (last?.role === "assistant" && last.isStreaming) {
        chat.currentAssistantMsg = last;
        chat.isProcessing = true;
      }
    } else {
      try {
        const msgs = await listMessages(id);
        chat.loadMessages(
          msgs.map((m) => ({
            id: m.id,
            role: m.role,
            content: m.content,
            created_at: m.created_at,
          })),
        );
      } catch {
        chat.clearMessages();
      }
    }
    router.push("/chat");
  }

  /** 禅模式会话切换（优先缓存，缓存未命中则从 DB 加载） */
  async function zenSwitchTo(id: string) {
    if (session.zenActiveId) {
      chat.saveSessionCache(session.zenActiveId);
    }
    session.zenActiveId = id;
    if (session.sessionActivity[id] !== 'processing') {
      session.setSessionActivity(id, null);
    }

    const cached = chat.loadFromCache(id);
    chat.clearMessages();
    if (cached) {
      chat.messages.push(...cached);
      const last = chat.messages[chat.messages.length - 1];
      if (last?.role === "assistant" && last.isStreaming) {
        chat.currentAssistantMsg = last;
        chat.isProcessing = true;
      }
    } else {
      // 缓存未命中 → DB 兜底（重启后恢复已持久化的历史消息）
      try {
        const msgs = await listMessages(id);
        chat.loadMessages(msgs.map(m => ({
          id: m.id, role: m.role, content: m.content, created_at: m.created_at,
        })));
      } catch { /* 无消息——保持空白 */ }
    }
    router.push("/chat");
  }

  return { switchTo, zenSwitchTo };
}
