import { summarize } from './feedback.mjs';

// No filenames, default datasets, product state, or network access.
try {
  if (process.argv.length !== 2 || process.stdin.isTTY) throw new Error();
  const chunks = []; let size = 0;
  for await (const chunk of process.stdin) {
    size += chunk.length;
    if (size > 8 * 1024 * 1024) throw new Error();
    chunks.push(chunk);
  }
  const input = Buffer.concat(chunks).toString('utf8').replace(/^\uFEFF/, '');
  const rows = input.split(/\r?\n/).filter(line => line.trim() !== '').map(line => JSON.parse(line));
  process.stdout.write(`${JSON.stringify(summarize(rows), null, 2)}\n`);
} catch {
  process.stderr.write('INVALID_INPUT\n');
  process.exitCode = 1;
}
