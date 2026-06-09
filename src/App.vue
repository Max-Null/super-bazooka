<script setup lang="ts">
import { computed } from "vue";
import AppShell from "./components/layout/AppShell.vue";
import { useStreamProcessor } from "./composables/useStreamProcessor";
import { useSettingsStore } from "./stores/settings";
import { onMounted, onUnmounted } from "vue";

const { startListening, stopListening } = useStreamProcessor();
const settings = useSettingsStore();

const themeAttr = computed(() => settings.theme);

onMounted(() => {
  startListening();
  // Apply theme from stored preference
  document.documentElement.setAttribute("data-theme", settings.theme);
});

onUnmounted(() => {
  stopListening();
});

// Watch theme changes and apply to root
import { watch } from "vue";
watch(() => settings.theme, (t) => {
  document.documentElement.setAttribute("data-theme", t);
});
</script>

<template>
  <div :data-theme="themeAttr" class="h-full">
    <AppShell />
  </div>
</template>
