<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted } from "vue";

import { readFileContent, readFileBase64, getClaudeDir } from "@/lib/tauri-bridge";
import { highlightCode } from "@/composables/useHighlight";
import { isImageFile, mimeType } from "@/composables/useFilePreview";
import MarkdownRenderer from "./MarkdownRenderer.vue";
import mammoth from "mammoth";
import DOMPurify from "dompurify";
import { emitChatCommand } from "@/composables/useCommandPalette";

const props = defineProps<{ file: { name: string; path: string } | null }>();
const emit = defineEmits<{ close: [] }>();

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
interface DomInfo { tag: string; id: string; classes: string; text: string; attrs: Record<string, string> }
const selectedDom = ref<DomInfo | null>(null);

function onIframeMessage(e: MessageEvent) {
  if (e.data?.type !== "dom-selected") return;
  selectedDom.value = e.data.info as DomInfo;
}
onMounted(() => window.addEventListener("message", onIframeMessage));
onUnmounted(() => window.removeEventListener("message", onIframeMessage));

/** 检查全局 docx skill 是否已安装（~/.claude/skills/docx/SKILL.md） */
async function checkDocxSkill(): Promise<boolean> {
  try {
    const dir = await getClaudeDir();
    const skillPath = dir.replace(/\.claude[\\/]?$/, "") + "skills/docx/SKILL.md";
    await readFileContent(skillPath);
    return true;
  } catch {
    return false;
  }
}

/** MD 文件 → 发送转 docx 指令（无 skill 则先安装） */
async function sendConvertDocx() {
  if (!props.file) return;
  const hasSkill = await checkDocxSkill();
  const msg = hasSkill
    ? `将 \`${props.file.path}\` 转换为 docx 格式，输出到原文件同级位置`
    : `请先全局安装 docx skill：npx skills add https://github.com/anthropics/skills --skill docx，然后执行：将 \`${props.file.path}\` 转换为 docx 格式，输出到原文件同级位置`;
  emitChatCommand(`md-convert:${msg}`);
  emit("close");
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
  emit("close");
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
window.parent.postMessage({type:'dom-selected',info:{
tag:el.tagName.toLowerCase(),
id:el.id||'',
classes:Array.from(el.classList).join(' '),
text:(el.textContent||'').trim().slice(0,300),
attrs:a
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

// ── 交互 ──

function onBackdrop(e: MouseEvent) {
  if (e.target === e.currentTarget) emit("close");
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
}

/** 编辑 tab 的语法高亮内容 */
const highlightedContent = computed(() => {
  if (!props.file || !content.value) return "";
  const lang = extToLang(props.file.name);
  return lang ? highlightCode(content.value, lang) : content.value;
});

// ── extToLang（语法高亮用）──
function extToLang(filename: string): string {
  const ext = filename.split(".").pop()?.toLowerCase() || "";
  const map: Record<string, string> = {
    js: "javascript", ts: "typescript", tsx: "typescript", jsx: "javascript",
    py: "python", rs: "rust", go: "go", java: "java", kt: "kotlin",
    c: "c", h: "c", cpp: "cpp", hpp: "cpp", cs: "csharp",
    rb: "ruby", php: "php", swift: "swift", r: "r", lua: "lua",
    sh: "bash", bash: "bash", zsh: "bash",
    ps1: "powershell", sql: "sql",
    html: "xml", htm: "xml", xml: "xml", svg: "xml",
    css: "css", scss: "css", less: "css",
    json: "json", yaml: "yaml", yml: "yaml",
    toml: "ini", ini: "ini", cfg: "ini", conf: "ini",
    md: "markdown", mdx: "markdown",
    dockerfile: "dockerfile", makefile: "makefile",
    vue: "xml", svelte: "xml", graphql: "graphql", gql: "graphql",
  };
  return map[ext] || "";
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="file"
      class="fixed inset-0 z-50 flex items-center justify-center"
      style="background: rgba(0,0,0,0.6); backdrop-filter: blur(4px)"
      @click="onBackdrop"
      @keydown="onKeydown"
      tabindex="0"
    >
      <div
        class="rounded-xl overflow-hidden flex flex-col shadow-2xl"
        style="background: var(--bg-surface); border: 1px solid var(--border-default); width: 92vw; max-height: 88vh"
      >
        <!-- Header：文件名 + Tab 切换 + 关闭 -->
        <div class="flex items-center px-4 py-2.5 shrink-0 gap-3" style="border-bottom: 1px solid var(--border-dim)">
          <!-- 文件名 -->
          <div class="flex items-center gap-2 min-w-0 flex-1">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2" stroke-linecap="round"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/></svg>
            <span class="text-sm font-medium truncate" style="color: var(--text-bright)" :title="file.path">{{ file.name }}</span>
          </div>
          <!-- Tab 切换 -->
          <div v-if="hasEdit && hasPreview" class="flex rounded-md shrink-0" style="background: var(--bg-root); border: 1px solid var(--border-dim)">
            <button
              v-for="t in (['edit', 'preview'] as const)"
              :key="t"
              @click="activeTab = t"
              class="text-[11px] px-3 py-1 font-medium transition-colors rounded-md"
              :style="{
                background: activeTab === t ? 'var(--accent)' : 'transparent',
                color: activeTab === t ? 'var(--bg-root)' : 'var(--text-muted)',
              }"
            >{{ t === 'edit' ? '编辑' : '预览' }}</button>
          </div>
          <!-- MD → docx -->
          <button
            v-if="fileKind === 'markdown'"
            @click="sendConvertDocx"
            class="text-[11px] px-2.5 py-1 rounded font-medium transition-colors hover:opacity-80 shrink-0"
            style="background: var(--accent); color: var(--bg-root)"
            title="发送转换指令到 CC"
          >转 docx</button>
          <!-- 关闭 -->
          <button
            @click="emit('close')"
            class="w-7 h-7 flex items-center justify-center rounded-md transition-colors hover:bg-[var(--bg-hover)] shrink-0"
            style="color: var(--text-muted)"
          >&times;</button>
        </div>

        <!-- Body -->
        <div class="flex flex-col" style="background: var(--bg-root); height: calc(88vh - 48px)">
          <!-- Loading -->
          <div v-if="loading" class="flex-1 flex items-center justify-center" style="color: var(--text-muted)">
            <span class="text-xs">加载中…</span>
          </div>

          <!-- Error -->
          <div v-else-if="error" class="flex-1 flex items-center justify-center p-6 text-sm" style="color: var(--coral)">{{ error }}</div>

          <!-- Unsupported -->
          <div v-else-if="fileKind === 'unsupported'" class="flex-1 flex flex-col items-center justify-center gap-3">
            <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" style="color: var(--text-muted); opacity: 0.4"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/></svg>
            <p class="text-sm" style="color: var(--text-muted)">不支持预览此文件类型</p>
          </div>

          <!-- ── 编辑 tab：语法高亮代码（原设计）── -->
          <div v-else-if="activeTab === 'edit'" class="flex-1 overflow-auto p-4">
            <pre class="text-xs leading-relaxed font-mono rounded-lg overflow-x-auto" style="background: transparent; padding: 0"><code class="hljs" v-html="highlightedContent"></code></pre>
          </div>

          <!-- ── 预览 tab ── -->

          <!-- 图片预览 -->
          <div v-else-if="fileKind === 'image'" class="flex-1 flex items-center justify-center p-4 overflow-auto">
            <img v-if="imageSrc" :src="imageSrc" :alt="file.name" class="max-w-full max-h-full object-contain rounded" />
          </div>

          <!-- HTML 预览 -->
          <div v-else-if="fileKind === 'html'" class="flex-1 flex flex-col" style="min-height: 0">
            <div class="flex-1 relative" style="min-height: 0">
              <iframe
                class="absolute inset-0 w-full h-full border-none"
                sandbox="allow-scripts"
                :src="previewHtmlBlob"
                style="background: #fff"
              />
            </div>
            <!-- 选中 DOM 信息条 -->
            <div
              v-if="selectedDom"
              class="shrink-0 flex items-center gap-2 px-3 py-2 text-[11px]"
              style="background: var(--bg-elevated); border-top: 1px solid var(--border-dim)"
            >
              <span class="font-mono" style="color: var(--accent)">
                &lt;{{ selectedDom.tag }}<template v-if="selectedDom.id">#{{ selectedDom.id }}</template><template v-if="selectedDom.classes">.{{ selectedDom.classes.split(' ').join('.') }}</template>&gt;
              </span>
              <span class="truncate flex-1" style="color: var(--text-muted)">{{ selectedDom.text }}</span>
              <button @click="selectedDom = null" class="shrink-0 w-5 h-5 flex items-center justify-center rounded hover:bg-[var(--bg-hover)]" style="color: var(--text-muted)">×</button>
              <button @click.stop="sendDomToChat" class="shrink-0 px-2 py-0.5 rounded text-[10px] font-medium hover:opacity-80" style="background: var(--accent); color: var(--bg-root)">发送到对话</button>
            </div>
          </div>

          <!-- Markdown 预览 -->
          <div v-else-if="fileKind === 'markdown'" class="flex-1 overflow-auto p-5">
            <MarkdownRenderer :content="content" />
          </div>

          <!-- Word 预览（mammoth HTML + CSS） -->
          <div v-else-if="fileKind === 'docx'" class="flex-1 overflow-auto p-5 docx-preview" v-html="previewHtml"></div>

          <!-- 兜底：预览 tab 不可用 → 显示编辑内容 -->
          <div v-else class="flex-1 overflow-auto p-4">
            <pre class="text-xs leading-relaxed font-mono rounded-lg overflow-x-auto" style="background: transparent; padding: 0"><code class="hljs" v-html="highlightedContent"></code></pre>
          </div>
        </div>
      </div>
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
</style>
