import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { useSettingsStore } from "./settings";

describe("settings store", () => {
  beforeEach(() => {
    localStorage.clear();
    setActivePinia(createPinia());
  });

  it("has default DeepSeek values", () => {
    const settings = useSettingsStore();
    expect(settings.baseUrl).toBe("https://api.deepseek.com");
    expect(settings.model).toBe("deepseek-v4-pro[1M]");
    expect(settings.planMode).toBe(false);
    expect(settings.autoMode).toBe(true);
    expect(settings.permissionMode).toBe("bypassPermissions");
    expect(settings.effort).toBe("high");
    expect(settings.theme).toBe("dark");
    expect(settings.locale).toBe("zh");
  });

  it("can change API settings", () => {
    const settings = useSettingsStore();
    settings.apiKey = "sk-test-key";
    settings.baseUrl = "https://custom.api.com";
    settings.model = "custom-model";

    expect(settings.apiKey).toBe("sk-test-key");
    expect(settings.baseUrl).toBe("https://custom.api.com");
    expect(settings.model).toBe("custom-model");
  });

  it("persists to localStorage", () => {
    const settings = useSettingsStore();
    settings.apiKey = "sk-persist-test";
    settings.save();

    const raw = localStorage.getItem("cc-gui-settings");
    expect(raw).toBeTruthy();
    const parsed = JSON.parse(raw!);
    expect(parsed.apiKey).toBe("sk-persist-test");
  });

  it("loads from localStorage on init", () => {
    localStorage.setItem(
      "cc-gui-settings",
      JSON.stringify({ apiKey: "sk-loaded", baseUrl: "https://x.com", model: "m1", planMode: true, autoMode: false, permissionMode: "default", effort: "xhigh", theme: "light", locale: "en" })
    );

    // Create a fresh Pinia + store to trigger loading
    setActivePinia(createPinia());
    const settings = useSettingsStore();
    expect(settings.apiKey).toBe("sk-loaded");
    expect(settings.baseUrl).toBe("https://x.com");
    expect(settings.locale).toBe("en");
    expect(settings.planMode).toBe(true);
    expect(settings.autoMode).toBe(false);
    expect(settings.permissionMode).toBe("default");
    expect(settings.effort).toBe("xhigh");
  });

  // ── Mode fields ──

  it("switches between plan and auto mode", () => {
    const settings = useSettingsStore();
    settings.planMode = true;
    expect(settings.planMode).toBe(true);
    settings.autoMode = true;
    expect(settings.autoMode).toBe(true);
  });

  it("switches permission modes", () => {
    const settings = useSettingsStore();
    settings.permissionMode = "acceptEdits";
    expect(settings.permissionMode).toBe("acceptEdits");
    settings.permissionMode = "bypassPermissions";
    expect(settings.permissionMode).toBe("bypassPermissions");
  });

  it("switches effort levels including ultracode", () => {
    const settings = useSettingsStore();
    settings.effort = "xhigh";
    expect(settings.effort).toBe("xhigh");
    settings.effort = "ultracode";
    expect(settings.effort).toBe("ultracode");
    settings.effort = "low";
    expect(settings.effort).toBe("low");
  });

  it("persists mode settings", () => {
    const settings = useSettingsStore();
    settings.planMode = true;
    settings.effort = "ultracode";
    settings.save();

    const raw = localStorage.getItem("cc-gui-settings");
    expect(raw).toBeTruthy();
    const parsed = JSON.parse(raw!);
    expect(parsed.planMode).toBe(true);
    expect(parsed.effort).toBe("ultracode");
  });
});
