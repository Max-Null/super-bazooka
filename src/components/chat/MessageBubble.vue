<script setup lang="ts">
import type { Message } from "@/stores/chat";
import { ref, computed, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import MarkdownRenderer from "../shared/MarkdownRenderer.vue";

const { t } = useI18n();
const props = defineProps<{ message: Message }>();
const copied = ref(false);

// ── Live timer during streaming ──
const liveElapsedMs = ref(0);
let timerInterval: ReturnType<typeof setInterval> | null = null;

function startTimer() {
  stopTimer();
  liveElapsedMs.value = 0;
  timerInterval = setInterval(() => {
    if (props.message.isStreaming) {
      liveElapsedMs.value = Date.now() - props.message.timestamp;
    } else {
      stopTimer();
    }
  }, 100);
}

function stopTimer() {
  if (timerInterval) {
    clearInterval(timerInterval);
    timerInterval = null;
  }
}

// Restart timer on mount if message is still streaming
if (props.message.isStreaming) {
  startTimer();
}

// Watch for streaming state changes
import { watch } from "vue";
watch(() => props.message.isStreaming, (streaming) => {
  if (streaming) startTimer();
  else stopTimer();
});

onUnmounted(() => stopTimer());

// ── Elapsed display ──
const elapsedSeconds = computed(() => {
  // Final duration from the result event (most accurate)
  if (props.message.durationMs) {
    return (props.message.durationMs / 1000).toFixed(1);
  }
  // Live timer during streaming
  if (props.message.isStreaming && liveElapsedMs.value > 0) {
    return (liveElapsedMs.value / 1000).toFixed(1);
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

function formatNum(n: number): string {
  if (n >= 1000) return (n / 1000).toFixed(1) + "k";
  return String(n);
}

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
      <!-- Name + copy -->
      <div class="flex items-center gap-1.5 px-0.5">
        <span class="text-[11px] font-medium" style="color:var(--text-muted)">
          {{ message.role === 'user' ? 'You' : 'Claude' }}
        </span>
        <button
          v-if="message.content && !message.isStreaming"
          @click="copyContent"
          class="w-4 h-4 flex items-center justify-center rounded transition-colors hover:bg-[var(--bg-hover)]"
          :style="{ color: copied ? 'var(--accent)' : 'var(--text-muted)' }"
          :title="copied ? 'Copied' : 'Copy'"
        >
          <svg v-if="!copied" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
          <svg v-else width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>
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
            <span class="font-medium" style="color:var(--violet)">{{ tu.name }}</span>
            <span class="ml-2" style="color:var(--text-muted)">
              {{ summarizeInput(tu.input) }}
            </span>
          </summary>
          <pre class="px-3 py-2 m-0 text-[11px] leading-relaxed overflow-x-auto whitespace-pre" style="border-top:1px solid var(--border-dim); color:var(--text-muted); max-height:240px">{{ formatJSON(tu.input) }}</pre>
        </details>
      </div>

      <!-- Content (answer text AFTER tools) -->
      <div
        v-if="message.content"
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
      </div>

      <!-- Thinking (collapsed by default) — timer + tokens integrated in summary -->
      <details v-if="message.thinking" class="group" :open="message.isStreaming && !message.content">
        <summary class="text-xs cursor-pointer select-none px-2.5 py-1.5 rounded-md transition-colors hover:bg-[var(--bg-elevated)]" style="color:var(--amber-dim)">
          <span class="inline-flex items-center gap-1.5">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/></svg>
            {{ message.isStreaming ? t('chat.thinking') : t('chat.thinkingDone') }}
            <span v-if="elapsedSeconds" class="text-[10px] tabular-nums" style="font-variant-numeric: tabular-nums">· {{ elapsedLabel }}</span>
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
        <span v-if="elapsedLabel" class="font-mono tabular-nums">⏱ {{ elapsedLabel }}</span>
        <span v-if="message.isStreaming" class="w-1.5 h-1.5 rounded-full animate-pulse" style="background:var(--accent)"></span>
        <template v-if="!message.isStreaming">
          <span v-if="tokenLabel">⏐ {{ tokenLabel }}</span>
          <span v-if="message.costUSD !== undefined">${{ message.costUSD.toFixed(4) }}</span>
        </template>
      </div>
    </div>
  </div>
</template>
