<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, provide, type Ref } from "vue";

import { listDir, readFileContent, readFileBase64, getWorkspaceRoot, type FileEntry } from "@/lib/tauri-bridge";
import mammoth from "mammoth";
import { translateError } from "@/lib/utils";
import { useI18n } from "vue-i18n";
import ErrorBoundary from "@/components/shared/ErrorBoundary.vue";
import FileTree from "./FileTree.vue";
import FilePreview from "./FilePreview.vue";
import FilePreviewModal from "@/components/shared/FilePreviewModal.vue";

const props = defineProps<{ navCounter?: number; navPath?: string; forceClose?: number }>();

const { t } = useI18n();
const collapsed = ref(true);
const panelWidth = ref(280);
const rootPath = ref("");
const workspaceRoot = ref("");
const files = ref<FileEntry[]>([]);
const selectedFile = ref<string | null>(null);
const selectedFilePath = ref("");  // 完整路径，openModalPreview 从子目录选中文件时使用
const previewContent = ref("");
const previewFile = ref<{ name: string; path: string } | null>(null);

// ═══ 文件剪贴板（跨递归 FileTree 共享） ═══
interface ClipState { path: string; name: string; op: "copy" | "cut" }
const clipState = ref<ClipState | null>(null);
function setClip(v: ClipState | null) { clipState.value = v; }
provide("file-clipboard", { state: clipState as Ref<ClipState | null>, set: setClip });

// Drag-to-resize panel width
const draggingPanel = ref(false);
function onPanelDragStart(e: MouseEvent) {
  e.preventDefault();
  draggingPanel.value = true;
  const startX = e.clientX;
  const startW = panelWidth.value;
  const onMove = (ev: MouseEvent) => {
    panelWidth.value = Math.min(600, Math.max(200, startW - (ev.clientX - startX)));
  };
  const onUp = () => {
    draggingPanel.value = false;
    document.removeEventListener("mousemove", onMove);
    document.removeEventListener("mouseup", onUp);
  };
  document.addEventListener("mousemove", onMove);
  document.addEventListener("mouseup", onUp);
}

// Drag-to-resize splitter between file tree and preview
const splitRatio = ref(35); // 文件树占比 %
const draggingSplit = ref(false);
function onSplitDragStart(e: MouseEvent) {
  e.preventDefault();
  draggingSplit.value = true;
  const panelEl = (e.target as HTMLElement).closest(".sb-file-panel-body") as HTMLElement | null;
  if (!panelEl) return;
  const startY = e.clientY;
  const startRatio = splitRatio.value;
  const onMove = (ev: MouseEvent) => {
    const dy = ev.clientY - startY;
    const pct = (dy / panelEl.clientHeight) * 100;
    splitRatio.value = Math.min(80, Math.max(10, startRatio + pct));
  };
  const onUp = () => {
    draggingSplit.value = false;
    document.removeEventListener("mousemove", onMove);
    document.removeEventListener("mouseup", onUp);
  };
  document.addEventListener("mousemove", onMove);
  document.addEventListener("mouseup", onUp);
}

// CC 修改工作区文件后自动刷新文件面板
function onCcFileChanged() { refreshDir(); }

onMounted(async () => {
  try {
    workspaceRoot.value = await getWorkspaceRoot();
    // 优先使用外部传入的工作区路径（从 settings/SQLite 恢复），其次用自动检测的根目录
    rootPath.value = props.navPath || workspaceRoot.value;
    files.value = await listDir(rootPath.value);
  } catch { workspaceRoot.value = ""; }
  window.addEventListener("cc-file-changed", onCcFileChanged);
});

onUnmounted(() => {
  window.removeEventListener("cc-file-changed", onCcFileChanged);
});

// Listen for external navigation signal (e.g. header CWD click)
watch(() => props.navCounter, async () => {
  const path = props.navPath;
  if (!path) return;
  collapsed.value = false;
  rootPath.value = path;
  clearPreview();
  try { files.value = await listDir(path); } catch {}
});
// navPath 变更 → 同步文件面板（覆盖刷新后 settings 从 SQLite 恢复的场景）
watch(() => props.navPath, async (path) => {
  if (!path || path === rootPath.value) return;
  rootPath.value = path;
  clearPreview();
  try { files.value = await listDir(path); } catch {}
});
watch(() => props.forceClose, () => { collapsed.value = true; });
// 打开面板时自动刷新目录（CC 可能已修改文件）
watch(collapsed, async (v) => {
  if (!v && rootPath.value) { try { files.value = await listDir(rootPath.value); } catch {} }
});

/** 清空文件预览状态 */
function clearPreview() {
  selectedFile.value = null;
  selectedFilePath.value = "";
  previewContent.value = "";
}

function goRoot() {
  navigateTo(workspaceRoot.value);
}

/** 刷新当前目录列表 */
async function refreshDir() {
  try { files.value = await listDir(rootPath.value); } catch {}
}

async function navigateTo(path: string) {
  rootPath.value = path;
  try { files.value = await listDir(path); } catch {}
}

/** Word 文档扩展名 */
const DOCX_EXTS = new Set(["docx", "doc"]);

async function openFile(entry: FileEntry) {
  if (entry.is_dir) { navigateTo(entry.path); return; }
  selectedFile.value = entry.name;
  selectedFilePath.value = entry.path;  // 完整路径，供 openModalPreview 使用
  const ext = entry.name.split(".").pop()?.toLowerCase() || "";
  if (DOCX_EXTS.has(ext)) {
    // Word 文档：base64 → ArrayBuffer → mammoth 提取纯文本
    try {
      const b64 = await readFileBase64(entry.path);
      const bytes = Uint8Array.from(atob(b64), c => c.charCodeAt(0));
      const result = await mammoth.extractRawText({ arrayBuffer: bytes.buffer });
      previewContent.value = result.value;
    } catch (e) {
      const { key, params } = translateError(e);
      previewContent.value = t(key, params as any);
    }
  } else {
    try { previewContent.value = await readFileContent(entry.path); }
    catch (e) {
      const { key, params } = translateError(e);
      previewContent.value = t(key, params as any);
    }
  }
}

function openModalPreview() {
  if (selectedFile.value && selectedFilePath.value) {
    previewFile.value = { name: selectedFile.value, path: selectedFilePath.value };
  }
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
  <ErrorBoundary name="FilePanel">
    <div class="sb-file-panel flex shrink-0 overflow-hidden">
    <!-- Drag handle (thin strip, visible when panel is open) -->
    <div
      v-if="!collapsed"
      @mousedown="onPanelDragStart"
      class="w-1.5 shrink-0 cursor-col-resize hover:bg-[var(--accent)]/30 transition-colors select-none"
      :class="draggingPanel ? 'bg-[var(--accent)]/40' : ''"
      :style="{ background: draggingPanel ? 'var(--accent-dim)' : 'transparent' }"
    ></div>

    <!-- Drawer pull-tab -->
    <button
      @click="collapsed = !collapsed"
      class="shrink-0 w-7 flex items-center justify-center rounded-l-lg cursor-pointer select-none transition-all duration-200"
      :style="{
        background: collapsed ? 'var(--bg-surface)' : 'var(--bg-elevated)',
        color: 'var(--text-secondary)',
        border: collapsed ? '1px solid var(--border-dim)' : 'none',
        borderRight: 'none',
      }"
      :title="collapsed ? $t('file.title') : ''"
    >
      <span
        class="transition-transform duration-200"
        :style="{ writingMode: 'vertical-rl', fontSize: '10px', letterSpacing: '3px', transform: collapsed ? '' : 'rotate(180deg)' }"
      >
        {{ collapsed ? $t('file.title') : '◀' }}
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
        <div class="flex items-center gap-0.5 px-2 py-1.5 text-[11px] shrink-0 overflow-hidden"
          :style="{ color: 'var(--text-secondary)', borderBottom: '1px solid var(--border-dim)' }">
          <button @click="goRoot" class="hover:text-[var(--accent)] transition-colors shrink-0 mr-0.5" :title="$t('file.backToRoot')">
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
          <!-- 刷新按钮靠右 -->
          <button @click="refreshDir" class="hover:text-[var(--accent)] transition-colors shrink-0 ml-auto" :title="$t('file.refresh')">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 11-2.12-9.36L23 10"/></svg>
          </button>
        </div>

        <!-- File tree + preview（可拖动分隔条） -->
        <div class="sb-file-panel-body flex flex-col flex-1 min-h-0">
          <!-- File tree -->
          <div
            class="overflow-y-auto px-1 py-0.5 min-h-0"
            :style="selectedFile && previewContent ? { height: splitRatio + '%' } : { flex: 1 }"
          >
            <FileTree :entries="files" :selected="selectedFile" :onFileChanged="refreshDir" @selectFile="openFile" @navigateTo="navigateTo" />
          </div>

          <!-- 拖动分隔条 -->
          <div
            v-if="selectedFile && previewContent"
            @mousedown="onSplitDragStart"
            class="shrink-0 cursor-row-resize select-none flex items-center justify-center"
            :style="{
              height: '6px',
              background: draggingSplit ? 'var(--accent-dim)' : 'transparent',
              borderTop: '1px solid var(--border-dim)',
            }"
          >
            <div
              class="rounded-full transition-colors"
              :style="{
                width: '24px', height: '3px',
                background: draggingSplit ? 'var(--accent)' : 'var(--border-bright)',
              }"
            />
          </div>

          <!-- Inline preview panel -->
          <div v-if="selectedFile && previewContent" class="flex flex-col min-h-0" :style="{ flex: 1 }">
            <div class="flex items-center justify-between px-3 h-8 text-[11px] shrink-0"
              :style="{ color: 'var(--text-secondary)', borderBottom: '1px solid var(--border-dim)' }">
              <span class="truncate font-medium">{{ selectedFile }}</span>
              <div class="flex items-center gap-1">
                <button
                  @click="openModalPreview"
                  class="w-5 h-5 flex items-center justify-center rounded transition-colors hover:bg-[var(--bg-elevated)]"
                  style="color: var(--accent)"
                  :title="$t('file.openPreview')"
                >
                  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/></svg>
                </button>
                <button
                  @click="selectedFile = null; selectedFilePath = ''; previewContent = ''"
                  class="w-6 h-6 flex items-center justify-center rounded-md hover:bg-[var(--bg-hover)] transition-colors text-sm shrink-0"
                  :style="{ color: 'var(--text-secondary)' }"
                  :title="$t('file.closePreview')"
                >&times;</button>
              </div>
            </div>
            <div class="flex-1 overflow-auto">
              <FilePreview :content="previewContent" :filename="selectedFile" />
            </div>
          </div>
        </div>
      </div>
    </aside>
  </div>
  </ErrorBoundary>

  <!-- File preview modal -->
  <FilePreviewModal :file="previewFile" @close="previewFile = null" />
</template>
