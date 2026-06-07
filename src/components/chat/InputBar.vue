<script setup lang="ts">
import { ref, nextTick } from "vue";

const props = defineProps<{ disabled: boolean }>();
const emit = defineEmits<{ send: [text: string] }>();

const input = ref("");
const focused = ref(false);

function send() {
  const text = input.value.trim();
  if (!text || props.disabled) return;
  emit("send", text);
  input.value = "";
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); send(); }
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
        border: focused ? '1px solid var(--accent)' : '1px solid var(--border-default)',
        padding: '0 6px 0 0'
      }"
    >
      <textarea
        v-model="input"
        :disabled="disabled"
        @keydown="onKeydown"
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
      <button
        :disabled="disabled || !input.trim()"
        @click="send"
        class="shrink-0 w-8 h-8 flex items-center justify-center rounded-lg transition-all duration-150"
        :style="{
          background: input.trim() && !disabled ? 'var(--accent)' : 'transparent',
          color: input.trim() && !disabled ? 'var(--bg-root)' : 'var(--text-muted)',
          opacity: disabled ? 0.3 : 1,
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
