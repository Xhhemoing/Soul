import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
const source = await readFile(new URL('./timer.js', import.meta.url), 'utf8');
const { createPairTimer } = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);

function clock(recovered = false) {
  let now = 0;
  return { timer: createPairTimer(() => now, recovered), advance(ms) { now += ms; } };
}
test('unfinished timing is missing, measured subsecond completion remains zero', () => {
  const { timer, advance } = clock();
  assert.equal(timer.snapshot(0).edit_seconds, null);
  timer.start(0); advance(400);
  assert.equal(timer.snapshot(0).edit_seconds, null);
  timer.finish(0);
  assert.equal(timer.snapshot(0).edit_seconds, 0);
  assert.equal(timer.snapshot(0).pause_seconds, 0);
});
test('active and paused intervals are counted separately across resume', () => {
  const { timer, advance } = clock();
  timer.start(0); advance(1800); timer.pause(0);
  advance(2400); timer.resume(0); advance(1700); timer.finish(0);
  assert.equal(timer.snapshot(0).edit_seconds, 3);
  assert.equal(timer.snapshot(0).pause_seconds, 2);
});
test('only one side can run; refused start leaves the other untouched', () => {
  const { timer } = clock();
  timer.start(0);
  assert.throws(() => timer.start(1));
  assert.equal(timer.snapshot(1).status, 'idle');
  timer.pause(0); timer.start(1);
  assert.throws(() => timer.resume(0));
});
test('focus loss invalidates both sides and pauses active timing without finishing', () => {
  const { timer, advance } = clock();
  timer.start(0); advance(2400); timer.invalidate();
  assert.equal(timer.snapshot(0).status, 'paused');
  timer.resume(0); advance(2000); timer.finish(0);
  assert.equal(timer.snapshot(0).edit_seconds, null);
  timer.start(1); timer.finish(1);
  assert.equal(timer.snapshot(1).pause_seconds, null);
});
test('a recovered card can never recover reliable timing by restarting', () => {
  const { timer } = clock(true);
  timer.start(0); timer.finish(0);
  assert.equal(timer.snapshot(0).edit_seconds, null);
  assert.throws(() => timer.start(0));
});
test('finishing an unstarted side does not fabricate a sample', () => {
  const { timer } = clock();
  assert.throws(() => timer.finish(1));
  assert.equal(timer.snapshot(1).edit_seconds, null);
});
