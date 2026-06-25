<script setup lang="ts">
import { ref, nextTick, watch, onMounted } from "vue";
import { useChatStore } from "@/stores/chat";
import { useSessionStore } from "@/stores/session";
import { useDebugLog } from "@/composables/useDebugLog";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  sendMessage,
  sendStdin,
  getAutoModeStatus,
  stopSession,
  listMessages,
  writeFile,
  updateMessageContent,
  deleteMessagesAfter,
} from "@/lib/tauri-bridge";
import { useFilePreview } from "@/composables/useFilePreview";
import { useSettingsStore } from "@/stores/settings";
import { translateError } from "@/lib/utils";
import ErrorBoundary from "@/components/shared/ErrorBoundary.vue";
import InputBar from "./InputBar.vue";
import InputBarToolbar from "./InputBarToolbar.vue";
import MessageBubble from "./MessageBubble.vue";
import ThinkingIndicator from "./ThinkingIndicator.vue";
import FilePreviewModal from "@/components/shared/FilePreviewModal.vue";
import ContextUsageModal from "@/components/shared/ContextUsageModal.vue";
import ManagePanel from "@/components/shared/ManagePanel.vue";
import ModalShell from "@/components/shared/ModalShell.vue";
import MarkdownRenderer from "@/components/shared/MarkdownRenderer.vue";
import { useCommandPaletteBus, useChatCommandBus } from "@/composables/useCommandPalette";
import { useI18n } from "vue-i18n";
const { t } = useI18n();
import { useCommandRegistry } from "@/composables/useCommandRegistry";

const chat = useChatStore();
const session = useSessionStore();
const settings = useSettingsStore();
const debugLog = useDebugLog();
const scrollContainer = ref<HTMLElement | null>(null);
const isNearBottom = ref(true);
const autoScroll = ref(true);
const commandBus = useCommandPaletteBus();
import { isImageFile } from "@/composables/useFilePreview";
const { getThumbnail, thumbnails } = useFilePreview();

// 复制 debug 日志
function copyDebugLog() {
  const text = debugLog.lines.value.join('\n');
  const ta = document.createElement('textarea');
  ta.value = text;
  document.body.appendChild(ta);
  ta.select();
  document.execCommand('copy');
  document.body.removeChild(ta);
}

// ── Attached files ──
interface AttachedFile { name: string; path: string }
const attachedFiles = ref<AttachedFile[]>([]);

function removeAttachedFile(index: number) {
  attachedFiles.value.splice(index, 1);
}

const previewFile = ref<{ name: string; path: string } | null>(null);

// ── 状态消息（临时横幅，不挤占消息区域）──
const statusMessage = ref("");
const showContextModal = ref(false);
const showRenameModal = ref(false);
const renameTitle = ref("");
const showExportPreview = ref(false);
const showAbout = ref(false);
const showManage = ref(false);
const manageTab = ref<string>("plugins");
const exportContent = ref("");
const exportFileName = ref("");

function prepareExport() {
  const sid = session.activeSessionId;
  if (!sid || chat.messages.length === 0) return;
  const active = session.sessions.find(s => s.id === sid);
  const title = active?.title || "Chat Export";
  exportFileName.value = `${title.replace(/[^a-zA-Z0-9一-鿿_-]/g, "_")}.md`;
  exportContent.value = chat.exportMarkdown(title);
  showExportPreview.value = true;
}

async function doExport() {
  try {
    const filePath = await save({
      defaultPath: exportFileName.value,
      filters: [{ name: "Markdown", extensions: ["md"] }],
    });
    if (!filePath) return;
    await writeFile(filePath, exportContent.value);
    showExportPreview.value = false;
    showStatus(t('status.exportDone'));
  } catch (e) {
    showStatus(t('status.exportFail', { error: String(e) }));
  }
}

function confirmRename() {
  const title = renameTitle.value.trim();
  if (title && session.activeSessionId) {
    session.renameSession(session.activeSessionId, title);
    showStatus(t('status.renamed', { title }));
  }
  showRenameModal.value = false;
}
let statusTimer: ReturnType<typeof setTimeout> | null = null;
function showStatus(msg: string) {
  statusMessage.value = msg;
  if (statusTimer) clearTimeout(statusTimer);
  statusTimer = setTimeout(() => { statusMessage.value = ""; }, 5000);
}

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

// ── 命令面板聊天命令监听 ──
const { chatCommand } = useChatCommandBus();
const { register } = useCommandRegistry();

// 向命令面板注册聊天相关命令
register({ id: "continue-session", group: "session", labelKey: "command.continueSession", cliKey: "--continue", icon: "📋" });
register({ id: "rename-session", group: "session", labelKey: "command.renameSession", keys: "F2", icon: "✏️" });
register({ id: "delete-session", group: "session", labelKey: "command.deleteSession", keys: "Del", icon: "🗑️" });
register({ id: "export-session", group: "session", labelKey: "command.exportSession", descKey: "command.exportSessionDesc", icon: "📤" });
register({ id: "attach-file", group: "tools", labelKey: "command.attachFile", descKey: "command.attachFileDesc", icon: "📎" });
watch(() => chatCommand.value.ts, (ts) => {
  if (!ts) return;
  const action = chatCommand.value.action;
  switch (action) {
    case "continue-session": {
      const others = session.sessions.filter(s => s.id !== session.activeSessionId);
      if (others.length > 0) {
        const target = others[0];
        session.setActiveSession(target.id);
        listMessages(target.id).then(msgs => {
          chat.loadMessages(msgs.map(m => ({ id: m.id, role: m.role, content: m.content, created_at: m.created_at })));
          showStatus(t('session.switchSuccess', { title: target.title }));
        }).catch(() => showStatus(t('session.loadFailed')));
      }
      break;
    }
    case "rename-session": {
      const active = session.sessions.find(s => s.id === session.activeSessionId);
      if (active) {
        renameTitle.value = active.title;
        showRenameModal.value = true;
      }
      break;
    }
    case "delete-session": {
      const active = session.sessions.find(s => s.id === session.activeSessionId);
      if (active && confirm(t('session.confirmDelete', { title: active.title }))) {
        session.deleteSession(session.activeSessionId);
        chat.clearMessages();
        showStatus(t('status.sessionDeleted'));
      }
      break;
    }
    case "slash-clear": handleSend("/clear"); break;
    case "export-session":
      if (!session.activeSessionId || chat.messages.length === 0) {
        showStatus(t('status.exportEmpty'));
      } else {
        prepareExport();
      }
      break;
    case "slash-compact":   handleSend("/compact"); break;
    case "slash-context":   showContextModal.value = true; break;
    case "slash-cost":      handleSend("/cost"); break;
    case "slash-review":    handleSend("/review"); break;
    case "slash-simplify":  handleSend("/simplify"); break;
    case "slash-security":  handleSend("/security-review"); break;
    case "slash-doctor":    handleSend("/doctor"); break;
    case "slash-init":      handleSend("/init"); break;
    case "manage-plugins":    manageTab.value = "plugins"; showManage.value = true; break;
    case "manage-memory":     manageTab.value = "memory"; showManage.value = true; break;
    case "manage-mcp":        manageTab.value = "mcp"; showManage.value = true; break;
    case "manage-skills":     manageTab.value = "skills"; showManage.value = true; break;
    case "manage-agents":     manageTab.value = "agents"; showManage.value = true; break;
    case "manage-hooks":      manageTab.value = "hooks"; showManage.value = true; break;
    case "manage-permissions": manageTab.value = "permissions"; showManage.value = true; break;
    case "manage-styles":     manageTab.value = "styles"; showManage.value = true; break;
    case "attach-file":
      handleAttachFile();
      break;
    case "about":
      showAbout.value = true;
      break;
  }
});

// ── Sticky question banner ──
const stickyQuestion = ref("");
const showSticky = ref(false);

// 自动滚动检测：必须立即响应，不能节流。否则在 50ms 节流窗口内，
// 新的 token 到达时 autoScroll 还没变成 false，会把用户拽回底部。
function updateAutoScroll() {
  const container = scrollContainer.value;
  if (!container) return;
  const threshold = 60;
  const distFromBottom = container.scrollHeight - container.scrollTop - container.clientHeight;
  const near = distFromBottom < threshold;
  isNearBottom.value = near;
  autoScroll.value = near;
}

// 置顶问题横幅：DOM 查询开销大，节流处理
function updateStickyBanner() {
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
  updateAutoScroll(); // 立即处理，防止 autoScroll 滞后
  if (scrollTimer) return;
  scrollTimer = setTimeout(() => { scrollTimer = null; updateStickyBanner(); }, 100);
}

async function handleSend(text: string) {
  debugLog.clear();
  let sid = session.activeSessionId;
  if (!sid) sid = await session.createSession(settings.model);

  // Collect attached file paths before clearing
  const filePaths = attachedFiles.value.map(f => f.path);
  attachedFiles.value = [];

  const attachments = filePaths.length > 0 ? filePaths.map(p => ({ name: p.split(/[/\\]/).pop() || p, path: p })) : undefined;
  chat.addUserMessage(text, attachments);

  // 用户消息由 Rust 后端在 send_message 中统一保存，
  // 前端不再重复保存，避免历史回显时出现双份用户消息。
  chat.startAssistantMessage();
  chat.isProcessing = true;
  autoScroll.value = true;
  isNearBottom.value = true;
  await scrollToBottomInstant();
  try {
    await sendMessage(sid, text, {
      planMode: settings.planMode,
      autoMode: settings.autoMode,
      permissionMode: settings.permissionMode,
      effort: settings.effort,
      ultracode: settings.effort === "ultracode",
      model: settings.model,
      filePaths: filePaths.length > 0 ? filePaths : undefined,
      claudePath: settings.claudePath || undefined,
    });
    session.loadSessions().catch(() => {});
  } catch (err) {
    debugLog.add(`>>> Error: ${err}`);
    debugLog.visible.value = true;
    const { key, params } = translateError(err);
    chat.appendText(`\n\n> ❌ ${t(key, params as any)}`);
    chat.finishAssistantMessage();
  }
}

// control_response 格式（Goose PR #7420 + Agent SDK 确认）:
// allow: {"type":"control_response","request_id":"...","response":{"decision":"allow"}}
// deny:  {"type":"control_response","request_id":"...","response":{"decision":"deny","message":"..."}}
async function handleAllow() {
  const cr = chat.pendingControlRequest; if (!cr) return;
  const payload = {
    type: "control_response",
    request_id: cr.request_id || "",
    response: { decision: "allow" },
  };
  debugLog.add(`📤 control_response allow: ${JSON.stringify(payload)}`);
  await sendStdin(session.activeSessionId, JSON.stringify(payload));
  chat.resolveControlRequest("allow");
}
async function handleDeny() {
  const cr = chat.pendingControlRequest; if (!cr) return;
  const payload = {
    type: "control_response",
    request_id: cr.request_id || "",
    response: { decision: "deny", message: "User denied this action" },
  };
  debugLog.add(`📤 control_response deny: ${JSON.stringify(payload)}`);
  await sendStdin(session.activeSessionId, JSON.stringify(payload));
  chat.resolveControlRequest("deny");
}

// ── Edit + Resend: truncate subsequent messages, update content, resend ──
async function handleEditSave(id: string, newContent: string) {
  const originalMsg = chat.messages.find(m => m.id === id);
  const sid = session.activeSessionId;
  if (!sid) return;

  chat.updateMessage(id, newContent);

  const persistContent = originalMsg?.attachments?.length
    ? JSON.stringify({ text: newContent, attachments: originalMsg.attachments })
    : newContent;

  try {
    await updateMessageContent(id, sid, persistContent);
    await deleteMessagesAfter(id, sid);
  } catch (e) {
    console.error("Failed to persist edit:", e);
  }

  chat.truncateAfterMessage(id);

  // 恢复原始附件到文件列表，使 CLI 重新获得 --add-dir 权限
  if (originalMsg?.attachments?.length) {
    attachedFiles.value = originalMsg.attachments.map(a => ({
      name: a.name,
      path: a.path,
    }));
  }

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

// 即时滚动：流式输出每来一个 token 就触发，不能用 smooth，否则一直抖
async function scrollToBottomInstant() {
  await nextTick();
  if (scrollContainer.value) scrollContainer.value.scrollTop = scrollContainer.value.scrollHeight;
}
// 平滑滚动：用户点击"滚动到底部"按钮时用，有动画过渡
function scrollToBottomSmooth() {
  if (scrollContainer.value) {
    scrollContainer.value.scrollTo({ top: scrollContainer.value.scrollHeight, behavior: "smooth" });
  }
}
function scrollToBottomAndResume() {
  autoScroll.value = true;
  isNearBottom.value = true;
  scrollToBottomSmooth();
}
function scrollToBottomIfAuto() {
  if (autoScroll.value) scrollToBottomInstant();
}
watch(() => chat.messages.length, () => scrollToBottomIfAuto());
watch(() => chat.currentAssistantMsg?.content, () => scrollToBottomIfAuto());
watch(() => chat.currentAssistantMsg?.thinking, () => scrollToBottomIfAuto());
watch(() => chat.currentAssistantMsg?.toolUses.length, () => scrollToBottomIfAuto());
</script>

<template>
  <ErrorBoundary name="ChatPanel">
  <div class="flex flex-col flex-1 h-full relative overflow-hidden">
    <!-- Sticky question banner -->
    <div
      v-if="showSticky"
      class="shrink-0 px-4 py-1.5 text-xs truncate z-10"
      style="background:var(--bg-elevated); border-bottom:1px solid var(--border-dim); color:var(--text-muted); backdrop-filter:blur(8px)"
    >
      <span class="font-medium" style="color:var(--text-secondary)">↳ </span>{{ stickyQuestion }}
    </div>

    <!-- Messages -->
    <div ref="scrollContainer" :class="['flex-1 relative', chat.messages.length > 0 ? 'overflow-y-auto' : '']" @scroll="onScrollThrottled">
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
            @click="prepareExport"
            class="flex items-center gap-1.5 px-2.5 py-1 rounded-md text-[11px] transition-colors hover:bg-[var(--bg-hover)]"
            style="color: var(--text-muted)"
            :title="$t('chat.exportTitle')"
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
            <span>{{ $t('chat.export') }}</span>
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
        <div class="flex items-center gap-2">
          <button @click="debugLog.toggle()" class="text-[11px] transition-colors hover:text-[var(--text-secondary)]" style="color:var(--text-muted)">
            {{ debugLog.visible.value ? '▾' : '▸' }} Debug ({{ debugLog.lines.value.length }})
          </button>
          <button
            @click="copyDebugLog()"
            class="text-[10px] px-1.5 py-0.5 rounded transition-colors hover:bg-[var(--bg-hover)]"
            style="color:var(--text-muted)"
          >📋 复制</button>
        </div>
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
      <button @click="handleAllow" class="px-3 py-1 rounded-md text-xs font-medium transition-colors" style="background:var(--accent-dim); color:white">{{ $t('chat.allow') }}</button>
      <button @click="handleDeny" class="px-3 py-1 rounded-md text-xs font-medium transition-colors" style="border:1px solid var(--coral); color:var(--coral)">{{ $t('chat.deny') }}</button>
    </div>

    <!-- 滚动到底按钮 — 绝对定位在工具栏上方居中，不占文档流高度 -->
    <Transition name="scroll-btn">
      <button
        v-if="!isNearBottom && chat.messages.length > 0"
        @click="scrollToBottomAndResume"
        class="absolute bottom-[140px] left-1/2 -translate-x-1/2 z-10 flex items-center justify-center w-7 h-7 rounded-full transition-all duration-150 hover:scale-110"
        style="background: var(--bg-elevated); color: var(--text-secondary); border: 1px solid var(--border-bright); box-shadow: 0 1px 6px rgba(0,0,0,0.15)"
        :title="$t('chat.scrollToBottom')"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </button>
    </Transition>

    <!-- 状态消息：绝对定位在输入框上方，不挤占消息区域 -->
    <Transition name="scroll-btn">
      <div
        v-if="statusMessage"
        class="absolute bottom-[140px] left-0 right-0 flex justify-center z-10 pointer-events-none"
      >
        <span
          class="text-[11px] font-mono px-2.5 py-0.5 rounded-full"
          style="background: var(--accent-glow); color: var(--accent)"
        >{{ statusMessage }}</span>
      </div>
    </Transition>

    <!-- Toolbar (attach, command menu, mode, effort) -->
    <InputBarToolbar
      @attach-file="handleAttachFile"
      @open-command-menu="commandBus.open()"
      @send-slash="(t: string) => handleSend(t)"
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
          :title="$t('chat.remove')"
        >&times;</button>
      </div>
    </div>

    <!-- File preview modal -->
    <FilePreviewModal :file="previewFile" @close="previewFile = null" />
    <ContextUsageModal :open="showContextModal" @close="showContextModal = false" />
    <ManagePanel :open="showManage" :initialTab="manageTab" @close="showManage = false" @send-slash="(t) => handleSend(t)" />

    <!-- 关于弹窗 -->
    <ModalShell :open="showAbout" size="sm" @close="showAbout = false">
      <template #header>
        <span class="text-sm font-semibold" :style="{ color: 'var(--text-bright)' }">关于 cc-gui</span>
      </template>
      <div class="text-center py-4 space-y-3">
        <div class="text-lg font-bold" :style="{ color: 'var(--text-bright)' }">cc-gui</div>
        <div class="text-xs" :style="{ color: 'var(--text-secondary)' }">Claude Code Desktop GUI</div>
        <div class="text-[11px] font-mono" :style="{ color: 'var(--text-muted)' }">v0.1.0</div>
        <div class="text-[11px]" :style="{ color: 'var(--text-muted)' }">
          Tauri 2 + Vue 3 + TypeScript<br/>
          Rust 后端 · SQLite 持久化<br/>
          DeepSeek API 兼容
        </div>
        <div class="text-[10px] pt-2" :style="{ color: 'var(--text-muted)' }">
          © 2026 cc-gui contributors · MIT
        </div>
      </div>
    </ModalShell>

    <!-- 导出预览弹窗 -->
    <ModalShell :open="showExportPreview" size="lg" @close="showExportPreview = false">
      <template #header>
        <span class="text-sm font-semibold" :style="{ color: 'var(--text-bright)' }">导出预览 — {{ exportFileName }}</span>
      </template>
      <div class="overflow-y-auto p-1 min-w-0" style="max-height: 60vh">
        <div class="overflow-x-auto">
          <MarkdownRenderer :content="exportContent" />
        </div>
      </div>
      <div class="flex justify-end gap-2 mt-3">
        <button @click="showExportPreview = false" class="px-3 py-1.5 rounded-md text-xs transition-colors hover:bg-[var(--bg-hover)]" :style="{ color: 'var(--text-muted)' }">取消</button>
        <button @click="doExport" class="px-4 py-1.5 rounded-md text-xs font-medium transition-colors" :style="{ background: 'var(--accent)', color: 'var(--bg-root)' }">选择目录并导出</button>
      </div>
    </ModalShell>
    <!-- 重命名弹窗 -->
    <ModalShell :open="showRenameModal" size="sm" @close="showRenameModal = false">
      <template #header>
        <span class="text-sm font-semibold" :style="{ color: 'var(--text-bright)' }">重命名会话</span>
      </template>
      <input
        v-model="renameTitle"
        @keydown.enter="confirmRename"
        @keydown.escape="showRenameModal = false"
        class="w-full bg-transparent text-sm px-3 py-2 rounded-md border outline-none"
        :style="{ color: 'var(--text-bright)', borderColor: 'var(--border-default)', background: 'var(--bg-elevated)' }"
        autofocus
      />
      <div class="flex justify-end gap-2 mt-3">
        <button @click="showRenameModal = false" class="px-3 py-1.5 rounded-md text-xs transition-colors hover:bg-[var(--bg-hover)]" :style="{ color: 'var(--text-muted)' }">取消</button>
        <button @click="confirmRename" class="px-3 py-1.5 rounded-md text-xs font-medium transition-colors" :style="{ background: 'var(--accent)', color: 'var(--bg-root)' }">确认</button>
      </div>
    </ModalShell>

    <!-- Input -->
    <InputBar :disabled="chat.isProcessing" :auto-mode="autoModeActive" @send="handleSend" @stop="handleStop" @files="(fs) => { for (const f of fs) { if (!attachedFiles.some(af => af.path === f.path)) attachedFiles.push(f); } }" />
  </div>
  </ErrorBoundary>
</template>

<style scoped>
/* Scroll-to-bottom button transition */
.scroll-btn-enter-active { transition: all 200ms ease-out; }
.scroll-btn-leave-active { transition: all 150ms ease-in; }
.scroll-btn-enter-from { opacity: 0; transform: translateY(8px) scale(0.9); }
.scroll-btn-leave-to { opacity: 0; transform: translateY(4px) scale(0.95); }
</style>
