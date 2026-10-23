import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { listen } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "../App";
import * as core from "../core";
import { startRuntimeEvents, type ForgetReceipt, type MemoryDetail, type PeopleGraph } from "../core";
import { aMemoryDetail, aMemoryList, aPeopleGraph, CLOSED_SNAPSHOT, COLLECT_OFF, OPEN_SESSION, installFakeCore } from "../test/fakeCore";
import { Memory } from "./Memory";
import { Graph } from "./Graph";
import { DraftScopedVoice } from "./DraftScopedVoice";
import { Import } from "./Import";
import { DurableImport } from "./durableImport";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
const MEMORY = "0192f000-0000-7000-8000-0000000000a1";
const PERSON = "11111111-1111-4111-8111-111111111111";
let deliver: (event: { payload: unknown }) => void;
let stop: (() => void) | undefined;
function event(sequence: number, phase: "started" | "settled") {
  return { payload: { epoch: "10000000-0000-4000-8000-000000000001", sequence,
    operation_id: "20000000-0000-4000-8000-000000000001", caller: { transport: "http", webview_label: null },
    correlation_id: null, kind: "memory_edit", phase, outcome: phase === "settled" ? "refused" : null } };
}
async function bridge() {
  stop = startRuntimeEvents();
  await Promise.resolve(); await Promise.resolve(); await Promise.resolve();
}
beforeEach(() => {
  vi.mocked(listen).mockImplementation(async (_name, handler) => {
    deliver = handler as typeof deliver; return () => undefined;
  });
});
afterEach(() => { stop?.(); stop = undefined; vi.restoreAllMocks(); });
const rows = aMemoryList({ memories: [{ ...aMemoryList().memories[0]!, memory_id: MEMORY }] });
describe("mounted routes observe foreign runtime changes", () => {
  it("a stale import confirmation DOM handler cannot confirm after external invalidation", async () => {
    const fake = installFakeCore(); await bridge();
    render(<Import lane={new DurableImport()} />);
    const file = new File(["SYNTHETIC"], "synthetic.jsonl");
    Object.defineProperty(file, "text", { value: vi.fn().mockResolvedValue("SYNTHETIC") });
    fireEvent.change(screen.getByLabelText("选择导出文件"), { target: { files: [file] } });
    const staleConfirm = await screen.findByRole("button", { name: "确认导入这份准备" });
    await act(async () => { deliver(event(1, "started")); staleConfirm.click(); });
    expect(fake.callsTo("confirm_import")).toHaveLength(0);
    act(() => deliver(event(2, "settled")));
    expect(screen.queryByRole("button", { name: "确认导入这份准备" })).toBeNull();
    expect(fake.callsTo("prepare_import")).toHaveLength(1);
  });
  it("an old import lookup rejection cannot replace state after its external invalidation", async () => {
    let reject!: (reason: unknown) => void;
    const pending = new Promise<never>((_yes, no) => { reject = no; });
    installFakeCore({ recentImports: () => pending }); await bridge();
    render(<Import lane={new DurableImport()} />); const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "核对最近记录" }));
    await act(async () => { deliver(event(1, "started")); deliver(event(2, "settled"));
      reject({ reason_code: "ROUTINE", explanation: "synthetic obsolete lookup failure" }); await pending.catch(() => undefined); });
    expect(screen.queryByText("synthetic obsolete lookup failure")).toBeNull();
  });
  it("failed external writes hide plaintext and forget quotes immediately, then refresh only list/detail", async () => {
    let summary = "synthetic old plaintext";
    const fake = installFakeCore({ memories: () => rows, opening: () => aMemoryDetail({ memory_id: MEMORY, summary }) });
    await bridge();
    window.location.hash = `#/memory?memory_id=${MEMORY}`;
    await new Promise<void>((resolve) => window.setTimeout(resolve, 0));
    render(<Memory />); const user = userEvent.setup();
    await screen.findByDisplayValue(summary);
    await user.click(within(screen.getByTestId(`memory-${MEMORY}`)).getByRole("button", { name: "看遗忘会影响什么" }));
    const staleConfirm = await screen.findByRole("button", { name: "就按上面这些，遗忘它" });
    act(() => { deliver(event(1, "started")); staleConfirm.click(); });
    expect(screen.queryByDisplayValue(summary)).toBeNull();
    expect(screen.queryByTestId("forget-preview")).toBeNull();
    expect(fake.callsTo("forget_memory")).toHaveLength(0);
    summary = "synthetic refreshed plaintext";
    act(() => { deliver(event(2, "settled")); });
    expect(await screen.findByDisplayValue(summary)).toBeVisible();
    expect(fake.callsTo("memory_detail")).toHaveLength(2);
    expect(fake.callsTo("preview_forget")).toHaveLength(1);
    for (const name of ["update_memory", "forget_memory", "prepare_import", "draft_scoped_reply"]) expect(fake.callsTo(name)).toHaveLength(0);
  });
  it("foreign changes do not release a physically pending update or discard its success receipt", async () => {
    let resolve!: (value: MemoryDetail) => void;
    const pending = new Promise<MemoryDetail>((yes) => { resolve = yes; });
    const fake = installFakeCore({ memories: () => rows,
      opening: () => aMemoryDetail({ memory_id: MEMORY, summary: "synthetic current" }), editing: () => pending });
    await bridge(); window.location.hash = `#/memory?memory_id=${MEMORY}`;
    await new Promise<void>((yes) => window.setTimeout(yes, 0));
    render(<Memory />); const user = userEvent.setup();
    await screen.findByDisplayValue("synthetic current");
    await user.click(screen.getByRole("button", { name: "改好了" }));
    await waitFor(() => expect(fake.callsTo("update_memory")).toHaveLength(1));
    await act(async () => { deliver(event(1, "started")); deliver(event(2, "settled")); });
    expect(fake.callsTo("memory_detail")).toHaveLength(1);
    await act(async () => { resolve(aMemoryDetail({ memory_id: MEMORY, summary: "synthetic old receipt" })); await pending; });
    expect(await screen.findByDisplayValue("synthetic current")).toBeVisible();
    expect(screen.getByTestId("memory-write-source-changed")).toHaveTextContent("写入已收到核心回执");
    expect(screen.queryByDisplayValue("synthetic old receipt")).toBeNull();
    expect(fake.callsTo("update_memory")).toHaveLength(1);
  });
  it("a mismatched forget cannot restore its quote when a foreign change overlapped the attempt", async () => {
    let reject!: (reason: unknown) => void;
    const pending = new Promise<ForgetReceipt>((_yes, no) => { reject = no; });
    const fake = installFakeCore({ memories: () => rows, forgetting: () => pending });
    await bridge(); window.location.hash = "#/memory";
    await new Promise<void>((yes) => window.setTimeout(yes, 0));
    render(<Memory />); const user = userEvent.setup();
    await user.click(within(await screen.findByTestId(`memory-${MEMORY}`)).getByRole("button", { name: "看遗忘会影响什么" }));
    await user.click(await screen.findByRole("button", { name: "就按上面这些，遗忘它" }));
    await waitFor(() => expect(fake.callsTo("forget_memory")).toHaveLength(1));
    await act(async () => { deliver(event(1, "started")); deliver(event(2, "settled"));
      reject({ reason_code: "PLAN_HASH_MISMATCH", explanation: "synthetic mismatch" }); await pending.catch(() => undefined); });
    await screen.findByRole("heading", { name: /这台机器上的记忆/ });
    expect(screen.queryByTestId("forget-preview")).toBeNull();
    expect(fake.callsTo("forget_memory")).toHaveLength(1);
    expect(fake.callsTo("preview_forget")).toHaveLength(1);
  });
  it("graph refresh reads people after settle without regenerating a summary or correcting ties", async () => {
    let graph = aPeopleGraph();
    const fake = installFakeCore({ graphing: () => graph }); await bridge();
    window.location.hash = "#/graph"; render(<Graph />);
    await screen.findAllByRole("button", { name: "看这个人的摘要" });
    act(() => deliver(event(1, "started")));
    expect(screen.queryByRole("button", { name: "看这个人的摘要" })).toBeNull();
    graph = { ...graph, people: graph.people.map((person) => ({ ...person, identifier_hint: "synthetic-refreshed" })) };
    act(() => deliver(event(2, "settled")));
    await screen.findAllByText(/synthetic-refreshed/);
    expect(fake.callsTo("people_graph")).toHaveLength(2);
    for (const name of ["person_summary", "correct_tie", "release_tie"]) expect(fake.callsTo(name)).toHaveLength(0);
  });
  it("a late local tie receipt does not replace the graph read after foreign changes", async () => {
    let graph = aPeopleGraph(); let resolve!: (value: PeopleGraph) => void;
    const pending = new Promise<PeopleGraph>((yes) => { resolve = yes; });
    installFakeCore({ graphing: () => graph }); await bridge();
    const correction = vi.spyOn(core, "correctTie").mockReturnValueOnce(pending);
    window.location.hash = "#/graph"; render(<Graph />); const user = userEvent.setup();
    await screen.findAllByRole("button", { name: "看这个人的摘要" });
    await user.click(screen.getAllByRole("button", { name: "强" })[0]!);
    expect(correction).toHaveBeenCalledTimes(1);
    act(() => deliver(event(1, "started")));
    graph = { ...graph, people: graph.people.map((person) => ({ ...person, identifier_hint: "synthetic-latest-graph" })) };
    act(() => deliver(event(2, "settled")));
    await screen.findAllByText(/synthetic-latest-graph/);
    await act(async () => { resolve(aPeopleGraph()); await pending; });
    expect(screen.getAllByText(/synthetic-latest-graph/).length).toBeGreaterThan(0);
  });
  it("a selected scoped status can refresh after external settlement without drafting or confirming", async () => {
    let state: "active" | "inactive" = "active";
    const fake = installFakeCore({ scopedRecipients: [{ person_id: PERSON, identifier_hint: "synthetic-person" }],
      readingScopedRegisterStatus: () => ({ person_id: PERSON, occasion: "personal", state,
        preference: state === "active" ? { preference_id: "preference", revision: "revision", memory_id: MEMORY, register: "casual" } : null,
        memory_id: MEMORY, notice: `synthetic-status-${state}` }) });
    await bridge(); render(<DraftScopedVoice onExit={() => undefined} />); const user = userEvent.setup();
    await user.selectOptions(await screen.findByLabelText("收件人物"), PERSON);
    await user.selectOptions(screen.getByLabelText("场合"), "personal");
    await user.click(screen.getByRole("button", { name: "查看此范围当前偏好" }));
    await screen.findByText("synthetic-status-active");
    act(() => deliver(event(1, "started")));
    expect(screen.queryByText("synthetic-status-active")).toBeNull();
    state = "inactive"; act(() => deliver(event(2, "settled")));
    expect(await screen.findByText("synthetic-status-inactive")).toBeVisible();
    expect(fake.callsTo("scoped_register_recipients").length).toBeGreaterThan(1);
    for (const name of ["draft_scoped_reply", "propose_scoped_register", "confirm_scoped_register"]) expect(fake.callsTo(name)).toHaveLength(0);
  });
  it("App owns the bridge and rereads configuration/status after a foreign failed mutation", async () => {
    const fake = installFakeCore(); window.location.hash = "#/home"; render(<App />);
    await screen.findByRole("navigation", { name: "主导航" });
    const reads = fake.callsTo("session_status").length;
    await act(async () => { deliver(event(1, "started")); deliver(event(2, "settled")); });
    await waitFor(() => expect(fake.callsTo("session_status").length).toBeGreaterThan(reads));
    expect(fake.callsTo("config_snapshot").length).toBeGreaterThan(1);
    expect(fake.callsTo("forget_memory")).toHaveLength(0);
  });
  it("App recovers after a fresh snapshot arrives before an already owned local mutation settles", async () => {
    mockWindows("main");
    const calls: string[] = [];
    let failNextSnapshot = false;
    let resolve!: (value: core.ConfigSnapshot) => void;
    const pending = new Promise<core.ConfigSnapshot>((yes) => { resolve = yes; });
    mockIPC((command) => {
      calls.push(command);
      if (command === "runtime_event_snapshot" && failNextSnapshot) {
        failNextSnapshot = false;
        return Promise.reject(new Error("synthetic snapshot failure"));
      }
      if (command === "runtime_event_snapshot") return { epoch: "10000000-0000-4000-8000-000000000001", sequence: 0, active_operations: [] };
      if (command === "clear_user_endpoint") return pending;
      if (command === "config_snapshot") return CLOSED_SNAPSHOT;
      if (command === "session_status") return OPEN_SESSION;
      if (command === "collect_status") return COLLECT_OFF;
      throw new Error(`unexpected synthetic command: ${command}`);
    }, { shouldMockEvents: true });
    window.location.hash = "#/home"; render(<App />);
    await screen.findByRole("navigation", { name: "主导航" });
    failNextSnapshot = true;
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    expect(await screen.findByRole("alert")).toHaveTextContent("本机状态同步未连接");
    let mutation!: Promise<core.ConfigSnapshot>;
    await act(async () => { mutation = core.clearUserEndpoint(); });
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    expect(core.getRuntimeSyncFailure()).toBeNull();
    expect(core.hasPendingSourceChanges()).toBe(true);
    const statusReads = calls.filter((command) => command === "session_status").length;
    await act(async () => { resolve(CLOSED_SNAPSHOT); await expect(mutation).resolves.toEqual(CLOSED_SNAPSHOT); });
    expect(await screen.findByRole("navigation", { name: "主导航" })).toBeVisible();
    expect(calls.filter((command) => command === "session_status").length).toBeGreaterThan(statusReads);
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
