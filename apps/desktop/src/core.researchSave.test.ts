import { invoke } from "@tauri-apps/api/core";
import { createElement } from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";

import * as core from "./core";
import { ResearchSave } from "./routes/ResearchSave";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
beforeEach(() => { vi.mocked(invoke).mockReset(); });

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

function request(name: string): core.ResearchSavePrepare {
  return { scope: { from_day: "2024-02-29", to_day: "2024-03-01" }, destination: `C:\\research\\${name}.zip` };
}

function preview(id: string): core.ResearchSavePreview {
  return { request_id: id, destination: request(id).destination, prepared_at_utc: "2024-03-02T00:00:00Z",
    dataset: { scope: request(id).scope, rows: [], counts: {
      candidate_events: 0, included_events: 0, excluded_policy_events: 0,
      excluded_unavailable_events: 0, excluded_time_events: 0, out_of_range_events: 0,
    } }, notice: "synthetic preview" };
}

function backend() {
  const first = deferred<core.ResearchSavePreview>();
  const second = deferred<core.ResearchSavePreview>();
  let held: string | null = "existing";
  let prepares = 0;
  vi.mocked(invoke).mockImplementation((command, args) => {
    if (command === core.COMMANDS.prepareResearchSave) {
      const pending = prepares++ === 0 ? first : second;
      return pending.promise.then((value) => { held = value.request_id; return value; }) as Promise<never>;
    }
    if (command !== core.COMMANDS.cancelResearchSave && command !== core.COMMANDS.confirmResearchSave) {
      throw new Error(`unexpected command ${command}`);
    }
    const id = (args as { requestId: string }).requestId;
    if (command === core.COMMANDS.cancelResearchSave) {
      const cancelled = held === id;
      if (cancelled) held = null;
      return Promise.resolve({ cancelled, already_published: false }) as Promise<never>;
    }
    if (command === core.COMMANDS.confirmResearchSave) {
      return held === id
        ? Promise.resolve({ request_id: id, destination: request(id).destination, row_count: 0,
          included_events: 0, written_to_disk: true, transaction_closed: true }) as Promise<never>
        : Promise.reject(new Error("synthetic stale held request"));
    }
    throw new Error("unexpected command");
  });
  return { first, second, held: () => held };
}

async function flush() {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}

function prepareCalls() {
  return vi.mocked(invoke).mock.calls.filter(([command]) => command === core.COMMANDS.prepareResearchSave);
}

it("实际准备wrapper在前一个invoke完成前不派发第二个请求", async () => {
  const remote = backend();
  const first = core.prepareResearchSave(request("A"));
  const second = core.prepareResearchSave(request("B"));
  try {
    await flush();
    expect(prepareCalls()).toEqual([[core.COMMANDS.prepareResearchSave, { request: request("A") }]]);
    remote.first.resolve(preview("A"));
    await first;
    await flush();
    expect(prepareCalls()).toHaveLength(2);
    expect(prepareCalls()[1]).toEqual([core.COMMANDS.prepareResearchSave, { request: request("B") }]);
    remote.second.resolve(preview("B"));
    await second;
    expect(remote.held()).toBe("B");
    // A's abandoned consumer may report its cancellation after B is held.
    await core.cancelResearchSave("A");
    expect(remote.held()).toBe("B");
    await expect(core.confirmResearchSave("B")).resolves.toMatchObject({ written_to_disk: true });
  } finally {
    remote.first.resolve(preview("A"));
    remote.second.resolve(preview("B"));
    await Promise.allSettled([first, second]);
  }
});

it("前一个prepare失败后后续请求仍派发且不重复执行", async () => {
  const remote = backend();
  const first = core.prepareResearchSave(request("A"));
  const rejected = expect(first).rejects.toThrow("synthetic prepare failure");
  const second = core.prepareResearchSave(request("B"));
  try {
    await flush();
    expect(prepareCalls()).toHaveLength(1);
    remote.first.reject(new Error("synthetic prepare failure"));
    await rejected;
    await flush();
    expect(prepareCalls()).toHaveLength(2);
    remote.second.resolve(preview("B"));
    await expect(second).resolves.toMatchObject({ request_id: "B" });
  } finally {
    remote.first.reject(new Error("synthetic prepare failure"));
    remote.second.resolve(preview("B"));
    await Promise.allSettled([rejected, second]);
  }
});

it("confirm和cancel不加入prepare队列，未完成prepare不能阻塞它们", async () => {
  const remote = backend();
  const first = core.prepareResearchSave(request("A"));
  try {
    await flush();
    await expect(core.confirmResearchSave("existing")).resolves.toMatchObject({ written_to_disk: true });
    await expect(core.cancelResearchSave("existing")).resolves.toMatchObject({ cancelled: true });
    expect(prepareCalls()).toHaveLength(1);
  } finally {
    remote.first.resolve(preview("A"));
    await first;
  }
});

function enter(name: string) {
  fireEvent.change(screen.getByLabelText("UTC 开始日期"), { target: { value: "2024-02-29" } });
  fireEvent.change(screen.getByLabelText("UTC 结束日期（不含）"), { target: { value: "2024-03-01" } });
  fireEvent.change(screen.getByLabelText("新的 ZIP 文件绝对路径"), { target: { value: request(name).destination } });
  fireEvent.click(screen.getByRole("button", { name: "准备研究保存" }));
}

it("组件卸载重挂仍共享队列，旧准备取消后新预览可实际确认", async () => {
  const remote = backend();
  const old = render(createElement(ResearchSave));
  enter("A");
  old.unmount();
  const current = render(createElement(ResearchSave));
  enter("B");
  try {
    await act(flush);
    expect(prepareCalls()).toHaveLength(1);
    await act(async () => { remote.first.resolve(preview("A")); await flush(); });
    expect(prepareCalls()).toHaveLength(2);
    await act(async () => { remote.second.resolve(preview("B")); await flush(); });
    expect(await screen.findByTestId("research-save-preview")).toHaveTextContent("B");
    expect(remote.held()).toBe("B");
    fireEvent.click(screen.getByRole("button", { name: "确认保存这份研究数据" }));
    expect(await screen.findByTestId("research-save-receipt")).toHaveTextContent("已经保存");
    expect(vi.mocked(invoke).mock.calls).toContainEqual([core.COMMANDS.cancelResearchSave, { requestId: "A" }]);
  } finally {
    await act(async () => {
      remote.first.resolve(preview("A")); remote.second.resolve(preview("B")); await flush();
    });
    current.unmount();
  }
});
