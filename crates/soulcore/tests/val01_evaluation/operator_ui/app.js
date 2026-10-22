import { createPairTimer } from './timer.js';

const workspace = document.querySelector('#workspace');
const notice = document.querySelector('#status');
const sessionActions = document.querySelector('#session-actions');
const token = location.pathname.split('/').filter(Boolean)[0];
const BODY_SLOT = '（这里写你要说的内容）'; // soul-draft/src/template.rs
let stage = 'loading';
let card = null;
let task = null;
let timer = null;
let panels = [];
let busy = false;
let sequence = 0;
let closed = false;
let report = null;
let deadlineTimer = null;

function clearDeadline() {
  if (deadlineTimer !== null) clearTimeout(deadlineTimer);
  deadlineTimer = null;
  document.querySelector('#deadline').textContent = '';
}
function expire() {
  clearDeadline();
  ++sequence;
  closed = true;
  busy = true;
  clearText();
  workspace.replaceChildren();
  sessionActions.hidden = true;
  title('本次演练期限已到');
  announce('正文已清除，编辑与评分已停止。服务在终端输出纯元数据，未提交反馈不计入。');
}
function armDeadline(seconds) {
  clearDeadline();
  if (!Number.isSafeInteger(seconds) || seconds < 0) throw new Error('invalid deadline');
  if (seconds === 0) { expire(); return false; }
  document.querySelector('#deadline').textContent = `服务最近确认的剩余期限：${seconds} 秒。到期自动清除正文，未提交反馈不计入。`;
  deadlineTimer = setTimeout(expire, seconds * 1000);
  return true;
}

function node(tag, text, className) {
  const el = document.createElement(tag);
  if (text !== undefined) el.textContent = text;
  if (className) el.className = className;
  return el;
}
function announce(text) { notice.textContent = text; }
function button(text, action, className) {
  const el = node('button', text, className);
  el.type = 'button';
  el.addEventListener('click', action);
  return el;
}
function title(text) {
  const el = node('h2', text);
  el.id = 'stage-title';
  el.tabIndex = -1;
  workspace.append(el);
  el.focus();
}
function selectField(parent, id, label, options) {
  const wrap = node('label', label);
  wrap.htmlFor = id;
  const input = node('select');
  input.id = id;
  input.append(new Option('请选择', ''));
  for (const [value, text] of options) input.append(new Option(text, value));
  wrap.append(input); parent.append(wrap);
  return input;
}
function numberField(parent, id, label) {
  const wrap = node('label', label);
  wrap.htmlFor = id;
  const input = node('input');
  Object.assign(input, { id, type: 'number', min: '0', max: '4294967295', step: '1', value: '' });
  wrap.append(input); parent.append(wrap);
  return input;
}
function clearText() {
  for (const el of document.querySelectorAll('textarea')) el.value = '';
  panels = [];
  timer = null;
}
function updateTimers() {
  if (!timer) return;
  const active = [0, 1].some((i) => timer.snapshot(i).status === 'active');
  panels.forEach((p, i) => {
    const s = timer.snapshot(i);
    p.editor.disabled = busy || s.status !== 'active';
    p.start.disabled = busy || active || s.status !== 'idle';
    p.pause.disabled = busy || s.status !== 'active';
    p.resume.disabled = busy || active || s.status !== 'paused';
    p.finish.disabled = busy || !['active', 'paused'].includes(s.status);
    const names = { idle: '未开始', active: '计时中', paused: '已暂停', finished: '已结束' };
    p.timing.textContent = `${names[s.status]} · ${s.reliable
      ? (s.status === 'finished' ? `编辑 ${s.edit_seconds} 秒 / 暂停 ${s.pause_seconds} 秒` : '结束计时后才有有效测量')
      : '计时不可靠，本侧时长将记为缺测'}`;
  });
}
function setBusy(value) {
  busy = value;
  for (const el of document.querySelectorAll('button, input, select, textarea')) el.disabled = value;
  updateTimers();
}
async function request(payload, restored = false, terminating = false) {
  if (busy || closed) return;
  setBusy(true);
  const op = ++sequence;
  if (terminating) {
    clearDeadline();
    clearText();
    workspace.replaceChildren();
    title('正在结束演练');
    closed = true;
  }
  try {
    const response = await fetch('api', {
      method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Soul-Operator': token },
      body: JSON.stringify(payload), cache: 'no-store', credentials: 'omit',
    });
    if (op !== sequence) return;
    if (!response.ok) throw new Error('request');
    const data = await response.json();
    if (op !== sequence) return;
    if (data.stage === 'finished') clearDeadline();
    else if (!armDeadline(data.remaining_seconds)) return;
    render(data, restored);
  } catch {
    if (op !== sequence) return;
    // An ambiguous submit/next result must never silently restore editable old prose.
    clearText();
    workspace.replaceChildren();
    title('连接或操作未完成');
    announce('未能确认服务状态。正文已清除，计时不再有效；请刷新页面读取服务状态。');
    closed = true;
    sessionActions.hidden = true;
  } finally {
    if (op === sequence) setBusy(false);
  }
}
function timingAction(i, action) {
  try {
    timer[action](i);
    updateTimers();
    if (action === 'start' || action === 'resume') panels[i].editor.focus();
  } catch (error) { announce(error.message); }
}
function sidePanel(i, text) {
  const name = i === 0 ? '左侧' : '右侧';
  const panel = node('section', undefined, 'candidate');
  panel.append(node('h3', `${name}候选`));
  const editorLabel = node('label', `${name}编辑正文（开始计时后可编辑）`);
  editorLabel.htmlFor = `text-${i}`;
  const editor = node('textarea');
  editor.id = `text-${i}`;
  editor.value = text;
  editor.spellcheck = false;
  editor.autocomplete = 'off';
  editor.disabled = true;
  editorLabel.append(editor); panel.append(editorLabel);
  const p = { editor, edited: false };
  editor.addEventListener('input', () => {
    p.edited = true;
    if (p.first.value === 'yes') p.first.value = '';
  });
  const controls = node('div', undefined, 'timer-actions');
  for (const [action, label] of [['start', '开始'], ['pause', '暂停'], ['resume', '恢复'], ['finish', '结束计时']]) {
    p[action] = button(label, () => timingAction(i, action), 'secondary');
    p[action].setAttribute('aria-label', `${name}${label}`);
    controls.append(p[action]);
  }
  panel.append(controls);
  p.timing = node('p', '', 'timing');
  p.timing.setAttribute('aria-live', 'polite');
  panel.append(p.timing);
  panel.append(button('将本侧计时标为不可靠', () => { timer.invalidate(i); updateTimers(); }, 'text-button'));
  const fields = node('div', undefined, 'fields');
  p.completed = selectField(fields, `completed-${i}`, `${name}是否完成`, [['yes', '已完成'], ['no', '未完成 / 不评分']]);
  p.first = selectField(fields, `first-${i}`, `${name}是否无需编辑就接受首稿`, [['yes', '是，明确接受原首稿'], ['no', '否']]);
  p.help = numberField(fields, `help-${i}`, `${name}帮助次数（没有求助请明确填 0）`);
  p.unsupported = task === 't1' ? null : selectField(fields, `unsupported-${i}`, `${name}是否出现无依据事实`, [['yes', '是'], ['no', '否'], ['unrated', '未评分']]);
  p.errors = task === 't3' ? numberField(fields, `errors-${i}`, `${name}事实错误数（T3 完成时必填）`) : null;
  p.safety = selectField(fields, `safety-${i}`, `${name}安全问题是否导致停止`, [['yes', '是'], ['no', '否']]);
  const slotWrap = node('label', undefined, 'check');
  p.slot = node('input'); p.slot.type = 'checkbox';
  p.slot.id = `slot-${i}`; slotWrap.htmlFor = p.slot.id;
  slotWrap.append(p.slot, document.createTextNode(`${name}已补全正文槽（T1 完成必选）`));
  if (task === 't1') {
    fields.append(node('p', 'T1 必须编辑并补全正文槽；补槽后不能计为首稿接受。', 'hint'), slotWrap);
  }
  panel.append(fields);
  panels.push(p);
  return panel;
}
function requireChoice(input, label) {
  if (!input.value) { input.focus(); throw new Error(`请明确选择：${label}`); }
  return input.value;
}
function count(input, label, required) {
  if (input.value === '' && !required) return null;
  const value = Number(input.value);
  if (input.value === '' || !Number.isInteger(value) || value < 0 || value > 4294967295) {
    input.focus(); throw new Error(`请填写有效的非负整数：${label}`);
  }
  return value;
}
function feedbackSide(p, i) {
  const completed = requireChoice(p.completed, '任务完成情况') === 'yes';
  const safety = requireChoice(p.safety, '安全停止') === 'yes';
  const help = count(p.help, '求助次数', true);
  let first = p.first.value === 'yes' ? true : p.first.value === 'no' ? false : null;
  if (completed) first = requireChoice(p.first, '首稿接受') === 'yes';
  if (first && (p.edited || task === 't1')) throw new Error('编辑过的文本和 T1 补槽均不能记为首稿接受。');
  const unsupported = task === 't1' ? null : p.unsupported.value === 'yes' ? true : p.unsupported.value === 'no' ? false : null;
  const errors = task === 't3' ? count(p.errors, '事实错误数', completed) : null;
  if (completed && task === 't1' && (!p.edited || !p.slot.checked || !p.editor.value.trim() || p.editor.value.includes(BODY_SLOT))) {
    throw new Error('T1 完成前须编辑文本、移除正文槽并明确勾选已补槽。');
  }
  if (completed && task === 't2' && unsupported !== false) throw new Error('T2 完成须明确确认没有无依据事实。');
  if (completed && task === 't3' && unsupported === null) throw new Error('T3 完成须明确选择是否出现无依据事实。');
  if (completed && safety) throw new Error('安全停止的候选不能同时标为已完成。');
  const measured = timer.snapshot(i);
  return { completed, first_pass: first, edit_seconds: measured.edit_seconds,
    pause_seconds: measured.pause_seconds, help_count: help, unsupported_claim: unsupported,
    fact_errors: errors, safety_stop: safety, t1_slot_filled: task === 't1' && p.slot.checked };
}
function render(data, restored) {
  clearText();
  workspace.replaceChildren();
  stage = data.stage;
  card = data.card ?? null;
  task = data.task ?? null;
  sessionActions.hidden = stage === 'finished';
  if (stage === 'welcome') {
    title('准备开始');
    workspace.append(node('p', '每侧单独计时，暂停后可切换另一侧。切换窗口、刷新或隐藏页面会使时长缺测。评分不预填，锁定反馈后才可揭盲。'));
    workspace.append(button('开始五卡合成演练', () => request({ action: 'start' })));
    announce('尚未生成反馈。');
  } else if (stage === 'pair') {
    title(`${data.label} · 第 ${card + 1} / 5 张`);
    workspace.append(node('p', data.instruction, 'instruction'));
    timer = createPairTimer(() => performance.now(), restored);
    const pair = node('div', undefined, 'pair');
    pair.append(sidePanel(0, data.left_text), sidePanel(1, data.right_text));
    workspace.append(pair);
    const form = node('div', undefined, 'decision');
    const preference = selectField(form, 'preference', '配对偏好', [
      ['left', '左侧'], ['right', '右侧'], ['tie', '持平'], ['unrated', '不评分'],
    ]);
    const blind = selectField(form, 'blind', '盲态是否完整', [['intact', '完整，尚未获知映射'], ['broken', '已破盲']]);
    form.append(button('锁定本卡反馈', () => {
      try {
        const feedback = { preference: requireChoice(preference, '整体偏好'),
          blind_status: requireChoice(blind, '盲态'), sides: panels.map(feedbackSide) };
        request({ action: 'submit', card, feedback });
      } catch (error) { announce(error.message); }
    }));
    workspace.append(form);
    announce(restored ? '已恢复卡片；此前计时不可恢复，本卡两侧时长将记为缺测。' : '请阅读任务，分别开始计时并明确填写反馈。');
  } else if (stage === 'recorded') {
    title('本卡反馈已锁定');
    workspace.append(node('p', '正文已清除。现在可以揭盲，反馈不能再修改。'));
    workspace.append(button('揭盲本卡', () => request({ action: 'reveal', card })));
    announce('反馈已记录，尚未揭盲。');
  } else if (stage === 'revealed') {
    title('本卡已揭盲');
    workspace.append(node('p', `左侧：${data.mapping?.[0] ?? '未知'}；右侧：${data.mapping?.[1] ?? '未知'}`));
    workspace.append(button(card === 4 ? '完成演练并查看元数据' : '下一张卡', () => request({ action: 'next', card })));
    announce('映射仅用于本次合成流程核对，不代表效果优劣。');
  } else if (stage === 'finished') {
    closed = true;
    report = data.report ?? null;
    title('演练已结束');
    announce('正文已清除。以下仅为合成操作元数据，不是真人效用结果。');
    if (report) {
      const summary = node('pre', JSON.stringify(report.summary, null, 2));
      workspace.append(node('h3', '合成演练摘要'), summary);
      workspace.append(button('下载合成元数据 JSON', () => {
        const blob = new Blob([JSON.stringify(report.metadata, null, 2)], { type: 'application/json;charset=utf-8' });
        const url = URL.createObjectURL(blob);
        const link = node('a');
        link.href = url; link.download = 'soul-synthetic-operator-metadata.json';
        link.click();
        setTimeout(() => URL.revokeObjectURL(url), 1000);
      }));
    } else workspace.append(node('p', '没有可下载的报告。'));
  } else throw new Error('unknown stage');
}
function loseFocus() {
  if (stage !== 'pair' || !timer) return;
  timer.invalidate();
  updateTimers();
  announce('页面失焦或隐藏，已暂停计时。本卡两侧时长记为缺测，可继续完成操作和评分。');
}
window.addEventListener('blur', loseFocus);
document.addEventListener('visibilitychange', () => { if (document.hidden) loseFocus(); });
window.addEventListener('pagehide', () => {
  clearDeadline();
  ++sequence; closed = true; clearText(); workspace.replaceChildren(); report = null;
});
window.addEventListener('pageshow', (event) => {
  if (event.persisted) { announce('页面已从缓存恢复，正文已清除。请刷新读取服务状态。'); sessionActions.hidden = true; }
});
document.querySelector('#stop').addEventListener('click', () => request({ action: 'stop' }, false, true));
document.querySelector('#withdraw').addEventListener('click', () => request({ action: 'withdraw' }, false, true));
request({ action: 'state' }, true);
