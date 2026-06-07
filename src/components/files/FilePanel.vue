<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { listDir, readFileContent, getWorkspaceRoot, type FileEntry } from "@/lib/tauri-bridge";
import FileTree from "./FileTree.vue";
import FilePreview from "./FilePreview.vue";

const { t } = useI18n();
const collapsed = ref(true);
const rootPath = ref("");
const files = ref<FileEntry[]>([]);
const selectedFile = ref<string | null>(null);
const previewContent = ref("");

onMounted(async () => {
  try {
    rootPath.value = await getWorkspaceRoot();
    files.value = await listDir(rootPath.value);
  } catch { rootPath.value = ""; }
});

async function navigateTo(path: string) {
  rootPath.value = path;
  try { files.value = await listDir(path); } catch {}
}

async function openFile(entry: FileEntry) {
  if (entry.is_dir) { navigateTo(entry.path); return; }
  selectedFile.value = entry.name;
  try { previewContent.value = await readFileContent(entry.path); }
  catch (e) { previewContent.value = `Error: ${e}`; }
}

// Split path into clickable segments for breadcrumb
import { computed } from "vue";

const pathSegments = computed(() => {
  const path = rootPath.value.replace(/[\\/]$/, "");
  const parts = path.split(/[\\/]/).filter(Boolean);
  const segments: { label: string; fullPath: string }[] = [];
  for (let i = 0; i < parts.length; i++) {
    const full = parts.slice(0, i + 1).join("\\");
    segments.push({ label: parts[i], fullPath: full + (i === 0 ? "\\" : "") });
  }
  return segments;
});

function goUp() {
  if (pathSegments.value.length <= 1) return;
  const parent = pathSegments.value[pathSegments.value.length - 2];
  navigateTo(parent.fullPath);
}
</script>

<template>
  <div class="flex shrink-0 overflow-hidden">
    <!-- Drawer pull-tab (always visible, acts as open/close handle) -->
    <button
      @click="collapsed = !collapsed"
      class="shrink-0 w-7 flex items-center justify-center rounded-l-lg cursor-pointer select-none transition-all duration-200"
      :style="{
        background: collapsed ? 'var(--bg-surface)' : 'var(--bg-elevated)',
        color: 'var(--text-muted)',
        border: '1px solid var(--border-dim)',
        borderRight: 'none',
      }"
      :title="collapsed ? t('file.title') : ''"
    >
      <span
        class="transition-transform duration-200"
        :style="{ writingMode: 'vertical-rl', fontSize: '10px', letterSpacing: '3px', transform: collapsed ? '' : 'rotate(180deg)' }"
      >
        {{ collapsed ? t('file.title') : '◀' }}
      </span>
    </button>

    <!-- Panel body -->
    <aside
      :class="['flex flex-col border-l overflow-hidden transition-all duration-200',
        collapsed ? 'w-0 border-transparent' : 'w-72']"
      :style="{ background: 'var(--bg-surface)', borderColor: 'var(--border-dim)' }"
    >
      <div v-if="!collapsed" class="flex flex-col h-full">
        <!-- Breadcrumb -->
        <div class="flex items-center gap-0.5 px-2 py-1.5 text-[11px] shrink-0 overflow-x-auto"
          :style="{ color: 'var(--text-muted)', borderBottom: '1px solid var(--border-dim)' }">
          <button @click="goUp" class="hover:text-[var(--accent)] transition-colors shrink-0 mr-0.5" title="Up">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M15 18l-6-6 6-6"/></svg>
          </button>
          <template v-for="(seg, i) in pathSegments" :key="seg.fullPath">
            <span class="text-[10px]" :style="{ color: 'var(--border-bright)' }" v-if="i > 0">›</span>
            <button
              @click="navigateTo(seg.fullPath)"
              class="hover:text-[var(--accent)] transition-colors truncate max-w-[120px] shrink-0"
              :class="{ 'font-medium': i === pathSegments.length - 1 }"
              :style="{ color: i === pathSegments.length - 1 ? 'var(--text-primary)' : 'var(--text-muted)' }"
            >
              {{ seg.label }}
            </button>
          </template>
        </div>

        <!-- File tree -->
        <div class="flex-1 overflow-y-auto px-1 py-0.5">
          <FileTree :entries="files" :selected="selectedFile" @select="openFile" @navigate="navigateTo" />
        </div>

        <!-- Preview -->
        <div v-if="selectedFile && previewContent" class="border-t shrink-0"
          :style="{ borderColor: 'var(--border-dim)' }">
          <div class="flex items-center justify-between px-3 h-7 text-[10px]"
            :style="{ color: 'var(--text-muted)', borderBottom: '1px solid var(--border-dim)' }">
            <span class="truncate">{{ selectedFile }}</span>
            <button @click="selectedFile = null; previewContent = ''" class="hover:text-[var(--accent)]">&times;</button>
          </div>
          <div class="max-h-64 overflow-auto">
            <FilePreview :content="previewContent" :filename="selectedFile" />
          </div>
        </div>
      </div>
    </aside>
  </div>
</template>
