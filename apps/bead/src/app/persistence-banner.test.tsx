import { describe, expect, it } from "vitest";
import { screen } from "@testing-library/react";

import { FlakyStorage } from "../test/flaky-storage.ts";
import { renderApp } from "../test/render.tsx";
import { createRepository } from "../stores/repository.ts";
import { mintProjectId } from "../stores/ids.ts";
import type { Project } from "../stores/types.ts";

const BANNER = "本地存储不可用，本次更改不会保存。";

function project(): Project {
  return {
    id: mintProjectId(),
    title: "夏日鸟居",
    sourcePatternId: null,
    status: "active",
    createdAt: 1,
    backdrop: "black",
    backdropColor: "#101014",
  };
}

describe("DATA-1：存储写不进去时用户看得见", () => {
  it("健康的存储不挂横幅", async () => {
    renderApp({ route: "/workspace" });
    expect(await screen.findByRole("heading", { name: "拼装台" })).toBeInTheDocument();
    expect(screen.queryByText(BANNER)).not.toBeInTheDocument();
  });

  it("写失败后两套外壳都挂上常显横幅", async () => {
    const storage = new FlakyStorage();
    const seeded = project();
    storage.setItem("bead.state", JSON.stringify({ projects: [seeded] }));
    storage.full = true;

    const repository = createRepository(storage);
    const shell = renderApp({ route: "/workspace", repository });
    expect(await screen.findByRole("alert")).toHaveTextContent(BANNER);
    shell.unmount();

    renderApp({ route: `/assemble/${seeded.id}`, repository });
    expect(await screen.findByRole("alert")).toHaveTextContent(BANNER);
  });
});
