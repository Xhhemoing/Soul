#!/usr/bin/env python3
"""tie_strength 契约探针（只读审计，不改权威 docs）。

对 docs/schemas/relationship.schema.json 的 tie_strength 子模式逐条断言
D58/D59/D32/D48 声称的行为，并把「声称」与「实测」并列打印。

用法：python3 schema_probe.py   （需要 pip install jsonschema）
退出码 0 表示每条探针的实测都与本文件记录的预期一致；非 0 表示模式行为
已经与本报告的记录漂移。
"""

import json
import pathlib
import sys

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

SCHEMA_DIR = pathlib.Path(__file__).resolve().parents[4] / "docs" / "schemas"


def load(name):
    return json.loads((SCHEMA_DIR / name).read_text(encoding="utf-8"))


registry = Registry().with_resources(
    [
        (
            f"https://soul.local/schemas/{name}",
            Resource.from_contents(load(name)),
        )
        for name in ("_defs.schema.json", "relationship.schema.json")
    ]
)

TIE = load("relationship.schema.json")["properties"]["tie_strength"]
VALIDATOR = Draft202012Validator(TIE, registry=registry)

FULL_T4D = {
    "band": "strong",
    "algorithm_id": "T4D",
    "interaction_count": 24,
    "outgoing_count": 13,
    "incoming_count": 11,
    "direct_out_count": 12,
    "direct_in_count": 12,
    "group_out_count": 1,
    "group_in_count": 0,
    "conversation_count": 3,
    "active_day_count": 9,
    "direct_active_day_count": 6,
    "first_contact_utc": "2025-01-02T03:04:05Z",
    "last_contact_utc": "2025-06-02T03:04:05Z",
    "silent_days": 12,
    "as_of_utc": "2025-06-14T03:04:05Z",
}


def without(*keys):
    return {k: v for k, v in FULL_T4D.items() if k not in keys}


# (编号, 探针说明, 实例, 期望是否合法, 该探针对应的决议/条款)
PROBES = [
    ("P01", "空对象仍合法（D59）", {}, True, "D59"),
    ("P02", "整套 T4D 可复核面", FULL_T4D, True, "D58/D59"),
    ("P03", "声明 T4D 但缺 silent_days", without("silent_days"), False, "D59 整包必填"),
    ("P04", "声明 T4D 但缺 as_of_utc", without("as_of_utc"), False, "D59 / as_of 纪律"),
    ("P05", "声明 T4D 但缺 direct_active_day_count",
     without("direct_active_day_count"), False, "D59 整包必填"),
    ("P06", "分列计数不带 algorithm_id（棘轮）",
     {"direct_out_count": 5}, False, "dependentRequired 棘轮"),
    ("P07", "silent_days 不带 algorithm_id（棘轮）",
     {"silent_days": 5}, False, "dependentRequired 棘轮"),
    ("P08", "band=none 不可存（A2 的 Band::None 无落点）",
     {"band": "none"}, False, "缺口 S1"),
    ("P09", "第三套判档规则偷渡", dict(FULL_T4D, algorithm_id="T3R"), False, "DECISION §5"),
    ("P10", "A2 字段名 direct_count 直接落库",
     dict(FULL_T4D, direct_count=12), False, "缺口 S2 字段名不对齐"),
    ("P11", "last_contact_evidence_id 无落点",
     dict(FULL_T4D, last_contact_evidence_id="0192f0c1-0000-7000-8000-000000000001"),
     False, "缺口 S3"),
    ("P12", "any_direct 无落点", dict(FULL_T4D, any_direct=True), False, "缺口 S4"),
    ("P13", "locked_by_user=true 但无 user_band（D32 反向未强制）",
     dict(FULL_T4D, locked_by_user=True), True, "缺口 S5"),
    ("P14", "user_band 无 locked_by_user", dict(FULL_T4D, user_band="strong"),
     False, "D32 正向已强制"),
    ("P15", "user_band=null + locked_by_user=false（未锁）",
     dict(FULL_T4D, user_band=None, locked_by_user=False), True, "D32/D48"),
    ("P16", "machine_band 与 band 并存", dict(FULL_T4D, machine_band="moderate"),
     True, "D32/D48"),
    ("P17", "last_direct_contact_utc=null 允许跳过序列化",
     dict(FULL_T4D, last_direct_contact_utc=None), True, "DECISION §4.3"),
    ("P18", "裸 band 无 algorithm_id（保留 Goal 1 今日宽松度）",
     {"band": "strong"}, True, "D58 遗留债"),
    ("P19", "裸 interaction_count 不触发棘轮（棘轮只覆盖 T4D 新字段）",
     {"interaction_count": 999}, True, "缺口 S6"),
    ("P20", "silent_days 为负", dict(FULL_T4D, silent_days=-1), False, "minimum:0"),
    ("P21", "silent_days 与 as_of−last_contact 矛盾（无跨字段约束）",
     dict(FULL_T4D, silent_days=99999), True, "缺口 S7"),
    ("P22", "群聊计数带 algorithm_id 但 direct 全缺",
     {"algorithm_id": "T4D", "group_out_count": 3}, False, "D59 整包必填"),
    ("P23", "user_band 有值但 locked_by_user=false（D32 直接反例）",
     dict(FULL_T4D, user_band="strong", locked_by_user=False), True, "缺口 S5"),
    ("P24", "locked_by_user=true 但 user_band=null（D32 直接反例）",
     dict(FULL_T4D, user_band=None, locked_by_user=True), True, "缺口 S5"),
]


def main():
    width = max(len(text) for _, text, _, _, _ in PROBES)
    failures = []
    print(f"{'探针':<5} {'说明':<{width}}  期望   实测   依据")
    print("-" * (width + 34))
    for probe_id, text, instance, expected_valid, clause in PROBES:
        errors = sorted(VALIDATOR.iter_errors(instance), key=str)
        actual_valid = not errors
        mark = "一致" if actual_valid == expected_valid else "!! 漂移"
        print(
            f"{probe_id:<5} {text:<{width}}  "
            f"{'合法' if expected_valid else '拒绝':<4} "
            f"{'合法' if actual_valid else '拒绝':<4} {clause}  {mark}"
        )
        if actual_valid != expected_valid:
            failures.append((probe_id, text, [e.message for e in errors]))

    print()
    if failures:
        print(f"漂移 {len(failures)} 条：")
        for probe_id, text, messages in failures:
            print(f"  {probe_id} {text}: {messages}")
        return 1
    print(f"{len(PROBES)}/{len(PROBES)} 条探针的实测与报告记录一致。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
