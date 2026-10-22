"""Research boundary regressions; no model imports, network, or real data."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
import json
from unittest.mock import patch, Mock

gpt_spec = importlib.util.spec_from_file_location("gpt_probe", Path(__file__).parents[1] / "task-gpt-probe.py")
gpt = importlib.util.module_from_spec(gpt_spec)
gpt_spec.loader.exec_module(gpt)

spec = importlib.util.spec_from_file_location("probe", Path(__file__).parents[1] / "task-model-probe.py")
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class Boundaries(unittest.TestCase):
    def test_metadata_input_is_explicit_and_cannot_smuggle_annotation_keys(self):
        row = {"case_id":"newcase","task":{},"information_options":[],
               "candidates":[{"id":"c1"}],"unavailable_ids":[]}
        self.assertEqual(probe.instructions(row), probe.INSTRUCTIONS)
        row["source_metadata"] = [{"source_id":"c1","subject":"synthetic",
            "scope":"example","valid_time":"2030-01-01","topics":["location"],"supersedes":[]}]
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "input.jsonl"
            path.write_text(json.dumps(row), encoding="utf-8")
            self.assertEqual(probe.inputs(path), [row])
            self.assertIn("source_metadata", probe.instructions(row))
            row["source_metadata"][0]["gold"] = "supported"
            path.write_text(json.dumps(row), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "source_metadata_schema"):
                probe.inputs(path)

    def test_gold_input_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "inputs.jsonl"
            path.write_text(json.dumps({"case_id": "a", "gold": {}}), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "input_only_schema"):
                probe.inputs(path)

    def test_timeout_reservation_is_never_released_or_retried(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "cost.jsonl"
            probe.reserve(path, probe.JEV_MAX_CHARGE * 1.5)
            with self.assertRaisesRegex(ValueError, "budget_exhausted"):
                probe.reserve(path, probe.JEV_MAX_CHARGE * 1.5)
            self.assertEqual(len(path.read_text(encoding="utf-8").splitlines()), 1)

    def test_redirect_never_carries_authorization(self):
        with self.assertRaisesRegex(ValueError, "redirect_rejected"):
            probe.NoRedirect().redirect_request(None, None, 302, "", {}, "https://other.invalid")

    def test_outputs_cannot_overwrite_failed_runs(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "result.json"
            probe.write_new(path, {"failure": True})
            with self.assertRaises(FileExistsError):
                probe.write_new(path, {"success": True})

    def test_successful_inference_with_billing_failure_blocks_resume(self):
        import io
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            rows = root / "input.jsonl"
            row = {"case_id":"abc","task":{},"information_options":[],"candidates":[],"unavailable_ids":[]}
            rows.write_text(json.dumps(row)+"\n", encoding="utf-8")
            ledger = root / "ledger.jsonl"
            ledger.write_text('{"api_cost_usd":0.1}\n', encoding="utf-8")
            replies = [io.BytesIO(json.dumps({"unit":"USD","usage":{"total":{"actual_cost":1,"requests":1}}}).encode()),
                       io.BytesIO(json.dumps({"model":"gpt-6-luna","status":"completed","output":[],"usage":{}}).encode()), TimeoutError()]
            opener = Mock()
            opener.open.side_effect = replies
            argv = ["probe","--model","gpt-6-luna","--inputs",str(rows),"--output",str(root/"run1"),"--ledger",str(ledger)]
            with patch.dict(gpt.os.environ, {"SOUL_PROBE_GPT_KEY":"synthetic-test-key"}, clear=True), patch.object(gpt.urllib.request,"build_opener",return_value=opener), patch("sys.argv",argv):
                with self.assertRaisesRegex(ValueError,"billing_query_failed_stop"):
                    gpt.main()
            self.assertIsNone(json.loads(ledger.read_text(encoding="utf-8").splitlines()[-1])["api_cost_usd"])
            self.assertTrue((root/"run1"/"prediction-abc.json").exists())
            measurement=json.loads((root/"run1"/"measurements.jsonl").read_text(encoding="utf-8"))
            self.assertTrue(measurement["inference_returned"])
            self.assertFalse(measurement["billing_verified_for_single_request"])
            opener.reset_mock()
            argv[argv.index(str(root/"run1"))]=str(root/"run2")
            with patch.dict(gpt.os.environ, {"SOUL_PROBE_GPT_KEY":"synthetic-test-key"}, clear=True), patch.object(gpt.urllib.request,"build_opener",return_value=opener), patch("sys.argv",argv):
                with self.assertRaisesRegex(ValueError,"unsettled_previous_request_stop"):
                    gpt.main()
            opener.open.assert_not_called()


if __name__ == "__main__":
    unittest.main()
