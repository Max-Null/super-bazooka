<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, shallowRef } from "vue";
import { EditorView, keymap, lineNumbers, highlightActiveLine } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
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

const props = defineProps<{ content: string; filename: string }>();

const container = ref<HTMLElement | null>(null);
const view = shallowRef<EditorView | null>(null);

function langFromFilename(name: string) {
  const ext = name.split(".").pop()?.toLowerCase();
  switch (ext) {
    case "ts": case "tsx": case "js": case "jsx": return javascript();
    case "py": return python();
    case "rs": return rust();
    case "css": return css();
    case "html": case "vue": case "svelte": return html();
    case "json": return json();
    case "md": return markdown();
    case "sql": return sql();
    case "xml": case "svg": return xml();
    case "yaml": case "yml": return yaml();
    default: return null;
  }
}

onMounted(() => {
  if (!container.value) return;
  const extensions = [
    lineNumbers(),
    highlightActiveLine(),
    history(),
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    oneDark,
    keymap.of([...defaultKeymap, ...historyKeymap]),
    EditorState.readOnly.of(true),
    EditorView.editable.of(false),
  ];
  const lang = langFromFilename(props.filename);
  if (lang) extensions.push(lang);

  view.value = new EditorView({
    state: EditorState.create({ doc: props.content, extensions }),
    parent: container.value,
  });
});

watch(() => props.content, (val) => {
  if (!view.value) return;
  view.value.dispatch({
    changes: { from: 0, to: view.value.state.doc.length, insert: val },
  });
});

onUnmounted(() => {
  view.value?.destroy();
});
</script>

<template>
  <div ref="container" class="text-xs"></div>
</template>
