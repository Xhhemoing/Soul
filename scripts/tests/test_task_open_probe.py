"""Synthetic-only boundary tests; all HTTP calls use an in-memory fake."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch


SCRIPTS = Path(__file__).resolve().parents[1]


def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / filename)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class FakeHttp:
    def __init__(self):
        self.requests = []

    def open(self, request, timeout):
        self.requests.append(request)
        index = len(self.requests)
        if request.full_url.endswith('/usage'):
            value = {"unit": "USD", "usage": {"total": {
                "actual_cost": 10 if index == 1 else 10.01,
                "requests": 20 if index == 1 else 21}}}
        else:
            value = {"model": "gpt-6-sol", "status": "completed", "usage": {},
                     "output": [{"type": "message", "content": [
                         {"type": "output_text", "text": '{"case_id":"abc123"}'}]}]}
        return io.BytesIO(json.dumps(value).encode('utf-8'))


class RunnerReuseTests(unittest.TestCase):
    def run_case(self, custom):
        gpt = load('gpt_probe_test', 'task-gpt-probe.py')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'input.jsonl'
            row = {"case_id": "abc123", "task": {}, "information_options": [],
                   "candidates": [], "unavailable_ids": []}
            if custom:
                row = {"case_id": "abc123", "task_text": "synthetic task", "sources": []}
            source.write_text(json.dumps(row) + '\n', encoding='utf-8')
            output = root / 'output'
            http = FakeHttp()
            argv = ['probe', '--inputs', str(source), '--output', str(output),
                    '--model', 'gpt-6-sol', '--ledger', str(root / 'ledger.jsonl')]
            with patch.object(sys, 'argv', argv), patch.dict(os.environ, {'SOUL_PROBE_GPT_KEY': 'synthetic-key'}), \
                    patch.object(gpt.urllib.request, 'build_opener', return_value=http), \
                    contextlib.redirect_stdout(io.StringIO()):
                if custom:
                    try:
                        gpt.main(input_loader=lambda path: [row],
                                 instruction_builder=lambda item: 'open synthetic instructions',
                                 max_output_tokens=3072)
                    except TypeError as error:
                        self.fail(f'Existing budgeted runner cannot yet accept the open protocol: {error}')
                else:
                    gpt.main()
            body = json.loads(http.requests[1].data)
            recorded = json.loads((output / 'run-0.json').read_text(encoding='utf-8'))
            measurement = json.loads((output / 'measurements.jsonl').read_text(encoding='utf-8'))
            ledger = json.loads((root / 'ledger.jsonl').read_text(encoding='utf-8'))
            self.assertEqual(len(http.requests), 3)
            self.assertTrue(measurement['billing_verified_for_single_request'])
            self.assertAlmostEqual(ledger['api_cost_usd'], .01)
            self.assertEqual(body['max_output_tokens'], 3072 if custom else 768)
            self.assertEqual(recorded['max_output_tokens'], body['max_output_tokens'])
            self.assertEqual(body['instructions'], recorded['prompt'])
            self.assertFalse(body['store'])
            self.assertEqual(json.loads(body['input'][0]['content']), row)

    def test_existing_protocol_and_budget_receipts_remain_unchanged(self):
        self.run_case(False)

    def test_open_protocol_reuses_actual_billing_path_and_records_output_limit(self):
        self.run_case(True)


class OpenInputTests(unittest.TestCase):
    def setUp(self):
        self.module = load('open_probe_test', 'task-open-probe.py')
        self.row = {'case_id': 'abc123', 'task_text': 'Write a synthetic note.',
                    'sources': [{'id': 's01', 'title': 'Synthetic', 'text': 'For team A.', 'readable': True}]}

    def read(self, row, prepared=False):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'inputs.jsonl'
            path.write_text(json.dumps(row) + '\n', encoding='utf-8')
            return (self.module.prepared_inputs if prepared else self.module.inputs)(path)

    def test_raw_task_needs_no_information_options(self):
        self.assertEqual(self.read(self.row), [self.row])

    def test_gold_cannot_be_attached_to_raw_task(self):
        self.row['gold'] = {'status': 'supported'}
        with self.assertRaisesRegex(ValueError, 'open_input_only_schema'):
            self.read(self.row)

    def test_gold_cannot_be_nested_in_a_source(self):
        self.row['sources'][0]['expected_status'] = 'missing'
        with self.assertRaisesRegex(ValueError, 'open_source_schema'):
            self.read(self.row)

    def test_prepared_control_cannot_enter_the_raw_loader(self):
        self.row['prepared_context'] = {'needs': [], 'source_metadata': [
            {'source_id': 's01', 'subject': 'team A', 'scope': None, 'time': None,
             'supersedes': [], 'correction_quote': None}]}
        with self.assertRaisesRegex(ValueError, 'open_input_only_schema'):
            self.read(self.row)
        self.assertEqual(self.read(self.row, prepared=True), [self.row])

    def test_prepared_control_cannot_carry_status_fields(self):
        self.row['prepared_context'] = {'needs': [], 'source_metadata': [], 'status': 'ready'}
        with self.assertRaisesRegex(ValueError, 'prepared_context_schema'):
            self.read(self.row, prepared=True)

    def test_duplicate_source_ids_are_rejected(self):
        self.row['sources'].append(dict(self.row['sources'][0]))
        with self.assertRaisesRegex(ValueError, 'open_source_id'):
            self.read(self.row)


if __name__ == '__main__':
    unittest.main()
