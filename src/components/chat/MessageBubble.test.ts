import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import { createI18n } from "vue-i18n";
import MessageBubble from "./MessageBubble.vue";
import type { Message } from "@/stores/chat";

const i18n = createI18n({
  legacy: false,
  locale: "en",
  messages: {
    en: {
      chat: { thinking: "Thinking...", thinkingDone: "Thinking" },
      mode: { askBefore: "Ask", editAuto: "Edit", plan: "Plan", auto: "Auto" },
    },
    zh: {
      chat: { thinking: "思考中...", thinkingDone: "思考过程" },
      mode: { askBefore: "询问", editAuto: "编辑", plan: "计划", auto: "自动" },
    },
  },
});

function makeMsg(overrides: Partial<Message> = {}): Message {
  return {
    id: "test-1",
    role: "assistant",
    content: "",
    thinking: "",
    toolUses: [],
    timestamp: Date.now(),
    isStreaming: false,
    ...overrides,
  };
}

describe("MessageBubble", () => {
  it("renders user message with U avatar", () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: makeMsg({ role: "user", content: "Hello AI" }),
      },
      global: { plugins: [i18n] },
    });
    expect(wrapper.text()).toContain("Hello AI");
    expect(wrapper.text()).toContain("You");
  });

  it("renders assistant message with C avatar", () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: makeMsg({ role: "assistant", content: "Hello human" }),
      },
      global: { plugins: [i18n] },
    });
    expect(wrapper.text()).toContain("Hello human");
    expect(wrapper.text()).toContain("Claude");
  });

  it("shows thinking section when thinking content exists", () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: makeMsg({
          role: "assistant",
          content: "Answer",
          thinking: "Step 1: analyze...",
        }),
      },
      global: { plugins: [i18n] },
    });
    expect(wrapper.text()).toContain("Step 1: analyze...");
  });

  it("does not show thinking section when empty", () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: makeMsg({ role: "assistant", content: "Answer", thinking: "" }),
      },
      global: { plugins: [i18n] },
    });
    expect(wrapper.text()).not.toContain("Thinking...");
    expect(wrapper.text()).not.toContain("思考中...");
  });

  it("shows tool use details", () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: makeMsg({
          role: "assistant",
          toolUses: [
            {
              id: "tu_1",
              name: "Bash",
              input: { command: "ls" },
              result: "file1.txt\nfile2.txt",
            },
          ],
        }),
      },
      global: { plugins: [i18n] },
    });
    expect(wrapper.text()).toContain("Bash");
    expect(wrapper.text()).toContain("ls");
  });

  it("shows streaming cursor when isStreaming", () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: makeMsg({ role: "assistant", content: "partial", isStreaming: true }),
      },
      global: { plugins: [i18n] },
    });
    expect(wrapper.html()).toContain("stream-cursor");
  });

  it("does not show streaming cursor when finished", () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: makeMsg({ role: "assistant", content: "done", isStreaming: false }),
      },
      global: { plugins: [i18n] },
    });
    expect(wrapper.html()).not.toContain("stream-cursor");
  });
});
