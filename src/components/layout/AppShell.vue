<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter, useRoute } from "vue-router";
import SessionSidebar from "@/components/session/SessionSidebar.vue";
import FilePanel from "@/components/files/FilePanel.vue";
import { getWorkspaceRoot } from "@/lib/tauri-bridge";

const router = useRouter();
const route = useRoute();
const drawerOpen = ref(false);
const cwd = ref("");
const fileNavCounter = ref(0);    // increment to trigger FilePanel navigation

onMounted(async () => {
  try { cwd.value = await getWorkspaceRoot(); } catch {}
});

function isActive(path: string): boolean {
  return route.path === path;
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
          title="Toggle sidebar"
        >
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <path d="M3 12h18M3 6h18M3 18h18" />
          </svg>
        </button>

        <span class="text-sm font-semibold tracking-tight leading-none" style="color:var(--text-bright)">cc-gui</span>
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
        <!-- Refresh -->
        <button
          @click="router.go(0)"
          class="w-7 h-7 flex items-center justify-center rounded-md transition-colors hover:bg-[var(--bg-hover)]"
          style="color:var(--text-secondary)"
          title="Refresh"
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
          title="Settings"
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
      <FilePanel :navCounter="fileNavCounter" :navPath="cwd" />
    </div>
  </div>
</template>
