#!/usr/bin/env python3
"""Probe for the relationship.schema.json candidate.

Answers three questions the plan cannot answer by prose:

1. Is the candidate a well-formed 2020-12 schema that resolves `_defs`?
2. Does today's Goal 1 output (`TieStrength` with eight fields, no T4D split)
   still validate?  If not, the candidate is a silent breaking change.
3. Does the candidate actually reject what the plan says it rejects
   (half-migrated T4D, invented scores, `band: none`)?

It also prints the sha256 of both the candidate and the schema Goal 1 pinned in
`schemas.lock.json`, so the merge obligation can quote a real delta.

Run:  python3 verify_relationship_schema.py
Needs: jsonschema >= 4.18 (2020-12 + `referencing`).  No network.
"""

from __future__ import annotations

import hashlib
import json
import pathlib
import subprocess
import sys

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

HERE = pathlib.Path(__file__).resolve().parent
CANDIDATE = HERE.parent / "relationship.schema.json"
REPO = HERE.parents[4]
DEFS = REPO / "docs" / "schemas" / "_defs.schema.json"
GOAL1_REF = "origin/cursor/soul-goal1-7b1c"
LOCKED_RELATIONSHIP_SHA = "df34747e28ef2e01ff02f094b26a2b7722c096256563474835f2c61f1ec122ff"

TS = "2026-08-24T14:00:00Z"

BASE = {
    "schema_version": "1.0.0",
    "relationship_id": "019210f4-3c8e-7a10-8b2c-2f0d5a7e1c01",
    "from_contact_id": "019210f4-3c8e-7a10-8b2c-2f0d5a7e1c02",
    "to_contact_id": "019210f4-3c8e-7a10-8b2c-2f0d5a7e1c03",
    "evidence_ids": ["019210f4-3c8e-7a10-8b2c-2f0d5a7e1c04"],
    "egress_scope": "local_only",
}

GOAL1_TODAY = {
    "band": "strong",
    "interaction_count": 12,
    "outgoing_count": 6,
    "incoming_count": 6,
    "conversation_count": 1,
    "active_day_count": 6,
    "first_contact_utc": "2026-08-12T09:00:00Z",
    "last_contact_utc": "2026-08-21T09:00:00Z",
}

T4D_FULL = GOAL1_TODAY | {
    "algorithm_id": "T4D",
    "direct_out_count": 6,
    "direct_in_count": 6,
    "group_out_count": 0,
    "group_in_count": 0,
    "direct_active_day_count": 6,
    "last_direct_contact_utc": "2026-08-21T09:00:00Z",
    "silent_days": 3,
    "as_of_utc": TS,
}

# group_heavy_plus_one_direct_each_way, as T4D scores it (numbers read off
# `cargo run -p soul-algo-tie --example matrix`): 30 group rows over 10 days plus
# one private row each way -> Weak, and the edge carries the numbers that say why.
T4D_GROUP_HEAVY = {
    "band": "weak",
    "algorithm_id": "T4D",
    "interaction_count": 32,
    "outgoing_count": 16,
    "incoming_count": 16,
    "direct_out_count": 1,
    "direct_in_count": 1,
    "group_out_count": 15,
    "group_in_count": 15,
    "conversation_count": 2,
    "active_day_count": 10,
    "direct_active_day_count": 1,
    "first_contact_utc": "2026-08-13T09:00:00Z",
    "last_contact_utc": "2026-08-22T09:00:00Z",
    "last_direct_contact_utc": "2026-08-22T09:00:00Z",
    "silent_days": 2,
    "as_of_utc": TS,
}

CASES: list[tuple[str, dict, bool]] = [
    ("goal1_today_eight_fields", GOAL1_TODAY, True),
    ("t4d_full_bundle_lilei_12", T4D_FULL, True),
    ("t4d_group_heavy_is_weak_with_split_counts", T4D_GROUP_HEAVY, True),
    ("group_only_has_no_last_direct_contact", T4D_GROUP_HEAVY | {"direct_out_count": 0, "direct_in_count": 0, "direct_active_day_count": 0, "last_direct_contact_utc": None}, True),
    # rejections
    ("half_migration_split_without_algorithm_id", GOAL1_TODAY | {"direct_out_count": 6}, False),
    ("half_migration_algorithm_id_without_split", GOAL1_TODAY | {"algorithm_id": "T4D"}, False),
    ("t4d_bundle_missing_as_of", {k: v for k, v in T4D_FULL.items() if k != "as_of_utc"}, False),
    ("invented_score_field", T4D_FULL | {"score": 0.87}, False),
    ("invented_weight_field", T4D_FULL | {"tie_weight": 3}, False),
    ("band_none_is_not_a_tie_band", T4D_FULL | {"band": "none"}, False),
    ("negative_count", T4D_FULL | {"direct_in_count": -1}, False),
    ("third_algorithm_smuggled_in", T4D_FULL | {"algorithm_id": "T5"}, False),
    ("as_of_is_not_a_unix_integer", T4D_FULL | {"as_of_utc": 1787529600}, False),
]


def sha256(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def validator_for(schema: dict) -> Draft202012Validator:
    defs = json.loads(DEFS.read_text(encoding="utf-8"))
    registry = Registry().with_resources(
        [
            (defs["$id"], Resource.from_contents(defs)),
            (schema["$id"], Resource.from_contents(schema)),
        ]
    )
    Draft202012Validator.check_schema(schema)
    return Draft202012Validator(schema, registry=registry)


def run(name: str, validator: Draft202012Validator) -> int:
    failures = 0
    for case, tie, want_valid in CASES:
        errors = list(validator.iter_errors(BASE | {"tie_strength": tie}))
        got_valid = not errors
        mark = "ok " if got_valid == want_valid else "FAIL"
        if got_valid != want_valid:
            failures += 1
        detail = "" if got_valid else f"  <- {errors[0].message[:90]}"
        print(f"  [{mark}] {case}: expected {'valid' if want_valid else 'rejected'}{detail}")
    print(f"  {name}: {len(CASES) - failures}/{len(CASES)} as expected")
    return failures


def main() -> int:
    candidate_text = CANDIDATE.read_text(encoding="utf-8")
    candidate = json.loads(candidate_text)

    goal1_text = subprocess.run(
        ["git", "show", f"{GOAL1_REF}:docs/schemas/relationship.schema.json"],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    goal1 = json.loads(goal1_text)

    print("hashes (LF, as `schema-freeze` computes them)")
    print(f"  goal1 relationship.schema.json : {sha256(goal1_text)}")
    print(f"  schemas.lock.json pins         : {LOCKED_RELATIONSHIP_SHA}")
    print(f"  candidate                      : {sha256(candidate_text)}")
    lock_ok = sha256(goal1_text) == LOCKED_RELATIONSHIP_SHA
    print(f"  goal1 file matches its lock    : {lock_ok}")
    print(f"  candidate changes the lock     : {sha256(candidate_text) != LOCKED_RELATIONSHIP_SHA}")

    print("\ntop-level `required` is byte-identical to Goal 1 (no silent tightening)")
    same_required = candidate["required"] == goal1["required"]
    print(f"  {candidate['required']}")
    print(f"  identical: {same_required}")

    print("\ncandidate")
    failures = run("candidate", validator_for(candidate))

    print("\nGoal 1 schema today, same cases (shows what a bare `object` lets through)")
    goal1_validator = validator_for(goal1)
    caught = 0
    for case, tie, want_valid in CASES:
        errors = list(goal1_validator.iter_errors(BASE | {"tie_strength": tie}))
        if not want_valid and errors:
            caught += 1
    rejections = sum(1 for _, _, want_valid in CASES if not want_valid)
    print(f"  rejects {caught}/{rejections} of the cases the candidate rejects")

    ok = failures == 0 and lock_ok and same_required
    print("\nRESULT:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
