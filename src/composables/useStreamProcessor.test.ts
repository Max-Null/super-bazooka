import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useChatStore } from "@/stores/chat";
import { useSessionStore } from "@/stores/session";
import { useStreamProcessor } from "./useStreamProcessor";

const { listeners, saveMessageMock, storeClaudeSessionMock } = vi.hoisted(() => ({
  listeners: new Map<string, (event: { payload: any }) => void>(),
  saveMessageMock: vi.fn(),
  storeClaudeSessionMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (eventName: string, callback: (event: { payload: any }) => void) => {
    listeners.set(eventName, callback);
    return () => listeners.delete(eventName);
  }),
}));

vi.mock("@/composables/useDebugLog", () => ({
  useDebugLog: () => ({
    add: vi.fn(),
    clear: vi.fn(),
    visible: { value: false },
    setSession: vi.fn(),
    exportLines: vi.fn().mockReturnValue([]),
  }),
}));

vi.mock("@/composables/useStderrLog", () => ({
  useStderrLog: () => ({
    add: vi.fn(),
    clear: vi.fn(),
    lines: { value: [] },
    visible: { value: false },
    setSession: vi.fn(),
    exportLines: vi.fn().mockReturnValue([]),
  }),
}));

vi.mock("vue-i18n", () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@/lib/tauri-bridge", async () => {
  const actual = await vi.importActual<typeof import("@/lib/tauri-bridge")>("@/lib/tauri-bridge");
  return {
    ...actual,
    saveMessage: saveMessageMock,
    storeClaudeSession: storeClaudeSessionMock,
    saveSessionDebugLog: vi.fn().mockResolvedValue(undefined),
    saveSessionStderrLog: vi.fn().mockResolvedValue(undefined),
    listSessions: vi.fn().mockResolvedValue([]),
  };
});

describe("useStreamProcessor", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    listeners.clear();
    saveMessageMock.mockReset();
    saveMessageMock.mockResolvedValue(undefined);
    storeClaudeSessionMock.mockReset();
    storeClaudeSessionMock.mockResolvedValue(undefined);
  });

  afterEach(() => {
    useStreamProcessor().stopListening();
    listeners.clear();
  });

  it("saves result to session_id from event, not activeSessionId", async () => {
    const chat = useChatStore();
    const session = useSessionStore();
    session.setActiveSession("active-session");

    chat.addUserMessage("hello");
    chat.startAssistantMessage();
    chat.appendText("assistant reply");

    const { startListening, stopListening } = useStreamProcessor();
    await startListening();

    listeners.get("stream-event")?.({
      payload: {
        type: "result",
        session_id: "event-session",
        text: "",
        thinking: "",
        is_final: true,
        duration_ms: 1200,
        input_tokens: 10,
        output_tokens: 20,
        cost_usd: 0.001,
      },
    });

    expect(saveMessageMock).toHaveBeenCalledOnce();
    expect(saveMessageMock).toHaveBeenCalledWith(
      expect.any(String),
      "event-session",
      "assistant",
      expect.any(String),
      "{}",
    );

    stopListening();
  });

  it("falls back to activeSessionId when event has no session_id", async () => {
    const chat = useChatStore();
    const session = useSessionStore();
    session.setActiveSession("active-session");

    chat.addUserMessage("hello");
    chat.startAssistantMessage();
    chat.appendText("assistant reply");

    const { startListening, stopListening } = useStreamProcessor();
    await startListening();

    listeners.get("stream-event")?.({
      payload: {
        type: "result",
        text: "",
        thinking: "",
        is_final: true,
        duration_ms: 1200,
        input_tokens: 10,
        output_tokens: 20,
        cost_usd: 0.001,
      },
    });

    expect(saveMessageMock).toHaveBeenCalledOnce();
    expect(saveMessageMock).toHaveBeenCalledWith(
      expect.any(String),
      "active-session",
      "assistant",
      expect.any(String),
      "{}",
    );

    stopListening();
  });
});
