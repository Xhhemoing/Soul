#!/usr/bin/env python3
"""Round 3 fable-b 独立探针：核验 Round 2 落地（c3d5960）。

只读 docs/ 与 crates/；结果写 stdout（由调用方重定向到本目录 RESULTS.txt）。
与 round2/opus-a 的探针相互独立：不复用其代码，且本探针跑在最终落地字节上
（round2/opus-a/RESULTS.txt 记录的 relationship 哈希 350a7ffc… 是中间稿，
最终落地是 ebf049a9…，见 Section A）。
"""

import hashlib
import json
import re
import sys
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

# argv[1]：可指向 c3d5960 的干净 worktree，以排除同树并发未提交改动的影响。
ROOT = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("/workspace")
print(f"# 审计树：{ROOT}")
SCHEMAS = ROOT / "docs" / "schemas"

passed = 0
failed = 0


def check(name: str, ok: bool, detail: str = "") -> None:
    global passed, failed
    if ok:
        passed += 1
        print(f"  PASS {name}")
    else:
        failed += 1
        print(f"  FAIL {name}" + (f" — {detail}" if detail else ""))


# ---------------------------------------------------------------- Section A
print("# A. schemas.lock.json 与在盘字节一致（独立重算）")
lock = json.loads((SCHEMAS / "schemas.lock.json").read_text())
listed = lock["schemas"]
on_disk = {p.name for p in SCHEMAS.glob("*.json")} - {"schemas.lock.json"}
check("lock/covers_every_schema_file", set(listed) == on_disk,
      f"listed={sorted(listed)} disk={sorted(on_disk)}")
for name, want in sorted(listed.items()):
    got = hashlib.sha256((SCHEMAS / name).read_bytes()).hexdigest()
    check(f"lock/{name}", got == want, f"lock={want[:12]} actual={got[:12]}")
rel_hash = hashlib.sha256((SCHEMAS / "relationship.schema.json").read_bytes()).hexdigest()
print(f"  info relationship sha256 = {rel_hash}")
print(f"  info round2/opus-a RESULTS.txt 声称的落地哈希 350a7ffc… "
      f"{'匹配' if rel_hash.startswith('350a7ffc') else '不匹配（其探针跑在中间稿上）'}")

# ---------------------------------------------------------------- Section B
print("# B. tie_strength 结构断言（最终字节）")
rel = json.loads((SCHEMAS / "relationship.schema.json").read_text())
ts = rel["properties"]["tie_strength"]
props = ts.get("properties", {})
check("struct/typed_object", ts.get("type") == "object" and bool(props))
check("struct/additionalProperties_false", ts.get("additionalProperties") is False)
for f in ["machine_band", "user_band", "locked_by_user"]:
    check(f"struct/lock_field_{f}_present", f in props, "D32/D48 三字段")
then_req = set(ts.get("then", {}).get("required", []))
for f in ["machine_band", "user_band", "locked_by_user"]:
    check(f"struct/lock_field_{f}_not_conditionally_required", f not in then_req,
          "锁定是可选状态，不得进 if/then required")
dep = ts.get("dependentRequired", {})
check("struct/dependentRequired_user_band_pins_locked_by_user",
      dep.get("user_band") == ["locked_by_user"])
check("struct/algorithm_id_enum_exactly_T4D_T4",
      props.get("algorithm_id", {}).get("enum") == ["T4D", "T4"])
check("struct/then_required_15_fields", len(then_req) == 15, f"got {sorted(then_req)}")
check("struct/last_direct_contact_not_required",
      "last_direct_contact_utc" not in then_req, "可为 None、允许跳过序列化")
check("struct/no_score_like_property",
      not ({"score", "weight", "tie_weight", "percentile", "rank"} & set(props)))

# ---------------------------------------------------------------- Section C
print("# C. 实例探针（Draft 2020-12，本地 registry，不出网）")
resources = []
for p in SCHEMAS.glob("*.schema.json"):
    doc = json.loads(p.read_text())
    resources.append((doc["$id"], Resource.from_contents(doc)))
registry = Registry().with_resources(resources)
validator = Draft202012Validator(rel, registry=registry)

UUID = "01890000-0000-7000-8000-000000000000"
ENVELOPE = {
    "schema_version": "1.0.0",
    "relationship_id": UUID,
    "from_contact_id": UUID,
    "to_contact_id": UUID,
    "evidence_ids": [UUID],
    "egress_scope": "local_only",
}

def edge(tie):
    d = dict(ENVELOPE)
    if tie is not None:
        d["tie_strength"] = tie
    return d

def probe(name, instance, expect_valid, why=""):
    errs = list(validator.iter_errors(instance))
    ok = (not errs) == expect_valid
    detail = why if ok else (errs[0].message[:120] if errs else "本应被拒却通过了")
    check(("accept/" if expect_valid else "reject/") + name, ok, detail)

# unblock 线 build.rs::tie_strength_of 重建边（未锁）——P0-1 的核心场景
rebuilt = {
    "band": "weak", "interaction_count": 2, "outgoing_count": 1, "incoming_count": 1,
    "conversation_count": 1, "active_day_count": 2,
    "first_contact_utc": "2026-08-01T09:00:00Z", "last_contact_utc": "2026-08-02T09:00:00Z",
    "direct_out_count": 1, "direct_in_count": 1, "group_out_count": 0, "group_in_count": 0,
    "direct_active_day_count": 2, "last_direct_contact_utc": "2026-08-02T09:00:00Z",
    "silent_days": 3, "as_of_utc": "2026-08-05T09:00:00Z",
    "algorithm_id": "T4D", "machine_band": "weak",
}
probe("rebuilt_unlocked_edge_with_machine_band", edge(rebuilt), True,
      "D32：重建边 machine_band 恒 Some")

# 锁定重建边（correct_tie 之后再 rebuild：band=user_band，三字段齐）
locked = dict(rebuilt, band="strong", user_band="strong", locked_by_user=True)
probe("rebuilt_locked_edge_full_lock_fields", edge(locked), True, "D48 生效档=用户档")

# release_tie 之后（machine/user/locked 全省略）
probe("released_edge_no_lock_fields", edge(rebuilt), True)

# T4 回退合法
probe("algorithm_id_T4_fallback", edge(dict(rebuilt, algorithm_id="T4")), True,
      "DECISION.md 第 5 节回退")

# 空对象与缺席（Goal 1 今天的宽松度）
probe("empty_tie_strength_object", edge({}), True, "D59：空对象仍合法")
probe("tie_strength_absent", edge(None), True)

# 主线 df5d2dd 八字段边（无 algorithm_id、无分列）
legacy8 = {k: rebuilt[k] for k in [
    "band", "interaction_count", "outgoing_count", "incoming_count",
    "conversation_count", "active_day_count", "first_contact_utc", "last_contact_utc"]}
probe("mainline_eight_field_edge", edge(legacy8), True,
      "df5d2dd 主线 TieStrength 只有八字段；t4d_band.rs legacy 夹具同形")

# 负向：score/权重类偷渡
probe("score_field", edge({"score": 0.9}), False)
probe("score_smuggled_into_valid_package", edge(dict(rebuilt, score=0.9)), False)
probe("tie_weight_field", edge({"tie_weight": 3}), False)
probe("percentile_field", edge({"percentile": 88}), False)

# 负向：半迁移（声明算法但可复核面不齐）
probe("half_migration_band_only", edge({"algorithm_id": "T4D", "band": "weak"}), False)
missing_silent = {k: v for k, v in rebuilt.items() if k != "silent_days"}
probe("t4d_missing_silent_days", edge(missing_silent), False)
missing_asof = {k: v for k, v in rebuilt.items() if k != "as_of_utc"}
probe("t4d_missing_as_of", edge(missing_asof), False)

# 负向：分列计数不带 algorithm_id（dependentRequired）
probe("split_count_without_algorithm_id",
      edge(dict(legacy8, direct_out_count=1)), False)

# 负向：user_band 不带 locked_by_user（D32 锁定语义）
probe("user_band_without_locked_by_user",
      edge(dict(rebuilt, user_band="strong")), False)

# 负向：band/machine_band 取 evidenceBand 里被排除的 "none"
probe("band_none", edge(dict(rebuilt, band="none")), False)
probe("machine_band_none", edge(dict(rebuilt, machine_band="none")), False)

# 负向：第三套算法字符串 / 空串 algorithm_id
probe("third_algorithm_string", edge(dict(rebuilt, algorithm_id="T5")), False)
probe("empty_algorithm_id", edge(dict(rebuilt, algorithm_id="")), False,
      "legacy 行 roundtrip 形态——见 REGRESSIONS.md R-1 潜在红点")

# 负向：负计数 / 负 silent_days
probe("negative_direct_out_count", edge(dict(rebuilt, direct_out_count=-1)), False)
probe("negative_silent_days", edge(dict(rebuilt, silent_days=-2)), False)

# 信封层
probe("envelope_minimal", edge(None), True)
bad_env = dict(ENVELOPE); bad_env["extra_field"] = 1
probe("envelope_unknown_property", bad_env, False)
bad_scope = dict(ENVELOPE); bad_scope["egress_scope"] = "cloud"
probe("envelope_egress_scope_not_local_only", bad_scope, False)
bad_ev = dict(ENVELOPE); bad_ev["evidence_ids"] = []
probe("envelope_empty_evidence_ids", bad_ev, False)

# ---------------------------------------------------------------- Section D
print("# D. 文档面探针")
docs = ROOT / "docs"
formal = (docs / "FORMAL_WORK_PROMPT.md").read_text()
decisions = (docs / "DECISIONS.md").read_text()
status = (docs / "STATUS.md").read_text()
security = (docs / "SECURITY.md").read_text()
index = (docs / "PLAN_INDEX.md").read_text()
verify = (docs / "PLAN_VERIFY_PROMPT.md").read_text()
readme = (ROOT / "README.md").read_text()

for n in range(28, 35):
    check(f"formal/AC-{n}_row_exists",
          re.search(rf"^\| AC-{n} \|", formal, re.M) is not None)
check("formal/AC-27_has_no_matrix_row",
      re.search(r"^\| AC-27 \|", formal, re.M) is None, "v0.1.1 不占表行")
check("formal/AC-28_uses_full_count_names",
      "`direct_out_count` / `direct_in_count` / `group_out_count` / `group_in_count`" in formal)
check("formal/historical_section_marked", "### 历史段" in formal
      and "不要照着再跑一遍" in formal and "（历史段到此为止。以下各节仍然有效。）" in formal)
check("formal/first_action_is_sole_path", "唯一可执行的开工路径" in formal)
check("formal/redline11_self_check", "关于红线 11 的自查" in formal)
consts_txt = (ROOT / "crates/soul-algo-tie/src/constants.rs").read_text()
for c in ["MODERATE_MIN_INTERACTIONS", "STRONG_MIN_INTERACTIONS",
          "STRONG_MIN_ACTIVE_DAYS", "DEMOTE_ONE_BAND_DAYS", "FORCE_WEAK_DAYS"]:
    check(f"formal/constant_name_{c}_cited_and_exists",
          c in formal and c in consts_txt)

core = {"PRODUCT_LOCK.md": (docs / "PRODUCT_LOCK.md").read_text(),
        "DECISIONS.md": decisions, "FORMAL_WORK_PROMPT.md": formal,
        "PLAN_INDEX.md": index, "SECURITY.md": security, "STATUS.md": status,
        "PLAN_VERIFY_PROMPT.md": verify, "README.md": readme}
for fname, text in core.items():
    check(f"leak/no_180_or_360_in_{fname}",
          re.search(r"\b(180|360)\b", text) is None)

check("decisions/D59_exists", re.search(r"^\| D59 \|", decisions, re.M) is not None)
d52 = re.search(r"^\| D52 \|.*$", decisions, re.M).group(0)
check("decisions/D52_constant_name_no_literal",
      "DEMOTE_ONE_BAND_DAYS" in d52 and "180" not in d52)
d40 = re.search(r"^\| D40 \|.*$", decisions, re.M).group(0)
check("decisions/D40_keeps_filed_band_loss", "filed_band" in d40)
check("decisions/header_D41_open_range", "**D41 起**" in decisions
      and "D41–D56" not in decisions)
check("decisions/D57_same_tree_consistency",
      "同一棵树内对同一分支的「核于」提交号必须一致" in decisions)
check("decisions/footnote_pr6_d32_checklist",
      "PR #6 整份合入时" in decisions and "届时下一个空闲 ID" in decisions)

tips_status = set(re.findall(r"`cursor/soul-goal1-7b1c`（PR #2） \| `([0-9a-f]{7})`", status))
tips_sec = set(re.findall(r"尖端 `([0-9a-f]{7})`", security))
check("status_security/same_goal1_tip",
      tips_status == tips_sec == {"df5d2dd"},
      f"status={tips_status} security={tips_sec}（git ls-remote 实测尖端 df5d2dd6…）")
schema_row = re.search(r"^\| schema 倒退 \|.*$", status, re.M)
check("status/schema_row_closed", schema_row is not None
      and "已关闭" in schema_row.group(0) and "machine_band" in schema_row.group(0))

check("index/D1_D59", "D1–D59" in index)
check("index/ten_schema_bodies", "十份正文" in index)
check("index/tighten_D58_D59", "D58/D59" in index)
check("index/plan_verify_archived", "一次性提示词，已执行完毕" in index)
check("index/historical_section_listed", "「历史段」" in index)
check("verify/banner_and_tail", verify.splitlines()[2].startswith("> **历史存档")
      and "已经执行完毕" in verify)
check("readme/plan_verify_marked_archived", "历史存档**，勿当新工单" in readme)

snap = ROOT / ".agent_workspace/context/plan/README.md"
check("l4/snapshot_readme_banner", snap.exists()
      and "历史快照，不是权威" in snap.read_text())
check("d27/no_second_product_md",
      not [p for p in ROOT.rglob("PRODUCT.md")
           if ".agent_workspace" not in str(p) and "target" not in str(p)])

print(f"\n{passed}/{passed + failed} passed")
sys.exit(1 if failed else 0)
