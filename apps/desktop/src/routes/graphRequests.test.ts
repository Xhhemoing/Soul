import assert from "node:assert/strict";
import { describe, it } from "vitest";

import { GraphRequests } from "./graphRequests";

function active(): GraphRequests {
  const requests = new GraphRequests();
  requests.activate();
  return requests;
}

describe("graph request lifecycle", () => {
  it("does not admit work before mount", () => {
    const requests = new GraphRequests();
    assert.equal(requests.beginWrite(), null);
    assert.equal(requests.beginSummary("a"), null);
  });

  it("claims a write before another caller can begin", () => {
    const requests = active();
    const write = requests.beginWrite();
    assert.notEqual(write, null);
    assert.equal(requests.beginWrite(), null);
    assert.equal(requests.beginSummary("a"), null);
  });

  it("does not consume a write when a wrong ticket settles", () => {
    const requests = active();
    const write = requests.beginWrite();
    assert.notEqual(write, null);
    assert.equal(requests.finishWrite(-1), false);
    assert.equal(requests.beginWrite(), null);
    assert.equal(requests.finishWrite(write!), true);
  });

  it("finishes each write only once and admits retry", () => {
    const requests = active();
    const write = requests.beginWrite()!;
    assert.equal(requests.finishWrite(write), true);
    assert.equal(requests.finishWrite(write), false);
    const next = requests.beginWrite();
    assert.notEqual(next, null);
    assert.notEqual(next, write);
  });

  it("invalidates a pending summary as soon as a write begins", () => {
    const requests = active();
    const summary = requests.beginSummary("a")!;
    const write = requests.beginWrite()!;
    assert.equal(requests.finishSummary(summary), false);
    assert.equal(requests.finishWrite(write), true);
    assert.equal(requests.finishSummary(summary), false);
  });

  it("allows a new summary after a write settles", () => {
    const requests = active();
    assert.equal(requests.finishWrite(requests.beginWrite()!), true);
    assert.notEqual(requests.beginSummary("a"), null);
  });

  it("coalesces a duplicate pending summary for the same person", () => {
    const requests = active();
    const first = requests.beginSummary("a")!;
    assert.equal(requests.beginSummary("a"), null);
    assert.equal(requests.finishSummary(first), true);
  });

  it("permits refresh after the summary has settled", () => {
    const requests = active();
    const first = requests.beginSummary("a")!;
    assert.equal(requests.finishSummary(first), true);
    const second = requests.beginSummary("a")!;
    assert.notEqual(first.sequence, second.sequence);
    assert.equal(requests.finishSummary(second), true);
  });

  it("lets a newer person supersede the previous response", () => {
    const requests = active();
    const first = requests.beginSummary("a")!;
    const second = requests.beginSummary("b")!;
    assert.equal(requests.finishSummary(first), false);
    assert.equal(requests.finishSummary(second), true);
  });

  it("rejects an old response even after selecting the same person again", () => {
    const requests = active();
    const first = requests.beginSummary("a")!;
    const second = requests.beginSummary("b")!;
    const third = requests.beginSummary("a")!;
    assert.equal(requests.finishSummary(first), false);
    assert.equal(requests.finishSummary(second), false);
    assert.equal(requests.finishSummary(third), true);
  });

  it("does not accept a copied ticket as the pending request", () => {
    const requests = active();
    const first = requests.beginSummary("a")!;
    assert.equal(requests.finishSummary({ ...first }), false);
    assert.equal(requests.finishSummary(first), true);
    assert.equal(requests.finishSummary(first), false);
  });

  it("does not accept a summary after unmount", () => {
    const requests = active();
    const summary = requests.beginSummary("a")!;
    requests.deactivate();
    assert.equal(requests.finishSummary(summary), false);
    assert.equal(requests.beginSummary("b"), null);
    assert.equal(requests.beginWrite(), null);
  });

  it("does not accept a write after unmount", () => {
    const requests = active();
    const write = requests.beginWrite()!;
    requests.deactivate();
    assert.equal(requests.finishWrite(write), false);
  });

  it("supports effect cleanup and setup without reviving an old summary", () => {
    const requests = active();
    const previous = requests.beginSummary("a")!;
    requests.deactivate();
    requests.activate();
    const current = requests.beginSummary("a")!;
    assert.equal(requests.finishSummary(previous), false);
    assert.equal(requests.finishSummary(current), true);
  });

  it("supports effect cleanup and setup without reviving an old write", () => {
    const requests = active();
    const previous = requests.beginWrite()!;
    requests.deactivate();
    requests.activate();
    const current = requests.beginWrite()!;
    assert.equal(requests.finishWrite(previous), false);
    assert.equal(requests.beginWrite(), null);
    assert.equal(requests.finishWrite(current), true);
  });

  it("keeps separate mounted pages independent", () => {
    const first = active();
    const second = active();
    const write = first.beginWrite()!;
    const summary = second.beginSummary("a")!;
    assert.equal(first.finishSummary(summary), false);
    assert.equal(second.finishSummary(summary), true);
    assert.equal(first.finishWrite(write), true);
  });
});