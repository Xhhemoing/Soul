import { mockIPC } from "@tauri-apps/api/mocks";
import { waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { requestDraftDiscard } from "./draftDiscard";
import { draftLane } from "./draftRequests";

it("源编辑排队discard且保留前一物理任务槽，drain结束前不准新prepare", async () => {
  let resolve!: (value: boolean) => void; const pending = new Promise<boolean>((yes) => { resolve = yes; });
  const native = vi.fn((cmd: string) => { if (cmd !== "discard_draft") throw new Error("unexpected command"); return pending; });
  mockIPC(native);
  const prior = draftLane.begin("prepare"); expect(prior).not.toBeNull();
  requestDraftDiscard(); requestDraftDiscard(); expect(native).not.toHaveBeenCalled();
  expect(draftLane.begin("prepare")).toBeNull();
  draftLane.finish(prior!, false); await waitFor(() => expect(native).toHaveBeenCalledTimes(1));
  expect(draftLane.snapshot().busy).toBe(true); expect(draftLane.begin("prepare")).toBeNull();
  resolve(true); await pending; await waitFor(() => expect(draftLane.snapshot().busy).toBe(false));
  const next = draftLane.begin("prepare"); expect(next).not.toBeNull(); draftLane.finish(next!, false);
});
