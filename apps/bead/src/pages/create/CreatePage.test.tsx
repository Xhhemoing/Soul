import { describe, expect, it } from "vitest";
import { screen, within } from "@testing-library/react";

import { renderApp } from "../../test/render.tsx";

/**
 * R6: a stub that does not name its owner gets filed as a bug — so B06 and B07
 * keep their owner labels here, unchanged.
 *
 * WP-B03 has landed, so the upload entry is no longer one of them. IA §5 maps
 * each retired assertion onto its replacement: the「归 WP-B03」label becomes the
 * 「现在就能用」label, and the "this is a placeholder, not a defect" panel
 * assertion becomes an assertion that the workbench itself is on the panel.
 * `upload.test.tsx` owns the workbench's behaviour; this file only pins that
 * `?entry=upload` reaches it.
 */
describe("/create 入口列表", () => {
  it("已实现的入口标「现在就能用」，未实现的仍标出归属工作包", async () => {
    renderApp({ route: "/create" });
    await screen.findByRole("heading", { level: 1, name: "创作与导入" });

    const upload = screen.getByRole("link", { name: "上传图片转豆图" }).closest("li");
    expect(upload).not.toBeNull();
    expect(within(upload!).getByText("现在就能用")).toBeInTheDocument();
    expect(screen.queryByText("尚未实现，归 WP-B03")).not.toBeInTheDocument();

    expect(screen.getAllByText("尚未实现，归 WP-B07")).toHaveLength(2);
    expect(screen.getByText("尚未实现，归 WP-B06")).toBeInTheDocument();
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

  it("B06 / B07 的占位面板照旧写明不是缺陷", async () => {
    const { user } = renderApp({ route: "/create" });
    await user.click(await screen.findByRole("link", { name: "空白项目" }));

    const panel = await screen.findByRole("region", { name: "空白项目 说明" });
    expect(panel).toHaveTextContent("WP-B06");
    expect(panel).toHaveTextContent("不是缺陷");
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
