<script setup lang="ts">
import { ref, onMounted } from "vue";
import { listDir, readFileContent, getWorkspaceRoot, type FileEntry } from "@/lib/tauri-bridge";
import FileTree from "./FileTree.vue";
import FilePreview from "./FilePreview.vue";

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

const collapsed = ref(true);
const rootPath = ref("");
const files = ref<FileEntry[]>([]);
const selectedFile = ref<string | null>(null);
const previewContent = ref("");
const previewLoading = ref(false);

onMounted(async () => {
  try {
    rootPath.value = await invoke<string>("get_workspace_root");
    files.value = await invoke<FileEntry[]>("list_dir", { path: rootPath.value });
  } catch {
    rootPath.value = "";
  }
});

async function navigateTo(path: string) {
  try {
    files.value = await invoke<FileEntry[]>("list_dir", { path });
  } catch {
    // Permission denied or not a directory
  }
}

async function openFile(entry: FileEntry) {
  if (entry.is_dir) {
    navigateTo(entry.path);
  } else {
    selectedFile.value = entry.name;
    previewLoading.value = true;
    try {
      previewContent.value = await invoke<string>("read_file_content", { path: entry.path });
    } catch (e) {
      previewContent.value = `Error: ${e}`;
    } finally {
      previewLoading.value = false;
    }
  }
}

function toggle() {
  collapsed.value = !collapsed.value;
}

function goUp() {
  const parent = rootPath.value.replace(/[\\/][^\\/]+$/, "");
  if (parent && parent !== rootPath.value) {
    rootPath.value = parent;
    navigateTo(parent);
  }
}
</script>

<template>
  <aside
    :class="['flex flex-col border-l overflow-hidden transition-all duration-200 shrink-0',
      collapsed ? 'w-0 border-transparent' : 'w-72']"
    :style="{ background: 'var(--bg-surface)', borderColor: 'var(--border-dim)' }"
  >
    <div v-if="!collapsed" class="flex flex-col h-full">
      <!-- Header -->
      <div
        class="flex items-center justify-between h-8 px-3 shrink-0 text-[11px] font-medium select-none"
        :style="{ color: 'var(--text-secondary)', borderBottom: '1px solid var(--border-dim)' }"
      >
        <span>Files</span>
        <button @click="toggle" class="hover:text-[var(--accent)] transition-colors text-xs">&times;</button>
      </div>

      <!-- Breadcrumb -->
      <div class="flex items-center gap-1 px-2 py-1 text-[10px] shrink-0" style="color:var(--text-muted)">
        <button @click="goUp" class="hover:text-[var(--accent)] transition-colors" title="Up">&larr;</button>
        <span class="truncate">{{ rootPath || '/' }}</span>
      </div>

      <!-- File tree -->
      <div class="flex-1 overflow-y-auto px-1 py-0.5">
        <FileTree
          :entries="files"
          :selected="selectedFile"
          @select="openFile"
          @navigate="navigateTo"
        />
      </div>

      <!-- Preview -->
      <div
        v-if="selectedFile && previewContent"
        class="border-t shrink-0"
        :style="{ borderColor: 'var(--border-dim)' }"
      >
        <div
          class="flex items-center justify-between px-3 h-7 text-[10px]"
          :style="{ color: 'var(--text-muted)', borderBottom: '1px solid var(--border-dim)' }"
        >
          <span class="truncate">{{ selectedFile }}</span>
          <button @click="selectedFile = null; previewContent = ''" class="hover:text-[var(--accent)]">&times;</button>
        </div>
        <div class="max-h-64 overflow-auto">
          <FilePreview :content="previewContent" :filename="selectedFile" />
        </div>
      </div>
    </div>
  </aside>

  <!-- Collapse toggle (visible when collapsed) -->
  <button
    v-if="collapsed"
    @click="toggle"
    class="absolute right-0 top-1/2 -translate-y-1/2 w-6 h-12 rounded-l-md flex items-center justify-center text-[10px] transition-colors z-10"
    :style="{ background: 'var(--bg-surface)', color: 'var(--text-muted)', border: '1px solid var(--border-dim)' }"
    title="Show files"
  >
    &lt;
  </button>
</template>
