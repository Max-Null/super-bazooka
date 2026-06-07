import { test, expect } from "@playwright/test";

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

test.describe("visual rendering", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector("#app");
    await page.waitForTimeout(200);
    await page.evaluate(() => {
      const mock = (window as unknown as Record<string, unknown>).__ccgui_mock as {
        mockInvokeHandlers: Map<string, (args: Record<string, unknown>) => unknown>;
      };
      mock.mockInvokeHandlers.set("send_message", () => "Complete");
    });
  });

  test("welcome screen screenshot", async ({ page }) => {
    await expect(page).toHaveScreenshot("welcome-screen.png");
  });

  test("chat with mixed content screenshot", async ({ page }) => {
    const textarea = page.locator("textarea");
    await textarea.fill("Explain async/await");
    await page.locator("textarea ~ button").click();

    await emit(page, "stream-event", {
      type: "assistant",
      text: "Async/await handles async code.\n\n```ts\nasync function f() {\n  const r = await fetch(url);\n  return r.json();\n}\n```",
      thinking: "Let me explain with a code example.",
      tool_use: null,
      is_final: false,
      error: null,
    });

    await emit(page, "stream-event", {
      type: "done", text: "", thinking: "", tool_use: null, is_final: true, error: null,
    });

    await page.waitForTimeout(300);
    await expect(page).toHaveScreenshot("chat-response.png");
  });

  test("tool use card renders", async ({ page }) => {
    const textarea = page.locator("textarea");
    await textarea.fill("List files");
    await page.locator("textarea ~ button").click();

    await emit(page, "stream-event", {
      type: "assistant",
      text: "",
      thinking: "",
      tool_use: [{ id: "tu_1", name: "Bash", input: { command: "ls -la" } }],
      is_final: false,
      error: null,
    });

    await page.waitForTimeout(300);
    // Expand collapsed <details>
    await page.getByText("Bash").click();
    await expect(page.getByText("ls -la").first()).toBeVisible();
  });

  test("debug panel shows with toggle", async ({ page }) => {
    const textarea = page.locator("textarea");
    await textarea.fill("Debug me");
    await page.locator("textarea ~ button").click();

    await emit(page, "stream-debug", JSON.stringify({ type: "system", subtype: "init" }));

    // Use role-based locator to avoid matching "Debug me" message text
    const debugToggle = page.getByRole("button", { name: /Debug/ });
    await expect(debugToggle).toBeVisible();
    await debugToggle.click();
    await expect(page.getByText("system")).toBeVisible();
  });
});
