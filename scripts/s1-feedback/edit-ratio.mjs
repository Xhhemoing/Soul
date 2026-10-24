import { createInterface } from 'node:readline';
import { Writable } from 'node:stream';
import { pathToFileURL } from 'node:url';

export function editRatio(draft, final) {
  let a = Array.from(draft), b = Array.from(final);
  if (a.length > 20000 || b.length > 20000 || a.length * b.length > 25000000) throw new Error('INPUT_TOO_LARGE');
  const denominator = Math.max(a.length, b.length);
  if (!denominator) return 0;
  if (a.length > b.length) [a, b] = [b, a];
  let previous = Array.from({ length: a.length + 1 }, (_, i) => i);
  for (let j = 1; j <= b.length; j++) {
    const next = [j];
    for (let i = 1; i <= a.length; i++) next[i] = Math.min(next[i - 1] + 1, previous[i] + 1, previous[i - 1] + (a[i - 1] === b[j - 1] ? 0 : 1));
    previous = next;
  }
  return previous[a.length] / denominator;
}

async function main() {
  let rl;
  try {
    if (process.argv.length !== 2) throw new Error();
    const terminal = Boolean(process.stdin.isTTY);
    // Mute readline's local echo; no pasted content is written to output.
    const muted = new Writable({ write(_chunk, _encoding, callback) { callback(); } });
    rl = createInterface({ input: process.stdin, output: muted, terminal, crlfDelay: Infinity, historySize: 0 });
    let parts = [], lines = [], length = 0, cancelled = false;
    rl.on('SIGINT', () => { cancelled = true; lines = []; parts = []; rl.close(); });
    if (terminal) process.stderr.write('Paste draft; finish with .end on its own line. Input echo is hidden.\n');
    for await (const line of rl) {
      if (parts.length === 2) throw new Error();
      if (line === '.end') {
        parts.push(lines.join('\n')); lines = []; length = 0;
        if (terminal && parts.length === 1) process.stderr.write('Paste final; finish with .end on its own line.\n');
        if (terminal && parts.length === 2) break;
      } else {
        length += line.length + 1;
        if (length > 40002) throw new Error();
        lines.push(line);
      }
    }
    if (cancelled) { process.exitCode = 130; return; }
    if (parts.length !== 2 || lines.length) throw new Error();
    const value = editRatio(parts[0], parts[1]);
    parts = []; lines = [];
    process.stdout.write(`${value}\n`);
  } catch {
    process.stderr.write('INVALID_INPUT\n'); process.exitCode = 1;
  } finally { rl?.close(); }
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await main();
