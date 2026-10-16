/** A session-level unknown outcome persists for this entire WebView lifetime. */
import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import * as core from "../core";
import { anE1Plan, aTemplateDraft } from "../test/fakeCore";
import { Draft } from "./Draft";

vi.mock("../core", () => ({
  discardDraft: vi.fn(), draftNotices: vi.fn(), draftReply: vi.fn(),
  generateDraft: vi.fn(), prepareDraft: vi.fn(),
  getDraftSourceRevision: vi.fn(), subscribeDraftSource: vi.fn(),
  DraftSourceChangedError: class DraftSourceChangedError extends Error {},
}));

it("unknown discard blocks endpoint reuse across remount without resending, while local drafts remain available", async () => {
  const revision = Symbol();
  const listeners = new Set<(value: symbol) => void>();
  vi.mocked(core.getDraftSourceRevision).mockReturnValue(revision);
  vi.mocked(core.subscribeDraftSource).mockImplementation((listener) => {
    listeners.add(listener);
    listener(revision);
    return () => { listeners.delete(listener); };
  });
  vi.mocked(core.draftNotices).mockResolvedValue({ not_sent: "合成说明", e1_plan: "合成计划", can_send: false });
  vi.mocked(core.prepareDraft).mockResolvedValue(anE1Plan());
  vi.mocked(core.draftReply).mockResolvedValue(aTemplateDraft());
  let reject!: (reason: unknown) => void;
  vi.mocked(core.discardDraft).mockImplementation(() => {
    // The real wrapper invalidates sources before invoking the discard IPC.
    listeners.forEach((listener) => listener(Symbol()));
    return new Promise<boolean>((_resolve, fail) => { reject = fail; });
  });
  const user = userEvent.setup();
  const old = render(<Draft />);
  fireEvent.change(await screen.findByLabelText("原文"), { target: { value: "合成原文" } });
  await user.click(screen.getByRole("button", { name: "用你自己的模型端点写" }));
  await user.click(await screen.findByRole("button", { name: "不了，丢掉这次准备" }));
  await act(async () => { reject(new Error("synthetic lost IPC reply")); });
  expect(await screen.findByTestId("draft-recovery")).toHaveTextContent("不会自动重试");
  expect(screen.getByRole("button", { name: "用你自己的模型端点写" })).toBeDisabled();
  old.unmount();
  render(<Draft />);
  fireEvent.change(await screen.findByLabelText("原文"), { target: { value: "合成原文" } });
  expect(screen.getByRole("button", { name: "用你自己的模型端点写" })).toBeDisabled();
  expect(screen.getByTestId("draft-recovery")).toHaveTextContent("完全退出并重新打开应用");
  const local = screen.getByRole("button", { name: "写一版草稿" });
  expect(local).toBeEnabled();
  await user.click(local);
  await screen.findByRole("heading", { name: "草稿" });
  expect(core.prepareDraft).toHaveBeenCalledTimes(1);
  expect(core.discardDraft).toHaveBeenCalledTimes(1);
  expect(core.generateDraft).not.toHaveBeenCalled();
});
