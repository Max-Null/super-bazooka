import sqlite3, json, os, sys

db_path = os.path.join(os.environ["APPDATA"], "cc-gui", "sessions.db")
conn = sqlite3.connect(db_path)
conn.row_factory = sqlite3.Row

sessions = conn.execute(
    "SELECT * FROM sessions ORDER BY updated_at DESC LIMIT 5"
).fetchall()

print("=== 最近 5 个会话 ===")
for s in sessions:
    msg_cnt = conn.execute(
        "SELECT COUNT(*) FROM messages WHERE session_id=?", (s["id"],)
    ).fetchone()[0]
    print(f"ID:       {s['id']}")
    print(f"标题:     {s['title']}")
    print(f"模型:     {s['model']}")
    print(f"状态:     {s['status']}")
    print(f"CLI SID:  {s['cli_session_id']}")
    print(f"目录:     {s['cwd']}")
    print(f"创建:     {s['created_at']}")
    print(f"更新:     {s['updated_at']}")
    print(f"消息数:   {msg_cnt}")
    print()

latest = sessions[0]
print(f"===== 最近会话: [{latest['title']}] =====")
print(f"Session ID: {latest['id']}")
print(f"消息总数:   {len(conn.execute('SELECT id FROM messages WHERE session_id=?', (latest['id'],)).fetchall())}")
print()

messages = conn.execute(
    "SELECT * FROM messages WHERE session_id=? ORDER BY created_at ASC",
    (latest["id"],),
).fetchall()

for i, m in enumerate(messages):
    print(f"--- [{i+1}/{len(messages)}] [{m['role'].upper()}] {m['id']} ---")
    print(f"时间: {m['created_at']}")
    try:
        content = json.loads(m["content"])
        if isinstance(content, dict):
            text = content.get("text", "")
            thinking = content.get("thinking", "")
            if text:
                print(f"文本: {text[:800]}")
            if thinking:
                print(f"思考: {thinking[:400]}")
            tool_uses = content.get("toolUses", [])
            if tool_uses:
                for tu in tool_uses:
                    name = tu.get("name", "?")
                    tid = tu.get("id", "?")
                    inp = json.dumps(tu.get("input", {}), ensure_ascii=False)[:300]
                    print(f"工具: {name} (id={tid})")
                    print(f"  参数: {inp}")
            if "durationMs" in content:
                print(f"耗时: {content['durationMs']}ms")
            if "inputTokens" in content:
                print(f"Token: in={content.get('inputTokens',0)} out={content.get('outputTokens',0)}")
            if "costUSD" in content:
                print(f"费用: ${content['costUSD']}")
        elif isinstance(content, str):
            print(f"内容: {content[:800]}")
    except Exception:
        raw = str(m["content"])[:800]
        print(f"原始: {raw}")
    print()

conn.close()
