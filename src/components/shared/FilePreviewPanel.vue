<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted, shallowRef, nextTick, inject } from "vue";

import { readFileContent, readFileBase64, saveFileContent, checkSkillInstalled } from "@/lib/tauri-bridge";
import { isImageFile, mimeType } from "@/composables/useFilePreview";
import { emitChatCommand } from "@/composables/useCommandPalette";
import MarkdownRenderer from "./MarkdownRenderer.vue";
import mammoth from "mammoth";
import DOMPurify from "dompurify";
import { useI18n } from "vue-i18n";
import { PANEL_LAYOUT_KEY } from "@/composables/usePanelLayout";
// CodeMirror 6（编辑 tab）
import { EditorView, keymap, lineNumbers, highlightActiveLine } from "@codemirror/view";
import { EditorState, Prec } from "@codemirror/state";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { syntaxHighlighting, defaultHighlightStyle } from "@codemirror/language";
import { oneDark } from "@codemirror/theme-one-dark";
import { javascript } from "@codemirror/lang-javascript";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";
import { css } from "@codemirror/lang-css";
import { html } from "@codemirror/lang-html";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { sql } from "@codemirror/lang-sql";
import { xml } from "@codemirror/lang-xml";
import { yaml } from "@codemirror/lang-yaml";

const props = defineProps<{
  file: { name: string; path: string } | null;
}>();

const emit = defineEmits<{ close: [] }>();

// 列宽由 usePanelLayout composable 统一管理
const layout = inject(PANEL_LAYOUT_KEY)!;

// HTML 预览宽度预设：0 = 跟随面板，>0 = 固定宽度
const HTML_PRESETS = [0, 375, 768, 1024, 1440, 1920] as const
const htmlWidth = ref(0)

const content = ref("");       // 原始文本（编辑 tab）
const previewHtml = ref("");   // 渲染后 HTML（预览 tab）
const loading = ref(false);
const error = ref("");
const imageSrc = ref("");
const activeTab = ref<"edit" | "preview">("edit");

// ── HTML 预览 Blob URL（绕过 srcdoc 的脚本执行 bug）──
const previewHtmlBlob = ref("");
function updateHtmlBlob(html: string) {
  URL.revokeObjectURL(previewHtmlBlob.value);
  previewHtmlBlob.value = URL.createObjectURL(new Blob([html], { type: "text/html" }));
}
onUnmounted(() => URL.revokeObjectURL(previewHtmlBlob.value));

// ── DOM 选择器（HTML 预览 tab）──
interface DomInfo { tag: string; id: string; classes: string; text: string; attrs: Record<string, string>; left: number; top: number; bottom: number }
const selectedDom = ref<DomInfo | null>(null);
const previewIframe = ref<HTMLIFrameElement | null>(null);

function onIframeMessage(e: MessageEvent) {
  if (e.data?.type !== "dom-selected") return;
  // 只处理来自当前实例 iframe 的消息，避免多实例同时响应
  if (e.source !== previewIframe.value?.contentWindow) return;
  const info = e.data.info as DomInfo;
  // iframe 内坐标 → 页面坐标（加上 iframe 偏移）
  const ir = previewIframe.value?.getBoundingClientRect();
  if (ir) {
    info.left += ir.left;
    info.top += ir.top;
    info.bottom += ir.top;
  }
  selectedDom.value = info;
}
onMounted(() => window.addEventListener("message", onIframeMessage));
onUnmounted(() => window.removeEventListener("message", onIframeMessage));

// CC 修改文件后自动重载编辑器内容
function onCcFileChanged() {
  if (!props.file) return;
  readFileContent(props.file.path).then(raw => {
      content.value = raw;
      // HTML 预览 tab：重新生成 iframe 的 blob URL，让预览区刷新
      if (fileKind.value === "html") {
        const injected = raw.replace("<body>", "<body>" + INSPECTOR_SCRIPT)
          || raw + INSPECTOR_SCRIPT;
        updateHtmlBlob(injected);
      }
      // 编辑 tab 才重建编辑器；Markdown/docx 等预览只刷新 content
      if (activeTab.value === "edit") {
        destroyEditor();
        dirty.value = false;
        maybeCreateEditor();
      }
    }).catch(() => {});
}
onMounted(() => window.addEventListener("cc-file-changed", onCcFileChanged));
onUnmounted(() => window.removeEventListener("cc-file-changed", onCcFileChanged));

const { t } = useI18n();
const converting = ref(false);

/** MD → docx：检测 docx skill → 安装或直接发送给 CC */
async function sendConvertDocx() {
  if (!props.file) return;
  converting.value = true;
  try {
    const hasSkill = await checkSkillInstalled("docx");
    if (hasSkill) {
      const cmd = `请使用 /docx 命令将 \`${props.file.path}\` 转换为 docx 格式，输出到原文件同级位置，中间文件请使用临时目录，完成后清理临时文件`;
      emitChatCommand(`md-convert:${cmd}`);
      // panel 模式不关闭，保持编辑
    } else {
      const cmd = `请先全局安装 docx skill：npx skills add https://github.com/anthropics/skills --skill docx，安装完成后将 \`${props.file.path}\` 转换为 docx 格式，输出到原文件同级位置，中间文件请使用临时目录并清理`;
      emitChatCommand(`md-convert:${cmd}`);
      // panel 模式不关闭，保持编辑
    }
  } finally {
    converting.value = false;
  }
}

function sendDomToChat() {
  if (!selectedDom.value || !props.file) return;
  const d = selectedDom.value;
  const parts = [`<${d.tag}`, d.id ? ` id="${d.id}"` : "", d.classes ? ` class="${d.classes}"` : ""];
  const a = Object.entries(d.attrs).map(([k, v]) => ` ${k}="${v}"`).join("");
  if (a) parts.push(a);
  parts.push(">");
  if (d.text) parts.push(`\n  ${d.text}\n`);
  parts.push(`</${d.tag}>`);
  const msg = `我在 \`${props.file.path}\` 中选中了这个元素，请修改其内容：\n\`\`\`html\n${parts.join("")}\n\`\`\``;
  emitChatCommand(`attach-dom:${msg}`);
  selectedDom.value = null;
  // panel 模式不关闭，保持编辑  // panel 模式不关闭，保持编辑
}

// ── MD 选中 → 发送修改建议到对话 ──
const mdSelection = ref<{ text: string; from: number; to: number; startLine: number; endLine: number } | null>(null);
const mdSuggestion = ref("");
const mdTipPos = ref<{ left: string; top: string }>({ left: "0px", top: "0px" });

function updateMdTipPos() {
  if (!mdSelection.value || !editorView.value) {
    mdTipPos.value = { left: "-9999px", top: "-9999px" };  // 隐藏
    return;
  }
  const coords = editorView.value.coordsAtPos(mdSelection.value.to);
  if (!coords) {
    mdTipPos.value = { left: "-9999px", top: "-9999px" };
    return;
  }
  // 定位到选区末尾右下方，偏移避免遮挡
  mdTipPos.value = {
    left: `${Math.min(coords.right + 8, window.innerWidth - 380)}px`,
    top: `${Math.min(coords.bottom + 6, window.innerHeight - 160)}px`,
  };
}

function domTipStyle() {
  if (!selectedDom.value) return { left: '-9999px', top: '-9999px' };
  return {
    left: Math.min(selectedDom.value.left, window.innerWidth - 300) + 'px',
    top: Math.max(selectedDom.value.bottom + 6, 8) + 'px',
  };
}

function sendMdToChat() {
  if (!mdSelection.value || !props.file) return;
  const sel = mdSelection.value;
  const suggestion = mdSuggestion.value.trim();
  // 包含文件路径 + 行列信息，让 AI 精确知道改哪里
  const loc = `文件 \`${props.file.path}\` 第 ${sel.startLine}-${sel.endLine} 行`;
  const body = suggestion
    ? `${loc}，请修改以下内容：\n\n\`\`\`markdown\n${sel.text}\n\`\`\`\n\n修改建议：${suggestion}`
    : `${loc}，请修改以下内容：\n\n\`\`\`markdown\n${sel.text}\n\`\`\``;
  emitChatCommand(`md-selection:${body}`);
  // 清除选区和输入
  mdSelection.value = null;
  mdSuggestion.value = "";
  editorView.value?.dispatch({ selection: { anchor: 0, head: 0 } });
}

// ── 注入到 HTML 预览 iframe 的选择器脚本 ──
const INSPECTOR_SCRIPT = `\x3Cscript\x3E
var __o=document.createElement('div');
__o.style.cssText='position:fixed;pointer-events:none;z-index:99999;border:2px solid #3b82f6;background:rgba(59,130,246,0.08);display:none;border-radius:2px;';
document.body.appendChild(__o);
var __last=null;
document.addEventListener('mouseover',function(e){
var el=e.target;
if(el===__o||el===document.body||el===document.documentElement)return;
if(el===__last)return;
__last=el;
var r=el.getBoundingClientRect();
__o.style.display='block';__o.style.left=r.left+'px';__o.style.top=r.top+'px';
__o.style.width=r.width+'px';__o.style.height=r.height+'px';
});
document.addEventListener('click',function(e){
e.preventDefault();e.stopPropagation();
__last=null;__o.style.display='none';
var el=e.target,a={};
for(var i=0;i<el.attributes.length;i++){
var at=el.attributes[i];
if(at.name==='class'||at.name==='id'||at.name==='style')continue;
a[at.name]=at.value;
}
var r=el.getBoundingClientRect();
window.parent.postMessage({type:'dom-selected',info:{
tag:el.tagName.toLowerCase(),
id:el.id||'',
classes:Array.from(el.classList).join(' '),
text:(el.textContent||'').trim().slice(0,300),
attrs:a,
left:r.left,top:r.top,bottom:r.bottom
}},'*');
});
\x3C\/script\x3E`;

// ── 文件类型检测 ──

type FileKind = "text" | "html" | "markdown" | "docx" | "image" | "unsupported";

const DOCX_EXTS = new Set(["docx", "doc"]);
const HTML_EXTS = new Set(["html", "htm"]);
const MD_EXTS = new Set(["md", "mdx", "markdown"]);

function detectKind(filename: string): FileKind {
  const ext = filename.split(".").pop()?.toLowerCase() || "";
  if (isImageFile(filename)) return "image";
  if (DOCX_EXTS.has(ext)) return "docx";
  if (HTML_EXTS.has(ext)) return "html";
  if (MD_EXTS.has(ext)) return "markdown";
  // 二进制 / 无文本预览
  const binary = new Set([
    "exe","dll","so","dylib","bin","dat","db","sqlite","sqlite3",
    "xlsx","xls","pptx","ppt","pdf","zip","tar","gz","rar","7z",
    "mp3","mp4","avi","mov","mkv","wav","flac",
    "ttf","otf","woff","woff2","eot","class","pyc","o","obj","lib","a","wasm",
  ]);
  if (binary.has(ext)) return "unsupported";
  return "text";
}

const fileKind = computed(() => props.file ? detectKind(props.file.name) : "unsupported");

/** 哪些 tab 可用 */
const hasEdit = computed(() => fileKind.value !== "image" && fileKind.value !== "unsupported");
const hasPreview = computed(() =>
  fileKind.value === "html" || fileKind.value === "markdown" || fileKind.value === "docx" || fileKind.value === "image"
);

// ── 加载文件 ──

watch(() => props.file, async (f) => {
  if (!f) return;
  content.value = "";
  previewHtml.value = "";
  error.value = "";
  imageSrc.value = "";
  loading.value = true;
  mdSelection.value = null;
  mdSuggestion.value = "";

  const kind = detectKind(f.name);

  if (kind === "image") {
    activeTab.value = "preview";
    try {
      const b64 = await readFileBase64(f.path);
      imageSrc.value = `data:${mimeType(f.name)};base64,${b64}`;
    } catch (e) { error.value = String(e); }
    loading.value = false;
    return;
  }

  if (kind === "unsupported") {
    activeTab.value = "edit";
    loading.value = false;
    return;
  }

  if (kind === "docx") {
    activeTab.value = "preview"; // Word 优先预览
    try {
      const b64 = await readFileBase64(f.path);
      const raw = Uint8Array.from(atob(b64), c => c.charCodeAt(0));
      const buf = raw.buffer.slice(raw.byteOffset, raw.byteOffset + raw.byteLength);
      const rawHtml = (await mammoth.convertToHtml({ arrayBuffer: buf })).value;
      previewHtml.value = DOMPurify.sanitize(rawHtml);
      content.value = (await mammoth.extractRawText({ arrayBuffer: buf })).value;
    } catch (e) { error.value = String(e); }
    loading.value = false;
    return;
  }

  // 文本文件（html / markdown / code / text）
  activeTab.value = hasPreview.value ? "preview" : "edit";
  try {
    const raw = await readFileContent(f.path);
    content.value = raw;
    if (kind === "html") {
      const injected = raw.replace("<body>", "<body>" + INSPECTOR_SCRIPT)
        || raw + INSPECTOR_SCRIPT;
      updateHtmlBlob(injected);
    }
    // markdown 预览由 MarkdownRenderer 处理，不需要 previewHtml
  } catch (e) { error.value = String(e); }
  loading.value = false;
}, { immediate: true });

// ── CodeMirror 编辑器 ──

const editorContainer = ref<HTMLElement | null>(null);
const editorView = shallowRef<EditorView | null>(null);
const dirty = ref(false);
const saving = ref(false);

function cmLang(filename: string) {
  const ext = filename.split(".").pop()?.toLowerCase();
  switch (ext) {
    case "ts": case "tsx": case "js": case "jsx": return javascript();
    case "py": return python();
    case "rs": return rust();
    case "css": return css();
    case "vue": case "svelte": return html();
    case "json": return json();
    case "md": return markdown();
    case "sql": return sql();
    case "xml": case "svg": return xml();
    case "yaml": case "yml": return yaml();
    default: return null;
  }
}

function createEditor(doc: string) {
  if (!editorContainer.value || !props.file) return;
  editorView.value?.destroy();
  const extensions = [
    lineNumbers(),
    highlightActiveLine(),
    history(),
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    oneDark,
    keymap.of([...defaultKeymap, ...historyKeymap]),
    // Ctrl+S 通过 CodeMirror keymap 注册（而非 DOM @keydown，避免被编辑器吞事件）
    Prec.high(keymap.of([{ key: "Mod-s", run: () => { saveFile(); return true; } }])),
    EditorView.updateListener.of((update) => {
      if (update.docChanged) dirty.value = true;
      // MD 文件选中检测 → 浮动修改建议 Tip
      if (props.file && fileKind.value === "markdown") {
        const sel = update.state.selection.main;
        if (!sel.empty) {
          const text = update.state.sliceDoc(sel.from, sel.to);
          const startLine = update.state.doc.lineAt(sel.from).number;
          const endLine = update.state.doc.lineAt(sel.to).number;
          mdSelection.value = { text, from: sel.from, to: sel.to, startLine, endLine };
          updateMdTipPos();
        } else {
          mdSelection.value = null;
          mdSuggestion.value = "";
        }
      }
    }),
  ];
  const lang = cmLang(props.file.name);
  if (lang) extensions.push(lang);
  editorView.value = new EditorView({
    state: EditorState.create({ doc, extensions }),
    parent: editorContainer.value,
  });
  // 编辑器滚动时更新 MD Tip 位置
  editorView.value.scrollDOM.addEventListener("scroll", updateMdTipPos, { passive: true });
}

function destroyEditor() {
  editorView.value?.destroy();
  editorView.value = null;
}

// 需要初始化/重建编辑器时调用
function maybeCreateEditor() {
  if (!editorView.value && content.value && props.file && hasEdit.value && activeTab.value === "edit") {
    nextTick(() => createEditor(content.value));
  }
}

// 文件切换时重建编辑器
watch(() => props.file?.path, () => {
  destroyEditor();
  dirty.value = false;
  maybeCreateEditor();
});

// 文件内容加载完成时初始化编辑器
watch(content, (val) => {
  if (val) maybeCreateEditor();
});

onMounted(() => maybeCreateEditor());
onUnmounted(() => destroyEditor());

// 切换到编辑 tab 时初始化编辑器，离开时销毁——v-else-if 会重建 DOM，
// 旧 EditorView 挂载的父元素已不在文档中，必须重建
watch(activeTab, (tab) => {
  if (tab === "edit") { maybeCreateEditor(); selectedDom.value = null; }
  else { destroyEditor(); }
});

// ── 保存 ──

async function saveFile() {
  if (!props.file || !editorView.value || saving.value) return;
  saving.value = true;
  try {
    const text = editorView.value.state.doc.toString();
    await saveFileContent(props.file.path, text);
    content.value = text;
    dirty.value = false;
  } catch (e) {
    console.error("[FilePreviewModal] Save failed:", e);
  } finally {
    saving.value = false;
  }
}

// ── 关闭确认 ──

function confirmClose() {
  if (confirm(t("file.unsavedChanges"))) {
    dirty.value = false;
    emit("close");
  }
}

function handleClose() {
  if (dirty.value) { confirmClose(); } else { emit("close"); }
}
</script>

<template>
  <div v-if="file" class="flex h-full">
    <!-- 拖拽把手 -->
    <div
      @mousedown="layout.startResize('preview', $event)"
      class="panel-drag-handle"
      :class="{ 'panel-drag-handle--active': layout.previewDragging.value }"
    />

    <!-- 面板主体 -->
    <div class="panel-body" :style="{ width: layout.previewWidth.value + 'px' }">
      <!-- Header -->
      <div class="panel-header">
        <div class="flex items-center gap-1.5 min-w-0 flex-1">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2" stroke-linecap="round"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/></svg>
          <span class="text-xs font-medium truncate" style="color: var(--text-bright)" :title="file.path">{{ file.name }}</span>
          <span v-if="dirty" class="shrink-0 w-1.5 h-1.5 rounded-full" style="background: var(--amber)" title="未保存" />
        </div>
        <div v-if="hasEdit && hasPreview" class="flex rounded-md shrink-0" style="background: var(--bg-root); border: 1px solid var(--border-dim)">
          <button
            v-for="t in (['edit', 'preview'] as const)"
            :key="t"
            @click="activeTab = t"
            class="text-[10px] px-2.5 py-0.5 font-medium transition-colors rounded-md"
            :style="{ background: activeTab === t ? 'var(--accent)' : 'transparent', color: activeTab === t ? 'var(--bg-root)' : 'var(--text-muted)' }"
          >{{ t === 'edit' ? $t('preview.edit') : $t('preview.previewTab') }}</button>
        </div>
        <button v-if="activeTab === 'edit' && dirty" @click="saveFile" :disabled="saving"
          class="text-[10px] px-2 py-0.5 rounded font-medium transition-colors shrink-0"
          :class="saving ? 'opacity-50 cursor-not-allowed' : 'hover:opacity-80'"
          style="background: var(--accent); color: var(--bg-root)"
        >{{ saving ? '…' : $t('preview.save') }}</button>
        <button v-if="fileKind === 'markdown'" @click="sendConvertDocx" :disabled="converting"
          class="text-[10px] px-2 py-0.5 rounded font-medium transition-colors shrink-0"
          :class="converting ? 'opacity-50 cursor-not-allowed' : 'hover:opacity-80'"
          style="background: var(--accent); color: var(--bg-root)"
          :title="$t('preview.convertDocx')"
        >{{ converting ? '⏳' : $t('preview.toDocx') }}</button>
        <button @click="handleClose"
          class="w-6 h-6 flex items-center justify-center rounded-md transition-colors hover:bg-[var(--bg-hover)] shrink-0"
          style="color: var(--text-muted)"
          :title="$t('file.closePreview')"
        >&times;</button>
      </div>

      <!-- Body -->
      <div class="panel-content">
        <div v-if="loading" class="flex-1 flex items-center justify-center" style="color: var(--text-muted)">
          <span class="text-xs">{{ $t('chat.loading') }}</span>
        </div>
        <div v-else-if="error" class="flex-1 flex items-center justify-center p-6 text-sm" style="color: var(--coral)">{{ error }}</div>
        <div v-else-if="fileKind === 'unsupported'" class="flex-1 flex flex-col items-center justify-center gap-3">
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" style="color: var(--text-muted); opacity: 0.4"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/></svg>
          <p class="text-sm" style="color: var(--text-muted)">{{ $t('preview.unsupported') }}</p>
        </div>
        <div v-else-if="activeTab === 'edit'" ref="editorContainer" class="flex-1 overflow-auto"></div>
        <div v-else-if="fileKind === 'image'" class="flex-1 flex items-center justify-center p-4 overflow-auto">
          <img v-if="imageSrc" :src="imageSrc" :alt="file.name" class="max-w-full max-h-full object-contain rounded" />
        </div>
        <div v-else-if="fileKind === 'html'" class="flex-1 flex flex-col" style="min-height: 0">
          <!-- 宽度预设工具栏 -->
          <div class="flex items-center gap-1 px-2 h-7 text-[10px] shrink-0" style="background: var(--bg-elevated); border-bottom: 1px solid var(--border-dim)">
            <button
              v-for="w in HTML_PRESETS" :key="w"
              @click="htmlWidth = w"
              class="px-1.5 py-0.5 rounded transition-colors font-medium"
              :style="{
                background: htmlWidth === w ? 'var(--accent)' : 'transparent',
                color: htmlWidth === w ? 'var(--bg-root)' : 'var(--text-muted)',
                border: htmlWidth === w ? 'none' : '1px solid var(--border-dim)',
              }"
            >{{ w === 0 ? $t('preview.htmlFit') : w }}</button>
          </div>
          <div class="flex-1" :style="{ minHeight: 0, overflow: htmlWidth > 0 ? 'auto' : 'hidden' }">
            <div :class="htmlWidth === 0 ? 'relative' : ''" :style="htmlWidth > 0 ? { width: htmlWidth + 'px', height: '100%' } : { height: '100%' }">
              <iframe
                :class="htmlWidth === 0 ? 'absolute inset-0 w-full h-full border-none' : 'h-full border-none'"
                ref="previewIframe"
                sandbox="allow-scripts"
                :src="previewHtmlBlob"
                :style="{ background: '#fff', width: htmlWidth > 0 ? htmlWidth + 'px' : '' }"
              />
            </div>
          </div>
        </div>
        <div v-else-if="fileKind === 'markdown'" class="flex-1 overflow-auto p-5">
          <MarkdownRenderer :content="content" />
        </div>
        <div v-else-if="fileKind === 'docx'" class="flex-1 overflow-auto p-5 docx-preview" v-html="previewHtml"></div>
        <div v-else-if="hasEdit" ref="editorContainer" class="flex-1 overflow-auto"></div>
        <div v-else class="flex-1 flex items-center justify-center" style="color: var(--text-muted)">
          <span class="text-xs">{{ $t('preview.noPreview') }}</span>
        </div>
      </div>

      <!-- MD 选中修改建议 Tip（浮动在选区旁） -->
      <Teleport to="body">
        <div
          v-if="mdSelection && fileKind === 'markdown' && activeTab === 'edit'"
          class="fixed z-[60] rounded-lg shadow-lg border flex flex-col gap-2 px-3 py-2.5 text-[11px]"
          :style="{
            left: mdTipPos.left,
            top: mdTipPos.top,
            background: 'var(--bg-elevated)',
            borderColor: 'var(--accent-dim)',
            minWidth: '320px',
          }"
        >
          <div class="flex items-center gap-2">
            <span style="color: var(--amber)">📝</span>
            <span class="truncate" style="color: var(--text-secondary)">
              "{{ mdSelection.text.slice(0, 50) }}{{ mdSelection.text.length > 50 ? '…' : '' }}"
            </span>
            <button @click="mdSelection = null; mdSuggestion = ''" class="shrink-0 w-5 h-5 flex items-center justify-center rounded hover:bg-[var(--bg-hover)] ml-auto" style="color: var(--text-muted)">×</button>
          </div>
          <div class="flex gap-2">
            <input
              v-model="mdSuggestion"
              @keydown.enter="sendMdToChat"
              :placeholder="$t('preview.mdSuggestionPlaceholder')"
              class="flex-1 text-[11px] bg-transparent border rounded px-2 py-1 outline-none"
              :style="{ borderColor: 'var(--border-default)', color: 'var(--text-primary)' }"
            />
            <button
              @click="sendMdToChat"
              class="shrink-0 px-3 py-1 rounded text-[10px] font-medium transition-colors hover:opacity-80"
              style="background: var(--accent); color: var(--bg-root)"
            >{{ $t('preview.sendToChat') }}</button>
          </div>
        </div>
      </Teleport>
    </div>
  </div>

  <!-- DOM 选中浮动 Tip（HTML 预览中点击元素后显示，modal/panel 共用） -->
  <Teleport to="body">
    <div
      v-if="selectedDom"
      class="fm-dom-tip"
      :style="domTipStyle()"
    >
      <span class="font-mono truncate" style="color: var(--accent)">
        &lt;{{ selectedDom.tag }}<template v-if="selectedDom.id">#{{ selectedDom.id }}</template><template v-if="selectedDom.classes">.{{ selectedDom.classes.split(' ').join('.') }}</template>&gt;
      </span>
      <span class="truncate flex-1" style="color: var(--text-muted)">{{ selectedDom.text.slice(0, 50) }}</span>
      <button @click="selectedDom = null" class="shrink-0 w-5 h-5 flex items-center justify-center rounded hover:bg-[var(--bg-hover)]" style="color: var(--text-muted)">×</button>
      <button @click.stop="sendDomToChat" class="shrink-0 px-2 py-0.5 rounded text-[10px] font-medium hover:opacity-80" style="background: var(--accent); color: var(--bg-root)">发送到对话</button>
    </div>
  </Teleport>
</template>

<style>
/* ── mammoth .docx → HTML 预览样式 ── */
.docx-preview {
  color: var(--text-primary);
  line-height: 1.7;
  font-size: 14px;
}
.docx-preview h1 {
  font-size: 1.4em;
  font-weight: 700;
  margin: 1em 0 0.5em;
  color: var(--text-bright);
}
.docx-preview h2 { font-size: 1.2em; font-weight: 600; margin: 0.8em 0 0.4em; }
.docx-preview p { margin: 0.4em 0; }
.docx-preview strong { font-weight: 600; color: var(--text-bright); }
.docx-preview table {
  border-collapse: collapse;
  width: 100%;
  margin: 0.8em 0;
}
.docx-preview td, .docx-preview th {
  border: 1px solid var(--border-dim);
  padding: 0.5em 0.75em;
  text-align: left;
}
.docx-preview tr:nth-child(even) { background: var(--bg-hover); }
.docx-preview tr:first-child { background: var(--bg-elevated); font-weight: 600; }

/* ── DOM 选中浮动 Tip ── */
.fm-dom-tip {
  position: fixed;
  z-index: 60;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  font-size: 11px;
  border-radius: 0.5rem;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  background: var(--bg-elevated);
  border: 1px solid var(--accent-dim);
  min-width: 280px;
}

/* ── 面板布局 ── */
.panel-drag-handle {
  width: 0.375rem;
  flex-shrink: 0;
  cursor: col-resize;
  transition: background-color 150ms;
  user-select: none;
  background: transparent;
}
.panel-drag-handle:hover { background: rgba(6, 214, 160, 0.1); }
.panel-drag-handle--active { background: var(--accent-dim); }

.panel-body {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-surface);
  border-left: 1px solid var(--border-dim);
}

.panel-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  flex-shrink: 0;
  border-bottom: 1px solid var(--border-dim);
}

.panel-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-root);
  min-height: 0;
}

/* CodeMirror 填满容器高度，防止水平滚动条被内容撑到可视区外 */
.panel-content .cm-editor { height: 100%; }
.panel-content .cm-scroller { overflow: auto; }
</style>
