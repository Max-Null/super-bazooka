<script setup lang="ts">
import { ref, nextTick, watch, onMounted } from "vue";
import { useChatStore } from "@/stores/chat";
import { useSessionStore } from "@/stores/session";
import { useDebugLog } from "@/composables/useDebugLog";
import { open } from "@tauri-apps/plugin-dialog";
import { sendMessage, sendStdin, getAutoModeStatus, stopSession, saveMessage } from "@/lib/tauri-bridge";
import { useFilePreview } from "@/composables/useFilePreview";
import { useSettingsStore } from "@/stores/settings";
import ErrorBoundary from "@/components/shared/ErrorBoundary.vue";
import InputBar from "./InputBar.vue";
import InputBarToolbar from "./InputBarToolbar.vue";
import MessageBubble from "./MessageBubble.vue";
import ThinkingIndicator from "./ThinkingIndicator.vue";
import FilePreviewModal from "@/components/shared/FilePreviewModal.vue";
import { useCommandPaletteBus } from "@/composables/useCommandPalette";

const chat = useChatStore();
const session = useSessionStore();
const settings = useSettingsStore();
const debugLog = useDebugLog();
const scrollContainer = ref<HTMLElement | null>(null);
const exporting = ref(false);
const exportedLabel = ref("");
const commandBus = useCommandPaletteBus();
import { isImageFile } from "@/composables/useFilePreview";
const { getThumbnail, thumbnails } = useFilePreview();

// ── Attached files ──
interface AttachedFile { name: string; path: string }
const attachedFiles = ref<AttachedFile[]>([]);

function removeAttachedFile(index: number) {
  attachedFiles.value.splice(index, 1);
}

const previewFile = ref<{ name: string; path: string } | null>(null);

// ── Auto mode detection ──
// Primary: frontend store (instant UI feedback when user switches)
// Calibration: on mount, verify actual settings.json (catches external modifications)
const autoModeActive = ref(settings.autoMode);

onMounted(async () => {
  try { autoModeActive.value = await getAutoModeStatus(); }
  catch { autoModeActive.value = settings.autoMode; }
});

// Sync on store change
watch(() => settings.autoMode, (v) => { autoModeActive.value = v; });

// ── Sticky question banner ──
const stickyQuestion = ref("");
const showSticky = ref(false);

function onScroll() {
  const container = scrollContainer.value;
  if (!container) return;
  const containerRect = container.getBoundingClientRect();
  const userMsgs = container.querySelectorAll<HTMLElement>('[data-role="user"]');
  let lastAbove = "";
  for (const el of userMsgs) {
    const r = el.getBoundingClientRect();
    if (r.bottom < containerRect.top + 4) {
      lastAbove = el.querySelector('.user-text')?.textContent || "";
    }
  }
  if (lastAbove) {
    stickyQuestion.value = lastAbove.length > 100 ? lastAbove.slice(0, 100) + "…" : lastAbove;
    showSticky.value = true;
  } else {
    showSticky.value = false;
  }
}

let scrollTimer: ReturnType<typeof setTimeout> | null = null;
function onScrollThrottled() {
  if (scrollTimer) return;
  scrollTimer = setTimeout(() => { scrollTimer = null; onScroll(); }, 50);
}

async function handleSend(text: string) {
  debugLog.clear();
  let sid = session.activeSessionId;
  if (!sid) sid = await session.createSession(settings.model);

  // Collect attached file paths before clearing
  const filePaths = attachedFiles.value.map(f => f.path);
  attachedFiles.value = [];

  const attachments = filePaths.length > 0 ? filePaths.map(p => ({ name: p.split(/[/\\]/).pop() || p, path: p })) : undefined;
  const userMsgId = chat.addUserMessage(text, attachments);

  // Save user message to SQLite
  if (attachments) {
    saveMessage(userMsgId, sid, "user", JSON.stringify({ text, attachments }), "{}").catch(() => {});
  } else {
    saveMessage(userMsgId, sid, "user", text, "{}").catch(() => {});
  }

  chat.startAssistantMessage();
  chat.isProcessing = true;
  await scrollToBottom();
  try {
    await sendMessage(sid, text, {
      planMode: settings.planMode,
      autoMode: settings.autoMode,
      permissionMode: settings.permissionMode,
      effort: settings.effort,
      ultracode: settings.effort === "ultracode",
      model: settings.model,
      filePaths: filePaths.length > 0 ? filePaths : undefined,
    });
    session.loadSessions().catch(() => {});
  } catch (err) {
    debugLog.add(`>>> Error: ${err}`);
    debugLog.visible.value = true;
    chat.appendText(`\n\n> ❌ Error: ${err}`);
    chat.finishAssistantMessage();
  }
}

async function handleAllow() {
  const cr = chat.pendingControlRequest; if (!cr) return;
  await sendStdin(session.activeSessionId, JSON.stringify({type:"control_response",response:"allow"}));
  chat.resolveControlRequest("allow");
}
async function handleDeny() {
  const cr = chat.pendingControlRequest; if (!cr) return;
  await sendStdin(session.activeSessionId, JSON.stringify({type:"control_response",response:"deny"}));
  chat.resolveControlRequest("deny");
}

// ── Edit + Resend: truncate subsequent messages, update content, resend ──
async function handleEditSave(id: string, newContent: string) {
  // Update the message content
  chat.updateMessage(id, newContent);
  // Remove all messages after the edited one (they were responses to the old text)
  chat.truncateAfterMessage(id);
  // Resend the edited message
  await handleSend(newContent);
}

async function handleResend(id: string, content: string) {
  // Resend the same message text
  await handleSend(content);
}

// ── Stop processing ──
async function handleStop() {
  if (!session.activeSessionId) return;
  try { await stopSession(session.activeSessionId); } catch {}
  chat.finishAssistantMessage();
}

// ── Session Export ──
async function handleExport() {
  const sid = session.activeSessionId;
  if (!sid || chat.messages.length === 0) return;
  exporting.value = true;
  try {
    const active = session.sessions.find(s => s.id === sid);
    const title = active?.title || "Chat Export";
    const md = chat.exportMarkdown(title);
    const blob = new Blob([md], { type: "text/markdown" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${title.replace(/[^a-zA-Z0-9一-鿿_-]/g, "_")}.md`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    exportedLabel.value = "Exported!";
    setTimeout(() => exportedLabel.value = "", 2000);
  } finally {
    exporting.value = false;
  }
}

// ── Attach file ──
async function handleAttachFile() {
  const selected = await open({
    multiple: true,
    title: "Attach Files",
  });
  if (!selected) return;
  const paths = Array.isArray(selected) ? selected : [selected];
  for (const p of paths) {
    const name = p.split(/[/\\]/).pop() || p;
    if (!attachedFiles.value.some(af => af.path === p)) {
      attachedFiles.value.push({ name, path: p });
    }
  }
}

async function scrollToBottom() {
  await nextTick();
  if (scrollContainer.value) scrollContainer.value.scrollTop = scrollContainer.value.scrollHeight;
}
watch(() => chat.messages.length, () => scrollToBottom());
watch(() => chat.currentAssistantMsg?.content, () => scrollToBottom());
watch(() => chat.currentAssistantMsg?.thinking, () => scrollToBottom());
watch(() => chat.currentAssistantMsg?.toolUses.length, () => scrollToBottom());
</script>

<template>
  <ErrorBoundary name="ChatPanel">
  <div class="flex flex-col flex-1 h-full overflow-hidden">
    <!-- Sticky question banner -->
    <div
      v-if="showSticky"
      class="shrink-0 px-4 py-1.5 text-xs truncate z-10"
      style="background:var(--bg-elevated); border-bottom:1px solid var(--border-dim); color:var(--text-muted); backdrop-filter:blur(8px)"
    >
      <span class="font-medium" style="color:var(--text-secondary)">↳ </span>{{ stickyQuestion }}
    </div>

    <!-- Messages -->
    <div ref="scrollContainer" :class="['flex-1', chat.messages.length > 0 ? 'overflow-y-auto' : '']" @scroll="onScrollThrottled">
      <!-- Welcome -->
      <div v-if="chat.messages.length === 0" class="flex items-center justify-center h-full">
        <div class="text-center max-w-sm px-6 pb-24">
          <!-- Icon: terminal cursor -->
          <div class="mb-6 inline-flex items-center justify-center w-16 h-16 rounded-2xl" style="background:var(--accent-glow)">
            <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="4 17 10 11 4 5" />
              <line x1="12" y1="19" x2="20" y2="19" />
            </svg>
          </div>
          <h2 class="text-xl font-bold mb-2 tracking-tight" style="color:var(--text-bright)">cc-gui</h2>
          <p class="text-sm leading-relaxed mb-6" style="color:var(--text-secondary)">
            Claude Code Desktop GUI
          </p>
          <div class="flex items-center justify-center gap-2 text-[11px]" style="color:var(--text-muted)">
            <kbd class="px-1.5 py-0.5 rounded text-[10px]" style="background:var(--bg-elevated); border:1px solid var(--border-dim)">Enter</kbd>
            <span>send</span>
            <span style="color:var(--border-default)">·</span>
            <kbd class="px-1.5 py-0.5 rounded text-[10px]" style="background:var(--bg-elevated); border:1px solid var(--border-dim)">Shift</kbd>
            <span>+</span>
            <kbd class="px-1.5 py-0.5 rounded text-[10px]" style="background:var(--bg-elevated); border:1px solid var(--border-dim)">Enter</kbd>
            <span>newline</span>
          </div>
        </div>
      </div>

      <!-- Message list -->
      <div class="max-w-3xl mx-auto px-4 py-6 space-y-5">
        <!-- Export bar (when messages exist) -->
        <div v-if="chat.messages.length > 0" class="flex items-center justify-end">
          <button
            @click="handleExport"
            :disabled="exporting"
            class="flex items-center gap-1.5 px-2.5 py-1 rounded-md text-[11px] transition-colors hover:bg-[var(--bg-hover)]"
            :style="{ color: exportedLabel ? 'var(--accent)' : 'var(--text-muted)' }"
            title="Export session as Markdown"
          >
            <svg v-if="!exporting" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
            <span v-if="exportedLabel">{{ exportedLabel }}</span>
            <span v-else>Export</span>
          </button>
        </div>
        <TransitionGroup name="msg">
          <MessageBubble
            v-for="msg in chat.messages"
            :key="msg.id"
            :message="msg"
            @edit-save="handleEditSave"
            @resend="handleResend"
            @preview-file="(f) => previewFile = f"
          />
        </TransitionGroup>

        <!-- Processing indicator (initial phase, before any content arrives) -->
        <ThinkingIndicator
          v-if="chat.isProcessing && !chat.currentAssistantMsg?.content && !chat.currentAssistantMsg?.thinking"
          :start-timestamp="chat.currentAssistantMsg?.timestamp"
        />
      </div>

      <!-- Debug -->
      <div v-if="debugLog.lines.value.length > 0" class="max-w-3xl mx-auto px-4 pb-4">
        <button @click="debugLog.toggle()" class="text-[11px] transition-colors hover:text-[var(--text-secondary)]" style="color:var(--text-muted)">
          {{ debugLog.visible.value ? '▾' : '▸' }} Debug ({{ debugLog.lines.value.length }})
        </button>
        <pre
          v-if="debugLog.visible.value"
          class="mt-2 p-3 rounded-lg text-[11px] font-mono leading-relaxed whitespace-pre-wrap break-all max-h-48 overflow-y-auto"
          style="background:var(--bg-root); border:1px solid var(--border-dim); color:var(--text-muted)"
        >{{ debugLog.lines.value.join('\n') }}</pre>
      </div>
    </div>

    <!-- Permission bar -->
    <div
      v-if="chat.pendingControlRequest"
      class="shrink-0 px-4 py-2.5 flex items-center gap-3"
      style="background:var(--amber-glow); border-top:1px solid var(--amber); border-color:var(--amber); --tw-border-opacity:0.25"
    >
      <span class="text-xs flex-1" style="color:var(--amber)">
        Allow <code class="px-1 py-px rounded text-[11px]" style="background:rgba(245,166,35,0.1)">{{ chat.pendingControlRequest.tool_name }}</code>?
      </span>
      <button @click="handleAllow" class="px-3 py-1 rounded-md text-xs font-medium transition-colors" style="background:var(--accent-dim); color:white">Allow</button>
      <button @click="handleDeny" class="px-3 py-1 rounded-md text-xs font-medium transition-colors" style="border:1px solid var(--coral); color:var(--coral)">Deny</button>
    </div>

    <!-- Toolbar (attach, command menu, mode, effort) -->
    <InputBarToolbar
      @attach-file="handleAttachFile"
      @open-command-menu="commandBus.open()"
    />

    <!-- Attached files chips -->
    <div v-if="attachedFiles.length > 0" class="max-w-3xl mx-auto px-1 pb-1.5 flex flex-wrap gap-1.5">
      <div
        v-for="(file, i) in attachedFiles"
        :key="file.path"
        class="flex items-center gap-1.5 pl-2 pr-1 py-1 rounded-md text-[11px] group shrink-0 max-w-[220px] cursor-pointer transition-colors"
        :style="{ background: 'var(--bg-elevated)', border: '1px solid var(--border-dim)', color: 'var(--text-secondary)' }"
        @click="previewFile = file"
      >
        <!-- File thumbnail / icon -->
        <img
          v-if="isImageFile(file.name)"
          :src="thumbnails[file.path] || ''"
          @vue:mounted="getThumbnail(file.path, file.name)"
          class="w-5 h-5 rounded object-cover shrink-0"
          style="border: 1px solid var(--border-dim)"
          v-show="thumbnails[file.path]"
        />
        <svg v-else width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2" stroke-linecap="round" class="shrink-0"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/></svg>
        <span class="truncate text-[11px] font-medium" :title="file.path">{{ file.name }}</span>
        <button
          @click.stop="removeAttachedFile(i)"
          class="w-4 h-4 flex items-center justify-center rounded transition-colors hover:bg-[var(--bg-hover)] shrink-0 opacity-50 group-hover:opacity-100"
          style="color: var(--text-muted)"
          title="Remove"
        >&times;</button>
      </div>
    </div>

    <!-- File preview modal -->
    <FilePreviewModal :file="previewFile" @close="previewFile = null" />

    <!-- Input -->
    <InputBar :disabled="chat.isProcessing" :auto-mode="autoModeActive" @send="handleSend" @stop="handleStop" @files="(fs) => { for (const f of fs) { if (!attachedFiles.some(af => af.path === f.path)) attachedFiles.push(f); } }" />
  </div>
  </ErrorBoundary>
</template>
