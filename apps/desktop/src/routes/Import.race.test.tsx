/** Controlled browser reads and IPC responses exercise only reachable UI races. */
import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  commitSoulImportV1,
  commitTelegram,
  previewSoulImportV1,
  previewTelegram,
  type ImportPreview,
  type ImportReceipt,
} from "../core";
import { anImportPreview, anImportReceipt } from "../test/fakeCore";
import { Import } from "./Import";

vi.mock("../core", () => ({
  commitSoulImportV1: vi.fn(),
  commitTelegram: vi.fn(),
  previewSoulImportV1: vi.fn(),
  previewTelegram: vi.fn(),
}));

const REPO = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "..");
const JSONL = readFileSync(
  join(REPO, "fixtures", "import", "soul-import-v1", "valid_basic.jsonl"),
  "utf8",
);

const TELEGRAM = readFileSync(
  join(REPO, "fixtures", "import", "telegram", "result_basic.json"),
  "utf8",
);

interface Deferred<T> {
  readonly promise: Promise<T>;
  resolve(value: T): void;
  reject(reason: unknown): void;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((ok, fail) => {
    resolve = ok;
    reject = fail;
  });
  return { promise, resolve, reject };
}

function controlledFile(reading: Deferred<string>): File {
  const file = new File([JSONL], "valid_basic.jsonl", { type: "application/json" });
  vi.spyOn(file, "text").mockReturnValue(reading.promise);
  return file;
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(previewSoulImportV1).mockResolvedValue(anImportPreview());
  vi.mocked(previewTelegram).mockResolvedValue(anImportPreview({ source: "telegram-desktop" }));
  vi.mocked(commitSoulImportV1).mockResolvedValue(anImportReceipt());
  vi.mocked(commitTelegram).mockResolvedValue(anImportReceipt({ source: "telegram-desktop" }));
});

describe("导入页异步交互", () => {
  it.each(["文件读取", "核心预览"])("%s等待时切格式不会让原文件走另一种提交", async (phase) => {
    const reading = deferred<string>();
    const preview = deferred<ImportPreview>();
    vi.mocked(previewSoulImportV1).mockReturnValue(preview.promise);
    const user = userEvent.setup();
    render(<Import />);
    const soul = screen.getByRole("radio", { name: "soul-import-v1 JSONL" });
    const telegram = screen.getByRole("radio", { name: "Telegram Desktop 的 result.json" });
    const input = screen.getByLabelText("选择文件");

    await user.upload(input, controlledFile(reading));
    if (phase === "核心预览") {
      await act(async () => reading.resolve(JSONL));
    }
    await user.click(telegram);
    expect.soft(soul).toBeDisabled();
    expect.soft(telegram).toBeDisabled();
    expect(input).toBeDisabled();
    if (phase === "文件读取") {
      await act(async () => reading.resolve(JSONL));
    }
    await act(async () => preview.resolve(anImportPreview()));

    expect.soft(soul).toBeChecked();
    expect.soft(telegram).not.toBeChecked();
    expect(screen.getByTestId("preview-source")).toHaveTextContent("soul-import-v1");
    expect(previewSoulImportV1).toHaveBeenCalledExactlyOnceWith(JSONL);
    expect(previewTelegram).not.toHaveBeenCalled();
    expect(commitSoulImportV1).not.toHaveBeenCalled();
    expect(commitTelegram).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "确认导入" }));
    expect.soft(commitSoulImportV1).toHaveBeenCalledExactlyOnceWith(JSONL);
    expect(commitTelegram).not.toHaveBeenCalled();
    expect(screen.getByTestId("receipt-source")).toHaveTextContent("soul-import-v1");
  });

  it.each(["文件读取", "核心预览"])("%s失败后可以换格式重选，旧错误不残留", async (phase) => {
    const reading = deferred<string>();
    const firstPreview = deferred<ImportPreview>();
    const nextPreview = deferred<ImportPreview>();
    vi.mocked(previewSoulImportV1).mockReturnValue(firstPreview.promise);
    vi.mocked(previewTelegram).mockReturnValue(nextPreview.promise);
    const user = userEvent.setup();
    render(<Import />);
    const input = screen.getByLabelText("选择文件");
    const telegram = screen.getByRole("radio", { name: "Telegram Desktop 的 result.json" });
    await user.upload(input, controlledFile(reading));
    if (phase === "核心预览") {
      await act(async () => reading.resolve(JSONL));
    }

    // A second request cannot start while the first is in flight. Do not
    // manufacture an unreachable late rejection by bypassing disabled inputs.
    expect(input).toBeDisabled();
    expect(telegram).toBeDisabled();
    await user.click(telegram);
    expect(telegram).not.toBeChecked();
    const refusal = { reason_code: "IMPORT_UNREADABLE", explanation: "这个文件读不成。" };
    await act(async () => {
      if (phase === "文件读取") reading.reject(refusal);
      else firstPreview.reject(refusal);
    });
    expect(screen.getByRole("alert")).toHaveTextContent("IMPORT_UNREADABLE");
    expect(screen.queryByRole("status")).toBeNull();
    expect(input).toBeEnabled();
    expect(telegram).toBeEnabled();

    await user.click(telegram);
    await user.upload(input, new File([TELEGRAM], "result.json", { type: "application/json" }));
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.queryByTestId("preview-counts")).toBeNull();
    expect(previewTelegram).toHaveBeenCalledExactlyOnceWith(TELEGRAM);
    expect(previewSoulImportV1).toHaveBeenCalledTimes(phase === "文件读取" ? 0 : 1);
    await act(async () => nextPreview.resolve(anImportPreview({ source: "telegram-desktop" })));
    expect(screen.getByTestId("preview-source")).toHaveTextContent("telegram-desktop");
    expect(screen.queryByRole("alert")).toBeNull();
    expect(commitSoulImportV1).not.toHaveBeenCalled();
    expect(commitTelegram).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "确认导入" }));
    expect(commitTelegram).toHaveBeenCalledExactlyOnceWith(TELEGRAM);
    expect(commitSoulImportV1).not.toHaveBeenCalled();
    expect(screen.getByTestId("receipt-source")).toHaveTextContent("telegram-desktop");
  });

  it("取消后换文件只显示新预览，确认只交付新文本", async () => {
    const nextPreview = deferred<ImportPreview>();
    vi.mocked(previewTelegram).mockReturnValue(nextPreview.promise);
    const user = userEvent.setup();
    render(<Import />);
    const input = screen.getByLabelText("选择文件");
    await user.upload(input, new File([JSONL], "valid_basic.jsonl"));
    await screen.findByTestId("preview-counts");
    await user.click(screen.getByRole("button", { name: "不了，换一个文件" }));
    expect(screen.queryByTestId("preview-counts")).toBeNull();
    expect(screen.queryByTestId("import-file-size")).toBeNull();
    expect(screen.queryByTestId("receipt-source")).toBeNull();
    expect(commitSoulImportV1).not.toHaveBeenCalled();
    expect(commitTelegram).not.toHaveBeenCalled();

    await user.click(screen.getByRole("radio", { name: "Telegram Desktop 的 result.json" }));
    await user.upload(input, new File([TELEGRAM], "result.json"));
    expect(screen.queryByTestId("preview-counts")).toBeNull();
    expect(screen.queryByRole("button", { name: "不了，换一个文件" })).toBeNull();
    await act(async () => nextPreview.resolve(anImportPreview({ source: "telegram-desktop", messages: 23 })));
    expect(screen.getByTestId("preview-counts")).toHaveTextContent("23 条消息");
    expect(screen.getByTestId("preview-source")).toHaveTextContent("telegram-desktop");
    await user.click(screen.getByRole("button", { name: "确认导入" }));
    expect(commitTelegram).toHaveBeenCalledExactlyOnceWith(TELEGRAM);
    expect(commitSoulImportV1).not.toHaveBeenCalled();
  });

  it.each(["取消", "切换格式"])("%s后可以重新选择同一个文件重新预览", async (action) => {
    const user = userEvent.setup();
    render(<Import />);
    const input = screen.getByLabelText("选择文件");
    const file = new File([JSONL], "valid_basic.jsonl");
    await user.upload(input, file);
    await screen.findByTestId("preview-counts");
    if (action === "取消") {
      await user.click(screen.getByRole("button", { name: "不了，换一个文件" }));
    } else {
      await user.click(screen.getByRole("radio", { name: "Telegram Desktop 的 result.json" }));
      await user.click(screen.getByRole("radio", { name: "soul-import-v1 JSONL" }));
    }
    await user.upload(input, file);
    expect(previewSoulImportV1).toHaveBeenCalledTimes(2);
    expect(await screen.findByTestId("preview-source")).toHaveTextContent("soul-import-v1");
    expect(commitSoulImportV1).not.toHaveBeenCalled();
    expect(commitTelegram).not.toHaveBeenCalled();
  });

  it("快速双击确认在写入完成前只产生一次提交", async () => {
    const writing = deferred<ImportReceipt>();
    vi.mocked(commitSoulImportV1).mockReturnValue(writing.promise);
    const user = userEvent.setup();
    render(<Import />);
    const input = screen.getByLabelText("选择文件");
    await user.upload(input, new File([JSONL], "valid_basic.jsonl"));
    await screen.findByTestId("preview-counts");
    const confirm = screen.getByRole("button", { name: "确认导入" });
    const abandon = screen.getByRole("button", { name: "不了，换一个文件" });
    await user.dblClick(confirm);
    expect(commitSoulImportV1).toHaveBeenCalledExactlyOnceWith(JSONL);
    expect(commitTelegram).not.toHaveBeenCalled();
    expect(confirm).toBeDisabled();
    expect(abandon).toBeDisabled();
    expect(input).toBeDisabled();
    for (const radio of screen.getAllByRole("radio")) expect(radio).toBeDisabled();
    expect(screen.getByRole("status")).toHaveTextContent(/正在.+请稍候/);
    await user.click(abandon);
    await user.click(screen.getByRole("radio", { name: "Telegram Desktop 的 result.json" }));
    expect(screen.getByTestId("preview-source")).toHaveTextContent("soul-import-v1");
    await act(async () => writing.resolve(anImportReceipt()));
    expect(screen.getAllByRole("heading", { name: "导入完成" })).toHaveLength(1);
    expect(screen.queryByRole("button", { name: "确认导入" })).toBeNull();
    expect(screen.queryByRole("status")).toBeNull();
    expect(commitSoulImportV1).toHaveBeenCalledTimes(1);
  });

  it("提交失败后保留预览并可重试，重试成功清掉错误", async () => {
    const first = deferred<ImportReceipt>();
    const second = deferred<ImportReceipt>();
    vi.mocked(commitSoulImportV1).mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    const user = userEvent.setup();
    render(<Import />);
    await user.upload(screen.getByLabelText("选择文件"), new File([JSONL], "valid_basic.jsonl"));
    await screen.findByTestId("preview-counts");
    await user.click(screen.getByRole("button", { name: "确认导入" }));
    await act(async () => first.reject({ reason_code: "IMPORT_REFUSED", explanation: "这次没有导入。" }));
    expect(screen.getByRole("alert")).toHaveTextContent("IMPORT_REFUSED");
    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.getByRole("button", { name: "确认导入" })).toBeEnabled();
    expect(screen.getByTestId("preview-source")).toHaveTextContent("soul-import-v1");
    await user.click(screen.getByRole("button", { name: "确认导入" }));
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.getByRole("button", { name: "确认导入" })).toBeDisabled();
    await act(async () => second.resolve(anImportReceipt()));
    expect(screen.getByTestId("receipt-source")).toHaveTextContent("soul-import-v1");
    expect(screen.queryByRole("alert")).toBeNull();
    expect(commitSoulImportV1).toHaveBeenCalledTimes(2);
    expect(vi.mocked(commitSoulImportV1).mock.calls).toEqual([[JSONL], [JSONL]]);
    expect(commitTelegram).not.toHaveBeenCalled();
  });

  it("只靠Tab、方向键、空格和Enter能选择格式、取消并确认", async () => {
    const writing = deferred<ImportReceipt>();
    vi.mocked(commitTelegram).mockReturnValue(writing.promise);
    const user = userEvent.setup();
    render(<Import />);
    const soul = screen.getByRole("radio", { name: "soul-import-v1 JSONL" });
    const telegram = screen.getByRole("radio", { name: "Telegram Desktop 的 result.json" });
    const input = screen.getByLabelText("选择文件");
    await user.tab();
    expect(soul).toHaveFocus();
    await user.keyboard("{ArrowDown}");
    expect(telegram).toHaveFocus();
    expect(telegram).toBeChecked();
    await user.tab();
    expect(input).toHaveFocus();
    // userEvent supplies the native picker result; all in-page decisions use keys.
    await user.upload(input, new File([TELEGRAM], "result.json"));
    await screen.findByTestId("preview-counts");
    await user.tab();
    expect(telegram).toHaveFocus();
    await user.tab();
    expect(input).toHaveFocus();
    await user.tab();
    expect(screen.getByRole("button", { name: "确认导入" })).toHaveFocus();
    await user.tab();
    expect(screen.getByRole("button", { name: "不了，换一个文件" })).toHaveFocus();
    await user.keyboard(" ");
    expect(screen.queryByTestId("preview-counts")).toBeNull();
    expect(commitTelegram).not.toHaveBeenCalled();

    await user.tab();
    expect(telegram).toHaveFocus();
    await user.tab();
    expect(input).toHaveFocus();
    await user.upload(input, new File([TELEGRAM], "second-result.json"));
    await screen.findByTestId("preview-counts");
    await user.tab();
    expect(telegram).toHaveFocus();
    await user.tab();
    expect(input).toHaveFocus();
    await user.tab();
    const confirm = screen.getByRole("button", { name: "确认导入" });
    expect(confirm).toHaveFocus();
    await user.keyboard("{Enter}{Enter}");
    expect(confirm).toBeDisabled();
    expect(screen.getByRole("status")).toHaveTextContent(/正在.+请稍候/);
    expect(commitTelegram).toHaveBeenCalledExactlyOnceWith(TELEGRAM);
    expect(commitSoulImportV1).not.toHaveBeenCalled();
    await act(async () => writing.resolve(anImportReceipt({ source: "telegram-desktop" })));
    expect(screen.getByTestId("receipt-source")).toHaveTextContent("telegram-desktop");
    expect(screen.queryByRole("status")).toBeNull();
  });

  it("文件读取和核心预览等待时给出可读状态，完成后撤掉等待状态", async () => {
    const reading = deferred<string>();
    const preview = deferred<ImportPreview>();
    vi.mocked(previewSoulImportV1).mockReturnValue(preview.promise);
    const user = userEvent.setup();
    render(<Import />);

    await user.upload(screen.getByLabelText("选择文件"), controlledFile(reading));
    expect(screen.getByRole("status")).toHaveTextContent(/正在.+请稍候/);
    await act(async () => reading.resolve(JSONL));
    expect(screen.getByRole("status")).toHaveTextContent(/正在.+请稍候/);
    expect(screen.queryByRole("button", { name: "确认导入" })).toBeNull();
    await act(async () => preview.resolve(anImportPreview()));
    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.getByRole("button", { name: "确认导入" })).toBeEnabled();
  });
});
