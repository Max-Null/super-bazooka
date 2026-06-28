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

export async function createSession(model?: string): Promise<SessionData> {
  return invoke("create_session", { model: model ?? null });
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
export async function connectLLM(
  apiKey: string,
  baseUrl: string,
  model: string,
  providerId: string,
): Promise<string> {
  return invoke("connect_llm", { apiKey, baseUrl, model, providerId });
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

export async function getClaudeDir(): Promise<string> {
  return invoke("get_claude_dir");
}

/** 返回 claude CLI 的自动检测路径（不依赖用户手动配置） */
export async function resolveClaudePath(): Promise<string> {
  return invoke("resolve_claude_path");
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
): Promise<DescriptionItem[]> {
  return invoke("generate_mcp_descriptions", { names, apiKey, baseUrl });
}

export async function ensureItemDescriptions(
  items: DescriptionItem[],
  apiKey: string,
  baseUrl: string,
): Promise<DescriptionItem[]> {
  return invoke("ensure_item_descriptions", { items, apiKey, baseUrl });
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

/**
 * Emitted by Rust when a claude process exits.
 */
export interface ProcessExitedEvent {
  session_id: string;
  exit_code: number | null;
  success: boolean;
}
