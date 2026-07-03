<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";

const props = defineProps<{
  startTimestamp?: number;
  elapsedMs?: number;
  /** 当前正在执行的工具名（如 Bash、Write），有则显示 "⚡ 执行 xxx…" */
  toolName?: string;
}>();

const liveMs = ref(0);
let timer: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
  if (props.startTimestamp) {
    liveMs.value = Date.now() - props.startTimestamp;
    timer = setInterval(() => {
      liveMs.value = Date.now() - props.startTimestamp!;
    }, 100);
  }
});

onUnmounted(() => {
  if (timer) clearInterval(timer);
});

const elapsed = computed(() => {
  const ms = props.elapsedMs ?? liveMs.value;
  if (ms < 1000) return "";
  return (ms / 1000).toFixed(1) + "s";
});
</script>

<template>
  <div
    class="flex items-center gap-2 px-3 py-1.5 rounded-md select-none"
    style="background: var(--accent-glow); border: 1px solid var(--accent-dim)"
  >
    <!-- 动画点（直接放 flex 容器内，去掉多余嵌套） -->
    <span class="w-2 h-2 rounded-full animate-pulse shrink-0" style="background:var(--accent)" />
    <span class="w-2 h-2 rounded-full animate-pulse shrink-0" style="background:var(--accent); animation-delay:0.15s" />
    <span class="w-2 h-2 rounded-full animate-pulse shrink-0" style="background:var(--accent); animation-delay:0.3s" />

    <!-- 阶段文字 -->
    <span v-if="toolName" class="text-xs font-medium shrink-0" style="color: var(--accent)">
      ⚡ {{ toolName }}
    </span>
    <span v-else class="text-xs font-medium shrink-0" style="color: var(--accent)">
      {{ $t('chat.thinking') }}
    </span>

    <!-- 耗时 -->
    <span v-if="elapsed" class="text-[11px] font-mono tabular-nums opacity-60 shrink-0 leading-none" style="color: var(--accent)">
      {{ elapsed }}
    </span>
  </div>
</template>
