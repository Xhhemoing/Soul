import { describe, expect, it } from "vitest";
import { anE1Plan } from "../test/fakeCore";
import { DraftLane, DraftRequests, isDraftRefusal } from "./draftRequests";

function page(lane = new DraftLane()) {
  const requests = new DraftRequests(lane);
  requests.activate();
  const revision = requests.edit("合成原文 A");
  return { requests, lane, revision };
}

describe("draft request ownership", () => {
  it("admits once synchronously and keeps the slot through A/B/A edits", () => {
    const { requests, revision, lane } = page();
    const first = requests.begin("prepare", revision)!;
    expect(requests.begin("local", revision)).toBeNull();
    requests.edit("合成原文 B");
    const newer = requests.edit("合成原文 A");
    expect(requests.owns(first)).toBe(false);
    expect(requests.begin("prepare", newer)).toBeNull();
    expect(requests.finish(first)).toBe(false);
    expect(lane.snapshot().busy).toBe(false);
    expect(requests.begin("prepare", newer)).not.toBeNull();
  });

  it("rejects a stale button's revision even when its string is unchanged", () => {
    const { requests, revision } = page();
    requests.edit("合成原文 B");
    requests.edit("合成原文 A");
    expect(requests.begin("prepare", revision)).toBeNull();
  });

  it("binds approval and original exemption to the exact displayed quote", () => {
    const { requests, revision } = page();
    const ticket = requests.begin("prepare", revision)!;
    const quote = requests.remember(ticket, anE1Plan())!;
    requests.finish(ticket);
    expect(requests.begin("generate", revision, { ...quote })).toBeNull();
    expect(requests.begin("original", revision, { ...quote })).toBeNull();
    const original = requests.begin("original", revision, quote)!;
    expect(original.text).toBe("合成原文 A");
    expect(requests.begin("generate", revision, quote)).toBeNull();
    const second = requests.remember(original, anE1Plan({ carries_exempted_original: true }))!;
    requests.finish(original);
    expect(requests.begin("original", revision, second)).toBeNull();
    expect(requests.begin("generate", revision, quote)).toBeNull();
    expect(requests.begin("generate", revision, second)).not.toBeNull();
  });

  it("editing invalidates the original exemption even after editing back", () => {
    const { requests, revision } = page();
    const ticket = requests.begin("prepare", revision)!;
    const quote = requests.remember(ticket, anE1Plan())!;
    requests.finish(ticket);
    requests.edit("合成原文 B");
    const newer = requests.edit("合成原文 A");
    expect(requests.begin("original", newer, quote)).toBeNull();
    expect(requests.begin("generate", newer, quote)).toBeNull();
    expect(requests.begin("discard", newer, quote)).toBeNull();
  });

  it("does not let an old finish free a newer physical slot", () => {
    const { requests, revision, lane } = page();
    const old = requests.begin("local", revision)!;
    requests.finish(old);
    const current = requests.begin("local", revision)!;
    expect(requests.finish(old)).toBe(false);
    expect(lane.snapshot().busy).toBe(true);
    expect(requests.owns(current)).toBe(true);
  });

  it("source invalidation during a busy-state notification cannot adopt a newer revision", () => {
    const { requests, revision, lane } = page();
    const unsubscribe = lane.subscribe(() => { requests.invalidate(); });
    const ticket = requests.begin("prepare", revision)!;
    expect(requests.owns(ticket)).toBe(false);
    unsubscribe();
    expect(requests.finish(ticket)).toBe(false);
  });

  it("rechecks ownership after completion notifies subscribers", () => {
    const { requests, revision, lane } = page();
    const ticket = requests.begin("local", revision)!;
    lane.subscribe(() => { requests.invalidate(); });
    expect(requests.finish(ticket)).toBe(false);
    expect(lane.snapshot().busy).toBe(false);
  });

  it("a discarded route keeps its physical slot until the old command settles", () => {
    const { requests, revision, lane } = page();
    const first = requests.begin("prepare", revision)!;
    const quote = requests.remember(first, anE1Plan())!;
    requests.finish(first);
    const discard = requests.begin("discard", revision, quote)!;
    requests.dispose();
    const next = page(lane);
    expect(next.requests.begin("prepare", next.revision)).toBeNull();
    expect(requests.finish(discard)).toBe(false);
    expect(next.requests.begin("prepare", next.revision)).not.toBeNull();
  });

  it("reactivation cannot adopt an old lifetime's result", () => {
    const { requests, revision, lane } = page();
    const first = requests.begin("prepare", revision)!;
    requests.dispose();
    requests.activate();
    expect(requests.remember(first, anE1Plan())).toBeNull();
    expect(requests.finish(first)).toBe(false);
    expect(lane.snapshot().busy).toBe(false);
  });

  it.each(["prepare", "generate", "discard"] as const)(
    "an unknown %s outcome blocks endpoint reuse across remount, but permits local templates",
    (kind) => {
      const { requests, revision, lane } = page();
      const first = requests.begin("prepare", revision)!;
      const quote = requests.remember(first, anE1Plan())!;
      requests.finish(first);
      const ticket = requests.begin(kind, revision, quote)!;
      requests.finish(ticket, true);
      requests.dispose();
      const next = page(lane);
      expect(next.requests.begin("prepare", next.revision)).toBeNull();
      expect(next.requests.begin("local", next.revision)).not.toBeNull();
      expect(lane.snapshot().uncertain).toBe(true);
    },
  );

  it("structured refusals and unknown transport errors remain distinct", () => {
    expect(isDraftRefusal({ reason_code: "NO", explanation: "合成拒绝" })).toBe(true);
    expect(isDraftRefusal(new Error("lost reply"))).toBe(false);
    expect(isDraftRefusal({ reason_code: "NO" })).toBe(false);
    expect(isDraftRefusal(null)).toBe(false);
  });
});
