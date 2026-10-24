import test from 'node:test';
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { summarize, makeBatchStart, makeTask, makeBatchEnd } from './feedback.mjs';
import { editRatio } from './edit-ratio.mjs';

const at = '2026-10-23T10:00:00-07:00';
function batch(changes) {
  const start = makeBatchStart(at);
  return [start, ...changes.map(change => ({
    ...makeTask(start.batch_id, 'T3', at), actual_source: 'endpoint',
    outcome: 'accepted', content_acceptable: 'yes', severe_error: 'none',
    ...change,
  })), makeBatchEnd(start.batch_id, at)];
}
const run = (file, input, args = []) => spawnSync(process.execPath,
  [fileURLToPath(new URL(file, import.meta.url)), ...args], { input, encoding: 'utf8' });

test('Unicode code points, empty text and replacement have exact edit ratios', () => {
  assert.equal(editRatio('', ''), 0);
  assert.equal(editRatio('😀甲', '😀乙'), 0.5);
  assert.equal(editRatio('abc', ''), 1);
  assert.equal(editRatio('kitten', 'sitting'), 3 / 7);
  assert.equal(editRatio('é', 'e\u0301'), 1);
});

test('ten eligible correction opportunities with one recurrence give 1/10', () => {
  const data = batch(Array.from({ length: 10 }, (_, i) => ({
    correction_opportunity: 'yes', correction_status: i ? 'none' : 'd109_covered_repeated',
  })));
  data.splice(-1, 0, { ...makeTask(data[0].batch_id, 'T3', at) });
  const result = summarize(data);
  assert.deepEqual(result.strata.T3.endpoint.correction_recurrence,
    { numerator: 1, denominator: 10, rate: 0.1 });
  assert.equal(result.strata.T3.none.correction_opportunity_unassessed, 1);
  assert.equal(summarize(batch([])).strata.T3.endpoint.correction_recurrence.rate, null);
});

test('funnel retains downgrade, non-generation, abandonment, no-feedback and unassessed', () => {
  const records = batch([
    { edit_ratio: 0.1, task_seconds: 20 },
    { actual_source: 'template', fallback_reason: '回复为空', content_acceptable: 'not_applicable', severe_error: 'unassessed' },
    { actual_source: 'none', fallback_reason: '网络拒绝', outcome: 'not_generated', content_acceptable: 'unassessed', severe_error: 'unassessed' },
    { outcome: 'abandoned', content_acceptable: 'unassessed', severe_error: 'unassessed' },
    { outcome: 'no_feedback', content_acceptable: 'unassessed', severe_error: 'unassessed' },
    { content_acceptable: 'unassessed', severe_error: 'unassessed' },
    { outcome: 'light_edit', severe_error: 'commitment_or_boundary_reversal', edit_ratio: 0.2, task_seconds: 40 },
  ]);
  records.at(-1).recording_seconds = 9;
  const s = summarize(records);
  assert.equal(s.total.started, 7);
  assert.equal(s.total.generated, 6);
  assert.equal(s.total.with_feedback, 4);
  assert.equal(s.total.acceptability, undefined);
  assert.deepEqual(s.strata.T3.endpoint.acceptability, { numerator: 2, denominator: 2, rate: 1 });
  const edit = s.strata.T3.endpoint.edit_ratio;
  assert.ok(Math.abs(edit.median - 0.15) < Number.EPSILON);
  assert.deepEqual({ ...edit, median: null }, { measured: 2, missing: 3, not_generated: 0, median: null, distribution: { zero: 0, up_to_quarter: 2, up_to_half: 0, over_half: 0 } });
  assert.deepEqual(s.total.task_seconds, { measured: 2, missing: 5, sum: 60 });
  assert.equal(s.strata.T3.endpoint.task_seconds.median, 30);
  assert.equal(s.recording_seconds.sum, 9);
  assert.equal(s.strata.T3.template.acceptability.denominator, 0);
  assert.equal(s.strata.T3.endpoint.severe_errors.denominator, 2);
  assert.equal(s.strata.T3.endpoint.severe_errors.numerator, 1);
  assert.equal(s.strata.T3.endpoint.severe_error_unassessed, 3);
  assert.equal(s.total.outcomes.abandoned, 1);
  assert.equal(s.total.outcomes.no_feedback, 1);
  assert.equal(s.strata.T3.endpoint.acceptability_unassessed, 1);
  assert.equal(s.strata.T3.template.content_not_applicable, 1);
});

test('T1 skeleton assessment does not enter content or serious-error denominators', () => {
  const data = batch([]);
  data.splice(1, 0, { ...makeTask(data[0].batch_id, 'T1', at), actual_source: 'template', outcome: 'light_edit', origin: 'organic', scope_used: true, correction_status: 'unmemorable' });
  const s = summarize(data);
  assert.equal(s.strata.T1.template.acceptability.rate, 1);
  assert.equal(s.strata.T1.template.severe_errors.denominator, 0);
  assert.equal(s.total.origins.organic, 1);
  assert.equal(s.total.unmemorable, 1);
});

test('cancel after detecting privacy leakage remains classified without entering endpoint error rate', () => {
  const data = batch([{ actual_source: 'none', outcome: 'abandoned', content_acceptable: 'unassessed',
    severe_error: 'privacy', failure_layer: '辅助', failure_type: '隐私问题', severity: 3 }]);
  const s = summarize(data);
  assert.equal(s.total.outcomes.abandoned, 1);
  assert.equal(s.total.failures.types['隐私问题'], 1);
  assert.equal(s.strata.T3.none.non_endpoint_severe_events.privacy, 1);
  assert.equal(s.strata.T3.none.severe_errors.denominator, 0);
  assert.equal(s.strata.T3.none.severe_errors.rate, null);
});

test('ten non-generated tasks do not dilute the measured editing median', () => {
  const rows = Array.from({ length: 10 }, () => ({ edit_ratio: 0.1 }));
  rows.push(...Array.from({ length: 10 }, () => ({ actual_source: 'none', outcome: 'not_generated', content_acceptable: 'unassessed', severe_error: 'unassessed' })));
  const s = summarize(batch(rows));
  assert.equal(s.total.started, 20);
  assert.equal(s.strata.T3.endpoint.edit_ratio.median, 0.1);
  assert.equal(s.strata.T3.endpoint.edit_ratio.measured, 10);
  assert.equal(s.strata.T3.none.edit_ratio.not_generated, 10);
  assert.equal(s.strata.T3.none.edit_ratio.median, null);
});

test('unknown keys, enum values, linked identifiers and invalid numbers are rejected', () => {
  const changes = [ { note: 'PRIVATE_MARKER' }, { person_id: randomUUID() },
    { failure_type: 'PRIVATE_MARKER' }, { content_acceptable: 'PRIVATE_MARKER' },
    { edit_ratio: -1 }, { edit_ratio: 1.1 }, { task_seconds: -1 },
    { scope_used: 'true' }, { correction_status: 'd109_covered_repeated', correction_opportunity: 'no' },
    { actual_source: 'template', content_acceptable: 'yes' },
    { requested_mode: 'T1', actual_source: 'endpoint' },
    { recorded_at: '2026-10-23T10:00:00' }, { recorded_at: '2026-02-30T00:00:00Z' },
    { failure_layer: '平台' }, { task_id: 'person-name' },
  ];
  for (const change of changes) assert.throws(() => summarize(batch([change])), /^Error: INVALID_RECORD$/);
});

test('batch closure, task uniqueness, matching identifiers and time range are enforced', () => {
  const good = batch([{}]);
  assert.throws(() => summarize(good.slice(0, -1)));
  assert.throws(() => summarize([...good, good.at(-1)]));
  assert.throws(() => summarize([good[0], good[1], good[1], good[2]]));
  assert.throws(() => summarize([good[1], good[0], good[2]]));
  assert.throws(() => summarize(batch([{ batch_id: randomUUID() }])));
  assert.throws(() => summarize(batch([{ recorded_at: '2026-10-23T11:00:00-07:00' }])));
});

test('CLI errors never echo rejected body, field name, IDs, or input paths', () => {
  for (const input of ['PRIVATE_MARKER', '{"PRIVATE_MARKER":"secret"}', batch([{ note: 'PRIVATE_MARKER' }]).map(JSON.stringify).join('\n')]) {
    const r = run('./summarize.mjs', input);
    assert.equal(r.status, 1);
    assert.equal(r.stdout, '');
    assert.equal(r.stderr, 'INVALID_INPUT\n');
  }
  const args = run('./summarize.mjs', '', ['PRIVATE_MARKER']);
  assert.equal(args.status, 1);
  assert.equal(args.stderr, 'INVALID_INPUT\n');
});

test('CLI summarizes synthetic JSONL without leaking random row identifiers', () => {
  const data = batch([{ outcome: 'rejected', content_acceptable: 'no', failure_layer: '理解', failure_type: '理解错', severity: 2 }]);
  const r = run('./summarize.mjs', data.map(JSON.stringify).join('\n'));
  assert.equal(r.status, 0);
  assert.equal(r.stderr, '');
  for (const record of data) assert.equal(r.stdout.includes(record.batch_id), false);
  assert.equal(r.stdout.includes(data[1].task_id), false);
  assert.equal(JSON.parse(r.stdout).total.failures.types['理解错'], 1);
});

test('edit CLI accepts stdin only, prints a number, rejects incomplete or extra input', () => {
  const good = run('./edit-ratio.mjs', '😀甲\n.end\n😀乙\n.end\n');
  assert.equal(good.status, 0);
  assert.equal(good.stdout, '0.5\n');
  const bad = run('./edit-ratio.mjs', 'PRIVATE_MARKER\n.end\n');
  assert.equal(bad.status, 1);
  assert.equal(bad.stdout, '');
  assert.equal(bad.stderr.includes('PRIVATE_MARKER'), false);
  assert.equal(run('./edit-ratio.mjs', 'x\n.end\nx\n.end\nextra\n').status, 1);
  assert.equal(run('./edit-ratio.mjs', '', ['PRIVATE_MARKER']).status, 1);
});

test('template CLI supplies valid metadata and default missing measurements', () => {
  const start = run('./template.mjs', '', ['start']);
  assert.equal(start.status, 0);
  const b = JSON.parse(start.stdout);
  const task = run('./template.mjs', '', ['task', b.batch_id, 'T1']);
  assert.equal(task.status, 0);
  const row = JSON.parse(task.stdout);
  assert.equal(row.edit_ratio, null);
  assert.equal(row.content_acceptable, 'not_applicable');
  const end = run('./template.mjs', '', ['end', b.batch_id]);
  assert.equal(end.status, 0);
  assert.equal(summarize([b, row, JSON.parse(end.stdout)]).total.started, 1);
});

test('no measured time is unknown, and summary leaves are only numbers or null', () => {
  const s = summarize(batch([{}]));
  assert.equal(s.total.task_seconds.sum, null);
  assert.equal(s.recording_seconds.sum, null);
  const visit = value => {
    if (value !== null && typeof value === 'object') Object.values(value).forEach(visit);
    else assert.ok(value === null || (typeof value === 'number' && Number.isFinite(value)));
  };
  visit(s);
  assert.throws(() => editRatio('a'.repeat(5001), 'b'.repeat(5001)), /INPUT_TOO_LARGE/);
});

test('satisfactory tasks without failure classification are counted as tasks, not failures', () => {
  const s = summarize(batch([{}, { outcome: 'rejected', content_acceptable: 'no', failure_layer: '理解', failure_type: '理解错', severity: 2 }]));
  for (const group of [s.total, s.strata.T3.endpoint]) {
    assert.equal(group.unclassified_task_count, 1);
    assert.equal(Object.hasOwn(group.failures, 'unclassified'), false);
    assert.equal(Object.values(group.failures.types).reduce((a, b) => a + b, 0), 1);
    assert.equal(group.failures.types['理解错'], 1);
  }
});

test('unclassified task counts retain rejected tasks and agree across mode and source strata', () => {
  const s = summarize(batch([
    {},
    { outcome: 'rejected', content_acceptable: 'no' },
    { outcome: 'rejected', content_acceptable: 'no', failure_layer: '理解', failure_type: '理解错', severity: 2 },
    { requested_mode: 'T1', actual_source: 'template', content_acceptable: 'not_applicable' },
    { requested_mode: 'T1', actual_source: 'none', outcome: 'not_generated', content_acceptable: 'not_applicable' },
    { actual_source: 'template', fallback_reason: '端点未配置', content_acceptable: 'not_applicable' },
    { actual_source: 'none', outcome: 'abandoned', content_acceptable: 'unassessed' },
  ]));
  assert.equal(s.total.unclassified_task_count, 6);
  assert.equal(s.strata.T3.endpoint.unclassified_task_count, 2);
  assert.equal(s.strata.T1.endpoint.unclassified_task_count, 0);
  const strata = Object.values(s.strata).flatMap(Object.values);
  assert.equal(strata.reduce((sum, group) => sum + group.unclassified_task_count, 0), s.total.unclassified_task_count);
  for (const group of [s.total, ...strata]) {
    const classified = Object.values(group.failures.types).reduce((sum, count) => sum + count, 0);
    assert.equal(group.unclassified_task_count + classified, group.started);
    assert.equal(Object.hasOwn(group.failures, 'unclassified'), false);
  }
  assert.deepEqual(s.strata.T3.endpoint.acceptability, { numerator: 1, denominator: 3, rate: 1 / 3 });
});
