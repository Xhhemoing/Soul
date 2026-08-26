#!/usr/bin/env python3
"""Parse/compile all schemas and run focused looseness probes."""

from __future__ import annotations

import argparse
import json
from importlib.metadata import version
from pathlib import Path
from typing import Any


EXPECTED_A0_POSITIONS = ["leans_low", "mixed", "leans_high", "unknown"]
DECISION_TIE_OUTPUT_FIELDS = [
    "band",
    "raw counts",
    "direct/group split",
    "last_contact",
    "silent_days",
    "as_of",
]
DECISION_CONSTANTS = {
    "MODERATE_MIN_INTERACTIONS": 3,
    "STRONG_MIN_INTERACTIONS": 10,
    "STRONG_MIN_ACTIVE_DAYS": 3,
    "DEMOTE_ONE_BAND_DAYS": 180,
    "FORCE_WEAK_DAYS": 360,
    "DORMANT_NOTE_DAYS": "DEMOTE_ONE_BAND_DAYS",
}


def load_schemas(schema_dir: Path) -> dict[str, dict[str, Any]]:
    loaded: dict[str, dict[str, Any]] = {}
    for path in sorted(schema_dir.glob("*.json")):
        value = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(value, dict):
            raise TypeError(f"{path}: schema root is not an object")
        loaded[path.name] = value
    return loaded


def relationship_shape(schema: dict[str, Any]) -> dict[str, Any]:
    tie = schema["properties"]["tie_strength"]
    return {
        "relationship_required": schema["required"],
        "tie_strength_required_at_root": "tie_strength" in schema["required"],
        "tie_strength_schema": tie,
        "tie_strength_has_properties": "properties" in tie,
        "tie_strength_has_required": "required" in tie,
        "tie_strength_closes_additional_properties": tie.get("additionalProperties") is False,
        "decision_tie_output_fields": DECISION_TIE_OUTPUT_FIELDS,
        "decision_constants": DECISION_CONSTANTS,
        "constants_are_data_fields_required_by_decision": False,
        "note": (
            "The constants belong in the algorithm's single definition, not as "
            "serialized fields. The schema nevertheless does not constrain the "
            "fields needed to audit their resulting TieScore."
        ),
    }


def profile_shape(schema: dict[str, Any]) -> dict[str, Any]:
    axis = schema["properties"]["trait_axes"]["items"]
    serialized = json.dumps(schema, ensure_ascii=False).lower()
    return {
        "declares_score_property": '"score"' in serialized,
        "declares_percentile_property": '"percentile"' in serialized,
        "axis_additional_properties": axis.get("additionalProperties"),
        "axis_required": axis["required"],
        "position_enum": axis["properties"]["position"]["enum"],
        "matches_a0_position_enum": (
            axis["properties"]["position"]["enum"] == EXPECTED_A0_POSITIONS
        ),
        "locked_by_user_required": "locked_by_user" in axis["required"],
        "evidence_ids_required": "evidence_ids" in axis["required"],
        "nested_looseness": (
            "voice is a free object and values/boundaries are untyped arrays, so "
            "the schema does not globally forbid score/percentile keys in those "
            "containers even though trait-axis items forbid extra keys."
        ),
    }


def run_jsonschema_checks(schemas: dict[str, dict[str, Any]]) -> dict[str, Any]:
    try:
        import jsonschema
        from jsonschema.validators import validator_for
    except ImportError as exc:
        return {
            "available": False,
            "reason": str(exc),
            "compiled": [],
            "focused_probes": {},
        }

    compiled: list[str] = []
    validators: dict[str, Any] = {}
    for name, schema in schemas.items():
        validator_class = validator_for(schema)
        validator_class.check_schema(schema)
        validators[name] = validator_class(schema)
        compiled.append(name)

    relationship_validator = validators["relationship.schema.json"]
    relationship_base = {
        "schema_version": "1.0.0",
        "relationship_id": "relationship-1",
        "from_contact_id": "self",
        "to_contact_id": "peer",
        "evidence_ids": ["evidence-1"],
    }
    relationship_validator.validate(relationship_base)
    relationship_validator.validate({**relationship_base, "tie_strength": {}})
    relationship_validator.validate(
        {
            **relationship_base,
            "tie_strength": {
                "score": 9.75,
                "percentile": 99,
                "algorithm": "not-frozen",
                "arbitrary": ["anything"],
            },
        }
    )

    profile_validator = validators["profile.schema.json"]
    profile_base = {
        "schema_version": "1.0.0",
        "profile_id": "profile-1",
        "voice": {},
        "trait_axes": [
            {
                "axis_id": "curiosity",
                "position": "leans_low",
                "evidence_band": "moderate",
                "clinical_claim": False,
            }
        ],
        "clinical_claim": False,
    }
    profile_validator.validate(profile_base)
    profile_validator.validate({**profile_base, "voice": {"score": 88}})

    axis_score_rejected = False
    axis_score_error = ""
    with_axis_score = json.loads(json.dumps(profile_base))
    with_axis_score["trait_axes"][0]["score"] = 88
    try:
        profile_validator.validate(with_axis_score)
    except jsonschema.ValidationError as exc:
        axis_score_rejected = True
        axis_score_error = exc.message

    numeric_position_rejected = False
    numeric_position_error = ""
    with_numeric_position = json.loads(json.dumps(profile_base))
    with_numeric_position["trait_axes"][0]["position"] = 0.8
    try:
        profile_validator.validate(with_numeric_position)
    except jsonschema.ValidationError as exc:
        numeric_position_rejected = True
        numeric_position_error = exc.message

    return {
        "available": True,
        "version": version("jsonschema"),
        "compiled": compiled,
        "focused_probes": {
            "relationship_without_tie_strength_accepted": True,
            "relationship_empty_tie_strength_accepted": True,
            "relationship_arbitrary_tie_strength_accepted": True,
            "profile_axis_without_lock_or_evidence_ids_accepted": True,
            "profile_voice_with_score_accepted": True,
            "profile_axis_score_rejected": axis_score_rejected,
            "profile_axis_score_error": axis_score_error,
            "profile_numeric_position_rejected": numeric_position_rejected,
            "profile_numeric_position_error": numeric_position_error,
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--schema-dir", type=Path, default=Path("/workspace/docs/schemas")
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).with_name("SCHEMA_RESULTS.json"),
    )
    args = parser.parse_args()

    schemas = load_schemas(args.schema_dir)
    result = {
        "schema_directory": str(args.schema_dir),
        "parsed_count": len(schemas),
        "parsed": sorted(schemas),
        "relationship": relationship_shape(schemas["relationship.schema.json"]),
        "profile": profile_shape(schemas["profile.schema.json"]),
        "jsonschema": run_jsonschema_checks(schemas),
    }
    args.output.write_text(
        json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
