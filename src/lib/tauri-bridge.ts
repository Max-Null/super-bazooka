import { invoke } from "@tauri-apps/api/core";

export interface StreamEvent {
  type: string;
  session_id?: string;
  text: string;
  thinking: string;
  tool_use?: Array<{
    id: string;
    name: string;
    input: Record<string, unknown>;
  }>;
  /** CC content 块原始顺序（text/thinking/tool_use 交替），用于按时间线渲染 */
  content_blocks?: Array<{
    type: string;
    text?: string;
    thinking?: string;
    id?: string;
    name?: string;
    input?: Record<string, unknown>;
  }>;
  control_request?: {
    subtype: string;
    tool_name?: string;
    tool_input: Record<string, unknown>;
    request_id?: string;
  };
  duration_ms?: number;
  input_tokens?: number;
  output_tokens?: number;
  cost_usd?: number;
  is_final: boolean;
  error?: string;
  /** 工具执行结果（从 user 事件中提取） */
  tool_results?: Array<{
    tool_use_id: string;
    content: string;
    is_error?: boolean;
  }>;
}

/**
 * Send a stdin line to a running CLI session (e.g., permission response).
 */
export async function sendStdin(sessionId: string, data: string): Promise<void> {
  return invoke("send_stdin", { sessionId, data });
}

/**
 * Send a message to the Rust backend, which spawns the Claude CLI.
 * sessionId is our internal session ID; the Rust backend resolves it
 * to the real claude session UUID for --resume.
 */
export interface SendOptions {
  planMode?: boolean;
  autoMode?: boolean;
  permissionMode?: string;
  effort?: string;
  /** ultracode: xhigh effort + auto Workflow orchestration (harness-level, not an API param) */
  ultracode?: boolean;
  /** Model name (e.g. deepseek-v4-pro[1M]), passed to CLI via --model */
  model?: string;
  /** File paths to attach (parent dirs are added via --add-dir) */
  filePaths?: string[];
  /** Manual claude CLI path (overrides auto-detect) */
  claudePath?: string;
  /** Working directory (overrides session cwd, used when workspace changes) */
  cwd?: string;
  /** Resume a specific claude session (for forking) */
  resumeId?: string;
  /** Use --fork-session to branch from the resumed session */
  forkSession?: boolean;
}

export async function sendMessage(sessionId: string, message: string, options?: SendOptions): Promise<string> {
  return invoke("send_message", {
    sessionId,
    message,
    planMode: options?.planMode ?? false,
    autoMode: options?.autoMode ?? true,
    permissionMode: options?.permissionMode ?? "bypassPermissions",
    effort: options?.effort ?? "high",
    ultracode: options?.ultracode ?? false,
    model: options?.model ?? null,
    filePaths: options?.filePaths ?? null,
    claudePath: options?.claudePath ?? null,
    cwd: options?.cwd ?? null,
    resumeId: options?.resumeId ?? null,
    forkSession: options?.forkSession ?? false,
  });
}

export interface SessionCreatedEvent {
  ourId: string;
  claudeSessionId: string;
}

/**
 * Store the claude session UUID on the Rust side so subsequent
 * send_message calls can use --resume.
 */
export async function storeClaudeSession(
  ourSessionId: string,
  claudeSessionId: string
): Promise<void> {
  return invoke("store_claude_session", { ourSessionId, claudeSessionId });
}

/**
 * Stop a running session (kill the claude process).
 */
export async function stopSession(sessionId: string): Promise<void> {
  return invoke("stop_session", { sessionId });
}

// ── Session management ──

export interface SessionData {
  id: string;
  title: string;
  cli_session_id: string | null;
  cwd: string;
  model: string;
  status: string;
  mode: string;
  created_at: string;
  updated_at: string;
  message_count: number;
  total_tokens: number | null;
  total_cost: number | null;
}

export interface MessageData {
  id: string;
  session_id: string;
  role: string;
  content: string;
  token_usage: string;
  created_at: string;
}

export async function createSession(model?: string, cwd?: string, mode?: string, title?: string): Promise<SessionData> {
  return invoke("create_session", { model: model ?? null, cwd: cwd ?? null, mode: mode ?? null, title: title ?? null });
}

export async function listSessions(): Promise<SessionData[]> {
  return invoke("list_sessions");
}

export async function deleteSession(sessionId: string): Promise<void> {
  return invoke("delete_session", { sessionId });
}

export async function renameSession(sessionId: string, title: string): Promise<void> {
  return invoke("rename_session", { sessionId, title });
}

export async function getSession(sessionId: string): Promise<SessionData> {
  return invoke("get_session", { sessionId });
}

export async function listMessages(sessionId: string): Promise<MessageData[]> {
  return invoke("list_messages", { sessionId });
}

/**
 * Test connection to the DeepSeek API
 */
export interface ConnectionTestResult {
  cc: string;
  chat: string | null;
}

export async function connectLLM(
  apiKey: string,
  baseUrl: string,
  model: string,
  providerId: string,
  optimizeApiUrl?: string,
): Promise<ConnectionTestResult> {
  return invoke("connect_llm", { apiKey, baseUrl, model, providerId, optimizeApiUrl: optimizeApiUrl || null });
}

// ── Approved Scenarios ──

export async function addApprovedScenario(toolName: string, pattern: string): Promise<void> {
  return invoke("add_approved_scenario", { toolName, pattern });
}

export async function removeApprovedScenario(toolName: string, pattern: string): Promise<void> {
  return invoke("remove_approved_scenario", { toolName, pattern });
}

export async function listApprovedScenarios(): Promise<Array<{ tool_name: string; pattern: string }>> {
  return invoke("list_approved_scenarios");
}

/**
 * Save a message to the backend SQLite store.
 */
export async function saveMessage(
  id: string,
  sessionId: string,
  role: string,
  content: string,
  tokenUsage?: string,
): Promise<void> {
  return invoke("save_message", { id, sessionId, role, content, tokenUsage: tokenUsage || "{}" });
}

export async function updateMessageContent(
  messageId: string,
  sessionId: string,
  content: string,
): Promise<void> {
  return invoke("update_message_content", { messageId, sessionId, content });
}

export async function deleteMessagesAfter(
  messageId: string,
  sessionId: string,
): Promise<number> {
  return invoke("delete_messages_after", { messageId, sessionId });
}

// ── File operations ──

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

export async function listDir(path: string): Promise<FileEntry[]> {
  return invoke("list_dir", { path });
}

export async function readFileContent(path: string): Promise<string> {
  return invoke("read_file_content", { path });
}

export async function getWorkspaceRoot(): Promise<string> {
  return invoke("get_workspace_root");
}

export async function writeFile(path: string, content: string): Promise<void> {
  return invoke("write_file", { path, content });
}

/** 保存文件内容（无路径限制，供编辑保存使用） */
export async function saveFileContent(path: string, content: string): Promise<void> {
  return invoke("save_file_content", { path, content });
}

/** 删除文件或目录 */
export async function deleteFile(path: string): Promise<void> {
  return invoke("delete_file", { path });
}

/** 重命名文件，返回新路径 */
export async function renameFile(path: string, newName: string): Promise<string> {
  return invoke("rename_file", { path, newName });
}

/** 移动文件到目标目录，返回新路径 */
export async function moveFile(path: string, destDir: string): Promise<string> {
  return invoke("move_file", { path, destDir });
}

/** 复制文件到目标目录，返回新路径 */
export async function copyFile(path: string, destDir: string): Promise<string> {
  return invoke("copy_file", { path, destDir });
}

/** 创建目录 */
export async function createDir(path: string): Promise<void> {
  return invoke("create_dir", { path });
}

export async function getClaudeDir(): Promise<string> {
  return invoke("get_claude_dir");
}

/** 返回 claude CLI 的自动检测路径（不依赖用户手动配置） */
export async function resolveClaudePath(): Promise<string> {
  return invoke("resolve_claude_path");
}
/** 一键安装 Claude Code CLI，返回退出码（0 成功） */
export async function installClaudeCode(): Promise<number> {
  return invoke("install_claude_code");
}

/** 用 LLM 优化用户输入的提示词 */
export async function optimizePrompt(apiKey: string, baseUrl: string, prompt: string, optimizeUrl?: string): Promise<string> {
  return invoke("optimize_prompt", { apiKey, baseUrl, prompt, optimizeUrl: optimizeUrl || null });
}

/** 禅模式：直接调 LLM chat/completions API（SSE 流式），绕过 CC CLI */
export async function zenSendMessage(
  sessionId: string,
  message: string,
  apiKey: string,
  chatUrl: string,
  model: string,
): Promise<string> {
  return invoke("zen_send_message", { sessionId, message, apiKey, chatUrl, model });
}

/** 从 ~/.claude/settings.json 读取配置 */
export async function getClaudeSettings(): Promise<{
  api_key: string; base_url: string; model: string; effort: string; permission_mode: string;
  provider_id: string; models: string[];
}> {
  return invoke("get_claude_settings");
}

/** 将配置写入 ~/.claude/settings.json */
export async function setClaudeSettings(
  apiKey: string, baseUrl: string, model: string, effort: string, permissionMode: string,
  providerId: string,
): Promise<void> {
  return invoke("set_claude_settings", { apiKey, baseUrl, model, effort, permissionMode, providerId });
}

/** 保存单个 provider 配置到 SQLite，切换前调用 */
export async function saveProviderConfig(
  providerId: string, apiKey: string, baseUrl: string, model: string,
): Promise<void> {
  return invoke("save_provider_config", { providerId, apiKey, baseUrl, model });
}

/** 加载所有已保存的 provider 配置 */
export async function loadProviderConfigs(): Promise<Record<string, { apiKey: string; baseUrl: string; model: string }>> {
  return invoke("load_provider_configs");
}

// ── UI 设置持久化（SQLite，不受 Tauri identifier 变更影响）──

/** 保存前端 UI 设置到 SQLite（JSON blob） */
export async function saveUiSettings(json: string): Promise<void> {
  return invoke("save_ui_settings", { json });
}

/** 从 SQLite 加载前端 UI 设置，无记录返回 "{}" */
export async function loadUiSettings(): Promise<string> {
  return invoke("load_ui_settings");
}

// ── 项目描述（翻译 + 缓存）──

export interface DescriptionItem {
  item_type: string;
  name: string;
  desc_en?: string | null;
  desc_zh?: string | null;
}

/**
 * 确保每个 item 都有中英双语描述。DB 有缓存直接返回，
 * 缺中文的自动调用 DeepSeek API 翻译后存入 DB。
 */
/** 清空所有翻译缓存 */
export async function clearItemDescriptions(): Promise<void> {
  return invoke("clear_item_descriptions");
}

/** 只清空 MCP 描述缓存 */
export async function clearMcpDescriptions(): Promise<void> {
  return invoke("clear_mcp_descriptions");
}

/**
 * 用 DeepSeek API 为 MCP 服务器名称批量生成中文描述，缓存到 DB。
 * 返回 name→desc_zh 的映射列表。
 */
export async function generateMcpDescriptions(
  names: string[],
  apiKey: string,
  baseUrl: string,
  optimizeApiUrl?: string,
): Promise<DescriptionItem[]> {
  return invoke("generate_mcp_descriptions", { names, apiKey, baseUrl, optimizeApiUrl: optimizeApiUrl || null });
}

export async function ensureItemDescriptions(
  items: DescriptionItem[],
  apiKey: string,
  baseUrl: string,
  optimizeApiUrl?: string,
): Promise<DescriptionItem[]> {
  return invoke("ensure_item_descriptions", { items, apiKey, baseUrl, optimizeApiUrl: optimizeApiUrl || null });
}

export async function revealInExplorer(path: string): Promise<void> {
  return invoke("reveal_in_explorer", { path });
}

/** Check if auto mode is active in settings.json (may have been changed externally) */
export async function getAutoModeStatus(): Promise<boolean> {
  return invoke("get_auto_mode_status");
}

/** Read a file as base64-encoded string (for image thumbnails) */
export async function readFileBase64(path: string): Promise<string> {
  return invoke("read_file_base64", { path });
}

/** 检查 skill 是否已安装（Rust 后端扫描 ~/.claude/skills/ + 插件缓存） */
export async function checkSkillInstalled(name: string): Promise<boolean> {
  return invoke("check_skill_installed", { name });
}

/** 持久化会话 debug 日志 */
export async function saveSessionDebugLog(sessionId: string, linesJson: string): Promise<void> {
  return invoke("save_session_debug_log", { sessionId, linesJson });
}

/** 持久化会话 stderr 日志 */
export async function saveSessionStderrLog(sessionId: string, linesJson: string): Promise<void> {
  return invoke("save_session_stderr_log", { sessionId, linesJson });
}

/** 加载会话日志（返回 [debugJson, stderrJson] 或 null） */
export async function loadSessionLogs(sessionId: string): Promise<[string | null, string | null]> {
  return invoke("load_session_logs", { sessionId });
}

/**
 * Emitted by Rust when a claude process exits.
 */
export interface ProcessExitedEvent {
  session_id: string;
  exit_code: number | null;
  success: boolean;
}
