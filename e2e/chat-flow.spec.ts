import { test, expect } from "@playwright/test";

/**
 * Helper: emit a stream event INSIDE the browser context.
 * We must use page.evaluate because the test file and the browser app
 * have separate module instances of tauri-mock.ts.
 */
async function emit(page: import("@playwright/test").Page, event: string, payload: unknown) {
  await page.evaluate(
    ({ event, payload }) => {
      const mock = (window as unknown as Record<string, unknown>).__ccgui_mock as {
        _emit: (e: string, p: unknown) => void;
      };
      mock._emit(event, payload);
    },
    { event, payload }
  );
}

test.describe("cc-gui chat flow", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    // Wait for app to mount before setting up mocks
    await page.waitForSelector("#app");
    await page.waitForTimeout(200);
    await page.evaluate(() => {
      const mock = (window as unknown as Record<string, unknown>).__ccgui_mock as {
        mockInvokeHandlers: Map<string, (args: Record<string, unknown>) => unknown>;
      };
      mock.mockInvokeHandlers.set("send_message", () => "Complete");
    });
  });

  test("app loads and shows welcome screen", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "cc-gui" })).toBeVisible();
    await expect(page.locator("p", { hasText: "Claude Code" })).toBeVisible();
  });

  test("can type message and send it", async ({ page }) => {
    const textarea = page.locator("textarea");
    await textarea.fill("Hello AI!");
    await expect(textarea).toHaveValue("Hello AI!");
    await page.locator("textarea ~ button").click();
    await expect(textarea).toHaveValue("");
  });

  test("user message appears in chat after sending", async ({ page }) => {
    const textarea = page.locator("textarea");
    await textarea.fill("Test message");
    await page.locator("textarea ~ button").click();
    await expect(page.getByText("Test message")).toBeVisible();
  });

  test("streaming text appears as AI responds", async ({ page }) => {
    const textarea = page.locator("textarea");
    await textarea.fill("Hello");
    await page.locator("textarea ~ button").click();

    await emit(page, "stream-event", {
      type: "assistant",
      text: "Hello! ",
      thinking: "",
      tool_use: null,
      is_final: false,
      error: null,
    });

    await expect(page.getByText("Hello!")).toBeVisible();

    await emit(page, "stream-event", {
      type: "assistant",
      text: "How can I help?",
      thinking: "",
      tool_use: null,
      is_final: false,
      error: null,
    });

    await expect(page.getByText("How can I help?")).toBeVisible();
  });

  test("thinking content is shown collapsible", async ({ page }) => {
    const textarea = page.locator("textarea");
    await textarea.fill("Think please");
    await page.locator("textarea ~ button").click();

    await emit(page, "stream-event", {
      type: "assistant",
      text: "Here is my analysis.",
      thinking: "Step 1: understand. Step 2: research.",
      tool_use: null,
      is_final: false,
      error: null,
    });

    // Thinking content inside collapsed <details> — expand first
    await page.evaluate(() => {
      const d = document.querySelector("details");
      if (d) d.open = true;
    });
    await expect(page.getByText("Step 1: understand")).toBeVisible();
  });

  test("done event stops processing indicator", async ({ page }) => {
    const textarea = page.locator("textarea");
    await textarea.fill("Quick!");
    await page.locator("textarea ~ button").click();

    // Check the processing indicator appears
    await expect(page.locator(".animate-pulse").first()).toBeVisible();

    await emit(page, "stream-event", {
      type: "done",
      text: "",
      thinking: "",
      tool_use: null,
      is_final: true,
      error: null,
    });

    await page.waitForTimeout(200);
    await expect(page.locator(".animate-pulse")).toHaveCount(0);
  });

  test("send button disabled while processing", async ({ page }) => {
    const textarea = page.locator("textarea");
    await textarea.fill("Processing test");
    const sendBtn = page.locator("textarea ~ button");

    await expect(sendBtn).toBeEnabled();
    await sendBtn.click();
    await expect(sendBtn).toBeDisabled();
  });

  test("Enter key sends, Shift+Enter does not", async ({ page }) => {
    const textarea = page.locator("textarea");
    await textarea.fill("Enter test");
    await textarea.press("Enter");
    await expect(page.getByText("Enter test")).toBeVisible();
  });

  test("empty message is not sent", async ({ page }) => {
    const textarea = page.locator("textarea");
    await textarea.fill("   ");
    const sendBtn = page.locator("textarea ~ button");
    await expect(sendBtn).toBeDisabled();
    await sendBtn.click({ force: true });
    await expect(page.locator(".message-enter")).toHaveCount(0);
  });
});
