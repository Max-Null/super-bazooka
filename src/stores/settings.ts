import { defineStore } from "pinia";
import { ref, watch } from "vue";

const STORAGE_KEY = "cc-gui-settings";

/**
 * Permission mode mapping (CLI --permission-mode flag):
 *   default | acceptEdits | bypassPermissions | plan
 *
 * "auto" is NOT a CLI flag — it writes `permissions.defaultMode: "auto"`
 * to settings.json and runs CLI with --permission-mode default.
 * The harness then decides auto-approval based on context.
 */
export type PermissionMode = "default" | "acceptEdits" | "bypassPermissions" | "auto";

/**
 * Effort level mapping:
 *   low | medium | high | xhigh | max  → API-native effort levels
 *   ultracode → NOT an API effort level!
 *     = xhigh (model reasoning) + auto Workflow orchestration (harness)
 *     Backend translates: --effort xhigh + --settings '{"ultracode":true}'
 */
export type Effort = "low" | "medium" | "high" | "xhigh" | "max" | "ultracode";

export interface Settings {
  apiKey: string;
  baseUrl: string;
  model: string;
  planMode: boolean;
  autoMode: boolean;
  permissionMode: PermissionMode;
  effort: Effort;
  theme: "dark" | "light";
  locale: "zh" | "en";
}

function loadSettings(): Settings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      return { ...getDefaults(), ...JSON.parse(raw) };
    }
  } catch {
    // ignore
  }
  return getDefaults();
}

function getDefaults(): Settings {
  return {
    apiKey: "",
    baseUrl: "https://api.deepseek.com",
    model: "deepseek-v4-pro[1M]",
    planMode: false,
    autoMode: true,
    permissionMode: "bypassPermissions",
    effort: "high",
    theme: "dark",
    locale: "zh",
  };
}

export const useSettingsStore = defineStore("settings", () => {
  const apiKey = ref("");
  const baseUrl = ref("https://api.deepseek.com");
  const model = ref("deepseek-v4-pro[1M]");
  const planMode = ref(false);
  const autoMode = ref(true);
  const permissionMode = ref<PermissionMode>("bypassPermissions");
  const effort = ref<Effort>("high");
  const theme = ref<"dark" | "light">("dark");
  const locale = ref<"zh" | "en">("zh");

  const loaded = loadSettings();
  apiKey.value = loaded.apiKey;
  baseUrl.value = loaded.baseUrl;
  model.value = loaded.model;
  planMode.value = loaded.planMode;
  autoMode.value = loaded.autoMode;
  permissionMode.value = loaded.permissionMode;
  effort.value = loaded.effort;
  theme.value = loaded.theme;
  locale.value = loaded.locale;

  function save() {
    const s: Settings = {
      apiKey: apiKey.value,
      baseUrl: baseUrl.value,
      model: model.value,
      planMode: planMode.value,
      autoMode: autoMode.value,
      permissionMode: permissionMode.value,
      effort: effort.value,
      theme: theme.value,
      locale: locale.value,
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
  }

  // Auto-save on changes
  watch([apiKey, baseUrl, model, planMode, autoMode, permissionMode, effort, theme, locale], save, { deep: true });

  return { apiKey, baseUrl, model, planMode, autoMode, permissionMode, effort, theme, locale, save };
});
