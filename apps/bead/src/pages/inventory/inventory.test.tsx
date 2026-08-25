import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { screen, within } from "@testing-library/react";
import type { UserEvent } from "@testing-library/user-event";

import { FlakyStorage } from "../../test/flaky-storage.ts";
import { renderApp } from "../../test/render.tsx";
import { asPatternId, mintProjectId } from "../../stores/ids.ts";
import { createGrid } from "../../algo/grid.ts";
import { createPatternDoc } from "../../stores/patterns.ts";
import {
  createInMemoryRepository,
  createRepository,
  type Repository,
} from "../../stores/repository.ts";
import {
  buildPurchaseText,
  selectRequirements,
  selectShortages,
  selectSubstituteGroups,
} from "../../stores/inventory.ts";
import {
  GALLERY_PALETTE,
  type InventoryEntry,
  type Project,
  type ProjectStatus,
} from "../../stores/types.ts";

// 画廊 gal-lantern-04 的 palette 就是这一页的 BOM 真源（D-INV-1）：
// R04 朱红 #d8412f 112 / Y01 明黄 #f5d13b 76 / B05 墨黑 #1b1b1f 152。
const LANTERN = "gal-lantern-04";
const BANNER = "本地存储不可用，本次更改不会保存。";

function project(patternId: string | null = LANTERN, status: ProjectStatus = "active"): Project {
  return {
    id: mintProjectId(),
    title: "元宵提灯",
    sourcePatternId: patternId === null ? null : asPatternId(patternId),
    status,
    createdAt: 1,
    backdrop: "black",
    backdropColor: "#101014",
  };
}

function stock(code: string, hex: string, beads: number, name = `名-${code}`): InventoryEntry {
  return { paletteId: GALLERY_PALETTE, code, name, hex, beads };
}

function shortageRegion(): HTMLElement {
  return screen.getByRole("region", { name: "缺口预警" });
}

async function fillStockForm(
  user: UserEvent,
  values: { code: string; name: string; hex: string; beads: string },
): Promise<void> {
  await user.clear(screen.getByLabelText("色号"));
  await user.type(screen.getByLabelText("色号"), values.code);
  await user.clear(screen.getByLabelText("名称"));
  await user.type(screen.getByLabelText("名称"), values.name);
  await user.clear(screen.getByLabelText("色值"));
  await user.type(screen.getByLabelText("色值"), values.hex);
  await user.clear(screen.getByLabelText("颗数"));
  await user.type(screen.getByLabelText("颗数"), values.beads);
  await user.click(screen.getByRole("button", { name: "加入库存" }));
}

// jsdom 不实现 createObjectURL；下载通道本身是运行时 blob:（BD15 安全），
// 这里补一个最小替身，好让 `<a download>` 真的渲染出来被断言。
const objectUrls: string[] = [];
const urlApi = globalThis.URL as unknown as Record<string, unknown>;
const originalCreate = urlApi["createObjectURL"];
const originalRevoke = urlApi["revokeObjectURL"];

beforeAll(() => {
  urlApi["createObjectURL"] = () => {
    const href = `blob:bead-test/${objectUrls.length}`;
    objectUrls.push(href);
    return href;
  };
  urlApi["revokeObjectURL"] = () => {};
});

afterAll(() => {
  if (originalCreate === undefined) delete urlApi["createObjectURL"];
  else urlApi["createObjectURL"] = originalCreate;
  if (originalRevoke === undefined) delete urlApi["revokeObjectURL"];
  else urlApi["revokeObjectURL"] = originalRevoke;
});

describe("T-INV-6 录入（D-INV-4 / D-INV-6）", () => {
  it("填完表单后库存里多一行，码文本在场", async () => {
    const { user } = renderApp({ route: "/inventory" });
    await screen.findByRole("heading", { level: 1, name: "资产" });

    await fillStockForm(user, { code: " r04 ", name: "朱红", hex: "D8412F", beads: "60" });

    const list = screen.getByRole("region", { name: "色号库存" });
    expect(within(list).getByText(/R04 朱红/)).toBeInTheDocument();
    expect(within(list).getByLabelText("R04 颗数")).toHaveValue(60);
    expect(within(list).getByText("#d8412f")).toBeInTheDocument();
  });

  it("重复码给内联错误，不静默合并成第二行", async () => {
    const { user } = renderApp({
      route: "/inventory",
      seed: { inventory: [stock("R04", "#d8412f", 60, "朱红")] },
    });
    await screen.findByRole("heading", { level: 1, name: "资产" });

    await fillStockForm(user, { code: "r04", name: "别的红", hex: "#ff0000", beads: "10" });

    expect(await screen.findByText("色号已存在，请直接修改颗数")).toBeInTheDocument();
    expect(screen.getAllByLabelText("R04 颗数")).toHaveLength(1);
    expect(screen.getByLabelText("R04 颗数")).toHaveValue(60);
  });
});

describe("T-INV-7 改颗数（缺口实时重算）", () => {
  it("库存颗数一改，缺口行跟着变", async () => {
    const { user } = renderApp({
      route: "/inventory",
      seed: { projects: [project()], inventory: [stock("R04", "#d8412f", 100, "朱红")] },
    });
    await screen.findByRole("heading", { level: 1, name: "资产" });
    expect(within(shortageRegion()).getByText("缺 12 颗")).toBeInTheDocument();

    const beads = screen.getByLabelText("R04 颗数");
    await user.clear(beads);
    await user.type(beads, "40");

    expect(within(shortageRegion()).getByText("缺 72 颗")).toBeInTheDocument();
    expect(within(shortageRegion()).getByText("库存 40 颗")).toBeInTheDocument();
  });
});

describe("T-INV-8 删除（D-INV-7：无确认层）", () => {
  it("按「删除 <码>」删掉一行，缺口相应增大", async () => {
    const { user } = renderApp({
      route: "/inventory",
      seed: { projects: [project()], inventory: [stock("R04", "#d8412f", 100, "朱红")] },
    });
    await screen.findByRole("heading", { level: 1, name: "资产" });

    await user.click(screen.getByRole("button", { name: "删除 R04" }));

    expect(screen.queryByLabelText("R04 颗数")).not.toBeInTheDocument();
    expect(within(shortageRegion()).getByText("缺 112 颗")).toBeInTheDocument();
    expect(screen.getByText("还没有库存记录：先在上方录入色号和颗数")).toBeInTheDocument();
  });
});

describe("T-INV-9 非法输入（条目不入库）", () => {
  it("hex 不合法时报错，库存不变", async () => {
    const { user } = renderApp({ route: "/inventory" });
    await screen.findByRole("heading", { level: 1, name: "资产" });

    await fillStockForm(user, { code: "R04", name: "朱红", hex: "#abc", beads: "60" });

    expect(await screen.findByText("色值要写成 #rrggbb 这样的六位十六进制")).toBeInTheDocument();
    expect(screen.queryByLabelText("R04 颗数")).not.toBeInTheDocument();
    expect(screen.getByText("还没有库存记录：先在上方录入色号和颗数")).toBeInTheDocument();
  });

  it("颗数为负或不是整数时报错，库存不变", async () => {
    const { user } = renderApp({ route: "/inventory" });
    await screen.findByRole("heading", { level: 1, name: "资产" });

    await fillStockForm(user, { code: "R04", name: "朱红", hex: "#d8412f", beads: "-5" });
    expect(await screen.findByText("颗数要填 0 到 99999 的整数")).toBeInTheDocument();
    expect(screen.queryByLabelText("R04 颗数")).not.toBeInTheDocument();

    await user.clear(screen.getByLabelText("颗数"));
    await user.type(screen.getByLabelText("颗数"), "3.5");
    await user.click(screen.getByRole("button", { name: "加入库存" }));
    expect(screen.getByText("颗数要填 0 到 99999 的整数")).toBeInTheDocument();
    expect(screen.queryByLabelText("R04 颗数")).not.toBeInTheDocument();
  });

  it("空色号被拦下，错误挂在字段上（aria-describedby）", async () => {
    const { user } = renderApp({ route: "/inventory" });
    await screen.findByRole("heading", { level: 1, name: "资产" });

    await user.type(screen.getByLabelText("色值"), "#d8412f");
    await user.type(screen.getByLabelText("颗数"), "10");
    await user.click(screen.getByRole("button", { name: "加入库存" }));

    const codeField = await screen.findByLabelText("色号");
    expect(codeField).toHaveAttribute("aria-invalid", "true");
    expect(codeField).toHaveAccessibleDescription("请填写色号");
  });
});

describe("T-INV-10 零态矩阵（§7）", () => {
  it("没有在拼项目时缺口区指路 /explore", async () => {
    renderApp({ route: "/inventory" });
    await screen.findByRole("heading", { level: 1, name: "资产" });

    const region = shortageRegion();
    expect(within(region).getByText("没有正在拼或待拼的项目，先去挑一张图纸")).toBeInTheDocument();
    expect(within(region).getByRole("link", { name: "去灵感挑图纸" })).toHaveAttribute(
      "href",
      "/explore",
    );
    expect(
      within(screen.getByRole("region", { name: "近似色替代" })).getByText(
        "没有缺口时不需要替代建议",
      ),
    ).toBeInTheDocument();
  });

  it("在拼的全是空白项目时说明没有配色来源，且不给 CTA", async () => {
    renderApp({ route: "/inventory", seed: { projects: [project(null)] } });
    await screen.findByRole("heading", { level: 1, name: "资产" });

    const region = shortageRegion();
    expect(within(region).getByText("当前项目没有配色清单来源，无法计算缺口")).toBeInTheDocument();
    expect(within(region).queryByRole("link")).not.toBeInTheDocument();
  });

  it("库存覆盖全部需求时说库存足够，且不渲染导出控件", async () => {
    renderApp({
      route: "/inventory",
      seed: {
        projects: [project()],
        inventory: [
          stock("R04", "#d8412f", 112, "朱红"),
          stock("Y01", "#f5d13b", 76, "明黄"),
          stock("B05", "#1b1b1f", 152, "墨黑"),
        ],
      },
    });
    await screen.findByRole("heading", { level: 1, name: "资产" });

    expect(within(shortageRegion()).getByText("库存足够，当前没有缺口")).toBeInTheDocument();
    expect(screen.queryByLabelText("采购清单文本")).not.toBeInTheDocument();
  });

  it("库存为空仍然出全量缺口（D-INV-8），不是零态", async () => {
    renderApp({ route: "/inventory", seed: { projects: [project()] } });
    await screen.findByRole("heading", { level: 1, name: "资产" });

    const region = shortageRegion();
    expect(within(region).getByText("缺 152 颗")).toBeInTheDocument();
    expect(within(region).getByText("缺 112 颗")).toBeInTheDocument();
    expect(within(region).getByText("缺 76 颗")).toBeInTheDocument();
    expect(within(region).getAllByText("库存 0 颗")).toHaveLength(3);
    expect(screen.getByLabelText("采购清单文本")).toBeInTheDocument();
  });

  it("库存区自身零态不再链去 /create，表单就在上方", async () => {
    renderApp({ route: "/inventory" });
    await screen.findByRole("heading", { level: 1, name: "资产" });

    const region = screen.getByRole("region", { name: "色号库存" });
    expect(within(region).getByText("还没有库存记录：先在上方录入色号和颗数")).toBeInTheDocument();
    expect(within(region).queryByRole("link")).not.toBeInTheDocument();
    expect(within(region).getByRole("button", { name: "加入库存" })).toBeInTheDocument();
  });
});

describe("T-INV-11 替代建议永远不只给颜色（D-INV-9）", () => {
  it("候选行同时给码、名、hex 与 ΔE00", async () => {
    renderApp({
      route: "/inventory",
      seed: { projects: [project()], inventory: [stock("G12", "#da4331", 96, "Rose")] },
    });
    await screen.findByRole("heading", { level: 1, name: "资产" });

    const region = screen.getByRole("region", { name: "近似色替代" });
    const candidate = within(region).getByText("G12 Rose").closest("li");
    expect(candidate).not.toBeNull();
    expect(within(candidate!).getByText("#da4331")).toBeInTheDocument();
    expect(within(candidate!).getByText("ΔE00 0.66")).toBeInTheDocument();
    expect(within(candidate!).getByText("余 96 颗")).toBeInTheDocument();
  });

  it("某个缺口色没有够近的候选时，那一组显式说没有", async () => {
    renderApp({ route: "/inventory", seed: { projects: [project()] } });
    await screen.findByRole("heading", { level: 1, name: "资产" });

    const region = screen.getByRole("region", { name: "近似色替代" });
    expect(within(region).getAllByText("库存内没有 ΔE00 < 3 的替代")).toHaveLength(3);
  });
});

describe("T-INV-12 导出采购文本（D-INV-11）", () => {
  it("textarea 内容与 buildPurchaseText 逐字相同，下载走 blob 而不是剪贴板", async () => {
    const inventory = [stock("G12", "#da4331", 96, "Rose")];
    const seeded = project();
    renderApp({ route: "/inventory", seed: { projects: [seeded], inventory } });
    await screen.findByRole("heading", { level: 1, name: "资产" });

    const requirements = selectRequirements([seeded]);
    const shortages = selectShortages(requirements, inventory);
    const expected = buildPurchaseText(
      shortages,
      selectSubstituteGroups(shortages, requirements, inventory),
    );

    expect(screen.getByLabelText("采购清单文本")).toHaveValue(expected);
    expect(expected).not.toMatch(/http|:\/\//);
    const download = screen.getByRole("link", { name: "下载采购清单" });
    expect(download).toHaveAttribute("download", "bead-purchase-list.txt");
    expect(download.getAttribute("href")).toMatch(/^blob:/);
  });
});

describe("T-UP-19 转换项目需求并入缺口（BD20 / §4.3）", () => {
  // generic-5mm 第 7 条是 G07 Silver；画廊 gal-moss-07 的 G07 是苔绿。
  // 同一个码、两个命名空间，页面上必须是两行。
  const converted = project(null, "todo");
  const doc = createPatternDoc(
    converted.id,
    createGrid(4, 1, [6, 6, 6, 6]),
    { kind: "PixelArt", ditherApplied: false },
  );

  it("豆图读回来之后 generic-5mm 的缺口行在场，且带命名空间标签", async () => {
    renderApp({ route: "/inventory", seed: { projects: [converted] }, patterns: [doc] });
    await screen.findByRole("heading", { level: 1, name: "资产" });

    const row = (await within(shortageRegion()).findByText("G07 Silver")).closest("li");
    expect(row).not.toBeNull();
    expect(within(row!).getByText("【通用5mm】")).toBeInTheDocument();
    expect(within(row!).getByText("缺 4 颗")).toBeInTheDocument();
  });

  it("画廊 G07 库存不抵扣转换项目的 G07（T-UP-18 的页面一半）", async () => {
    renderApp({
      route: "/inventory",
      seed: {
        projects: [converted, project(LANTERN)],
        inventory: [stock("G07", "#4c7a44", 500, "苔绿")],
      },
      patterns: [doc],
    });
    await screen.findByRole("heading", { level: 1, name: "资产" });

    const region = shortageRegion();
    expect(await within(region).findByText("G07 Silver")).toBeInTheDocument();
    expect(within(region).queryByText("G07 苔绿")).not.toBeInTheDocument();
    expect(within(region).getByText("缺 4 颗")).toBeInTheDocument();
  });

  it("读取窗口有显式状态，不假装缺口已经算全（R-UP-4）", async () => {
    // 永不 settle 的 loadPatternDoc：加载窗口被钉住，好让状态可断言。
    const repository: Repository = {
      ...createInMemoryRepository({ projects: [converted] }),
      loadPatternDoc: () => new Promise(() => {}),
    };
    renderApp({ route: "/inventory", repository });
    await screen.findByRole("heading", { level: 1, name: "资产" });

    expect(
      within(shortageRegion()).getByText("正在读取转换项目豆图……"),
    ).toBeInTheDocument();
  });
});

describe("T-INV-13 写失败（DATA-1 / D-INV-14）", () => {
  it("横幅常显，库存 CRUD 仍然改得动 UI，且没有第二个错误面", async () => {
    const storage = new FlakyStorage();
    storage.full = true;
    const { user } = renderApp({
      route: "/inventory",
      repository: createRepository(storage),
    });
    await screen.findByRole("heading", { level: 1, name: "资产" });
    expect(await screen.findByText(BANNER)).toBeInTheDocument();

    await fillStockForm(user, { code: "R04", name: "朱红", hex: "#d8412f", beads: "60" });

    expect(screen.getByLabelText("R04 颗数")).toHaveValue(60);
    expect(screen.getAllByText(BANNER)).toHaveLength(1);

    await user.click(screen.getByRole("button", { name: "删除 R04" }));
    expect(screen.queryByLabelText("R04 颗数")).not.toBeInTheDocument();
    expect(screen.getAllByText(BANNER)).toHaveLength(1);
  });
});
