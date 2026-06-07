/**
 * Real-stream replay test — verifies the frontend correctly renders
 * actual claude CLI stream-json output.
 *
 * Fixtures recorded via: bash scripts/record_claude_output.sh "prompt"
 * Run: npx playwright test e2e/real-stream.spec.ts
 */
import { test, expect } from "@playwright/test";
import { readFileSync, readdirSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const FIXTURE_DIR = resolve(dirname(fileURLToPath(import.meta.url)), "fixtures");

function findLatestFixture() {
  const files = readdirSync(FIXTURE_DIR)
    .filter((f) => f.startsWith("real-stream-") && f.endsWith(".jsonl"))
    .sort().reverse();
  if (files.length === 0) return null;
  const path = resolve(FIXTURE_DIR, files[0]);
  const events = readFileSync(path, "utf-8").split("\n")
    .filter((l) => l.trim()).map((l) => JSON.parse(l));
  return { path, events };
}

// Helper: emit events through the browser's mock bridge
async function emit(page: import("@playwright/test").Page, event: string, payload: unknown) {
  await page.evaluate(({ event, payload }) => {
    const mock = (window as unknown as Record<string, unknown>).__ccgui_mock as {
      _emit: (e: string, p: unknown) => void;
    };
    mock._emit(event, payload);
  }, { event, payload });
}

// Helper: setup mock handler in browser
async function setupMock(page: import("@playwright/test").Page) {
  await page.evaluate(() => {
    const mock = (window as unknown as Record<string, unknown>).__ccgui_mock as {
      mockInvokeHandlers: Map<string, (args: Record<string, unknown>) => unknown>;
    };
    if (mock) mock.mockInvokeHandlers.set("send_message", () => "Complete");
  });
}

// ── Fixture validation (no browser needed) ──

test("fixture exists and has expected structure", () => {
  const f = findLatestFixture();
  expect(f).not.toBeNull();
  expect(f!.events.length).toBeGreaterThan(5);
  const types = f!.events.map((e: Record<string, unknown>) => e.type);
  expect(types).toContain("system");
  expect(types).toContain("assistant");
  expect(types).toContain("result");
});

// ── Browser replay tests ──

test.describe("real stream replay in browser", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await setupMock(page);
  });

  test("assistant text from real fixture renders correctly", async ({ page }) => {
    const f = findLatestFixture();
    if (!f) { test.skip(true, "No fixture"); return; }

    // Create user message first
    await page.locator("textarea").fill("Hello");
    // Click send button (SVG icon inside input bar, last button in DOM)
    await page.locator("textarea ~ button").click();

    // Replay real assistant events
    for (const event of f.events) {
      const e = event as Record<string, unknown>;
      if (e.type !== "assistant" && e.type !== "result") continue;

      if (e.type === "assistant") {
        const content = (e.message as Record<string, unknown>)?.content as Array<Record<string, unknown>>;
        let text = "", thinking = "";
        const toolUses: unknown[] = [];
        if (content) {
          for (const b of content) {
            if (b.type === "text" && b.text) text += b.text;
            if (b.type === "thinking" && b.thinking) thinking += b.thinking;
            if (b.type === "tool_use") toolUses.push(b);
          }
        }
        if (text || thinking) {
          await emit(page, "stream-event", {
            type: "assistant", text, thinking,
            tool_use: toolUses.length > 0 ? toolUses : null,
            is_final: false, error: null,
          });
        }
      }

      if (e.type === "result") {
        await emit(page, "stream-event", {
          type: "done", text: "", thinking: "", tool_use: null, is_final: true, error: null,
        });
      }
    }

    await page.waitForTimeout(300);

    // Verify AI avatar rendered
    await expect(page.getByText("Claude")).toBeVisible();

    // Screenshot for OCR
    await page.screenshot({ path: "e2e/fixtures/real-stream-output.png" });
  });

  test("real thinking content is collapsible", async ({ page }) => {
    const f = findLatestFixture();
    if (!f) { test.skip(true, "No fixture"); return; }

    // Check if fixture has thinking
    const hasThinking = f.events.some((e: Record<string, unknown>) => {
      if (e.type !== "assistant") return false;
      const content = (e.message as Record<string, unknown>)?.content as Array<Record<string, unknown>>;
      return content?.some((b) => b.type === "thinking");
    });

    if (!hasThinking) {
      test.skip(true, "Fixture has no thinking — use a model that outputs thinking");
      return;
    }

    await page.locator("textarea").fill("Think");
    // Click send button (SVG icon inside input bar, last button in DOM)
    await page.locator("textarea ~ button").click();

    // Replay only the first assistant event with thinking
    for (const event of f.events) {
      const e = event as Record<string, unknown>;
      if (e.type !== "assistant") continue;
      const content = (e.message as Record<string, unknown>)?.content as Array<Record<string, unknown>>;
      let text = "", thinking = "";
      if (content) {
        for (const b of content) {
          if (b.type === "text" && b.text) text += b.text;
          if (b.type === "thinking" && b.thinking) thinking += b.thinking;
        }
      }
      if (thinking) {
        await emit(page, "stream-event", {
          type: "assistant", text, thinking, tool_use: null, is_final: false, error: null,
        });
        break;
      }
    }

    // Thinking content from real fixture should be in the DOM
    await expect(page.locator("details")).toBeVisible({ timeout: 3000 });
  });
});
