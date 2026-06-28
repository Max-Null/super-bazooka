/**
 * Mock Tauri APIs for Playwright E2E testing.
 * Replaces @tauri-apps/api/core so the app runs in a regular browser.
 *
 * Usage: set VITE_TAURI_MOCK=true before running vite dev,
 * or use the playwright config's alias.
 */

interface MockListener {
  (event: string, handler: (data: unknown) => void): Promise<() => void>;
}

// Store registered listeners and mock responses
const listeners: Map<string, Array<(data: unknown) => void>> = new Map();
let mockInvokeHandlers: Map<string, (args: Record<string, unknown>) => unknown> = new Map();

/**
 * Register a mock for `invoke(command, args)`
 */
export function mockInvoke(command: string, handler: (args: Record<string, unknown>) => unknown) {
  mockInvokeHandlers.set(command, handler);
}

/**
 * Emit a mock event to all registered listeners
 */
export function mockEmit(event: string, payload: unknown) {
  const handlers = listeners.get(event) || [];
  for (const handler of handlers) {
    handler(payload);
  }
}

/**
 * Clear all mocks and listeners (call between tests)
 */
export function resetMocks() {
  listeners.clear();
  mockInvokeHandlers.clear();
}

// --- Fake @tauri-apps/api/core ---

export async function invoke(command: string, args?: Record<string, unknown>): Promise<unknown> {
  const handler = mockInvokeHandlers.get(command);
  if (handler) {
    const result = handler(args || {});
    // If it throws, propagate; if it's a value, return it
    return result;
  }
  console.warn(`[tauri-mock] Unmocked invoke: ${command}`, args);
  return "mocked-ok";
}

// --- Fake @tauri-apps/api/event ---

export async function listen<T>(
  event: string,
  handler: (event: { payload: T }) => void
): Promise<() => void> {
  if (!listeners.has(event)) {
    listeners.set(event, []);
  }
  const wrappedHandler = (data: unknown) => handler({ payload: data as T });
  listeners.get(event)!.push(wrappedHandler);

  // Return unlisten function
  return () => {
    const handlers = listeners.get(event) || [];
    const idx = handlers.indexOf(wrappedHandler);
    if (idx >= 0) handlers.splice(idx, 1);
  };
}

// Default mocks for Tauri commands that have no registered handler
if (typeof window !== "undefined") {
  mockInvokeHandlers.set("get_claude_settings", () => ({
    api_key: "",
    base_url: "https://api.deepseek.com",
    model: "deepseek-v4-pro[1M]",
    effort: "high",
    permission_mode: "default",
  }));
  mockInvokeHandlers.set("set_claude_settings", () => undefined);
  mockInvokeHandlers.set("save_provider_config", () => undefined);
  mockInvokeHandlers.set("load_provider_configs", () => ({}));
  mockInvokeHandlers.set("clear_item_descriptions", () => undefined);
  mockInvokeHandlers.set("ensure_item_descriptions", (args: any) => {
    // Return items unchanged (no translation in mock)
    return (args.items || []).map((it: any) => ({
      ...it,
      desc_zh: null,
    }));
  });
}

// Window.__TAURI__ mock (for any Tauri internals)
if (typeof window !== "undefined") {
  (window as unknown as Record<string, unknown>).__TAURI__ = {
    invoke,
  };
  // Debug hooks for Playwright tests
  (window as unknown as Record<string, unknown>).__ccgui_mock = {
    listeners,
    mockInvokeHandlers,
    _emit: mockEmit,
    _invoke: invoke,
    _listen: listen,
  };
  console.log("[tauri-mock] Loaded. All Tauri APIs are mocked for browser testing.");
}
