import { StrictMode } from "react";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import { Draft } from "./Draft";
import { aTemplateDraft, installFakeCore, type FakeCoreOptions } from "../test/fakeCore";

const PERSON = "11111111-1111-4111-8111-111111111111";
const OTHER = "22222222-2222-4222-8222-222222222222";
const B = "合成范围编辑 B，只留本地";
const SUCCESS = "已复制到剪贴板。";

function deferred() {
  let resolve!: () => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<void>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

async function open(options: FakeCoreOptions = {}) {
  const core = installFakeCore({
    scopedRecipients: [{ person_id: PERSON, identifier_hint: "synthetic-person" }, { person_id: OTHER, identifier_hint: "synthetic-other" }],
    scopedDrafting: (_pasted, person_id, occasion) => ({ result_id: "synthetic-result", person_id, occasion,
      draft: aTemplateDraft({ text: "合成范围草稿 A" }), applied_preference: null }),
    ...options,
  });
  const user = userEvent.setup();
  const view = render(<StrictMode><Draft /></StrictMode>);
  await user.click(screen.getByRole("button", { name: "按联系人和场合本地起草" }));
  await user.selectOptions(await screen.findByLabelText("收件人物"), PERSON);
  await user.selectOptions(screen.getByLabelText("场合"), "personal");
  fireEvent.change(screen.getByLabelText("原文"), { target: { value: "合成收到的消息" } });
  await user.click(screen.getByRole("button", { name: "写一版本地草稿" }));
  await screen.findByLabelText("你可以直接改");
  const writeText = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined);
  vi.spyOn(navigator, "clipboard", "get").mockReturnValue({ writeText } as unknown as Clipboard);
  return { core, user, writeText, ...view };
}

function edit() {
  fireEvent.change(screen.getByLabelText("你可以直接改"), { target: { value: B } });
}

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

describe("范围草稿复制的当前归属与恢复", () => {
  it("成功后编辑清除提示，重试只复制当前编辑，不把编辑交给core", async () => {
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

  it("编辑后旧成功不能重新显示成功", async () => {
    const { user, writeText } = await open();
    const old = deferred();
    writeText.mockReturnValueOnce(old.promise);
    await user.click(screen.getByRole("button", { name: "复制" }));
    edit();
    await act(async () => { old.resolve(); });
    expect(screen.queryByText(SUCCESS)).toBeNull();
  });

  it("偏好撤销回执才应用编辑时，等待期间启动的复制也随实际编辑失效", async () => {
    let finishDiscard!: (value: boolean) => void;
    const discarding = new Promise<boolean>((done) => { finishDiscard = done; });
    const proposal = { proposal_id: "synthetic-proposal", result_id: "synthetic-result", edit_revision: 0,
      person_id: PERSON, occasion: "personal" as const, register: "formal" as const,
      expected_revision: null, notice: "合成偏好预览" };
    const { user, writeText } = await open({ proposingScopedRegister: () => proposal, discardingScopedRegister: () => discarding });
    await user.selectOptions(screen.getByLabelText("我的语域选择"), "formal");
    await user.click(screen.getByRole("button", { name: "保存我的表达偏好" }));
    await screen.findByText(proposal.notice);
    edit();
    expect(screen.getByLabelText("你可以直接改")).not.toHaveValue(B);
    const old = deferred();
    writeText.mockReturnValueOnce(old.promise);
    await user.click(screen.getByRole("button", { name: "复制" }));
    await act(async () => { finishDiscard(true); });
    expect(screen.getByLabelText("你可以直接改")).toHaveValue(B);
    await act(async () => { old.resolve(); });
    expect(screen.queryByText(SUCCESS)).toBeNull();
  });

  it.each(["person", "occasion", "draft"] as const)("%s 改变并得到新结果后，旧成功不能污染结果", async (change) => {
    const { user, writeText } = await open();
    const old = deferred();
    writeText.mockReturnValueOnce(old.promise);
    await user.click(screen.getByRole("button", { name: "复制" }));
    if (change === "person") await user.selectOptions(screen.getByLabelText("收件人物"), OTHER);
    if (change === "occasion") await user.selectOptions(screen.getByLabelText("场合"), "work");
    await user.click(screen.getByRole("button", { name: "写一版本地草稿" }));
    await screen.findByLabelText("你可以直接改");
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

  it.each(["missing", "reject", "throw"] as const)("clipboard %s 提示手动复制且重试可恢复", async (failure) => {
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

  it("离开范围分支后旧拒绝被处理，重新起草无旧提示", async () => {
    const { user, writeText } = await open();
    const old = deferred();
    writeText.mockReturnValueOnce(old.promise);
    await user.click(screen.getByRole("button", { name: "复制" }));
    await user.click(screen.getByRole("button", { name: "返回普通起草" }));
    await act(async () => { old.reject(new Error("late synthetic refusal")); });
    expect(screen.queryByText(SUCCESS)).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
