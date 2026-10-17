/** Reachable browser-read/query races under the new immutable import API. */
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ImportOperationView } from "../core";
import { aPreparedImport, anImportOperation, anImportOperationView, IMPORT_OPERATION_ID, installFakeCore } from "../test/fakeCore";
import { Import } from "./Import";
import { DurableImport } from "./durableImport";
function deferred<T>() {let resolve!:(value:T)=>void;const promise=new Promise<T>((yes)=>{resolve=yes;});return {promise,resolve};}
function file(text:Promise<string>):File {const value=new File([],"synthetic.jsonl");Object.defineProperty(value,"text",{value:vi.fn(()=>text)});return value;}
beforeEach(()=>{localStorage.clear();});
describe("import current read and query races",()=>{
 it("file.text在卸载后返回不发prepare，也不把正文交到新挂载",async()=>{
  const read=deferred<string>();const core=installFakeCore();const lane=new DurableImport();const old=render(<Import lane={lane}/>);
  fireEvent.change(screen.getByLabelText("选择导出文件"),{target:{files:[file(read.promise)]}});old.unmount();render(<Import lane={lane}/>);
  expect(screen.getByLabelText("选择导出文件")).toBeDisabled();
  await act(async()=>{read.resolve("PRIVATE SYNTHETIC BODY");await read.promise;});
  expect(core.callsTo("prepare_import")).toHaveLength(0);expect(core.callsTo("confirm_import")).toHaveLength(0);
  expect(screen.queryByText("PRIVATE SYNTHETIC BODY")).toBeNull();
 });
 it("物理读取中强行再触发file事件也只读取第一份，选择控件保持锁定",async()=>{
  const read=deferred<string>();const first=file(read.promise);const second=file(Promise.resolve("SECOND BODY"));
  const core=installFakeCore();render(<Import lane={new DurableImport()}/>);
  fireEvent.change(screen.getByLabelText("选择导出文件"),{target:{files:[first]}});
  expect(screen.getByLabelText("选择导出文件")).toBeDisabled();
  fireEvent.change(screen.getByLabelText("选择导出文件"),{target:{files:[second]}});
  await act(async()=>{read.resolve("FIRST BODY");await read.promise;});
  await screen.findByRole("heading",{name:"确认这份不可变导入准备"});
  expect(second.text).not.toHaveBeenCalled();expect(core.callsTo("prepare_import")).toHaveLength(1);
  expect(core.callsTo("prepare_import")[0]?.payload).toMatchObject({request:{text:"FIRST BODY"}});
 });
 it("准备后原File变化不会被confirm重新读取，payload保持只有IDs",async()=>{
  let words="ORIGINAL BODY";const chosen=new File([],"same.jsonl");const text=vi.fn(async()=>words);Object.defineProperty(chosen,"text",{value:text});
  const core=installFakeCore({preparingImport:()=>aPreparedImport()});const user=userEvent.setup();render(<Import lane={new DurableImport()}/>);
  fireEvent.change(screen.getByLabelText("选择导出文件"),{target:{files:[chosen]}});await screen.findByRole("heading",{name:"确认这份不可变导入准备"});
  words="CHANGED BODY";await user.click(screen.getByRole("button",{name:"确认导入这份准备"}));
  expect(await screen.findByText(/已提交：新增事件/)).toBeVisible();expect(text).toHaveBeenCalledTimes(1);
  expect(JSON.stringify(core.callsTo("confirm_import"))).not.toMatch(/ORIGINAL BODY|CHANGED BODY|text/);
 });
 it("旧路由查询迟到结果不会交付给新挂载",async()=>{
  const pending=deferred<ImportOperationView>();const core=installFakeCore({lookingUpImport:()=>pending.promise});
  const lane=new DurableImport();lane.remember(IMPORT_OPERATION_ID);const user=userEvent.setup();const old=render(<Import lane={lane}/>);
  await user.click(screen.getByRole("button",{name:"查询这个编号"}));old.unmount();render(<Import lane={lane}/>);
  await act(async()=>{pending.resolve(anImportOperationView({operation:anImportOperation({state:"committed",receipt:anImportOperation().preview})}));await pending.promise;});
  expect(screen.queryByText(/已提交：新增事件/)).toBeNull();expect(lane.snapshot().receipt).toBeNull();
  expect(core.callsTo("import_operation")).toHaveLength(1);expect(core.callsTo("confirm_import")).toHaveLength(0);
 });
 it("没有文件的change不创建任何操作",async()=>{
  const core=installFakeCore();render(<Import lane={new DurableImport()}/>);
  fireEvent.change(screen.getByLabelText("选择导出文件"),{target:{files:[]}});
  await waitFor(()=>expect(screen.getByLabelText("选择导出文件")).toBeEnabled());
  expect(core.callsTo("prepare_import")).toHaveLength(0);expect(core.callsTo("confirm_import")).toHaveLength(0);
 });
});
