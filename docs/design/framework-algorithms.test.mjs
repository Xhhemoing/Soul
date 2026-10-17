import test from 'node:test';
import assert from 'node:assert/strict';
import { DraftFlow, OperationControl, selectContext, resolveField } from './framework-algorithms.mjs';

const control = (capacity = 1) => new OperationControl({ origin: 'https://model.example', capacity });
const prepare = (c, id = 'a', extra = {}) => c.prepare({ id, snapshot: c.snapshot(), now: 0, deadline: 10, ...extra });
const row = (id, extra = {}) => ({ id, active: true, readable: true, granted: true,
  purposes: ['draft'], evidenceValid: true, citations: [`e-${id}`], source: id,
  recordedAt: 1, validFrom: 1, validTo: null, cost: 2, ...extra });
const select = (rows, rankings, extra = {}) => selectContext({ rows, rankings,
  purpose: 'draft', asOf: 5, knownAt: 5, limit: 3, budget: 10, perSource: 1, rrfK: 60, ...extra });
const ids = r => r.selected.map(x => x.id);

test('edit A→B→A does not resurrect an old preparation', () => {
  const f = new DraftFlow(), ticket = f.begin();
  f.edit(); f.edit();
  assert.equal(f.finish(ticket, {}), false);
  assert.notEqual(f.begin(), null); // Stale completion releases its own slot.
});
test('same-tick duplicate admission is refused before a render', () => {
  const f = new DraftFlow(); assert.ok(f.begin()); assert.equal(f.begin(), null);
});
test('editing a visible plan invalidates approval and original-body exemption', () => {
  const f = new DraftFlow(), p = {};
  assert.equal(f.finish(f.begin(), p), true);
  f.edit(); assert.equal(f.begin(p), null);
});
test('old completion cannot free or replace a newer active operation', () => {
  const f = new DraftFlow(), old = f.begin(); f.finish(old);
  const current = f.begin(); assert.equal(f.finish(old, {}), false);
  assert.equal(f.begin(), null); assert.equal(f.finish(current), true);
});
test('unmount drops results without asserting core cancellation', () => {
  const f = new DraftFlow(), t = f.begin(); f.dispose();
  assert.equal(f.finish(t, {}), false); assert.equal(f.begin(), null);
});
test('abandon never races a still pending UI request in the model', () => {
  const f = new DraftFlow(), t = f.begin(); assert.equal(f.abandon(), false);
  const p = {}; f.finish(t, p); assert.equal(f.abandon(), true); assert.equal(f.begin(p), null);
});
test('revocation before dispatch issues no permit', () => {
  const c = control(); prepare(c); c.revoke();
  assert.deepEqual(c.dispatch('a', 1), { ok: false, reason: 'not_prepared' });
});
test('revocation after dispatch prevents acceptance, not already-issued traffic', () => {
  const c = control(); prepare(c); assert.equal(c.dispatch('a', 1).ok, true);
  c.revoke(); assert.deepEqual(c.complete('a', 2), { ok: false, reason: 'revoked' });
});
test('endpoint ABA does not make an old approval current again', () => {
  const c = control(); prepare(c); c.configure('https://other.example');
  c.configure('https://model.example'); assert.equal(c.dispatch('a', 1).ok, false);
});
test('a snapshot read before a write cannot register as a fresh preparation', () => {
  const c = control(), snapshot = c.snapshot(); c.beginMutation(); c.endMutation();
  assert.deepEqual(prepare(c, 'a', { snapshot }), { ok: false, reason: 'source_changed' });
});
test('profile correction or same-id memory replacement rejects old result', () => {
  const c = control(); prepare(c); c.dispatch('a', 1); c.beginMutation(); c.endMutation();
  assert.deepEqual(c.complete('a', 2), { ok: false, reason: 'source_changed' });
});
test('write-in-progress prevents prepare and completion', () => {
  const c = control(2); prepare(c); c.dispatch('a', 1); c.beginMutation();
  assert.equal(prepare(c, 'b', { now: 1 }).ok, false);
  assert.equal(c.complete('a', 2).ok, false); c.endMutation();
});
test('rolled-back writes conservatively invalidate old snapshots', () => {
  const c = control(); prepare(c); c.beginMutation(); c.endMutation();
  assert.equal(c.dispatch('a', 1).ok, false);
});
test('one preparation issues at most one permit and accepts at most once', () => {
  const c = control(); prepare(c); assert.equal(c.dispatch('a', 1).ok, true);
  assert.equal(c.dispatch('a', 1).ok, false); assert.equal(c.complete('a', 2).ok, true);
  assert.equal(c.complete('a', 2).ok, false);
  assert.equal(prepare(c, 'a', { now: 2 }).reason, 'duplicate_id');
});
test('deadline equality is expired and regressing monotonic clock is rejected', () => {
  const c = control(); prepare(c); assert.equal(c.dispatch('a', 10).reason, 'expired');
  assert.throws(() => prepare(c, 'b'), /clock regressed/);
});
test('cancelled blocking work retains physical capacity until it actually settles', () => {
  const c = control(); prepare(c); c.dispatch('a', 1); c.cancel('a');
  assert.equal(c.occupied, 1); assert.equal(prepare(c, 'b', { now: 1 }).reason, 'busy');
  assert.equal(c.complete('a', 2).reason, 'cancelled'); assert.equal(c.occupied, 0);
  assert.equal(prepare(c, 'b', { now: 2 }).ok, true);
});
test('queued cancellation frees capacity without consuming a dispatch', () => {
  const c = control(); prepare(c); c.cancel('a');
  assert.equal(c.dispatch('a', 1).ok, false); assert.equal(prepare(c, 'b', { now: 1 }).ok, true);
});
test('revoked queued work cannot permanently starve a new request', () => {
  const c = control(); prepare(c); c.revoke(); c.configure('https://model.example');
  assert.equal(c.occupied, 0); assert.equal(prepare(c, 'b', { now: 6 }).ok, true);
});
test('expired queued work is swept before checking admission capacity', () => {
  const c = control(); prepare(c, 'a', { deadline: 5 });
  assert.equal(prepare(c, 'b', { now: 6 }).ok, true);
  assert.equal(c.dispatch('a', 6).ok, false);
});
test('source invalidation frees only queued work, preserving dispatched physical slots', () => {
  const c = control(2); prepare(c, 'a'); prepare(c, 'b'); c.dispatch('a', 1);
  c.beginMutation(); c.endMutation(); assert.equal(c.occupied, 1);
  assert.equal(prepare(c, 'c', { now: 1 }).ok, true);
  assert.equal(c.complete('a', 2).ok, false);
});
test('invalid or noncanonical endpoint representation never issues a model permit', () => {
  for (const origin of [undefined, '', false, 'model.example', 'https://model.example/',
    'https://user:pass@model.example', 'ftp://model.example']) {
    assert.throws(() => new OperationControl({ origin, capacity: 1 }), /origin/);
  }
  const c = new OperationControl({ origin: null, capacity: 1 });
  assert.equal(prepare(c).reason, 'not_configured');
});
test('all 120 distinct event orders preserve revocation and single-use invariants', () => {
  const permutations = xs => xs.length ? xs.flatMap((x, i) =>
    permutations(xs.filter((_, j) => j !== i)).map(ys => [x, ...ys])) : [[]];
  let checked = 0;
  for (const actions of permutations(['dispatch', 'revoke', 'cancel', 'complete', 'write'])) {
    const c = control(); prepare(c); let revoked = false, cancelled = false, changed = false;
    let permits = 0, accepts = 0, dispatched = false;
    for (const action of actions) {
      if (action === 'revoke') { c.revoke(); revoked = true; }
      if (action === 'cancel') { c.cancel('a'); cancelled = true; }
      if (action === 'write') { c.beginMutation(); c.endMutation(); changed = true; }
      if (action === 'dispatch') {
        const result = c.dispatch('a', 1);
        if (result.ok) { assert.equal(revoked || cancelled || changed, false); permits++; dispatched = true; }
      }
      if (action === 'complete') {
        const result = c.complete('a', 1);
        if (result.ok) { assert.equal(dispatched && !revoked && !cancelled && !changed, true); accepts++; }
      }
    }
    assert.ok(permits <= 1 && accepts <= permits); checked++;
  }
  assert.equal(checked, 120);
});
test('top-ranked denied/forgotten/dangling evidence cannot enter context', () => {
  const rows = [row('denied', { granted: false }), row('forgotten', { active: false }),
    row('dangling', { evidenceValid: false }), row('ok')];
  assert.deepEqual(ids(select(rows, [rows.map(r => r.id)])), ['ok']);
});
test('research permission does not confer draft permission', () => {
  assert.equal(select([row('a', { purposes: ['research'] })], [['a']]).disposition, 'abstain');
});
test('malformed permission and citation fields cannot silently pass trusted adapters', () => {
  for (const patch of [{ purposes: 'redraft' }, { purposes: ['draft', null] },
    { purposes: null }, { citations: 'not-an-evidence-array' }, { citations: [''] },
    { citations: ['valid', 7] }]) {
    assert.deepEqual(ids(select([row('a', patch)], [['a']])), []);
  }
});
test('unretrieved conflicts do not block an unrelated query or fabricate hits', () => {
  const rows = [row('hit'), row('a', { claimKey: 'city', claimValue: 'X' }),
    row('b', { claimKey: 'city', claimValue: 'Y' })];
  assert.equal(select(rows, [['hit']]).disposition, 'context');
  assert.equal(select(rows, []).disposition, 'abstain');
});
test('unknown metadata fails closed rather than turning into a zero/default grant', () => {
  for (const patch of [{ granted: undefined }, { readable: undefined }, { evidenceValid: undefined },
    { recordedAt: null }, { validFrom: null }, { validTo: undefined }, { citations: [] }]) {
    assert.deepEqual(ids(select([row('a', patch)], [['a']])), []);
  }
});
test('time intervals are half-open; knowledge cutoff also applies', () => {
  assert.deepEqual(ids(select([row('old', { validTo: 5 }), row('new', { validFrom: 5 }),
    row('future', { recordedAt: 6 })], [['old', 'future', 'new']])), ['new']);
});
test('time-free retrieval may include unknown event time but never unknown recorded time', () => {
  assert.deepEqual(ids(select([row('a', { validFrom: null }), row('b', { recordedAt: null })],
    [['a', 'b']], { asOf: null })), ['a']);
});
test('contradictory claims outside top-K still force clarification', () => {
  const result = select([row('a', { claimKey: 'city', claimValue: 'X' }),
    row('b', { claimKey: 'city', claimValue: 'Y' })], [['a', 'b']], { limit: 1 });
  assert.equal(result.disposition, 'clarify'); assert.deepEqual(ids(result), []);
});
test('nonoverlapping historical updates are not contradictions at a specified time', () => {
  const rows = [row('a', { claimKey: 'city', claimValue: 'X', validTo: 5 }),
    row('b', { claimKey: 'city', claimValue: 'Y', validFrom: 5 })];
  assert.deepEqual(ids(select(rows, [['a', 'b']])), ['b']);
});
test('duplicate ranks do not add votes; unauthorized insertions do not change eligible ranks', () => {
  const rows = [row('a'), row('b'), row('x', { granted: false })];
  const expected = ids(select(rows, [['a', 'b'], ['b', 'a']]));
  assert.deepEqual(ids(select(rows, [['x', 'a', 'a', 'b'], ['b', 'x', 'b', 'a']])), expected);
});
test('row order does not change deterministic tie-breaking', () => {
  const rows = [row('z'), row('a')];
  assert.deepEqual(ids(select(rows, [['z', 'a'], ['a', 'z']])), ['a', 'z']);
  assert.deepEqual(ids(select(rows.reverse(), [['z', 'a'], ['a', 'z']])), ['a', 'z']);
});
test('budget skips an oversized hit and source cap avoids duplicate-source domination', () => {
  const rows = [row('big', { cost: 99 }), row('a', { source: 'same' }),
    row('b', { source: 'same' }), row('c')];
  const result = select(rows, [['big', 'a', 'b', 'c']], { budget: 4 });
  assert.deepEqual(ids(result), ['a', 'c']); assert.equal(result.used, 4);
});
test('no hits abstains and zero budget never squeezes in an unmeasured item', () => {
  assert.equal(select([row('a')], []).disposition, 'abstain');
  assert.deepEqual(ids(select([row('a')], [['a']], { budget: 0 })), []);
  assert.throws(() => select([row('a', { cost: 0 })], [['a']]), /cost/);
});
test('duplicate row identities and invalid tuning parameters are rejected', () => {
  assert.throws(() => select([row('a'), row('a')], [['a']]), /unique/);
  assert.throws(() => select([row('a')], [['a']], { rrfK: 0 }), /rrfK/);
});
test('user lock dominates inference, including a missing locked value', () => {
  assert.deepEqual(resolveField({ locked: true, userValue: 'warm', machineValue: 'cool', evidenceValid: true }),
    { state: 'known', source: 'user', value: 'warm' });
  assert.equal(resolveField({ locked: true, userValue: null, machineValue: 'cool', evidenceValid: true }).state, 'unknown');
  assert.equal(resolveField({ locked: false, machineValue: 'cool', evidenceValid: false }).state, 'unknown');
});
