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

interface Row { labelZh: string; labelEn: string; tokens: number }
const rows = computed<Row[]>(() => [
  { labelZh: "系统提示词",   labelEn: "System Prompt",   tokens: systemPrompt },
  { labelZh: "系统工具",     labelEn: "System Tools",    tokens: systemTools },
  { labelZh: "MCP 工具",     labelEn: "MCP Tools",       tokens: mcpTools },
  { labelZh: "自定义 Agent", labelEn: "Custom Agents",   tokens: customAgents },
  { labelZh: "记忆文件",     labelEn: "Memory Files",    tokens: memoryFiles },
  { labelZh: "技能",         labelEn: "Skills",          tokens: skills },
  { labelZh: "消息",         labelEn: "Messages",        tokens: msgTokens.value },
]);
</script>

<template>
  <ModalShell :open="open" @close="emit('close')">
    <template #header>
      <span class="text-sm font-semibold" :style="{ color: 'var(--text-bright)' }">
        上下文用量 <span class="italic text-[11px] font-normal ml-1.5" :style="{ color: 'var(--text-muted)' }">Context Usage</span>
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
    <div class="text-[10px] font-semibold uppercase tracking-wider mb-1.5" :style="{ color: 'var(--text-muted)' }">Category 分类</div>
    <table class="w-full text-xs mb-3">
      <thead>
        <tr :style="{ color: 'var(--text-muted)' }">
          <th class="text-left font-normal pb-1 w-28">Category</th>
          <th class="text-right font-normal pb-1 w-16">Tokens</th>
          <th class="text-right font-normal pb-1 w-12">%</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="r in rows" :key="r.labelEn" class="border-b" :style="{ borderColor: 'var(--border-dim)' }">
          <td class="py-1.5" :style="{ color: 'var(--text-secondary)' }">
            {{ r.labelZh }}
            <span class="italic text-[11px] ml-1.5" :style="{ color: 'var(--text-muted)' }">{{ r.labelEn }}</span>
          </td>
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
        空闲空间 <span class="italic text-[11px] font-normal ml-1.5" :style="{ color: 'var(--text-muted)' }">Free Space</span>
      </span>
      <span class="text-xs font-mono tabular-nums" :style="{ color: 'var(--accent)' }">
        {{ formatNum(freeSpace) }} ({{ pct(freeSpace) }}%)
      </span>
    </div>

    <p class="text-[10px] leading-relaxed pb-1" :style="{ color: 'var(--text-muted)' }">
      System / Tools / Agents / Memory / Skills 为静态估算值。消息 tokens 来自实际 API 返回。精确值请运行 <code :style="{ color: 'var(--accent)', background: 'var(--bg-hover)', padding: '1px 4px', borderRadius: '2px' }">/context</code>。
    </p>
  </ModalShell>
</template>
