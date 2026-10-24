import { randomUUID } from 'node:crypto';

export const ENUMS = Object.freeze({
  requested_mode: ['T1', 'T3'], actual_source: ['template', 'endpoint', 'none'],
  fallback_reason: ['无', '端点未配置', '审批不匹配', '网络拒绝', '回复不可读', '回复为空', '其他'],
  origin: ['requested', 'organic'],
  outcome: ['not_generated', 'abandoned', 'rejected', 'heavy_edit', 'light_edit', 'accepted', 'no_feedback'],
  content_acceptable: ['yes', 'no', 'unassessed', 'not_applicable'],
  severe_error: ['none', 'fact_error', 'commitment_or_boundary_reversal', 'relationship_error', 'privacy', 'unassessed'],
  correction_status: ['none', 'd109_covered_new', 'd109_covered_repeated', 'unmemorable'],
  correction_opportunity: ['yes', 'no', 'unassessed'],
  failure_layer: ['感知', '理解', '召回', '辅助', '校正', '平台'],
  failure_type: ['缺信息', '理解错', '语气不像', '召回漏', '召回错', '事实错误', '承诺或边界反转', '关系误判', '隐私问题', '卡顿', '崩溃', '操作负担过重', '其他'],
});
const fail = () => { throw new Error('INVALID_RECORD'); };
const uuid = value => typeof value === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(value);
const number = value => typeof value === 'number' && Number.isFinite(value) && value >= 0;
const optionalNumber = value => value === null || number(value);
const feedbackOutcomes = ['rejected', 'heavy_edit', 'light_edit', 'accepted'];

function timestamp(value) {
  if (typeof value !== 'string') return false;
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.\d{1,3})?(Z|[+-](\d{2}):(\d{2}))$/.exec(value);
  if (!m || !Number.isFinite(Date.parse(value))) return false;
  const [, y, mo, d, h, min, s, , oh, om] = m;
  const year = +y, month = +mo, day = +d;
  const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const days = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  return month >= 1 && month <= 12 && day >= 1 && day <= days[month - 1]
    && +h < 24 && +min < 60 && +s < 60 && (oh === undefined || (+oh <= 14 && +om < 60 && (+oh !== 14 || +om === 0)));
}

export function makeBatchStart(at = new Date().toISOString()) {
  return { record_type: 'batch_start', batch_id: randomUUID(), recorded_at: at };
}
export function makeBatchEnd(batch_id, at = new Date().toISOString()) {
  return { record_type: 'batch_end', batch_id, recorded_at: at, recording_seconds: null };
}
export function makeTask(batch_id, requested_mode = 'T1', at = new Date().toISOString()) {
  return { record_type: 'task', task_id: randomUUID(), batch_id, recorded_at: at,
    requested_mode, actual_source: 'none', fallback_reason: '无', origin: 'requested', scope_used: false,
    outcome: 'not_generated', content_acceptable: requested_mode === 'T1' ? 'not_applicable' : 'unassessed',
    severe_error: 'unassessed', edit_ratio: null, task_seconds: null, correction_status: 'none',
    correction_opportunity: 'unassessed', failure_layer: null, failure_type: null, severity: null };
}

export function validateRecord(row) {
  if (!row || typeof row !== 'object' || Array.isArray(row)) fail();
  let fields;
  if (row.record_type === 'batch_start') fields = ['record_type', 'batch_id', 'recorded_at'];
  else if (row.record_type === 'batch_end') fields = ['record_type', 'batch_id', 'recorded_at', 'recording_seconds'];
  else if (row.record_type === 'task') fields = Object.keys(makeTask(''));
  else fail();
  if (Object.keys(row).length !== fields.length || !fields.every(k => Object.hasOwn(row, k))) fail();
  if (!uuid(row.batch_id) || !timestamp(row.recorded_at)) fail();
  if (row.record_type === 'batch_end' && !optionalNumber(row.recording_seconds)) fail();
  if (row.record_type !== 'task') return row;
  if (!uuid(row.task_id)) fail();
  for (const [field, values] of Object.entries(ENUMS)) {
    if ((field === 'failure_layer' || field === 'failure_type') && row[field] === null) continue;
    if (!values.includes(row[field])) fail();
  }
  if (typeof row.scope_used !== 'boolean' || !optionalNumber(row.edit_ratio)
    || (row.edit_ratio !== null && row.edit_ratio > 1) || !optionalNumber(row.task_seconds)) fail();
  if (![null, 1, 2, 3].includes(row.severity)) fail();
  const classification = [row.failure_layer, row.failure_type, row.severity];
  if (classification.some(v => v === null) && !classification.every(v => v === null)) fail();
  if (row.requested_mode === 'T1' && (row.actual_source === 'endpoint' || row.content_acceptable !== 'not_applicable')) fail();
  if (row.actual_source === 'endpoint' && (row.fallback_reason !== '无' || row.scope_used || row.content_acceptable === 'not_applicable')) fail();
  if (row.actual_source === 'template' && row.content_acceptable !== 'not_applicable') fail();
  if (row.actual_source === 'none' && (!['not_generated', 'abandoned'].includes(row.outcome) || row.edit_ratio !== null || row.scope_used)) fail();
  if (row.actual_source !== 'none' && row.outcome === 'not_generated') fail();
  if (row.actual_source === 'none' && row.requested_mode === 'T3' && row.content_acceptable !== 'unassessed') fail();
  if (row.requested_mode === 'T1' && row.fallback_reason !== '无') fail();
  if (row.requested_mode === 'T3' && row.actual_source === 'template' && row.fallback_reason === '无') fail();
  if (row.correction_status === 'd109_covered_repeated' && row.correction_opportunity !== 'yes') fail();
  return row;
}

const counts = values => Object.fromEntries(values.map(v => [v, 0]));
const ratio = (numerator, denominator) => ({ numerator, denominator, rate: denominator ? numerator / denominator : null });
function median(values) {
  if (!values.length) return null;
  const a = [...values].sort((a, b) => a - b), mid = Math.floor(a.length / 2);
  return a.length % 2 ? a[mid] : (a[mid - 1] + a[mid]) / 2;
}
function metrics(rows) {
  const generated = rows.filter(r => r.actual_source !== 'none');
  const feedback = generated.filter(r => feedbackOutcomes.includes(r.outcome));
  const assessed = feedback.filter(r => r.requested_mode === 'T1' || (r.actual_source === 'endpoint' && ['yes', 'no'].includes(r.content_acceptable)));
  const acceptable = assessed.filter(r => ['accepted', 'light_edit'].includes(r.outcome) && (r.requested_mode === 'T1' || r.content_acceptable === 'yes'));
  const endpoint = rows.filter(r => r.actual_source === 'endpoint');
  const severeAssessed = endpoint.filter(r => r.severe_error !== 'unassessed');
  const severeTypes = counts(ENUMS.severe_error.filter(v => !['none', 'unassessed'].includes(v)));
  const otherSevereTypes = counts(Object.keys(severeTypes));
  for (const r of rows) if (r.actual_source !== 'endpoint' && Object.hasOwn(otherSevereTypes, r.severe_error)) otherSevereTypes[r.severe_error]++;
  for (const r of severeAssessed) if (r.severe_error !== 'none') severeTypes[r.severe_error]++;
  const opportunities = rows.filter(r => r.correction_opportunity === 'yes');
  const edits = generated.filter(r => r.edit_ratio !== null).map(r => r.edit_ratio);
  const seconds = rows.filter(r => r.task_seconds !== null).map(r => r.task_seconds);
  const distribution = { zero: 0, up_to_quarter: 0, up_to_half: 0, over_half: 0 };
  for (const value of edits) distribution[value === 0 ? 'zero' : value <= .25 ? 'up_to_quarter' : value <= .5 ? 'up_to_half' : 'over_half']++;
  const outcomes = counts(ENUMS.outcome), sources = counts(ENUMS.actual_source), origins = counts(ENUMS.origin);
  const failures = { layers: counts(ENUMS.failure_layer), types: counts(ENUMS.failure_type), severity: { '1': 0, '2': 0, '3': 0 } };
  let unclassifiedTaskCount = 0;
  const unmemorableTypes = { ...counts(ENUMS.failure_type), unclassified: 0 };
  const fallbackReasons = counts(ENUMS.fallback_reason);
  for (const r of rows) {
    outcomes[r.outcome]++; sources[r.actual_source]++; origins[r.origin]++; fallbackReasons[r.fallback_reason]++;
    if (r.failure_layer !== null) { failures.layers[r.failure_layer]++; failures.types[r.failure_type]++; failures.severity[r.severity]++; }
    else unclassifiedTaskCount++;
    if (r.correction_status === 'unmemorable') unmemorableTypes[r.failure_type ?? 'unclassified']++;
  }
  return { started: rows.length, generated: generated.length, sources, with_feedback: feedback.length,
    outcomes, origins, fallback_reasons: fallbackReasons, scope_used: rows.filter(r => r.scope_used).length,
    acceptability: ratio(acceptable.length, assessed.length),
    acceptability_unassessed: feedback.filter(r => r.content_acceptable === 'unassessed').length,
    content_not_applicable: feedback.filter(r => r.requested_mode === 'T3' && r.content_acceptable === 'not_applicable').length,
    edit_ratio: { measured: edits.length, missing: generated.length - edits.length, not_generated: rows.length - generated.length, median: median(edits), distribution },
    task_seconds: { measured: seconds.length, missing: rows.length - seconds.length, median: median(seconds), sum: seconds.length ? seconds.reduce((a, b) => a + b, 0) : null },
    severe_errors: { ...ratio(severeAssessed.filter(r => r.severe_error !== 'none').length, severeAssessed.length), types: severeTypes },
    severe_error_unassessed: endpoint.length - severeAssessed.length,
    non_endpoint_severe_events: otherSevereTypes,
    correction_recurrence: ratio(opportunities.filter(r => r.correction_status === 'd109_covered_repeated').length, opportunities.length),
    correction_opportunity_unassessed: rows.filter(r => r.correction_opportunity === 'unassessed').length,
    correction_opportunity_not_applicable: rows.filter(r => r.correction_opportunity === 'no').length,
    unmemorable: rows.filter(r => r.correction_status === 'unmemorable').length,
    unmemorable_types: unmemorableTypes, unclassified_task_count: unclassifiedTaskCount, failures };
}

export function summarize(records) {
  const batches = new Map(), tasks = [], ids = new Set(), recording = [];
  for (const row of records) {
    validateRecord(row);
    if (row.record_type === 'batch_start') {
      if (batches.has(row.batch_id)) fail();
      batches.set(row.batch_id, { start: Date.parse(row.recorded_at), end: null, latest: Date.parse(row.recorded_at) });
    } else {
      const b = batches.get(row.batch_id), time = Date.parse(row.recorded_at);
      if (!b || b.end !== null || time < b.start) fail();
      if (row.record_type === 'batch_end') {
        if (time < b.latest) fail();
        b.end = time; recording.push(row.recording_seconds);
      } else {
        if (ids.has(row.task_id)) fail();
        ids.add(row.task_id); tasks.push(row); b.latest = Math.max(b.latest, time);
      }
    }
  }
  if ([...batches.values()].some(b => b.end === null)) fail();
  const measuredRecording = recording.filter(v => v !== null);
  const all = metrics(tasks);
  // Overall counts retain the complete funnel. Quality rates/medians are only
  // exposed within requested-mode/actual-source strata, never pooled T1/T3.
  const total = { started: all.started, generated: all.generated, sources: all.sources,
    with_feedback: all.with_feedback, outcomes: all.outcomes, origins: all.origins,
    fallback_reasons: all.fallback_reasons, scope_used: all.scope_used,
    task_seconds: { measured: all.task_seconds.measured, missing: all.task_seconds.missing, sum: all.task_seconds.sum },
    unclassified_task_count: all.unclassified_task_count,
    failures: all.failures, unmemorable: all.unmemorable, unmemorable_types: all.unmemorable_types };
  return { batches: batches.size,
    recording_seconds: { measured_batches: measuredRecording.length, missing_batches: recording.length - measuredRecording.length, sum: measuredRecording.length ? measuredRecording.reduce((a, b) => a + b, 0) : null },
    total, strata: Object.fromEntries(ENUMS.requested_mode.map(mode => [mode,
      Object.fromEntries(ENUMS.actual_source.map(source => [source, metrics(tasks.filter(r => r.requested_mode === mode && r.actual_source === source))]))])) };
}
