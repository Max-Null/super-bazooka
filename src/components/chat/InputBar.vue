<script setup lang="ts">
import { ref, nextTick } from "vue";
import { optimizePrompt } from "@/lib/tauri-bridge";
import { useSettingsStore } from "@/stores/settings";

const props = defineProps<{ disabled: boolean; autoMode?: boolean; apiKey?: string; baseUrl?: string }>();
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
  if (!text) return;
  // 中途发送由 ChatPanel.handleSend 处理，不在此处拦
  emit("send", text);
  input.value = "";
  // JS 赋值不触发 @input，手动恢复初始高度
  autoResize();
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

// ── 提示词优化 ──
const settings = useSettingsStore();
const optimizing = ref(false);
const optimizeError = ref("");

async function handleOptimize() {
  const text = input.value.trim();
  if (!text || !props.apiKey || !props.baseUrl) return;
  if (!settings.optimizeApiUrl) {
    optimizeError.value = "请先在设置中配置「聊天 API 地址」";
    setTimeout(() => { optimizeError.value = ""; }, 5000);
    return;
  }
  optimizing.value = true;
  optimizeError.value = "";
  try {
    const result = await optimizePrompt(props.apiKey, props.baseUrl, text, settings.optimizeApiUrl || undefined);
    input.value = result;
    await nextTick();
    autoResize();
  } catch (e) {
    optimizeError.value = typeof e === "string" ? e : (e as Error).message || String(e);
    setTimeout(() => { optimizeError.value = ""; }, 5000);
  } finally {
    optimizing.value = false;
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
        @keydown="onKeydown"
        @paste="onPaste"
        @input="autoResize"
        @focus="focused = true"
        @blur="focused = false"
        :placeholder="$t('chat.placeholder')"
        rows="1"
        class="chat-textarea flex-1 resize-none bg-transparent text-sm leading-relaxed py-3 pl-4 pr-2 disabled:opacity-30"
        :style="{
          color: 'var(--text-primary)',
          caretColor: 'var(--accent)',
          border: 'none',
          outline: 'none',
          boxShadow: 'none',
          maxHeight: '180px',
          overflowY: 'auto',
          minHeight: '22px'
        }"
      ></textarea>
      <!-- ✨ 提示词优化 -->
      <button
        @click="handleOptimize"
        :disabled="!input.trim() || optimizing"
        class="shrink-0 w-8 h-8 flex items-center justify-center rounded-lg transition-all duration-150"
        :style="{
          background: optimizeError ? 'var(--coral-glow)' : 'transparent',
          color: optimizeError ? 'var(--coral)' : input.trim() ? 'var(--text-secondary)' : 'var(--text-muted)',
          border: 'none',
        }"
        :title="optimizeError || $t('toolbar.optimizeTitle')"
      >
        <!-- loading 旋转动画 -->
        <svg v-if="optimizing" class="animate-spin" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <path d="M21 12a9 9 0 1 1-6.219-8.56"/>
        </svg>
        <!-- 双星图标 -->
        <svg v-else width="15" height="15" viewBox="1 1 22 22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M9.813 15.904L9 18.75l-.813-2.846a4.5 4.5 0 0 0-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 0 0 3.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 0 0 3.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 0 0-3.09 3.09zM18.259 8.715L18 9.75l-.259-1.035a3.375 3.375 0 0 0-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 0 0 2.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 0 0 2.455 2.456L21.75 6l-1.036.259a3.375 3.375 0 0 0-2.455 2.456z"/>
        </svg>
      </button>
      <!-- Stop button (visible when processing) -->
      <button
        v-if="disabled"
        @click="emit('stop')"
        class="shrink-0 w-8 h-8 flex items-center justify-center rounded-lg transition-all duration-150"
        style="background: var(--coral); color: white; border: none"
        :title="$t('chat.stop')"
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><rect x="4" y="4" width="16" height="16" rx="2"/></svg>
      </button>

      <!-- Send button (visible when idle) -->
      <button
        v-else
        :disabled="!input.trim() || optimizing"
        @click="send"
        class="shrink-0 w-8 h-8 flex items-center justify-center rounded-lg transition-all duration-150"
        :style="{
          background: (input.trim() && !optimizing) ? 'var(--accent)' : 'transparent',
          color: (input.trim() && !optimizing) ? 'var(--bg-root)' : 'var(--text-muted)',
          border: 'none'
        }"
      >
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" style="transform: translate(-1px, 1px)">
          <line x1="22" y1="2" x2="11" y2="13" />
          <polygon points="22 2 15 22 11 13 2 9 22 2" />
        </svg>
      </button>
    </div>
    <!-- 优化错误提示 -->
    <div v-if="optimizeError" class="max-w-3xl mx-auto mt-1 text-[11px] px-1" style="color: var(--coral)">
      ✕ {{ optimizeError }}
    </div>
  </div>
</template>
