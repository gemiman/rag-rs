# 压测后清理测试数据：删除所有 loadtest_ 前缀的用户及其会话、消息
# 用法（在项目根目录运行）：python scripts/loadtest/cleanup.py
# 会自动备份数据库后再清理，避免误删真实数据
import sqlite3
import shutil
import datetime
import os

# 数据库路径：scripts/loadtest/ -> 项目根/backend/data/rag.db
DB = os.path.normpath(os.path.join(os.path.dirname(__file__), '..', '..', 'backend', 'data', 'rag.db'))


def main():
    if not os.path.exists(DB):
        print(f'数据库不存在: {DB}')
        return

    # 1. 先备份
    bak = f'{DB}.bak_{datetime.datetime.now().strftime("%Y%m%d_%H%M%S")}'
    shutil.copy(DB, bak)
    print(f'已备份数据库: {bak}')

    # 2. 清理（按 消息 -> 会话 -> 用户 顺序，因为外键不级联删除）
    db = sqlite3.connect(DB)
    c = db.cursor()
    before = c.execute(
        "SELECT COUNT(*) FROM users WHERE username LIKE 'loadtest_%'"
    ).fetchone()[0]

    c.execute(
        "DELETE FROM messages WHERE conversation_id IN "
        "(SELECT id FROM conversations WHERE user_id IN "
        "(SELECT id FROM users WHERE username LIKE 'loadtest_%'))"
    )
    c.execute(
        "DELETE FROM conversations WHERE user_id IN "
        "(SELECT id FROM users WHERE username LIKE 'loadtest_%')"
    )
    c.execute("DELETE FROM users WHERE username LIKE 'loadtest_%'")
    db.commit()

    print(f'已删除 {before} 个测试用户及其会话/消息')
    print('剩余用户:', c.execute('SELECT id, username FROM users ORDER BY id').fetchall())
    db.close()


if __name__ == '__main__':
    main()
