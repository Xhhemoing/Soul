/**
 * The import screen: two named formats, one file input, counts on the way in
 * and counts on the way out.
 *
 * The crates already prove that the parser reads these two files and that what
 * it writes lands sealed. What only this side can check is that a Windows user
 * has a way to reach that parser at all — the gap this route closes — and that
 * the page they reach it through does not put the export's own sentences on
 * screen. The fixture used here is the real one from `fixtures/import/`, and
 * every test asserts against the file's own text rather than a paraphrase of
 * it, so a component that ever rendered the body would fail rather than be
 * noticed by a reviewer.
 */

import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import { Import } from "./Import";
import type { ImportPreview, ImportReceipt } from "../core";
import { denylistHits, diagnosticTerms, renderedText } from "../test/denylist";
import {
  anImportPreview,
  anImportReceipt,
  forbidNetwork,
  IMPORT_LOCAL_ONLY_NOTICE,
  installFakeCore,
  type FakeCoreOptions,
} from "../test/fakeCore";

const REPO = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "..");

function fixture(...parts: string[]): string {
  return readFileSync(join(REPO, "fixtures", "import", ...parts), "utf8");
}

const JSONL = fixture("soul-import-v1", "valid_basic.jsonl");
const TELEGRAM = fixture("telegram", "result_basic.json");

/** A sentence a person wrote, which no part of this screen may ever show. */
const A_SENTENCE = "明天上午十点在公司门口见";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

function fileWithText(name: string, text: Promise<string>): File {
  const file = new File([], name, { type: "application/json" });
  Object.defineProperty(file, "text", { value: () => text });
  return file;
}

async function pick(text: string, name: string, options: FakeCoreOptions = {}) {
  const core = installFakeCore(options);
  const user = userEvent.setup();
  render(<Import />);
  await user.upload(
    screen.getByLabelText("选择文件"),
    new File([text], name, { type: "application/json" }),
  );
  return { core, user };
}

describe("导入页", () => {
  it("写清楚只认哪两种文件，并且给出一个选文件的输入框", () => {
    installFakeCore();
    render(<Import />);

    expect(screen.getByText(/soul-import-v1 JSONL/)).toBeVisible();
    expect(screen.getByText(/Telegram Desktop 的 result.json/)).toBeVisible();
    expect(screen.getByText(/Export Telegram data/)).toBeVisible();
    expect(screen.getByText(/Machine-readable JSON/)).toBeVisible();
    expect(screen.getByLabelText("选择文件")).toHaveAttribute("type", "file");
  });

  /**
   * There is no account to sign into and no archive to open, and the way to
   * check that is the same way the drafting screen checks that it cannot send:
   * nothing on the page can be pressed to do it.
   */
  it("页面上没有登录、授权账号或者解压压缩包的按钮", () => {
    installFakeCore();
    render(<Import />);

    for (const button of screen.queryAllByRole("button")) {
      expect(button.textContent ?? "").not.toMatch(/登录|授权|连接|下载|上传|解压|执行/);
    }
    expect(renderedText()).not.toMatch(/OAuth|\.zip/i);
  });

  it("选中文件之后显示的是计数，不是文件里的话", async () => {
    await pick(JSONL, "valid_basic.jsonl");

    expect(await screen.findByTestId("preview-counts")).toHaveTextContent("4 个人，2 个会话，16 条消息");
    expect(screen.getByTestId("preview-source")).toHaveTextContent("soul-import-v1");
    expect(screen.getByTestId("preview-writes")).toHaveTextContent(
      "人、会话、消息一条都没有写进库里",
    );
    expect(screen.getByTestId("preview-notice")).toHaveTextContent(IMPORT_LOCAL_ONLY_NOTICE);
    expect(renderedText()).not.toContain(A_SENTENCE);
    expect(renderedText()).not.toContain("u-lilei");
  });

  /**
   * 「到这一步还什么都没有写进库里」 was false for exactly the user who most
   * needed it to be true. Both preview commands call `Session::note_injection`
   * before returning, so a file carrying injection markers has already
   * appended an `injection.blocked` row to the audit chain by the time this
   * panel renders, and 换一个文件 leaves that row standing. What the preview
   * can promise is that none of the file's people, conversations or messages
   * were sealed; the audit row is said out loud instead of covered over.
   */
  it("预览只保证这个文件的内容没入库，注入标记留下的审计行照直说", async () => {
    await pick(JSONL, "valid_basic.jsonl", {
      readingImport: (source) => anImportPreview({ source, messages_with_injection_markers: 2 }),
    });

    const writes = await screen.findByTestId("preview-writes");
    expect(writes).toHaveTextContent("人、会话、消息一条都没有写进库里");
    expect(writes).toHaveTextContent("审计链上留下了一行");
    expect(writes.textContent).not.toContain("还什么都没有写进库里");
  });

  /** The whole file went to the core, and none of it came back onto the page. */
  it("整份文本交给核心，屏幕上一句正文都没有", async () => {
    const { core } = await pick(JSONL, "valid_basic.jsonl");
    await screen.findByTestId("preview-counts");

    expect(core.callsTo("preview_soul_import_v1")[0]?.payload).toEqual({ text: JSONL });
    for (const line of JSONL.split("\n").filter((line) => line.trim() !== "")) {
      expect(renderedText()).not.toContain(line);
    }
  });

  it("确认之后把同一份文本交回去，回执还是计数", async () => {
    const { core, user } = await pick(JSONL, "valid_basic.jsonl");
    await screen.findByTestId("preview-counts");

    await user.click(screen.getByRole("button", { name: "确认导入" }));

    expect(await screen.findByTestId("receipt-contacts")).toHaveTextContent("新建 4 个人");
    expect(screen.getByTestId("receipt-events")).toHaveTextContent("写进 16 条往来记录");
    expect(screen.getByTestId("receipt-ties")).toHaveTextContent("重算出 3 条关系");
    expect(core.callsTo("commit_soul_import_v1")[0]?.payload).toEqual({ text: JSONL });
    expect(renderedText()).not.toContain(A_SENTENCE);
    expect(screen.queryByRole("button", { name: "确认导入" })).toBeNull();
  });

  /**
   * A line that tried to give instructions is a count on both screens. It is
   * not quoted, and there is nothing on the page that acts on it — the core
   * stores it as material and says so.
   */
  it("写成命令样子的消息只被数出来，没有被照做", async () => {
    const { user } = await pick(JSONL, "valid_basic.jsonl", {
      readingImport: (source) => anImportPreview({ source, messages_with_injection_markers: 2 }),
      importing: (source) => anImportReceipt({ source, messages_with_injection_markers: 2 }),
    });

    expect(await screen.findByTestId("preview-injection")).toHaveTextContent(
      "有 2 条消息写成了命令的样子",
    );
    await user.click(screen.getByRole("button", { name: "确认导入" }));

    expect(await screen.findByTestId("receipt-injection")).toHaveTextContent("没有被执行");
  });

  /**
   * v0.1 keeps no external-id index, so the same export committed twice writes
   * its events twice and the graph counts every duplicate as a real
   * interaction — a tie can cross a band on duplicate evidence alone. The
   * scoping decision stands; what may not stand is a screen that never says
   * so. Both the screen before the decision and the screen after it warn.
   */
  it("预览和回执都写明同一份文件再导一次会把往来记录再写一遍", async () => {
    const { user } = await pick(JSONL, "valid_basic.jsonl");

    const before = await screen.findByTestId("preview-reimport");
    expect(before).toHaveTextContent("同一份文件再导一次");
    expect(before).toHaveTextContent("往来次数和关系强度");

    await user.click(screen.getByRole("button", { name: "确认导入" }));

    const after = await screen.findByTestId("receipt-reimport");
    expect(after).toHaveTextContent("再导一遍");
    expect(after).toHaveTextContent("人会对上不会重复");
    expect(after).toHaveTextContent("往来次数和关系强度");
  });

  it("挑 Telegram 的时候走的是 Telegram 那条命令", async () => {
    const core = installFakeCore();
    const user = userEvent.setup();
    render(<Import />);

    await user.click(screen.getByLabelText(/Telegram Desktop 的 result.json/));
    await user.upload(
      screen.getByLabelText("选择文件"),
      new File([TELEGRAM], "result.json", { type: "application/json" }),
    );

    expect(await screen.findByTestId("preview-source")).toHaveTextContent("telegram-desktop");
    expect(core.callsTo("preview_telegram")).toHaveLength(1);
    expect(core.callsTo("preview_soul_import_v1")).toHaveLength(0);
    await user.click(screen.getByRole("button", { name: "确认导入" }));
    expect(await screen.findByTestId("receipt-source")).toHaveTextContent("telegram-desktop");
    expect(core.callsTo("commit_telegram")[0]?.payload).toEqual({ text: TELEGRAM });
    expect(core.callsTo("commit_soul_import_v1")).toHaveLength(0);
  });

  it("读文件、等预览和提交期间都不能切换格式", async () => {
    const contents = deferred<string>();
    const preview = deferred<ImportPreview>();
    const receipt = deferred<ImportReceipt>();
    mockIPC((command) => {
      if (command === "preview_soul_import_v1") return preview.promise;
      if (command === "commit_soul_import_v1") return receipt.promise;
      throw new Error(`unexpected command: ${command}`);
    });
    const user = userEvent.setup();
    render(<Import />);
    await user.upload(screen.getByLabelText("选择文件"), fileWithText("first.jsonl", contents.promise));

    const telegram = screen.getByLabelText(/Telegram Desktop 的 result.json/);
    for (const radio of screen.getAllByRole("radio")) expect(radio).toBeDisabled();
    await user.click(telegram);
    expect(telegram).not.toBeChecked();

    await act(async () => contents.resolve(JSONL));
    for (const radio of screen.getAllByRole("radio")) expect(radio).toBeDisabled();
    expect(screen.queryByRole("button", { name: "确认导入" })).toBeNull();
    await act(async () => preview.resolve(anImportPreview()));
    for (const radio of screen.getAllByRole("radio")) expect(radio).toBeEnabled();

    await user.click(screen.getByRole("button", { name: "确认导入" }));
    for (const radio of screen.getAllByRole("radio")) expect(radio).toBeDisabled();
    expect(screen.getByRole("button", { name: "不了，换一个文件" })).toBeDisabled();
    await user.click(telegram);
    expect(telegram).not.toBeChecked();
    await act(async () => receipt.resolve(anImportReceipt()));
    expect(screen.getByTestId("receipt-source")).toHaveTextContent("soul-import-v1");
    for (const radio of screen.getAllByRole("radio")) expect(radio).toBeEnabled();
  });

  it.each(["success", "failure"] as const)("取消文件选择后忽略旧读取的 %s", async (outcome) => {
    const contents = deferred<string>();
    const core = installFakeCore();
    const user = userEvent.setup();
    render(<Import />);
    const input = screen.getByLabelText("选择文件");
    await user.upload(input, fileWithText("old.jsonl", contents.promise));

    // A queued native change may clear the selection after reading started.
    fireEvent.change(input, { target: { files: [] } });
    await act(async () => {
      if (outcome === "success") contents.resolve(JSONL);
      else contents.reject({ reason_code: "IMPORT_UNREADABLE", explanation: "旧文件读失败。" });
    });

    expect(input).toBeEnabled();
    expect(screen.queryByTestId("preview-counts")).toBeNull();
    expect(screen.queryByTestId("import-file-size")).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
    expect(core.callsTo("preview_soul_import_v1")).toHaveLength(0);
  });

  it.each(["success", "failure"] as const)("新文件预览期间忽略旧预览的 %s", async (outcome) => {
    const oldPreview = deferred<ImportPreview>();
    const newPreview = deferred<ImportPreview>();
    const commits: { command: string; payload: unknown }[] = [];
    mockIPC((command, payload) => {
      if (command === "preview_soul_import_v1") return oldPreview.promise;
      if (command === "preview_telegram") return newPreview.promise;
      if (command === "commit_telegram" || command === "commit_soul_import_v1") {
        commits.push({ command, payload });
        return anImportReceipt({ source: "telegram-desktop" });
      }
      throw new Error(`unexpected command: ${command}`);
    });
    const user = userEvent.setup();
    render(<Import />);
    const input = screen.getByLabelText("选择文件");
    await user.upload(input, fileWithText("old.jsonl", Promise.resolve(JSONL)));
    fireEvent.change(input, { target: { files: [] } });
    await user.click(screen.getByLabelText(/Telegram Desktop 的 result.json/));
    // Deliver a queued selection even if the old request incorrectly kept busy set.
    fireEvent.change(input, { target: { files: [fileWithText("result.json", Promise.resolve(TELEGRAM))] } });

    await act(async () => {
      if (outcome === "success") oldPreview.resolve(anImportPreview({ messages: 99 }));
      else oldPreview.reject({ reason_code: "IMPORT_UNREADABLE", explanation: "旧预览失败。" });
    });
    expect(input).toBeDisabled();
    expect(screen.queryByTestId("preview-counts")).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();

    await act(async () => newPreview.resolve(anImportPreview({ source: "telegram-desktop", messages: 7 })));
    expect(screen.getByTestId("preview-source")).toHaveTextContent("telegram-desktop");
    expect(screen.getByTestId("preview-counts")).toHaveTextContent("7 条消息");
    expect(screen.getByTestId("import-file-size")).toHaveTextContent(`${TELEGRAM.length} 个字符`);
    await user.click(screen.getByRole("button", { name: "确认导入" }));
    expect(await screen.findByTestId("receipt-source")).toHaveTextContent("telegram-desktop");
    expect(commits).toEqual([{ command: "commit_telegram", payload: { text: TELEGRAM } }]);
  });

  it("离开导入页后完成的文件读取不再调用预览", async () => {
    const contents = deferred<string>();
    const core = installFakeCore();
    const user = userEvent.setup();
    const view = render(<Import />);
    await user.upload(screen.getByLabelText("选择文件"), fileWithText("old.jsonl", contents.promise));

    view.unmount();
    await act(async () => contents.resolve(JSONL));

    expect(core.callsTo("preview_soul_import_v1")).toHaveLength(0);
  });

  it("提交失败后仍能用已预览的同一份文件重试", async () => {
    let attempts = 0;
    const { core, user } = await pick(JSONL, "valid_basic.jsonl", {
      importing: (source) => {
        if (attempts++ === 0) throw { reason_code: "IMPORT_UNREADABLE", explanation: "这次没有导入。" };
        return anImportReceipt({ source });
      },
    });
    await screen.findByTestId("preview-counts");
    await user.click(screen.getByRole("button", { name: "确认导入" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("这次没有导入");
    expect(screen.getByTestId("preview-counts")).toBeVisible();
    expect(screen.getByRole("button", { name: "确认导入" })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: "确认导入" }));
    expect(await screen.findByTestId("receipt-source")).toHaveTextContent("soul-import-v1");
    expect(screen.queryByRole("alert")).toBeNull();
    expect(core.callsTo("commit_soul_import_v1").map((call) => call.payload)).toEqual([
      { text: JSONL },
      { text: JSONL },
    ]);
  });

  it("放弃预览后清空文件状态，换格式和重新选文件都可继续", async () => {
    const { core, user } = await pick(JSONL, "valid_basic.jsonl");
    await screen.findByTestId("preview-counts");
    await user.click(screen.getByRole("button", { name: "不了，换一个文件" }));

    expect(screen.queryByTestId("preview-counts")).toBeNull();
    expect(screen.queryByTestId("import-file-size")).toBeNull();
    expect(screen.getByLabelText("选择文件")).toBeEnabled();
    await user.click(screen.getByLabelText(/Telegram Desktop 的 result.json/));
    await user.upload(screen.getByLabelText("选择文件"), new File([TELEGRAM], "result.json"));
    expect(await screen.findByTestId("preview-source")).toHaveTextContent("telegram-desktop");
    expect(core.callsTo("commit_soul_import_v1")).toHaveLength(0);
  });

  it("放弃预览后可以重新选择同一个文件", async () => {
    const core = installFakeCore();
    const user = userEvent.setup();
    const file = new File([JSONL], "valid_basic.jsonl", { type: "application/json" });
    render(<Import />);
    const input = screen.getByLabelText("选择文件") as HTMLInputElement;

    await user.upload(input, file);
    await screen.findByTestId("preview-counts");
    await user.click(screen.getByRole("button", { name: "不了，换一个文件" }));
    await user.upload(input, file);

    expect(await screen.findByTestId("preview-counts")).toBeVisible();
    expect(core.callsTo("preview_soul_import_v1")).toHaveLength(2);
  });

  it("预览失败后可以重新选择同一个文件", async () => {
    let attempts = 0;
    const core = installFakeCore({
      readingImport: (source) => {
        if (attempts++ === 0) throw { reason_code: "IMPORT_UNREADABLE", explanation: "预览失败。" };
        return anImportPreview({ source });
      },
    });
    const user = userEvent.setup();
    const file = new File([JSONL], "valid_basic.jsonl", { type: "application/json" });
    render(<Import />);
    const input = screen.getByLabelText("选择文件") as HTMLInputElement;

    await user.upload(input, file);
    expect(await screen.findByRole("alert")).toHaveTextContent("预览失败");
    await user.upload(input, file);

    expect(await screen.findByTestId("preview-counts")).toBeVisible();
    expect(core.callsTo("preview_soul_import_v1")).toHaveLength(2);
  });

  it("导入成功后可以在同一页面重新选择同一个文件", async () => {
    const core = installFakeCore();
    const user = userEvent.setup();
    const file = new File([JSONL], "valid_basic.jsonl", { type: "application/json" });
    render(<Import />);
    const input = screen.getByLabelText("选择文件") as HTMLInputElement;

    await user.upload(input, file);
    await screen.findByTestId("preview-counts");
    await user.click(screen.getByRole("button", { name: "确认导入" }));
    await screen.findByTestId("receipt-source");
    await user.upload(input, file);

    expect(await screen.findByTestId("preview-counts")).toBeVisible();
    expect(core.callsTo("preview_soul_import_v1")).toHaveLength(2);
  });

  /**
   * A file that does not parse arrives as a refusal value, and the refusal is
   * the core's own sentence. The screen shows it without adding a quotation of
   * the line that failed.
   */
  it("读不成的文件给出理由码和核心那句话，不回贴文件内容", async () => {
    await pick(JSONL, "valid_basic.jsonl", {
      readingImport: () => {
        throw {
          reason_code: "IMPORT_UNREADABLE",
          explanation: "这个文件的第 3 行少了必需的字段，这次没有导入任何东西。",
        };
      },
    });

    expect(await screen.findByTestId("import-refusal-code")).toHaveTextContent("IMPORT_UNREADABLE");
    expect(screen.getByRole("alert")).toHaveTextContent("第 3 行少了必需的字段");
    expect(screen.queryByTestId("preview-counts")).toBeNull();
    expect(renderedText()).not.toContain(A_SENTENCE);
  });

  it("认不出哪一个是你的时候，不给按确认的机会", async () => {
    await pick(JSONL, "valid_basic.jsonl", {
      readingImport: (source) => anImportPreview({ source, owner_identified: false }),
    });

    expect(await screen.findByTestId("preview-owner")).toHaveTextContent("导入会被拒绝");
    expect(screen.getByRole("button", { name: "确认导入" })).toBeDisabled();
  });

  it("渲染出来的页面里没有一个诊断词或量表词", async () => {
    const { user } = await pick(JSONL, "valid_basic.jsonl");
    await screen.findByTestId("preview-counts");
    await user.click(screen.getByRole("button", { name: "确认导入" }));
    await screen.findByTestId("receipt-contacts");

    expect(denylistHits(renderedText(), diagnosticTerms())).toEqual([]);
  });

  it("整个导入过程没有碰过网络", async () => {
    const attempts = forbidNetwork();
    const { user } = await pick(JSONL, "valid_basic.jsonl");
    await screen.findByTestId("preview-counts");
    await user.click(screen.getByRole("button", { name: "确认导入" }));
    await screen.findByTestId("receipt-contacts");

    expect(attempts).toEqual([]);
  });
});
