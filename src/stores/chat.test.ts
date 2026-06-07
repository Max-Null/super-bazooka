import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { useChatStore } from "./chat";

describe("chat store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("starts with empty messages", () => {
    const chat = useChatStore();
    expect(chat.messages).toHaveLength(0);
    expect(chat.isProcessing).toBe(false);
  });

  it("adds user message", () => {
    const chat = useChatStore();
    const id = chat.addUserMessage("Hello");
    expect(chat.messages).toHaveLength(1);
    expect(chat.messages[0].role).toBe("user");
    expect(chat.messages[0].content).toBe("Hello");
    expect(chat.messages[0].isStreaming).toBe(false);
  });

  it("creates assistant message and streams text", () => {
    const chat = useChatStore();
    chat.addUserMessage("Hi");
    const id = chat.startAssistantMessage();

    expect(chat.messages).toHaveLength(2);
    expect(chat.currentAssistantMsg).not.toBeNull();
    expect(chat.currentAssistantMsg!.isStreaming).toBe(true);

    chat.appendText("Hello ");
    chat.appendText("world!");
    expect(chat.currentAssistantMsg!.content).toBe("Hello world!");
  });

  it("handles thinking content", () => {
    const chat = useChatStore();
    chat.addUserMessage("Think");
    chat.startAssistantMessage();

    chat.appendThinking("Let me think...");
    chat.appendThinking("Done thinking.");
    expect(chat.currentAssistantMsg!.thinking).toBe("Let me think...Done thinking.");
  });

  it("handles tool use", () => {
    const chat = useChatStore();
    chat.addUserMessage("Run command");
    chat.startAssistantMessage();

    chat.addToolUse({
      id: "tu_001",
      name: "Bash",
      input: { command: "ls" },
    });

    expect(chat.currentAssistantMsg!.toolUses).toHaveLength(1);
    expect(chat.currentAssistantMsg!.toolUses[0].name).toBe("Bash");
  });

  it("finishes assistant message", () => {
    const chat = useChatStore();
    chat.addUserMessage("Done");
    chat.startAssistantMessage();

    chat.appendText("All done.");
    chat.finishAssistantMessage();

    expect(chat.currentAssistantMsg).toBeNull();
    expect(chat.messages[1].isStreaming).toBe(false);
    expect(chat.isProcessing).toBe(false);
  });

  it("clears all messages", () => {
    const chat = useChatStore();
    chat.addUserMessage("msg1");
    chat.addUserMessage("msg2");
    expect(chat.messages).toHaveLength(2);

    chat.clearMessages();
    expect(chat.messages).toHaveLength(0);
    expect(chat.currentAssistantMsg).toBeNull();
  });

  // ── Control Request ──

  it("adds and resolves control request", () => {
    const chat = useChatStore();
    const cr = { subtype: "can_use_tool", tool_name: "Bash", tool_input: { command: "ls" } };
    chat.addControlRequest(cr);
    expect(chat.pendingControlRequest).not.toBeNull();
    expect(chat.pendingControlRequest!.tool_name).toBe("Bash");

    chat.resolveControlRequest("allow");
    expect(chat.pendingControlRequest).toBeNull();
  });

  it("control request is cleared with clearMessages", () => {
    const chat = useChatStore();
    chat.addControlRequest({ subtype: "can_use_tool", tool_name: "Read", tool_input: { file_path: "x" } });
    expect(chat.pendingControlRequest).not.toBeNull();

    chat.clearMessages();
    expect(chat.pendingControlRequest).toBeNull();
  });

  // ── Token & Duration Stats ──

  it("records token and duration stats on finish", () => {
    const chat = useChatStore();
    chat.addUserMessage("Hi");
    chat.startAssistantMessage();
    chat.appendText("Done.");

    chat.finishAssistantMessage(1234, 50, 30, 0.005);
    const msg = chat.messages[1];
    expect(msg.isStreaming).toBe(false);
    expect(msg.durationMs).toBe(1234);
    expect(msg.inputTokens).toBe(50);
    expect(msg.outputTokens).toBe(30);
    expect(msg.costUSD).toBe(0.005);
  });

  it("finish without stats works (backward compat)", () => {
    const chat = useChatStore();
    chat.addUserMessage("Hi");
    chat.startAssistantMessage();
    chat.finishAssistantMessage();
    expect(chat.messages[1].durationMs).toBeUndefined();
    expect(chat.messages[1].inputTokens).toBeUndefined();
  });

  // ── Load Messages from DB ──

  it("loadMessages restores user messages", () => {
    const chat = useChatStore();
    chat.loadMessages([
      { id: "u1", role: "user", content: "Hello", created_at: "2026-01-01T00:00:00" },
      { id: "u2", role: "user", content: "World", created_at: "2026-01-01T00:01:00" },
    ]);
    expect(chat.messages).toHaveLength(2);
    expect(chat.messages[0].role).toBe("user");
    expect(chat.messages[0].content).toBe("Hello");
  });

  it("loadMessages parses assistant JSON content", () => {
    const chat = useChatStore();
    chat.loadMessages([
      {
        id: "a1",
        role: "assistant",
        content: JSON.stringify({
          text: "Answer text",
          thinking: "Deep thought",
          toolUses: [{ id: "t1", name: "Bash", input: { command: "ls" } }],
          durationMs: 500,
          inputTokens: 10,
          outputTokens: 20,
          costUSD: 0.001,
        }),
        created_at: "2026-01-01T00:00:00",
      },
    ]);
    expect(chat.messages).toHaveLength(1);
    const msg = chat.messages[0];
    expect(msg.content).toBe("Answer text");
    expect(msg.thinking).toBe("Deep thought");
    expect(msg.toolUses).toHaveLength(1);
    expect(msg.toolUses[0].name).toBe("Bash");
    expect(msg.durationMs).toBe(500);
    expect(msg.costUSD).toBe(0.001);
  });

  it("loadMessages falls back to plain text for old format", () => {
    const chat = useChatStore();
    chat.loadMessages([
      { id: "a1", role: "assistant", content: "Plain old text", created_at: "2026-01-01T00:00:00" },
    ]);
    expect(chat.messages).toHaveLength(1);
    expect(chat.messages[0].content).toBe("Plain old text");
    expect(chat.messages[0].thinking).toBe("");
    expect(chat.messages[0].toolUses).toEqual([]);
  });

  it("loadMessages clears existing messages first", () => {
    const chat = useChatStore();
    chat.addUserMessage("existing");
    expect(chat.messages).toHaveLength(1);

    chat.loadMessages([{ id: "n1", role: "user", content: "new", created_at: "2026-01-01T00:00:00" }]);
    expect(chat.messages).toHaveLength(1);
    expect(chat.messages[0].content).toBe("new");
  });
});
