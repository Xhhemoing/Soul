/** Real React event/IPC boundaries with deferred, synthetic core replies. */
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as core from "../core";
import type { Draft as DraftValue, E1DraftPlan } from "../core";
import { anE1Plan, aTemplateDraft } from "../test/fakeCore";
import { Draft } from "./Draft";
import { DraftRequests } from "./draftRequests";

vi.mock("../core", () => ({
  discardDraft: vi.fn(), draftNotices: vi.fn(), draftReply: vi.fn(),
  generateDraft: vi.fn(), prepareDraft: vi.fn(),
  getDraftSourceRevision: vi.fn(), subscribeDraftSource: vi.fn(),
  DraftSourceChangedError: class DraftSourceChangedError extends Error {},
}));

const A = "合成原文 A";
const B = "合成原文 B";
const sourceListeners = new Set<(revision: symbol) => void>();
let sourceRevision = Symbol();

function sourceChanged(): void {
  sourceRevision = Symbol();
  sourceListeners.forEach((listener) => listener(sourceRevision));
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

async function page() {
  const mounted = render(<Draft />);
  const user = userEvent.setup();
  const input = await screen.findByLabelText("原文");
  fireEvent.change(input, { target: { value: A } });
  return {
    ...mounted, user, input,
    local: screen.getByRole("button", { name: "写一版草稿" }),
    endpoint: screen.getByRole("button", { name: "用你自己的模型端点写" }),
  };
}

async function prepared() {
  const view = await page();
  await view.user.click(view.endpoint);
  await screen.findByRole("heading", { name: "确认这一次要生成什么" });
  return view;
}

beforeEach(() => {
  vi.restoreAllMocks();
  vi.resetAllMocks();
  sourceListeners.clear();
  sourceRevision = Symbol();
  vi.mocked(core.getDraftSourceRevision).mockImplementation(() => sourceRevision);
  vi.mocked(core.subscribeDraftSource).mockImplementation((listener) => {
    sourceListeners.add(listener);
    listener(sourceRevision);
    return () => { sourceListeners.delete(listener); };
  });
  vi.mocked(core.draftNotices).mockResolvedValue({ not_sent: "合成说明", e1_plan: "合成计划", can_send: false });
  vi.mocked(core.prepareDraft).mockResolvedValue(anE1Plan());
  vi.mocked(core.generateDraft).mockResolvedValue(aTemplateDraft());
  vi.mocked(core.draftReply).mockResolvedValue(aTemplateDraft());
  vi.mocked(core.discardDraft).mockResolvedValue(true);
});

describe("drafting request ownership", () => {
  it("ignores a late local result after A/B/A edits and keeps the physical slot", async () => {
    const pending = deferred<DraftValue>();
    vi.mocked(core.draftReply).mockReturnValue(pending.promise);
    const view = await page();
    await view.user.click(view.local);
    fireEvent.change(view.input, { target: { value: B } });
    fireEvent.change(view.input, { target: { value: A } });
    expect(view.local).toBeDisabled();
    expect(view.endpoint).toBeDisabled();
    await act(async () => { pending.resolve(aTemplateDraft()); });
    expect(screen.queryByRole("heading", { name: "草稿" })).toBeNull();
    expect(view.input).toHaveValue(A);
    expect(view.local).toBeEnabled();
    expect(core.draftReply).toHaveBeenCalledExactlyOnceWith(A);
  });

  it("does not display a stale preparation or its original exemption", async () => {
    const pending = deferred<E1DraftPlan>();
    vi.mocked(core.prepareDraft).mockReturnValueOnce(pending.promise);
    const view = await page();
    await view.user.click(view.endpoint);
    fireEvent.change(view.input, { target: { value: B } });
    fireEvent.change(view.input, { target: { value: A } });
    await act(async () => { pending.resolve(anE1Plan()); });
    expect(screen.queryByRole("button", { name: "这一条按原文带上" })).toBeNull();
    expect(screen.queryByRole("button", { name: "确认，开始生成" })).toBeNull();
    await view.user.click(view.endpoint);
    await screen.findByRole("heading", { name: "确认这一次要生成什么" });
    expect(core.prepareDraft).toHaveBeenLastCalledWith(A, false);
  });

  it("invalidates approval synchronously on edit and resets original exemption", async () => {
    const view = await prepared();
    const carry = screen.getByRole("button", { name: "这一条按原文带上" });
    const approve = screen.getByRole("button", { name: "确认，开始生成" });
    fireEvent.change(view.input, { target: { value: B } });
    fireEvent.change(view.input, { target: { value: A } });
    fireEvent.click(carry);
    fireEvent.click(approve);
    await act(async () => {});
    expect(core.prepareDraft).toHaveBeenCalledTimes(1);
    expect(core.generateDraft).not.toHaveBeenCalled();
    expect(screen.queryByRole("heading", { name: "确认这一次要生成什么" })).toBeNull();
    await view.user.click(view.endpoint);
    await screen.findByRole("heading", { name: "确认这一次要生成什么" });
    expect(core.prepareDraft).toHaveBeenLastCalledWith(A, false);
  });

  it("rejects synchronous double admission before the disabled render", async () => {
    const pending = deferred<E1DraftPlan>();
    vi.mocked(core.prepareDraft).mockReturnValue(pending.promise);
    const view = await page();
    act(() => { view.endpoint.click(); view.endpoint.click(); view.local.click(); });
    await waitFor(() => expect(core.prepareDraft).toHaveBeenCalledTimes(1));
    expect(core.draftReply).not.toHaveBeenCalled();
    await act(async () => { pending.resolve(anE1Plan()); });
  });

  it("starts no IPC when synchronous admission rejects the action", async () => {
    const view = await page();
    vi.spyOn(DraftRequests.prototype, "begin").mockReturnValueOnce(null);
    await view.user.click(view.endpoint);
    expect(core.prepareDraft).not.toHaveBeenCalled();
    expect(view.endpoint).toBeEnabled();
  });

  it("checks input ownership again before the original exemption reaches IPC", async () => {
    const view = await prepared();
    const carry = screen.getByRole("button", { name: "这一条按原文带上" });
    act(() => {
      carry.click();
      fireEvent.change(view.input, { target: { value: B } });
    });
    await act(async () => {});
    expect(core.prepareDraft).toHaveBeenCalledTimes(1);
    expect(view.endpoint).toBeEnabled();
    expect(screen.queryByRole("heading", { name: "确认这一次要生成什么" })).toBeNull();
  });

  it("checks ownership again before approval reaches IPC", async () => {
    const view = await prepared();
    const approve = screen.getByRole("button", { name: "确认，开始生成" });
    act(() => {
      approve.click();
      fireEvent.change(view.input, { target: { value: B } });
    });
    await act(async () => {});
    expect(core.generateDraft).not.toHaveBeenCalled();
    expect(view.endpoint).toBeEnabled();
  });

  it("a late generation cannot replace a revised input, and approval is single use", async () => {
    const pending = deferred<DraftValue>();
    vi.mocked(core.generateDraft).mockReturnValue(pending.promise);
    const view = await prepared();
    const approve = screen.getByRole("button", { name: "确认，开始生成" });
    act(() => { approve.click(); approve.click(); });
    await waitFor(() => expect(core.generateDraft).toHaveBeenCalledTimes(1));
    fireEvent.change(view.input, { target: { value: B } });
    await act(async () => { pending.resolve(aTemplateDraft()); });
    expect(screen.queryByRole("heading", { name: "草稿" })).toBeNull();
    expect(view.input).toHaveValue(B);
    expect(view.endpoint).toBeEnabled();
  });

  it("waits for a started discard before allowing a new preparation", async () => {
    const pending = deferred<boolean>();
    vi.mocked(core.discardDraft).mockReturnValue(pending.promise);
    const view = await prepared();
    await view.user.click(screen.getByRole("button", { name: "不了，丢掉这次准备" }));
    fireEvent.change(view.input, { target: { value: B } });
    expect(view.endpoint).toBeDisabled();
    fireEvent.click(view.endpoint);
    expect(core.prepareDraft).toHaveBeenCalledTimes(1);
    await act(async () => { pending.resolve(true); });
    await view.user.click(view.endpoint);
    await screen.findByRole("heading", { name: "确认这一次要生成什么" });
    expect(core.prepareDraft).toHaveBeenLastCalledWith(B, false);
    expect(core.discardDraft).toHaveBeenCalledTimes(1);
  });

  it("a remounted page also waits for an old discard, without adopting its result", async () => {
    const pending = deferred<boolean>();
    vi.mocked(core.discardDraft).mockReturnValue(pending.promise);
    const old = await prepared();
    await old.user.click(screen.getByRole("button", { name: "不了，丢掉这次准备" }));
    old.unmount();
    const current = await page();
    expect(current.endpoint).toBeDisabled();
    await act(async () => { pending.resolve(true); });
    expect(current.endpoint).toBeEnabled();
    expect(screen.queryByRole("heading", { name: "草稿" })).toBeNull();
    await current.user.click(current.endpoint);
    await screen.findByRole("heading", { name: "确认这一次要生成什么" });
    expect(core.prepareDraft).toHaveBeenCalledTimes(2);
  });

  it("unmount before IPC prevents the old callback from starting any request", async () => {
    const old = await page();
    act(() => { old.endpoint.click(); old.unmount(); });
    await act(async () => {});
    expect(core.prepareDraft).not.toHaveBeenCalled();
    const current = await page();
    expect(current.endpoint).toBeEnabled();
  });

  it("a late refusal cannot overwrite the new input", async () => {
    const pending = deferred<E1DraftPlan>();
    vi.mocked(core.prepareDraft).mockReturnValue(pending.promise);
    const view = await page();
    await view.user.click(view.endpoint);
    fireEvent.change(view.input, { target: { value: B } });
    await act(async () => { pending.reject({ reason_code: "NO", explanation: "合成旧拒绝" }); });
    expect(screen.queryByTestId("refusal-code")).toBeNull();
    expect(view.input).toHaveValue(B);
    expect(view.endpoint).toBeEnabled();
  });

  it("source changes invalidate displayed approval without changing the pasted text", async () => {
    const view = await prepared();
    act(sourceChanged);
    expect(screen.queryByRole("button", { name: "确认，开始生成" })).toBeNull();
    expect(view.input).toHaveValue(A);
    await view.user.click(view.endpoint);
    await screen.findByRole("heading", { name: "确认这一次要生成什么" });
    expect(core.prepareDraft).toHaveBeenLastCalledWith(A, false);
  });

  it("source changes remove a displayed result and reject the callback/UI microtask gap", async () => {
    const view = await page();
    await view.user.click(view.local);
    await screen.findByRole("heading", { name: "草稿" });
    act(sourceChanged);
    expect(screen.queryByRole("heading", { name: "草稿" })).toBeNull();
    const pending = deferred<DraftValue>();
    vi.mocked(core.draftReply).mockReturnValue(pending.promise);
    await view.user.click(view.local);
    await act(async () => {
      pending.resolve(aTemplateDraft());
      sourceChanged();
    });
    expect(screen.queryByRole("heading", { name: "草稿" })).toBeNull();
    expect(view.local).toBeEnabled();
  });

  it("a source-change rejection is not an unknown IPC outcome", async () => {
    const pending = deferred<E1DraftPlan>();
    vi.mocked(core.prepareDraft).mockReturnValue(pending.promise);
    const view = await page();
    await view.user.click(view.endpoint);
    act(sourceChanged);
    await act(async () => { pending.reject(new core.DraftSourceChangedError()); });
    expect(screen.queryByTestId("draft-recovery")).toBeNull();
    expect(view.endpoint).toBeEnabled();
  });

});
