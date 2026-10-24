/** App + real core/runtime bridge; only native IPC/events and clipboard are synthetic. */
import { listen } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../App";
import * as core from "../core";
import { aTemplateDraft, CLOSED_SNAPSHOT, DRAFT_NOTICES, forbidNetwork, OPEN_SESSION } from "../test/fakeCore";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const PASTED = "合成邀请：周五一起核对测试安排，可以吗？";
const GENERATED = "可以，周五我们一起核对合成测试安排。";
const EDITED = "可以，周五下午我们一起核对合成测试安排。";
const EPOCH = "10000000-0000-4000-8000-000000000001";
const unlisten = vi.fn();
let calls: { command: string; payload: unknown }[];
let draftResponse: core.Draft | Promise<core.Draft>;
let networkAttempts: string[];

function callsTo(command: string) {
  return calls.filter((call) => call.command === command);
}

beforeEach(() => {
  // Keep user-event, waitFor and setup.ts's navigation timeout on real timers.
  // Only the bridge's actual 10-second periodic callback is advanced manually.
  vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
  mockWindows("main");
  networkAttempts = forbidNetwork();
  calls = [];
  draftResponse = aTemplateDraft({ text: GENERATED });
  unlisten.mockReset();
  vi.mocked(listen).mockReset();
  vi.mocked(listen).mockResolvedValue(unlisten);
  mockIPC((command, payload) => {
    calls.push({ command, payload });
    switch (command) {
      case "runtime_event_snapshot": return { epoch: EPOCH, sequence: 0, active_operations: [] };
      case "config_snapshot": return CLOSED_SNAPSHOT;
      case "session_status": return OPEN_SESSION;
      case "draft_notices": return DRAFT_NOTICES;
      // Ordinary native draft_reply does not publish a runtime mutation event.
      case "draft_reply": return draftResponse;
      default: throw new Error(`Unexpected synthetic local-flow IPC: ${command}`);
    }
  }, { shouldMockEvents: true });
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  expect(networkAttempts).toEqual([]);
});

async function page() {
  window.location.hash = "#/draft";
  const user = userEvent.setup();
  render(<App />);
  const input = await screen.findByLabelText("原文");
  await waitFor(() => expect(core.hasPendingSourceChanges()).toBe(false));
  expect(listen).toHaveBeenCalledWith("soul-runtime-change", expect.any(Function));
  fireEvent.change(input, { target: { value: PASTED } });
  return { user, input, write: screen.getByRole("button", { name: "写一版草稿" }) };
}

async function unchangedSnapshot(trigger: "periodic" | "focus") {
  const reads = callsTo("runtime_event_snapshot").length;
  const source = core.getDraftSourceRevision();
  await act(async () => {
    if (trigger === "periodic") await vi.advanceTimersByTimeAsync(10_000);
    else window.dispatchEvent(new Event("focus"));
  });
  await waitFor(() => expect(callsTo("runtime_event_snapshot")).toHaveLength(reads + 1));
  expect(core.getDraftSourceRevision()).toBe(source);
  expect(core.hasPendingSourceChanges()).toBe(false);
}

describe("ordinary T1 through App and the native runtime bridge", () => {
  it("keeps a generated/edited draft across periodic snapshots and focus, and copies the edit", async () => {
    const { user, input, write } = await page();
    const clipboard = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue();
    await user.click(write);
    const result = await screen.findByLabelText("你可以直接改");
    expect(result).toHaveValue(GENERATED);
    expect(input).toHaveValue(PASTED);
    expect(write).toBeEnabled();
    expect(callsTo("draft_reply")).toEqual([{ command: "draft_reply", payload: { pasted: PASTED } }]);

    await unchangedSnapshot("periodic");
    await unchangedSnapshot("focus");
    expect(screen.getByLabelText("你可以直接改")).toBe(result);
    expect(result).toHaveValue(GENERATED);
    await user.clear(result);
    await user.type(result, EDITED);
    await unchangedSnapshot("periodic");
    await unchangedSnapshot("focus");
    expect(screen.getByLabelText("你可以直接改")).toBe(result);
    expect(result).toHaveValue(EDITED);
    await user.click(screen.getByRole("button", { name: "复制" }));
    expect(await screen.findByText("已复制到剪贴板。")).toBeVisible();
    expect(clipboard).toHaveBeenCalledExactlyOnceWith(EDITED);
    expect(callsTo("draft_reply")).toHaveLength(1);
    expect(screen.queryByTestId("draft-source-changed")).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
    expect(callsTo("prepare_draft")).toHaveLength(0);
    expect(callsTo("generate_draft")).toHaveLength(0);
  });

  it("accepts a pending local reply after unchanged periodic/focus snapshots without clearing or replaying it", async () => {
    let resolve!: (draft: core.Draft) => void;
    const pending = new Promise<core.Draft>((done) => { resolve = done; });
    draftResponse = pending;
    const { user, write } = await page();
    await user.click(write);
    await waitFor(() => expect(callsTo("draft_reply")).toHaveLength(1));
    expect(write).toBeDisabled();
    await unchangedSnapshot("periodic");
    await unchangedSnapshot("focus");
    expect(screen.queryByLabelText("你可以直接改")).toBeNull();
    await act(async () => { resolve(aTemplateDraft({ text: GENERATED })); await pending; });
    const result = await screen.findByLabelText("你可以直接改");
    expect(result).toHaveValue(GENERATED);
    expect(write).toBeEnabled();
    await unchangedSnapshot("periodic");
    expect(screen.getByLabelText("你可以直接改")).toBe(result);
    expect(callsTo("draft_reply")).toHaveLength(1);
    expect(screen.queryByTestId("draft-source-changed")).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("moves focus into the editable result when local drafting succeeds", async () => {
    const { user, write } = await page();
    await user.click(write);
    const result = await screen.findByLabelText("你可以直接改");
    expect(result).toHaveValue(GENERATED);
    // jsdom cannot establish viewport placement. Focus is the observable
    // keyboard/accessibility handoff into the newly mounted result instead.
    expect(result).toHaveFocus();
  });

  it("does not display or focus a late reply after the pasted input changed", async () => {
    let resolve!: (draft: core.Draft) => void;
    const pending = new Promise<core.Draft>((done) => { resolve = done; });
    draftResponse = pending;
    const { user, input, write } = await page();
    await user.click(write);
    await waitFor(() => expect(callsTo("draft_reply")).toHaveLength(1));
    await user.click(input);
    fireEvent.change(input, { target: { value: "合成更正：改成下周再核对。" } });
    expect(input).toHaveFocus();
    await act(async () => { resolve(aTemplateDraft({ text: GENERATED })); await pending; });
    expect(write).toBeEnabled();
    expect(screen.queryByLabelText("你可以直接改")).toBeNull();
    expect(input).toHaveFocus();
    expect(callsTo("draft_reply")).toHaveLength(1);
  });
});
