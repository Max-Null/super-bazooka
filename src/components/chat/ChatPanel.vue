<script setup lang="ts">
import { ref, nextTick, watch } from "vue";
import { useChatStore } from "@/stores/chat";
import { useSessionStore } from "@/stores/session";
import { useDebugLog } from "@/composables/useDebugLog";
import { sendMessage, sendStdin } from "@/lib/tauri-bridge";
import { useSettingsStore } from "@/stores/settings";
import InputBar from "./InputBar.vue";
import MessageBubble from "./MessageBubble.vue";
import ModeBar from "./ModeBar.vue";
import ThinkingIndicator from "./ThinkingIndicator.vue";

const chat = useChatStore();
const session = useSessionStore();
const settings = useSettingsStore();
const debugLog = useDebugLog();
const scrollContainer = ref<HTMLElement | null>(null);

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
  chat.addUserMessage(text);
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
    <div ref="scrollContainer" class="flex-1 overflow-y-auto" @scroll="onScrollThrottled">
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
        <MessageBubble v-for="msg in chat.messages" :key="msg.id" :message="msg" />

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

    <!-- Mode toolbar -->
    <ModeBar />

    <!-- Input -->
    <InputBar :disabled="chat.isProcessing" @send="handleSend" />
  </div>
</template>
