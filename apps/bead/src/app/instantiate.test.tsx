import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";

import { currentPath, renderApp } from "../test/render.tsx";
import { ID_PREFIX } from "../stores/ids.ts";

/**
 * Point 7 of the IA review's B02 acceptance line: a gallery pattern must be
 * instantiable into a project, otherwise Workspace stays empty forever and
 * `/assemble/:id` is unreachable, which makes the navigation tests hollow.
 */
describe("图纸实例化为项目", () => {
  it("「转入工作台」建项目并跳到拼装台，再从那里能进 /assemble/proj-*", async () => {
    const { user, router } = renderApp({ route: "/pattern/gal-slime-01" });

    await user.click(await screen.findByRole("button", { name: "转入工作台" }));
    await waitFor(() => expect(currentPath(router)).toBe("/workspace"));

    const inProgress = await screen.findByRole("region", { name: "正在拼" });
    const resume = within(inProgress).getByRole("link", { name: "继续拼豆" });
    const href = resume.getAttribute("href") ?? "";
    expect(href.startsWith(`/assemble/${ID_PREFIX.project}`)).toBe(true);

    await user.click(resume);
    await waitFor(() => expect(currentPath(router)).toBe(href));
    expect(await screen.findByRole("heading", { level: 1, name: "史莱姆小队" })).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "主导航" })).not.toBeInTheDocument();
  });

  it("「加入待拼」留在原地，并让待办计数变成 1", async () => {
    const { user, router } = renderApp({ route: "/pattern/gal-torii-02" });

    await user.click(await screen.findByRole("button", { name: "加入待拼" }));
    expect(currentPath(router)).toBe("/pattern/gal-torii-02");
    expect(await screen.findByRole("status")).toHaveTextContent("已加入待拼：夏日鸟居");

    const nav = screen.getByRole("navigation", { name: "主导航" });
    await user.click(within(nav).getByRole("link", { name: "灵感" }));

    const strip = await screen.findByRole("region", { name: "个人条" });
    expect(within(strip).getByRole("link", { name: /待办/ })).toHaveTextContent("1");
  });

  it("收藏在个人条里立刻计数", async () => {
    const { user } = renderApp({ route: "/pattern/gal-slime-01" });

    await user.click(await screen.findByRole("button", { name: "收藏" }));
    expect(screen.getByRole("button", { name: "已收藏" })).toHaveAttribute("aria-pressed", "true");

    const nav = screen.getByRole("navigation", { name: "主导航" });
    await user.click(within(nav).getByRole("link", { name: "灵感" }));
    const strip = await screen.findByRole("region", { name: "个人条" });
    expect(within(strip).getByRole("link", { name: /收藏/ })).toHaveTextContent("1");
  });
});
