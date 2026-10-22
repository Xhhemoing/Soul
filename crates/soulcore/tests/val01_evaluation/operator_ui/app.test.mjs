import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';

const timerSource = await readFile(new URL('./timer.js', import.meta.url), 'utf8');
const { createPairTimer } = await import(`data:text/javascript;base64,${Buffer.from(timerSource).toString('base64')}`);
const appSource = await readFile(new URL('./app.js', import.meta.url), 'utf8');

// A minimal DOM boundary double: run the actual app handlers and fetch payload.
// Browser layout, keyboard behavior and accessibility remain browser checks.
class Element {
  constructor(tag) {
    this.tagName = tag;
    this.children = [];
    this.events = {};
    this.value = '';
    this.disabled = false;
    this.textContent = '';
  }
  append(...items) { this.children.push(...items); }
  replaceChildren(...items) { this.children = items; }
  setAttribute(key, value) { this[key] = value; }
  addEventListener(name, fn) { (this.events[name] ??= []).push(fn); }
  focus() {}
  click() { if (!this.disabled) this.dispatch('click'); }
  dispatch(name) { for (const fn of this.events[name] ?? []) fn({}); }
}
function harness(task = 't1') {
  const root = new Element('main');
  for (const [tag, id] of [['section', 'workspace'], ['p', 'status'], ['p', 'deadline'], ['footer', 'session-actions'], ['button', 'stop'], ['button', 'withdraw']]) {
    const el = new Element(tag); el.id = id; root.append(el);
  }
  const all = () => {
    const found = [];
    function walk(el) { found.push(el); for (const child of el.children) walk(child); }
    walk(root); return found;
  };
  const byId = (id) => all().find((el) => el.id === id);
  const document = {
    querySelector: (selector) => byId(selector.slice(1)),
    querySelectorAll: (selector) => all().filter((el) => selector.split(',').map((s) => s.trim()).includes(el.tagName)),
    createElement: (tag) => new Element(tag),
    createTextNode: (text) => Object.assign(new Element('#text'), { textContent: text }),
    addEventListener() {},
  };
  const requests = [];
  const timeouts = new Map();
  let timeoutId = 0;
  const responses = [
    { stage: 'welcome', remaining_seconds: 60 },
    { stage: 'pair', card: 0, label: 'S1', task, instruction: '合成任务',
      left_text: '左模板（这里写你要说的内容）', right_text: '右模板（这里写你要说的内容）', remaining_seconds: 59 },
    { stage: 'recorded', card: 0, remaining_seconds: 58 },
  ];
  vm.runInNewContext(appSource.replace(/^import[^\n]+\n/, ''), {
    createPairTimer, document, window: { addEventListener() {} },
    location: { pathname: '/00000000-0000-4000-8000-000000000001/' },
    performance: { now: () => 0 }, Option: function (text, value) {
      return Object.assign(new Element('option'), { textContent: text, value });
    },
    fetch: async (url, options) => {
      requests.push({ url, ...options });
      const response = await responses.shift();
      return { ok: true, json: async () => response };
    },
    setTimeout(fn, delay) { timeouts.set(++timeoutId, { fn, delay }); return timeoutId; },
    clearTimeout(id) { timeouts.delete(id); },
  });
  return { byId, requests, all, responses, timeouts, click(text) {
    const found = all().find((el) => el.tagName === 'button' && el.textContent === text);
    assert.ok(found, `button exists: ${text}`); found.click();
  } };
}
const settle = () => new Promise((resolve) => setImmediate(resolve));

test('actual UI requires explicit feedback and sends metadata only after editing; locked prose is cleared', async () => {
  const ui = harness();
  await settle();
  ui.click('开始五卡合成演练');
  await settle();
  for (const el of ui.all().filter((el) => ['select', 'input'].includes(el.tagName))) {
    assert.equal(el.value, '', 'no implicit feedback value');
  }
  assert.equal(ui.byId('text-0').disabled, true);
  assert.equal(ui.byId('unsupported-0'), undefined, 'T1 must not offer an inapplicable rating');
  assert.equal(ui.byId('errors-0'), undefined, 'T1 must not offer an inapplicable fact count');
  ui.click('锁定本卡反馈');
  await settle();
  assert.equal(ui.requests.length, 2, 'unrated initial form must not submit');
  ui.all().find((el) => el['aria-label'] === '左侧开始').click();
  const editor = ui.byId('text-0');
  assert.equal(editor.disabled, false);
  editor.value = 'SYNTHETIC_EDIT_SENTINEL_DO_NOT_TRANSMIT';
  editor.dispatch('input');
  ui.all().find((el) => el['aria-label'] === '左侧结束计时').click();
  assert.equal(editor.disabled, true);
  for (const [id, value] of Object.entries({
    'completed-0': 'yes', 'first-0': 'no', 'help-0': '0', 'safety-0': 'no',
    'completed-1': 'no', 'help-1': '0', 'safety-1': 'no',
    preference: 'left', blind: 'intact',
  })) ui.byId(id).value = value;
  ui.byId('slot-0').checked = true;
  ui.click('锁定本卡反馈');
  await settle();
  assert.equal(ui.requests.length, 3);
  const submitted = JSON.parse(ui.requests[2].body);
  assert.deepEqual(Object.keys(submitted).sort(), ['action', 'card', 'feedback']);
  assert.equal(submitted.action, 'submit');
  assert.equal(submitted.feedback.sides[0].completed, true);
  assert.equal(submitted.feedback.sides[0].first_pass, false);
  assert.equal(submitted.feedback.sides[0].edit_seconds, 0);
  assert.equal(submitted.feedback.sides[0].unsupported_claim, null, 'T1 unsupported is not applicable');
  assert.equal(submitted.feedback.sides[0].fact_errors, null, 'T1 fact count is not applicable');
  assert.equal(submitted.feedback.sides[1].edit_seconds, null);
  assert.equal(submitted.feedback.sides[1].first_pass, null, 'unrated first pass must remain missing');
  assert.equal(ui.requests.some((r) => /SYNTHETIC_EDIT_SENTINEL|左模板|右模板/.test(r.body)), false);
  assert.equal(editor.value, '', 'detached editor must also be erased');
  assert.equal(ui.byId('text-0'), undefined);
});

test('T2 offers unsupported rating but no fact count; an unselected first pass remains null', async () => {
  const ui = harness('t2');
  await settle();
  ui.click('开始五卡合成演练');
  await settle();
  assert.ok(ui.byId('unsupported-0'));
  assert.equal(ui.byId('errors-0'), undefined);
  for (const [id, value] of Object.entries({
    'completed-0': 'yes', 'first-0': 'yes', 'help-0': '0', 'safety-0': 'no', 'unsupported-0': 'no',
    'completed-1': 'no', 'help-1': '0', 'safety-1': 'no',
    preference: 'left', blind: 'intact',
  })) ui.byId(id).value = value;
  ui.click('锁定本卡反馈');
  await settle();
  assert.equal(ui.requests.length, 3);
  const feedback = JSON.parse(ui.requests[2].body).feedback;
  assert.equal(feedback.sides[0].first_pass, true);
  assert.equal(feedback.sides[0].unsupported_claim, false);
  assert.equal(feedback.sides[0].fact_errors, null);
  assert.equal(feedback.sides[1].first_pass, null);
});

test('deadline clears prose and invalidates a pending response so it cannot restore the card', async () => {
  const ui = harness();
  await settle();
  ui.click('开始五卡合成演练');
  await settle();
  const editor = ui.byId('text-0');
  assert.ok(editor.value.includes('左模板'));
  for (const [id, value] of Object.entries({
    'completed-0': 'no', 'help-0': '0', 'safety-0': 'no',
    'completed-1': 'no', 'help-1': '0', 'safety-1': 'no',
    preference: 'unrated', blind: 'intact',
  })) ui.byId(id).value = value;
  let release;
  ui.responses[0] = new Promise((resolve) => { release = resolve; });
  ui.click('锁定本卡反馈');
  assert.equal(ui.requests.length, 3);
  assert.equal(ui.timeouts.size, 1, 'response replaces the earlier deadline');
  const deadline = [...ui.timeouts.values()][0];
  assert.equal(deadline.delay, 59000);
  deadline.fn();
  assert.equal(editor.value, '');
  assert.equal(ui.byId('text-0'), undefined);
  assert.equal(ui.byId('session-actions').hidden, true);
  release({ stage: 'pair', card: 0, label: 'S1', task: 't1',
    instruction: '旧响应', left_text: 'LATE_PROSE', right_text: 'LATE_PROSE', remaining_seconds: 50 });
  await settle();
  assert.equal(ui.byId('text-0'), undefined);
  assert.equal(ui.all().some((el) => el.value === 'LATE_PROSE'), false);
  assert.match(ui.byId('stage-title').textContent, /期限已到/);
  assert.equal(ui.timeouts.size, 0);
});
