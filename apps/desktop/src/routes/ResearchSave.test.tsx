import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import * as core from "../core";
import { ResearchSave } from "./ResearchSave";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

function preview(requestId = "request-1"): core.ResearchSavePreview {
  return {
    request_id: requestId, destination: "C:\\research\\activity.zip",
    prepared_at_utc: "2024-03-02T00:00:00Z",
    dataset: {
      scope: { from_day: "2024-02-29", to_day: "2024-03-01" },
      rows: [{ event_kind: "app.foreground", time_bucket_utc: "2024-02-29T09:00:00Z", aggregate_count: 2 }],
      counts: { candidate_events: 6, included_events: 2, excluded_policy_events: 1,
        excluded_unavailable_events: 1, excluded_time_events: 1, out_of_range_events: 1 },
    },
    notice: "明文独立副本；原库遗忘不会删除本文件；不能用于恢复 Soul。",
  };
}

function receipt(requestId = "request-1"): core.ResearchSaveReceipt {
  return { request_id: requestId, destination: "C:\\research\\activity.zip",
    row_count: 1, included_events: 2, written_to_disk: true, transaction_closed: true };
}

function fill() {
  fireEvent.change(screen.getByLabelText("UTC 开始日期"), { target: { value: "2024-02-29" } });
  fireEvent.change(screen.getByLabelText("UTC 结束日期（不含）"), { target: { value: "2024-03-01" } });
  fireEvent.change(screen.getByLabelText("新的 ZIP 文件绝对路径"), { target: { value: "C:\\research\\activity.zip" } });
}

async function prepare() {
  fill();
  fireEvent.click(screen.getByRole("button", { name: "准备研究保存" }));
  await screen.findByRole("button", { name: "确认保存这份研究数据" });
}

beforeEach(() => {
  vi.spyOn(core, "prepareResearchSave").mockResolvedValue(preview());
  vi.spyOn(core, "confirmResearchSave").mockResolvedValue(receipt());
  vi.spyOn(core, "cancelResearchSave").mockResolvedValue({ cancelled: true, already_published: false });
});

afterEach(() => vi.restoreAllMocks());

describe("独立研究保存", () => {
  it("先解释用途与明文边界，准备结果展示范围与全部统计，确认前不保存", async () => {
    render(<ResearchSave />);
    await prepare();
    expect(core.prepareResearchSave).toHaveBeenCalledWith({
      scope: { from_day: "2024-02-29", to_day: "2024-03-01" }, destination: "C:\\research\\activity.zip",
    });
    expect(core.confirmResearchSave).not.toHaveBeenCalled();
    expect(screen.getByTestId("research-save-preview")).toHaveTextContent("完整范围 1 行");
    expect(screen.getByTestId("research-save-preview")).toHaveTextContent("纳入 2 个事件");
    expect(screen.getByTestId("research-save-preview")).toHaveTextContent("用途排除 1");
    expect(screen.getByTestId("research-save-preview")).toHaveTextContent("不可用 1");
    expect(screen.getByTestId("research-save-preview")).toHaveTextContent("时间排除 1");
    expect(screen.getByTestId("research-save-preview")).toHaveTextContent("范围外 1");
    expect(screen.getByText(/缺失不补零/)).toBeInTheDocument();
    expect(screen.getByTestId("research-save-preview")).toHaveTextContent("原库遗忘不会删除");
  });

  it("确认只传请求编号，同步双击也只保存一次", async () => {
    const done = deferred<core.ResearchSaveReceipt>();
    vi.mocked(core.confirmResearchSave).mockReturnValue(done.promise);
    render(<ResearchSave />);
    await prepare();
    const button = screen.getByRole("button", { name: "确认保存这份研究数据" });
    fireEvent.click(button);
    fireEvent.click(button);
    expect(core.confirmResearchSave).toHaveBeenCalledTimes(1);
    expect(core.confirmResearchSave).toHaveBeenCalledWith("request-1");
    await act(async () => done.resolve(receipt()));
    expect(await screen.findByTestId("research-save-receipt")).toHaveTextContent("已经保存");
  });

  it("准备期间取消，迟到结果不能显示且其编号被取消", async () => {
    const done = deferred<core.ResearchSavePreview>();
    vi.mocked(core.prepareResearchSave).mockReturnValue(done.promise);
    render(<ResearchSave />);
    fill();
    fireEvent.click(screen.getByRole("button", { name: "准备研究保存" }));
    fireEvent.click(screen.getByRole("button", { name: "取消本次保存" }));
    await act(async () => done.resolve(preview()));
    await waitFor(() => expect(core.cancelResearchSave).toHaveBeenCalledWith("request-1"));
    expect(screen.queryByTestId("research-save-preview")).toBeNull();
    expect(core.confirmResearchSave).not.toHaveBeenCalled();
  });

  it("准备结果到达前卸载，迟到编号也只取消一次", async () => {
    const done = deferred<core.ResearchSavePreview>();
    vi.mocked(core.prepareResearchSave).mockReturnValue(done.promise);
    const mounted = render(<ResearchSave />);
    fill();
    fireEvent.click(screen.getByRole("button", { name: "准备研究保存" }));
    mounted.unmount();
    await act(async () => done.resolve(preview()));
    expect(core.cancelResearchSave).toHaveBeenCalledExactlyOnceWith("request-1");
  });

  it("已有预览卸载只取消其请求一次", async () => {
    const mounted = render(<ResearchSave />);
    await prepare();
    mounted.unmount();
    expect(core.cancelResearchSave).toHaveBeenCalledExactlyOnceWith("request-1");
  });

  it("变更目的地立即作废旧预览，不能用旧编号确认", async () => {
    render(<ResearchSave />);
    await prepare();
    fireEvent.change(screen.getByLabelText("新的 ZIP 文件绝对路径"), { target: { value: "C:\\research\\other.zip" } });
    expect(screen.queryByTestId("research-save-preview")).toBeNull();
    expect(core.cancelResearchSave).toHaveBeenCalledExactlyOnceWith("request-1");
    expect(core.confirmResearchSave).not.toHaveBeenCalled();
  });

  it("变更输入后的迟到准备结果不得覆盖新的预览", async () => {
    const old = deferred<core.ResearchSavePreview>();
    vi.mocked(core.prepareResearchSave).mockReturnValueOnce(old.promise).mockResolvedValueOnce(preview("request-2"));
    render(<ResearchSave />);
    fill();
    fireEvent.click(screen.getByRole("button", { name: "准备研究保存" }));
    fireEvent.change(screen.getByLabelText("UTC 结束日期（不含）"), { target: { value: "2024-03-02" } });
    fireEvent.click(screen.getByRole("button", { name: "准备研究保存" }));
    await screen.findByTestId("research-save-preview");
    await act(async () => old.resolve(preview("request-old")));
    expect(core.cancelResearchSave).toHaveBeenCalledWith("request-old");
    expect(screen.getByTestId("research-save-preview")).toHaveTextContent("request-2");
    expect(screen.getByTestId("research-save-preview")).not.toHaveTextContent("request-old");
  });

  for (const cancelMode of ["published", "failed", "cancelled"] as const) {
    it(`保存途中取消${cancelMode}仍等待并展示真实成功收据`, async () => {
      const done = deferred<core.ResearchSaveReceipt>();
      vi.mocked(core.confirmResearchSave).mockReturnValue(done.promise);
      if (cancelMode === "published") {
        vi.mocked(core.cancelResearchSave).mockResolvedValue({ cancelled: false, already_published: true });
      } else if (cancelMode === "failed") {
        vi.mocked(core.cancelResearchSave).mockRejectedValue(new Error("cancel failed"));
      }
      render(<ResearchSave />);
      await prepare();
      fireEvent.click(screen.getByRole("button", { name: "确认保存这份研究数据" }));
      fireEvent.click(screen.getByRole("button", { name: "取消本次保存" }));
      await waitFor(() => expect(core.cancelResearchSave).toHaveBeenCalledWith("request-1"));
      expect(screen.queryByText(/已撤回|文件已删除|没有保存/)).toBeNull();
      await act(async () => done.resolve(receipt()));
      expect(await screen.findByTestId("research-save-receipt")).toHaveTextContent("已经保存");
      expect(screen.getByTestId("research-save-receipt")).toHaveTextContent("request-1");
    });
  }

  it("保存期间卸载只取消一次，迟到成功也不会重试保存", async () => {
    const done = deferred<core.ResearchSaveReceipt>();
    vi.mocked(core.confirmResearchSave).mockReturnValue(done.promise);
    const mounted = render(<ResearchSave />);
    await prepare();
    fireEvent.click(screen.getByRole("button", { name: "确认保存这份研究数据" }));
    mounted.unmount();
    await act(async () => done.resolve(receipt()));
    expect(core.cancelResearchSave).toHaveBeenCalledExactlyOnceWith("request-1");
    expect(core.confirmResearchSave).toHaveBeenCalledTimes(1);
  });

  it("改变输入不能把旧请求已保存的成功收据丢掉", async () => {
    const done = deferred<core.ResearchSaveReceipt>();
    vi.mocked(core.confirmResearchSave).mockReturnValue(done.promise);
    render(<ResearchSave />);
    await prepare();
    fireEvent.click(screen.getByRole("button", { name: "确认保存这份研究数据" }));
    fireEvent.change(screen.getByLabelText("新的 ZIP 文件绝对路径"), { target: { value: "C:\\research\\next.zip" } });
    await act(async () => done.resolve({ ...receipt(), transaction_closed: false }));
    expect(await screen.findByTestId("research-save-receipt")).toHaveTextContent("已经保存");
    expect(screen.getByTestId("research-save-receipt")).toHaveTextContent("事务结束未确认");
    expect(screen.getByTestId("research-save-receipt")).toHaveTextContent("activity.zip");
  });

  it("写入失败后重新准备，不重复使用旧批准", async () => {
    vi.mocked(core.confirmResearchSave).mockRejectedValue(new Error("write failed"));
    render(<ResearchSave />);
    await prepare();
    fireEvent.click(screen.getByRole("button", { name: "确认保存这份研究数据" }));
    await screen.findByRole("alert");
    expect(screen.queryByRole("button", { name: "确认保存这份研究数据" })).toBeNull();
    vi.mocked(core.prepareResearchSave).mockResolvedValue(preview("request-2"));
    fireEvent.click(screen.getByRole("button", { name: "准备研究保存" }));
    expect(await screen.findByTestId("research-save-preview")).toHaveTextContent("request-2");
  });
});
