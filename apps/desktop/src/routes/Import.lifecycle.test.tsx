/**
 * React/IPC tests for the import page. Parser calls are mocked and all text is
 * synthetic. The reentrant test exercises admission within one callback; it
 * does not claim that ordinary separate browser clicks bypass React's updates.
 */
import { StrictMode } from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  commitSoulImportV1,
  commitTelegram,
  MAX_IMPORT_BYTES,
  previewSoulImportV1,
  previewTelegram,
  type ImportReceipt,
} from "../core";
import { anImportPreview, anImportReceipt } from "../test/fakeCore";
import { Import } from "./Import";

vi.mock("../core", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core")>()),
  commitSoulImportV1: vi.fn(),
  commitTelegram: vi.fn(),
  previewSoulImportV1: vi.fn(),
  previewTelegram: vi.fn(),
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

const TEXT = "SYNTHETIC IMPORT BODY - parser is mocked";

function file(): File {
  return new File([TEXT], "sample.jsonl", { type: "application/json" });
}

async function stage(user: ReturnType<typeof userEvent.setup>): Promise<HTMLElement> {
  await user.upload(screen.getByLabelText("选择文件"), file());
  await screen.findByTestId("preview-counts");
  return screen.getByRole("button", { name: "确认导入" });
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(previewSoulImportV1).mockResolvedValue(anImportPreview());
  vi.mocked(previewTelegram).mockResolvedValue(anImportPreview({ source: "telegram-desktop" }));
  vi.mocked(commitSoulImportV1).mockResolvedValue(anImportReceipt());
  vi.mocked(commitTelegram).mockResolvedValue(anImportReceipt({ source: "telegram-desktop" }));
});

describe("import confirmation lifecycle", () => {
  it.each(["soul-import-v1", "telegram-desktop"] as const)(
    "admits a reentrant %s confirmation only once before invoking the core",
    async (format) => {
      const pending = deferred<ImportReceipt>();
      const user = userEvent.setup();
      render(<Import />);
      if (format === "telegram-desktop") {
        await user.click(screen.getByRole("radio", { name: "Telegram Desktop 的 result.json" }));
      }
      // Both parsers are mocked; the chosen extension must still pass the input filter.
      const uploaded = new File([TEXT], format === "telegram-desktop" ? "result.json" : "sample.jsonl");
      await user.upload(screen.getByLabelText("选择文件"), uploaded);
      const confirm = await screen.findByRole("button", { name: "确认导入" });
      const commit = vi.mocked(format === "telegram-desktop" ? commitTelegram : commitSoulImportV1);
      let nested = false;
      commit.mockImplementation(() => {
        if (!nested) {
          nested = true;
          fireEvent.click(confirm);
        }
        return pending.promise;
      });
      await user.click(confirm);
      expect(commit).toHaveBeenCalledExactlyOnceWith(TEXT);
      expect(confirm).toBeDisabled();
      expect(screen.getByRole("button", { name: "不了，换一个文件" })).toBeDisabled();
      expect(screen.getByLabelText("选择文件")).toBeDisabled();
      await act(async () => pending.resolve(anImportReceipt({ source: format })));
      expect(screen.getByTestId("receipt-source")).toHaveTextContent(format);
      expect(commit).toHaveBeenCalledTimes(1);
    },
  );

  it("clears the successful preview and size display without automatically reimporting", async () => {
    const user = userEvent.setup();
    render(<Import />);
    const confirm = await stage(user);
    expect(screen.getByTestId("import-file-size")).toBeVisible();
    await user.click(confirm);
    await screen.findByTestId("receipt-events");
    expect(screen.queryByTestId("preview-counts")).toBeNull();
    expect(screen.queryByTestId("import-file-size")).toBeNull();
    expect(screen.getByLabelText("选择文件")).toHaveValue("");
    expect(previewSoulImportV1).toHaveBeenCalledTimes(1);
    expect(commitSoulImportV1).toHaveBeenCalledTimes(1);

    // A fresh user selection is still permitted; this patch is not message deduplication.
    await user.upload(screen.getByLabelText("选择文件"), file());
    await screen.findByTestId("preview-counts");
    expect(previewSoulImportV1).toHaveBeenCalledTimes(2);
    expect(commitSoulImportV1).toHaveBeenCalledTimes(1);
  });

  it("keeps a rejected preview retryable without changing the approved text", async () => {
    vi.mocked(commitSoulImportV1).mockRejectedValueOnce({
      reason_code: "ROUTINE", explanation: "合成的提交失败。",
    });
    const user = userEvent.setup();
    render(<Import />);
    await user.click(await stage(user));
    await screen.findByRole("alert");
    expect(screen.getByTestId("preview-counts")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "确认导入" }));
    await screen.findByTestId("receipt-events");
    expect(vi.mocked(commitSoulImportV1).mock.calls).toEqual([[TEXT], [TEXT]]);
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("unlocks confirmation after a synchronously throwing core wrapper", async () => {
    vi.mocked(commitSoulImportV1).mockImplementationOnce(() => {
      throw new Error("synthetic synchronous refusal");
    });
    const user = userEvent.setup();
    render(<Import />);
    await user.click(await stage(user));
    await screen.findByRole("alert");
    const confirm = screen.getByRole("button", { name: "确认导入" });
    expect(confirm).toBeEnabled();
    await user.click(confirm);
    await screen.findByTestId("receipt-events");
    expect(commitSoulImportV1).toHaveBeenCalledTimes(2);
  });

  it("releases a budget-refused read before another file is chosen", async () => {
    const tooBig = file();
    Object.defineProperty(tooBig, "size", { value: MAX_IMPORT_BYTES + 1 });
    const reading = vi.spyOn(tooBig, "text");
    const user = userEvent.setup();
    render(<Import />);
    await user.upload(screen.getByLabelText("选择文件"), tooBig);
    await screen.findByRole("alert");
    expect(reading).not.toHaveBeenCalled();
    expect(previewSoulImportV1).not.toHaveBeenCalled();
    await stage(user);
    expect(screen.queryByRole("alert")).toBeNull();
    expect(previewSoulImportV1).toHaveBeenCalledExactlyOnceWith(TEXT);
  });

  it("does not offer confirmation without an identified owner", async () => {
    vi.mocked(previewSoulImportV1).mockResolvedValue(anImportPreview({ owner_identified: false }));
    const user = userEvent.setup();
    render(<Import />);
    const confirm = await stage(user);
    expect(confirm).toBeDisabled();
    await user.click(confirm);
    expect(commitSoulImportV1).not.toHaveBeenCalled();
  });

  it.each(["success", "failure"] as const)(
    "does not preview an unmounted file read after its %s",
    async (outcome) => {
      const reading = deferred<string>();
      const pendingFile = file();
      vi.spyOn(pendingFile, "text").mockReturnValue(reading.promise);
      const user = userEvent.setup();
      const previous = render(<Import />);
      await user.upload(screen.getByLabelText("选择文件"), pendingFile);
      previous.unmount();
      render(<Import />);
      await stage(user);
      await act(async () => {
        if (outcome === "success") reading.resolve("OLD SYNTHETIC BODY");
        else reading.reject(new Error("old synthetic failure"));
      });
      expect(previewSoulImportV1).toHaveBeenCalledExactlyOnceWith(TEXT);
      expect(screen.getByTestId("preview-source")).toHaveTextContent("soul-import-v1");
      expect(screen.queryByRole("alert")).toBeNull();
    },
  );

  it.each(["success", "failure"] as const)(
    "does not present an unmounted commit %s as the fresh page's result",
    async (outcome) => {
      const pending = deferred<ImportReceipt>();
      vi.mocked(commitSoulImportV1).mockReturnValueOnce(pending.promise);
      const user = userEvent.setup();
      const previous = render(<Import />);
      await user.click(await stage(user));
      previous.unmount();
      render(<Import />);
      await stage(user);
      await act(async () => {
        if (outcome === "success") pending.resolve(anImportReceipt());
        else pending.reject({ reason_code: "ROUTINE", explanation: "旧的合成失败。" });
      });
      expect(screen.getByTestId("preview-counts")).toBeVisible();
      expect(screen.queryByTestId("receipt-events")).toBeNull();
      expect(screen.queryByRole("alert")).toBeNull();
      expect(commitSoulImportV1).toHaveBeenCalledTimes(1);
    },
  );

  it("can preview and confirm after StrictMode replays effect setup", async () => {
    const user = userEvent.setup();
    render(<StrictMode><Import /></StrictMode>);
    await user.click(await stage(user));
    await screen.findByTestId("receipt-events");
    expect(commitSoulImportV1).toHaveBeenCalledExactlyOnceWith(TEXT);
  });
});
