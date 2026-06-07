<script setup lang="ts">
import AppShell from "./components/layout/AppShell.vue";
import { useStreamProcessor } from "./composables/useStreamProcessor";
import { onMounted, onUnmounted } from "vue";

const { startListening, stopListening } = useStreamProcessor();

onMounted(() => {
  startListening();
});

onUnmounted(() => {
  stopListening();
});
</script>

<template>
  <AppShell />
</template>
