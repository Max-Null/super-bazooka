<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useSessionStore } from "@/stores/session";
import { useChatStore } from "@/stores/chat";
import { useDebugLog } from "@/composables/useDebugLog";
import { useNewSession } from "@/composables/useNewSession";
import { listMessages } from "@/lib/tauri-bridge";
import { formatTokenCount } from "@/lib/utils";

const router = useRouter();
const sessionStore = useSessionStore();
const chatStore = useChatStore();
const debugLog = useDebugLog();
const { handleNew } = useNewSession();

const searchQuery = ref("");
const editingId = ref<string | null>(null);
const editingTitle = ref("");

const filteredSessions = computed(() => {
  const q = searchQuery.value.trim().toLowerCase();
  if (!q) return sessionStore.sessions;
  return sessionStore.sessions.filter(s => s.title.toLowerCase().includes(q));
});

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

    <!-- Search -->
    <div class="px-3 pb-2">
      <div class="flex items-center gap-2 px-2.5 py-1.5 rounded-md text-xs" :style="{ background: 'var(--bg-root)', border: '1px solid var(--border-dim)' }">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--text-muted)" stroke-width="2"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/></svg>
        <input
          v-model="searchQuery"
          placeholder="Search sessions…"
          class="flex-1 bg-transparent outline-none text-xs"
          :style="{ color: 'var(--text-primary)', caretColor: 'var(--accent)' }"
        />
        <button
          v-if="searchQuery"
          @click="searchQuery = ''"
          class="w-4 h-4 flex items-center justify-center rounded transition-colors hover:bg-[var(--bg-hover)]"
          style="color: var(--text-muted)"
          title="Clear"
        >✕</button>
      </div>
    </div>

    <!-- List -->
    <div class="flex-1 overflow-y-auto px-2 pb-2 space-y-px">
      <button
        v-for="s in filteredSessions"
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
        <div v-else class="truncate flex-1 min-w-0">
          <span class="block truncate">{{ s.title }}</span>
          <span
            v-if="s.totalTokens"
            class="block text-[10px] truncate"
            :style="{ color: 'var(--text-muted)' }"
          >
            {{ formatTokenCount(s.totalTokens) }}
            <span v-if="s.totalCost" class="ml-1">· ${{ s.totalCost.toFixed(3) }}</span>
          </span>
        </div>

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
      <div v-else-if="filteredSessions.length === 0" class="px-3 py-12 text-center text-xs" style="color:var(--text-muted)">
        No matching sessions
      </div>
    </div>
  </div>
</template>
