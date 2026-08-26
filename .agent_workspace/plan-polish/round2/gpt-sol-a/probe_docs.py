#!/usr/bin/env python3
"""Round 2 read-only probes for plan authority and schema integrity."""

from __future__ import annotations

import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
DOCS = ROOT / "docs"
SCHEMA_DIR = DOCS / "schemas"
LOCK_PATH = SCHEMA_DIR / "schemas.lock.json"
SCAN_PATHS = (
    DOCS / "PRODUCT_LOCK.md",
    DOCS / "DECISIONS.md",
    DOCS / "FORMAL_WORK_PROMPT.md",
    DOCS / "STATUS.md",
    ROOT / "README.md",
    DOCS / "PLAN_INDEX.md",
)
TOKENS = ("180", "360", "MODERATE_MIN", "T0", "尚未写应用代码", "v0.1 实现未开始")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def display(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def schema_probe() -> bool:
    print("== 1. schema lock integrity ==")
    lock = json.loads(LOCK_PATH.read_text(encoding="utf-8"))
    expected = lock.get("schemas", {})
    actual_paths = {
        path.name: path
        for path in SCHEMA_DIR.glob("*.json")
        if path.name != LOCK_PATH.name
    }
    failures: list[str] = []

    if lock.get("algorithm") != "sha256":
        failures.append(f"unsupported lock algorithm: {lock.get('algorithm')!r}")

    missing_files = sorted(set(expected) - set(actual_paths))
    unlocked_files = sorted(set(actual_paths) - set(expected))
    if missing_files:
        failures.append(f"missing schema files: {', '.join(missing_files)}")
    if unlocked_files:
        failures.append(f"schema files absent from lock: {', '.join(unlocked_files)}")

    for name in sorted(set(expected) & set(actual_paths)):
        actual_hash = sha256(actual_paths[name])
        state = "MATCH" if actual_hash == expected[name] else "MISMATCH"
        print(f"{state} {name} expected={expected[name]} actual={actual_hash}")
        if state == "MISMATCH":
            failures.append(name)

    print(
        f"RESULT {'PASS' if not failures else 'FAIL'}: "
        f"{len(actual_paths)} schema documents, {len(expected)} lock entries"
    )
    for failure in failures:
        print(f"ERROR {failure}")
    print()
    return not failures


def literal_probe() -> dict[str, list[tuple[str, int, str]]]:
    print("== 2. requested literal scan ==")
    hits: dict[str, list[tuple[str, int, str]]] = {token: [] for token in TOKENS}
    for path in SCAN_PATHS:
        for line_number, line in enumerate(
            path.read_text(encoding="utf-8").splitlines(), start=1
        ):
            for token in TOKENS:
                if token in line:
                    hits[token].append((display(path), line_number, line.strip()))

    for token in TOKENS:
        token_hits = hits[token]
        print(f"{token!r}: {len(token_hits)} hit(s)")
        for path, line_number, line in token_hits:
            print(f"  {path}:{line_number}: {line}")
    print()
    return hits


def dual_source_probe() -> bool:
    print("== 3. PRODUCT_LOCK dual-source hash ==")
    authority = DOCS / "PRODUCT_LOCK.md"
    context_copy = ROOT / ".agent_workspace/context/plan/PRODUCT_LOCK.md"
    authority_hash = sha256(authority)
    context_hash = sha256(context_copy)
    diverged = authority_hash != context_hash
    print(f"{display(authority)} sha256={authority_hash}")
    print(f"{display(context_copy)} sha256={context_hash}")
    print(f"RESULT {'PASS' if diverged else 'FAIL'}: {'DIVERGED' if diverged else 'IDENTICAL'}")
    print()
    return diverged


def formal_ac_probe() -> bool:
    print("== 4. FORMAL AC-28..AC-33 rows ==")
    formal = (DOCS / "FORMAL_WORK_PROMPT.md").read_text(encoding="utf-8")
    ids = re.findall(r"^\|\s*(AC-\d{2})\s*\|", formal, flags=re.MULTILINE)
    counts = Counter(ids)
    required = [f"AC-{number:02d}" for number in range(28, 34)]
    failures = []
    for ac_id in required:
        count = counts[ac_id]
        state = "PASS" if count == 1 else "FAIL"
        print(f"{state} {ac_id} row_count={count}")
        if count != 1:
            failures.append(ac_id)
    print(f"RESULT {'PASS' if not failures else 'FAIL'}")
    print()
    return not failures


def decision_id_probe() -> bool:
    print("== 5. DECISIONS D32..D58 rows and duplicate IDs ==")
    decisions = (DOCS / "DECISIONS.md").read_text(encoding="utf-8")
    ids = [int(value) for value in re.findall(r"^\|\s*D(\d+)\s*\|", decisions, flags=re.MULTILINE)]
    counts = Counter(ids)
    required = list(range(32, 59))
    missing = [value for value in required if counts[value] == 0]
    duplicates = sorted(value for value, count in counts.items() if count > 1)
    print(f"parsed_rows={len(ids)} unique_ids={len(counts)} range=D{min(ids)}..D{max(ids)}")
    print(f"D32..D58 missing={','.join(f'D{x}' for x in missing) if missing else 'none'}")
    print(
        "duplicate table IDs="
        + (", ".join(f"D{x} (count={counts[x]})" for x in duplicates) if duplicates else "none")
    )
    passed = not missing and not duplicates
    print(f"RESULT {'PASS' if passed else 'FAIL'}")
    print()
    return passed


def main() -> int:
    checks = [schema_probe()]
    literal_probe()
    checks.extend(
        [
            dual_source_probe(),
            formal_ac_probe(),
            decision_id_probe(),
        ]
    )
    print(f"MECHANICAL_RESULT {'PASS' if all(checks) else 'FAIL'}")
    return 0 if all(checks) else 1


if __name__ == "__main__":
    sys.exit(main())
