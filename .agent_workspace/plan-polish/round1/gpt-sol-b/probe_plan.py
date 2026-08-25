#!/usr/bin/env python3
"""Extract v0.1 acceptance/slice coverage and scan algorithm-boundary terms."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


CORE_PLAN_FILES = (
    "README.md",
    "docs/PRODUCT_LOCK.md",
    "docs/DECISIONS.md",
    "docs/FORMAL_WORK_PROMPT.md",
    "docs/SECURITY.md",
    "docs/STATUS.md",
)

# This is an audit mapping, not a product change. Notes identify where an AC
# supports only part of a compound PRODUCT_LOCK item.
SLICE_TO_AC = {
    1: ["AC-01", "AC-02"],
    2: ["AC-03", "AC-04", "AC-05", "AC-25"],
    3: ["AC-03", "AC-06", "AC-08"],
    4: ["AC-07"],
    5: ["AC-14", "AC-15"],
    6: ["AC-16", "AC-17"],
    7: ["AC-09", "AC-10"],
    8: ["AC-07", "AC-11", "AC-12", "AC-13", "AC-17", "AC-25"],
    9: ["AC-18", "AC-19", "AC-25"],
    10: ["AC-20"],
    11: ["AC-11", "AC-17", "AC-21", "AC-22"],
    12: ["AC-14", "AC-15", "AC-23", "AC-24"],
    13: ["AC-01", "AC-26"],
}

SLICE_ASSESSMENT = {
    1: (
        "full",
        "AC-01 covers install/tray/no elevation; AC-02 covers default-off "
        "configuration after onboarding.",
    ),
    2: (
        "full",
        "AC-03/04/05 cover questionnaire and both named imports; AC-25 is the "
        "cross-cutting hostile-import test.",
    ),
    3: (
        "partial",
        "AC-03/06/08 cover a nonempty profile, evidence-linked inferences, and "
        "a graph. No AC explicitly checks editing axes/preferences/boundaries, "
        "the evidence-band field, or frozen T4D direct/group/as_of behavior.",
    ),
    4: (
        "partial",
        "AC-07 proves a user-edited tone changes drafting and is not overwritten. "
        "It does not exercise an A0 axis correction followed by questionnaire "
        "intake/replay, and no AC names relationship correction/correct_tie.",
    ),
    5: (
        "full",
        "AC-14 covers memory CRUD and content-free audit; AC-15 covers impact "
        "preview, key destruction, restart, orphaning, and retained audit.",
    ),
    6: (
        "partial",
        "AC-16/17 cover evidence, no diagnosis, and no-key fallback. No AC pins "
        "A2 as a pure consumer of the T4D band or forbids a second threshold set.",
    ),
    7: (
        "full",
        "AC-09/10 cover default-off zero events and the one-second stop bound.",
    ),
    8: (
        "full",
        "AC-11/12/13 cover no-send, E1 origin, default redaction, and one-shot "
        "exception; AC-07/17 cover tone and deterministic fallback.",
    ),
    9: (
        "full",
        "AC-18 covers read-only preview, disk invariance, and unauthorized "
        "rejection; AC-19/25 cover execution/refusal and hostile filenames.",
    ),
    10: (
        "full",
        "AC-20 directly covers zero third-party rows and written_to_disk=false.",
    ),
    11: (
        "full",
        "AC-21/22 cover zero E0 and the disabled cloud toggle; AC-11/17 delimit "
        "the separately allowed user-directed E1 path.",
    ),
    12: (
        "partial",
        "AC-23 applies after matrix actions and AC-24 covers crash recovery; "
        "AC-14/15 name memory/forget audit. AC-23 does not enumerate and assert "
        "all ten PRODUCT_LOCK audit action kinds as separate expected records.",
    ),
    13: (
        "full",
        "AC-01 includes installation smoke and AC-26 is the repository CI gate.",
    ),
}

KEYWORDS = {
    "T4D": re.compile(r"\bT4D\b"),
    "T0": re.compile(r"\bT0\b"),
    "as_of": re.compile(r"\bas_of\b"),
    "DEMOTE_ONE_BAND_DAYS": re.compile(r"\bDEMOTE_ONE_BAND_DAYS\b"),
    "soul-algo-tie": re.compile(r"\bsoul-algo-tie\b"),
    "correct_tie": re.compile(r"\bcorrect_tie\b"),
}
INTAKE_LOCK_LITERAL = re.compile(r"\bintake lock\b", re.IGNORECASE)
INTAKE_LOCK_SEMANTIC = re.compile(
    r"(?:\bintake\b.{0,60}(?:\block\b|锁)|(?:\block\b|锁).{0,60}\bintake\b)",
    re.IGNORECASE,
)


def extract_acceptance_ids(text: str) -> list[str]:
    return list(dict.fromkeys(re.findall(r"\bAC-\d{2}\b", text)))


def extract_goal1_matrix_ids(text: str) -> list[str]:
    ids: list[str] = []
    in_matrix = False
    for line in text.splitlines():
        if line.startswith("### Goal 1 验收矩阵"):
            in_matrix = True
            continue
        if in_matrix and line.startswith("### "):
            break
        match = re.match(r"\|\s*(AC-\d{2})\s*\|", line)
        if match:
            ids.append(match.group(1))
    return ids


def extract_slice_items(text: str) -> list[dict[str, object]]:
    items: list[dict[str, object]] = []
    in_slice = False
    for line in text.splitlines():
        if line.startswith("## v0.1 最小垂直切片"):
            in_slice = True
            continue
        if not in_slice:
            continue
        if in_slice and line.startswith("### "):
            break
        match = re.match(r"^(\d+)\.\s+(.+)$", line)
        if match:
            items.append(
                {
                    "slice_id": f"SLICE-{int(match.group(1)):02d}",
                    "number": int(match.group(1)),
                    "text": match.group(2),
                }
            )
    return items


def candidate_markdown_files(root: Path) -> list[Path]:
    return [root / "README.md", *sorted((root / "docs").rglob("*.md"))]


def find_lines(files: list[Path], pattern: re.Pattern[str], root: Path) -> list[dict[str, object]]:
    matches: list[dict[str, object]] = []
    for path in files:
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if pattern.search(line):
                matches.append(
                    {
                        "file": path.relative_to(root).as_posix(),
                        "line": line_number,
                        "text": line.strip(),
                    }
                )
    return matches


def check_negative_scope(root: Path) -> dict[str, object]:
    product = (root / "docs/PRODUCT_LOCK.md").read_text(encoding="utf-8")
    decisions = (root / "docs/DECISIONS.md").read_text(encoding="utf-8")
    formal = (root / "docs/FORMAL_WORK_PROMPT.md").read_text(encoding="utf-8")
    security = (root / "docs/SECURITY.md").read_text(encoding="utf-8")

    checks = {
        "file_write_execution_not_required_in_v0.1": all(
            (
                "v0.1 **不执行写**" in product,
                "文件整理执行与撤销 | 砍 | v0.1.1" in product,
                "v0.1 不实现文件写" in formal,
                "执行在 v0.1.1" in decisions,
                "v0.1 不消费写文件令牌" in security,
            )
        ),
        "oauth_not_required_in_v0.1": all(
            (
                "OAuth 不在 v0.1" in product,
                "| OAuth | 砍 | v0.2 |" in product,
                "v0.1 无 OAuth" in decisions,
            )
        ),
        "e0_http_not_required_in_v0.1": all(
            (
                "v0.1 无云端 HTTP" in formal,
                "无 E0" in formal,
                "云端适配器 HTTP | 砍 | v0.4" in product,
                "E0 无代码路径" in product,
                "无实现、无域名、无 HTTP client" in decisions,
                "E0 在 v0.1 构建期消除" in security,
            )
        ),
    }
    return {
        "scope": list(CORE_PLAN_FILES),
        "checks": checks,
        "all_passed": all(checks.values()),
        "interpretation": (
            "Mentions of these features are prohibitions, deferred roadmap items, "
            "or denial tests; none is a positive v0.1 implementation requirement."
        ),
    }


def build_output(root: Path) -> dict[str, object]:
    formal_path = root / "docs/FORMAL_WORK_PROMPT.md"
    product_path = root / "docs/PRODUCT_LOCK.md"
    formal = formal_path.read_text(encoding="utf-8")
    product = product_path.read_text(encoding="utf-8")

    all_ids = extract_acceptance_ids(formal)
    matrix_ids = extract_goal1_matrix_ids(formal)
    slices = extract_slice_items(product)
    assert len(slices) == 13, f"expected 13 slice items, found {len(slices)}"
    assert [item["number"] for item in slices] == list(range(1, 14))
    assert matrix_ids == [f"AC-{number:02d}" for number in range(1, 27)]
    assert all_ids == [*matrix_ids, "AC-27"]

    for item in slices:
        number = int(item["number"])
        status, note = SLICE_ASSESSMENT[number]
        item["backed_by"] = SLICE_TO_AC[number]
        item["coverage"] = status
        item["assessment"] = note

    ac_to_slice = {acceptance_id: [] for acceptance_id in all_ids}
    for item in slices:
        for acceptance_id in item["backed_by"]:
            assert acceptance_id in ac_to_slice
            ac_to_slice[acceptance_id].append(item["slice_id"])

    markdown_files = candidate_markdown_files(root)
    legacy_plan_files = sorted(
        path
        for path in (root / ".agent_workspace/context/plan").glob("*")
        if path.is_file()
    )
    core_paths = {root / relative for relative in CORE_PLAN_FILES}
    keyword_scan: dict[str, object] = {}
    for keyword, pattern in KEYWORDS.items():
        matches = find_lines(markdown_files, pattern, root)
        keyword_scan[keyword] = {
            "matches": matches,
            "files": sorted({match["file"] for match in matches}),
            "legacy_context_plan_matches": find_lines(
                legacy_plan_files, pattern, root
            ),
            "core_plan_files": sorted(
                {
                    match["file"]
                    for match in matches
                    if root / str(match["file"]) in core_paths
                }
            ),
        }

    literal_matches = find_lines(markdown_files, INTAKE_LOCK_LITERAL, root)
    semantic_matches = find_lines(markdown_files, INTAKE_LOCK_SEMANTIC, root)
    keyword_scan["intake lock"] = {
        "literal_matches": literal_matches,
        "semantic_matches": semantic_matches,
        "files": sorted({match["file"] for match in semantic_matches}),
        "legacy_context_plan_literal_matches": find_lines(
            legacy_plan_files, INTAKE_LOCK_LITERAL, root
        ),
        "legacy_context_plan_semantic_matches": find_lines(
            legacy_plan_files, INTAKE_LOCK_SEMANTIC, root
        ),
        "core_plan_files": sorted(
            {
                match["file"]
                for match in semantic_matches
                if root / str(match["file"]) in core_paths
            }
        ),
        "match_note": "Semantic scan accepts Chinese 锁 within 60 characters of intake.",
    }

    orphan_matrix_acs = [
        acceptance_id for acceptance_id in matrix_ids if not ac_to_slice[acceptance_id]
    ]
    deferred_outside_slice = [
        acceptance_id
        for acceptance_id in all_ids
        if acceptance_id not in matrix_ids and not ac_to_slice[acceptance_id]
    ]

    return {
        "sources": {
            "acceptance_criteria": formal_path.relative_to(root).as_posix(),
            "slice": product_path.relative_to(root).as_posix(),
        },
        "extracted": {
            "all_ac_ids": all_ids,
            "goal1_matrix_ac_ids": matrix_ids,
            "deferred_ac_ids": [item for item in all_ids if item not in matrix_ids],
            "slice_count": len(slices),
        },
        "slice_items": slices,
        "ac_to_slice": ac_to_slice,
        "orphans": {
            "slice_items_without_any_ac": [
                item["slice_id"] for item in slices if not item["backed_by"]
            ],
            "goal1_matrix_acs_without_slice": orphan_matrix_acs,
            "deferred_acs_outside_13_item_slice": deferred_outside_slice,
            "note": "AC-27 is explicitly v0.1.1 and is not a Goal 1/v0.1 orphan defect.",
        },
        "partial_slice_items": [
            item["slice_id"] for item in slices if item["coverage"] == "partial"
        ],
        "keyword_scan": keyword_scan,
        "negative_scope": check_negative_scope(root),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path("/workspace"))
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).with_name("COVERAGE.json"),
    )
    args = parser.parse_args()
    result = build_output(args.root.resolve())
    args.output.write_text(
        json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(f"wrote {args.output}")
    print(
        f"ACs={len(result['extracted']['all_ac_ids'])} "
        f"Goal1={len(result['extracted']['goal1_matrix_ac_ids'])} "
        f"slices={result['extracted']['slice_count']}"
    )
    print(f"partial={','.join(result['partial_slice_items'])}")
    print(f"orphans={json.dumps(result['orphans'], ensure_ascii=False)}")
    print(
        "negative_scope="
        + json.dumps(result["negative_scope"]["checks"], ensure_ascii=False, sort_keys=True)
    )
    for keyword, finding in result["keyword_scan"].items():
        print(
            f"keyword[{keyword}] files="
            + json.dumps(finding["files"], ensure_ascii=False)
            + " core="
            + json.dumps(finding["core_plan_files"], ensure_ascii=False)
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
