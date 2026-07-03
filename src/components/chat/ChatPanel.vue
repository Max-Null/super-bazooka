<script setup lang="ts">
import { ref, computed, nextTick, watch, onMounted } from "vue";
import { useChatStore, type AttachedFile } from "@/stores/chat";
import { useSessionStore } from "@/stores/session";
import { useDebugLog } from "@/composables/useDebugLog";
import { useStderrLog } from "@/composables/useStderrLog";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  sendMessage,
  sendStdin,
  zenSendMessage,
  getAutoModeStatus,
  stopSession,
  listMessages,
  writeFile,
  loadSessionLogs,
} from "@/lib/tauri-bridge";
import { useFilePreview } from "@/composables/useFilePreview";
import { useSettingsStore, PROVIDER_LOGOS } from "@/stores/settings";
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
import { useCommandPaletteBus, useChatCommandBus, emitGlobalCommand } from "@/composables/useCommandPalette";
import { useI18n } from "vue-i18n";
const { t } = useI18n();
import { useCommandRegistry } from "@/composables/useCommandRegistry";
import { useSlashCommands } from "@/composables/useSlashCommands";

const chat = useChatStore();
const session = useSessionStore();
const slashCommands = useSlashCommands();
const settings = useSettingsStore();
const appVersion = __APP_VERSION__;

const debugLog = useDebugLog();
const stderrLog = useStderrLog();
const scrollContainer = ref<HTMLElement | null>(null);
const isNearBottom = ref(true);
const autoScroll = ref(true);
const commandBus = useCommandPaletteBus();
import { isImageFile } from "@/composables/useFilePreview";
const { getThumbnail, thumbnails } = useFilePreview();

// ── InputBar ref（供外部命令设置输入文本）──
const inputBar = ref<InstanceType<typeof InputBar> | null>(null);

// 翻译 CC 工具名（中文环境 Bash→命令行、Write→写入文件，英文保持原名）
function toolLabel(name: string): string {
  const key = `tools.${name}`;
  const translated = t(key);
  return translated !== key ? translated : name;
}

// 当前正在执行且未完成的工具名（用于 ThinkingIndicator）
const activeToolName = computed(() => {
  if (!chat.isProcessing) return undefined;
  const msg = chat.currentAssistantMsg;
  if (!msg?.toolUses.length) return undefined;
  // 最后一个 tool_use 没有 result 说明正在执行中
  const last = msg.toolUses[msg.toolUses.length - 1];
  return last.result === undefined ? toolLabel(last.name) : undefined;
});

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

// 复制 LLM stderr 日志
function copyStderrLog() {
  const text = stderrLog.lines.value.join('\n');
  const ta = document.createElement('textarea');
  ta.value = text;
  document.body.appendChild(ta);
  ta.select();
  document.execCommand('copy');
  document.body.removeChild(ta);
}

// ── Attached files ──
const attachedFiles = ref<AttachedFile[]>([]);

function removeAttachedFile(index: number) {
  attachedFiles.value.splice(index, 1);
}

// ── DOM 选择器片段（来自文件预览）──
interface DomSnippet { html: string; filePath: string; selector: string }
const domSnippet = ref<DomSnippet | null>(null);
function removeDomSnippet() { domSnippet.value = null; }

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
// 审批队列清空 → blocked 降级为 processing
watch(() => chat.pendingControlRequest, (cr) => {
  if (!cr && session.activeSessionId) {
    if (session.sessionActivity[session.activeSessionId] === 'blocked') {
      session.setSessionActivity(session.activeSessionId, 'processing');
    }
  }
});
// Ponytail 模式变更（设置面板或工具栏）→ 发送 /ponytail 命令给 CC
let ponytailInit = true;
watch(() => settings.ponytailMode, (v) => {
  if (ponytailInit) { ponytailInit = false; return; }  // 跳过初始值
  handleSend(`/ponytail ${v}`).catch(() => {});
});
// 会话切换时同步 debug 日志到当前会话
watch(() => session.activeSessionId, async (sid) => {
  if (!sid) return;
  debugLog.setSession(sid);
  stderrLog.setSession(sid);
  // 从 DB 恢复持久化的日志
  try {
    const [debugJson, stderrJson] = await loadSessionLogs(sid);
    if (debugJson) {
      try { debugLog.importLines(sid, JSON.parse(debugJson)); } catch {}
    }
    if (stderrJson) {
      try { stderrLog.importLines(sid, JSON.parse(stderrJson)); } catch {}
    }
  } catch { /* 静默，DB 加载失败不影响功能 */ }
}, { immediate: true });

// ── 命令面板聊天命令监听 ──
const { chatCommand } = useChatCommandBus();
const { register } = useCommandRegistry();

// 向命令面板注册聊天相关命令
register({ id: "continue-session", group: "session", labelKey: "command.continueSession", cliKey: "--continue", icon: "📋" });
register({ id: "rename-session", group: "session", labelKey: "command.renameSession", keys: "F2", icon: "✏️" });
register({ id: "delete-session", group: "session", labelKey: "command.deleteSession", keys: "Del", icon: "🗑️" });
register({ id: "export-session", group: "session", labelKey: "command.exportSession", descKey: "command.exportSessionDesc", icon: "📤" });
register({ id: "attach-file", group: "tools", labelKey: "command.attachFile", descKey: "command.attachFileDesc", icon: "📎" });
watch(() => chatCommand.value.ts, async (ts) => {
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
      const sid = session.activeSessionId;
      const active = session.sessions.find(s => s.id === sid);
      if (active && confirm(t('session.confirmDelete', { title: active.title }))) {
        // 先终止 CC 进程，防止后台残留
        try { await stopSession(sid); } catch { /* 无进程 */ }
        await session.deleteSession(sid);
        chat.clearMessages();
        showStatus(t('status.sessionDeleted'));
      }
      break;
    }
    case "slash-clear": handleSend("/clear"); break;
    case "install-ponytail": handleSend("请帮我安装 Ponytail 插件（ponytail@claude-plugins-official）"); break;
    default:
      if (action.startsWith("md-convert:")) {
        // MD → docx fallback：通过 CC /docx skill 转换
        const msg = action.slice("md-convert:".length);
        if (chat.isProcessing) {
          inputBar.value?.setText(msg);
        } else {
          handleSend(msg);
        }
      } else if (action.startsWith("attach-dom:")) {
        // 来自 FilePreview DOM 选择器 — 存为卡片，随下次消息一起发送
        const data = action.slice("attach-dom:".length);
        const lines = data.split("\n");
        // 格式：我在 `full/path` 中选中了这个元素，请修改其内容：
        const fileMatch = lines[0]?.match(/`([^`]+)`/);
        const fullPath = fileMatch?.[1] || "";
        const htmlLines = lines.slice(1).join("\n");
        domSnippet.value = {
          filePath: fullPath,
          selector: htmlLines.match(/<(\w+)/)?.[1] || "element",
          html: data,
        };
      } else if (action.startsWith("switch-workspace:")) {
        const newPath = action.slice("switch-workspace:".length);
        // 先停止当前会话的 CC 进程（避免旧 cwd 的进程继续 emit 事件到新会话）
        const sid = session.activeSessionId;
        if (sid) {
          try {
            await sendStdin(sid, JSON.stringify({
              type: "control_request",
              request_id: `interrupt_${Date.now()}`,
              request: { subtype: "interrupt" },
            }));
          } catch {}
          // 给 CC 1 秒优雅退出，超时则强杀
          await new Promise(r => setTimeout(r, 1000));
          try { await stopSession(sid); } catch {}
        }
        chat.clearMessages();
        isNearBottom.value = true;
        autoScroll.value = true;
        try {
          await session.createSession(settings.model, newPath, undefined, settings.locale);
          showStatus(t('status.workspaceSwitched', { path: newPath }));
        } catch {
          showStatus(t('status.sessionCreateFailed'));
        }
      } else {
        // 通用文本 → 作为普通消息发送给 CC
        handleSend(action);
      }
      break;
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
// 消息清空（新建会话 / clear）时同步隐藏置顶问题横幅
watch(() => chat.messages.length, (len) => {
  if (len === 0) {
    stickyQuestion.value = "";
    showSticky.value = false;
  }
});

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
  // 记录斜杠命令（仅用户主动发送的，非中途追加）
  if (text.startsWith("/")) slashCommands.recordCommand(text);
  const isMidProcessing = chat.isProcessing;
  // 新消息（非中途追加）才清日志和计划
  if (!isMidProcessing) {
    debugLog.clear();
    stderrLog.clear();
    savedPlan.value = { plan: "", planFilePath: "" };
  }
  let sid: string;
  if (settings.zenMode) {
    // 禅模式：用 Zen 会话 ID（持久化到 DB，mode='zen'）
    sid = session.zenActiveId;
    if (!sid) {
      chat.clearMessages();
      sid = await session.createSession(settings.model, undefined, "zen", settings.locale);
      session.zenActiveId = sid;
    }
  } else {
    sid = session.activeSessionId;
    if (!sid) {
      chat.clearMessages();  // 新建会话时清空旧消息记录
      sid = await session.createSession(settings.model, undefined, undefined, settings.locale);
    }
  }

  // Collect attached file paths + DOM snippet before clearing
  const filePaths = attachedFiles.value.map(f => f.path);
  attachedFiles.value = [];

  // DOM 选择器片段：拼到消息文本前面（显示 + 发送都包含）
  const domText = domSnippet.value ? `${domSnippet.value.html}\n\n` : "";
  domSnippet.value = null;
  const fullText = domText + text;

  const attachments = filePaths.length > 0 ? filePaths.map(p => ({ name: p.split(/[/\\]/).pop() || p, path: p })) : undefined;
  chat.addUserMessage(fullText, attachments);

  // 用户消息由 Rust 后端在 send_message 中统一保存，
  // 前端不再重复保存，避免历史回显时出现双份用户消息。

  // isMidProcessing 已在函数开头捕获
  if (!isMidProcessing) {
    chat.startAssistantMessage();
  }
  chat.isProcessing = true;
  // 发送即设绿点（不等第一条 assistant 事件）
  session.setSessionActivity(sid, 'processing');
  autoScroll.value = true;
  isNearBottom.value = true;
  await scrollToBottomInstant();
  try {
    if (settings.zenMode) {
      // 禅模式：直接调 LLM chat/completions（SSE 流式），绕过 CC CLI
      const chatUrl = settings.optimizeApiUrl;
      if (!chatUrl) {
        throw new Error("未配置聊天 API 地址，请在设置中填写 LLM API 地址（含完整路径，如 https://api.deepseek.com/v1/chat/completions）");
      }
      // 用 model 字段（用户当前选择的模型），禅模式下通常应为 OpenAI-format 模型名（如 deepseek-chat）
      await zenSendMessage(sid, fullText, settings.apiKey, chatUrl, settings.model);
    } else {
      await sendMessage(sid, fullText, {
        planMode: settings.planMode,
        autoMode: settings.autoMode,
        permissionMode: settings.permissionMode,
        effort: settings.effort,
        ultracode: settings.effort === "ultracode",
        model: settings.model,
        filePaths: filePaths.length > 0 ? filePaths : undefined,
        claudePath: settings.claudePath || undefined,
      });
    }
    // 侧栏统计在 useStreamProcessor result 事件后刷新（token 已入库）
  } catch (err) {
    debugLog.add(`>>> Error: ${err}`);
    debugLog.visible.value = true;
    const { key, params } = translateError(err);
    chat.appendText(`\n\n> ❌ ${t(key, params as any)}`);
    chat.finishAssistantMessage();
  }
}

// control_response 格式（cli-agent-protocol skill 确认 + 实测通过）:
// 嵌套：response.subtype/request_id/response.behavior/updatedInput/message
async function handleAllow() {
  const cr = chat.pendingControlRequest; if (!cr) return;
  const payload = {
    type: "control_response",
    response: {
      subtype: "success",
      request_id: cr.request_id || "",
      response: {
        behavior: "allow",
        updatedInput: cr.tool_input,  // 必须原样回传工具输入
      },
    },
  };
  debugLog.add(`📤 control_response allow: ${JSON.stringify(payload)}`);
  await sendStdin(session.activeSessionId, JSON.stringify(payload));
  chat.resolveControlRequest("allow");
}
async function handleDeny() {
  const cr = chat.pendingControlRequest; if (!cr) return;
  const payload = {
    type: "control_response",
    response: {
      subtype: "success",
      request_id: cr.request_id || "",
      response: {
        behavior: "deny",
        message: "User denied this action",  // deny 必须填 message
      },
    },
  };
  debugLog.add(`📤 control_response deny: ${JSON.stringify(payload)}`);
  await sendStdin(session.activeSessionId, JSON.stringify(payload));
  chat.resolveControlRequest("deny");
}

// ── Edit + Resend: 保留原消息不动，新消息追加到末尾 ──

/** 从消息文本中提取 DOM 片段，恢复为卡片并返回去掉片段的文本 */
function extractDomSnippet(content: string): string {
  const match = content.match(/^我在 `(.+?)` 中选中了这个元素，请修改其内容：\n```html\n([\s\S]+?)\n```\n\n/);
  if (!match) return content;
  const filePath = match[1];
  const html = match[2];
  const tagMatch = html.match(/<(\w+)/);
  domSnippet.value = {
    filePath,
    selector: tagMatch?.[1] || "element",
    html: match[0].slice(0, -2), // 去掉末尾 \n\n
  };
  return content.slice(match[0].length);
}

async function handleEditSave(id: string, newContent: string) {
  const originalMsg = chat.messages.find(m => m.id === id);

  // 恢复原始附件
  if (originalMsg?.attachments?.length) {
    attachedFiles.value = originalMsg.attachments.map(a => ({
      name: a.name,
      path: a.path,
    }));
  }

  const cleanContent = extractDomSnippet(newContent);
  await handleSend(cleanContent);
}

async function handleResend(id: string, content: string) {
  const originalMsg = chat.messages.find(m => m.id === id);
  if (originalMsg?.attachments?.length) {
    attachedFiles.value = originalMsg.attachments.map(a => ({
      name: a.name,
      path: a.path,
    }));
  }
  const cleanContent = extractDomSnippet(content);
  await handleSend(cleanContent);
}

// ── AskUserQuestion 问答状态 ──
interface Question { question: string; header: string; options: { label: string; description: string }[]; multiSelect: boolean }
const questionAnswers = ref<Map<string, string | string[]>>(new Map());
const questionOther = ref<Map<string, string>>(new Map());

function getQuestions(): Question[] {
  const input = chat.pendingControlRequest?.tool_input as Record<string, unknown> | undefined;
  if (!input || !Array.isArray(input.questions)) return [];
  return input.questions as Question[];
}

function toggleAnswer(question: string, label: string, multi: boolean) {
  const cur = questionAnswers.value.get(question);
  if (multi) {
    const arr = (Array.isArray(cur) ? [...cur] : []) as string[];
    const idx = arr.indexOf(label);
    idx >= 0 ? arr.splice(idx, 1) : arr.push(label);
    questionAnswers.value.set(question, arr);
  } else {
    questionAnswers.value.set(question, cur === label ? '' : label);
  }
  questionOther.value.delete(question);
}

function setOther(question: string, text: string) {
  questionAnswers.value.delete(question);
  questionOther.value.set(question, text);
}

async function submitAnswers() {
  const cr = chat.pendingControlRequest; if (!cr) return;
  const answers: Record<string, string | string[]> = {};
  const questions = getQuestions();
  for (const q of questions) {
    const other = questionOther.value.get(q.question);
    if (other !== undefined) { answers[q.question] = other; continue; }
    const ans = questionAnswers.value.get(q.question);
    if (ans) answers[q.question] = ans;
  }
  const payload = {
    type: "control_response",
    response: {
      subtype: "success",
      request_id: cr.request_id || "",
      response: {
        behavior: "allow",
        updatedInput: { questions: cr.tool_input.questions, answers },
      },
    },
  };
  debugLog.add(`📤 AskUserQuestion answers: ${JSON.stringify(payload)}`);
  await sendStdin(session.activeSessionId, JSON.stringify(payload));
  questionAnswers.value.clear();
  questionOther.value.clear();
  chat.resolveControlRequest("allow");
}

function skipQuestions() {
  handleDeny();
  questionAnswers.value.clear();
  questionOther.value.clear();
}

// ── ExitPlanMode 计划审核 ──
const planFeedback = ref("");
// 保存最近一次计划，关闭弹窗后可重新打开查看
const savedPlan = ref<{ plan: string; planFilePath: string }>({ plan: "", planFilePath: "" });
const showPlanModal = ref(false);

function getPlan(): { plan: string; planFilePath: string } {
  const input = chat.pendingControlRequest?.tool_input as Record<string, unknown> | undefined;
  // 优先取当前审批中的计划，否则取已保存的
  if (input?.plan) return { plan: input.plan as string, planFilePath: (input.planFilePath as string) || "" };
  return savedPlan.value;
}

function savePlan() {
  const input = chat.pendingControlRequest?.tool_input as Record<string, unknown> | undefined;
  if (input?.plan) {
    savedPlan.value = { plan: input.plan as string, planFilePath: (input.planFilePath as string) || "" };
  }
}

function isPlanPending() {
  return chat.pendingControlRequest?.tool_name === "ExitPlanMode";
}

async function approvePlan() {
  const cr = chat.pendingControlRequest; if (!cr) return;
  const sid = session.activeSessionId;
  if (!sid) return;
  savePlan();
  try {
    // 批准 ExitPlanMode
    await sendStdin(sid, JSON.stringify({
      type: "control_response",
      response: {
        subtype: "success",
        request_id: cr.request_id || "",
        response: { behavior: "allow", updatedInput: cr.tool_input },
      },
    }));
    // 切换到 acceptEdits 模式执行计划（绕过 CC SDK 已知 bug：plan→acceptEdits 无限循环）
    await sendStdin(sid, JSON.stringify({
      type: "control_request",
      request_id: `setmode_${Date.now()}`,
      request: { subtype: "set_permission_mode", mode: "acceptEdits" },
    }));
    settings.planMode = false;
    settings.permissionMode = "acceptEdits";
    planFeedback.value = "";
    chat.resolveControlRequest("allow");
  } catch (e) {
    console.error("Plan approval failed:", e);
    showStatus(t("chat.planError") || "计划执行失败，请重试");
    chat.resolveControlRequest("deny");
  }
}

async function rejectPlan(message: string) {
  const cr = chat.pendingControlRequest; if (!cr) return;
  const sid = session.activeSessionId;
  if (!sid) return;
  savePlan();
  try {
    await sendStdin(sid, JSON.stringify({
      type: "control_response",
      response: {
        subtype: "success",
        request_id: cr.request_id || "",
        response: { behavior: "deny", message: message || "Plan rejected" },
      },
    }));
  } catch (e) {
    console.error("Plan rejection failed:", e);
  } finally {
    planFeedback.value = "";
    chat.resolveControlRequest("deny");
  }
}

async function exportPlan() {
  const { plan } = getPlan();
  if (!plan) return;
  const blob = new Blob([plan], { type: "text/markdown" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url; a.download = "plan.md";
  document.body.appendChild(a); a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

// ── Stop processing ──
async function handleStop() {
  const sid = settings.zenMode ? session.zenActiveId : session.activeSessionId;
  if (!sid) return;
  // 禅模式：无 CC 进程可终止，直接标记停止
  if (settings.zenMode) {
    chat.markStopped();
    chat.finishAssistantMessage();
    return;
  }
  // 先标记停止（必须在 interrupt 之前，否则 CC 的 result 事件会先触发 finish 清掉 currentAssistantMsg）
  chat.markStopped();
  // 发 interrupt 控制请求，让 CC 优雅中断
  try {
    await sendStdin(sid, JSON.stringify({
      type: "control_request",
      request_id: `interrupt_${Date.now()}`,
      request: { subtype: "interrupt" },
    }));
  } catch {}
  // 等待 3 秒让 CC 优雅退出，超时再强杀
  await new Promise(r => setTimeout(r, 3000));
  try { await stopSession(sid); } catch {}
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
watch(
  () => [
    chat.messages.length,
    chat.currentAssistantMsg?.content,
    chat.currentAssistantMsg?.thinking,
    chat.currentAssistantMsg?.toolUses.length,
  ],
  () => scrollToBottomIfAuto(),
);
</script>

<template>
  <ErrorBoundary name="ChatPanel">
  <div class="sb-chat-panel flex flex-col flex-1 h-full relative overflow-hidden">
    <!-- Sticky question banner -->
    <div
      v-if="showSticky"
      class="shrink-0 px-4 py-1.5 text-xs truncate z-10"
      style="background:var(--bg-elevated); border-bottom:1px solid var(--border-dim); color:var(--text-muted); backdrop-filter:blur(8px)"
    >
      <span class="font-medium" style="color:var(--text-secondary)">↳ </span>{{ stickyQuestion }}
    </div>

    <!-- Messages -->
    <div ref="scrollContainer" class="flex-1 relative overflow-y-auto" @scroll="onScrollThrottled">
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
          <h2 class="text-xl font-bold mb-2 tracking-tight" style="color:var(--text-bright)">{{ $t('chat.welcomeTitle') }}</h2>
          <p class="text-sm leading-relaxed mb-6" style="color:var(--text-secondary)">{{ $t('chat.welcomeSubtitle') }}</p>
          <div class="flex items-center justify-center gap-2 text-[11px]" style="color:var(--text-muted)">
            <kbd class="px-1.5 py-0.5 rounded text-[10px]" style="background:var(--bg-elevated); border:1px solid var(--border-dim)">Enter</kbd>
            <span>{{ $t('chat.welcomeSend') }}</span>
            <span style="color:var(--border-default)">·</span>
            <kbd class="px-1.5 py-0.5 rounded text-[10px]" style="background:var(--bg-elevated); border:1px solid var(--border-dim)">Shift</kbd>
            <span>+</span>
            <kbd class="px-1.5 py-0.5 rounded text-[10px]" style="background:var(--bg-elevated); border:1px solid var(--border-dim)">Enter</kbd>
            <span>{{ $t('chat.welcomeNewline') }}</span>
          </div>
        </div>
      </div>

      <!-- Message list -->
      <div v-if="chat.messages.length > 0" class="max-w-3xl mx-auto px-4 py-6 space-y-5">
        <!-- Export bar (when messages exist) -->
        <div class="flex items-center justify-end">
          <button
            @click="prepareExport"
            class="flex items-center gap-1.5 px-2.5 py-1 rounded-md text-[11px] transition-colors hover:bg-[var(--bg-hover)]"
            style="color: var(--text-secondary)"
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

        <!-- 处理中指示器：仅在消息还没内容时显示（有内容后时间线底部状态行接管） -->
        <ThinkingIndicator
          v-if="chat.isProcessing && !chat.currentAssistantMsg?.content && !chat.currentAssistantMsg?.thinking"
          :tool-name="activeToolName"
        />
      </div>
    </div>

    <!-- Debug / LLM 展开内容：绝对定位弹出 + 点击外部关闭 -->
    <Teleport to="body">
      <div
        v-if="debugLog.visible.value || stderrLog.visible.value"
        class="fixed inset-0 z-40"
        @click="debugLog.visible.value = false; stderrLog.visible.value = false"
      >
        <div
          @click.stop
          class="absolute bottom-[160px] left-4 right-4 max-w-3xl mx-auto flex flex-col gap-1.5"
        >
          <div class="flex items-center gap-4 self-start px-3 py-1 rounded-md" style="background: var(--bg-surface); border: 1px solid var(--border-dim); box-shadow: 0 2px 6px rgba(0,0,0,0.15)">
            <span v-if="debugLog.visible.value" class="text-[11px]" :style="{ color: 'var(--text-bright)' }">{{ $t('chat.debugLabel') }} ({{ debugLog.lines.value.length }})</span>
            <span v-if="stderrLog.visible.value" class="text-[11px]" :style="{ color: 'var(--accent)' }">📤 {{ $t('chat.llmRequestLabel') }} ({{ stderrLog.lines.value.length }})</span>
            <button @click="debugLog.visible.value ? copyDebugLog() : copyStderrLog()" class="cursor-pointer w-4 h-4 flex items-center justify-center rounded hover:bg-[var(--bg-hover)] transition-colors" :style="{ color: 'var(--text-muted)' }" :title="$t('chat.copy')">
              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
            </button>
          </div>
          <pre v-if="debugLog.visible.value" class="p-3 rounded-lg text-[11px] font-mono leading-relaxed whitespace-pre-wrap break-all max-h-48 overflow-y-auto" style="background:var(--bg-elevated); border:1px solid var(--border-dim); color:var(--text-muted); box-shadow: 0 4px 12px rgba(0,0,0,0.4)">{{ debugLog.lines.value.join('\n') }}</pre>
          <pre v-if="stderrLog.visible.value" class="p-3 rounded-lg text-[11px] font-mono leading-relaxed whitespace-pre-wrap break-all max-h-96 overflow-y-auto" style="background:var(--bg-elevated); border:1px solid var(--accent-glow); color:var(--text-muted); box-shadow: 0 4px 12px rgba(0,0,0,0.4)">{{ stderrLog.lines.value.join('\n') }}</pre>
        </div>
      </div>
    </Teleport>

    <!-- Permission bar（AskUserQuestion 走独立问答弹窗）-->
    <div
      v-if="chat.pendingControlRequest && chat.pendingControlRequest.tool_name !== 'AskUserQuestion' && chat.pendingControlRequest.tool_name !== 'ExitPlanMode'"
      class="shrink-0 px-4 py-2.5 flex items-center gap-3"
      style="background:var(--amber-glow); border-top:1px solid var(--amber); border-color:var(--amber); --tw-border-opacity:0.25"
    >
      <span class="text-xs flex-1" style="color:var(--amber)">
        {{ $t('chat.allowTool', { tool: toolLabel(chat.pendingControlRequest.tool_name || '') }) }}
      </span>
      <button @click="handleAllow" class="px-3 py-1 rounded-md text-xs font-medium transition-colors" style="background:var(--accent-dim); color:white">{{ $t('chat.allow') }}</button>
      <button @click="handleDeny" class="px-3 py-1 rounded-md text-xs font-medium transition-colors" style="border:1px solid var(--coral); color:var(--coral)">{{ $t('chat.deny') }}</button>
    </div>

    <!-- 滚动到底按钮 — 绝对定位在工具栏上方居中，不占文档流高度 -->
    <Transition name="scroll-btn">
      <button
        v-if="!isNearBottom && chat.messages.length > 0"
        @click="scrollToBottomAndResume"
        class="scroll-to-bottom-btn"
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

    <!-- Toolbar（禅模式下隐藏——模式/effort/Ponytail 对直接 LLM 无意义） -->
    <InputBarToolbar
      v-if="!settings.zenMode"
      @attach-file="handleAttachFile"
      @open-command-menu="commandBus.open()"
      @send-slash="(t: string) => handleSend(t)"
      @show-context="showContextModal = true"
    >
      <template #left>
        <template v-if="debugLog.lines.value.length > 0 || stderrLog.lines.value.length > 0">
          <button
            v-if="debugLog.lines.value.length > 0"
            @click="debugLog.toggle(); if (debugLog.visible.value) stderrLog.visible.value = false"
            class="cursor-pointer text-[11px] transition-colors hover:text-[var(--text-secondary)] flex items-center gap-1"
            :style="{ color: debugLog.visible.value ? 'var(--text-bright)' : 'var(--text-muted)' }"
          >
            <span>{{ debugLog.visible.value ? '▾' : '▸' }}</span>
            <span>{{ $t('chat.debugLabel') }} ({{ debugLog.lines.value.length }})</span>
          </button>
          <button
            v-if="stderrLog.lines.value.length > 0"
            @click="stderrLog.toggle(); if (stderrLog.visible.value) debugLog.visible.value = false"
            class="cursor-pointer text-[11px] transition-colors hover:text-[var(--text-secondary)] flex items-center gap-1"
            :style="{ color: stderrLog.visible.value ? 'var(--accent)' : 'var(--text-muted)' }"
          >
            <span>{{ stderrLog.visible.value ? '▾' : '▸' }}</span>
            <span>📤 {{ $t('chat.llmRequestLabel') }} ({{ stderrLog.lines.value.length }})</span>
          </button>
          <div class="w-px h-4 shrink-0" style="background: var(--border-dim)"></div>
        </template>
      </template>
    </InputBarToolbar>

    <!-- 禅模式指示器 — 点击退出禅模式 -->
    <div
      v-if="settings.zenMode"
      class="sb-toolbar"
    >
      <button
        @click="emitGlobalCommand('zen-mode')"
        class="text-[11px] px-1.5 py-0.5 rounded font-medium shrink-0 cursor-pointer transition-colors hover:opacity-80"
        :style="{ color: 'var(--accent)', background: 'var(--accent-glow)' }"
        :title="$t('header.exitZenMode')"
      >
        <img
          v-if="PROVIDER_LOGOS[settings.providerId]"
          :src="PROVIDER_LOGOS[settings.providerId]"
          class="w-3.5 h-3.5 shrink-0 inline-block align-middle"
          alt=""
        />
        <span v-else>🤖</span>
        {{ $t('chat.zenActive') }} · {{ $t('header.exitZenMode') }}
      </button>
    </div>

    <!-- 已保存计划的快捷入口 + 附件 chips — 无内容时不占高度 -->
    <div v-if="(savedPlan.plan && !isPlanPending()) || attachedFiles.length > 0 || domSnippet" class="sb-attachment-bar">
      <button
        v-if="savedPlan.plan && !isPlanPending()"
        @click="showPlanModal = true"
        class="flex items-center gap-1 pl-2 pr-1 py-1 rounded-md text-[11px] shrink-0 transition-colors hover:bg-[var(--bg-hover)]"
        :style="{ color: 'var(--text-muted)', border: '1px dashed var(--border-dim)' }"
      >📋 {{ $t('chat.viewPlan') }}</button>
      <!-- DOM 选择器片段卡片 -->
      <div
        v-if="domSnippet"
        class="flex items-center gap-1.5 pl-2 pr-1 py-1 rounded-md text-[11px] group shrink-0 max-w-[260px]"
        :style="{ background: 'var(--accent-glow)', border: '1px solid var(--accent-dim)', color: 'var(--accent)' }"
      >
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" class="shrink-0"><polyline points="6 9 12 15 18 9"/><line x1="4" y1="20" x2="20" y2="20"/></svg>
        <span class="truncate text-[11px] font-medium" :title="domSnippet.filePath">{{ domSnippet.filePath.split(/[\\/]/).pop() }} · &lt;{{ domSnippet.selector }}&gt;</span>
        <button @click="removeDomSnippet" class="shrink-0 w-4 h-4 flex items-center justify-center rounded hover:bg-[var(--bg-hover)] transition-colors">×</button>
      </div>
      <!-- Attached files chips -->
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
          style="color: var(--text-secondary)"
          :title="$t('chat.remove')"
        >&times;</button>
      </div>
    </div>

    <!-- File preview modal -->
    <FilePreviewModal :file="previewFile" @close="previewFile = null" />
    <ContextUsageModal :open="showContextModal" @close="showContextModal = false" @compact="showContextModal = false; handleSend('/compact')" />
    <ManagePanel :open="showManage" :initialTab="manageTab" @close="showManage = false" @send-slash="(t) => handleSend(t)" />

    <!-- AskUserQuestion 问答弹窗 -->
    <ModalShell :open="chat.pendingControlRequest?.tool_name === 'AskUserQuestion'" size="md" position="top" @close="skipQuestions">
      <template #header>
        <span class="text-sm font-semibold" :style="{ color: 'var(--text-bright)' }">{{ $t('chat.askUserQuestion') }}</span>
      </template>
      <div class="space-y-4 px-1">
        <div v-for="(q, qi) in getQuestions()" :key="qi" class="space-y-2">
          <div class="flex items-center gap-1.5">
            <span v-if="q.header" class="text-[10px] px-1.5 py-0.5 rounded font-medium whitespace-nowrap shrink-0" :style="{ background: 'var(--accent-glow)', color: 'var(--accent)' }">{{ q.header }}</span>
            <span class="text-xs font-medium" :style="{ color: 'var(--text-primary)' }">{{ q.question }}</span>
          </div>
          <div class="space-y-1 ml-1">
            <label
              v-for="opt in q.options"
              :key="opt.label"
              class="flex items-start gap-2 px-2 py-1.5 rounded cursor-pointer transition-colors hover:bg-[var(--bg-hover)]"
            >
              <input
                :type="q.multiSelect ? 'checkbox' : 'radio'"
                :name="`q_${qi}`"
                :checked="q.multiSelect
                  ? (Array.isArray(questionAnswers.get(q.question)) && (questionAnswers.get(q.question) as string[]).includes(opt.label))
                  : questionAnswers.get(q.question) === opt.label"
                @change="toggleAnswer(q.question, opt.label, q.multiSelect)"
                class="mt-0.5 shrink-0"
              />
              <div class="min-w-0">
                <div class="text-xs font-medium" :style="{ color: 'var(--text-secondary)' }">{{ opt.label }}</div>
                <div class="text-[11px] leading-relaxed" :style="{ color: 'var(--text-muted)' }">{{ opt.description }}</div>
              </div>
            </label>
            <!-- Other 自由输入 -->
            <label class="flex items-start gap-2 px-2 py-1.5 rounded cursor-pointer transition-colors hover:bg-[var(--bg-hover)]">
              <input
                :type="q.multiSelect ? 'checkbox' : 'radio'"
                :name="`q_${qi}`"
                :checked="questionOther.has(q.question)"
                @change="questionOther.set(q.question, '')"
                class="mt-0.5 shrink-0"
              />
              <div class="flex-1 min-w-0">
                <div class="text-xs font-medium" :style="{ color: 'var(--text-secondary)' }">Other</div>
                <input
                  v-if="questionOther.has(q.question)"
                  :value="questionOther.get(q.question) || ''"
                  @input="(e) => setOther(q.question, (e.target as HTMLInputElement).value)"
                  placeholder="输入自定义答案..."
                  class="mt-1 w-full rounded px-2 py-1 text-xs outline-none"
                  :style="{ background: 'var(--bg-elevated)', border: '1px solid var(--border-dim)', color: 'var(--text-primary)', caretColor: 'var(--accent)' }"
                />
              </div>
            </label>
          </div>
        </div>
      </div>
      <template #footer>
        <div class="flex items-center justify-end gap-2">
          <button @click="skipQuestions" class="text-xs px-3 py-1.5 rounded transition-colors hover:bg-[var(--bg-hover)]" :style="{ color: 'var(--text-muted)' }">{{ $t('chat.skip') }}</button>
          <button @click="submitAnswers" class="px-4 py-1.5 rounded text-xs font-medium transition-colors" :style="{ background: 'var(--accent)', color: 'var(--bg-root)' }">{{ $t('chat.submit') }}</button>
        </div>
      </template>
    </ModalShell>

    <!-- ExitPlanMode 计划审核弹窗（审批中或已保存重新打开）-->
    <ModalShell :open="isPlanPending() || showPlanModal" size="xl" position="top" @close="isPlanPending() ? rejectPlan('Plan review cancelled') : (showPlanModal = false)">
      <template #header>
        <div class="flex items-center gap-2">
          <span class="text-sm font-semibold" :style="{ color: 'var(--text-bright)' }">{{ $t('chat.planReview') }}</span>
          <span class="text-[10px] font-mono opacity-50" :style="{ color: 'var(--text-muted)' }">{{ getPlan().planFilePath }}</span>
        </div>
      </template>
      <!-- 计划内容 -->
      <div class="max-h-[55vh] overflow-y-auto">
        <MarkdownRenderer v-if="getPlan().plan" :content="getPlan().plan" />
        <div v-else class="text-xs py-8 text-center" :style="{ color: 'var(--text-muted)' }">{{ $t('chat.noPlanContent') }}</div>
      </div>
      <!-- 反馈输入 -->
      <div class="mt-3">
        <input
          v-model="planFeedback"
          :placeholder="$t('chat.planFeedbackPlaceholder')"
          class="w-full rounded-lg px-3 py-2 text-xs outline-none"
          :style="{ background: 'var(--bg-elevated)', border: '1px solid var(--border-dim)', color: 'var(--text-primary)', caretColor: 'var(--accent)' }"
          @keydown.enter="rejectPlan(planFeedback)"
        />
      </div>
      <template #footer>
        <div v-if="isPlanPending()" class="flex items-center gap-1.5 flex-wrap">
          <button @click="approvePlan()" class="px-3 py-1.5 rounded text-xs font-medium transition-colors hover:brightness-110" :style="{ background: 'var(--accent)', color: 'var(--bg-root)' }">✅ {{ $t('chat.planExecute') }}</button>
          <button @click="rejectPlan('继续优化计划设计')" class="px-3 py-1.5 rounded text-xs font-medium transition-colors hover:bg-[var(--bg-hover)]" :style="{ color: 'var(--text-secondary)', border: '1px solid var(--border-dim)' }">✏️ {{ $t('chat.planContinueDesign') }}</button>
          <button @click="rejectPlan('停止计划')" class="px-3 py-1.5 rounded text-xs font-medium transition-colors hover:bg-[var(--bg-hover)]" :style="{ color: 'var(--text-secondary)', border: '1px solid var(--border-dim)' }">⏹ {{ $t('chat.planStop') }}</button>
          <button @click="exportPlan()" class="px-3 py-1.5 rounded text-xs font-medium transition-colors hover:bg-[var(--bg-hover)]" :style="{ color: 'var(--text-muted)', border: '1px dashed var(--border-dim)' }">📤 {{ $t('chat.planExport') }}</button>
          <button
            v-if="planFeedback"
            @click="rejectPlan(planFeedback)"
            class="px-3 py-1.5 rounded text-xs font-medium transition-colors hover:bg-[var(--bg-hover)] ml-auto"
            :style="{ color: 'var(--text-secondary)', border: '1px solid var(--border-dim)' }"
          >💬 {{ $t('chat.planOther') }}</button>
        </div>
        <div v-else class="flex items-center justify-end">
          <button @click="exportPlan()" class="px-3 py-1.5 rounded text-xs font-medium transition-colors hover:bg-[var(--bg-hover)]" :style="{ color: 'var(--text-muted)', border: '1px dashed var(--border-dim)' }">📤 {{ $t('chat.planExport') }}</button>
          <button @click="showPlanModal = false" class="px-3 py-1.5 rounded text-xs transition-colors hover:bg-[var(--bg-hover)] ml-2" :style="{ color: 'var(--text-muted)' }">{{ $t('chat.close') }}</button>
        </div>
      </template>
    </ModalShell>

    <!-- 关于弹窗 -->
    <ModalShell :open="showAbout" size="sm" @close="showAbout = false">
      <template #header>
        <span class="text-sm font-semibold" :style="{ color: 'var(--text-bright)' }">{{ $t('chat.aboutTitle') }}</span>
      </template>
      <div class="text-center py-4 space-y-3">
        <div class="text-lg font-bold" :style="{ color: 'var(--text-bright)' }">{{ $t('app.title') }}</div>
        <div class="text-xs" :style="{ color: 'var(--text-secondary)' }">{{ $t('chat.aboutSubtitle') }}</div>
        <div class="text-[11px] font-mono" :style="{ color: 'var(--text-muted)' }">v{{ appVersion }}</div>
        <div class="text-[11px]" :style="{ color: 'var(--text-muted)' }">
          Tauri 2 + Vue 3 + TypeScript<br/>
          Rust 后端 · SQLite 持久化<br/>
          多厂商 API 兼容
        </div>
        <div class="text-[10px] pt-2" :style="{ color: 'var(--text-muted)' }">
          © 2026 Super Bazooka contributors · MIT
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
    <InputBar ref="inputBar" :disabled="chat.isProcessing" :auto-mode="autoModeActive" :api-key="settings.apiKey" :base-url="settings.baseUrl" @send="handleSend" @stop="handleStop" @files="(fs) => { for (const f of fs) { if (!attachedFiles.some(af => af.path === f.path)) attachedFiles.push(f); } }" />
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
