/** Current immutable protocol, physical ownership across route lifetimes. */
import { StrictMode } from "react";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ImportOperationView, PreparedImport } from "../core";
import { aPreparedImport, anImportOperation, anImportOperationView, installFakeCore } from "../test/fakeCore";
import { Import } from "./Import";
import { DurableImport } from "./durableImport";
function deferred<T>() { let resolve!: (value:T)=>void; const promise=new Promise<T>((yes)=>{resolve=yes;});return {promise,resolve}; }
function file():File { const value=new File(["SYNTHETIC"],"test.jsonl");Object.defineProperty(value,"text",{value:vi.fn().mockResolvedValue("SYNTHETIC")});return value; }
async function stage():Promise<void> { fireEvent.change(screen.getByLabelText("选择导出文件"),{target:{files:[file()]}});await screen.findByRole("heading",{name:"确认这份不可变导入准备"}); }
function committed():ImportOperationView { return anImportOperationView({operation:anImportOperation({state:"committed",receipt:anImportOperation().preview,audit_state:"recorded"}),prepared_here:false}); }
beforeEach(()=>{localStorage.clear();});
describe("import current confirmation lifecycle",()=>{
 it("StrictMode与同一callback双击只准入一个confirm",async()=>{
  const pending=deferred<ImportOperationView>();const core=installFakeCore({preparingImport:()=>aPreparedImport(),confirmingImport:()=>pending.promise});
  const lane=new DurableImport();render(<StrictMode><Import lane={lane}/></StrictMode>);await stage();
  const button=screen.getByRole("button",{name:"确认导入这份准备"});
  act(()=>{fireEvent.click(button);fireEvent.click(button);});
  await waitFor(()=>expect(core.callsTo("confirm_import")).toHaveLength(1));expect(lane.snapshot().busy).toBe(true);
  await act(async()=>{pending.resolve(committed());await pending.promise;});
  expect(await screen.findByText(/已提交：新增事件/)).toBeVisible();expect(core.callsTo("confirm_import")).toHaveLength(1);
 });
 it("切页不释放confirm任务，新挂载收到共享lane回执",async()=>{
  const pending=deferred<ImportOperationView>();const core=installFakeCore({preparingImport:()=>aPreparedImport(),confirmingImport:()=>pending.promise});
  const user=userEvent.setup();const lane=new DurableImport();const old=render(<Import lane={lane}/>);await stage();
  await user.click(screen.getByRole("button",{name:"确认导入这份准备"}));old.unmount();render(<Import lane={lane}/>);
  expect(screen.getByLabelText("选择导出文件")).toBeDisabled();expect(core.callsTo("confirm_import")).toHaveLength(1);
  await act(async()=>{pending.resolve(committed());await pending.promise;});
  expect(await screen.findByText(/已提交：新增事件/)).toBeVisible();expect(screen.getByLabelText("选择导出文件")).toBeEnabled();
 });
 it("准备在路由离开后返回不把旧确认面板交给新挂载",async()=>{
  const pending=deferred<PreparedImport>();const core=installFakeCore({preparingImport:()=>pending.promise});
  const lane=new DurableImport();const old=render(<Import lane={lane}/>);
  fireEvent.change(screen.getByLabelText("选择导出文件"),{target:{files:[file()]}});
  await waitFor(()=>expect(core.callsTo("prepare_import")).toHaveLength(1));old.unmount();render(<Import lane={lane}/>);
  await act(async()=>{pending.resolve(aPreparedImport());await pending.promise;});
  expect(screen.queryByRole("button",{name:"确认导入这份准备"})).toBeNull();expect(core.callsTo("confirm_import")).toHaveLength(0);
 });
 it("discard物理调用期间仍阻止新prepare和第二次discard",async()=>{
  const pending=deferred<ImportOperationView>();const core=installFakeCore({preparingImport:()=>aPreparedImport(),discardingImport:()=>pending.promise});
  const lane=new DurableImport();render(<Import lane={lane}/>);await stage();const button=screen.getByRole("button",{name:"丢弃这份准备"});
  act(()=>{fireEvent.click(button);fireEvent.click(button);});await waitFor(()=>expect(core.callsTo("discard_import")).toHaveLength(1));
  expect(screen.getByLabelText("选择导出文件")).toBeDisabled();
  await act(async()=>{pending.resolve(anImportOperationView({operation:anImportOperation({state:"discarded"})}));await pending.promise;});
  expect(screen.getByLabelText("选择导出文件")).toBeEnabled();expect(core.callsTo("confirm_import")).toHaveLength(0);
 });
 it("确定的核心拒绝清旧准备但不显示成功，也不自动重试",async()=>{
  const core=installFakeCore({preparingImport:()=>aPreparedImport(),confirmingImport:()=>{throw {reason_code:"IMPORT_SOURCE_CHANGED",explanation:"合成资料已变化"};}});
  const user=userEvent.setup();render(<Import lane={new DurableImport()}/>);await stage();
  await user.click(screen.getByRole("button",{name:"确认导入这份准备"}));expect(await screen.findByText("合成资料已变化")).toBeVisible();
  expect(screen.queryByText(/已提交：新增事件/)).toBeNull();expect(core.callsTo("confirm_import")).toHaveLength(1);
  expect(screen.queryByRole("button",{name:"确认导入这份准备"})).toBeNull();
 });
});
