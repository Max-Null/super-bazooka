import { defineStore } from "pinia";
import { ref, watch } from "vue";
import { getClaudeSettings, setClaudeSettings } from "@/lib/tauri-bridge";

const STORAGE_KEY = "cc-gui-ui-settings";

/**
 * 权限模式映射（对应 CLI --permission-mode 标志）:
 *   default | acceptEdits | bypassPermissions | plan | dontAsk | auto
 */
export type PermissionMode = "default" | "acceptEdits" | "bypassPermissions" | "dontAsk" | "auto";

export type Effort = "low" | "medium" | "high" | "xhigh" | "max" | "ultracode";

/** 仅存 localStorage 的 UI 偏好 */
interface UiSettings {
  planMode: boolean;
  autoMode: boolean;
  permissionMode: PermissionMode;
  effort: Effort;
  theme: "dark" | "light" | "system";
  locale: "zh" | "en";
}

function getUiDefaults(): UiSettings {
  return {
    planMode: false,
    autoMode: true,
    permissionMode: "bypassPermissions",
    effort: "high",
    theme: "dark",
    locale: "zh",
  };
}

function loadUiSettings(): UiSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return { ...getUiDefaults(), ...JSON.parse(raw) };
  } catch { /* ignore */ }
  return getUiDefaults();
}

export const useSettingsStore = defineStore("settings", () => {
  // ── API 配置 — 读写 ~/.claude/settings.json ──
  const apiKey = ref("");
  const baseUrl = ref("https://api.deepseek.com");
  const model = ref("deepseek-v4-pro[1M]");

  // ── UI 偏好 — localStorage ──
  const ui = loadUiSettings();
  const planMode = ref(ui.planMode);
  const autoMode = ref(ui.autoMode);
  const permissionMode = ref<PermissionMode>(ui.permissionMode);
  const effort = ref<Effort>(ui.effort);
  const theme = ref<"dark" | "light" | "system">(ui.theme);
  const locale = ref<"zh" | "en">(ui.locale);

  // 启动时从 ~/.claude/settings.json 加载配置
  getClaudeSettings().then(s => {
    apiKey.value = s.api_key;
    baseUrl.value = s.base_url;
    model.value = s.model;
    // effort 只在 cc-gui 没设置过时从 settings.json 取
    if (!ui.effort || ui.effort === getUiDefaults().effort) {
      const eff = s.effort as Effort;
      if (["low","medium","high","xhigh","max","ultracode"].includes(eff)) {
        effort.value = eff;
      }
    }
    // 权限模式：从 permissions.defaultMode 还原
    switch (s.permission_mode) {
      case "auto": planMode.value = false; autoMode.value = true; break;
      case "plan": planMode.value = true; autoMode.value = false; break;
      default:
        planMode.value = false; autoMode.value = false;
        if (["default","acceptEdits","bypassPermissions","dontAsk"].includes(s.permission_mode)) {
          permissionMode.value = s.permission_mode as PermissionMode;
        }
    }
  }).catch(() => {});

  // 权限模式 → 解析为 settings.json 的 permissions.defaultMode
  function resolvePermissionMode(): string {
    if (autoMode.value) return "auto";
    if (planMode.value) return "plan";
    return permissionMode.value;
  }

  // 配置变更 → 写回 ~/.claude/settings.json
  watch(
    [apiKey, baseUrl, model, effort, planMode, autoMode, permissionMode],
    ([k, u, m, e]) => {
      setClaudeSettings(k, u, m, e, resolvePermissionMode()).catch(() => {});
    },
    { deep: true },
  );

  // UI 偏好变更 → 写 localStorage
  watch([planMode, autoMode, permissionMode, effort, theme, locale], () => {
    const s: UiSettings = {
      planMode: planMode.value,
      autoMode: autoMode.value,
      permissionMode: permissionMode.value,
      effort: effort.value,
      theme: theme.value,
      locale: locale.value,
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
  }, { deep: true });

  return { apiKey, baseUrl, model, planMode, autoMode, permissionMode, effort, theme, locale };
});
