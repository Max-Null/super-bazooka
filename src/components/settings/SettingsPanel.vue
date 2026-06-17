<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { useSettingsStore } from "@/stores/settings";
import { connectLLM } from "@/lib/tauri-bridge";
import ErrorBoundary from "@/components/shared/ErrorBoundary.vue";

const router = useRouter();
const settings = useSettingsStore();

// ── 权限模式 computed（与工具栏 activeMode 逻辑一致）──
const activeMode = computed({
  get: () => {
    if (settings.planMode) return "plan";
    if (settings.autoMode) return "auto";
    if (settings.permissionMode === "bypassPermissions") return "bypassPermissions";
    if (settings.permissionMode === "dontAsk") return "dontAsk";
    if (settings.permissionMode === "acceptEdits") return "acceptEdits";
    return "default";
  },
  set: (v: string) => {
    settings.planMode = v === "plan";
    settings.autoMode = v === "auto";
    settings.permissionMode =
      v === "bypassPermissions" ? "bypassPermissions"
      : v === "dontAsk" ? "dontAsk"
      : v === "acceptEdits" ? "acceptEdits"
      : "default";
  },
});

// ── 自定义下拉 ──
const openDropdown = ref<"perm" | "effort" | null>(null);
function toggleDropdown(k: "perm" | "effort") {
  openDropdown.value = openDropdown.value === k ? null : k;
}
function closeDropdowns() { openDropdown.value = null; }
function onBodyClick(e: MouseEvent) {
  if (!(e.target as HTMLElement).closest(".settings-dropdown")) closeDropdowns();
}
onMounted(() => document.addEventListener("click", onBodyClick));
onUnmounted(() => document.removeEventListener("click", onBodyClick));

// ── 权限模式选项（图标 + 中文 + 英文 CLI key + 描述）──
interface PermOption { value: "auto" | "plan" | "default" | "acceptEdits" | "bypassPermissions" | "dontAsk"; icon: string; cliKey: string; labelKey: string; descKey: string }
const permOptions: PermOption[] = [
  { value: "auto",       icon: "🤖", cliKey: "auto",             labelKey: "mode.auto",       descKey: "mode.autoDesc" },
  { value: "plan",       icon: "📋", cliKey: "plan",             labelKey: "mode.plan",       descKey: "mode.planDesc" },
  { value: "default",    icon: "🔒", cliKey: "default",          labelKey: "mode.askBefore",  descKey: "mode.askBeforeDesc" },
  { value: "acceptEdits",icon: "✏️", cliKey: "acceptEdits",      labelKey: "mode.editAuto",   descKey: "mode.editAutoDesc" },
  { value: "bypassPermissions", icon: "⚡", cliKey: "bypassPermissions", labelKey: "mode.bypass", descKey: "mode.bypassDesc" },
  { value: "dontAsk",    icon: "✅", cliKey: "dontAsk",           labelKey: "mode.dontAsk",    descKey: "mode.dontAskDesc" },
];
const currentPerm = computed(() => permOptions.find(o => o.value === activeMode.value)!);

// ── 思考深度选项 ──
interface EffortOption { value: import("@/stores/settings").Effort; icon: string; cliKey: string; labelKey: string; color: string }
const effortOptions: EffortOption[] = [
  { value: "low",       icon: "🐢", cliKey: "low",    color: "#22c55e", labelKey: "mode.effort.low" },
  { value: "medium",    icon: "🐇", cliKey: "medium", color: "#14b8a6", labelKey: "mode.effort.medium" },
  { value: "high",      icon: "🧠", cliKey: "high",   color: "#f59e0b", labelKey: "mode.effort.high" },
  { value: "xhigh",     icon: "🔬", cliKey: "xhigh",  color: "#f97316", labelKey: "mode.effort.xhigh" },
  { value: "max",       icon: "🚀", cliKey: "max",    color: "#ef4444", labelKey: "mode.effort.max" },
  { value: "ultracode", icon: "⚡", cliKey: "xhigh",  color: "#8b5cf6", labelKey: "mode.effort.ultracode" },
];
const currentEffort = computed(() => effortOptions.find(o => o.value === settings.effort)!);

// ── 连接测试 ──
const testResult = ref<string | null>(null);
const testError = ref<string | null>(null);
const isTesting = ref(false);

async function handleTest() {
  testResult.value = null; testError.value = null; isTesting.value = true;
  try { testResult.value = await connectLLM(settings.apiKey, settings.baseUrl, settings.model); }
  catch (err) { testError.value = String(err); }
  finally { isTesting.value = false; }
}

const modelPresets = [
  "deepseek-v4-pro[1M]",
  "deepseek-v4-flash",
  "deepseek-v4",
];
</script>

<template>
  <ErrorBoundary name="SettingsPanel">
    <div class="h-full overflow-y-auto">
    <div class="max-w-3xl mx-auto p-8 flex flex-col min-h-full">
      <!-- Header -->
      <div class="flex items-center gap-3 mb-8">
        <button @click="router.push('/chat')" class="w-7 h-7 flex items-center justify-center rounded-md transition-colors hover:bg-[var(--bg-hover)]" style="color:var(--text-muted)">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="m15 18-6-6 6-6"/></svg>
        </button>
        <h2 class="text-lg font-semibold tracking-tight" style="color:var(--text-bright)">{{ $t('settings.title') }}</h2>
      </div>

      <!-- 两栏布局 -->
      <div class="grid grid-cols-2 gap-8 flex-1">

        <!-- 左：API 配置 -->
        <section class="space-y-4">
          <h3 class="text-[10px] font-semibold uppercase tracking-widest" :style="{ color: 'var(--text-muted)' }">{{ $t('settings.apiConfig') }}</h3>
          <div>
            <label class="block text-xs font-medium mb-1.5" style="color:var(--text-muted)">{{ $t('settings.baseUrl') }}</label>
            <input v-model="settings.baseUrl" type="text" placeholder="https://api.deepseek.com"
              class="w-full rounded-lg px-3.5 py-2 text-sm outline-none"
              style="background:var(--bg-elevated); border:1px solid var(--border-default); color:var(--text-primary); caret-color:var(--accent)" />
          </div>
          <div>
            <label class="block text-xs font-medium mb-1.5" style="color:var(--text-muted)">{{ $t('settings.apiKey') }}</label>
            <input v-model="settings.apiKey" type="password" placeholder="sk-…"
              class="w-full rounded-lg px-3.5 py-2 text-sm outline-none"
              style="background:var(--bg-elevated); border:1px solid var(--border-default); color:var(--text-primary); caret-color:var(--accent)" />
          </div>
          <div>
            <label class="block text-xs font-medium mb-1.5" style="color:var(--text-muted)">{{ $t('settings.model') }}</label>
            <input v-model="settings.model" type="text" :list="'model-list'" placeholder="deepseek-v4-pro[1M]"
              class="w-full rounded-lg px-3.5 py-2 text-sm outline-none"
              style="background:var(--bg-elevated); border:1px solid var(--border-default); color:var(--text-primary); caret-color:var(--accent)" />
            <datalist id="model-list">
              <option v-for="m in modelPresets" :key="m" :value="m" />
            </datalist>
          </div>
          <button @click="handleTest" :disabled="isTesting || !settings.apiKey"
            class="w-full py-2.5 rounded-lg text-sm font-medium transition-all duration-150"
            :style="{ background: isTesting ? 'var(--bg-elevated)' : 'var(--accent)', color: isTesting ? 'var(--text-muted)' : '#09090b', opacity: (!settings.apiKey) ? 0.3 : 1 }">
            {{ isTesting ? $t('settings.testing') : $t('settings.test') }}
          </button>
          <div v-if="testResult" class="p-3 rounded-lg text-xs" style="background:var(--accent-glow); color:var(--accent)">✓ {{ testResult }}</div>
          <div v-if="testError" class="p-3 rounded-lg text-xs" style="background:var(--coral-glow); color:var(--coral); border:1px solid var(--coral); --tw-border-opacity:0.3">✕ {{ testError }}</div>
        </section>

        <!-- 右：cc-gui 设置 -->
        <section class="space-y-4">
          <h3 class="text-[10px] font-semibold uppercase tracking-widest" :style="{ color: 'var(--text-muted)' }">{{ $t('settings.ccGuiSettings') }}</h3>

          <!-- 语言 -->
          <div>
            <label class="block text-xs font-medium mb-1.5" style="color:var(--text-muted)">{{ $t('settings.language') }}</label>
            <select v-model="settings.locale"
              class="w-full rounded-lg px-3.5 py-2 text-xs outline-none"
              style="background:var(--bg-elevated); border:1px solid var(--border-default); color:var(--text-primary)">
              <option value="zh">中文</option>
              <option value="en">English</option>
            </select>
          </div>

          <!-- 主题 -->
          <div>
            <label class="block text-xs font-medium mb-1.5" style="color:var(--text-muted)">{{ $t('settings.theme') }}</label>
            <select v-model="settings.theme"
              class="w-full rounded-lg px-3.5 py-2 text-xs outline-none"
              style="background:var(--bg-elevated); border:1px solid var(--border-default); color:var(--text-primary)">
              <option value="dark">{{ $t('settings.themeDark') }}</option>
              <option value="light">{{ $t('settings.themeLight') }}</option>
              <option value="system">{{ $t('settings.themeSystem') }}</option>
            </select>
          </div>

          <!-- 权限模式 -->
          <div>
            <label class="block text-xs font-medium mb-1.5" style="color:var(--text-muted)">{{ $t('settings.defaultMode') }}</label>
            <div
              class="settings-dropdown relative cursor-pointer rounded-lg px-3.5 py-2 text-xs flex items-center gap-1.5 select-none transition-colors"
              :style="{
                background: 'var(--bg-elevated)',
                border: openDropdown === 'perm' ? '1px solid var(--accent)' : '1px solid var(--border-default)',
                color: 'var(--text-primary)'
              }"
              @click.stop="toggleDropdown('perm')"
            >
              <span class="text-[13px]">{{ currentPerm.icon }}</span>
              <span class="font-medium truncate">{{ $t(currentPerm.labelKey) }}</span>
              <span class="italic text-[10px] opacity-50 hidden sm:inline" style="color:var(--text-muted)">{{ currentPerm.cliKey }}</span>
              <svg width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"
                :style="{ opacity: 0.4, transition: 'transform 150ms', transform: openDropdown === 'perm' ? 'rotate(180deg)' : '' }">
                <polyline points="6 9 12 15 18 9"/>
              </svg>
              <Transition name="drop-settings">
                <div
                  v-if="openDropdown === 'perm'"
                  class="absolute right-0 top-full mt-1 py-1 rounded-lg z-30 min-w-[260px]"
                  style="background: var(--bg-elevated); border: 1px solid var(--border-default); box-shadow: 0 8px 24px rgba(0,0,0,0.35)"
                >
                  <button
                    v-for="o in permOptions"
                    :key="o.value"
                    @click="activeMode = o.value; closeDropdowns()"
                    class="w-full text-left px-3 py-2 transition-colors hover:bg-[var(--bg-hover)]"
                    :style="{ background: activeMode === o.value ? 'var(--accent-glow)' : 'transparent' }"
                  >
                    <div class="flex items-center gap-1.5">
                      <span class="text-[13px]">{{ o.icon }}</span>
                      <span class="text-xs font-medium" :style="{ color: activeMode === o.value ? 'var(--accent)' : 'var(--text-primary)' }">{{ $t(o.labelKey) }}</span>
                      <span class="italic text-[10px] ml-auto" style="color:var(--text-muted)">{{ o.cliKey }}</span>
                    </div>
                    <div class="text-[10px] mt-0.5 ml-5" style="color:var(--text-muted)">{{ $t(o.descKey) }}</div>
                  </button>
                </div>
              </Transition>
            </div>
          </div>

          <!-- 思考深度 -->
          <div>
            <label class="block text-xs font-medium mb-1.5" style="color:var(--text-muted)">{{ $t('settings.defaultEffort') }}</label>
            <div
              class="settings-dropdown relative cursor-pointer rounded-lg px-3.5 py-2 text-xs flex items-center gap-1.5 select-none transition-colors"
              :style="{
                background: 'var(--bg-elevated)',
                border: openDropdown === 'effort' ? '1px solid var(--accent)' : '1px solid var(--border-default)',
                color: currentEffort.color
              }"
              @click.stop="toggleDropdown('effort')"
            >
              <span class="text-[13px]">{{ currentEffort.icon }}</span>
              <span class="font-medium">{{ $t(currentEffort.labelKey) }}</span>
              <span class="italic text-[10px] opacity-50 hidden sm:inline" style="color:var(--text-muted)">{{ currentEffort.cliKey }}</span>
              <svg width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"
                :style="{ opacity: 0.4, transition: 'transform 150ms', transform: openDropdown === 'effort' ? 'rotate(180deg)' : '' }">
                <polyline points="6 9 12 15 18 9"/>
              </svg>
              <Transition name="drop-settings">
                <div
                  v-if="openDropdown === 'effort'"
                  class="absolute right-0 top-full mt-1 py-1 rounded-lg z-30 min-w-[220px]"
                  style="background: var(--bg-elevated); border: 1px solid var(--border-default); box-shadow: 0 8px 24px rgba(0,0,0,0.35)"
                >
                  <button
                    v-for="o in effortOptions"
                    :key="o.value"
                    @click="settings.effort = o.value; closeDropdowns()"
                    class="w-full text-left px-3 py-2 transition-colors hover:bg-[var(--bg-hover)]"
                    :style="{ background: settings.effort === o.value ? o.color + '18' : 'transparent' }"
                  >
                    <div class="flex items-center gap-1.5">
                      <span class="text-[13px]">{{ o.icon }}</span>
                      <span class="text-xs font-medium" :style="{ color: settings.effort === o.value ? o.color : 'var(--text-primary)' }">{{ $t(o.labelKey) }}</span>
                      <span class="italic text-[10px] ml-auto" style="color:var(--text-muted)">{{ o.cliKey }}</span>
                    </div>
                  </button>
                </div>
              </Transition>
            </div>
          </div>
        </section>
      </div>

      <!-- Footer：关于 -->
      <footer class="mt-8 pt-4" style="border-top:1px solid var(--border-dim)">
        <div class="flex items-center justify-between text-[10px]" :style="{ color: 'var(--text-muted)' }">
          <span>{{ $t('settings.aboutDesc') }}</span>
          <span class="font-mono">{{ $t('chat.aboutVersion') }}</span>
        </div>
      </footer>
    </div>
  </div>
  </ErrorBoundary>
</template>

<style scoped>
/* 下拉动画 */
.drop-settings-enter-active { transition: all 120ms ease-out; }
.drop-settings-leave-active { transition: all 100ms ease-in; }
.drop-settings-enter-from { opacity: 0; transform: translateY(4px) scale(0.96); }
.drop-settings-leave-to { opacity: 0; transform: translateY(2px) scale(0.98); }
</style>
