#!/usr/bin/env python3
"""Round 2 opus-a 探针：relationship.tie_strength 收紧 + schemas.lock.json 一致性。

用法：
    python3 verify_tie_strength.py [--schemas-dir docs/schemas]

两段检查，各自独立退出码贡献：

A. 锁一致性（无第三方依赖，只用 hashlib）：
   - lock 列出的每一份 schema 都存在，且 sha256 与文件字节一致；
   - docs/schemas/ 下除 schemas.lock.json 外没有未被锁住的文件。

B. 校验行为（需要 jsonschema + referencing；缺失则跳过 B 并说明，A 仍然跑）：
   四条硬要求 —— 空 tie_strength 通过、T4D 完整包通过、score 字段被拒、
   T4D 半迁移被拒 —— 加上 D22 / 档位 / as_of 纪律的回归用例。
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[5]
DEFAULT_SCHEMAS_DIR = REPO / "docs" / "schemas"
LOCK_NAME = "schemas.lock.json"
# 与 Goal 1 `crates/xtask/src/schema_freeze.rs` 对齐：冻结集恰好十一份。
EXPECTED_SCHEMA_COUNT = 11

UUID7 = "018f3a2b-7c41-7d9e-8a10-2b3c4d5e6f70"
UUID7_B = "018f3a2b-7c41-7d9e-8a10-2b3c4d5e6f71"
UUID7_C = "018f3a2b-7c41-7d9e-8a10-2b3c4d5e6f72"
UUID7_D = "018f3a2b-7c41-7d9e-8a10-2b3c4d5e6f73"

PASSED: list[str] = []
FAILED: list[str] = []


def check(name: str, ok: bool, detail: str = "") -> None:
    if ok:
        PASSED.append(name)
    else:
        FAILED.append(f"{name}{(' — ' + detail) if detail else ''}")


def sha256_file(path: Path) -> str:
    """与 xtask `digest_file` 同语义：先把 CRLF 折成 LF 再哈希。

    行尾不属于契约，否则 Windows 检出会在没有任何真实改动时把 CI 打红。
    单独的 CR 不动，真改了 CR 仍然换哈希。
    """
    return hashlib.sha256(path.read_bytes().replace(b"\r\n", b"\n")).hexdigest()


# --------------------------------------------------------------------------
# A. 锁一致性
# --------------------------------------------------------------------------
def check_lock(schemas_dir: Path) -> None:
    lock_path = schemas_dir / LOCK_NAME
    if not lock_path.is_file():
        check("lock/present", False, f"{lock_path} 不存在")
        return
    check("lock/present", True)

    lock = json.loads(lock_path.read_text(encoding="utf-8"))
    check("lock/algorithm_is_sha256", lock.get("algorithm") == "sha256", repr(lock.get("algorithm")))
    check("lock/keeps_note", bool(lock.get("note")))
    check(
        "lock/key_order_matches_xtask",
        list(lock) == ["note", "algorithm", "schemas"],
        repr(list(lock)),
    )
    check(
        "lock/schema_names_sorted",
        list(lock.get("schemas", {})) == sorted(lock.get("schemas", {})),
        "xtask 用 BTreeMap 序列化，键必须是字典序",
    )

    pinned = lock.get("schemas", {})
    on_disk = {p.name for p in sorted(schemas_dir.glob("*.json")) if p.name != LOCK_NAME}
    check(
        "lock/frozen_set_is_eleven",
        len(on_disk) == EXPECTED_SCHEMA_COUNT,
        f"盘上 {len(on_disk)} 份，xtask 期望 {EXPECTED_SCHEMA_COUNT} 份",
    )

    missing = sorted(on_disk - set(pinned))
    check("lock/covers_every_file", not missing, f"未被锁住：{missing}")

    stale = sorted(set(pinned) - on_disk)
    check("lock/no_phantom_entries", not stale, f"锁里有但盘上没有：{stale}")

    for name in sorted(set(pinned) & on_disk):
        actual = sha256_file(schemas_dir / name)
        check(f"lock/hash::{name}", actual == pinned[name], f"盘上 {actual[:16]}… 锁里 {pinned[name][:16]}…")


# --------------------------------------------------------------------------
# B. 校验行为
# --------------------------------------------------------------------------
def base_edge(tie_strength: dict | None) -> dict:
    edge = {
        "schema_version": "1.0.0",
        "relationship_id": UUID7,
        "from_contact_id": UUID7_B,
        "to_contact_id": UUID7_C,
        "evidence_ids": [UUID7_D],
        "egress_scope": "local_only",
    }
    if tie_strength is not None:
        edge["tie_strength"] = tie_strength
    return edge


GOAL1_EIGHT = {
    "band": "moderate",
    "interaction_count": 24,
    "outgoing_count": 11,
    "incoming_count": 13,
    "conversation_count": 6,
    "active_day_count": 9,
    "first_contact_utc": "2025-01-04T08:15:00Z",
    "last_contact_utc": "2026-07-30T21:02:00Z",
}

T4D_FULL = {
    **GOAL1_EIGHT,
    "algorithm_id": "T4D",
    "direct_out_count": 7,
    "direct_in_count": 8,
    "group_out_count": 4,
    "group_in_count": 5,
    "direct_active_day_count": 6,
    "last_direct_contact_utc": "2026-07-28T10:00:00Z",
    "silent_days": 26,
    "as_of_utc": "2026-08-25T00:00:00Z",
}

T4D_GROUP_ONLY = {
    **T4D_FULL,
    "band": "weak",
    "direct_out_count": 0,
    "direct_in_count": 0,
    "direct_active_day_count": 0,
    "last_direct_contact_utc": None,
}


def without(d: dict, *keys: str) -> dict:
    return {k: v for k, v in d.items() if k not in keys}


def structural_checks(schema: dict) -> None:
    """无 jsonschema 时也必须成立的结构断言。"""
    props = schema["properties"]
    ts = props["tie_strength"]

    check(
        "struct/top_required_unchanged",
        schema["required"]
        == [
            "schema_version",
            "relationship_id",
            "from_contact_id",
            "to_contact_id",
            "evidence_ids",
        ],
        repr(schema["required"]),
    )
    check("struct/top_additionalProperties_false", schema.get("additionalProperties") is False)
    check("struct/tie_strength_not_top_required", "tie_strength" not in schema["required"])
    check("struct/tie_strength_additionalProperties_false", ts.get("additionalProperties") is False)
    check(
        "struct/tie_strength_no_unconditional_required",
        "required" not in ts,
        "本层设了无条件 required，空对象会被拒",
    )
    check("struct/tie_strength_has_dependentRequired", isinstance(ts.get("dependentRequired"), dict))
    check("struct/tie_strength_has_if_then", "if" in ts and "then" in ts)

    ts_props = ts.get("properties", {})
    check("struct/tie_strength_is_typed", bool(ts_props), "tie_strength 仍是裸 object，没有 properties")

    band_all_of = ts_props.get("band", {}).get("allOf", [])
    band_enum = next((c["enum"] for c in band_all_of if "enum" in c), None)
    check("struct/band_enum_excludes_none", band_enum == ["weak", "moderate", "strong"], repr(band_enum))
    check(
        "struct/band_refs_defs",
        any("$ref" in c and c["$ref"].endswith("evidenceBand") for c in band_all_of),
    )
    check(
        "struct/algorithm_id_enum_two_values",
        ts_props.get("algorithm_id", {}).get("enum") == ["T4D", "T4"],
    )

    banned = {"score", "percentile", "tie_weight", "weight", "rank"}
    check(
        "struct/no_score_like_property",
        bool(ts_props) and not (banned & set(ts_props)),
        f"裸 object 挡不住 {sorted(banned)}" if not ts_props else f"出现了被 D22 禁止的字段：{sorted(banned & set(ts_props))}",
    )

    then_required = set(ts.get("then", {}).get("required", []))
    check(
        "struct/then_requires_split_counts",
        {"direct_out_count", "direct_in_count", "group_out_count", "group_in_count"} <= then_required,
    )
    check("struct/then_requires_as_of", "as_of_utc" in then_required)
    check("struct/then_requires_base_eight", set(GOAL1_EIGHT) <= then_required)
    check(
        "struct/then_omits_nullable_last_direct",
        "last_direct_contact_utc" not in then_required,
        "可为 null 的字段被设成必填，会与跳过序列化的实现打架",
    )


def behaviour_checks(schemas_dir: Path, schema: dict) -> bool:
    try:
        from jsonschema import Draft202012Validator
        from referencing import Registry, Resource
    except ImportError as exc:  # pragma: no cover
        print(f"[skip] B 段跳过：{exc}（A 段与结构断言仍然执行）")
        return False

    registry = Registry()
    for path in sorted(schemas_dir.glob("*.json")):
        if path.name == LOCK_NAME:
            continue
        doc = json.loads(path.read_text(encoding="utf-8"))
        if "$id" in doc:
            registry = registry.with_resource(doc["$id"], Resource.from_contents(doc))

    validator = Draft202012Validator(schema, registry=registry)

    def accepts(name: str, instance: dict) -> None:
        errors = sorted(validator.iter_errors(instance), key=str)
        check(f"accept/{name}", not errors, errors[0].message if errors else "")

    def rejects(name: str, instance: dict) -> None:
        errors = list(validator.iter_errors(instance))
        check(f"reject/{name}", bool(errors), "本应被拒却通过了")

    # 四条硬要求
    accepts("empty_tie_strength", base_edge({}))
    accepts("t4d_complete", base_edge(T4D_FULL))
    rejects("score_field", base_edge({**GOAL1_EIGHT, "score": 0.87}))
    rejects("partial_t4d_extras", base_edge({**GOAL1_EIGHT, "direct_out_count": 7, "direct_in_count": 8}))

    # 兼容面
    accepts("no_tie_strength_at_all", base_edge(None))
    accepts("goal1_today_eight_fields", base_edge(GOAL1_EIGHT))
    accepts("t4d_group_only_null_last_direct", base_edge(T4D_GROUP_ONLY))
    accepts("t4d_without_optional_last_direct", base_edge(without(T4D_FULL, "last_direct_contact_utc")))

    # 半迁移的其它形态
    rejects("algorithm_id_alone", base_edge({"algorithm_id": "T4D"}))
    rejects("t4d_missing_as_of", base_edge(without(T4D_FULL, "as_of_utc")))
    rejects("t4d_missing_direct_active_days", base_edge(without(T4D_FULL, "direct_active_day_count")))
    rejects("t4d_missing_silent_days", base_edge(without(T4D_FULL, "silent_days")))
    rejects("t4d_missing_band", base_edge(without(T4D_FULL, "band")))
    rejects("silent_days_without_algorithm_id", base_edge({**GOAL1_EIGHT, "silent_days": 26}))
    rejects("as_of_without_algorithm_id", base_edge({**GOAL1_EIGHT, "as_of_utc": "2026-08-25T00:00:00Z"}))

    # D22 与档位纪律
    rejects("tie_weight_field", base_edge({**GOAL1_EIGHT, "tie_weight": 3}))
    rejects("percentile_field", base_edge({**GOAL1_EIGHT, "percentile": 91}))
    rejects("band_none", base_edge({**GOAL1_EIGHT, "band": "none"}))
    rejects("negative_count", base_edge({**GOAL1_EIGHT, "interaction_count": -1}))
    rejects("third_algorithm_id", base_edge({**T4D_FULL, "algorithm_id": "T5"}))
    rejects("as_of_as_unix_integer", base_edge({**T4D_FULL, "as_of_utc": 1787616000}))
    rejects("tie_strength_not_object", base_edge(None) | {"tie_strength": "strong"})

    return True


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--schemas-dir", default=str(DEFAULT_SCHEMAS_DIR))
    args = parser.parse_args()

    schemas_dir = Path(args.schemas_dir).resolve()
    print(f"schemas dir : {schemas_dir}")

    rel = schemas_dir / "relationship.schema.json"
    if not rel.is_file():
        print(f"FATAL: {rel} 不存在")
        return 2
    print(f"relationship: {sha256_file(rel)}")

    schema = json.loads(rel.read_text(encoding="utf-8"))

    check_lock(schemas_dir)
    structural_checks(schema)
    ran_b = behaviour_checks(schemas_dir, schema)

    print()
    for line in FAILED:
        print(f"  FAIL {line}")
    total = len(PASSED) + len(FAILED)
    print(f"{len(PASSED)}/{total} passed" + ("" if ran_b else "（B 段未跑：缺 jsonschema/referencing）"))
    return 1 if FAILED else 0


if __name__ == "__main__":
    sys.exit(main())
