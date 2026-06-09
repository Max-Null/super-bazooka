<script setup lang="ts">
import { ref, watch, nextTick } from "vue";
import mermaid from "mermaid";
import DOMPurify from "dompurify";

const props = defineProps<{ code: string }>();
const svg = ref("");
const error = ref("");

mermaid.initialize({
  startOnLoad: false,
  securityLevel: "strict",
  theme: "dark",
  themeVariables: {
    primaryColor: "#06d6a0",
    primaryTextColor: "#c0c0d0",
    lineColor: "#2a2a3a",
    secondaryColor: "#1e1e26",
    tertiaryColor: "#111113",
  },
});

async function render() {
  if (!props.code.trim()) return;
  error.value = "";
  try {
    const id = "mermaid-" + Math.random().toString(36).slice(2);
    const { svg: result } = await mermaid.render(id, props.code);
    svg.value = DOMPurify.sanitize(result);
  } catch (e: unknown) {
    error.value = (e as Error).message || String(e);
    svg.value = "";
  }
}

watch(() => props.code, async () => {
  await nextTick();
  render();
}, { immediate: true });
</script>

<template>
  <div v-if="svg" class="mermaid-container my-3 flex justify-center" v-html="svg"></div>
  <div v-if="error" class="text-xs p-2 rounded" :style="{ color: 'var(--coral)', background: 'rgba(255,94,91,0.08)' }">
    Mermaid error: {{ error }}
  </div>
</template>
