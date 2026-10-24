import { makeBatchStart, makeBatchEnd, makeTask, validateRecord } from './feedback.mjs';

try {
  const [command, batch, mode, ...extra] = process.argv.slice(2);
  let record;
  if (command === 'start' && batch === undefined) record = makeBatchStart();
  else if (command === 'task' && batch && ['T1', 'T3'].includes(mode) && !extra.length) record = makeTask(batch, mode);
  else if (command === 'end' && batch && mode === undefined) record = makeBatchEnd(batch);
  else throw new Error();
  validateRecord(record);
  process.stdout.write(`${JSON.stringify(record)}\n`);
} catch {
  process.stderr.write('INVALID_INPUT\n');
  process.exitCode = 1;
}
