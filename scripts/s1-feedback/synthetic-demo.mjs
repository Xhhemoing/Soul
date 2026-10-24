import { makeBatchStart, makeTask, makeBatchEnd } from './feedback.mjs';

// Metadata only: no endpoint call, product database or text fixture.
const at = '2026-10-23T10:00:00-07:00';
const start = makeBatchStart(at);
const variants = [
  { requested_mode: 'T1', actual_source: 'template', outcome: 'accepted', content_acceptable: 'not_applicable', scope_used: true, origin: 'organic', edit_ratio: 0.1, task_seconds: 30, correction_opportunity: 'yes' },
  { actual_source: 'endpoint', outcome: 'light_edit', content_acceptable: 'yes', severe_error: 'none', edit_ratio: 0.1, task_seconds: 45 },
  { actual_source: 'template', fallback_reason: '回复为空', outcome: 'heavy_edit', content_acceptable: 'not_applicable', edit_ratio: 0.8, task_seconds: 90, failure_layer: '辅助', failure_type: '缺信息', severity: 2 },
  { actual_source: 'none', outcome: 'abandoned', severe_error: 'privacy', failure_layer: '辅助', failure_type: '隐私问题', severity: 3 },
  { actual_source: 'none', fallback_reason: '网络拒绝', outcome: 'not_generated', failure_layer: '平台', failure_type: '其他', severity: 2 },
  { actual_source: 'endpoint', outcome: 'no_feedback' },
  { actual_source: 'endpoint', outcome: 'accepted', content_acceptable: 'unassessed' },
  { actual_source: 'template', fallback_reason: '端点未配置', outcome: 'rejected', content_acceptable: 'not_applicable', correction_status: 'unmemorable', failure_layer: '校正', failure_type: '语气不像', severity: 2 },
];
const rows = [start, ...variants.map(v => ({ ...makeTask(start.batch_id, 'T3', at), ...v })), makeBatchEnd(start.batch_id, at)];
rows.at(-1).recording_seconds = 60;
process.stdout.write(`${rows.map(row => JSON.stringify(row)).join('\n')}\n`);
