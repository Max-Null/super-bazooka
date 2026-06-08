<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, nextTick } from "vue";

const emit = defineEmits<{
  close: [];
  command: [action: string];
}>();

const open = ref(false);
const query = ref("");
const selectedIdx = ref(0);
const inputEl = ref<HTMLInputElement | null>(null);

const actions = [
  { id: "new-session", label: "New Session", keys: "Ctrl+N" },
  { id: "toggle-sidebar", label: "Toggle Sidebar", keys: "Ctrl+B" },
  { id: "toggle-files", label: "Toggle File Panel", keys: "Ctrl+E" },
  { id: "settings", label: "Open Settings", keys: "Ctrl+," },
  { id: "plan-mode", label: "Plan Mode", keys: "" },
  { id: "auto-mode", label: "Auto Mode", keys: "" },
  { id: "accept-edits", label: "Edit Automatically", keys: "" },
  { id: "bypass", label: "Bypass Permissions", keys: "" },
];

const filtered = computed(() => {
  if (!query.value) return actions;
  const q = query.value.toLowerCase();
  return actions.filter((a) => a.label.toLowerCase().includes(q) || a.id.toLowerCase().includes(q));
});

function show() { open.value = true; query.value = ""; selectedIdx.value = 0; nextTick(() => inputEl.value?.focus()); }
function hide() { open.value = false; emit("close"); }
function run(action: string) { hide(); emit("command", action); }

function onKeydown(e: KeyboardEvent) {
  if (!open.value) {
    if ((e.ctrlKey || e.metaKey) && e.key === "k") { e.preventDefault(); show(); }
    return;
  }
  if (e.key === "Escape") { e.preventDefault(); hide(); }
  else if (e.key === "ArrowDown") { e.preventDefault(); selectedIdx.value = Math.min(selectedIdx.value + 1, filtered.value.length - 1); }
  else if (e.key === "ArrowUp") { e.preventDefault(); selectedIdx.value = Math.max(selectedIdx.value - 1, 0); }
  else if (e.key === "Enter") { e.preventDefault(); if (filtered.value[selectedIdx.value]) run(filtered.value[selectedIdx.value].id); }
}

onMounted(() => document.addEventListener("keydown", onKeydown));
onUnmounted(() => document.removeEventListener("keydown", onKeydown));

defineExpose({ show, hide });
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="fixed inset-0 z-50 flex items-start justify-center pt-[20vh]" @click.self="hide">
      <div class="w-[480px] rounded-xl overflow-hidden shadow-2xl border" :style="{ background: 'var(--bg-elevated)', borderColor: 'var(--border-default)' }">
        <!-- Input -->
        <div class="flex items-center px-4 h-12 border-b" :style="{ borderColor: 'var(--border-dim)' }">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--text-muted)" stroke-width="2"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/></svg>
          <input
            ref="inputEl"
            v-model="query"
            placeholder="Type a command…"
            class="flex-1 ml-3 bg-transparent text-sm outline-none"
            :style="{ color: 'var(--text-primary)', caretColor: 'var(--accent)' }"
          />
          <kbd class="text-[10px] px-1.5 py-0.5 rounded" :style="{ background: 'var(--bg-hover)', color: 'var(--text-muted)' }">esc</kbd>
        </div>

        <!-- List -->
        <div class="max-h-64 overflow-y-auto py-1">
          <button
            v-for="(action, i) in filtered"
            :key="action.id"
            @click="run(action.id)"
            :class="['w-full flex items-center justify-between px-4 py-2 text-sm transition-colors',
              i === selectedIdx ? 'bg-[var(--accent)]/10 text-[var(--accent)]' : 'hover:bg-[var(--bg-hover)]']"
            :style="{ color: i === selectedIdx ? 'var(--accent)' : 'var(--text-secondary)' }"
          >
            <span>{{ action.label }}</span>
            <kbd v-if="action.keys" class="text-[10px] px-1.5 py-0.5 rounded" :style="{ background: 'var(--bg-root)', color: 'var(--text-muted)' }">{{ action.keys }}</kbd>
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
