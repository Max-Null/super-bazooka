<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, nextTick, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useCommandPaletteBus } from "@/composables/useCommandPalette";
import { useSettingsStore } from "@/stores/settings";
import { toPinyinInitials } from "@/lib/pinyin";
import ModalShell from "./ModalShell.vue";

const { t } = useI18n();
const settings = useSettingsStore();

const emit = defineEmits<{
  close: [];
  command: [action: string];
}>();

// ── 类型定义 ──
interface CommandAction {
  id: string;
  group: string;
  labelKey: string;
  descKey?: string;
  keys?: string;
  icon?: string;
  /** 对应的 CLI 命令名，斜体浅色显示在中文后，如 /context */
  cliKey?: string;
  /** 仅在特定条件下显示，省略则始终显示 */
  visible?: () => boolean;
}

// ── 命令定义（对标 Claude Code 功能域，6 大分组）──
const actions: CommandAction[] = [
  // ── 💬 会话 ──
  { id: "new-session",     group: "session",    labelKey: "command.newSession",     keys: "Ctrl+N", icon: "🆕" },
  { id: "continue-session",group: "session",    labelKey: "command.continueSession", cliKey: "--continue",  icon: "📋" },
  { id: "resume-session",  group: "session",    labelKey: "command.resumeSession",   cliKey: "--resume",    icon: "📂" },
  { id: "rename-session",  group: "session",    labelKey: "command.renameSession",  keys: "F2",       icon: "✏️" },
  { id: "delete-session",  group: "session",    labelKey: "command.deleteSession",  keys: "Del",      icon: "🗑️" },
  { id: "clear-conversation", group: "session", labelKey: "command.clearConversation", descKey: "command.clearConversationDesc", cliKey: "/clear", icon: "🧹" },
  { id: "export-session",  group: "session",    labelKey: "command.exportSession",  descKey: "command.exportSessionDesc", icon: "📤" },
  { id: "switch-session",  group: "session",    labelKey: "command.switchSession",  keys: "Ctrl+P",   icon: "🔀" },

  // ── 🖥 视图 ──
  { id: "toggle-sidebar",  group: "view",       labelKey: "command.toggleSidebar",  keys: "Ctrl+B",   icon: "📋" },
  { id: "toggle-files",    group: "view",       labelKey: "command.toggleFiles",    keys: "Ctrl+E",   icon: "📂" },
  { id: "focus-input",     group: "view",       labelKey: "command.focusInput",     keys: "Ctrl+L",   icon: "⌨️" },
  { id: "toggle-fullscreen", group: "view",     labelKey: "command.toggleFullscreen", keys: "F11",     icon: "🖥️" },
  { id: "zen-mode",        group: "view",       labelKey: "command.zenMode",        descKey: "command.zenModeDesc", icon: "🧘" },

  // ── 🛡 权限与模式 ──
  { id: "perm-default",    group: "permission", labelKey: "command.permDefault",    descKey: "command.permDefaultDesc",    cliKey: "default", icon: "🔒" },
  { id: "perm-plan",       group: "permission", labelKey: "command.permPlan",       descKey: "command.permPlanDesc",       cliKey: "plan",    icon: "📋" },
  { id: "perm-edit-auto",  group: "permission", labelKey: "command.permEditAuto",   descKey: "command.permEditAutoDesc",   cliKey: "acceptEdits", icon: "✏️" },
  { id: "perm-auto",       group: "permission", labelKey: "command.permAuto",       descKey: "command.permAutoDesc",       cliKey: "auto",    icon: "🤖" },
  { id: "perm-bypass",     group: "permission", labelKey: "command.permBypass",     descKey: "command.permBypassDesc",     cliKey: "bypassPermissions", icon: "⚡" },
  { id: "perm-dontask",    group: "permission", labelKey: "command.permDontAsk",    descKey: "command.permDontAskDesc",    cliKey: "dontAsk", icon: "✅" },

  // ── 🧠 思考深度 ──
  { id: "effort-low",      group: "effort",     labelKey: "command.effortLow",       cliKey: "low",     icon: "🐢" },
  { id: "effort-medium",   group: "effort",     labelKey: "command.effortMedium",    cliKey: "medium",  icon: "🐇" },
  { id: "effort-high",     group: "effort",     labelKey: "command.effortHigh",      cliKey: "high",    icon: "🧠" },
  { id: "effort-xhigh",    group: "effort",     labelKey: "command.effortXhigh",     cliKey: "xhigh",   icon: "🔬" },
  { id: "effort-max",      group: "effort",     labelKey: "command.effortMax",       cliKey: "max",     icon: "🚀" },
  { id: "effort-ultracode",group: "effort",     labelKey: "command.effortUltracode", descKey: "command.effortUltracodeDesc", cliKey: "ultracode", icon: "⚡" },

  // ── 📊 上下文 ──
  { id: "compact",         group: "context",    labelKey: "command.compactContext",  descKey: "command.compactContextDesc",  cliKey: "/compact", icon: "🗜️" },
  { id: "show-usage",      group: "context",    labelKey: "command.viewUsage",       descKey: "command.viewUsageDesc",       cliKey: "/context", icon: "📊" },
  { id: "show-cost",       group: "context",    labelKey: "command.viewCost",        cliKey: "/cost",    icon: "💰" },

  // ── 🔌 工具 ──
  { id: "attach-file",     group: "tools",      labelKey: "command.attachFile",      descKey: "command.attachFileDesc",     icon: "📎" },
  { id: "code-review",     group: "tools",      labelKey: "command.codeReview",      descKey: "command.codeReviewDesc",     cliKey: "code-review", icon: "🔍" },
  { id: "code-simplify",   group: "tools",      labelKey: "command.codeSimplify",    descKey: "command.codeSimplifyDesc",   cliKey: "code-simplifier", icon: "✨" },
  { id: "security-audit",  group: "tools",      labelKey: "command.securityAudit",   descKey: "command.securityAuditDesc",  cliKey: "security-review", icon: "🛡️" },
  { id: "modernize-assess",group: "tools",      labelKey: "command.modernizeAssess", descKey: "command.modernizeAssessDesc",cliKey: "modernize", icon: "🏗️" },
  { id: "open-terminal",   group: "tools",      labelKey: "command.openTerminal",    descKey: "command.openTerminalDesc",   icon: "💻" },
  { id: "open-explorer",   group: "tools",      labelKey: "command.openExplorer",    descKey: "command.openExplorerDesc",   icon: "📁" },
  { id: "dev-tools",       group: "tools",      labelKey: "command.devTools",        descKey: "command.devToolsDesc",       icon: "🔧" },
  { id: "run-doctor",      group: "tools",      labelKey: "command.runDoctor",       descKey: "command.runDoctorDesc",      cliKey: "/doctor", icon: "🩺" },
  { id: "init-claude-md",  group: "tools",      labelKey: "command.initClaudeMd",    descKey: "command.initClaudeMdDesc",   cliKey: "/init",   icon: "📝" },
  { id: "keybindings-ref", group: "tools",      labelKey: "command.keybindingsReference", descKey: "command.keybindingsReferenceDesc", icon: "⌨️" },

  // ── ⚙ 设置 ──
  { id: "settings",        group: "settings",   labelKey: "command.openSettings",    keys: "Ctrl+,",   icon: "⚙️" },
  { id: "theme-dark",      group: "settings",   labelKey: "command.themeDark",                          icon: "🌙" },
  { id: "theme-light",     group: "settings",   labelKey: "command.themeLight",                         icon: "☀️" },
  { id: "theme-system",    group: "settings",   labelKey: "command.themeSystem",                        icon: "🔄" },
  { id: "manage-approvals",group: "settings",   labelKey: "command.manageApprovals", descKey: "command.manageApprovalsDesc", icon: "✅" },
  { id: "test-connection", group: "settings",   labelKey: "command.testConnection",  descKey: "command.testConnectionDesc", icon: "🔌" },
  { id: "check-update",    group: "settings",   labelKey: "command.checkUpdate",                         icon: "🆙" },
  { id: "about",           group: "settings",   labelKey: "command.about",           descKey: "command.aboutDesc",          icon: "ℹ️" },
];

// ── 分组顺序 ──
const groupOrder = ["session", "view", "permission", "effort", "context", "tools", "settings"];

// ── 状态 ──
const open = ref(false);
const query = ref("");
const selectedIdx = ref(0);
const inputEl = ref<HTMLInputElement | null>(null);

// ── 搜索匹配 ──
// 策略：id > 中文 label > 拼音首字母 > 描述 > 分组名 > 快捷键
function matchesQuery(a: CommandAction, q: string): boolean {
  const ql = q.toLowerCase();
  // 匹配 id
  if (a.id.toLowerCase().includes(ql)) return true;
  // 匹配翻译后的中文 label
  const label = t(a.labelKey).toLowerCase();
  if (label.includes(ql)) return true;
  // 匹配拼音首字母（xjhh → 新建会话）
  if (ql.length >= 2 && toPinyinInitials(label).includes(ql)) return true;
  // 匹配描述
  if (a.descKey) {
    const desc = t(a.descKey).toLowerCase();
    if (desc.includes(ql)) return true;
  }
  // 匹配分组名
  const groupLabel = t(`command.groups.${a.group}`).toLowerCase();
  if (groupLabel.includes(ql)) return true;
  // 匹配快捷键
  if (a.keys && a.keys.toLowerCase().includes(ql)) return true;
  return false;
}

// ── 当前选中状态（权限模式 / 思考深度 / 主题）──
function isActive(action: CommandAction): boolean {
  const g = action.group;
  if (g === "permission") {
    if (settings.planMode) return action.id === "perm-plan";
    if (settings.autoMode) return action.id === "perm-auto";
    const modeMap: Record<string, string> = {
      default: "perm-default",
      acceptEdits: "perm-edit-auto",
      bypassPermissions: "perm-bypass",
      dontAsk: "perm-dontask",
    };
    return action.id === (modeMap[settings.permissionMode] || "");
  }
  if (g === "effort") return action.id === `effort-${settings.effort}`;
  if (g === "settings") {
    if (action.id === "theme-dark") return settings.theme === "dark";
    if (action.id === "theme-light") return settings.theme === "light";
    if (action.id === "theme-system") return settings.theme === "system";
  }
  return false;
}

// ── 分组后的命令列表 ──
interface FlatItem {
  type: "group" | "command";
  groupId?: string;
  action?: CommandAction;
  actionIndex?: number; // 在所有可见 action 中的索引
}

const flatList = computed<FlatItem[]>(() => {
  const q = query.value.trim();
  const visible = q
    ? actions.filter((a) => matchesQuery(a, q))
    : actions.filter((a) => (a.visible ? a.visible() : true));

  const items: FlatItem[] = [];
  let actionIdx = 0;
  for (const gid of groupOrder) {
    const groupActions = visible.filter((a) => a.group === gid);
    if (groupActions.length === 0) continue;
    items.push({ type: "group", groupId: gid });
    for (const a of groupActions) {
      items.push({ type: "command", action: a, groupId: gid, actionIndex: actionIdx });
      actionIdx++;
    }
  }
  return items;
});

// ── 最近使用（localStorage 持久化，最多 5 条）──
const RECENT_KEY = "cc-gui-cmd-recent";
const recentIds = ref<string[]>(loadRecent());

function loadRecent(): string[] {
  try {
    const raw = localStorage.getItem(RECENT_KEY);
    if (raw) return JSON.parse(raw);
  } catch {}
  return [];
}

function saveRecent() {
  localStorage.setItem(RECENT_KEY, JSON.stringify(recentIds.value));
}

function pushRecent(id: string) {
  recentIds.value = [id, ...recentIds.value.filter((x) => x !== id)].slice(0, 5);
  saveRecent();
}

// 无搜索词时顶部显示最近使用，有搜索词时按搜索结果展示
const recentActions = computed(() => {
  if (query.value.trim()) return []; // 有搜索词时不显示最近使用
  return recentIds.value
    .map((id) => actions.find((a) => a.id === id))
    .filter((a): a is CommandAction => !!a);
});

// ── 只含命令的平铺列表（用于键盘导航）──
const commandItems = computed(() =>
  flatList.value.filter((it): it is FlatItem & { action: CommandAction; actionIndex: number } => it.type === "command")
);

// ── 操作 ──
function show() {
  open.value = true;
  query.value = "";
  selectedIdx.value = 0;
  nextTick(() => inputEl.value?.focus());
}
function hide() { open.value = false; emit("close"); }
function run(action: string) { pushRecent(action); hide(); emit("command", action); }

// 监听外部触发
const bus = useCommandPaletteBus();
watch(() => bus.trigger.value, () => { show(); });

function onKeydown(e: KeyboardEvent) {
  if (!open.value) {
    if ((e.ctrlKey || e.metaKey) && e.key === "k") { e.preventDefault(); show(); }
    return;
  }
  // ESC 由 ModalShell 统一处理
  if (e.key === "ArrowDown") {
    e.preventDefault();
    // 跳过 group 标题行，跳转到下一个命令
    const cmdCount = commandItems.value.length;
    if (cmdCount > 0) selectedIdx.value = Math.min(selectedIdx.value + 1, cmdCount - 1);
  }
  else if (e.key === "ArrowUp") {
    e.preventDefault();
    selectedIdx.value = Math.max(selectedIdx.value - 1, 0);
  }
  else if (e.key === "Enter") {
    e.preventDefault();
    const cmd = commandItems.value[selectedIdx.value];
    if (cmd) run(cmd.action.id);
  }
}

onMounted(() => document.addEventListener("keydown", onKeydown));
onUnmounted(() => document.removeEventListener("keydown", onKeydown));

defineExpose({ show, hide });
</script>

<template>
  <ModalShell :open="open" position="top" size="xl" @close="hide">
    <template #header>
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="var(--text-muted)" stroke-width="2" class="shrink-0"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/></svg>
      <input
        ref="inputEl"
        v-model="query"
        :placeholder="t('command.placeholder')"
        class="flex-1 bg-transparent text-sm outline-none focus:outline-none min-w-0 ml-2.5"
        :style="{ color: 'var(--text-bright)', caretColor: 'var(--accent)' }"
      />
    </template>

    <!-- 最近使用（无搜索词时显示） -->
    <template v-if="recentActions.length > 0">
      <div class="px-1 pt-1 pb-1 text-[10px] font-semibold tracking-wider uppercase select-none" :style="{ color: 'var(--text-muted)' }">
        {{ t('command.recent') }}
      </div>
      <button
        v-for="a in recentActions"
        :key="'recent-' + a.id"
        @click="run(a.id)"
        class="w-full flex items-center gap-3 px-1 py-2 text-sm transition-colors text-left hover:bg-[var(--bg-hover)]"
      >
        <span v-if="a.icon" class="text-base shrink-0 w-5 text-center">{{ a.icon }}</span>
        <span v-else class="w-5 shrink-0"></span>
        <span class="flex-1 text-sm truncate" :style="{ color: 'var(--text-secondary)' }">
          {{ t(a.labelKey) }}
          <span v-if="a.cliKey" class="italic text-[11px] ml-1.5" :style="{ color: 'var(--text-muted)' }">{{ a.cliKey }}</span>
        </span>
        <kbd v-if="a.keys" class="text-[10px] px-1.5 py-0.5 rounded shrink-0" :style="{ background: 'var(--bg-root)', color: 'var(--text-muted)' }">{{ a.keys }}</kbd>
      </button>
      <div class="mx-1 my-1 border-t" :style="{ borderColor: 'var(--border-dim)' }"></div>
    </template>

    <!-- 分组命令列表 -->
    <template v-for="(item, i) in flatList" :key="item.type === 'group' ? `g-${item.groupId}` : `c-${item.action!.id}`">
      <div
        v-if="item.type === 'group'"
        class="px-1 pt-3 pb-1 text-[10px] font-semibold tracking-wider uppercase select-none"
        :style="{ color: 'var(--text-muted)' }"
      >
        {{ t(`command.groups.${item.groupId}`) }}
      </div>

      <button
        v-else
        @click="run(item.action!.id)"
        :class="[
          'w-full flex items-center gap-3 px-1 py-2 text-sm transition-colors text-left',
          item.actionIndex === selectedIdx
            ? 'bg-[var(--accent)]/10'
            : 'hover:bg-[var(--bg-hover)]'
        ]"
      >
        <span v-if="item.action!.icon" class="text-base shrink-0 w-5 text-center">{{ item.action!.icon }}</span>
        <span v-else class="w-5 shrink-0"></span>

        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-1.5">
            <span class="text-sm" :style="{ color: item.actionIndex === selectedIdx ? 'var(--accent)' : 'var(--text-secondary)' }">
              <span class="truncate">{{ t(item.action!.labelKey) }}</span>
              <span v-if="item.action!.cliKey" class="italic text-[11px] ml-1.5 shrink-0" :style="{ color: 'var(--text-muted)' }">{{ item.action!.cliKey }}</span>
            </span>
            <span v-if="item.action && isActive(item.action)" class="shrink-0 w-1.5 h-1.5 rounded-full" style="background: var(--accent)"></span>
          </div>
          <div v-if="item.action!.descKey" class="text-[11px] truncate mt-0.5" :style="{ color: 'var(--text-muted)' }">
            {{ t(item.action!.descKey) }}
          </div>
        </div>

        <kbd v-if="item.action!.keys" class="text-[10px] px-1.5 py-0.5 rounded shrink-0" :style="{ background: 'var(--bg-root)', color: 'var(--text-muted)' }">{{ item.action!.keys }}</kbd>
      </button>
    </template>

    <div
      v-if="flatList.filter(it => it.type === 'command').length === 0"
      class="px-1 py-8 text-center text-sm"
      :style="{ color: 'var(--text-muted)' }"
    >
      {{ t('command.noResults') }}
    </div>
  </ModalShell>
</template>
