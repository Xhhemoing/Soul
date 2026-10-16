/** Current route and real core.ts IPC; only the native far side is synthetic. */
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { IMPORT_OVER_BYTE_BUDGET_NOTICE, MAX_IMPORT_BYTES } from "../core";
import { renderedText } from "../test/denylist";
import { aPreparedImport, anImportOperation, anImportOperationView, forbidNetwork,
  IMPORT_NAMESPACE_ID, IMPORT_OPERATION_ID, IMPORT_PREVIEW_ID, installFakeCore } from "../test/fakeCore";
import { Import } from "./Import";
import { DurableImport } from "./durableImport";

const TEXT = "合成正文：明天上午十点在公司门口见。u-private-user";
function file(text = TEXT): File {
  const value = new File([text], "synthetic.jsonl", { type: "application/json" });
  Object.defineProperty(value, "text", { value: vi.fn().mockResolvedValue(text) }); return value;
}
async function pick(value = file()): Promise<void> {
  fireEvent.change(screen.getByLabelText("选择导出文件"), { target: { files: [value] } });
  await screen.findByRole("heading", { name: "确认这份不可变导入准备" });
}
beforeEach(() => { localStorage.clear(); });

describe("不可变导入准备与持久回执", () => {
  it("说明明确格式和只读边界，没有登录、归档解包或网络动作", () => {
    const attempts = forbidNetwork(); installFakeCore(); render(<Import lane={new DurableImport()} />);
    expect(screen.getByRole("option", { name: "soul-import-v1 JSONL" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "Telegram Desktop result.json" })).toBeInTheDocument();
    expect(screen.getByLabelText("选择导出文件")).toHaveAttribute("type", "file");
    for (const button of screen.getAllByRole("button")) expect(button.textContent).not.toMatch(/登录|授权账号|解压|下载|上传|执行/);
    expect(renderedText()).toContain("不打开归档、不登录账号、不自动出网"); expect(attempts).toEqual([]);
  });
  it("只有prepare传全文，屏幕只展示计数而不展示正文或源用户ID", async () => {
    const core = installFakeCore(); render(<Import lane={new DurableImport()} />); await pick();
    expect(core.callsTo("prepare_import")[0]?.payload).toEqual({ request: {
      text: TEXT, source: "soul-import-v1", namespace_id: null, legacy_choice: "reject" } });
    expect(renderedText()).toContain("文件包含 16 条消息");
    expect(renderedText()).not.toContain("明天上午十点"); expect(renderedText()).not.toContain("u-private-user");
    expect(core.callsTo("preview_soul_import_v1")).toHaveLength(0); expect(core.callsTo("commit_soul_import_v1")).toHaveLength(0);
  });
  it("格式、明确namespace及forward-only选择绑定同一份准备", async () => {
    const core = installFakeCore({ importNamespaces: [{ namespace_id: IMPORT_NAMESPACE_ID, source: "telegram-desktop", state: "active", coverage: "forward_only" }] });
    const user = userEvent.setup(); render(<Import lane={new DurableImport()} />);
    await user.selectOptions(screen.getByLabelText("导入格式"), "telegram-desktop");
    await screen.findByRole("option", { name: new RegExp(IMPORT_NAMESPACE_ID) });
    await user.selectOptions(screen.getByLabelText("来源身份"), IMPORT_NAMESPACE_ID);
    await user.click(screen.getByRole("checkbox")); await pick();
    expect(core.callsTo("prepare_import")[0]?.payload).toEqual({ request: {
      text: TEXT, source: "telegram-desktop", namespace_id: IMPORT_NAMESPACE_ID, legacy_choice: "forward_only" } });
    expect(renderedText()).toContain("历史可能重叠");
  });
  it("文件超限先于读取与IPC拒绝，允许用户另选合规文件", async () => {
    const core = installFakeCore(); render(<Import lane={new DurableImport()} />);
    const oversized = file(); Object.defineProperty(oversized, "size", { value: MAX_IMPORT_BYTES + 1 });
    fireEvent.change(screen.getByLabelText("选择导出文件"), { target: { files: [oversized] } });
    expect(await screen.findByText(IMPORT_OVER_BYTE_BUDGET_NOTICE)).toBeVisible();
    expect(oversized.text).not.toHaveBeenCalled(); expect(core.callsTo("prepare_import")).toHaveLength(0);
    await pick(); expect(core.callsTo("prepare_import")).toHaveLength(1);
  });
  it("准备后禁换全部输入，确认只交两个编号，回执准确解释关系重算", async () => {
    const core = installFakeCore({ preparingImport: () => aPreparedImport() });
    const user = userEvent.setup(); render(<Import lane={new DurableImport()} />); await pick();
    for (const label of ["导入格式", "来源身份", "选择导出文件"]) expect(screen.getByLabelText(label)).toBeDisabled();
    expect(screen.getByRole("checkbox")).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "确认导入这份准备" }));
    expect(await screen.findByText(/已提交：新增事件 16 条/)).toBeVisible();
    expect(core.callsTo("confirm_import")[0]?.payload).toEqual({ operationId: IMPORT_OPERATION_ID, previewId: IMPORT_PREVIEW_ID });
    expect(JSON.stringify(core.callsTo("confirm_import"))).not.toContain(TEXT);
    expect(renderedText()).toContain("包括已有关系，不是新增关系数");
    expect(screen.getByRole("link", { name: "查看图谱、证据与纠正" })).toHaveAttribute("href", "#/graph");
    expect(screen.getByRole("link", { name: "核对本机审计" })).toHaveAttribute("href", "#/audit");
  });
  it("丢弃只提交operation ID，然后才能建立新意图", async () => {
    const core = installFakeCore({ preparingImport: () => aPreparedImport() });
    const user = userEvent.setup(); render(<Import lane={new DurableImport()} />); await pick();
    await user.click(screen.getByRole("button", { name: "丢弃这份准备" }));
    await waitFor(() => expect(screen.queryByRole("heading", { name: "确认这份不可变导入准备" })).toBeNull());
    expect(core.callsTo("discard_import")[0]?.payload).toEqual({ operationId: IMPORT_OPERATION_ID });
    expect(core.callsTo("confirm_import")).toHaveLength(0); expect(screen.getByLabelText("选择导出文件")).toBeEnabled();
  });
  it("未识别本人时禁止确认且不猜测owner", async () => {
    const core = installFakeCore({ readingImport: (source) => ({ ...aPreparedImport().preview, source, owner_identified: false }) });
    render(<Import lane={new DurableImport()} />); await pick();
    expect(screen.getByRole("button", { name: "确认导入这份准备" })).toBeDisabled(); expect(core.callsTo("confirm_import")).toHaveLength(0);
  });
  it("来源遗忘后的回执是历史数字，不恢复事实或密钥", async () => {
    const committed = anImportOperationView({ namespace_revoked: true, prepared_here: false,
      operation: anImportOperation({ state: "committed", receipt: aPreparedImport().operation.preview, audit_state: "recorded" }) });
    const core = installFakeCore({ importOperations: [committed] }); const user = userEvent.setup();
    render(<Import lane={new DurableImport()} />); await user.click(screen.getByRole("button", { name: "核对最近记录" }));
    expect(await screen.findByText(/来源已遗忘；上述数字仅为历史回执/)).toBeVisible();
    expect(core.callsTo("confirm_import")).toHaveLength(0); expect(core.callsTo("recent_import_operations")).toHaveLength(1);
  });
  it("Interrupted无成功回执不声称回滚，也不自动重放", async () => {
    const core = installFakeCore({ importOperations: [anImportOperationView({ operation: anImportOperation({ state: "interrupted", error_code: "INTERRUPTED" }) })] });
    const user = userEvent.setup(); render(<Import lane={new DurableImport()} />);
    await user.click(screen.getByRole("button", { name: "核对最近记录" }));
    expect(await screen.findByText("没有成功提交回执，不能据此重放原操作。")).toBeVisible();
    expect(core.callsTo("confirm_import")).toHaveLength(0); expect(renderedText()).not.toContain("已回滚");
  });
  it("forward-only历史回执始终说明覆盖边界，不冒称全历史去重", async () => {
    installFakeCore({ importOperations: [anImportOperationView({ operation: anImportOperation({ legacy_choice: "forward_only", state: "committed", receipt: anImportOperation().preview }) })] });
    const user = userEvent.setup(); render(<Import lane={new DurableImport()} />);
    await user.click(screen.getByRole("button", { name: "核对最近记录" }));
    expect(await screen.findByText("本次及以后建立索引；旧库历史可能重叠，这份回执不代表全历史去重。")).toBeVisible();
  });
});
