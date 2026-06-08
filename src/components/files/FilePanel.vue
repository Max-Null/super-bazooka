<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { useI18n } from "vue-i18n";
import { listDir, readFileContent, getWorkspaceRoot, type FileEntry } from "@/lib/tauri-bridge";
import FileTree from "./FileTree.vue";
import FilePreview from "./FilePreview.vue";

const props = defineProps<{ navCounter?: number; navPath?: string }>();
const { t } = useI18n();
const collapsed = ref(true);
const panelWidth = ref(280);
const rootPath = ref("");
const workspaceRoot = ref("");
const files = ref<FileEntry[]>([]);
const selectedFile = ref<string | null>(null);
const previewContent = ref("");

// Drag-to-resize
const dragging = ref(false);
function onDragStart(e: MouseEvent) {
  e.preventDefault();
  dragging.value = true;
  const startX = e.clientX;
  const startW = panelWidth.value;
  const onMove = (ev: MouseEvent) => {
    panelWidth.value = Math.min(600, Math.max(200, startW - (ev.clientX - startX)));
  };
  const onUp = () => {
    dragging.value = false;
    document.removeEventListener("mousemove", onMove);
    document.removeEventListener("mouseup", onUp);
  };
  document.addEventListener("mousemove", onMove);
  document.addEventListener("mouseup", onUp);
}

onMounted(async () => {
  try {
    workspaceRoot.value = await getWorkspaceRoot();
    rootPath.value = workspaceRoot.value;
    files.value = await listDir(rootPath.value);
  } catch { workspaceRoot.value = ""; }
});

// Listen for external navigation signal (e.g. header CWD click)
watch(() => props.navCounter, async () => {
  const path = props.navPath;
  if (!path) return;
  collapsed.value = false;
  rootPath.value = path;
  try { files.value = await listDir(path); } catch {}
});

function goRoot() {
  navigateTo(workspaceRoot.value);
}

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
    <!-- Drag handle (thin strip, visible when panel is open) -->
    <div
      v-if="!collapsed"
      @mousedown="onDragStart"
      class="w-1.5 shrink-0 cursor-col-resize hover:bg-[var(--accent)]/30 transition-colors select-none"
      :class="dragging ? 'bg-[var(--accent)]/40' : ''"
      :style="{ background: dragging ? 'var(--accent-dim)' : 'transparent' }"
    ></div>

    <!-- Drawer pull-tab -->
    <button
      @click="collapsed = !collapsed"
      class="shrink-0 w-7 flex items-center justify-center rounded-l-lg cursor-pointer select-none transition-all duration-200"
      :style="{
        background: collapsed ? 'var(--bg-surface)' : 'var(--bg-elevated)',
        color: 'var(--text-muted)',
        border: collapsed ? '1px solid var(--border-dim)' : 'none',
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
      :class="['flex flex-col overflow-hidden transition-all duration-200',
        collapsed ? 'w-0' : '']"
      :style="{
        width: collapsed ? '0' : panelWidth + 'px',
        background: 'var(--bg-surface)',
        borderLeft: collapsed ? 'none' : '1px solid var(--border-dim)',
      }"
    >
      <div v-if="!collapsed" class="flex flex-col h-full">
        <!-- Breadcrumb -->
        <div class="flex items-center gap-0.5 px-2 py-1.5 text-[11px] shrink-0 overflow-x-auto"
          :style="{ color: 'var(--text-muted)', borderBottom: '1px solid var(--border-dim)' }">
          <button @click="goRoot" class="hover:text-[var(--accent)] transition-colors shrink-0 mr-0.5" title="Back to workspace root">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>
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
          <FileTree :entries="files" :selected="selectedFile" @selectFile="openFile" @navigateTo="navigateTo" />
        </div>

        <!-- Preview — flex-1 to fill remaining space -->
        <div v-if="selectedFile && previewContent" class="border-t flex flex-col flex-1 min-h-0"
          :style="{ borderColor: 'var(--border-dim)' }">
          <div class="flex items-center justify-between px-3 h-8 text-[11px] shrink-0"
            :style="{ color: 'var(--text-secondary)', borderBottom: '1px solid var(--border-dim)' }">
            <span class="truncate font-medium">{{ selectedFile }}</span>
            <button
              @click="selectedFile = null; previewContent = ''"
              class="w-6 h-6 flex items-center justify-center rounded-md hover:bg-[var(--bg-hover)] transition-colors text-sm shrink-0"
              :style="{ color: 'var(--text-muted)' }"
              title="Close preview"
            >&times;</button>
          </div>
          <div class="flex-1 overflow-auto">
            <FilePreview :content="previewContent" :filename="selectedFile" />
          </div>
        </div>
      </div>
    </aside>
  </div>
</template>
