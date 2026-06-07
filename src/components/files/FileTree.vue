<script setup lang="ts">
defineProps<{
  entries: { name: string; path: string; is_dir: boolean; size: number }[];
  selected: string | null;
}>();

const emit = defineEmits<{
  select: [entry: { name: string; path: string; is_dir: boolean; size: number }];
  navigate: [path: string];
}>();

function icon(isDir: boolean, name: string): string {
  if (isDir) return "📁";
  const ext = name.split(".").pop()?.toLowerCase();
  const map: Record<string, string> = {
    ts: "🔷", tsx: "⚛️", js: "🟨", jsx: "⚛️", vue: "💚", rs: "🦀",
    py: "🐍", css: "🎨", html: "🌐", json: "📋", md: "📝", yml: "⚙️",
    yaml: "⚙️", sql: "🗄️", toml: "⚙️", lock: "🔒", gitignore: "🙈",
    svg: "🖼️", png: "🖼️", jpg: "🖼️", ico: "🖼️", woff: "🔤", ttf: "🔤",
    sh: "💻", bat: "💻", ps1: "💻", exe: "⚡", dll: "🔧",
  };
  return map[ext || ""] || "📄";
}

function sizeStr(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
</script>

<template>
  <div class="text-xs select-none leading-relaxed">
    <div
      v-for="entry in entries"
      :key="entry.path"
      @click="emit('select', entry)"
      :class="[
        'flex items-center gap-1.5 px-2 py-0.5 rounded cursor-pointer transition-colors truncate',
        selected === entry.name
          ? 'bg-[var(--accent)]/10 text-[var(--accent)]'
          : 'hover:bg-[var(--bg-hover)]'
      ]"
      :style="{ color: selected === entry.name ? 'var(--accent)' : 'var(--text-secondary)' }"
    >
      <span class="text-xs shrink-0">{{ icon(entry.is_dir, entry.name) }}</span>
      <span class="truncate">{{ entry.name }}</span>
      <span
        v-if="!entry.is_dir"
        class="ml-auto text-[10px] shrink-0"
        :style="{ color: 'var(--text-muted)' }"
      >{{ sizeStr(entry.size) }}</span>
    </div>
    <div
      v-if="entries.length === 0"
      class="px-2 py-4 text-center"
      :style="{ color: 'var(--text-muted)' }"
    >
      Empty directory
    </div>
  </div>
</template>
