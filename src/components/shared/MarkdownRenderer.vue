<script setup lang="ts">
import { computed, watch, ref, nextTick } from "vue";
import { useHighlight } from "@/composables/useHighlight";

const props = defineProps<{ content: string }>();
const { highlight } = useHighlight();
const container = ref<HTMLElement | null>(null);

function parseTable(lines: string[]): string | null {
  // A markdown table needs at least 2 lines: header + separator
  if (lines.length < 2) return null;
  if (!lines[0].includes('|')) return null;

  // Check separator line: |---|---|
  const sep = lines[1];
  if (!/^\|?[\s:-]+\|[\s|:-]+\|?$/.test(sep)) return null;

  // Parse alignments from separator
  const alignments: string[] = [];
  for (const cell of sep.split('|').filter(c => c.trim())) {
    const trimmed = cell.trim();
    const left = trimmed.startsWith(':');
    const right = trimmed.endsWith(':');
    if (left && right) alignments.push('center');
    else if (right) alignments.push('right');
    else alignments.push('left');
  }

  function rowToHTML(row: string, tag: 'th' | 'td'): string {
    const cells = row.split('|').filter((_, i, a) => i > 0 || a[0].trim() !== '').filter((_, i, a) => {
      // Remove trailing empty cell if last
      return i < a.length - 1 || _.trim() !== '';
    });
    return '<tr>' + cells.map((c, i) => {
      const align = alignments[i] ? ` style="text-align:${alignments[i]}"` : '';
      return `<${tag}${align}>${c.trim()}</${tag}>`;
    }).join('') + '</tr>';
  }

  const header = rowToHTML(lines[0], 'th');
  const body = lines.slice(2).map(r => rowToHTML(r, 'td')).join('');
  return `<table><thead>${header}</thead><tbody>${body}</tbody></table>`;
}

function renderMarkdown(text: string): string {
  let html = text
    .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

  // Process line by line for block-level elements
  const lines = html.split('\n');
  const output: string[] = [];
  let i = 0;

  // Helper: collect consecutive lines matching a predicate
  function collectLines(start: number, pred: (line: string) => string | null): { html: string; end: number } {
    const items: string[] = [];
    let j = start;
    while (j < lines.length) {
      const result = pred(lines[j]);
      if (result === null) break;
      items.push(result);
      j++;
    }
    return { html: items.join('\n'), end: j };
  }

  while (i < lines.length) {
    // Fenced code block (multi-line)
    if (/^```/.test(lines[i])) {
      const lang = lines[i].slice(3).trim();
      const codeLines: string[] = [];
      i++;
      while (i < lines.length && !/^```/.test(lines[i])) {
        codeLines.push(lines[i]);
        i++;
      }
      output.push(`<pre><code class="language-${lang || 'plaintext'}">${codeLines.join('\n')}</code></pre>`);
      i++; // skip closing ```
      continue;
    }

    // Table (multi-line): detect header + separator pattern
    if (lines[i].includes('|') && i + 1 < lines.length &&
        /^\|?[\s:-]+\|[\s|:-]+\|?$/.test(lines[i + 1])) {
      const tableLines: string[] = [lines[i], lines[i + 1]];
      i += 2;
      while (i < lines.length && lines[i].includes('|')) {
        tableLines.push(lines[i]);
        i++;
      }
      const tableHTML = parseTable(tableLines);
      if (tableHTML) {
        output.push(tableHTML);
        continue;
      }
    }

    // Headers
    if (/^### (.+)$/.test(lines[i])) {
      output.push(lines[i].replace(/^### (.+)$/, '<h3>$1</h3>'));
    } else if (/^## (.+)$/.test(lines[i])) {
      output.push(lines[i].replace(/^## (.+)$/, '<h2>$1</h2>'));
    } else if (/^# (.+)$/.test(lines[i])) {
      output.push(lines[i].replace(/^# (.+)$/, '<h1>$1</h1>'));
    }
    // HR
    else if (/^---$/.test(lines[i])) {
      output.push('<hr>');
    }
    // Blockquote
    else if (/^&gt; (.+)$/.test(lines[i])) {
      output.push(lines[i].replace(/^&gt; (.+)$/, '<blockquote>$1</blockquote>'));
    }
    // Unordered list (group consecutive items into <ul>)
    else if (/^- (.+)$/.test(lines[i])) {
      const result = collectLines(i, (line) => {
        const m = line.match(/^- (.+)$/);
        return m ? `<li>${m[1]}</li>` : null;
      });
      output.push(`<ul>${result.html}</ul>`);
      i = result.end;
      continue;
    }
    // Ordered list (group consecutive items into <ol>)
    else if (/^\d+\. (.+)$/.test(lines[i])) {
      const result = collectLines(i, (line) => {
        const m = line.match(/^\d+\. (.+)$/);
        return m ? `<li>${m[1]}</li>` : null;
      });
      output.push(`<ol>${result.html}</ol>`);
      i = result.end;
      continue;
    }
    // Empty line
    else if (lines[i].trim() === '') {
      output.push('');
    }
    // Regular text
    else {
      output.push(lines[i]);
    }
    i++;
  }

  html = output.join('\n');

  // Protect <pre> blocks from inline replacements (they may contain backticks, *, etc.)
  const preBlocks: string[] = [];
  html = html.replace(/<pre[\s\S]*?<\/pre>/g, (m) => {
    preBlocks.push(m);
    return `￿PRE${preBlocks.length - 1}￿`;
  });

  // Inline formatting (after block processing)
  html = html.replace(/`([^`]+)`/g, '<code>$1</code>');
  html = html.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>");
  html = html.replace(/\*(.+?)\*/g, "<em>$1</em>");
  // Links
  html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>');

  // Restore protected <pre> blocks
  html = html.replace(/￿PRE(\d+)￿/g, (_, i) => preBlocks[Number(i)] || '');

  // Paragraphs: wrap text blocks in <p>, preserve block elements
  const parts = html.split('\n');
  const wrapped: string[] = [];
  for (const part of parts) {
    const trimmed = part.trim();
    if (trimmed === '') {
      wrapped.push('');
    } else if (/^<(h[1-6]|table|thead|tbody|tr|th|td|pre|ul|ol|blockquote|hr|li)/.test(trimmed)) {
      wrapped.push(trimmed);
    } else {
      wrapped.push(`<p>${trimmed}</p>`);
    }
  }
  html = wrapped.join('\n');
  html = html.replace(/<p><\/p>/g, '');

  return html;
}

const rendered = computed(() => renderMarkdown(props.content));

watch(() => props.content, async () => {
  await nextTick();
  highlight(container.value);
});
</script>

<template>
  <div ref="container" class="markdown-body" v-html="rendered"></div>
</template>

<style scoped>
/* ── Paragraphs ── */
.markdown-body :deep(p) { margin: 0.25em 0; line-height: 1.65; }
.markdown-body :deep(p:first-child) { margin-top: 0; }
.markdown-body :deep(p:last-child) { margin-bottom: 0; }

/* ── Headers ── */
.markdown-body :deep(h1) { font-size: 1.25em; font-weight: 700; margin: 0.75em 0 0.4em; color: var(--text-bright); }
.markdown-body :deep(h2) { font-size: 1.1em; font-weight: 600; margin: 0.65em 0 0.3em; color: var(--text-bright); }
.markdown-body :deep(h3) { font-size: 1em; font-weight: 600; margin: 0.5em 0 0.25em; color: var(--text-bright); }

/* ── Bold / Italic ── */
.markdown-body :deep(strong) { color: var(--text-bright); font-weight: 600; }
.markdown-body :deep(em) { color: var(--text-secondary); }

/* ── Lists ── */
.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  margin: 0.35em 0;
  padding-left: 1.5em;
}
.markdown-body :deep(li) {
  margin: 0.12em 0;
  line-height: 1.55;
}
.markdown-body :deep(ul) { list-style-type: disc; }
.markdown-body :deep(ol) { list-style-type: decimal; }
/* Nested list support */
.markdown-body :deep(li ul),
.markdown-body :deep(li ol) { margin: 0.1em 0; }

/* ── Blockquote ── */
.markdown-body :deep(blockquote) {
  margin: 0.5em 0;
  padding: 0.35em 0.9em;
  border-left: 2.5px solid var(--border-bright);
  color: var(--text-secondary);
  background: var(--bg-elevated);
  border-radius: 0 4px 4px 0;
}
.markdown-body :deep(blockquote p) { margin: 0.15em 0; }

/* ── Horizontal Rule ── */
.markdown-body :deep(hr) {
  margin: 0.75em 0;
  border: none;
  border-top: 1px solid var(--border-dim);
  opacity: 0.6;
}

/* ── Inline Code ── */
.markdown-body :deep(code) {
  font-family: var(--font-mono, 'Consolas', 'Monaco', monospace);
  font-size: 0.85em;
  background: var(--bg-hover);
  padding: 0.12em 0.35em;
  border-radius: 3px;
  color: var(--accent);
}

/* ── Fenced Code Blocks ── */
.markdown-body :deep(pre) {
  margin: 0.5em 0;
  padding: 0.75em 1em;
  background: var(--bg-root);
  border: 1px solid var(--border-dim);
  border-radius: 6px;
  overflow-x: auto;
  line-height: 1.5;
  font-size: 0.85em;
}
.markdown-body :deep(pre code) {
  background: transparent;
  padding: 0;
  border-radius: 0;
  color: var(--text-primary);
  font-size: inherit;
}

/* ── Tables ── */
.markdown-body :deep(table) {
  width: 100%;
  margin: 0.6em 0;
  border-collapse: collapse;
  font-size: 0.875em;
  border-radius: 6px;
  overflow: hidden;
  border: 1px solid var(--border-dim);
}
.markdown-body :deep(th) {
  text-align: left;
  padding: 0.45em 0.75em;
  font-weight: 600;
  color: var(--text-bright);
  background: var(--bg-hover);
  border-bottom: 2px solid var(--border-dim);
  white-space: nowrap;
}
.markdown-body :deep(td) {
  padding: 0.35em 0.75em;
  border-bottom: 1px solid var(--border-dim);
  color: var(--text-primary);
  line-height: 1.5;
}
.markdown-body :deep(tr:last-child td) { border-bottom: none; }
.markdown-body :deep(tr:nth-child(even) td) { background: var(--bg-elevated); }

/* ── Links ── */
.markdown-body :deep(a) {
  color: var(--accent);
  text-decoration: underline;
  text-underline-offset: 2px;
  transition: opacity 0.15s;
}
.markdown-body :deep(a:hover) { opacity: 0.8; }

/* ── Inline formatting inside special blocks ── */
.markdown-body :deep(th strong),
.markdown-body :deep(td strong) { color: var(--text-bright); }
.markdown-body :deep(blockquote strong) { color: var(--text-primary); }
.markdown-body :deep(li strong) { color: var(--text-bright); }

/* ── Streaming cursor ── */
:deep(.stream-cursor) {
  display: inline-block;
  width: 0.5em;
  height: 1em;
  background: var(--accent);
  margin-left: 1px;
  vertical-align: text-bottom;
  animation: blink 0.8s step-end infinite;
}
@keyframes blink { 50% { opacity: 0; } }
</style>
