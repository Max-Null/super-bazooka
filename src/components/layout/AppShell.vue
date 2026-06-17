<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useRouter, useRoute } from "vue-router";
import SessionSidebar from "@/components/session/SessionSidebar.vue";
import FilePanel from "@/components/files/FilePanel.vue";
import CommandPalette from "@/components/shared/CommandPalette.vue";
import { emitChatCommand } from "@/composables/useCommandPalette";
import { useNewSession } from "@/composables/useNewSession";
import { getWorkspaceRoot } from "@/lib/tauri-bridge";
import { useSettingsStore } from "@/stores/settings";

const router = useRouter();
const route = useRoute();
const settings = useSettingsStore();
const { handleNew } = useNewSession();
const drawerOpen = ref(false);
const zenMode = ref(false);
const fileNavCounter = ref(0);
const filePanelForceClose = ref(0);
const cwd = ref("");

const commandPalette = ref<InstanceType<typeof CommandPalette> | null>(null);

function handleCommand(action: string) {
  switch (action) {
    // ── 💬 会话 ──
    case "new-session": handleNew(); break;
    case "continue-session":
    case "rename-session":
    case "delete-session":
    case "export-session":
    case "attach-file":
      emitChatCommand(action);
      break;

    // ── 🖥 视图 ──
    case "toggle-sidebar": drawerOpen.value = !drawerOpen.value; break;
    case "toggle-files": fileNavCounter.value++; break;
    case "zen-mode":
      zenMode.value = !zenMode.value;
      if (zenMode.value) {
        drawerOpen.value = false;
        filePanelForceClose.value++;
      }
      break;

    // ── 🛡 权限与模式 ──
    case "perm-default":
      settings.permissionMode = "default";
      settings.planMode = false;
      settings.autoMode = false;
      break;
    case "perm-plan":
      settings.planMode = true;
      settings.autoMode = false;
      break;
    case "perm-edit-auto":
      settings.permissionMode = "acceptEdits";
      settings.planMode = false;
      settings.autoMode = false;
      break;
    case "perm-auto":
      settings.autoMode = true;
      settings.planMode = false;
      break;
    case "perm-bypass":
      settings.permissionMode = "bypassPermissions";
      settings.planMode = false;
      settings.autoMode = false;
      break;
    case "perm-dontask":
      settings.permissionMode = "dontAsk";
      settings.planMode = false;
      settings.autoMode = false;
      break;

    // ── 🧠 思考深度 ──
    case "effort-low": settings.effort = "low"; break;
    case "effort-medium": settings.effort = "medium"; break;
    case "effort-high": settings.effort = "high"; break;
    case "effort-xhigh": settings.effort = "xhigh"; break;
    case "effort-max": settings.effort = "max"; break;
    case "effort-ultracode": settings.effort = "ultracode"; break;

    // ── 🔌 工具 ──
    case "open-explorer": openFilePanelTo(cwd.value || "."); break;
    case "slash-compact":
    case "slash-context":
    case "slash-cost":
    case "slash-clear":
    case "slash-review":
    case "slash-simplify":
    case "slash-security":
    case "slash-doctor":
    case "slash-init":
    case "manage-plugins":
    case "manage-mcp":
    case "manage-skills":
    case "manage-agents":
    case "manage-hooks":
    case "manage-memory":
    case "manage-permissions":
    case "manage-styles":
      emitChatCommand(action);
      break;

    // ── ⚙ 设置 ──
    case "settings": router.push("/settings"); break;
    case "theme-dark": settings.theme = "dark"; break;
    case "theme-light": settings.theme = "light"; break;
    case "theme-system": settings.theme = "system"; break;
    case "about": /* ChatPanel 弹窗处理 */ emitChatCommand(action); break;

    // ── 兼容旧 action id ──
    case "plan-mode": settings.planMode = true; settings.autoMode = false; break;
    case "auto-mode": settings.autoMode = true; settings.planMode = false; break;
    case "accept-edits": settings.permissionMode = "acceptEdits"; break;
    case "bypass": settings.permissionMode = "bypassPermissions"; break;
    case "toggle-files-legacy": fileNavCounter.value++; break;
  }
}

// ── Global keyboard shortcuts ──
function onGlobalKeydown(e: KeyboardEvent) {
  if (!(e.ctrlKey || e.metaKey)) return;
  // Skip when focused on input/textarea/contenteditable
  const tag = (e.target as HTMLElement)?.tagName?.toLowerCase();
  const isEditable = (e.target as HTMLElement)?.isContentEditable;
  if (tag === "input" || tag === "textarea" || tag === "select" || isEditable) return;

  switch (e.key.toLowerCase()) {
    case "n": e.preventDefault(); handleCommand("new-session"); break;
    case "b": e.preventDefault(); handleCommand("toggle-sidebar"); break;
    case "e": e.preventDefault(); handleCommand("toggle-files"); break;
    case ",": e.preventDefault(); handleCommand("settings"); break;
  }
}

onMounted(async () => {
  try { cwd.value = await getWorkspaceRoot(); } catch {}
  document.addEventListener("keydown", onGlobalKeydown);
  document.addEventListener("fullscreenchange", onFullscreenChange);
});

onUnmounted(() => {
  document.removeEventListener("keydown", onGlobalKeydown);
  document.removeEventListener("fullscreenchange", onFullscreenChange);
});

function isActive(path: string): boolean {
  return route.path === path;
}

const isFullscreen = ref(false);
function onFullscreenChange() { isFullscreen.value = !!document.fullscreenElement; }

function toggleFullscreen() {
  if (!document.fullscreenElement) document.documentElement.requestFullscreen();
  else document.exitFullscreen();
}

function openFilePanelTo(path: string) {
  fileNavCounter.value++;
}
</script>

<template>
  <div class="h-screen flex flex-col" style="background:var(--bg-root)">
    <!-- Navbar -->
    <header
      class="flex items-center h-11 px-4 shrink-0 select-none"
      style="background:var(--bg-surface); border-bottom:1px solid var(--border-dim)"
    >
      <!-- Logo group -->
      <!-- Logo + CWD — left aligned, baseline aligned -->
      <div class="flex items-baseline gap-2.5 flex-1 min-w-0">
        <button
          @click="drawerOpen = !drawerOpen"
          class="w-7 h-7 flex items-center justify-center rounded-md transition-colors shrink-0 translate-y-0.5"
          :style="{ background: drawerOpen ? 'var(--bg-active)' : 'transparent', color: 'var(--text-secondary)' }"
          :class="drawerOpen ? '' : 'hover:bg-[var(--bg-hover)]'"
          :title="$t('header.toggleSidebar')"
        >
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <path d="M3 12h18M3 6h18M3 18h18" />
          </svg>
        </button>

        <span class="text-sm font-semibold tracking-tight leading-none" style="color:var(--text-bright)">cc-gui</span>

        <!-- 禅模式 -->
        <button
          @click="handleCommand('zen-mode')"
          class="w-6 h-6 flex items-center justify-center rounded-md transition-colors shrink-0"
          :style="{ background: zenMode ? 'var(--accent-glow)' : 'transparent', color: zenMode ? 'var(--accent)' : 'var(--text-muted)' }"
          :class="zenMode ? '' : 'hover:bg-[var(--bg-hover)]'"
          :title="zenMode ? $t('header.exitZenMode') : $t('header.zenMode')"
        >🧘</button>

        <span class="text-[10px] font-medium px-1.5 py-px rounded leading-none" style="background:var(--accent-glow); color:var(--accent-dim)">DEV</span>

        <!-- CWD — clickable, opens file panel to workspace root -->
        <button
          v-if="cwd"
          @click="openFilePanelTo(cwd)"
          class="text-[10px] font-mono truncate ml-1 px-2 py-0.5 rounded leading-none cursor-pointer transition-colors hover:border-[var(--accent)]"
          :style="{ background: 'var(--bg-root)', color: 'var(--accent)', border: '1px solid var(--border-dim)' }"
          :title="cwd + ' — click to browse'"
        >{{ cwd }}</button>
      </div>

      <!-- Actions group -->
      <div class="flex items-center gap-1">
        <!-- New session -->
        <button
          @click="handleCommand('new-session')"
          class="w-7 h-7 flex items-center justify-center rounded-md transition-colors hover:bg-[var(--bg-hover)]"
          style="color:var(--text-secondary)"
          :title="$t('header.newSession')"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
        </button>

        <!-- Theme toggle -->
        <button
          @click="settings.theme = settings.theme === 'dark' ? 'light' : 'dark'"
          class="w-7 h-7 flex items-center justify-center rounded-md transition-colors hover:bg-[var(--bg-hover)]"
          style="color:var(--text-secondary)"
          :title="settings.theme === 'dark' ? $t('header.lightMode') : $t('header.darkMode')"
        >
          <!-- Sun icon (light mode indicator) -->
          <svg v-if="settings.theme === 'dark'" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="5" />
            <line x1="12" y1="1" x2="12" y2="3" />
            <line x1="12" y1="21" x2="12" y2="23" />
            <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
            <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
            <line x1="1" y1="12" x2="3" y2="12" />
            <line x1="21" y1="12" x2="23" y2="12" />
            <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
            <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
          </svg>
          <!-- Moon icon (dark mode indicator) -->
          <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
          </svg>
        </button>

        <!-- 全屏 -->
        <button
          @click="toggleFullscreen"
          class="w-7 h-7 flex items-center justify-center rounded-md transition-colors hover:bg-[var(--bg-hover)]"
          style="color:var(--text-secondary)"
          :title="isFullscreen ? $t('header.exitFullscreen') : $t('header.fullscreen')"
        >
          <!-- 进入全屏：展开图标 -->
          <svg v-if="!isFullscreen" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="15 3 21 3 21 9" /><polyline points="9 21 3 21 3 15" /><line x1="21" y1="3" x2="14" y2="10" /><line x1="3" y1="21" x2="10" y2="14" />
          </svg>
          <!-- 退出全屏：收缩图标 -->
          <svg v-else width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="4 8 4 3 9 3" /><polyline points="20 16 20 21 15 21" /><line x1="4" y1="3" x2="10" y2="9" /><line x1="20" y1="21" x2="14" y2="15" />
          </svg>
        </button>

        <!-- Refresh -->
        <button
          @click="router.go(0)"
          class="w-7 h-7 flex items-center justify-center rounded-md transition-colors hover:bg-[var(--bg-hover)]"
          style="color:var(--text-secondary)"
          :title="$t('header.refresh')"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8" />
            <path d="M21 3v5h-5" />
            <path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16" />
            <path d="M3 21v-5h5" />
          </svg>
        </button>

        <!-- Settings -->
        <button
          @click="router.push(isActive('/settings') ? '/chat' : '/settings')"
          class="w-7 h-7 flex items-center justify-center rounded-md transition-colors"
          :style="{
            background: isActive('/settings') ? 'var(--accent-glow)' : 'transparent',
            color: isActive('/settings') ? 'var(--accent)' : 'var(--text-secondary)'
          }"
          :class="isActive('/settings') ? '' : 'hover:bg-[var(--bg-hover)]'"
          :title="$t('header.settings')"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
            <circle cx="12" cy="12" r="3" />
          </svg>
        </button>
      </div>
    </header>

    <!-- Body -->
    <div class="flex-1 flex overflow-hidden">
      <!-- Drawer -->
      <aside
        class="shrink-0 overflow-hidden transition-all duration-200 ease-in-out"
        :style="{
          width: drawerOpen ? '260px' : '0',
          borderRight: drawerOpen ? '1px solid var(--border-dim)' : 'none',
          background: 'var(--bg-surface)'
        }"
      >
        <div class="w-[260px] h-full">
          <SessionSidebar @navigate="drawerOpen = false" />
        </div>
      </aside>

      <!-- Main -->
      <main class="flex-1 flex overflow-hidden">
        <router-view />
      </main>

      <!-- File panel (right side) -->
      <FilePanel :navCounter="fileNavCounter" :navPath="cwd" :forceClose="filePanelForceClose" />
    </div>

    <CommandPalette @command="handleCommand" />
  </div>
</template>
