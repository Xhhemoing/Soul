/** A draft stays available until its own save is acknowledged. */
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as core from "../core";
import type { MemoryDetail, MemoryList } from "../core";
import { Memory } from "./Memory";
import { MemoryRequests } from "./memoryRequests";

vi.mock("../core", async (importOriginal) => ({
  ...await importOriginal<typeof import("../core")>(),
  createMemory: vi.fn(), forgetMemory: vi.fn(), memoryDetail: vi.fn(), memoryList: vi.fn(),
  previewForget: vi.fn(), retryForgetCleanup: vi.fn(), updateMemory: vi.fn(),
}));

const detail: MemoryDetail = {
  memory_id: "synthetic-memory", memory_type: "episodic", title: "合成标题",
  summary: "合成正文", third_party_content_present: false, content_key_id: "synthetic-key",
};
const list: MemoryList = {
  memories: [], memory_types: ["episodic", "semantic"], forget_notice: "合成测试说明",
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

async function draft() {
  const mounted = render(<Memory />);
  const user = userEvent.setup();
  const form = within(await screen.findByRole("region", { name: "记一条" }));
  const title = form.getByLabelText("一句话说清是什么");
  const summary = form.getByLabelText("再多写几句");
  const kind = form.getByLabelText("这是什么");
  await user.type(title, detail.title);
  await user.type(summary, detail.summary);
  return { ...mounted, user, title, summary, kind, save: form.getByRole("button", { name: "存下来" }) };
}

beforeEach(() => {
  vi.restoreAllMocks();
  vi.resetAllMocks();
  vi.mocked(core.memoryList).mockResolvedValue(list);
  vi.mocked(core.createMemory).mockResolvedValue(detail);
});

describe("记忆草稿的保存结果", () => {
  it.each([
    ["core refusal", { reason_code: "ROUTINE", explanation: "合成拒绝" }],
    ["unknown IPC outcome", new Error("synthetic lost reply")],
  ])("retains the draft after %s without resubmitting", async (_label, error) => {
    vi.mocked(core.createMemory).mockRejectedValue(error);
    const form = await draft();
    await form.user.click(form.save);

    await screen.findByTestId("memory-refusal-code");
    expect(form.title).toHaveValue(detail.title);
    expect(form.summary).toHaveValue(detail.summary);
    expect(form.save).toBeEnabled();
    expect(core.createMemory).toHaveBeenCalledTimes(1);
  });

  it("does not erase a draft when synchronous admission rejects the save", async () => {
    const form = await draft();
    vi.spyOn(MemoryRequests.prototype, "begin").mockReturnValueOnce(null);
    await form.user.click(form.save);

    expect(core.createMemory).not.toHaveBeenCalled();
    expect(form.title).toHaveValue(detail.title);
    expect(form.summary).toHaveValue(detail.summary);
    expect(form.save).toBeEnabled();
  });

  it("clears an unchanged draft only after its successful receipt", async () => {
    const pending = deferred<MemoryDetail>();
    vi.mocked(core.createMemory).mockReturnValue(pending.promise);
    const form = await draft();
    await form.user.click(form.save);

    expect(form.title).toHaveValue(detail.title);
    expect(form.summary).toHaveValue(detail.summary);
    expect(form.save).toBeDisabled();
    expect(core.createMemory).toHaveBeenCalledExactlyOnceWith({
      memory_type: detail.memory_type, title: detail.title, summary: detail.summary,
    });
    await act(async () => { pending.resolve(detail); });

    await waitFor(() => expect(form.title).toHaveValue(""));
    expect(form.summary).toHaveValue("");
    expect(screen.getByRole("region", { name: "打开的这一条" })).toHaveTextContent("同一把内容密钥");
  });

  it.each(["title", "summary", "kind"] as const)(
    "preserves later %s edits when the older save succeeds",
    async (field) => {
      const pending = deferred<MemoryDetail>();
      vi.mocked(core.createMemory).mockReturnValue(pending.promise);
      const form = await draft();
      await form.user.click(form.save);
      const next = field === "kind" ? "semantic" : "新的合成草稿";
      fireEvent.change(form[field], { target: { value: next } });
      await act(async () => { pending.resolve(detail); });

      expect(form.title).toHaveValue(field === "title" ? next : detail.title);
      expect(form.summary).toHaveValue(field === "summary" ? next : detail.summary);
      expect(form.kind).toHaveValue(field === "kind" ? next : detail.memory_type);
      expect(core.createMemory).toHaveBeenCalledExactlyOnceWith({
        memory_type: detail.memory_type, title: detail.title, summary: detail.summary,
      });
      expect(form.save).toBeEnabled();
    },
  );

  it("treats edits back to the original text as a newer draft", async () => {
    const pending = deferred<MemoryDetail>();
    vi.mocked(core.createMemory).mockReturnValue(pending.promise);
    const form = await draft();
    await form.user.click(form.save);
    fireEvent.change(form.title, { target: { value: "临时合成标题" } });
    fireEvent.change(form.title, { target: { value: detail.title } });
    await act(async () => { pending.resolve(detail); });

    expect(form.title).toHaveValue(detail.title);
    expect(form.summary).toHaveValue(detail.summary);
  });

  it("ignores an unmounted save without refreshing or clearing the new form", async () => {
    const pending = deferred<MemoryDetail>();
    vi.mocked(core.createMemory).mockReturnValue(pending.promise);
    const old = await draft();
    await old.user.click(old.save);
    old.unmount();
    const current = await draft();
    fireEvent.change(current.title, { target: { value: "重新打开后的合成草稿" } });
    const reads = vi.mocked(core.memoryList).mock.calls.length;
    await act(async () => { pending.resolve(detail); });

    expect(core.memoryList).toHaveBeenCalledTimes(reads);
    expect(current.title).toHaveValue("重新打开后的合成草稿");
    expect(current.summary).toHaveValue(detail.summary);
    expect(screen.queryByRole("region", { name: "打开的这一条" })).toBeNull();
  });
});
