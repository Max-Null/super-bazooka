<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import ModalShell from "./ModalShell.vue";
import { findEntry, getLastSeenVersion, markVersionSeen, APP_VERSION, localizedSections } from "@/data/changelog";

const { t, locale } = useI18n();

const emit = defineEmits<{ close: [] }>();

const entry = findEntry(APP_VERSION);
const lastSeen = getLastSeenVersion();
const show = computed(() => !!entry && lastSeen !== APP_VERSION);
const sections = computed(() => entry ? localizedSections(entry, locale.value) : []);

function onClose() {
  markVersionSeen(APP_VERSION);
  emit("close");
}
</script>

<template>
  <ModalShell :open="show" size="lg" position="top" @close="onClose">
    <template #header>
      <span class="text-base font-semibold" :style="{ color: 'var(--text-bright)' }">
        🎉 {{ $t('app.title') }} v{{ APP_VERSION }}
      </span>
    </template>

    <div class="text-[11px] mb-4" :style="{ color: 'var(--text-muted)' }">
      {{ entry?.date }}
    </div>

    <div v-if="sections.length" class="space-y-4">
      <div v-for="(section, si) in sections" :key="si">
        <h4 class="text-sm font-semibold mb-1.5" :style="{ color: 'var(--text-primary)' }">
          {{ section.title }}
        </h4>
        <ul
          class="text-[13px] leading-relaxed space-y-1 ml-1"
          :style="{ color: 'var(--text-secondary)' }"
        >
          <li v-for="(item, ii) in section.items" :key="ii" class="flex gap-2">
            <span
              class="shrink-0 select-none mt-1 w-1.5 h-1.5 rounded-full inline-block"
              style="background: var(--accent-dim)"
            ></span>
            <span>{{ item }}</span>
          </li>
        </ul>
      </div>
    </div>

    <div class="mt-5 pt-3 flex justify-end" :style="{ borderTop: '1px solid var(--border-dim)' }">
      <button
        @click="onClose"
        class="px-4 py-1.5 rounded-lg text-sm font-medium transition-colors"
        :style="{ background: 'var(--accent)', color: 'var(--bg-root)' }"
      >
        {{ $t('chat.close') }}
      </button>
    </div>
  </ModalShell>
</template>
