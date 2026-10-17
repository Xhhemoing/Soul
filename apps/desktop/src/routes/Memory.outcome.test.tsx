/** Current and legacy IPC outcomes, tested through the real Memory component. */
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as core from "../core";
import type { ForgetReceipt, MemoryList } from "../core";
import { Memory } from "./Memory";

vi.mock("../core", () => ({
  createMemory: vi.fn(), forgetMemory: vi.fn(), memoryDetail: vi.fn(), memoryList: vi.fn(),
  previewForget: vi.fn(), retryForgetCleanup: vi.fn(), updateMemory: vi.fn(),
}));

const id = "0192f000-0000-7000-8000-0000000000a1";
const receipt: ForgetReceipt = {
  memory_id: id, content_keys_destroyed: 1, sealed_blobs_destroyed: 2,
  inferences_orphaned: 1, matched_preview: true, logical_committed: true,
  cleanup: { state: "complete" }, audit: "recorded",
};
const list: MemoryList = {
  memories: [{ memory_id: id, memory_type: "episodic", forget_state: "active",
    third_party_content_present: false, title_chars: 4, summary_chars: 8 }],
  memory_types: ["episodic"], forget_notice: "合成测试说明",
};
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}
async function prepared() {
  const mounted = render(<Memory />);
  const user = userEvent.setup();
  await user.click(await screen.findByRole("button", { name: "看遗忘会影响什么" }));
  await screen.findByTestId("forget-preview");
  return { user, ...mounted };
}
async function confirm(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole("button", { name: "就按上面这些，遗忘它" }));
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(core.memoryList).mockResolvedValue(list);
  vi.mocked(core.previewForget).mockResolvedValue({
    preview_id: "synthetic-preview", memory_id: id, content_key_count: 1,
    memories_affected: 1, contacts_affected: 0, sealed_blobs_destroyed: 2,
    inferences_orphaned: 1, audit_entries_retained: 0, destroys_anything: false,
    notice: "合成测试预览",
  });
  vi.mocked(core.forgetMemory).mockResolvedValue(receipt);
  vi.mocked(core.retryForgetCleanup).mockResolvedValue({ cleanup: { state: "complete" } });
});

describe("遗忘结果的桌面接线", () => {
  it("shows all three observations from the current core", async () => {
    const { user } = await prepared(); await confirm(user);
    expect(await screen.findByTestId("forget-status")).toHaveTextContent("清理与审计均已确认");
    expect(screen.getByTestId("forget-commit")).toHaveTextContent("已提交");
    expect(screen.queryByTestId("forget-wal-warning")).toBeNull();
  });
  it("cleanup-only retry preserves counts and unconfirmed audit", async () => {
    vi.mocked(core.forgetMemory).mockResolvedValue({ ...receipt,
      cleanup: { state: "pending", checkpoint: null }, audit: "unconfirmed" });
    const { user } = await prepared(); await confirm(user);
    expect(await screen.findByTestId("forget-wal-warning")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "仅重试日志清理" }));
    expect(await screen.findByTestId("cleanup-result")).toHaveTextContent("本次日志清理已确认");
    expect(screen.getByTestId("forget-audit")).toHaveTextContent("未确认");
    expect(screen.getByTestId("forget-receipt")).toHaveTextContent("销毁了 1 把内容密钥");
    expect(core.forgetMemory).toHaveBeenCalledTimes(1);
    expect(core.retryForgetCleanup).toHaveBeenCalledTimes(1);
    expect(core.retryForgetCleanup).toHaveBeenCalledWith();
  });
  it("audit uncertainty does not return to the destruction confirmation", async () => {
    vi.mocked(core.forgetMemory).mockResolvedValue({ ...receipt, audit: "unconfirmed" });
    const { user } = await prepared(); await confirm(user);
    expect(await screen.findByTestId("forget-audit-warning")).toHaveTextContent("未确认不等于未写入");
    expect(screen.queryByTestId("forget-preview")).toBeNull();
  });
  it("a lost IPC reply clears destructive retry and reports uncertainty", async () => {
    vi.mocked(core.forgetMemory).mockRejectedValue(new Error("synthetic lost reply"));
    const { user } = await prepared(); await confirm(user);
    expect(await screen.findByTestId("forget-uncertain")).toBeVisible();
    expect(screen.queryByTestId("forget-preview")).toBeNull();
    expect(core.forgetMemory).toHaveBeenCalledTimes(1);
  });
  it("an explicit preview mismatch keeps the same confirmation", async () => {
    vi.mocked(core.forgetMemory).mockRejectedValue({ reason_code: "PLAN_HASH_MISMATCH", explanation: "synthetic refusal" });
    const { user } = await prepared(); await confirm(user);
    await screen.findByTestId("memory-refusal-code");
    expect(screen.getByTestId("forget-preview")).toBeVisible();
    expect(screen.queryByTestId("forget-uncertain")).toBeNull();
  });
  it("does not send another confirmation while the first is pending", async () => {
    const pending = deferred<ForgetReceipt>(); vi.mocked(core.forgetMemory).mockReturnValue(pending.promise);
    await prepared(); const button = screen.getByRole("button", { name: "就按上面这些，遗忘它" });
    fireEvent.click(button); fireEvent.click(button);
    await waitFor(() => expect(core.forgetMemory).toHaveBeenCalledTimes(1));
    await act(async () => { pending.resolve(receipt); });
  });
  it("an unmounted response cannot refresh or update the route", async () => {
    const pending = deferred<ForgetReceipt>(); vi.mocked(core.forgetMemory).mockReturnValue(pending.promise);
    const { user, unmount } = await prepared(); await confirm(user);
    const reads = vi.mocked(core.memoryList).mock.calls.length; unmount();
    await act(async () => { pending.resolve(receipt); });
    expect(core.memoryList).toHaveBeenCalledTimes(reads);
  });
  it("a legacy receipt is not upgraded by a successful cleanup", async () => {
    vi.mocked(core.forgetMemory).mockResolvedValue({ memory_id: id, content_keys_destroyed: 1,
      sealed_blobs_destroyed: 2, inferences_orphaned: 1, matched_preview: true });
    const { user } = await prepared(); await confirm(user);
    await screen.findByTestId("forget-uncertain");
    await user.click(screen.getByRole("button", { name: "仅重试日志清理" }));
    await screen.findByTestId("cleanup-result");
    expect(screen.getByTestId("forget-commit")).toHaveTextContent("未确认");
    expect(screen.getByTestId("forget-status")).toHaveTextContent("尚未确认");
  });
  it("a cleanup error retains the original receipt and allows only cleanup retry", async () => {
    vi.mocked(core.forgetMemory).mockResolvedValue({ ...receipt, cleanup: { state: "pending", checkpoint: null } });
    vi.mocked(core.retryForgetCleanup).mockRejectedValue(new Error("synthetic cleanup failure"));
    const { user } = await prepared(); await confirm(user); await screen.findByTestId("forget-receipt");
    await user.click(screen.getByRole("button", { name: "仅重试日志清理" }));
    await screen.findByTestId("memory-refusal-code");
    expect(screen.getByTestId("forget-receipt")).toBeVisible();
    expect(core.forgetMemory).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("button", { name: "仅重试日志清理" })).toBeEnabled();
  });
  it("a restarted tombstone offers cleanup without another preview", async () => {
    vi.mocked(core.memoryList).mockResolvedValue({ ...list,
      memories: list.memories.map((row) => ({ ...row, forget_state: "forgotten" })) });
    render(<Memory />); const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: "仅重试日志清理" }));
    await screen.findByTestId("cleanup-result");
    expect(core.previewForget).not.toHaveBeenCalled();
    expect(core.forgetMemory).not.toHaveBeenCalled();
  });
});
