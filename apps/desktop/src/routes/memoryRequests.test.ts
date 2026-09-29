import assert from "node:assert/strict";
import { it } from "vitest";
import { MemoryRequests } from "./memoryRequests";

function ready(): MemoryRequests { const r = new MemoryRequests(); r.activate(); return r; }
function preview(r: MemoryRequests): object {
  const value = { preview_id: "synthetic" };
  const ticket = r.begin(); assert.notEqual(ticket, null);
  r.remember(ticket!, value); assert.equal(r.finish(ticket!), true); return value;
}

it("does not admit before mount", () => {
  const r = new MemoryRequests(); assert.equal(r.begin(), null);
});
it("admits only one in-flight command", () => {
  const r = ready(); const ticket = r.begin()!;
  assert.equal(r.begin(), null); assert.equal(r.owns(ticket), true);
});
it("an unrelated finish cannot release the active command", () => {
  const r = ready(); const ticket = r.begin()!;
  assert.equal(r.finish(ticket + 1), false); assert.equal(r.begin(), null);
});
it("a finished ticket cannot accept a late response", () => {
  const r = ready(); const ticket = r.begin()!; r.finish(ticket);
  assert.equal(r.owns(ticket), false); assert.notEqual(r.begin(), ticket);
});
it("the displayed preview can be confirmed once while pending", () => {
  const r = ready(); const value = preview(r); const ticket = r.begin(value)!;
  assert.equal(r.owns(ticket), true); assert.equal(r.begin(value), null);
});
it("a copy of a preview does not own the confirmation callback", () => {
  const r = ready(); const value = preview(r); assert.equal(r.begin({ ...value }), null);
});
it("abandonment invalidates a retained confirmation callback", () => {
  const r = ready(); const value = preview(r); assert.equal(r.abandon(), true);
  assert.equal(r.begin(value), null);
});
it("replacement invalidates the earlier preview", () => {
  const r = ready(); const first = preview(r); const second = preview(r);
  assert.equal(r.begin(first), null); assert.notEqual(r.begin(second), null);
});
it("an explicit mismatch may preserve the preview for a valid retry", () => {
  const r = ready(); const value = preview(r); const ticket = r.begin(value)!;
  r.finish(ticket); assert.notEqual(r.begin(value), null);
});
it("success or an unknown outcome consumes page ownership", () => {
  const r = ready(); const value = preview(r); const ticket = r.begin(value)!;
  r.remember(ticket, null); r.finish(ticket); assert.equal(r.begin(value), null);
});
it("cannot abandon an in-flight confirmation", () => {
  const r = ready(); const value = preview(r); r.begin(value); assert.equal(r.abandon(), false);
});
it("unmount rejects both pending callbacks and new work", () => {
  const r = ready(); const value = preview(r); const ticket = r.begin(value)!;
  r.dispose(); assert.equal(r.owns(ticket), false); assert.equal(r.begin(value), null);
});
it("remount cannot revive old operation or preview identities", () => {
  const r = ready(); const value = preview(r); const ticket = r.begin(value)!;
  r.dispose(); r.activate(); assert.equal(r.owns(ticket), false); assert.equal(r.begin(value), null);
});
it("a later command invalidates an older list request", () => {
  const r = ready(); const list = r.nextList(); r.begin(); assert.equal(r.ownsList(list), false);
});
it("list responses cannot replace state while a command is pending", () => {
  const r = ready(); r.begin(); const list = r.nextList(); assert.equal(r.ownsList(list), false);
});
it("only the latest mounted idle list is current", () => {
  const r = ready(); const first = r.nextList(); const second = r.nextList();
  assert.equal(r.ownsList(first), false); assert.equal(r.ownsList(second), true);
  r.dispose(); assert.equal(r.ownsList(second), false);
});
it("an old response cannot resurrect a consumed preview", () => {
  const r = ready(); const value = preview(r); const ticket = r.begin(value)!;
  r.remember(ticket, null); r.finish(ticket); r.remember(ticket, value);
  assert.equal(r.begin(value), null);
});
