"""Synthetic in-memory ZIPs and temporary files; no Soul data or keys."""
import copy
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import warnings
import zipfile

SCRIPT = Path(__file__).resolve().parents[1] / "read-research-retention.py"
SPEC = importlib.util.spec_from_file_location("research_reader", SCRIPT)
READER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(READER)

NOTICE = "明文独立副本；原库遗忘不会删除本文件；不能用于恢复 Soul。"


def fixture():
    rows = [
        {"event_kind": "app.foreground", "time_bucket_utc": "2024-02-29T09:00:00Z", "aggregate_count": 2},
        {"event_kind": "app.foreground", "time_bucket_utc": "2024-02-29T10:00:00Z", "aggregate_count": 1},
    ]
    metadata = {
        "format_version": 1, "program_version": "0.1.0",
        "prepared_at_utc": "2024-03-02T01:02:03Z",
        "scope": {"from_day": "2024-02-29", "to_day": "2024-03-01"},
        "counts": {"candidate_events": 7, "included_events": 3,
                   "excluded_policy_events": 1, "excluded_unavailable_events": 1,
                   "excluded_time_events": 1, "out_of_range_events": 1},
        "row_count": 2, "fields": ["event_kind", "time_bucket_utc", "aggregate_count"],
        "purpose": "research", "population": "self", "granularity": "hour",
        "excluded_values": ["body", "identifier", "app", "duration", "profile"],
        "counts_timing": "preparation", "notice": NOTICE,
    }
    return metadata, rows


def archive(metadata=None, rows=None, *, extra=None, omit=None, compression=zipfile.ZIP_STORED):
    default_meta, default_rows = fixture()
    metadata = default_meta if metadata is None else metadata
    rows = default_rows if rows is None else rows
    entries = {
        "records.jsonl": "".join(json.dumps(row, ensure_ascii=False) + "\n" for row in rows).encode(),
        "metadata.json": json.dumps(metadata, ensure_ascii=False).encode(),
        "README.txt": NOTICE.encode(),
    }
    if omit:
        entries.pop(omit)
    if extra:
        entries.update(extra)
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w", compression=compression) as z:
        for name, data in entries.items():
            z.writestr(name, data)
    return output.getvalue()


class ReaderTests(unittest.TestCase):
    def read(self, data):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic.zip"
            path.write_bytes(data)
            before = sorted(Path(directory).iterdir())
            result = READER.read_archive(path)
            self.assertEqual(sorted(Path(directory).iterdir()), before)
            return result

    def reject(self, data):
        with self.assertRaises(READER.InvalidArchive):
            self.read(data)

    def test_valid_summary_and_no_extracted_files(self):
        result = self.read(archive())
        self.assertEqual(result, {"format_version": 1, "row_count": 2, "included_events": 3,
                                  "candidate_events": 7, "excluded_events": 4})

    def test_empty_dataset_is_valid(self):
        meta, _ = fixture()
        meta["row_count"] = 0
        meta["counts"] = dict.fromkeys(meta["counts"], 0)
        self.assertEqual(self.read(archive(meta, []))["row_count"], 0)

    def test_version_missing_entries_extra_entries_and_compression(self):
        meta, _ = fixture()
        for version in [2, True, "1"]:
            bad = copy.deepcopy(meta)
            bad["format_version"] = version
            self.reject(archive(bad))
        self.reject(archive(omit="README.txt"))
        self.reject(archive(extra={"../secret": b"synthetic"}))
        self.reject(archive(compression=zipfile.ZIP_DEFLATED))

    def test_corruption_truncation_oversize_duplicate_and_encrypted_flag(self):
        data = archive()
        self.reject(data[:-12])
        self.reject(b"not a zip")
        self.reject(data + b"x" * (1024 * 1024))
        damaged = bytearray(data)
        position = damaged.index(b"app.foreground")
        damaged[position] ^= 1
        self.reject(damaged)
        duplicate = io.BytesIO(data)
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", UserWarning)
            with zipfile.ZipFile(duplicate, "a") as z:
                z.writestr("README.txt", NOTICE)
        self.reject(duplicate.getvalue())
        flagged = bytearray(data)
        central = flagged.index(b"PK\x01\x02")
        flagged[central + 8] |= 1
        self.reject(flagged)

    def test_unknown_fields_duplicate_json_keys_and_wrong_types(self):
        meta, rows = fixture()
        for key in ["body", "event_id"]:
            bad = copy.deepcopy(rows)
            bad[0][key] = "synthetic private value"
            self.reject(archive(rows=bad))
        for field in ["row_count", "format_version"]:
            bad = copy.deepcopy(meta)
            bad[field] = True
            self.reject(archive(bad))
        bad = copy.deepcopy(meta)
        bad["extra"] = 1
        self.reject(archive(bad))
        bad = copy.deepcopy(meta)
        bad["counts"]["included_events"] = True
        self.reject(archive(bad))
        for count in [True, 0, -1, 2.0, "2", 100_001]:
            bad = copy.deepcopy(rows)
            bad[0]["aggregate_count"] = count
            self.reject(archive(rows=bad))
        self.reject(archive(extra={"metadata.json": b'{"format_version":1,"format_version":1}'}))
        for key in ["scope", "counts"]:
            bad = copy.deepcopy(meta)
            bad[key]["extra"] = 1
            self.reject(archive(bad))
        for key, value in [("purpose", "assistant"), ("population", "mixed"),
                           ("granularity", "event"), ("counts_timing", "live"),
                           ("prepared_at_utc", "2024-02-30T00:00:00Z")]:
            bad = copy.deepcopy(meta)
            bad[key] = value
            self.reject(archive(bad))

    def test_scope_time_sorting_and_conservation(self):
        meta, rows = fixture()
        for timestamp in ["2024-03-01T00:00:00Z", "2024-02-29T09:01:00Z",
                          "2024-02-29T09:00:00+00:00", "2023-02-29T09:00:00Z"]:
            bad = copy.deepcopy(rows)
            bad[0]["time_bucket_utc"] = timestamp
            self.reject(archive(rows=bad))
        self.reject(archive(rows=list(reversed(rows))))
        self.reject(archive(rows=[rows[0], rows[0]]))
        for field in ["included_events", "candidate_events"]:
            bad = copy.deepcopy(meta)
            bad["counts"][field] += 1
            self.reject(archive(bad))
        bad = copy.deepcopy(meta)
        bad["scope"]["from_day"] = "2023-02-29"
        self.reject(archive(bad))
        bad = copy.deepcopy(meta)
        bad["scope"]["to_day"] = bad["scope"]["from_day"]
        self.reject(archive(bad))

    def test_cli_only_prints_summary_or_generic_error(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic-private-filename.zip"
            path.write_bytes(archive())
            result = subprocess.run([sys.executable, "-B", str(SCRIPT), str(path)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0)
            self.assertEqual(json.loads(result.stdout)["included_events"], 3)
            path.write_bytes(b"synthetic private invalid content")
            result = subprocess.run([sys.executable, "-B", str(SCRIPT), str(path)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(result.stdout, "")
            self.assertEqual(result.stderr, "Research archive rejected.\n")
            self.assertNotIn(str(path), result.stderr)


if __name__ == "__main__":
    unittest.main()
