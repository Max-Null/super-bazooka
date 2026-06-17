<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";



const props = defineProps<{
  startTimestamp?: number;
  /** External elapsed override (e.g. from parent with precise ms) */
  elapsedMs?: number;
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

const display = computed(() => {
  const ms = props.elapsedMs ?? liveMs.value;
  if (ms < 1000) return "";
  return (ms / 1000).toFixed(1) + "s";
});
</script>

<template>
  <div class="flex items-center gap-2 text-xs px-0.5" style="color: var(--text-muted)">
    <span class="inline-flex items-center gap-1">
      <span class="w-1.5 h-1.5 rounded-full animate-pulse" style="background:var(--accent)"></span>
      <span class="w-1.5 h-1.5 rounded-full animate-pulse" style="background:var(--accent); animation-delay:0.15s"></span>
      <span class="w-1.5 h-1.5 rounded-full animate-pulse" style="background:var(--accent); animation-delay:0.3s"></span>
    </span>
    <span>{{ $t('chat.thinking') }}</span>
    <span v-if="display" class="font-mono tabular-nums opacity-60">· {{ display }}</span>
  </div>
</template>
