import sqlite3, json, os

db_path = os.path.join(os.environ["APPDATA"], "cc-gui", "sessions.db")
conn = sqlite3.connect(db_path)
conn.row_factory = sqlite3.Row

# 最近一次会话
sessions = conn.execute(
    "SELECT * FROM sessions ORDER BY updated_at DESC LIMIT 1"
).fetchall()
latest = sessions[0]

messages = conn.execute(
    "SELECT * FROM messages WHERE session_id=? AND role='assistant' ORDER BY created_at ASC",
    (latest["id"],),
).fetchall()

for i, m in enumerate(messages):
    try:
        content = json.loads(m["content"])
        text = content.get("text", "") if isinstance(content, dict) else str(m["content"])
        # 只输出包含升级方案的消息
        keywords = ["升级方案", "命令面板", "CommandPalette", "P0", "i18n", "分组",
                     "命令分类", "Claude Code", "对标", "设计哲学", "6 个大类"]
        if any(kw in text for kw in keywords):
            print(f"===== 消息 [{i+1}] {m['id']} =====")
            print(text)
            print()
    except Exception as e:
        pass

conn.close()
