#!/usr/bin/env python3
"""从 ECDICT stardict.db 裁剪常用词到 src-tauri/resources/ecdict.db。

用法:
  python3 scripts/build-ecdict.py /path/to/stardict.db
"""
from __future__ import annotations

import os
import sqlite3
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
DST = os.path.join(ROOT, "src-tauri", "resources", "ecdict.db")


def main() -> None:
    if len(sys.argv) < 2:
        print("usage: build-ecdict.py /path/to/stardict.db", file=sys.stderr)
        sys.exit(1)
    src = sys.argv[1]
    os.makedirs(os.path.dirname(DST), exist_ok=True)
    if os.path.exists(DST):
        os.remove(DST)
    sin = sqlite3.connect(src)
    sout = sqlite3.connect(DST)
    sout.execute(
        """CREATE TABLE stardict (
          word TEXT PRIMARY KEY COLLATE NOCASE,
          phonetic TEXT,
          translation TEXT,
          pos TEXT,
          exchange TEXT
        )"""
    )
    rows = sin.execute(
        """
        SELECT word, phonetic, translation, pos, exchange FROM stardict
        WHERE translation IS NOT NULL AND trim(translation) != ''
        AND (
          oxford=1 OR collins>=1
          OR (frq IS NOT NULL AND frq>0 AND frq<=10000)
          OR (bnc IS NOT NULL AND bnc>0 AND bnc<=10000)
          OR (tag IS NOT NULL AND tag != '' AND (
            tag LIKE '%cet4%' OR tag LIKE '%cet6%' OR tag LIKE '%ielts%' OR tag LIKE '%toefl%'
            OR tag LIKE '%zk%' OR tag LIKE '%gk%' OR tag LIKE '%ky%' OR tag LIKE '%gre%'
          ))
        )
        """
    ).fetchall()
    sout.executemany(
        "INSERT OR IGNORE INTO stardict(word,phonetic,translation,pos,exchange) VALUES (?,?,?,?,?)",
        rows,
    )
    sout.execute("CREATE INDEX idx_word ON stardict(word)")
    sout.commit()
    sout.execute("VACUUM")
    sout.close()
    sin.close()
    print(f"wrote {DST} entries={len(rows)} size_mb={os.path.getsize(DST)/1024/1024:.2f}")


if __name__ == "__main__":
    main()
