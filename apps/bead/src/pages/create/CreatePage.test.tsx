import { describe, expect, it } from "vitest";
import { screen } from "@testing-library/react";

import { renderApp } from "../../test/render.tsx";

// R6: a stub that does not name its owner gets filed as a bug.
describe("/create 占位入口", () => {
  it("未实现的入口在列表上就标出归属工作包", async () => {
    renderApp({ route: "/create" });
    await screen.findByRole("heading", { level: 1, name: "创作与导入" });
    expect(screen.getByText("尚未实现，归 WP-B03")).toBeInTheDocument();
    expect(screen.getAllByText("尚未实现，归 WP-B07")).toHaveLength(2);
    expect(screen.getByText("尚未实现，归 WP-B06")).toBeInTheDocument();
  });

  it("打开上传入口时说明页写明是占位、不是缺陷", async () => {
    const { user } = renderApp({ route: "/create" });
    await user.click(await screen.findByRole("link", { name: "上传图片转豆图" }));
    const panel = await screen.findByRole("region", { name: "上传图片转豆图 说明" });
    expect(panel).toHaveTextContent("WP-B03");
    expect(panel).toHaveTextContent("不是缺陷");
  });

  it("能用的入口不摆占位说明", async () => {
    const { user } = renderApp({ route: "/create" });
    await user.click(await screen.findByRole("link", { name: "从画廊选图纸" }));
    const panel = await screen.findByRole("region", { name: "从画廊选图纸 说明" });
    expect(panel).not.toHaveTextContent("不是缺陷");
    expect(screen.getByRole("link", { name: "去灵感" })).toHaveAttribute("href", "/explore");
  });
});
