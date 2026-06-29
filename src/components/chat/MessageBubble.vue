<script setup lang="ts">
import type { Message } from "@/stores/chat";
import { ref, computed, nextTick, onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";

import { isImageFile, useFilePreview } from "@/composables/useFilePreview";
import { formatNum } from "@/lib/utils";
import MarkdownRenderer from "../shared/MarkdownRenderer.vue";

const { t } = useI18n();
const { getThumbnail, thumbnails } = useFilePreview();

function toolLabel(name: string): string {
  const key = `tools.${name}`;
  const translated = t(key);
  return translated !== key ? translated : name;
}


// 每秒 tick，驱动流式期间的工具执行计时实时更新
const now = ref(Date.now());
let nowTimer: ReturnType<typeof setInterval> | null = null;
onMounted(() => { nowTimer = setInterval(() => now.value = Date.now(), 1000); });
onUnmounted(() => { if (nowTimer) clearInterval(nowTimer); });

const props = defineProps<{ message: Message }>();
const emit = defineEmits<{
  edit: [id: string, content: string];
  resend: [id: string, content: string];
  editSave: [id: string, newContent: string];
  previewFile: [file: { name: string; path: string }];
}>();
const copied = ref(false);

// ── Inline editing state ──
const isEditing = ref(false);
const editText = ref("");
const editTextarea = ref<HTMLTextAreaElement | null>(null);

function startEdit() {
  editText.value = props.message.content;
  isEditing.value = true;
  nextTick(() => {
    editTextarea.value?.focus();
    editTextarea.value?.select();
  });
}

function cancelEdit() {
  isEditing.value = false;
  editText.value = "";
}

function saveAndResend() {
  const text = editText.value.trim();
  if (!text) return;
  isEditing.value = false;
  emit("editSave", props.message.id, text);
}

function onEditKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    saveAndResend();
  }
  if (e.key === "Escape") {
    e.preventDefault();
    cancelEdit();
  }
}

// ── 思考总耗时：各段 tool_use 前思考时间求和（精确，无需实时计时器）──
function totalThinkingMs(): number {
  return props.message.toolUses.reduce((sum, tu) => sum + (tu.thinkingDurationMs || 0), 0);
}
// ── Elapsed display（思考时间，不含工具执行）──
const elapsedSeconds = computed(() => {
  // 各段 tool_use 前思考时间求和（精确，流式和结束后统一）
  const thinking = totalThinkingMs();
  if (thinking > 0) {
    return (thinking / 1000).toFixed(1);
  }
  // 纯文本回复（无工具调用）fallback 到 CC 报告的耗时
  if (props.message.durationMs) {
    return (props.message.durationMs / 1000).toFixed(1);
  }
  return null;
});

const elapsedLabel = computed(() => {
  const s = elapsedSeconds.value;
  return s ? `${s}s` : "";
});

// ── Token display ──
const tokenLabel = computed(() => {
  const parts: string[] = [];
  if (props.message.inputTokens) parts.push(`↑${formatNum(props.message.inputTokens)}`);
  if (props.message.outputTokens) parts.push(`↓${formatNum(props.message.outputTokens)}`);
  return parts.join(" ");
});

async function copyContent() {
  await navigator.clipboard.writeText(props.message.content);
  copied.value = true;
  setTimeout(() => copied.value = false, 1500);
}

// ── Tool input display helpers ──
function summarizeInput(input: Record<string, unknown>): string {
  const entries = Object.entries(input);
  if (entries.length === 0) return "(no args)";
  const previews = entries.slice(0, 3).map(([k, v]) => {
    if (typeof v === "string") return `${k}=${v.length > 40 ? v.slice(0, 40) + "…" : v}`;
    if (typeof v === "number" || typeof v === "boolean") return `${k}=${v}`;
    return `${k}={…}`;
  });
  let s = previews.join(", ");
  if (entries.length > 3) s += ` +${entries.length - 3} more`;
  return s;
}

function formatJSON(obj: unknown): string {
  if (typeof obj === "string") return obj;
  return JSON.stringify(obj, null, 2);
}
</script>

<template>
  <div :class="['flex gap-3', message.role === 'user' ? 'flex-row-reverse' : '']" :data-role="message.role">
    <!-- Avatar -->
    <div
      class="w-7 h-7 shrink-0 rounded-md flex items-center justify-center text-[11px] font-semibold"
      :style="{
        background: message.role === 'user'
          ? 'linear-gradient(135deg, #3b82f6, #6366f1)'
          : 'linear-gradient(135deg, #06d6a0, #0891b2)',
        color: 'white'
      }"
    >
      {{ message.role === 'user' ? 'U' : 'C' }}
    </div>

    <!-- Body -->
    <div :class="['flex-1 min-w-0 space-y-2', message.role === 'user' ? 'flex flex-col items-end' : '']">
      <!-- Name + actions -->
      <div class="flex items-center gap-1.5 px-0.5">
        <span class="text-[11px] font-medium" style="color:var(--text-muted)">
          {{ message.role === 'user' ? 'You' : 'Claude' }}
        </span>
        <!-- Copy -->
        <button
          v-if="message.content && !message.isStreaming"
          @click="copyContent"
          class="w-4 h-4 flex items-center justify-center rounded transition-colors hover:bg-[var(--bg-hover)]"
          :style="{ color: copied ? 'var(--accent)' : 'var(--text-muted)' }"
          :title="copied ? $t('chat.copied') : $t('chat.copy')"
        >
          <svg v-if="!copied" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
          <svg v-else width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>
        </button>
        <!-- Edit (user messages only) -->
        <button
          v-if="message.role === 'user' && !message.isStreaming && !isEditing"
          @click="startEdit"
          class="w-4 h-4 flex items-center justify-center rounded transition-colors hover:bg-[var(--bg-hover)]"
          style="color: var(--text-muted)"
          :title="$t('chat.edit')"
        >
          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 3a2.85 2.85 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z"/></svg>
        </button>
        <!-- Resend (user messages only) -->
        <button
          v-if="message.role === 'user' && !message.isStreaming && !isEditing"
          @click="emit('resend', message.id, message.content)"
          class="w-4 h-4 flex items-center justify-center rounded transition-colors hover:bg-[var(--bg-hover)]"
          style="color: var(--text-muted)"
          :title="$t('chat.resend')"
        >
          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg>
        </button>
      </div>

      <!-- Tool cards (BEFORE content — tools are called first) -->
      <div v-if="message.toolUses.length > 0" class="space-y-1.5">
        <details
          v-for="tu in message.toolUses"
          :key="tu.id"
          class="rounded-md text-xs font-mono group"
          style="background:var(--bg-root); border:1px solid var(--border-dim)"
        >
          <summary class="px-2.5 py-1.5 cursor-pointer select-none transition-colors hover:bg-[var(--bg-hover)]" style="color:var(--text-secondary)">
            <span class="font-medium" style="color:var(--violet)">{{ toolLabel(tu.name) }}</span>
            <span v-if="tu.thinkingDurationMs" class="ml-1" style="color:var(--text-muted)">🧠{{ (tu.thinkingDurationMs / 1000).toFixed(1) }}s</span>
            <span v-if="tu.executionDurationMs" class="ml-1" :style="{ color: tu.executionDurationMs > 5000 ? 'var(--coral)' : 'var(--text-muted)' }">⚡{{ (tu.executionDurationMs / 1000).toFixed(1) }}s</span>
            <!-- 流式期间最后一个工具无 executionDurationMs → 显示实时执行计时 -->
            <span v-if="!tu.executionDurationMs && tu.startedAt && message.isStreaming && tu === message.toolUses[message.toolUses.length-1]" class="ml-1 animate-pulse" style="color:var(--accent)">⚡{{ ((now - tu.startedAt) / 1000).toFixed(1) }}s</span>
            <span class="ml-2" style="color:var(--text-muted)">
              {{ summarizeInput(tu.input) }}
            </span>
          </summary>
          <pre class="px-3 py-2 m-0 text-[11px] leading-relaxed overflow-x-auto whitespace-pre" style="border-top:1px solid var(--border-dim); color:var(--text-muted); max-height:240px">{{ formatJSON(tu.input) }}</pre>
        </details>
      </div>

      <!-- Content (answer text AFTER tools) -->
      <!-- Edit mode: inline textarea for user messages -->
      <div v-if="isEditing" class="w-full">
        <textarea
          ref="editTextarea"
          v-model="editText"
          @keydown="onEditKeydown"
          rows="3"
          class="w-full resize-none bg-transparent text-sm leading-relaxed p-3 rounded-xl rounded-br-md border outline-none"
          :style="{
            color: 'var(--text-bright)',
            background: 'linear-gradient(135deg, rgba(59,130,246,0.15), rgba(99,102,241,0.10))',
            borderColor: 'var(--accent)',
          }"
        ></textarea>
        <div class="flex items-center gap-2 mt-2 justify-end">
          <button
            @click="cancelEdit"
            class="px-2.5 py-1 rounded-md text-xs transition-colors hover:bg-[var(--bg-hover)]"
            style="color: var(--text-muted)"
          >{{ $t('chat.cancel') }}</button>
          <button
            @click="saveAndResend"
            :disabled="!editText.trim()"
            class="px-3 py-1 rounded-md text-xs font-medium transition-colors flex items-center gap-1.5"
            :style="{
              background: editText.trim() ? 'var(--accent)' : 'var(--bg-elevated)',
              color: editText.trim() ? 'var(--bg-root)' : 'var(--text-muted)',
            }"
          >
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="22" y1="2" x2="11" y2="13"/><polygon points="22 2 15 22 11 13 2 9 22 2"/></svg>
            {{ $t('chat.saveResend') }}
          </button>
        </div>
      </div>

      <div
        v-else-if="message.content"
        :class="[
          'prose text-sm leading-relaxed',
          message.role === 'user'
            ? 'px-3.5 py-2.5 rounded-2xl rounded-tr-md user-text'
            : 'rounded-lg'
        ]"
        :style="message.role === 'user'
          ? { background: 'linear-gradient(135deg, rgba(59,130,246,0.15), rgba(99,102,241,0.10))', border: '1px solid rgba(59,130,246,0.15)', color: 'var(--text-bright)' }
          : {}
        "
      >
        <MarkdownRenderer v-if="message.content" :content="message.content" />
        <span v-if="message.isStreaming" class="stream-cursor"></span>

        <!-- Attachments in user message -->
        <div
          v-if="message.role === 'user' && message.attachments?.length"
          class="flex flex-wrap gap-1 mt-2 pt-2"
          :style="{ borderTop: '1px solid rgba(59,130,246,0.15)' }"
        >
          <div
            v-for="att in message.attachments"
            :key="att.path"
            class="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] cursor-pointer transition-colors hover:brightness-110 shrink-0"
            style="background: rgba(59,130,246,0.12); color: var(--text-secondary)"
            @click="emit('previewFile', att)"
          >
            <img
              v-if="isImageFile(att.name)"
              :src="thumbnails[att.path] || ''"
              @vue:mounted="getThumbnail(att.path, att.name)"
              class="w-3.5 h-3.5 rounded object-cover shrink-0"
              v-show="thumbnails[att.path]"
            />
            <svg v-else width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2" stroke-linecap="round" class="shrink-0"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/></svg>
            <span class="truncate max-w-[120px]">{{ att.name }}</span>
          </div>
        </div>
      </div>

      <!-- Thinking (collapsed by default) — 思考摘要行已包含时间/token/cost，无需额外统计条 -->
      <details v-if="message.thinking" class="group" :open="message.isStreaming && !message.content">
        <summary class="text-xs cursor-pointer select-none px-2.5 py-1.5 rounded-md transition-colors hover:bg-[var(--bg-elevated)]" style="color:var(--amber-dim)">
          <span class="inline-flex items-center gap-1.5">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/></svg>
            {{ message.isStreaming ? $t('chat.thinking') : $t('chat.thinkingDone') }}
            <span v-if="message.wasStopped" class="text-[10px] ml-0.5" :style="{ color: 'var(--coral)' }">· {{ $t('chat.stopped') }}</span>
            <span v-if="elapsedSeconds && !message.wasStopped" class="text-[10px] tabular-nums" style="font-variant-numeric: tabular-nums">· {{ elapsedLabel }}</span>
            <span v-if="!message.isStreaming && tokenLabel" class="text-[10px] tabular-nums opacity-70">· {{ tokenLabel }}</span>
            <span v-if="!message.isStreaming && message.costUSD !== undefined" class="text-[10px] tabular-nums opacity-70">· ${{ message.costUSD.toFixed(3) }}</span>
            <span v-if="message.isStreaming" class="w-1.5 h-1.5 rounded-full animate-pulse ml-1" style="background:var(--accent)"></span>
          </span>
        </summary>
        <div class="mt-1.5 px-2.5 py-2 rounded-md text-xs leading-relaxed whitespace-pre-wrap" style="background:var(--amber-glow); color:var(--text-secondary); border-left:2px solid var(--amber)">
          {{ message.thinking }}
        </div>
      </details>

      <!-- Inline stats when no thinking section exists (streaming or done without thinking) -->
      <div
        v-if="!message.thinking && (message.isStreaming || message.durationMs || message.inputTokens)"
        class="flex items-center gap-2 text-[11px] px-0.5"
        style="color: var(--text-muted)"
      >
        <span v-if="elapsedLabel && !message.wasStopped" class="font-mono tabular-nums">⏱ {{ elapsedLabel }}</span>
        <span v-if="message.wasStopped" class="font-mono tabular-nums" :style="{ color: 'var(--coral)' }">⏹ {{ $t('chat.stopped') }}</span>
        <span v-if="message.isStreaming" class="w-1.5 h-1.5 rounded-full animate-pulse" style="background:var(--accent)"></span>
        <template v-if="!message.isStreaming">
          <span v-if="tokenLabel">⏐ {{ tokenLabel }}</span>
          <span v-if="message.costUSD !== undefined">${{ message.costUSD.toFixed(4) }}</span>
        </template>
      </div>
    </div>
  </div>
</template>
