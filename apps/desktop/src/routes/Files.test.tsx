/**
 * The file-plan page as a product surface: pick a directory, read the
 * preview. Nothing here moves, renames, or deletes a file.
 */

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { authorizeRoot } from "../core";
import {
  CLOSED_FILEPLAN,
  forbidNetwork,
  installFakeCore,
  PLAN_PREVIEW_ONLY_EXPLANATION,
  REFUSED_TARGET_MESSAGE,
} from "../test/fakeCore";
import { Files } from "./Files";

describe("文件计划页", () => {
  it("还没有授权目录时，指路去设置页，句子里没有执行一类的词", async () => {
    const attempts = forbidNetwork();
    installFakeCore();
    render(<Files />);

    const empty = await screen.findByTestId("files-no-roots");
    expect(empty).toHaveTextContent("还没有授权任何目录");
    expect(empty.textContent ?? "").not.toMatch(/执行|应用|移动|重命名|删除/);
    expect(within(empty).getByRole("link", { name: "设置" })).toHaveAttribute(
      "href",
      "#/settings",
    );
    expect(attempts).toEqual([]);
  });

  it("授权目录出现在列表里，扫描之后建议表和说明都是核心给的原文", async () => {
    const attempts = forbidNetwork();
    const core = installFakeCore();
    await authorizeRoot("/tmp/soul-fixture");
    render(<Files />);

    const user = userEvent.setup();
    const root = await screen.findByRole("radio", { name: "/tmp/soul-fixture" });
    await user.click(root);
    await user.click(screen.getByRole("button", { name: "扫描并预览" }));

    expect(await screen.findByTestId("fileplan-notice")).toHaveTextContent(
      PLAN_PREVIEW_ONLY_EXPLANATION,
    );
    expect(screen.getByTestId("fileplan-notice").textContent).toBe(PLAN_PREVIEW_ONLY_EXPLANATION);
    expect(screen.getByTestId("fileplan-counts")).toHaveTextContent(
      `${String(CLOSED_FILEPLAN.file_count)} 个文件`,
    );
    expect(screen.getByTestId("fileplan-counts")).toHaveTextContent(
      `${String(CLOSED_FILEPLAN.entry_count)} 条建议`,
    );

    const rows = screen.getAllByTestId("fileplan-row");
    expect(rows).toHaveLength(CLOSED_FILEPLAN.entries.length);
    const first = CLOSED_FILEPLAN.entries[0];
    expect(first).toBeDefined();
    if (first === undefined) throw new Error("the double ships one suggestion");
    expect(rows[0]).toHaveTextContent(first.action_label);
    expect(rows[0]).toHaveTextContent(first.source_rel);
    expect(rows[0]).toHaveTextContent(first.target_rel ?? "");
    for (const row of rows) {
      expect(within(row).queryAllByRole("button")).toEqual([]);
      expect(within(row).queryAllByRole("link")).toEqual([]);
      expect(within(row).queryAllByRole("textbox")).toEqual([]);
    }

    expect(core.callsTo("fileplan_view")).toHaveLength(1);
    expect(attempts).toEqual([]);
  });

  it("没有选目录就扫描时，核心的拒绝语上屏", async () => {
    const attempts = forbidNetwork();
    installFakeCore();
    render(<Files />);

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "扫描并预览" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(REFUSED_TARGET_MESSAGE);
    expect(screen.queryByTestId("fileplan-result")).toBeNull();
    expect(attempts).toEqual([]);
  });

  it("整页没有任何执行控件", async () => {
    const attempts = forbidNetwork();
    installFakeCore();
    render(<Files />);
    await screen.findByTestId("files-no-roots");

    for (const button of screen.queryAllByRole("button")) {
      expect(button.textContent ?? "").not.toMatch(/执行|应用|移动|重命名|删除/);
    }
    expect(screen.getAllByRole("button").map((button) => button.textContent)).toEqual([
      "扫描并预览",
    ]);
    expect(attempts).toEqual([]);
  });
});
