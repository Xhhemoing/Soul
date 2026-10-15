/**
 * Executable DESIGN MODEL, not a Soul runtime module or security boundary.
 * Synthetic metadata only. No I/O, database, tokenizer, HTTP, crypto or LLM.
 * All calls are atomic in this single-threaded model. A Rust integration must
 * provide the synchronization and trusted adapters specified in the design.
 */

const integer = (n, name, min = 0) => {
  if (!Number.isSafeInteger(n) || n < min) throw new TypeError(name);
};
const order = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
const strings = xs => Array.isArray(xs) && xs.every(x => typeof x === 'string' && x.trim().length > 0);
const originValue = value => {
  if (value === null) return;
  if (typeof value !== 'string') throw new TypeError('canonical origin');
  let url;
  try { url = new URL(value); } catch { throw new TypeError('canonical origin'); }
  if (!['http:', 'https:'].includes(url.protocol) || url.origin !== value) {
    throw new TypeError('canonical origin');
  }
};

/** Page ownership. Editing invalidates a preview without freeing an in-flight slot. */
export class DraftFlow {
  #live = true;
  #revision = 0;
  #serial = 0;
  #active = null;
  #preview = null;

  edit() { this.#revision += 1; this.#preview = null; }
  dispose() { this.#live = false; this.#preview = null; }
  begin(preview = undefined) {
    if (!this.#live || this.#active !== null) return null;
    if (preview !== undefined && preview !== this.#preview) return null;
    const ticket = Object.freeze({ serial: ++this.#serial, revision: this.#revision });
    this.#active = ticket;
    this.#preview = null;
    return ticket;
  }
  owns(ticket) {
    return this.#live && ticket !== null && this.#active === ticket
      && ticket.revision === this.#revision;
  }
  finish(ticket, prepared = undefined) {
    const accepted = this.owns(ticket);
    if (this.#active === ticket) this.#active = null;
    if (accepted && prepared !== undefined) this.#preview = prepared;
    return accepted;
  }
  // UI-only hide. Real discard IPC must be drained before a new preparation,
  // or gain an explicitly approved identity-scoped core API in a later change.
  abandon() {
    if (!this.#live || this.#active !== null) return false;
    this.#preview = null;
    this.#revision += 1;
    return true;
  }
}

/**
 * Proposed control-plane semantics. DISPATCH means permit issuance, not first
 * socket byte. Revocation cannot retract a previously issued permit or bytes.
 * In-flight cancelled work retains its physical capacity until settlement.
 */
export class OperationControl {
  #epoch = 0;
  #dataVersion = 0;
  #writing = false;
  #origin;
  #capacity;
  #operations = new Map();
  #used = new Set();
  #now = 0;

  constructor({ origin, capacity }) {
    integer(capacity, 'capacity', 1);
    originValue(origin);
    this.#origin = origin;
    this.#capacity = capacity;
  }
  #clock(now) {
    integer(now, 'monotonic time');
    if (now < this.#now) throw new RangeError('clock regressed');
    this.#now = now;
  }
  snapshot() {
    return Object.freeze({ epoch: this.#epoch, dataVersion: this.#dataVersion,
      origin: this.#origin, writing: this.#writing });
  }
  #valid(snapshot) {
    return !this.#writing && snapshot.writing === false
      && snapshot.epoch === this.#epoch && snapshot.dataVersion === this.#dataVersion
      && snapshot.origin === this.#origin;
  }
  configure(origin) {
    originValue(origin);
    this.#epoch += 1; this.#origin = origin;
    this.#sweepPrepared(this.#now);
  }
  revoke() { this.configure(null); }
  beginMutation() {
    if (this.#writing) throw new Error('single writer already active');
    this.#writing = true;
    this.#dataVersion += 1; // Failed writes conservatively invalidate too.
    this.#sweepPrepared(this.#now);
  }
  endMutation() {
    if (!this.#writing) throw new Error('no mutation');
    this.#writing = false;
  }
  prepare({ id, snapshot, deadline, now }) {
    this.#clock(now); integer(deadline, 'deadline');
    this.#sweepPrepared(now);
    if (typeof id !== 'string' || id.length === 0) throw new TypeError('id');
    if (this.#used.has(id)) return { ok: false, reason: 'duplicate_id' };
    if (!this.#valid(snapshot)) return { ok: false, reason: 'source_changed' };
    if (this.#origin === null) return { ok: false, reason: 'not_configured' };
    if (now >= deadline) return { ok: false, reason: 'expired' };
    if (this.#operations.size >= this.#capacity) return { ok: false, reason: 'busy' };
    this.#used.add(id);
    this.#operations.set(id, { snapshot: Object.freeze({ ...snapshot }), deadline,
      phase: 'prepared', cancelled: false });
    return { ok: true };
  }
  #reason(op, now) {
    if (op.cancelled) return 'cancelled';
    if (now >= op.deadline) return 'expired';
    if (op.snapshot.epoch !== this.#epoch || this.#origin === null) return 'revoked';
    if (!this.#valid(op.snapshot)) return 'source_changed';
    return null;
  }
  #sweepPrepared(now) {
    // Stale queued work has no physical worker. Dispatched work must settle.
    for (const [id, op] of this.#operations) {
      if (op.phase === 'prepared' && this.#reason(op, now) !== null) {
        this.#operations.delete(id);
      }
    }
  }
  dispatch(id, now) {
    this.#clock(now);
    const op = this.#operations.get(id);
    if (!op || op.phase !== 'prepared') return { ok: false, reason: 'not_prepared' };
    const reason = this.#reason(op, now);
    if (reason !== null) { this.#operations.delete(id); return { ok: false, reason }; }
    op.phase = 'dispatched'; // Single linearization point; one permit maximum.
    return { ok: true, permit: Object.freeze({ id, origin: op.snapshot.origin }) };
  }
  cancel(id) {
    const op = this.#operations.get(id);
    if (!op) return false;
    op.cancelled = true;
    if (op.phase === 'prepared') this.#operations.delete(id);
    return true;
  }
  complete(id, now) {
    this.#clock(now);
    const op = this.#operations.get(id);
    if (!op || op.phase !== 'dispatched') return { ok: false, reason: 'not_dispatched' };
    const reason = this.#reason(op, now);
    this.#operations.delete(id); // Physical worker has actually settled.
    return reason === null ? { ok: true } : { ok: false, reason };
  }
  get occupied() { return this.#operations.size; }
}

/**
 * Permission-first rank fusion experiment. Every row is a TRUSTED ADAPTER'S
 * decision descriptor, never raw imported JSON or a model-generated grant.
 * cost is caller-measured budget units, not an estimated token count.
 * Null/missing time does not become zero. Query time is supplied, never Date.now.
 */
export function selectContext({ rows, rankings, purpose, asOf, knownAt,
  limit, budget, perSource, rrfK }) {
  integer(limit, 'limit'); integer(budget, 'budget');
  integer(perSource, 'perSource', 1); integer(rrfK, 'rrfK', 1);
  integer(knownAt, 'knownAt');
  if (asOf !== null) integer(asOf, 'asOf');
  if (typeof purpose !== 'string' || purpose.length === 0) throw new TypeError('purpose');
  if (!Array.isArray(rankings) || !rankings.every(strings)) throw new TypeError('rankings');
  const retrieved = new Set(rankings.flat());
  const records = new Map();
  const excluded = [];
  const eligible = new Map();
  for (const row of rows) {
    if (typeof row.id !== 'string' || row.id.length === 0 || records.has(row.id)) {
      throw new TypeError('unique row id required');
    }
    records.set(row.id, row);
    integer(row.cost, 'cost', 1);
    let reason = null;
    if (row.active !== true || row.readable !== true) reason = 'unavailable';
    else if (row.granted !== true || !strings(row.purposes) || !row.purposes.includes(purpose)) reason = 'not_allowed';
    else if (!strings(row.citations) || !row.citations.length || row.evidenceValid !== true) reason = 'no_evidence';
    else if (!Number.isSafeInteger(row.recordedAt) || row.recordedAt > knownAt
      || row.recordedAt < 0) reason = 'unknown_or_future_record';
    else if (asOf !== null && (!Number.isSafeInteger(row.validFrom)
      || row.validFrom < 0 || row.validFrom > asOf
      || (row.validTo !== null && (!Number.isSafeInteger(row.validTo)
        || row.validTo <= asOf)))) reason = 'outside_or_unknown_time';
    else if (typeof row.source !== 'string' || row.source.length === 0) reason = 'unknown_source';
    else if (!retrieved.has(row.id)) reason = 'not_retrieved';
    if (reason) excluded.push({ id: row.id, reason });
    else eligible.set(row.id, row);
  }
  // Contradictions are considered over the union of eligible retrieval hits,
  // not merely the first K. Unrelated records cannot force clarification.
  // Keys/values must be normalized by a trusted layer.
  const groups = new Map();
  for (const row of eligible.values()) {
    if (row.claimKey !== undefined) {
      if (typeof row.claimKey !== 'string' || typeof row.claimValue !== 'string') {
        throw new TypeError('structured claim key/value');
      }
      const group = groups.get(row.claimKey) ?? [];
      group.push(row); groups.set(row.claimKey, group);
    }
  }
  const conflicts = [];
  for (const [key, group] of groups) {
    if (new Set(group.map(row => row.claimValue)).size > 1) {
      conflicts.push(key);
      for (const row of group) {
        eligible.delete(row.id); excluded.push({ id: row.id, reason: 'conflict' });
      }
    }
  }
  const scores = new Map();
  for (const list of rankings) {
    const seen = new Set(); let rank = 0;
    for (const id of list) {
      if (!eligible.has(id) || seen.has(id)) continue;
      seen.add(id); rank += 1;
      scores.set(id, (scores.get(id) ?? 0) + 1 / (rrfK + rank));
    }
  }
  const ranked = [...scores].sort((a, b) => b[1] - a[1] || order(a[0], b[0]));
  const selected = []; let used = 0;
  const sourceCounts = new Map();
  for (const [id] of ranked) {
    const row = eligible.get(id);
    const count = sourceCounts.get(row.source) ?? 0;
    if (selected.length >= limit) { excluded.push({ id, reason: 'limit' }); continue; }
    if (count >= perSource) { excluded.push({ id, reason: 'source_cap' }); continue; }
    if (used + row.cost > budget) { excluded.push({ id, reason: 'budget' }); continue; }
    selected.push({ id, citations: [...row.citations], cost: row.cost });
    used += row.cost; sourceCounts.set(row.source, count + 1);
  }
  return { selected, used, conflicts: conflicts.sort(order),
    excluded: excluded.sort((a, b) => order(a.id, b.id)),
    disposition: conflicts.length ? 'clarify' : selected.length ? 'context' : 'abstain' };
}

/** Existing correction-priority principle only; not an A0/A1 reimplementation. */
export function resolveField({ locked, userValue, machineValue, evidenceValid }) {
  if (locked === true) return userValue == null
    ? { state: 'unknown', source: 'user_lock_without_value' }
    : { state: 'known', source: 'user', value: userValue };
  if (evidenceValid !== true || machineValue == null) return { state: 'unknown', source: 'none' };
  return { state: 'known', source: 'machine', value: machineValue };
}
