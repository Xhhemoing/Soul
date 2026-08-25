#!/usr/bin/env python3
"""重算 docs/schemas/schemas.lock.json。

本分支没有 `crates/xtask`（只有两个算法 crate），所以 `cargo run -p xtask --
schema-freeze --write` 在这里跑不了。本脚本复刻那个写法的字节形态：

- 摘要口径：先把 CRLF 折成 LF 再取 sha256（xtask `digest_file`）；
- 键序：`note` / `algorithm` / `schemas`，`schemas` 内按文件名字典序（BTreeMap）；
- 序列化：`serde_json::to_string_pretty`，两空格缩进，末尾补一个换行；
- 数量：恰好十一份，不含 lock 自身。

Goal 1 那条线仍应用 xtask 重算；本脚本只是让计划分支能自证一致。
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[5]
SCHEMAS_DIR = REPO / "docs" / "schemas"
LOCK_NAME = "schemas.lock.json"
EXPECTED_SCHEMA_COUNT = 11


def main() -> int:
    schemas_dir = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else SCHEMAS_DIR
    lock_path = schemas_dir / LOCK_NAME
    lock = json.loads(lock_path.read_text(encoding="utf-8"))

    digests = {
        p.name: hashlib.sha256(p.read_bytes().replace(b"\r\n", b"\n")).hexdigest()
        for p in sorted(schemas_dir.glob("*.json"))
        if p.name != LOCK_NAME
    }
    if len(digests) != EXPECTED_SCHEMA_COUNT:
        print(f"FATAL: 期望 {EXPECTED_SCHEMA_COUNT} 份 schema，实际 {len(digests)} 份：{sorted(digests)}")
        return 2

    rewritten = {"note": lock["note"], "algorithm": "sha256", "schemas": dict(sorted(digests.items()))}
    text = json.dumps(rewritten, indent=2, ensure_ascii=False) + "\n"
    changed = text != lock_path.read_text(encoding="utf-8")
    lock_path.write_text(text, encoding="utf-8")

    for name, digest in rewritten["schemas"].items():
        print(f"{digest}  {name}")
    print(f"\n{lock_path}: {'rewritten' if changed else 'unchanged'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
