import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { setActivePinia, createPinia } from "pinia";
import { createI18n } from "vue-i18n";
import type { Pinia } from "pinia";
import { useSettingsStore } from "@/stores/settings";
import SettingsPanel from "./SettingsPanel.vue";

const i18n = createI18n({
  legacy: false,
  locale: "en",
  messages: {
    en: {
      settings: {
        title: "Settings", apiConfig: "API Config", ccGuiSettings: "cc-gui Settings",
        baseUrl: "Base URL", apiKey: "API Key", model: "Model",
        test: "Test Connection", testing: "Testing…",
        language: "Language", theme: "Theme", defaultMode: "Permission Mode", defaultEffort: "Effort Level",
        themeDark: "Dark", themeLight: "Light", themeSystem: "System",
        aboutDesc: "Tauri 2 + Vue 3 + TypeScript",
      },
      mode: {
        askBefore: "Ask before edits", editAuto: "Edit auto",
        plan: "Plan mode", auto: "Auto mode",
        bypass: "Bypass", dontAsk: "Don't Ask",
        effort: { low: "Low", medium: "Med", high: "High", xhigh: "XHigh", max: "Max", ultracode: "Ultra" },
      },
      chat: { aboutVersion: "v0.1.0" },
    },
  },
});

const mockRouter = { push: () => {}, currentRoute: { value: { path: "/settings" } } };

let pinia: Pinia;

function mountPanel() {
  return mount(SettingsPanel, {
    global: {
      plugins: [pinia, i18n],
      provide: { router: mockRouter },
    },
  });
}

describe("SettingsPanel", () => {
  beforeEach(() => {
    localStorage.clear();
    pinia = createPinia();
    setActivePinia(pinia);
  });

  // ── activeMode computed (bridges planMode / autoMode / permissionMode) ──

  it("shows plan label when planMode=true", () => {
    const settings = useSettingsStore();
    settings.planMode = true;
    settings.autoMode = false;
    const wrapper = mountPanel();
    expect(wrapper.text()).toContain("Plan mode");
  });

  it("shows auto label when autoMode=true", () => {
    const settings = useSettingsStore();
    settings.autoMode = true;
    settings.planMode = false;
    const wrapper = mountPanel();
    expect(wrapper.text()).toContain("Auto mode");
  });

  it("shows bypass when permissionMode=bypassPermissions", () => {
    const settings = useSettingsStore();
    settings.permissionMode = "bypassPermissions";
    settings.autoMode = false;
    settings.planMode = false;
    const wrapper = mountPanel();
    expect(wrapper.text()).toContain("Bypass");
  });

  it("shows dontAsk when permissionMode=dontAsk", () => {
    const settings = useSettingsStore();
    settings.permissionMode = "dontAsk";
    settings.autoMode = false;
    settings.planMode = false;
    const wrapper = mountPanel();
    expect(wrapper.text()).toContain("Don't Ask");
  });

  it("shows editAuto when permissionMode=acceptEdits", () => {
    const settings = useSettingsStore();
    settings.permissionMode = "acceptEdits";
    settings.autoMode = false;
    settings.planMode = false;
    const wrapper = mountPanel();
    expect(wrapper.text()).toContain("Edit auto");
  });

  it("shows askBefore when permissionMode=default", () => {
    const settings = useSettingsStore();
    settings.permissionMode = "default";
    settings.autoMode = false;
    settings.planMode = false;
    const wrapper = mountPanel();
    expect(wrapper.text()).toContain("Ask before edits");
  });

  // ── Effort ──

  it("shows current effort level", () => {
    const settings = useSettingsStore();
    settings.effort = "max";
    const wrapper = mountPanel();
    expect(wrapper.text()).toContain("Max");
  });

  // ── Dropdown triggers ──

  it("has two settings dropdown triggers (perm + effort)", () => {
    const wrapper = mountPanel();
    const triggers = wrapper.findAll(".settings-dropdown");
    expect(triggers.length).toBe(2);
  });

  // ── Layout ──

  it("renders both sections", () => {
    const wrapper = mountPanel();
    expect(wrapper.text()).toContain("API Config");
    expect(wrapper.text()).toContain("cc-gui Settings");
  });

  it("renders about footer", () => {
    const wrapper = mountPanel();
    expect(wrapper.text()).toContain("Tauri 2 + Vue 3 + TypeScript");
    expect(wrapper.text()).toContain("v0.1.0");
  });

  // ── Connection test ──

  it("shows test connection button", () => {
    const wrapper = mountPanel();
    expect(wrapper.text()).toContain("Test Connection");
  });
});
