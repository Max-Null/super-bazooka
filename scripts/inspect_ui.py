"""Inspect cc-gui frontend layout via Playwright."""
import sys, os, json
from playwright.sync_api import sync_playwright

OUT = {}
with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    page = browser.new_page(viewport={"width": 1400, "height": 900})

    # Capture console errors
    errors = []
    page.on("console", lambda msg: errors.append(f"[{msg.type}] {msg.text}") if msg.type in ("error","warning") else None)

    page.goto("http://localhost:1421")
    page.wait_for_load_state("networkidle")
    page.wait_for_timeout(500)

    page.screenshot(path="scripts/welcome.png", full_page=False)
    OUT["welcome_html"] = page.locator("#app").inner_html()

    # Send a message
    page.locator("textarea").fill("Hello! Explain async/await in one sentence.")
    page.locator("button:has-text('发送')").click()
    page.wait_for_timeout(300)

    # Simulate streaming
    page.evaluate("""
        const m = window.__ccgui_mock;
        if (m) {
            m._emit("stream-event", {type:"assistant",text:"Async/await makes asynchronous code read like synchronous code.",thinking:"User wants one sentence.",tool_use:null,is_final:false,error:null});
            m._emit("stream-event", {type:"done",text:"",thinking:"",tool_use:null,is_final:true,error:null});
        }
    """)
    page.wait_for_timeout(500)

    page.screenshot(path="scripts/chat.png", full_page=False)
    OUT["chat_html"] = page.locator("#app").inner_html()

    # Diagnostics
    OUT["header_box"] = str(page.locator("header").bounding_box())
    OUT["main_box"] = str(page.locator("main").bounding_box())
    OUT["textarea_box"] = str(page.locator("textarea").bounding_box())
    OUT["send_btn_box"] = str(page.locator("button:has-text('发送')").bounding_box())
    OUT["message_count"] = page.locator(".message-enter").count()
    OUT["console_errors"] = errors

    # Tailwind check
    OUT["tw_bg"] = page.evaluate("""() => {
        const el = document.querySelector('.bg-gray-950');
        return el ? getComputedStyle(el).backgroundColor : 'NOT FOUND';
    }""")
    OUT["body_styles"] = page.evaluate("""() => {
        const s = getComputedStyle(document.body);
        return {bg:s.backgroundColor, color:s.color, font:s.fontFamily};
    }""")

    browser.close()

# Write results to JSON file (avoids encoding issues)
with open("scripts/ui_report.json", "w", encoding="utf-8") as f:
    json.dump(OUT, f, indent=2, ensure_ascii=False)

print("Report saved: scripts/ui_report.json")
print("Screenshots: scripts/welcome.png, scripts/chat.png")
print(f"Messages: {OUT['message_count']}")
print(f"Console errors: {len(OUT['console_errors'])}")
for e in OUT['console_errors'][:10]:
    print(f"  {e}")
