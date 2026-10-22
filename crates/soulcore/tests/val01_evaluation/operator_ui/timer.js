export function createPairTimer(now = () => performance.now(), recovered = false) {
  const sides = [0, 1].map(() => ({
    status: 'idle', reliable: !recovered, active: 0, paused: 0, since: 0,
  }));
  function side(index) {
    if (index !== 0 && index !== 1) throw new Error('计时侧无效');
    return sides[index];
  }
  function enter(index, from, status) {
    const item = side(index);
    if (!from.includes(item.status)) throw new Error('当前计时状态不允许此操作');
    if (status === 'active' && sides.some((s) => s.status === 'active')) {
      throw new Error('请先暂停或结束另一侧计时');
    }
    const time = now();
    if (item.status === 'active') item.active += Math.max(0, time - item.since);
    if (item.status === 'paused') item.paused += Math.max(0, time - item.since);
    item.since = time;
    item.status = status;
  }
  return {
    start(index) { enter(index, ['idle'], 'active'); },
    pause(index) { enter(index, ['active'], 'paused'); },
    resume(index) { enter(index, ['paused'], 'active'); },
    finish(index) { enter(index, ['active', 'paused'], 'finished'); },
    invalidate(index) {
      for (const i of index === undefined ? [0, 1] : [index]) {
        const item = side(i);
        item.reliable = false;
        if (item.status === 'active') enter(i, ['active'], 'paused');
      }
    },
    snapshot(index) {
      const item = side(index);
      const measured = item.reliable && item.status === 'finished';
      return {
        status: item.status, reliable: item.reliable,
        edit_seconds: measured ? Math.floor(item.active / 1000) : null,
        pause_seconds: measured ? Math.floor(item.paused / 1000) : null,
      };
    },
  };
}
