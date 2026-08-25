import { describe, expect, it } from "vitest";
import { screen, within } from "@testing-library/react";

import { renderApp } from "../../test/render.tsx";

/**
 * R6: a stub that does not name its owner gets filed as a bug — so B07 keeps
 * its owner labels here, unchanged.
 *
 * WP-B03 and now WP-B06 have landed, so neither the upload entry nor the blank
 * one is a stub any more. The pixel-editor IA §5.4 maps each retired assertion
 * onto its replacement: the「归 WP-B06」label becomes the「现在就能用」label,
 * and the "this is a placeholder, not a defect" panel case moves to an entry
 * that is still a stub (`import-pattern`, WP-B07) so the invariant it guards
 * keeps a case of its own. `edit.test.tsx` owns the mint form's behaviour and
 * `upload.test.tsx` the workbench's; this file only pins that the entries reach
 * them.
 */
describe("/create 入口列表", () => {
  it("已实现的入口标「现在就能用」，未实现的仍标出归属工作包", async () => {
    renderApp({ route: "/create" });
    await screen.findByRole("heading", { level: 1, name: "创作与导入" });

    const upload = screen.getByRole("link", { name: "上传图片转豆图" }).closest("li");
    expect(upload).not.toBeNull();
    expect(within(upload!).getByText("现在就能用")).toBeInTheDocument();
    expect(screen.queryByText("尚未实现，归 WP-B03")).not.toBeInTheDocument();

    // WP-B06 landed: 空白项目 now mints a board instead of naming an owner.
    const blank = screen.getByRole("link", { name: "空白项目" }).closest("li");
    expect(blank).not.toBeNull();
    expect(within(blank!).getByText("现在就能用")).toBeInTheDocument();
    expect(screen.queryByText("尚未实现，归 WP-B06")).not.toBeInTheDocument();

    expect(screen.getAllByText("尚未实现，归 WP-B07")).toHaveLength(2);
  });
});

describe("/create?entry=upload 打开的是工作台，不是占位说明", () => {
  it("点开上传入口出文件输入，且面板上没有占位话术", async () => {
    const { user } = renderApp({ route: "/create" });
    await user.click(await screen.findByRole("link", { name: "上传图片转豆图" }));

    const panel = await screen.findByRole("region", { name: "上传图片转豆图 说明" });
    expect(within(panel).getByLabelText("选择图片（png / jpg）")).toHaveAttribute("type", "file");
    expect(panel).not.toHaveTextContent("不是缺陷");
    expect(panel).not.toHaveTextContent("WP-B03");
  });

  it("直接用带 ?entry=upload 的地址进来也一样（D-UI-1：选中项在 URL 里）", async () => {
    renderApp({ route: "/create?entry=upload" });
    const panel = await screen.findByRole("region", { name: "上传图片转豆图 说明" });
    // 参数面板要等有图才出现——没选文件时只有文件输入，这条钉的是入口可直链。
    expect(within(panel).getByLabelText("选择图片（png / jpg）")).toBeInTheDocument();
    expect(within(panel).queryByRole("group", { name: "转换参数" })).not.toBeInTheDocument();
  });

  it("B07 的占位面板照旧写明不是缺陷", async () => {
    const { user } = renderApp({ route: "/create" });
    await user.click(await screen.findByRole("link", { name: "导入已有豆图" }));

    const panel = await screen.findByRole("region", { name: "导入已有豆图 说明" });
    expect(panel).toHaveTextContent("WP-B07");
    expect(panel).toHaveTextContent("不是缺陷");
  });
});

describe("/create?entry=blank 打开的是新建表单，不是占位说明（D-ED-4）", () => {
  it("点开空白入口出标题与板型，面板上没有占位话术", async () => {
    const { user } = renderApp({ route: "/create" });
    await user.click(await screen.findByRole("link", { name: "空白项目" }));

    const panel = await screen.findByRole("region", { name: "空白项目 说明" });
    expect(within(panel).getByLabelText("项目名称")).toBeInTheDocument();
    expect(within(panel).getByRole("radio", { name: "28×28" })).toBeChecked();
    expect(within(panel).getByRole("radio", { name: "56×56" })).toBeInTheDocument();
    expect(within(panel).getByRole("button", { name: "新建并开始编辑" })).toBeInTheDocument();
    expect(panel).not.toHaveTextContent("不是缺陷");
    expect(panel).not.toHaveTextContent("WP-B06");
  });

  it("直接用带 ?entry=blank 的地址进来也一样（D-UI-1）", async () => {
    renderApp({ route: "/create?entry=blank" });
    const panel = await screen.findByRole("region", { name: "空白项目 说明" });
    expect(within(panel).getByRole("button", { name: "新建并开始编辑" })).toBeInTheDocument();
  });
});

describe("/create?entry=gallery", () => {
  it("能用的入口不摆占位说明", async () => {
    const { user } = renderApp({ route: "/create" });
    await user.click(await screen.findByRole("link", { name: "从画廊选图纸" }));
    const panel = await screen.findByRole("region", { name: "从画廊选图纸 说明" });
    expect(panel).not.toHaveTextContent("不是缺陷");
    expect(screen.getByRole("link", { name: "去灵感" })).toHaveAttribute("href", "/explore");
  });
});
