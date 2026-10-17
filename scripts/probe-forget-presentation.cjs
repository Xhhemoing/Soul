#!/usr/bin/env node
'use strict';

// A dependency-light probe, NOT the project's Vitest, React, or native gate.
// Requires an already installed TypeScript (local resolution or NODE_PATH).
// It never installs packages, contacts a network, or opens a Soul database.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const ts = require('typescript');

const root = path.resolve(__dirname, '..');
const sourceDir = path.join(root, 'apps', 'desktop', 'src');
const helper = path.join(sourceDir, 'forgetOutcome.ts');
const options = {
  strict: true,
  noEmit: true,
  target: ts.ScriptTarget.ES2022,
  module: ts.ModuleKind.CommonJS,
  types: [],
};
const host = {
  getCanonicalFileName: (name) => name,
  getCurrentDirectory: () => root,
  getNewLine: () => '\n',
};
const program = ts.createProgram([helper], options);
const diagnostics = ts.getPreEmitDiagnostics(program);
if (diagnostics.length) {
  process.stderr.write(ts.formatDiagnosticsWithColorAndContext(diagnostics, host));
  process.exitCode = 1;
} else {
  console.log(`Node ${process.version}; TypeScript ${ts.version}`);
  console.log('Standalone helper strict check passed. Test harness: node:test, NOT Vitest.');
  const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'soul-forget-presentation-'));
  try {
    for (const name of ['forgetOutcome.ts', 'forgetOutcome.test.ts']) {
      let source = fs.readFileSync(path.join(sourceDir, name), 'utf8');
      if (name.endsWith('.test.ts')) {
        const original = 'import { test } from "vitest";';
        assert.equal(source.split(original).length, 2, 'expected exactly one harness import');
        // Only the harness import changes; assertions and test bodies are identical.
        source = source.replace(original, 'import { test } from "node:test";');
      }
      const result = ts.transpileModule(source, {
        fileName: name,
        reportDiagnostics: true,
        compilerOptions: {
          target: ts.ScriptTarget.ES2022,
          module: ts.ModuleKind.CommonJS,
          esModuleInterop: true,
        },
      });
      const errors = (result.diagnostics || []).filter(
        (diagnostic) => diagnostic.category === ts.DiagnosticCategory.Error,
      );
      assert.equal(errors.length, 0, ts.formatDiagnosticsWithColorAndContext(errors, host));
      fs.writeFileSync(path.join(scratch, name.replace(/\.ts$/, '.js')), result.outputText);
    }
    execFileSync(process.execPath, ['--test', path.join(scratch, 'forgetOutcome.test.js')], {
      stdio: 'inherit',
    });
  } catch (error) {
    console.error(error.message);
    process.exitCode = Number.isInteger(error.status) ? error.status : 1;
  } finally {
    fs.rmSync(scratch, { recursive: true, force: true });
  }
}
