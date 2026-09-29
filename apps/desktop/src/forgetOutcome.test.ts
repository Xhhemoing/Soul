import assert from "node:assert/strict";
import { test } from "vitest";

import { cleanupStatus, forgetStatus } from "./forgetOutcome";

const complete = {
  logical_committed: true,
  cleanup: { state: "complete" },
  audit: "recorded",
  matched_preview: true,
};
const pending = { state: "pending", checkpoint: null };

test("only a fully acknowledged matching receipt is complete", () => {
  assert.equal(forgetStatus(complete), "complete");
});

test("a committed operation can still have pending cleanup", () => {
  assert.equal(forgetStatus({ ...complete, cleanup: pending }), "cleanup_pending");
});

test("successful cleanup does not confirm audit", () => {
  assert.equal(forgetStatus({ ...complete, audit: "unconfirmed" }), "audit_unconfirmed");
});

test("cleanup and audit can both remain unconfirmed", () => {
  assert.equal(
    forgetStatus({ ...complete, cleanup: pending, audit: "unconfirmed" }),
    "cleanup_and_audit_unconfirmed",
  );
});

test("a mismatched preview does not receive the all-clear label", () => {
  assert.equal(forgetStatus({ ...complete, matched_preview: false }), "preview_mismatch");
});

test("logical commit must be an explicit true", () => {
  for (const logical_committed of [false, null, undefined, 1, "true"]) {
    assert.equal(forgetStatus({ ...complete, logical_committed }), "unconfirmed");
  }
});

test("unknown audit values are not acknowledged", () => {
  assert.equal(forgetStatus({ ...complete, audit: "new-state" }), "unconfirmed");
});

test("unknown cleanup values are not complete", () => {
  assert.equal(forgetStatus({ ...complete, cleanup: { state: "new-state" } }), "unconfirmed");
});

test("a legacy receipt does not acquire default success states", () => {
  assert.equal(forgetStatus({ memory_id: "synthetic", matched_preview: true }), "unconfirmed");
});

test("non-object input cannot claim success", () => {
  for (const value of [null, undefined, false, "complete", 0, []]) {
    assert.equal(forgetStatus(value), "unconfirmed");
  }
});

test("the UI does not infer completion from checkpoint counts", () => {
  assert.equal(
    forgetStatus({
      ...complete,
      cleanup: {
        state: "pending",
        checkpoint: { busy: 0, log_frames: 0, checkpointed_frames: 0 },
      },
    }),
    "cleanup_pending",
  );
});

test("standalone cleanup reports have their own conservative status", () => {
  assert.equal(cleanupStatus({ state: "complete" }), "complete");
  assert.equal(cleanupStatus(pending), "pending");
  for (const value of [null, undefined, {}, { state: "unknown" }, true, []]) {
    assert.equal(cleanupStatus(value), "unconfirmed");
  }
});

test("presenting a receipt does not mutate its independent observations", () => {
  const receipt = { ...complete, cleanup: pending, audit: "unconfirmed" };
  const before = JSON.stringify(receipt);
  forgetStatus(receipt);
  assert.equal(JSON.stringify(receipt), before);
});

test("a cleanup-only answer is not proof that a forget or audit happened", () => {
  assert.equal(forgetStatus({ state: "complete" }), "unconfirmed");
});
