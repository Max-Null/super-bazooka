<script setup lang="ts">
import { ref, watch } from "vue";

import { readFileContent, readFileBase64 } from "@/lib/tauri-bridge";
import { highlightCode } from "@/composables/useHighlight";
import { isImageFile, mimeType } from "@/composables/useFilePreview";
import MarkdownRenderer from "./MarkdownRenderer.vue";



const props = defineProps<{ file: { name: string; path: string } | null }>();
const emit = defineEmits<{ close: [] }>();

const content = ref("");
const loading = ref(false);
const error = ref("");
const previewType = ref<"image" | "code" | "markdown" | "text" | "unsupported">("text");

const imageSrc = ref("");

// Binary / unsupported extensions
const binaryExts = [
  "exe", "dll", "so", "dylib", "bin", "dat", "db", "sqlite", "sqlite3",
  "xlsx", "xls", "docx", "doc", "pptx", "ppt", "pdf",
  "zip", "tar", "gz", "rar", "7z", "bz2", "xz",
  "mp3", "mp4", "avi", "mov", "mkv", "wav", "flac",
  "ttf", "otf", "woff", "woff2", "eot",
  "class", "pyc", "o", "obj", "lib", "a",
  "wasm", "dex", "apk", "ipa",
];

// Extension → highlight.js language
function extToLang(filename: string): string {
  const ext = filename.split(".").pop()?.toLowerCase() || "";
  const map: Record<string, string> = {
    js: "javascript", mjs: "javascript", cjs: "javascript",
    ts: "typescript", mts: "typescript", cts: "typescript", tsx: "typescript",
    jsx: "javascript",
    py: "python", pyw: "python",
    rs: "rust",
    go: "go",
    java: "java", kt: "kotlin", scala: "scala",
    c: "c", h: "c", cpp: "cpp", hpp: "cpp", cc: "cpp", cxx: "cpp",
    cs: "csharp",
    rb: "ruby",
    php: "php",
    swift: "swift",
    r: "r",
    lua: "lua",
    sh: "bash", bash: "bash", zsh: "bash", fish: "bash",
    ps1: "powershell",
    sql: "sql",
    html: "xml", htm: "xml", xml: "xml", svg: "xml",
    css: "css", scss: "css", sass: "css", less: "css",
    json: "json", jsonc: "json",
    yaml: "yaml", yml: "yaml",
    toml: "ini", ini: "ini", cfg: "ini", conf: "ini",
    md: "markdown", mdx: "markdown", markdown: "markdown",
    dockerfile: "dockerfile",
    makefile: "makefile",
    env: "bash",
    vue: "xml", svelte: "xml",
    graphql: "graphql", gql: "graphql",
  };
  return map[ext] || "";
}

function detectPreviewType(filename: string) {
  const ext = filename.split(".").pop()?.toLowerCase() || "";
  if (isImageFile(filename)) return "image";
  if (binaryExts.includes(ext)) return "unsupported";
  if (["md", "mdx", "markdown"].includes(ext)) return "markdown";
  const lang = extToLang(filename);
  return lang ? "code" : "text";
}

watch(() => props.file, async (f) => {
  if (!f) return;
  content.value = "";
  error.value = "";
  imageSrc.value = "";

  const type = detectPreviewType(f.name);
  previewType.value = type;

  if (type === "image") {
    loading.value = true;
    try {
      const b64 = await readFileBase64(f.path);
      imageSrc.value = `data:${mimeType(f.name)};base64,${b64}`;
    } catch (e) {
      error.value = String(e);
    }
    loading.value = false;
    return;
  }

  if (type === "unsupported") {
    return; // Show unsupported message
  }

  // Read as text for code, markdown, or plain text
  loading.value = true;
  try {
    const raw = await readFileContent(f.path);
    if (type === "code") {
      const lang = extToLang(f.name);
      content.value = highlightCode(raw, lang);
    } else {
      content.value = raw;
    }
  } catch (e) {
    error.value = String(e);
  }
  loading.value = false;
}, { immediate: true });

function onBackdrop(e: MouseEvent) {
  if (e.target === e.currentTarget) emit("close");
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
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
        class="rounded-xl overflow-hidden max-w-3xl max-h-[85vh] flex flex-col shadow-2xl"
        style="background: var(--bg-surface); border: 1px solid var(--border-default); width: 90vw"
      >
        <!-- Header -->
        <div class="flex items-center justify-between px-4 py-3 shrink-0" style="border-bottom: 1px solid var(--border-dim)">
          <div class="flex items-center gap-2.5 min-w-0">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2" stroke-linecap="round"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/></svg>
            <span class="text-sm font-medium truncate" style="color: var(--text-bright)" :title="file.path">{{ file.name }}</span>
            <span class="text-[10px] px-1.5 py-px rounded shrink-0" style="background: var(--bg-hover); color: var(--text-muted)">{{ previewType }}</span>
          </div>
          <button
            @click="emit('close')"
            class="w-7 h-7 flex items-center justify-center rounded-md transition-colors hover:bg-[var(--bg-hover)] shrink-0"
            style="color: var(--text-muted)"
          >&times;</button>
        </div>

        <!-- Body -->
        <div class="flex-1 overflow-y-auto" style="background: var(--bg-root)">
          <!-- Image -->
          <div v-if="previewType === 'image' && !error" class="flex items-center justify-center p-4" style="min-height: 200px">
            <img v-if="imageSrc" :src="imageSrc" :alt="file.name" class="max-w-full max-h-[70vh] object-contain rounded" />
          </div>

          <!-- Loading -->
          <div v-else-if="loading" class="flex items-center justify-center p-12" style="color: var(--text-muted)">
            <span class="text-xs">{{ $t('preview.loading') }}</span>
          </div>

          <!-- Error -->
          <div v-else-if="error" class="p-6 text-sm" style="color: var(--coral)">
            {{ error }}
          </div>

          <!-- Unsupported -->
          <div v-else-if="previewType === 'unsupported'" class="flex flex-col items-center justify-center p-12 gap-3">
            <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" style="color: var(--text-muted); opacity: 0.4"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/></svg>
            <p class="text-sm" style="color: var(--text-muted)">{{ $t('preview.unsupported') }}</p>
            <p class="text-xs" style="color: var(--text-muted); opacity: 0.6">{{ $t('preview.unsupportedHint') }}</p>
          </div>

          <!-- Markdown -->
          <div v-else-if="previewType === 'markdown'" class="p-5">
            <MarkdownRenderer :content="content" />
          </div>

          <!-- Code (syntax highlighted) -->
          <div v-else-if="previewType === 'code'" class="p-4">
            <pre class="text-xs leading-relaxed font-mono rounded-lg overflow-x-auto" style="background: transparent; padding: 0"><code class="hljs" v-html="content"></code></pre>
          </div>

          <!-- Plain text -->
          <div v-else class="p-4">
            <pre class="text-xs leading-relaxed whitespace-pre-wrap font-mono" style="color: var(--text-primary); max-height: 65vh"><code>{{ content }}</code></pre>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
