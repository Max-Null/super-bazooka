<script setup lang="ts">
import { computed, watch, ref, nextTick } from "vue";
import { useHighlight } from "@/composables/useHighlight";
import MermaidRenderer from "./MermaidRenderer.vue";

const props = defineProps<{ content: string }>();
const { highlight } = useHighlight();
const container = ref<HTMLElement | null>(null);

interface Block {
  type: "markdown" | "mermaid";
  content: string;
}

function parseBlocks(text: string): Block[] {
  const blocks: Block[] = [];
  const regex = /```mermaid\n([\s\S]*?)```/g;
  let lastIdx = 0;
  let match: RegExpExecArray | null;

  while ((match = regex.exec(text)) !== null) {
    if (match.index > lastIdx) {
      blocks.push({ type: "markdown", content: text.slice(lastIdx, match.index) });
    }
    blocks.push({ type: "mermaid", content: match[1].trim() });
    lastIdx = match.index + match[0].length;
  }
  if (lastIdx < text.length) {
    blocks.push({ type: "markdown", content: text.slice(lastIdx) });
  }
  return blocks.length > 0 ? blocks : [{ type: "markdown", content: text }];
}

function renderMarkdown(text: string): string {
  let html = text
    .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

  const lines = html.split("\n");
  const output: string[] = [];
  let i = 0;

  function collectLines(start: number, pred: (line: string) => string | null): { html: string; end: number } {
    const items: string[] = [];
    let j = start;
    while (j < lines.length) {
      const result = pred(lines[j]);
      if (result === null) break;
      items.push(result);
      j++;
    }
    return { html: items.join("\n"), end: j };
  }

  while (i < lines.length) {
    if (/^```/.test(lines[i])) {
      const lang = lines[i].slice(3).trim();
      const codeLines: string[] = [];
      i++;
      while (i < lines.length && !/^```/.test(lines[i])) {
        codeLines.push(lines[i]);
        i++;
      }
      output.push(`<pre><code class="language-${lang || 'plaintext'}">${codeLines.join("\n")}</code></pre>`);
      i++;
      continue;
    }

    if (lines[i].includes("|") && i + 1 < lines.length &&
        /^\|?[\s:-]+\|[\s|:-]+\|?$/.test(lines[i + 1])) {
      const tableLines: string[] = [lines[i], lines[i + 1]];
      i += 2;
      while (i < lines.length && lines[i].includes("|")) { tableLines.push(lines[i]); i++; }
      const tableHTML = parseTable(tableLines);
      if (tableHTML) { output.push(tableHTML); continue; }
    }

    if (/^### (.+)$/.test(lines[i])) output.push(lines[i].replace(/^### (.+)$/, '<h3>$1</h3>'));
    else if (/^## (.+)$/.test(lines[i])) output.push(lines[i].replace(/^## (.+)$/, '<h2>$1</h2>'));
    else if (/^# (.+)$/.test(lines[i])) output.push(lines[i].replace(/^# (.+)$/, '<h1>$1</h1>'));
    else if (/^---$/.test(lines[i])) output.push('<hr>');
    else if (/^&gt; (.+)$/.test(lines[i])) output.push(lines[i].replace(/^&gt; (.+)$/, '<blockquote>$1</blockquote>'));
    else if (/^- (.+)$/.test(lines[i])) {
      const result = collectLines(i, (line) => { const m = line.match(/^- (.+)$/); return m ? `<li>${m[1]}</li>` : null; });
      output.push(`<ul>${result.html}</ul>`);
      i = result.end; continue;
    }
    else if (/^\d+\. (.+)$/.test(lines[i])) {
      const result = collectLines(i, (line) => { const m = line.match(/^\d+\. (.+)$/); return m ? `<li>${m[1]}</li>` : null; });
      output.push(`<ol>${result.html}</ol>`);
      i = result.end; continue;
    }
    else if (lines[i].trim() === "") output.push('');
    else output.push(lines[i]);
    i++;
  }

  html = output.join("\n");

  const preBlocks: string[] = [];
  html = html.replace(/<pre[\s\S]*?<\/pre>/g, (m) => { preBlocks.push(m); return `￿PRE${preBlocks.length - 1}￿`; });

  html = html.replace(/`([^`]+)`/g, '<code>$1</code>');
  html = html.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>");
  html = html.replace(/\*(.+?)\*/g, "<em>$1</em>");
  html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>');

  html = html.replace(/￿PRE(\d+)￿/g, (_, i) => preBlocks[Number(i)] || "");

  const parts = html.split("\n");
  const wrapped: string[] = [];
  for (const part of parts) {
    const t = part.trim();
    if (t === "") wrapped.push('');
    else if (/^<(h[1-6]|table|thead|tbody|tr|th|td|pre|ul|ol|blockquote|hr|li)/.test(t)) wrapped.push(t);
    else wrapped.push(`<p>${t}</p>`);
  }
  html = wrapped.join("\n");
  html = html.replace(/<p><\/p>/g, "");

  return html;
}

function parseTable(lines: string[]): string | null {
  if (lines.length < 2 || !lines[0].includes("|")) return null;
  const sep = lines[1];
  if (!/^\|?[\s:-]+\|[\s|:-]+\|?$/.test(sep)) return null;
  const alignments: string[] = [];
  for (const cell of sep.split("|").filter(c => c.trim())) {
    const t = cell.trim();
    if (t.startsWith(":") && t.endsWith(":")) alignments.push("center");
    else if (t.endsWith(":")) alignments.push("right");
    else alignments.push("left");
  }
  function rowToHTML(row: string, tag: "th" | "td"): string {
    const cells = row.split("|").filter((_, i, a) => i > 0 || a[0].trim() !== "").filter((_, i, a) => i < a.length - 1 || _.trim() !== "");
    return "<tr>" + cells.map((c, i) => `<${tag} style="text-align:${alignments[i] || "left"}">${c.trim()}</${tag}>`).join("") + "</tr>";
  }
  const header = rowToHTML(lines[0], "th");
  const body = lines.slice(2).map(r => rowToHTML(r, "td")).join("");
  return `<table><thead>${header}</thead><tbody>${body}</tbody></table>`;
}

const blocks = computed(() => parseBlocks(props.content));
const markdownBlocks = computed(() => blocks.value.filter(b => b.type === "markdown").map(b => renderMarkdown(b.content)));
const mermaidBlocks = computed(() => blocks.value.filter(b => b.type === "mermaid").map(b => b.content));

watch(() => props.content, async () => {
  await nextTick();
  highlight(container.value);
});
</script>

<template>
  <div ref="container" class="markdown-body">
    <template v-for="(block, idx) in blocks" :key="idx">
      <div v-if="block.type === 'markdown'" v-html="markdownBlocks[markdownBlocks.findIndex((_, i) => i <= idx)] || renderMarkdown(block.content)"></div>
      <MermaidRenderer v-else :code="block.content" />
    </template>
  </div>
</template>

<style scoped>
.markdown-body :deep(p) { margin: 0.25em 0; line-height: 1.65; }
.markdown-body :deep(p:first-child) { margin-top: 0; }
.markdown-body :deep(p:last-child) { margin-bottom: 0; }
.markdown-body :deep(h1) { font-size: 1.25em; font-weight: 700; margin: 0.75em 0 0.4em; color: var(--text-bright); }
.markdown-body :deep(h2) { font-size: 1.1em; font-weight: 600; margin: 0.65em 0 0.3em; color: var(--text-bright); }
.markdown-body :deep(h3) { font-size: 1em; font-weight: 600; margin: 0.5em 0 0.25em; color: var(--text-bright); }
.markdown-body :deep(strong) { color: var(--text-bright); font-weight: 600; }
.markdown-body :deep(em) { color: var(--text-secondary); }
.markdown-body :deep(ul), .markdown-body :deep(ol) { margin: 0.35em 0; padding-left: 1.5em; }
.markdown-body :deep(li) { margin: 0.12em 0; line-height: 1.55; }
.markdown-body :deep(ul) { list-style-type: disc; }
.markdown-body :deep(ol) { list-style-type: decimal; }
.markdown-body :deep(blockquote) { margin: 0.5em 0; padding: 0.3em 1em; border-left: 2px solid var(--border-bright); color: var(--text-secondary); font-style: italic; }
.markdown-body :deep(code) { font-family: var(--font-mono); font-size: 0.85em; background: var(--bg-hover); padding: 0.12em 0.35em; border-radius: 3px; }
.markdown-body :deep(pre) { margin: 0.5em 0; }
.markdown-body :deep(pre code) { background: transparent; padding: 0; }
.markdown-body :deep(hr) { margin: 0.75em 0; border: none; border-top: 1px solid var(--border-dim); }
.markdown-body :deep(table) { width: 100%; border-collapse: collapse; margin: 0.5em 0; font-size: 0.9em; }
.markdown-body :deep(th) { background: var(--bg-hover); text-align: left; padding: 0.4em 0.8em; border: 1px solid var(--border-dim); font-weight: 600; color: var(--text-bright); }
.markdown-body :deep(td) { padding: 0.3em 0.8em; border: 1px solid var(--border-dim); color: var(--text-primary); }
.markdown-body :deep(a) { color: var(--accent); text-decoration: none; }
.markdown-body :deep(a:hover) { text-decoration: underline; }
</style>
