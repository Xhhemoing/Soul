import { StrictMode } from "react";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Draft } from "./Draft";
import { updateMemory } from "../core";
import { aTemplateDraft, installFakeCore, type FakeCoreOptions } from "../test/fakeCore";

const PERSON = "11111111-1111-4111-8111-111111111111";
const preference = { preference_id: "pref", revision: "rev-1", memory_id: "0192f000-0000-7000-8000-0000000000a1", register: "casual" as const };
const result = { result_id: "result", person_id: PERSON, occasion: "personal" as const, draft: aTemplateDraft(), applied_preference: preference };
const proposal = { proposal_id: "proposal", result_id: "result", edit_revision: 0, person_id: PERSON, occasion: "personal" as const,
  register: "formal" as const, expected_revision: "rev-1", notice: "这是我对该联系人在私人场合的语域偏好，用于未来同范围本地回复，持续到更改或遗忘；普通 Memory 编辑会停用。" };
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((done) => { resolve = done; }); return { promise, resolve }; }
async function open(options: FakeCoreOptions = {}) {
  const core = installFakeCore({ scopedRecipients: [{ person_id: PERSON, identifier_hint: "synthetic-person" }], scopedDrafting: () => result,
    proposingScopedRegister: () => proposal, confirmingScopedRegister: () => preference, ...options });
  const user = userEvent.setup();
  const view = render(<StrictMode><Draft /></StrictMode>);
  await user.click(screen.getByRole("button", { name: "按联系人和场合本地起草" }));
  await user.selectOptions(await screen.findByLabelText("收件人物"), PERSON);
  await user.selectOptions(screen.getByLabelText("场合"), "personal");
  await user.type(screen.getByLabelText("原文"), "合成收到的消息");
  await user.click(screen.getByRole("button", { name: "写一版本地草稿" }));
  await screen.findByLabelText("你可以直接改");
  return { core, user, ...view };
}

describe("精确范围的本人语域偏好", () => {
  it("显式传递人物与场合，显示应用来源，仅提供本地模板", async () => {
    const { core } = await open();
    expect(core.callsTo("draft_scoped_reply")[0]?.payload).toEqual({ pasted: "合成收到的消息", personId: PERSON, occasion: "personal", correlationId: expect.any(String) });
    expect(screen.getByTestId("scoped-applied")).toHaveTextContent("casual");
    expect(screen.getByRole("link", { name: "查看或遗忘这条偏好" })).toHaveAttribute("href", `#/memory?memory_id=${preference.memory_id}`);
    expect(screen.queryByRole("button", { name: "用你自己的模型端点写" })).toBeNull();
    expect(screen.queryByLabelText("记忆查询")).toBeNull();
  });

  it("编辑只留本地；主动选择语域才提案，确认只传ID和revision", async () => {
    const { core, user } = await open();
    fireEvent.change(screen.getByLabelText("你可以直接改"), { target: { value: "我的合成编辑，不能上传" } });
    expect(core.callsTo("propose_scoped_register")).toHaveLength(0);
    await user.selectOptions(screen.getByLabelText("我的语域选择"), "formal");
    await user.click(screen.getByRole("button", { name: "保存我的表达偏好" }));
    expect(await screen.findByText(proposal.notice)).toBeVisible();
    expect(core.callsTo("propose_scoped_register")[0]?.payload).toEqual({ resultId: "result", editRevision: 1, register: "formal", correlationId: expect.any(String) });
    await user.click(screen.getByRole("button", { name: "确认保存此范围偏好" }));
    expect(await screen.findByTestId("scoped-saved")).toBeVisible();
    expect(core.callsTo("confirm_scoped_register")[0]?.payload).toEqual({ proposalId: "proposal", expectedRevision: "rev-1", correlationId: expect.any(String) });
    expect(JSON.stringify(core.calls)).not.toContain("我的合成编辑，不能上传");
  });

  it("取消预览等待后端撤销，禁止确认，不保存", async () => {
    const pending = deferred<boolean>();
    const { core, user } = await open({ discardingScopedRegister: () => pending.promise });
    await user.selectOptions(screen.getByLabelText("我的语域选择"), "formal");
    await user.click(screen.getByRole("button", { name: "保存我的表达偏好" }));
    await screen.findByText(proposal.notice);
    await user.click(screen.getByRole("button", { name: "取消这次偏好保存" }));
    expect(screen.queryByRole("button", { name: "确认保存此范围偏好" })).toBeNull();
    expect(screen.getByLabelText("你可以直接改")).toBeDisabled();
    await act(async () => { pending.resolve(true); });
    await waitFor(() => expect(screen.getByLabelText("你可以直接改")).toBeEnabled());
    expect(core.callsTo("confirm_scoped_register")).toHaveLength(0);
  });

  it("预览后的编辑先撤销，回执前不应用编辑", async () => {
    const pending = deferred<boolean>();
    const { core, user } = await open({ discardingScopedRegister: () => pending.promise });
    await user.selectOptions(screen.getByLabelText("我的语域选择"), "formal");
    await user.click(screen.getByRole("button", { name: "保存我的表达偏好" }));
    await screen.findByText(proposal.notice);
    const original = (screen.getByLabelText("你可以直接改") as HTMLTextAreaElement).value;
    fireEvent.change(screen.getByLabelText("你可以直接改"), { target: { value: "后续本地编辑" } });
    expect(screen.getByLabelText("你可以直接改")).toHaveValue(original);
    expect(screen.queryByRole("button", { name: "确认保存此范围偏好" })).toBeNull();
    await act(async () => { pending.resolve(true); });
    await waitFor(() => expect(screen.getByLabelText("你可以直接改")).toHaveValue("后续本地编辑"));
    expect(core.callsTo("discard_scoped_register_proposal")).toHaveLength(1);
  });

  it("提案晚到遇到编辑时清理旧ID且不显示预览", async () => {
    const pending = deferred<typeof proposal>();
    const { core, user } = await open({ proposingScopedRegister: () => pending.promise });
    await user.selectOptions(screen.getByLabelText("我的语域选择"), "formal");
    await user.click(screen.getByRole("button", { name: "保存我的表达偏好" }));
    await waitFor(() => expect(core.callsTo("propose_scoped_register")).toHaveLength(1));
    fireEvent.change(screen.getByLabelText("你可以直接改"), { target: { value: "更新后的合成编辑" } });
    await act(async () => { pending.resolve(proposal); });
    await waitFor(() => expect(core.callsTo("discard_scoped_register_proposal")).toHaveLength(1));
    expect(screen.queryByText(proposal.notice)).toBeNull();
  });

  it("确认提交期间锁定编辑和scope，自己的来源变更不会吞掉成功回执", async () => {
    const pending = deferred<typeof preference>();
    const { core, user } = await open({ confirmingScopedRegister: () => pending.promise });
    await user.selectOptions(screen.getByLabelText("我的语域选择"), "formal");
    await user.click(screen.getByRole("button", { name: "保存我的表达偏好" }));
    await screen.findByText(proposal.notice);
    await user.click(screen.getByRole("button", { name: "确认保存此范围偏好" }));
    await waitFor(() => expect(core.callsTo("confirm_scoped_register")).toHaveLength(1));
    expect(screen.getByLabelText("原文")).toBeDisabled();
    expect(screen.getByLabelText("收件人物")).toBeDisabled();
    expect(screen.getByLabelText("场合")).toBeDisabled();
    expect(screen.getByLabelText("你可以直接改")).toBeDisabled();
    await act(async () => { pending.resolve(preference); });
    expect(await screen.findByTestId("scoped-saved")).toBeVisible();
  });

  it("确认回执晚到期间发生另一项Memory编辑，仅保留提交回执并要求复核现行规则", async () => {
    const pending = deferred<typeof preference>();
    const { core, user } = await open({ confirmingScopedRegister: () => pending.promise });
    await user.selectOptions(screen.getByLabelText("我的语域选择"), "formal");
    await user.click(screen.getByRole("button", { name: "保存我的表达偏好" }));
    await screen.findByText(proposal.notice);
    await user.click(screen.getByRole("button", { name: "确认保存此范围偏好" }));
    await waitFor(() => expect(core.callsTo("confirm_scoped_register")).toHaveLength(1));
    await act(async () => { await updateMemory("memory", { summary: "合成的普通编辑" }); });
    await act(async () => { pending.resolve(preference); });
    expect(await screen.findByTestId("scoped-saved")).toBeVisible();
    expect(screen.getByTestId("scoped-confirm-source-changed")).toHaveTextContent("重新起草核对");
    expect(screen.getByTestId("scoped-saved")).not.toHaveTextContent("未来相同人物、相同场合的本地回复会使用");
  });

  it("离开scope分支清理持有状态，普通起草不继续显示已应用", async () => {
    const { core, user } = await open();
    await user.click(screen.getByRole("button", { name: "返回普通起草" }));
    await waitFor(() => expect(core.callsTo("discard_draft")).toHaveLength(2));
    expect(screen.queryByTestId("scoped-applied")).toBeNull();
    expect(screen.getByRole("button", { name: "用你自己的模型端点写" })).toBeVisible();
  });

  it("提案因资料变更失效且取消已获回执时，可以继续本地起草", async () => {
    const pending = deferred<typeof proposal>();
    const { core, user } = await open({ proposingScopedRegister: () => pending.promise });
    await user.selectOptions(screen.getByLabelText("我的语域选择"), "formal");
    await user.click(screen.getByRole("button", { name: "保存我的表达偏好" }));
    await waitFor(() => expect(core.callsTo("propose_scoped_register")).toHaveLength(1));
    await act(async () => { await updateMemory("memory", { summary: "合成资料变更" }); });
    await act(async () => { pending.resolve(proposal); });
    await waitFor(() => expect(core.callsTo("discard_scoped_register_proposal")).toHaveLength(1));
    expect(screen.queryByText(proposal.notice)).toBeNull();
    expect(screen.getByRole("button", { name: "写一版本地草稿" })).toBeEnabled();
  });

  it("取消回执即使是结构化拒绝也保持清理未知，锁住编辑和新起草", async () => {
    const { core, user } = await open({ discardingScopedRegister: async () => {
      throw { reason_code: "synthetic_cancel_refused", explanation: "合成的取消拒绝，不能当作清理回执" };
    } });
    await user.selectOptions(screen.getByLabelText("我的语域选择"), "formal");
    await user.click(screen.getByRole("button", { name: "保存我的表达偏好" }));
    await screen.findByText(proposal.notice);
    const original = (screen.getByLabelText("你可以直接改") as HTMLTextAreaElement).value;
    await user.click(screen.getByRole("button", { name: "取消这次偏好保存" }));
    expect(await screen.findByTestId("scoped-refusal")).toHaveTextContent("unavailable");
    expect(screen.getByText("未能确认上次操作是否完成。请完全退出并重新打开应用后继续；这里不会自动重试。")).toBeVisible();
    expect(screen.queryByRole("button", { name: "确认保存此范围偏好" })).toBeNull();
    expect(screen.getByLabelText("你可以直接改")).toBeDisabled();
    expect(screen.getByLabelText("原文")).toBeDisabled();
    expect(screen.getByRole("button", { name: "写一版本地草稿" })).toBeDisabled();
    fireEvent.change(screen.getByLabelText("你可以直接改"), { target: { value: "不能越过清理失败" } });
    expect(screen.getByLabelText("你可以直接改")).toHaveValue(original);
    expect(core.callsTo("confirm_scoped_register")).toHaveLength(0);
    expect(screen.queryByTestId("scoped-saved")).toBeNull();
  });
});
