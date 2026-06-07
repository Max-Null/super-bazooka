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

  it("does not emit when disabled", async () => {
    const wrapper = mount(InputBar, { props: { disabled: true } });

    const textarea = wrapper.find("textarea");
    await textarea.setValue("Should not send");
    await wrapper.find("button").trigger("click");

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

  it("disables button when input is empty", () => {
    const wrapper = mount(InputBar, { props: { disabled: false } });
    const button = wrapper.find("button");
    expect(button.attributes("disabled")).toBeDefined();
  });
});
