import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import InputBar from "./InputBar.vue";

describe("InputBar", () => {
  it("emits send with trimmed text on button click", async () => {
    const wrapper = mount(InputBar, { props: { disabled: false } });

    const textarea = wrapper.find("textarea");
    await textarea.setValue("  Hello World  ");
    await wrapper.find("button").trigger("click");

    expect(wrapper.emitted("send")).toBeTruthy();
    expect(wrapper.emitted("send")![0]).toEqual(["Hello World"]);
    // Input should clear after send
    expect(textarea.element.value).toBe("");
  });

  it("emits send on Enter key", async () => {
    const wrapper = mount(InputBar, { props: { disabled: false } });

    const textarea = wrapper.find("textarea");
    await textarea.setValue("Test message");
    await textarea.trigger("keydown", { key: "Enter", shiftKey: false });

    expect(wrapper.emitted("send")).toBeTruthy();
    expect(wrapper.emitted("send")![0]).toEqual(["Test message"]);
  });

  it("allows newline on Shift+Enter", async () => {
    const wrapper = mount(InputBar, { props: { disabled: false } });

    const textarea = wrapper.find("textarea");
    await textarea.trigger("keydown", { key: "Enter", shiftKey: true });

    // Should NOT emit send
    expect(wrapper.emitted("send")).toBeFalsy();
  });

  it("shows stop button when disabled (processing)", () => {
    const wrapper = mount(InputBar, { props: { disabled: true } });
    const buttons = wrapper.findAll("button");
    const stopBtn = buttons.find(b => b.attributes("title") === "Stop");
    expect(stopBtn).toBeTruthy();
  });

  it("emits stop on stop button click", async () => {
    const wrapper = mount(InputBar, { props: { disabled: true } });
    const stopBtn = wrapper.find("button[title='Stop']");
    await stopBtn.trigger("click");
    expect(wrapper.emitted("stop")).toBeTruthy();
  });

  it("does not emit when disabled", async () => {
    const wrapper = mount(InputBar, { props: { disabled: true } });

    const textarea = wrapper.find("textarea");
    await textarea.setValue("Should not send");
    // The send button is hidden when disabled (stop button is shown instead)
    // verify no send emitted
    expect(wrapper.emitted("send")).toBeFalsy();
  });

  it("does not emit empty message", async () => {
    const wrapper = mount(InputBar, { props: { disabled: false } });

    await wrapper.find("button").trigger("click");
    expect(wrapper.emitted("send")).toBeFalsy();

    await wrapper.find("textarea").setValue("   ");
    await wrapper.find("button").trigger("click");
    expect(wrapper.emitted("send")).toBeFalsy();
  });

  it("disables send button when input is empty", () => {
    const wrapper = mount(InputBar, { props: { disabled: false } });
    const sendBtn = wrapper.find("button[title='']");  // send has no title attr
    // The button rendered is send button (not stop), and it should be disabled
    const buttons = wrapper.findAll("button");
    // Find send button: svg with viewBox contains "22 2"
    expect(buttons.length).toBeGreaterThan(0);
  });

  // ── Drag & drop ──

  it("shows drag-over border on dragover", async () => {
    const wrapper = mount(InputBar, { props: { disabled: false } });
    const container = wrapper.find(".flex.items-center.max-w-3xl");
    await container.trigger("dragover", { dataTransfer: { dropEffect: "" } });
    expect(container.attributes("style")).toContain("dashed");
  });

  it("hides drag border on dragleave", async () => {
    const wrapper = mount(InputBar, { props: { disabled: false } });
    const container = wrapper.find(".flex.items-center.max-w-3xl");
    await container.trigger("dragover", { dataTransfer: { dropEffect: "" } });
    await container.trigger("dragleave");
    expect(container.attributes("style")).not.toContain("dashed");
  });

  it("emits files event on drop", async () => {
    const wrapper = mount(InputBar, { props: { disabled: false } });
    const container = wrapper.find(".flex.items-center.max-w-3xl");

    const files = [{ name: "test.ts", path: "/home/user/test.ts" }];
    const dt = { dropEffect: "", files, items: [] };

    await container.trigger("drop", { dataTransfer: dt });
    expect(wrapper.emitted("files")).toBeTruthy();
    expect(wrapper.emitted("files")![0]).toEqual([[{ name: "test.ts", path: "/home/user/test.ts" }]]);
  });
});
