import assert from "node:assert/strict";
import { describe, it } from "vitest";

import { ImportRequests, type ImportRead } from "./importRequests";

function active(): ImportRequests {
  const requests = new ImportRequests();
  requests.activate();
  return requests;
}

function ready(requests: ImportRequests): ImportRead {
  const read = requests.beginRead("soul-import-v1");
  assert.ok(read);
  assert.equal(requests.finishRead(read, true), true);
  return read;
}

describe("import request ownership", () => {
  it("does not read or reset before mount", () => {
    const requests = new ImportRequests();
    assert.equal(requests.beginRead("soul-import-v1"), null);
    assert.equal(requests.reset(), false);
  });

  it("claims one read synchronously and keeps its format", () => {
    const requests = active();
    const read = requests.beginRead("telegram-desktop");
    assert.ok(read);
    assert.equal(read.format, "telegram-desktop");
    assert.equal(requests.isReading(read), true);
    assert.equal(requests.beginRead("soul-import-v1"), null);
  });

  it("does not put file contents in a read ticket", () => {
    const requests = active();
    const read = requests.beginRead("soul-import-v1");
    assert.ok(read);
    assert.deepEqual(Object.keys(read).sort(), ["format", "sequence"]);
  });

  it("does not reset while the read is still in flight", () => {
    const requests = active();
    const read = requests.beginRead("soul-import-v1")!;
    assert.equal(requests.reset(), false);
    assert.equal(requests.isReading(read), true);
  });

  it("requires the original read ticket, not a copied one", () => {
    const requests = active();
    const read = requests.beginRead("soul-import-v1")!;
    assert.equal(requests.isReading({ ...read }), false);
    assert.equal(requests.finishRead({ ...read }, true), false);
    assert.equal(requests.isReading(read), true);
  });

  it("does not commit an unfinished preview", () => {
    const requests = active();
    const read = requests.beginRead("soul-import-v1")!;
    assert.equal(requests.beginCommit(read), null);
    assert.equal(requests.finishRead(read, true), true);
    assert.notEqual(requests.beginCommit(read), null);
  });

  it("allows a failed read to be replaced but not committed", () => {
    const requests = active();
    const read = requests.beginRead("soul-import-v1")!;
    assert.equal(requests.finishRead(read, false), true);
    assert.equal(requests.beginCommit(read), null);
    assert.notEqual(requests.beginRead("telegram-desktop"), null);
  });

  it("settles a preview only once", () => {
    const requests = active();
    const read = ready(requests);
    assert.equal(requests.finishRead(read, false), false);
    assert.notEqual(requests.beginCommit(read), null);
  });

  it("does not accept a copied successful preview as confirmation", () => {
    const requests = active();
    const read = ready(requests);
    assert.equal(requests.beginCommit({ ...read }), null);
    assert.notEqual(requests.beginCommit(read), null);
  });

  it("admits at most one commit before it settles", () => {
    const requests = active();
    const read = ready(requests);
    assert.notEqual(requests.beginCommit(read), null);
    assert.equal(requests.beginCommit(read), null);
  });

  it("blocks both new reads and abandonment during a commit", () => {
    const requests = active();
    const read = ready(requests);
    const write = requests.beginCommit(read)!;
    assert.equal(requests.beginRead("telegram-desktop"), null);
    assert.equal(requests.reset(), false);
    assert.equal(requests.finishCommit(write, true), true);
  });

  it("does not unlock the commit when another ticket settles", () => {
    const requests = active();
    const read = ready(requests);
    const write = requests.beginCommit(read)!;
    assert.equal(requests.finishCommit(write + 1, false), false);
    assert.equal(requests.beginCommit(read), null);
    assert.equal(requests.finishCommit(write, true), true);
  });

  it("consumes a successful preview so its callback cannot replay it", () => {
    const requests = active();
    const read = ready(requests);
    const write = requests.beginCommit(read)!;
    assert.equal(requests.finishCommit(write, true), true);
    assert.equal(requests.beginCommit(read), null);
    assert.equal(requests.finishCommit(write, false), false);
  });

  it("preserves explicit retry after a failed commit", () => {
    const requests = active();
    const read = ready(requests);
    const first = requests.beginCommit(read)!;
    assert.equal(requests.finishCommit(first, false), true);
    const retry = requests.beginCommit(read);
    assert.notEqual(retry, null);
    assert.notEqual(retry, first);
    assert.equal(requests.finishCommit(first, true), false);
    assert.equal(requests.finishCommit(retry!, true), true);
  });

  it("allows an explicit new read of the same file after success", () => {
    const requests = active();
    const first = ready(requests);
    assert.equal(requests.finishCommit(requests.beginCommit(first)!, true), true);
    const second = ready(requests);
    assert.notEqual(second.sequence, first.sequence);
    assert.notEqual(requests.beginCommit(second), null);
  });

  it("invalidates an abandoned preview immediately", () => {
    const requests = active();
    const first = ready(requests);
    assert.equal(requests.reset(), true);
    assert.equal(requests.beginCommit(first), null);
  });

  it("invalidates an older preview as soon as a new file starts reading", () => {
    const requests = active();
    const first = ready(requests);
    const second = requests.beginRead("telegram-desktop")!;
    assert.equal(requests.beginCommit(first), null);
    assert.equal(requests.finishRead(second, true), true);
    assert.equal(requests.beginCommit(first), null);
    assert.notEqual(requests.beginCommit(second), null);
  });

  it("does not revive the old preview when its replacement fails", () => {
    const requests = active();
    const first = ready(requests);
    const second = requests.beginRead("telegram-desktop")!;
    assert.equal(requests.finishRead(second, false), true);
    assert.equal(requests.beginCommit(first), null);
    assert.equal(requests.beginCommit(second), null);
  });

  it("ignores a late read after unmount", () => {
    const requests = active();
    const read = requests.beginRead("soul-import-v1")!;
    requests.deactivate();
    assert.equal(requests.isReading(read), false);
    assert.equal(requests.finishRead(read, true), false);
    assert.equal(requests.beginRead("telegram-desktop"), null);
  });

  it("does not commit an unmounted preview", () => {
    const requests = active();
    const read = ready(requests);
    requests.deactivate();
    assert.equal(requests.beginCommit(read), null);
    assert.equal(requests.reset(), false);
  });

  it("ignores late commit completion after unmount", () => {
    const requests = active();
    const write = requests.beginCommit(ready(requests))!;
    requests.deactivate();
    assert.equal(requests.finishCommit(write, true), false);
  });

  it("effect replay permits new reads without reviving old ones", () => {
    const requests = active();
    const old = requests.beginRead("soul-import-v1")!;
    requests.deactivate();
    requests.activate();
    const current = requests.beginRead("telegram-desktop")!;
    assert.equal(requests.finishRead(old, true), false);
    assert.equal(requests.finishRead(current, true), true);
  });

  it("effect replay cannot let old commit completion consume a new commit", () => {
    const requests = active();
    const old = requests.beginCommit(ready(requests))!;
    requests.deactivate();
    requests.activate();
    const current = requests.beginCommit(ready(requests))!;
    assert.equal(requests.finishCommit(old, true), false);
    assert.equal(requests.finishCommit(current, true), true);
  });

  it("does not accept another page's preview", () => {
    const first = active();
    const second = active();
    const read = ready(first);
    assert.equal(second.beginCommit(read), null);
    assert.notEqual(first.beginCommit(read), null);
  });
});
