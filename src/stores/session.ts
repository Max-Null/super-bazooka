import { defineStore } from "pinia";
import { ref } from "vue";
import {
  createSession as createSessionBackend,
  listSessions,
  deleteSession as deleteSessionBackend,
  renameSession as renameSessionBackend,
  type SessionData,
} from "@/lib/tauri-bridge";

export interface Session {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
  messageCount: number;
  totalTokens?: number | null;
  totalCost?: number | null;
  /** The real claude session UUID (for --resume) */
  claudeSessionId?: string;
}

export const useSessionStore = defineStore("session", () => {
  const sessions = ref<Session[]>([]);
  const activeSessionId = ref<string>("");

  /** Load sessions from Rust SQLite backend */
  async function loadSessions() {
    try {
      const list = await listSessions();
      sessions.value = list.map(toLocalSession);
      // Don't auto-select: user should start fresh or pick one explicitly
    } catch (err) {
      console.error("Failed to load sessions:", err);
    }
  }

  /** Create a new session via backend，可指定 CWD */
  async function createSession(model?: string, cwd?: string): Promise<string> {
    try {
      const s = await createSessionBackend(model, cwd);
      sessions.value.unshift(toLocalSession(s));
      activeSessionId.value = s.id;
      return s.id;
    } catch {
      // Fallback: local ID if backend unreachable
      const id = Date.now().toString(36);
      const session: Session = {
        id,
        title: "New Chat",
        createdAt: Date.now(),
        updatedAt: Date.now(),
        messageCount: 0,
      };
      sessions.value.unshift(session);
      activeSessionId.value = id;
      return id;
    }
  }

  function setActiveSession(id: string) {
    activeSessionId.value = id;
  }

  /** Rename session via backend */
  async function renameSession(id: string, title: string) {
    try {
      await renameSessionBackend(id, title);
    } catch (err) {
      console.error("Failed to rename session:", err);
    }
    const s = sessions.value.find((s) => s.id === id);
    if (s) {
      s.title = title;
      s.updatedAt = Date.now();
    }
  }

  /** Delete session via backend */
  async function deleteSession(id: string) {
    try {
      await deleteSessionBackend(id);
    } catch (err) {
      console.error("Failed to delete session:", err);
    }
    sessions.value = sessions.value.filter((s) => s.id !== id);
    if (activeSessionId.value === id) {
      activeSessionId.value = sessions.value[0]?.id || "";
    }
  }

  /** Store the real claude session UUID for a session */
  function setClaudeSessionId(ourId: string, claudeId: string) {
    const s = sessions.value.find((s) => s.id === ourId);
    if (s) {
      s.claudeSessionId = claudeId;
    }
  }

  /** Get the claude session UUID for the active session, if any */
  function getActiveClaudeSessionId(): string | undefined {
    const s = sessions.value.find((s) => s.id === activeSessionId.value);
    return s?.claudeSessionId;
  }

  return {
    sessions,
    activeSessionId,
    loadSessions,
    createSession,
    setActiveSession,
    renameSession,
    deleteSession,
    setClaudeSessionId,
    getActiveClaudeSessionId,
  };
});

/** Map backend SessionData to frontend Session */
function toLocalSession(s: SessionData): Session {
  return {
    id: s.id,
    title: s.title,
    createdAt: new Date(s.created_at + "Z").getTime(),
    updatedAt: new Date(s.updated_at + "Z").getTime(),
    messageCount: s.message_count,
    totalTokens: s.total_tokens,
    totalCost: s.total_cost,
    claudeSessionId: s.cli_session_id ?? undefined,
  };
}
