<script setup lang="ts">
import { computed } from "vue";
import { useChatStore } from "@/stores/chat";
import { useSettingsStore } from "@/stores/settings";
import { formatNum } from "@/lib/utils";

const chat = useChatStore();
const settings = useSettingsStore();

// 模型上下文窗口大小（tokens）
// DeepSeek v4 系列支持 1M 上下文，模型名中的 [1M] 后缀也明确标注
// Claude 模型为 200k
const contextLimits: Record<string, number> = {
  "deepseek-v4-pro": 1_000_000,
  "deepseek-v4": 1_000_000,
  "deepseek-v4-flash": 128_000,
  "claude-sonnet-4-6": 200_000,
  "claude-opus-4-8": 200_000,
  "claude-haiku-4-5": 200_000,
};

const limit = computed(() => {
  // 模型名中的 [1M] / [1m] 后缀直接指示 1M 上下文
  const modelLower = settings.model.toLowerCase();
  if (modelLower.includes("[1m]") || modelLower.includes("[1M]")) {
    return 1_000_000;
  }
  for (const [key, val] of Object.entries(contextLimits)) {
    if (modelLower.includes(key)) return val;
  }
  return 128_000; // 默认保守值
});

const usedTokens = computed(() => {
  let total = 0;
  for (const msg of chat.messages) {
    total += msg.inputTokens || 0;
    total += msg.outputTokens || 0;
  }
  return total;
});

const pct = computed(() => Math.min(100, Math.round((usedTokens.value / limit.value) * 100)));

const statusColor = computed(() => {
  if (pct.value >= 90) return "var(--coral)";
  if (pct.value >= 75) return "var(--amber)";
  return "var(--accent)";
});

const tooltip = computed(() =>
  `${formatNum(usedTokens.value)} / ${formatNum(limit.value)} tokens (${pct.value}%)`
);

// Progress bar segments
const barSegments = 10;
</script>

<template>
  <div
    v-if="chat.messages.length > 0"
    class="flex items-center gap-1.5 text-[10px] shrink-0 cursor-pointer"
    :style="{ color: 'var(--text-muted)' }"
    :title="tooltip"
  >
    <!-- Mini progress bar -->
    <div class="flex gap-px items-end" style="height: 10px">
      <div
        v-for="i in barSegments"
        :key="i"
        class="w-1 rounded-sm transition-colors"
        :style="{
          height: (i / barSegments) * 10 + 'px',
          background: i <= Math.round(pct / (100 / barSegments)) ? statusColor : 'var(--border-dim)',
          opacity: i <= Math.round(pct / (100 / barSegments)) ? 1 : 0.4,
        }"
      ></div>
    </div>
    <!-- Label -->
    <span class="tabular-nums" :style="{ color: statusColor }">
      {{ pct }}%
    </span>
  </div>
</template>
