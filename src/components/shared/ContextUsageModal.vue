<script setup lang="ts">
import { computed } from "vue";
import { useChatStore } from "@/stores/chat";
import { useSettingsStore } from "@/stores/settings";
import { formatNum } from "@/lib/utils";
import ModalShell from "./ModalShell.vue";

defineProps<{ open: boolean }>();
const emit = defineEmits<{ close: [] }>();

const chat = useChatStore();
const settings = useSettingsStore();

const limit = computed(() => {
  const m = settings.model.toLowerCase();
  if (m.includes("[1m]") || m.includes("deepseek-v4-pro") || m.includes("deepseek-v4")) return 1_000_000;
  if (m.includes("claude")) return 200_000;
  return 128_000;
});

const msgTokens = computed(() => {
  let total = 0;
  for (const m of chat.messages) total += (m.inputTokens || 0) + (m.outputTokens || 0);
  return total;
});

const systemPrompt = 1500;
const systemTools = 10500;
const mcpTools = 200;
const customAgents = 2600;
const memoryFiles = 1200;
const skills = 6100;

const totalUsed = computed(() =>
  msgTokens.value + systemPrompt + systemTools + mcpTools + customAgents + memoryFiles + skills
);
const freeSpace = computed(() => Math.max(0, limit.value - totalUsed.value));
const pct = (val: number) => ((val / limit.value) * 100).toFixed(1);
const pctNum = computed(() => (totalUsed.value / limit.value) * 100);

interface Row { key: string; tokens: number }
const rows = computed<Row[]>(() => [
  { key: "systemPrompt", tokens: systemPrompt },
  { key: "systemTools", tokens: systemTools },
  { key: "mcpTools", tokens: mcpTools },
  { key: "customAgents", tokens: customAgents },
  { key: "memoryFiles", tokens: memoryFiles },
  { key: "skills", tokens: skills },
  { key: "messages", tokens: msgTokens.value },
]);
</script>

<template>
  <ModalShell :open="open" @close="emit('close')">
    <template #header>
      <span class="text-sm font-semibold" :style="{ color: 'var(--text-bright)' }">
        {{ $t('contextUsage.title') }}
      </span>
      <span class="text-[11px] font-mono ml-2" :style="{ color: 'var(--text-muted)' }">{{ settings.model }}</span>
    </template>

    <!-- 用量摘要 -->
    <div class="text-xl font-bold font-mono tabular-nums mb-1" :style="{ color: 'var(--text-bright)' }">
      {{ formatNum(totalUsed) }} / {{ formatNum(limit) }} tokens
      <span class="text-sm ml-1.5" :style="{ color: pctNum > 90 ? 'var(--coral)' : pctNum > 75 ? 'var(--amber)' : 'var(--text-muted)' }">
        ({{ pctNum.toFixed(1) }}%)
      </span>
    </div>
    <div class="h-1.5 rounded-full mb-4" :style="{ background: 'var(--bg-elevated)' }">
      <div class="h-full rounded-full transition-all" :style="{ width: Math.min(100, pctNum).toFixed(1) + '%', background: pctNum > 90 ? 'var(--coral)' : pctNum > 75 ? 'var(--amber)' : 'var(--accent)' }"></div>
    </div>

    <!-- 分类表格 -->
    <div class="text-[10px] font-semibold uppercase tracking-wider mb-1.5" :style="{ color: 'var(--text-muted)' }">{{ $t('contextUsage.category') }}</div>
    <table class="w-full text-xs mb-3">
      <thead>
        <tr :style="{ color: 'var(--text-muted)' }">
          <th class="text-left font-normal pb-1 w-28">{{ $t('contextUsage.category') }}</th>
          <th class="text-right font-normal pb-1 w-16">{{ $t('contextUsage.tokens') }}</th>
          <th class="text-right font-normal pb-1 w-12">%</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="r in rows" :key="r.key" class="border-b" :style="{ borderColor: 'var(--border-dim)' }">
          <td class="py-1.5" :style="{ color: 'var(--text-secondary)' }">
            {{ $t('contextUsage.' + r.key) }}          </td>
          <td class="py-1.5 text-right tabular-nums font-mono" :style="{ color: 'var(--text-primary)' }">{{ formatNum(r.tokens) }}</td>
          <td class="py-1.5 text-right tabular-nums font-mono" :style="{ color: r.tokens / limit > 0.15 ? 'var(--amber)' : 'var(--text-muted)' }">
            {{ pct(r.tokens) }}%
          </td>
        </tr>
      </tbody>
    </table>

    <!-- 空闲空间 -->
    <div class="flex items-center justify-between py-2 px-3 rounded-lg mb-2" :style="{ background: 'var(--bg-elevated)' }">
      <span class="text-xs font-medium" :style="{ color: 'var(--text-bright)' }">
        {{ $t('contextUsage.freeSpace') }}
      </span>
      <span class="text-xs font-mono tabular-nums" :style="{ color: 'var(--accent)' }">
        {{ formatNum(freeSpace) }} ({{ pct(freeSpace) }}%)
      </span>
    </div>

    <p class="text-[10px] leading-relaxed pb-1" :style="{ color: 'var(--text-muted)' }">
      {{ $t('contextUsage.note') }}
    </p>
  </ModalShell>
</template>
