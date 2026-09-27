#!/usr/bin/env python3
"""Round 2 / opus-b 核验探针：schema 保真与 D32 棘轮（只读，不改任何权威文件）。

与 R1 opus-b 的 `schema_probe.py` **独立重写**，三处方法学差异是刻意的：

1. R1 抽出 `properties.tie_strength` 子模式单独校验；本探针校验**整份
   relationship 文档**，这样顶层 `additionalProperties:false` 与 `required`
   一起参与，能发现「字段在 tie_strength 里没落点，但顶层有没有」这类问题。
2. R1 未挂 `FormatChecker`。Draft 2020-12 里 `format` 默认只是注解不是断言，
   所以「时间戳」字段实际上不被校验。本探针对同一实例跑两遍（挂/不挂），把
   差值本身作为一条发现。
3. D32 只测三条反例；本探针枚举 (locked_by_user × user_band) 的**完整 3×3
   真值表**，并另测「生效档与锁定档矛盾」这一 R1 未覆盖的方向。

用法：python3 verify_r2_opus_b.py
退出码 0 = 每条探针实测与本文件登记的预期一致（预期含「已知缺口」）。
"""

import hashlib
import json
import pathlib
import sys

from jsonschema import Draft202012Validator, FormatChecker
from referencing import Registry, Resource

SCHEMA_DIR = pathlib.Path(__file__).resolve().parents[5] / "docs" / "schemas"
LOCK_NAME = "schemas.lock.json"

results = []  # (group, id, desc, expected, actual, note)


def record(group, pid, desc, expected, actual, note=""):
    results.append((group, pid, desc, expected, actual, note))


# --------------------------------------------------------------------------
# A. 锁完整性：独立重算，并双向检查覆盖面
# --------------------------------------------------------------------------
lock = json.loads((SCHEMA_DIR / LOCK_NAME).read_text(encoding="utf-8"))
locked = lock["schemas"]
on_disk = sorted(p.name for p in SCHEMA_DIR.glob("*.json") if p.name != LOCK_NAME)

record("A", "A00", "lock 的 algorithm 字段为 sha256", "sha256", lock.get("algorithm"))

for name in on_disk:
    digest = hashlib.sha256((SCHEMA_DIR / name).read_bytes()).hexdigest()
    record("A", f"A:{name}", "字节 sha256 与 lock 一致",
           locked.get(name, "<未被锁>"), digest)

record("A", "A98", "磁盘上每份 schema 都在 lock 里",
       set(), set(on_disk) - set(locked))
record("A", "A99", "lock 里每条都在磁盘上",
       set(), set(locked) - set(on_disk))

# --------------------------------------------------------------------------
# B. 校验器装配：整份 relationship 文档
# --------------------------------------------------------------------------
def load(name):
    return json.loads((SCHEMA_DIR / name).read_text(encoding="utf-8"))


registry = Registry().with_resources(
    [
        (f"https://soul.local/schemas/{name}", Resource.from_contents(load(name)))
        for name in on_disk
    ]
)

REL = load("relationship.schema.json")
V_DOC = Draft202012Validator(REL, registry=registry)
V_DOC_FMT = Draft202012Validator(REL, registry=registry, format_checker=FormatChecker())
V_TIE = Draft202012Validator(REL["properties"]["tie_strength"], registry=registry)

U = "0192f0c1-0000-7000-8000-00000000000"

BASE_REL = {
    "schema_version": "1.0.0",
    "relationship_id": f"{U}1",
    "from_contact_id": f"{U}2",
    "to_contact_id": f"{U}3",
    "evidence_ids": [f"{U}4"],
}

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


def doc(tie=None, **top):
    d = dict(BASE_REL, **top)
    if tie is not None:
        d["tie_strength"] = tie
    return d


def ok(validator, instance):
    return not list(validator.iter_errors(instance))


def tie_ok(tie):
    """同一个 tie_strength，子模式与整份文档必须给出同一判定。"""
    sub = ok(V_TIE, tie)
    whole = ok(V_DOC, doc(tie))
    assert sub == whole, f"子模式与整份文档判定分裂: {tie}"
    return whole


def with_t4d(**kw):
    return dict(FULL_T4D, **kw)


def without(*keys):
    return {k: v for k, v in FULL_T4D.items() if k not in keys}


# --------------------------------------------------------------------------
# C. 复核 R1 的 D59 棘轮结论（独立重写，判定应完全一致）
# --------------------------------------------------------------------------
C = [
    ("C01", "空 tie_strength 对象仍合法", {}, True),
    ("C02", "整套 T4D 可复核面", FULL_T4D, True),
    ("C03", "声明 T4D 缺 silent_days", without("silent_days"), False),
    ("C04", "声明 T4D 缺 as_of_utc", without("as_of_utc"), False),
    ("C05", "声明 T4D 缺 direct_active_day_count",
     without("direct_active_day_count"), False),
    ("C06", "分列计数不带 algorithm_id（棘轮）", {"direct_out_count": 5}, False),
    ("C07", "silent_days 不带 algorithm_id（棘轮）", {"silent_days": 5}, False),
    ("C08", "band=none 无落点", {"band": "none"}, False),
    ("C09", "algorithm_id=T3R 偷渡第三套规则", with_t4d(algorithm_id="T3R"), False),
    ("C10", "algorithm_id=T4 合法（DECISION §5 档内回退）",
     with_t4d(algorithm_id="T4"), True),
    ("C11", "裸 band 不触发棘轮", {"band": "strong"}, True),
    ("C12", "裸 interaction_count 不触发棘轮", {"interaction_count": 999}, True),
    ("C13", "silent_days 为负被拒", with_t4d(silent_days=-1), False),
    ("C14", "silent_days 与 as_of−last_contact 矛盾（无跨字段约束）",
     with_t4d(silent_days=99999), True),
    ("C15", "群聊计数带 algorithm_id 但 direct 全缺",
     {"algorithm_id": "T4D", "group_out_count": 3}, False),
    ("C16", "last_direct_contact_utc=null 可跳过序列化",
     with_t4d(last_direct_contact_utc=None), True),
]
for pid, desc, tie, expected in C:
    record("C", pid, desc, expected, tie_ok(tie))

# --------------------------------------------------------------------------
# D. D32 双向棘轮：(locked_by_user × user_band) 完整 3×3 真值表
#    D32 原文：locked ⟺ user_band.is_some()
# --------------------------------------------------------------------------
LOCK_STATES = [("缺席", {}), ("false", {"locked_by_user": False}),
               ("true", {"locked_by_user": True})]
BAND_STATES = [("缺席", {}), ("null", {"user_band": None}),
               ("\"strong\"", {"user_band": "strong"})]


def d32_consistent(lock_state, band_state):
    """D32 语义：locked ⟺ user_band 有值。键缺席一律读作「未锁 / 无锁定档」。"""
    is_locked = lock_state == "true"
    has_band = band_state == "\"strong\""
    return is_locked == has_band


n = 0
for lname, lpatch in LOCK_STATES:
    for bname, bpatch in BAND_STATES:
        n += 1
        tie = with_t4d(**lpatch, **bpatch)
        want_by_d32 = d32_consistent(lname, bname)
        actual = tie_ok(tie)
        record("D", f"D{n:02d}",
               f"locked_by_user={lname:<6} user_band={bname:<8} "
               f"→ D32 {'允许' if want_by_d32 else '禁止'}",
               want_by_d32, actual,
               "" if want_by_d32 == actual else
               ("schema 放行了 D32 禁止的组合" if actual else "schema 拒绝了 D32 允许的组合"))

# D32 / D48 的另一半：生效档必须等于锁定档。R1 未测这个方向。
record("E", "E01", "锁定边 band 与 user_band 矛盾（D48 生效档=锁定档）",
       False, tie_ok(with_t4d(band="weak", user_band="strong", locked_by_user=True)),
       "D48：生效档 = 用户锁定时的档")
record("E", "E02", "未锁边 band 与 machine_band 矛盾（生效档=机器档）",
       False, tie_ok(with_t4d(band="weak", machine_band="strong",
                              locked_by_user=False)),
       "D32：未锁边生效档应为机器档")
record("E", "E03", "重建边完全省略 machine_band（D32「未锁边也写」）",
       False, tie_ok(with_t4d(locked_by_user=False)),
       "D32：未锁边也写 machine_band")
record("E", "E04", "machine_band=none 被拒（与 band 同样排除 none）",
       False, tie_ok(with_t4d(machine_band="none")))
record("E", "E05", "user_band=none 被拒", False,
       tie_ok(with_t4d(user_band="none", locked_by_user=True)))

# --------------------------------------------------------------------------
# F. TieScore 保真：a2.rs 的每个字段能不能落进 tie_strength
# --------------------------------------------------------------------------
A2_FIELDS = {
    "band": "strong",
    "interaction_count": 24,
    "outgoing": 13,
    "incoming": 11,
    "active_day_count": 9,
    "conversation_count": 3,
    "any_direct": True,
    "direct_count": 24,
    "group_count": 1,
    "last_contact_unix": 1_748_000_000,
    "as_of_unix": 1_749_000_000,
    "evidence_ids": [f"{U}4"],
    "last_contact_evidence_id": f"{U}4",
}
for i, (field, value) in enumerate(A2_FIELDS.items(), start=1):
    accepted = tie_ok(with_t4d(**{field: value}))
    record("F", f"F{i:02d}", f"a2::TieScore.{field} 直接写入 tie_strength",
           None, accepted)

# 顶层也没有落点吗？
record("F", "F90", "last_contact_evidence_id 写在 relationship 顶层",
       False, ok(V_DOC, doc(FULL_T4D, last_contact_evidence_id=f"{U}4")))
record("F", "F91", "顶层 evidence_ids 存在且必填",
       True, "evidence_ids" in REL["required"])
record("F", "F92", "tie_strength 内没有任何证据字段",
       [], [k for k in REL["properties"]["tie_strength"]["properties"]
            if "evidence" in k])

# --------------------------------------------------------------------------
# G. format 断言：Draft 2020-12 默认 format 只是注解
# --------------------------------------------------------------------------
BAD_TS = with_t4d(as_of_utc="香蕉", first_contact_utc="2025-13-45",
                  last_contact_utc="")
record("G", "G00", "本机 FormatChecker 认识 date-time", True,
       "date-time" in FormatChecker().checkers,
       "jsonschema 需额外装 rfc3339-validator，否则 date-time 静默跳过")
record("G", "G01", "非法时间戳，未挂 FormatChecker（默认）", True,
       ok(V_DOC, doc(BAD_TS)), "format 在 2020-12 默认是注解不是断言")
record("G", "G02", "同一实例，挂上 FormatChecker", False,
       ok(V_DOC_FMT, doc(BAD_TS)))
record("G", "G03", "非法 uuid7 被 pattern 拦下（pattern 一直是断言）", False,
       ok(V_DOC, dict(BASE_REL, relationship_id="not-a-uuid")))

# --------------------------------------------------------------------------
# H. 顶层保真
# --------------------------------------------------------------------------
record("H", "H01", "省略 egress_scope 仍合法（const 但不在 required）",
       True, ok(V_DOC, doc(FULL_T4D)))
record("H", "H02", "egress_scope 不在顶层 required",
       True, "egress_scope" not in REL["required"])
record("H", "H03", "tie_strength 整体缺席合法", True, ok(V_DOC, doc()))
record("H", "H04", "evidence_ids 为空数组被拒（minItems:1）", False,
       ok(V_DOC, dict(BASE_REL, evidence_ids=[])))
record("H", "H05", "types 仍是裸 array（任何元素都进得去）", True,
       ok(V_DOC, doc(FULL_T4D, types=[{"任意": "对象"}, 42, None])))

# --------------------------------------------------------------------------
# 输出
# --------------------------------------------------------------------------
def fmt(v):
    if isinstance(v, bool):
        return "合法" if v else "拒绝"
    if isinstance(v, set):
        return "{}" if not v else str(sorted(v))
    if v is None:
        return "—"
    return str(v)


width = max(len(d) for _, _, d, _, _, _ in results)
drift = 0
current = None
for group, pid, desc, expected, actual, note in results:
    if group != current:
        current = group
        print(f"\n=== 组 {group} ===")
    if expected is None:
        mark = ""
    elif expected == actual:
        mark = "一致"
    else:
        mark = "!! 不符"
        drift += 1
    print(f"{pid:<14} {desc:<{width}}  预期 {fmt(expected):<10} "
          f"实测 {fmt(actual):<10} {mark} {note}")

print()
graded = [r for r in results if r[3] is not None]
print(f"{len(graded) - drift}/{len(graded)} 条判定与本文件登记的预期一致"
      f"（{len(results) - len(graded)} 条为无预期的事实记录）。")
sys.exit(1 if drift else 0)
