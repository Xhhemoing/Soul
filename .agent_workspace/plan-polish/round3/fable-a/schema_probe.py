#!/usr/bin/env python3
"""Round 3 fable-a: independent probe of docs/schemas/relationship.schema.json
(tie_strength typing per D58/D59). Read-only against docs/; writes nothing."""
import json
import sys
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

SCHEMAS = Path("/workspace/docs/schemas")

rel = json.loads((SCHEMAS / "relationship.schema.json").read_text())
defs = json.loads((SCHEMAS / "_defs.schema.json").read_text())
registry = Registry().with_resources(
    [(s["$id"], Resource.from_contents(s)) for s in (rel, defs)]
)
validator = Draft202012Validator(rel, registry=registry)

U = "0123abcd-0123-7abc-89ab-0123456789ab"  # matches _defs uuid7 pattern
TS = "2026-08-01T00:00:00Z"


def doc(tie=None, include_tie=True):
    d = {
        "schema_version": "1.0.0",
        "relationship_id": U,
        "from_contact_id": U,
        "to_contact_id": U,
        "evidence_ids": [U],
        "egress_scope": "local_only",
    }
    if include_tie:
        d["tie_strength"] = tie
    return d


FULL_T4D = {
    "algorithm_id": "T4D",
    "band": "weak",
    "interaction_count": 32,
    "outgoing_count": 16,
    "incoming_count": 16,
    "conversation_count": 2,
    "active_day_count": 10,
    "first_contact_utc": TS,
    "last_contact_utc": TS,
    "direct_out_count": 1,
    "direct_in_count": 1,
    "group_out_count": 15,
    "group_in_count": 15,
    "direct_active_day_count": 1,
    "silent_days": 23,
    "as_of_utc": "2026-08-24T00:00:00Z",
}

CASES = [
    # (name, document, expect_valid, what it proves)
    ("empty_object", doc({}), True,
     "Goal 1 today's looseness preserved: no algorithm_id => no required set (D59)"),
    ("tie_omitted", doc(include_tie=False), True,
     "tie_strength itself is not required"),
    ("score_rejected", doc({"score": 0.87}), False,
     "D22 enforcement point: additionalProperties=false kills scores/weights"),
    ("half_migration_rejected", doc({"algorithm_id": "T4D", "band": "strong"}), False,
     "algorithm_id triggers the full reviewable package (if/then required)"),
    ("full_t4d_plus_machine_band", doc({**FULL_T4D, "machine_band": "weak"}), True,
     "D32/D59: rebuilt edge carrying machine_band must stay green"),
    ("full_t4d_locked_edge", doc({**FULL_T4D, "machine_band": "moderate",
                                  "user_band": "strong", "locked_by_user": True}), True,
     "D48 locked edge: user_band+locked_by_user allowed alongside package"),
    ("user_band_without_lock_flag", doc({**FULL_T4D, "user_band": "strong"}), False,
     "dependentRequired: user_band demands locked_by_user"),
    ("split_count_without_algorithm", doc({"direct_out_count": 3}), False,
     "dependentRequired: T4D fields cannot appear without declaring the algorithm"),
    ("third_algorithm_rejected", doc({**FULL_T4D, "algorithm_id": "T5"}), False,
     "enum [T4D, T4]: no third banding rule can sneak in via a new string"),
    ("band_none_rejected", doc({**FULL_T4D, "band": "none"}), False,
     "band narrowed to weak/moderate/strong inside tie_strength"),
    ("null_last_direct_ok", doc({**FULL_T4D, "last_direct_contact_utc": None}), True,
     "never-direct peer: null allowed, field not in the required list"),
    ("lock_flag_alone_accepted", doc({**FULL_T4D, "locked_by_user": True}), True,
     "KNOWN SOFT SPOT (expected-accept): schema does not force user_band when "
     "locked_by_user=true; the locked<=>user_band.is_some() biconditional (D32) "
     "is enforced in Goal 1 code/tests, not in schema bytes"),
]

failures = 0
for name, document, expect_valid, why in CASES:
    errors = sorted(validator.iter_errors(document), key=str)
    ok = (not errors) == expect_valid
    status = "PASS" if ok else "FAIL"
    if not ok:
        failures += 1
    print(f"[{status}] {name}: expect {'valid' if expect_valid else 'invalid'}, "
          f"got {'valid' if not errors else 'invalid'}")
    print(f"        {why}")
    if errors and (not ok or not expect_valid):
        print(f"        first error: {errors[0].message[:120]}")

print(f"\n{len(CASES) - failures}/{len(CASES)} probes behaved as expected")
sys.exit(1 if failures else 0)
