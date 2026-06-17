<script setup lang="ts">
import { computed } from "vue";

import { useSettingsStore } from "@/stores/settings";


const settings = useSettingsStore();

const activeMode = computed({
  get: () => {
    if (settings.planMode) return "plan";
    if (settings.autoMode) return "auto";
    if (settings.permissionMode === "acceptEdits") return "editAuto";
    return "askBefore";
  },
  set: (v: string) => {
    settings.planMode = v === "plan";
    settings.autoMode = v === "auto";
    settings.permissionMode = v === "editAuto" ? "acceptEdits" : "default";
  },
});
</script>

<template>
  <div class="shrink-0 max-w-3xl mx-auto px-1 pb-3">
    <div class="flex items-center gap-1 text-xs select-none" style="color: var(--text-muted)">
      <!-- 4 mode options -->
      <label
        v-for="mode in ['askBefore', 'editAuto', 'plan', 'auto']"
        :key="mode"
        class="px-3 py-1 rounded-md cursor-pointer transition-all whitespace-nowrap"
        :style="{
          background: activeMode === mode ? 'var(--bg-hover)' : 'transparent',
          color: activeMode === mode ? 'var(--accent)' : undefined,
          border: activeMode === mode ? '1px solid var(--accent)' : '1px solid transparent'
        }"
      >
        <input v-model="activeMode" :value="mode" type="radio" class="sr-only" />
        {{ $t(`mode.${mode}`) }}
      </label>

      <span style="color: var(--border-default)">·</span>

      <!-- Effort in same row -->
      <select
        v-model="settings.effort"
        class="bg-transparent cursor-pointer outline-none ml-auto text-xs"
        style="color: inherit"
      >
        <option value="low" style="background:var(--bg-surface)">{{ $t('mode.effort.low') }}</option>
        <option value="medium" style="background:var(--bg-surface)">{{ $t('mode.effort.medium') }}</option>
        <option value="high" style="background:var(--bg-surface)">{{ $t('mode.effort.high') }}</option>
        <option value="xhigh" style="background:var(--bg-surface)">{{ $t('mode.effort.xhigh') }}</option>
        <option value="max" style="background:var(--bg-surface)">{{ $t('mode.effort.max') }}</option>
        <option value="ultracode" style="background:var(--bg-surface)">{{ $t('mode.effort.ultracode') }}</option>
      </select>
    </div>
  </div>
</template>
