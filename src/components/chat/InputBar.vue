<script setup lang="ts">
import { ref, nextTick } from "vue";

const props = defineProps<{ disabled: boolean; autoMode?: boolean }>();
const emit = defineEmits<{ send: [text: string]; files: [files: Array<{ name: string; path: string }>]; stop: [] }>();

function onDrop(e: DragEvent) {
  e.preventDefault();
  isDragOver.value = false;
  const files = e.dataTransfer?.files;
  if (!files || files.length === 0) return;

  const attached: Array<{ name: string; path: string }> = [];
  for (let i = 0; i < files.length; i++) {
    const f = files[i];
    attached.push({ name: f.name, path: (f as any).path || f.name });
  }
  emit("files", attached);
}

const input = ref("");
const focused = ref(false);
const isDragOver = ref(false);

function send() {
  const text = input.value.trim();
  if (!text || props.disabled) return;
  emit("send", text);
  input.value = "";
}

// ── Drag & drop files ──
function onDragOver(e: DragEvent) {
  e.preventDefault();
  if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
  isDragOver.value = true;
}

function onDragLeave() {
  isDragOver.value = false;
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); send(); }
}

function onPaste(e: ClipboardEvent) {
  const items = e.clipboardData?.items;
  if (!items) return;
  const files: Array<{ name: string; path: string }> = [];
  for (let i = 0; i < items.length; i++) {
    const item = items[i];
    if (item.kind === "file") {
      const file = item.getAsFile();
      if (file) {
        files.push({ name: file.name, path: (file as any).path || file.name });
      }
    }
  }
  if (files.length > 0) {
    e.preventDefault();
    emit("files", files);
  }
}

async function autoResize() {
  await nextTick();
  const el = document.querySelector(".chat-textarea") as HTMLTextAreaElement | null;
  if (el) { el.style.height = "auto"; el.style.height = Math.min(el.scrollHeight, 180) + "px"; }
}
</script>

<template>
  <div class="shrink-0 pt-3 pb-8" style="background: var(--bg-root)">
    <div
      class="flex items-center max-w-3xl mx-auto rounded-xl transition-colors duration-150"
      :style="{
        background: 'var(--bg-elevated)',
        border: isDragOver ? '2px dashed var(--accent)' : props.autoMode ? '1px solid #0ea5e9' : focused ? '1px solid var(--accent)' : '1px solid var(--border-default)',
        padding: '0 6px 0 0'
      }"
      @dragover="onDragOver"
      @dragleave="onDragLeave"
      @drop="onDrop"
    >
      <textarea
        v-model="input"
        :disabled="disabled"
        @keydown="onKeydown"
        @paste="onPaste"
        @input="autoResize"
        @focus="focused = true"
        @blur="focused = false"
        placeholder="Send a message…"
        rows="1"
        class="chat-textarea flex-1 resize-none bg-transparent text-sm leading-relaxed py-3 pl-4 pr-2 disabled:opacity-30"
        :style="{
          color: 'var(--text-primary)',
          caretColor: 'var(--accent)',
          border: 'none',
          outline: 'none',
          boxShadow: 'none',
          overflow: 'hidden',
          minHeight: '22px'
        }"
      ></textarea>
      <!-- Stop button (visible when processing) -->
      <button
        v-if="disabled"
        @click="emit('stop')"
        class="shrink-0 w-8 h-8 flex items-center justify-center rounded-lg transition-all duration-150"
        style="background: var(--coral); color: white; border: none"
        title="Stop"
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><rect x="4" y="4" width="16" height="16" rx="2"/></svg>
      </button>

      <!-- Send button (visible when idle) -->
      <button
        v-else
        :disabled="!input.trim()"
        @click="send"
        class="shrink-0 w-8 h-8 flex items-center justify-center rounded-lg transition-all duration-150"
        :style="{
          background: input.trim() ? 'var(--accent)' : 'transparent',
          color: input.trim() ? 'var(--bg-root)' : 'var(--text-muted)',
          border: 'none'
        }"
      >
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <line x1="22" y1="2" x2="11" y2="13" />
          <polygon points="22 2 15 22 11 13 2 9 22 2" />
        </svg>
      </button>
    </div>
  </div>
</template>
