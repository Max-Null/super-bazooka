<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useSessionStore } from "@/stores/session";
import { useChatStore } from "@/stores/chat";
import { useDebugLog } from "@/composables/useDebugLog";
import { useSettingsStore } from "@/stores/settings";
import { listMessages } from "@/lib/tauri-bridge";

const router = useRouter();
const sessionStore = useSessionStore();
const chatStore = useChatStore();
const debugLog = useDebugLog();
const settings = useSettingsStore();

const editingId = ref<string | null>(null);
const editingTitle = ref("");

onMounted(async () => { await sessionStore.loadSessions(); });

async function handleSelect(id: string) {
  sessionStore.setActiveSession(id);
  debugLog.clear();
  try {
    const msgs = await listMessages(id);
    chatStore.loadMessages(msgs.map(m => ({
      id: m.id,
      role: m.role,
      content: m.content,
      created_at: m.created_at,
    })));
  } catch {
    chatStore.clearMessages();
  }
  router.push("/chat");
}

async function handleNew() {
  const id = await sessionStore.createSession(settings.model);
  chatStore.clearMessages();
  debugLog.clear();
  router.push("/chat");
}

function startRename(id: string, title: string) { editingId.value = id; editingTitle.value = title; }
async function finishRename(id: string) {
  const t = editingTitle.value.trim();
  if (t) await sessionStore.renameSession(id, t);
  editingId.value = null;
}
function cancelRename() { editingId.value = null; }
async function handleDelete(id: string) {
  await sessionStore.deleteSession(id);
  if (sessionStore.activeSessionId === id) {
    sessionStore.sessions.length > 0 ? handleSelect(sessionStore.sessions[0].id) : handleNew();
  }
}
</script>

<template>
  <div class="flex flex-col h-full">
    <!-- Header -->
    <div class="flex items-center justify-between px-3 py-3">
      <span class="text-[11px] font-semibold uppercase tracking-widest" style="color:var(--text-muted)">Sessions</span>
      <button
        @click="handleNew"
        class="w-6 h-6 flex items-center justify-center rounded-md transition-colors hover:bg-[var(--bg-hover)]"
        style="color:var(--text-secondary)"
        title="New session"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <path d="M12 5v14M5 12h14" />
        </svg>
      </button>
    </div>

    <!-- List -->
    <div class="flex-1 overflow-y-auto px-2 pb-2 space-y-px">
      <button
        v-for="s in sessionStore.sessions"
        :key="s.id"
        @click="handleSelect(s.id)"
        class="w-full flex items-center gap-2.5 px-3 py-2 rounded-md text-left text-[13px] transition-colors group"
        :style="{
          background: s.id === sessionStore.activeSessionId ? 'var(--accent-glow)' : 'transparent',
          color: s.id === sessionStore.activeSessionId ? 'var(--accent)' : 'var(--text-secondary)'
        }"
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" :style="{ opacity: s.id === sessionStore.activeSessionId ? 1 : 0.35 }">
          <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
        </svg>

        <input
          v-if="editingId === s.id"
          v-model="editingTitle"
          @keydown.enter="finishRename(s.id)"
          @keydown.escape="cancelRename()"
          @blur="finishRename(s.id)"
          @click.stop
          class="flex-1 bg-transparent border-0 outline-0 text-[13px]"
          :style="{ color: 'var(--text-bright)', borderBottom: '1px solid var(--accent)' }"
          maxlength="100"
        />
        <span v-else class="truncate flex-1">{{ s.title }}</span>

        <!-- Hover actions -->
        <div class="hidden group-hover:flex items-center gap-0.5 shrink-0 ml-auto">
          <button @click.stop="startRename(s.id, s.title)" class="w-5 h-5 flex items-center justify-center rounded transition-colors hover:bg-[var(--bg-active)]" style="color:var(--text-muted)" title="Rename">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M17 3a2.85 2.85 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z"/></svg>
          </button>
          <button @click.stop="handleDelete(s.id)" class="w-5 h-5 flex items-center justify-center rounded transition-colors hover:bg-[var(--bg-active)]" style="color:var(--text-muted)" title="Delete">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
          </button>
        </div>
      </button>

      <div v-if="sessionStore.sessions.length === 0" class="px-3 py-12 text-center text-xs" style="color:var(--text-muted)">
        No sessions yet
      </div>
    </div>
  </div>
</template>
