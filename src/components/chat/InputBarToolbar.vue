<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { useSettingsStore } from "@/stores/settings";
import { getClaudeDir, readFileContent } from "@/lib/tauri-bridge";
import ContextIndicator from "./ContextIndicator.vue";
import type { Effort } from "@/stores/settings";

const { t } = useI18n();

// ── Ponytail 插件检测 ──
const hasPonytail = ref(false);
// ponytail: 启动时读 enabledPlugins 检测，未安装则显示安装按钮。Tauri 不可用时静默跳过
onMounted(async () => {
  try {
    const dir = await getClaudeDir();
    const raw = await readFileContent(`${dir}/settings.json`);
    const plugins: Record<string, boolean> = JSON.parse(raw).enabledPlugins || {};
    hasPonytail.value = Object.keys(plugins).some(k => k.startsWith("ponytail@"));
  } catch { /* 未安装或 Tauri 不可用，保持 false */ }
});

function installPonytail() {
  emit("sendSlash", "请帮我安装 Ponytail 插件（ponytail@claude-plugins-official）");
}

const emit = defineEmits<{
  attachFile: [];
  openCommandMenu: [];
  sendSlash: [text: string];
}>();

const settings = useSettingsStore();

const modeOptions = [
  { value: "askBefore", label: () => t("mode.askBefore") },
  { value: "editAuto", label: () => t("mode.editAuto") },
  { value: "plan", label: () => t("mode.plan") },
  { value: "auto", label: () => t("mode.auto") },
	  { value: "bypass", label: () => t("mode.bypass") },
	  { value: "dontAsk", label: () => t("mode.dontAsk") },
];

// Effort levels with progressive colors (low → calm, ultracode → intense)
const effortOptions: Array<{ value: Effort; label: () => string; color: string }> = [
  { value: "low", label: () => t("mode.effort.low"), color: "#22c55e" },
  { value: "medium", label: () => t("mode.effort.medium"), color: "#14b8a6" },
  { value: "high", label: () => t("mode.effort.high"), color: "#f59e0b" },
  { value: "xhigh", label: () => t("mode.effort.xhigh"), color: "#f97316" },
  { value: "max", label: () => t("mode.effort.max"), color: "#ef4444" },
  { value: "ultracode", label: () => t("mode.effort.ultracode"), color: "#8b5cf6" },
];

const currentEffortColor = computed(() => {
  return effortOptions.find(o => o.value === settings.effort)?.color || "var(--amber)";
});

// ── Ponytail mode ──
const ponytailOptions: Array<{ value: "off" | "lite" | "full" | "ultra"; icon: string; label: string; color: string }> = [
  { value: "off",   icon: "⬜", label: "关闭", color: "#6b7280" },
  { value: "lite",  icon: "🌱", label: "轻量", color: "#22c55e" },
  { value: "full",  icon: "🎯", label: "标准", color: "#f59e0b" },
  { value: "ultra", icon: "🔥", label: "极致", color: "#ef4444" },
];
const currentPonytail = computed(() => ponytailOptions.find(o => o.value === settings.ponytailMode)!);

function selectPonytail(value: "off" | "lite" | "full" | "ultra") {
  settings.ponytailMode = value;
  openMenu.value = null;
  emit("sendSlash", `/ponytail ${value}`);
}

// ── Custom dropdowns ──
const openMenu = ref<"mode" | "effort" | "ponytail" | null>(null);

function toggleMenu(menu: "mode" | "effort" | "ponytail") {
  openMenu.value = openMenu.value === menu ? null : menu;
}

function selectMode(value: string) {
  activeMode.value = value;
  openMenu.value = null;
}

const effortWarning = ref("");
let effortWarnTimer: ReturnType<typeof setTimeout> | null = null;

function selectEffort(value: Effort) {
  settings.effort = value;
  openMenu.value = null;

  if (value === "xhigh" && settings.model.toLowerCase().includes("deepseek")) {
    effortWarning.value = t("effortWarning");
    if (effortWarnTimer) clearTimeout(effortWarnTimer);
    effortWarnTimer = setTimeout(() => effortWarning.value = "", 5000);
  } else {
    effortWarning.value = "";
  }
}

function closeMenus() {
  openMenu.value = null;
}

// Click outside listener
function onBodyClick(e: MouseEvent) {
  if (!(e.target as HTMLElement).closest(".toolbar-dropdown")) {
    closeMenus();
  }
}
onMounted(() => document.addEventListener("click", onBodyClick));
onUnmounted(() => document.removeEventListener("click", onBodyClick));

const activeMode = computed({
  get: () => {
    if (settings.planMode) return "plan";
    if (settings.autoMode) return "auto";
    if (settings.permissionMode === "bypassPermissions") return "bypass";
	    if (settings.permissionMode === "dontAsk") return "dontAsk";
	    if (settings.permissionMode === "acceptEdits") return "editAuto";
    return "askBefore";
  },
  set: (v: string) => {
    settings.planMode = v === "plan";
    settings.autoMode = v === "auto";
    settings.permissionMode = v === "editAuto" ? "acceptEdits" : v === "bypass" ? "bypassPermissions" : v === "dontAsk" ? "dontAsk" : "default";
  },
});

const currentModeLabel = computed(() => {
  const m = modeOptions.find(o => o.value === activeMode.value);
  return m ? m.label() : activeMode.value;
});

const currentEffortLabel = computed(() => {
  const e = effortOptions.find(o => o.value === settings.effort);
  return e ? e.label() : settings.effort;
});
</script>

<template>
  <div class="toolbar flex items-center gap-1.5 max-w-3xl mx-auto pb-1 select-none">
    <!-- 📎 Attach File -->
    <button
      @click="emit('attachFile')"
      class="toolbar-btn flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] shrink-0"
      :title="$t('toolbar.attachTitle')"
    >
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"/></svg>
      <span class="hidden sm:inline font-medium">{{ $t('toolbar.attach') }}</span>
    </button>

    <!-- ☰ Command Menu -->
    <button
      @click="emit('openCommandMenu')"
      class="toolbar-btn flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] shrink-0"
      :title="$t('toolbar.commandsTitle')"
    >
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M3 12h18M3 6h18M3 18h18"/></svg>
      <span class="hidden sm:inline font-medium">{{ $t('toolbar.commands') }}</span>
    </button>

    <!-- Spacer -->
    <div class="flex-1"></div>

    <!-- Permission Mode dropdown -->
    <div
      class="toolbar-dropdown toolbar-pill flex items-center gap-1.5 text-[11px] shrink-0 rounded-lg relative cursor-pointer"
      @click.stop="toggleMenu('mode')"
    >
      <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" style="color: var(--accent); opacity: 0.7">
        <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
      </svg>
      <span class="font-medium" style="color: var(--accent)">{{ currentModeLabel }}</span>
      <svg width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" :style="{ color: 'var(--accent)', opacity: 0.5, transition: 'transform 150ms', transform: openMenu === 'mode' ? 'rotate(180deg)' : '' }"><polyline points="6 9 12 15 18 9"/></svg>

      <!-- Dropdown menu -->
      <Transition name="drop">
        <div
          v-if="openMenu === 'mode'"
          class="dropdown-menu absolute bottom-full right-0 mb-1 py-1 rounded-lg z-30 min-w-[140px]"
          style="background: var(--bg-elevated); border: 1px solid var(--border-default); box-shadow: 0 8px 24px rgba(0,0,0,0.4)"
        >
          <button
            v-for="m in modeOptions"
            :key="m.value"
            @click="selectMode(m.value)"
            class="w-full text-left px-3 py-1.5 text-[11px] transition-colors hover:bg-[var(--bg-hover)]"
            :style="{
              color: activeMode === m.value ? 'var(--accent)' : 'var(--text-secondary)',
              background: activeMode === m.value ? 'var(--accent-glow)' : 'transparent'
            }"
          >{{ m.label() }}</button>
        </div>
      </Transition>
    </div>

    <!-- Effort dropdown -->
    <div
      class="toolbar-dropdown toolbar-pill flex items-center gap-1.5 text-[11px] shrink-0 rounded-lg relative cursor-pointer"
      @click.stop="toggleMenu('effort')"
    >
      <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" :style="{ color: currentEffortColor, opacity: 0.7 }">
        <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>
      </svg>
      <span class="font-medium" :style="{ color: currentEffortColor }">{{ currentEffortLabel }}</span>
      <svg width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" :style="{ color: currentEffortColor, opacity: 0.5, transition: 'transform 150ms', transform: openMenu === 'effort' ? 'rotate(180deg)' : '' }"><polyline points="6 9 12 15 18 9"/></svg>

      <!-- Dropdown menu -->
      <Transition name="drop">
        <div
          v-if="openMenu === 'effort'"
          class="dropdown-menu absolute bottom-full right-0 mb-1 py-1 rounded-lg z-30 min-w-[130px]"
          style="background: var(--bg-elevated); border: 1px solid var(--border-default); box-shadow: 0 8px 24px rgba(0,0,0,0.4)"
        >
          <button
            v-for="e in effortOptions"
            :key="e.value"
            @click="selectEffort(e.value)"
            class="w-full text-left px-3 py-1.5 text-[11px] transition-colors hover:bg-[var(--bg-hover)]"
            :style="{
              color: settings.effort === e.value ? e.color : 'var(--text-secondary)',
              background: settings.effort === e.value ? e.color + '18' : 'transparent'
            }"
          >{{ e.label() }}</button>
        </div>
      </Transition>
    </div>

    <!-- Ponytail: 已安装 → 下拉；未安装 → 安装按钮 -->
    <div
      v-if="hasPonytail"
      class="toolbar-dropdown toolbar-pill flex items-center gap-1.5 text-[0.65rem] shrink-0 rounded-lg relative cursor-pointer"
      @click.stop="toggleMenu('ponytail')"
      :title="$t('toolbar.ponytailTitle')"
    >
      <span class="font-medium" :style="{ color: currentPonytail.color }">{{ currentPonytail.label }}</span>
      <svg width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" :style="{ color: currentPonytail.color, opacity: 0.5, transition: 'transform 150ms', transform: openMenu === 'ponytail' ? 'rotate(180deg)' : '' }"><polyline points="6 9 12 15 18 9"/></svg>

      <Transition name="drop">
        <div
          v-if="openMenu === 'ponytail'"
          class="dropdown-menu absolute bottom-full right-0 mb-1 py-1 rounded-lg z-30 min-w-[110px]"
          style="background: var(--bg-elevated); border: 1px solid var(--border-default); box-shadow: 0 8px 24px rgba(0,0,0,0.4)"
        >
          <button
            v-for="p in ponytailOptions"
            :key="p.value"
            @click="selectPonytail(p.value)"
            class="w-full text-left px-3 py-1.5 text-[0.65rem] transition-colors hover:bg-[var(--bg-hover)] flex items-center gap-2"
            :style="{
              color: settings.ponytailMode === p.value ? p.color : 'var(--text-secondary)',
              background: settings.ponytailMode === p.value ? p.color + '18' : 'transparent'
            }"
          >{{ p.label }}</button>
        </div>
      </Transition>
    </div>
    <button
      v-else
      @click="installPonytail"
      class="toolbar-pill flex items-center gap-1 text-[0.6rem] shrink-0 rounded-lg"
      :title="$t('toolbar.ponytailInstall')"
    >
      <span class="opacity-60">⬇</span>
      <span class="font-medium">{{ $t('toolbar.ponytailInstall') }}</span>
    </button>

    <!-- Mini divider -->
    <div class="w-px h-4 shrink-0" style="background: var(--border-dim)"></div>

    <!-- Context indicator -->
    <ContextIndicator />
  </div>

  <!-- Effort warning -->
  <div
    v-if="effortWarning"
    class="max-w-3xl mx-auto pb-1"
  >
    <div class="text-[10px] px-2 py-1 rounded-md flex items-center gap-1.5"
      style="background: var(--amber-glow); color: var(--amber); border: 1px solid rgba(245,166,35,0.2)"
    >
      <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/></svg>
      {{ effortWarning }}
    </div>
  </div>
</template>

<style scoped>
.toolbar {
  position: relative;
}

.toolbar-btn {
  color: var(--text-muted);
  background: transparent;
  border: 1px solid transparent;
  transition: all 150ms ease;
}

.toolbar-btn:hover {
  color: var(--text-secondary);
  background: var(--bg-elevated);
  border-color: var(--border-dim);
}

.toolbar-pill {
  background: var(--bg-elevated);
  border: 1px solid var(--border-dim);
  padding: 3px 8px 3px 8px;
  transition: border-color 150ms;
  outline: none;
  user-select: none;
}

.toolbar-pill:hover {
  border-color: var(--border-bright);
}

.dropdown-menu {
  overflow: hidden;
}

/* Dropdown animation */
.drop-enter-active { transition: all 120ms ease-out; }
.drop-leave-active { transition: all 100ms ease-in; }
.drop-enter-from { opacity: 0; transform: translateY(4px) scale(0.96); }
.drop-leave-to { opacity: 0; transform: translateY(2px) scale(0.98); }
</style>
