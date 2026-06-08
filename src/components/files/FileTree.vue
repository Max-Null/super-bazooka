<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { listDir, revealInExplorer as reveal, type FileEntry } from "@/lib/tauri-bridge";

const { t } = useI18n();
const props = defineProps<{
  entries: FileEntry[];
  selected: string | null;
}>();

const emit = defineEmits<{
  selectFile: [entry: FileEntry];
  navigateTo: [path: string];
}>();

// Context menu state
const ctxMenu = ref<{ x: number; y: number; entry: FileEntry } | null>(null);

function onContextMenu(e: MouseEvent, entry: FileEntry) {
  e.preventDefault();
  ctxMenu.value = { x: e.clientX, y: e.clientY, entry };
}

function closeMenu() { ctxMenu.value = null; }
onMounted(() => document.addEventListener("click", closeMenu));
onUnmounted(() => document.removeEventListener("click", closeMenu));

async function revealInExplorer(entry: FileEntry) {
  closeMenu();
  try { await reveal(entry.path); } catch {}
}

async function copyToClipboard(text: string) {
  closeMenu();
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    // clipboard fallback handled silently
  }
}

// Track expanded directories and their children
const expandedDirs = ref<Record<string, FileEntry[]>>({});
const loadingDirs = ref<Record<string, boolean>>({});

async function toggleDir(entry: FileEntry) {
  if (!entry.is_dir) {
    emit("selectFile", entry);
    return;
  }

  const key = entry.path;
  if (expandedDirs.value[key]) {
    // Collapse
    delete expandedDirs.value[key];
    expandedDirs.value = { ...expandedDirs.value }; // trigger reactivity
  } else {
    // Expand — load children
    loadingDirs.value[key] = true;
    loadingDirs.value = { ...loadingDirs.value };
    try {
      const children = await listDir(entry.path);
      expandedDirs.value[key] = children;
      expandedDirs.value = { ...expandedDirs.value };
    } catch { /* permission denied */ }
    finally {
      delete loadingDirs.value[key];
      loadingDirs.value = { ...loadingDirs.value };
    }
  }
}

function icon(isDir: boolean, name: string): string {
  if (isDir) {
    const key = name;
    // Check if expanded
    for (const k in expandedDirs.value) {
      if (k.endsWith("\\" + key) || k.endsWith("/" + key) || k === key) {
        return "📂";
      }
    }
    return "📁";
  }
  const ext = name.split(".").pop()?.toLowerCase();
  const map: Record<string, string> = {
    ts: "🔷", tsx: "⚛️", js: "🟨", vue: "💚", rs: "🦀",
    py: "🐍", css: "🎨", html: "🌐", json: "📋", md: "📝",
    yml: "⚙️", yaml: "⚙️", sql: "🗄️", toml: "⚙️",
    svg: "🖼️", png: "🖼️", ico: "🖼️",
    sh: "💻", bat: "💻", ps1: "💻",
  };
  return map[ext || ""] || "📄";
}

function sizeStr(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function isExpanded(path: string): boolean {
  return !!expandedDirs.value[path];
}

function isLoading(path: string): boolean {
  return !!loadingDirs.value[path];
}
</script>

<template>
  <div class="text-xs select-none leading-relaxed">
    <template v-for="entry in entries" :key="entry.path">
      <!-- Entry row -->
      <div
        @click="toggleDir(entry)"
        @contextmenu="onContextMenu($event, entry)"
        :class="[
          'flex items-center gap-1.5 px-2 py-0.5 rounded cursor-pointer transition-colors truncate',
          selected === entry.name
            ? 'bg-[var(--accent)]/10 text-[var(--accent)]'
            : 'hover:bg-[var(--bg-hover)]'
        ]"
        :style="{ color: selected === entry.name ? 'var(--accent)' : 'var(--text-secondary)' }"
      >
        <!-- Expand toggle for dirs -->
        <span
          v-if="entry.is_dir"
          class="w-3.5 shrink-0 text-center text-[10px]"
          :style="{ color: 'var(--text-muted)' }"
        >
          <span v-if="isLoading(entry.path)" class="inline-block animate-spin">⟳</span>
          <span v-else>{{ isExpanded(entry.path) ? '▼' : '▶' }}</span>
        </span>
        <span v-else class="w-3.5 shrink-0"></span>

        <span class="text-xs shrink-0">{{ icon(entry.is_dir, entry.name) }}</span>
        <span class="truncate">{{ entry.name }}</span>
        <span
          v-if="!entry.is_dir"
          class="ml-auto text-[10px] shrink-0"
          :style="{ color: 'var(--text-muted)' }"
        >{{ sizeStr(entry.size) }}</span>
      </div>

      <!-- Children (if expanded) -->
      <div v-if="entry.is_dir && isExpanded(entry.path)" class="ml-3.5 border-l"
        :style="{ borderColor: 'var(--border-dim)' }">
        <FileTree
          v-if="expandedDirs[entry.path]?.length"
          :entries="expandedDirs[entry.path]"
          :selected="selected"
          @selectFile="emit('selectFile', $event)"
          @navigateTo="emit('navigateTo', $event)"
        />
        <div v-else class="px-2 py-1 text-[10px]" :style="{ color: 'var(--text-muted)' }">
          Empty
        </div>
      </div>
    </template>

    <div
      v-if="entries.length === 0"
      class="px-2 py-4 text-center"
      :style="{ color: 'var(--text-muted)' }"
    >
      Empty directory
    </div>

    <!-- Context menu -->
    <Teleport to="body">
      <div
        v-if="ctxMenu"
        class="fixed z-50 py-1 rounded-lg shadow-lg border text-[11px] min-w-[160px]"
        :style="{
          left: ctxMenu.x + 'px',
          top: ctxMenu.y + 'px',
          background: 'var(--bg-elevated)',
          borderColor: 'var(--border-default)',
          color: 'var(--text-secondary)',
        }"
        @click.stop
      >
        <button
          @click="revealInExplorer(ctxMenu.entry)"
          class="w-full text-left px-3 py-1.5 hover:bg-[var(--bg-hover)] transition-colors flex items-center gap-2"
        >📂 {{ t('file.reveal') }}</button>
        <button
          @click="copyToClipboard(ctxMenu.entry.path)"
          class="w-full text-left px-3 py-1.5 hover:bg-[var(--bg-hover)] transition-colors flex items-center gap-2"
        >📋 {{ t('file.copyPath') }}</button>
        <button
          @click="copyToClipboard(ctxMenu.entry.name)"
          class="w-full text-left px-3 py-1.5 hover:bg-[var(--bg-hover)] transition-colors flex items-center gap-2"
        >📝 {{ t('file.copyName') }}</button>
      </div>
    </Teleport>
  </div>
</template>
