import { StrictMode } from "react";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import { Draft } from "./Draft";
import { aTemplateDraft, installFakeCore } from "../test/fakeCore";

const A = "合成草稿 A";
const B = "合成编辑 B，只留本地";
const SUCCESS = "已复制到剪贴板。";

function deferred() {
  let resolve!: () => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<void>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

async function open() {
  const core = installFakeCore({ drafting: () => aTemplateDraft({ text: A }) });
  const user = userEvent.setup();
  const view = render(<StrictMode><Draft /></StrictMode>);
  fireEvent.change(await screen.findByLabelText("原文"), { target: { value: "合成收到的消息" } });
  await user.click(screen.getByRole("button", { name: "写一版草稿" }));
  await screen.findByLabelText("你可以直接改");
  const writeText = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined);
  vi.spyOn(navigator, "clipboard", "get").mockReturnValue({ writeText } as unknown as Clipboard);
  return { core, user, writeText, ...view };
}

function edit() {
  fireEvent.change(screen.getByLabelText("你可以直接改"), { target: { value: B } });
}

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

describe("普通草稿复制的当前归属与恢复", () => {
  it("复制成功后编辑清除提示，重试复制当前编辑且不把编辑交给core", async () => {
    const { core, user, writeText } = await open();
    await user.click(screen.getByRole("button", { name: "复制" }));
    expect(screen.getByText(SUCCESS)).toBeVisible();
    edit();
    expect(screen.queryByText(SUCCESS)).toBeNull();
    await user.click(screen.getByRole("button", { name: "复制" }));
    expect(writeText).toHaveBeenLastCalledWith(B);
    expect(screen.getByText(SUCCESS)).toBeVisible();
    expect(JSON.stringify(core.calls)).not.toContain(B);
  });

  it("编辑后旧复制成功不能重新显示成功", async () => {
    const { user, writeText } = await open();
    const old = deferred();
    writeText.mockReturnValueOnce(old.promise);
    await user.click(screen.getByRole("button", { name: "复制" }));
    edit();
    await act(async () => { old.resolve(); });
    expect(screen.queryByText(SUCCESS)).toBeNull();
  });

  it.each(["resolve", "reject"] as const)("旧尝试的%s不能覆盖后一次失败", async (settle) => {
    const { user, writeText } = await open();
    const old = deferred();
    writeText.mockReturnValueOnce(old.promise).mockRejectedValueOnce(new Error("synthetic failure"));
    await user.click(screen.getByRole("button", { name: "复制" }));
    await user.click(screen.getByRole("button", { name: "复制" }));
    expect(screen.getByRole("alert")).toHaveTextContent("手动复制");
    await act(async () => { if (settle === "resolve") old.resolve(); else old.reject(new Error("old synthetic failure")); });
    expect(screen.getByRole("alert")).toHaveTextContent("手动复制");
    expect(screen.queryByText(SUCCESS)).toBeNull();
  });

  it("旧失败不能清除后一次成功", async () => {
    const { user, writeText } = await open();
    const old = deferred();
    writeText.mockReturnValueOnce(old.promise);
    await user.click(screen.getByRole("button", { name: "复制" }));
    await user.click(screen.getByRole("button", { name: "复制" }));
    await act(async () => { old.reject(new Error("old synthetic failure")); });
    expect(screen.getByText(SUCCESS)).toBeVisible();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it.each(["missing", "reject", "throw"] as const)("clipboard %s 时提示手动复制，保留编辑并允许重试", async (failure) => {
    const { user, writeText } = await open();
    edit();
    if (failure === "missing") vi.mocked(Object.getOwnPropertyDescriptor(navigator, "clipboard")!.get!).mockReturnValue(undefined);
    else if (failure === "reject") writeText.mockRejectedValueOnce(new Error("synthetic permission refusal"));
    else writeText.mockImplementationOnce(() => { throw new Error("synthetic synchronous refusal"); });
    await user.click(screen.getByRole("button", { name: "复制" }));
    expect(screen.getByRole("alert")).toHaveTextContent("手动复制");
    expect(screen.getByRole("alert")).not.toHaveTextContent("synthetic");
    expect(screen.getByLabelText("你可以直接改")).toHaveValue(B);
    expect(screen.getByLabelText("你可以直接改")).toBeEnabled();
    vi.mocked(Object.getOwnPropertyDescriptor(navigator, "clipboard")!.get!).mockReturnValue({ writeText } as unknown as Clipboard);
    await user.click(screen.getByRole("button", { name: "复制" }));
    expect(screen.getByText(SUCCESS)).toBeVisible();
    expect(screen.queryByRole("alert")).toBeNull();
    expect(writeText).toHaveBeenLastCalledWith(B);
  });

  it("新草稿替换结果后，旧复制回调不能污染新结果", async () => {
    const { user, writeText } = await open();
    const old = deferred();
    writeText.mockReturnValueOnce(old.promise);
    await user.click(screen.getByRole("button", { name: "复制" }));
    await user.click(screen.getByRole("button", { name: "写一版草稿" }));
    await screen.findByLabelText("你可以直接改");
    await act(async () => { old.resolve(); });
    expect(screen.queryByText(SUCCESS)).toBeNull();
  });

  it("卸载后旧拒绝被处理，重新进入没有旧复制提示", async () => {
    const { user, writeText, unmount } = await open();
    const old = deferred();
    writeText.mockReturnValueOnce(old.promise);
    await user.click(screen.getByRole("button", { name: "复制" }));
    unmount();
    await act(async () => { old.reject(new Error("late synthetic refusal")); });
    render(<Draft />);
    expect(screen.queryByText(SUCCESS)).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
