import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { createGrid } from "../../algo/grid.ts";
import { parseBeadprojText } from "../../schema/beadproj.ts";
import { asPatternId, mintProjectId } from "../../stores/ids.ts";
import { createPatternDoc } from "../../stores/patterns.ts";
import { createInMemoryRepository } from "../../stores/repository.ts";
import type {
  InventoryEntry,
  PatternDoc,
  ProgressCursor,
  Project,
} from "../../stores/types.ts";
import { renderApp, type RenderAppOptions } from "../../test/render.tsx";

/**
 * T-IE-13…19: the two import panels and the two export controls, driven
 * through the real route table.
 *
 * The object-URL factory is recorded rather than counted after the fact, so
 *「minted on click, not in a render」 (B05 review MED-1, D-IE-16) is an
 * assertion about *when* rather than about how many. The anchor's own `click`
 * is stubbed for the same reason a test never really downloads: it lets the
 * file name and the bytes be read straight off the element.
 */

const { decodeSpy } = vi.hoisted(() => ({ decodeSpy: vi.fn() }));

vi.mock("../../algo/decode.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../algo/decode.ts")>();
  return {
    ...actual,
    decodeImage: (...args: Parameters<typeof actual.decodeImage>) => {
      decodeSpy(...args);
      return actual.decodeImage(...args);
    },
  };
});

const PNG_BYTES = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d];
const PAT_BYTES = [0x50, 0x41, 0x54, 0x02, 0x00, 0x11, 0x22, 0x33];

const urlApi = globalThis.URL as unknown as Record<string, unknown>;
const originalCreate = urlApi["createObjectURL"];
const originalRevoke = urlApi["revokeObjectURL"];

const minted: { url: string; blob: Blob }[] = [];
const revoked: string[] = [];
const downloads: { name: string; href: string }[] = [];

beforeAll(() => {
  urlApi["createObjectURL"] = (blob: Blob) => {
    const url = `blob:bead-test/${minted.length + 1}`;
    minted.push({ url, blob });
    return url;
  };
  urlApi["revokeObjectURL"] = (url: string) => {
    revoked.push(url);
  };
});

afterAll(() => {
  urlApi["createObjectURL"] = originalCreate;
  urlApi["revokeObjectURL"] = originalRevoke;
});

beforeEach(() => {
  minted.length = 0;
  revoked.length = 0;
  downloads.length = 0;
  vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
    this: HTMLAnchorElement,
  ) {
    downloads.push({ name: this.download, href: this.getAttribute("href") ?? "" });
  });
});

const PANEL_LABEL = {
  "import-pattern": "导入已有豆图 说明",
  "import-project": "导入项目库 说明",
} as const;

async function openPanel(
  entry: keyof typeof PANEL_LABEL,
  options: Omit<RenderAppOptions, "route"> = {},
) {
  const app = renderApp({ route: `/create?entry=${entry}`, ...options });
  const panel = await screen.findByRole("region", { name: PANEL_LABEL[entry] });
  // `applyAccept: false`: the panel's own gate is what these cases are about,
  // not the picker dialog's filter.
  const picker = userEvent.setup({ applyAccept: false });
  const pick = (file: File) => picker.upload(within(panel).getByLabelText(/选择/), file);
  return { ...app, panel, pick };
}

function binaryFile(name: string, values: readonly number[]): File {
  return new File([new Uint8Array(values)], name);
}

function textFile(name: string, text: string): File {
  return new File([text], name, { type: "application/json" });
}

function entryOf(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    project: {
      title: "元宵提灯",
      sourcePatternId: null,
      status: "todo",
      createdAt: 1_730_000_000_000,
      backdrop: "black",
      backdropColor: "#101014",
    },
    pattern: { paletteId: "generic-5mm", width: 2, height: 2, cells: [0, 1, -1, 2] },
    ...overrides,
  };
}

function archive(entries: unknown[], inventory?: unknown[]): string {
  return JSON.stringify({
    format: "beadproj",
    version: 1,
    projects: entries,
    ...(inventory === undefined ? {} : { inventory }),
  });
}

function converted(title: string): Project {
  return {
    id: mintProjectId(),
    title,
    sourcePatternId: null,
    status: "active",
    createdAt: 1_730_000_000_000,
    backdrop: "black",
    backdropColor: "#101014",
  };
}

function galleryProject(title: string): Project {
  return {
    id: mintProjectId(),
    title,
    sourcePatternId: asPatternId("gal-slime-01"),
    status: "todo",
    createdAt: 1_720_000_000_000,
    backdrop: "black",
    backdropColor: "#101014",
  };
}

function docFor(project: Project): PatternDoc {
  return createPatternDoc(project.id, createGrid(2, 2, [0, 1, null, 2]), {
    kind: "PixelArt",
    ditherApplied: false,
  });
}

async function downloadedFile(): Promise<ReturnType<typeof parseBeadprojText>> {
  expect(minted).toHaveLength(1);
  return parseBeadprojText(await minted[0]!.blob.text());
}

describe("T-IE-13 两个入口现在都是面板，不是占位说明", () => {
  it("入口卡标「现在就能用」，面板里是文件输入", async () => {
    const { user } = renderApp({ route: "/create" });
    await screen.findByRole("heading", { level: 1, name: "创作与导入" });

    for (const [label, region] of [
      ["导入已有豆图", PANEL_LABEL["import-pattern"]],
      ["导入项目库", PANEL_LABEL["import-project"]],
    ] as const) {
      const card = screen.getByRole("link", { name: label }).closest("li");
      expect(within(card!).getByText("现在就能用")).toBeInTheDocument();

      await user.click(screen.getByRole("link", { name: label }));
      const panel = await screen.findByRole("region", { name: region });
      expect(within(panel).getByLabelText(/选择/)).toHaveAttribute("type", "file");
      expect(panel).not.toHaveTextContent("不是缺陷");
      expect(panel).not.toHaveTextContent("WP-B07");
    }
  });

  it("直接用带 ?entry= 的地址进来也一样（D-UI-1：零新路由）", async () => {
    const { panel, router } = await openPanel("import-project");
    expect(within(panel).getByLabelText(/选择 .beadproj/)).toHaveAttribute("type", "file");
    expect(router.state.location.pathname).toBe("/create");
    expect(router.state.location.search).toBe("?entry=import-project");
  });
});

describe("T-IE-14 文件门与 JSON 门", () => {
  it(">20MB 在读取字节之前就拒绝，内联 FILE_TOO_LARGE", async () => {
    const { pick } = await openPanel("import-project");
    const huge = textFile("big.beadproj", "{}");
    Object.defineProperty(huge, "size", { value: 21 * 1024 * 1024 });
    const read = vi.spyOn(huge, "text");

    await pick(huge);

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("FILE_TOO_LARGE");
    expect(alert).toHaveTextContent("20MB");
    expect(read).not.toHaveBeenCalled();
  });

  it("坏 JSON → 内联 SCHEMA_INVALID", async () => {
    const { pick } = await openPanel("import-project");
    await pick(textFile("broken.beadproj", "{ 这不是 JSON"));
    expect(await screen.findByRole("alert")).toHaveTextContent("SCHEMA_INVALID");
  });

  it("version 2 → UNSUPPORTED_VERSION，不猜测未来版本", async () => {
    const { pick } = await openPanel("import-project");
    await pick(
      textFile("future.beadproj", JSON.stringify({ format: "beadproj", version: 2, projects: [] })),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent("UNSUPPORTED_VERSION");
  });

  it("图片面同样先过文件门", async () => {
    const { pick } = await openPanel("import-pattern");
    const huge = binaryFile("huge.pat", PAT_BYTES);
    Object.defineProperty(huge, "size", { value: 20 * 1024 * 1024 + 1 });
    await pick(huge);
    expect(await screen.findByRole("alert")).toHaveTextContent("FILE_TOO_LARGE");
  });
});

describe("T-IE-15 import-pattern：识别即止，不解析、不解码", () => {
  it(".pat / .gamedev → UNSUPPORTED_FORMAT，文案不许诺解析能力", async () => {
    const { pick } = await openPanel("import-pattern");
    for (const name of ["旧作.pat", "旧作.gamedev"]) {
      await pick(binaryFile(name, PAT_BYTES));
      const alert = await screen.findByRole("alert");
      expect(alert).toHaveTextContent("UNSUPPORTED_FORMAT");
      expect(alert).toHaveTextContent(name);
      expect(alert).toHaveTextContent("暂不支持解析");
      expect(alert.textContent).not.toMatch(/正在解析|解析中|即将支持/);
    }
    expect(decodeSpy).not.toHaveBeenCalled();
  });

  it("PNG 字节 → 改道提示 + 指向 ?entry=upload 的链接，全程零 decodeImage", async () => {
    const { pick } = await openPanel("import-pattern");
    await pick(binaryFile("photo.png", PNG_BYTES));

    const notice = await screen.findByRole("status");
    expect(notice).toHaveTextContent("上传入口");
    expect(within(notice).getByRole("link", { name: "去上传入口" })).toHaveAttribute(
      "href",
      "/create?entry=upload",
    );
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(decodeSpy).not.toHaveBeenCalled();
  });

  it("扔进来的 .beadproj 被认出来，指路到导入项目库", async () => {
    const { pick } = await openPanel("import-pattern");
    await pick(textFile("lib.beadproj", archive([entryOf()])));

    const notice = await screen.findByRole("status");
    expect(within(notice).getByRole("link", { name: "去导入项目库" })).toHaveAttribute(
      "href",
      "/create?entry=import-project",
    );
  });

  it("识别面一个字节都不写盘", async () => {
    const repository = createInMemoryRepository();
    const save = vi.spyOn(repository, "savePatternDoc");
    const { pick } = await openPanel("import-pattern", { repository });
    await pick(binaryFile("photo.png", PNG_BYTES));
    await screen.findByRole("status");
    expect(save).not.toHaveBeenCalled();
    expect(await repository.loadProjects()).toEqual([]);
  });
});

describe("T-IE-16 导入成功：汇总、工作台、拼装台", () => {
  it("汇总进 role=status，项目现身工作台，带文档的项目能拼", async () => {
    const { pick, user } = await openPanel("import-project");
    await pick(
      textFile(
        "lib.beadproj",
        archive(
          [
            entryOf(),
            entryOf({
              project: {
                title: "画廊来的",
                sourcePatternId: "gal-slime-01",
                status: "todo",
                createdAt: 1_720_000_000_000,
                backdrop: "black",
                backdropColor: "#101014",
              },
              pattern: undefined,
            }),
          ],
          [{ paletteId: "generic-5mm", code: "G07", name: "Silver", hex: "#b7bfc6", beads: 500 }],
        ),
      ),
    );

    const status = await screen.findByText(/已导入/);
    expect(status).toHaveAttribute("role", "status");
    expect(status).toHaveTextContent("已导入 2 / 2 个项目");
    expect(status).toHaveTextContent("库存新增 1 / 跳过 0 / 丢弃 0");

    await user.click(screen.getByRole("link", { name: "拼装台" }));
    expect(await screen.findByText("元宵提灯")).toBeInTheDocument();
    expect(screen.getByText("画廊来的")).toBeInTheDocument();

    const card = screen.getByText("元宵提灯").closest("div")!;
    await user.click(within(card).getByRole("link", { name: "继续拼豆" }));
    expect(await screen.findByTestId("assemble-grid")).toBeInTheDocument();
  });

  it("被拒绝的条目逐条列出序号与原因，好的照样进去", async () => {
    const { pick } = await openPanel("import-project");
    await pick(
      textFile(
        "lib.beadproj",
        archive([
          entryOf(),
          entryOf({ pattern: { paletteId: "generic-5mm", width: 57, height: 10, cells: [] } }),
          entryOf({ project: { title: "缺字段" } }),
        ]),
      ),
    );

    expect(await screen.findByText(/已导入 1 \/ 3/)).toBeInTheDocument();
    const list = screen.getByRole("list", { name: "被拒绝的条目" });
    expect(within(list).getAllByRole("listitem")).toHaveLength(2);
    expect(list).toHaveTextContent("第 2 条");
    expect(list).toHaveTextContent("GRID_TOO_LARGE_FOR_V0");
    expect(list).toHaveTextContent("57×10");
    expect(list).toHaveTextContent("第 3 条");
  });

  it("全空网格的项目合法：拼装侧给空网格态，不崩", async () => {
    const { pick, user } = await openPanel("import-project");
    await pick(
      textFile(
        "blank.beadproj",
        archive([
          entryOf({
            project: {
              title: "空板",
              sourcePatternId: null,
              status: "draft",
              createdAt: 1_730_000_000_000,
              backdrop: "black",
              backdropColor: "#101014",
            },
            pattern: {
              paletteId: "generic-5mm",
              width: 4,
              height: 4,
              cells: new Array<number>(16).fill(-1),
            },
          }),
        ]),
      ),
    );
    await screen.findByText(/已导入 1 \/ 1/);

    await user.click(screen.getByRole("link", { name: "拼装台" }));
    const card = (await screen.findByText("空板")).closest("div")!;
    await user.click(within(card).getByRole("link", { name: "编辑豆图" }));
    expect(await screen.findByRole("heading", { level: 1, name: /空板/ })).toBeInTheDocument();
  });
});

describe("T-IE-17 单项目导出：点击时才铸 blob URL", () => {
  const project = converted("元宵提灯");
  const cursor: ProgressCursor = {
    projectId: project.id,
    mode: "tile",
    stepIndex: 2,
    elapsedMs: 1_000,
    updatedAt: 9,
  };

  it("渲染期零 createObjectURL，点击后恰好一个，用后 revoke", async () => {
    const { user } = renderApp({
      route: "/workspace",
      seed: { projects: [project], progress: [cursor] },
      patterns: [docFor(project)],
    });

    const button = await screen.findByRole("button", { name: "导出 .beadproj" });
    // MED-1 的反例锁死：链接渲染出来了，URL 还没铸。
    expect(minted).toHaveLength(0);

    await user.click(button);
    expect(minted).toHaveLength(1);
    expect(downloads).toEqual([{ name: "元宵提灯.beadproj", href: minted[0]!.url }]);

    await waitFor(() => expect(revoked).toEqual([minted[0]!.url]));
  });

  it("导出内容恰好一条 entry，过得了自己的校验器，带上游标", async () => {
    const { user } = renderApp({
      route: "/workspace",
      seed: { projects: [project], progress: [cursor] },
      patterns: [docFor(project)],
    });
    await user.click(await screen.findByRole("button", { name: "导出 .beadproj" }));

    const parsed = await downloadedFile();
    expect(parsed.ok).toBe(true);
    if (!parsed.ok) return;
    expect(parsed.document.entries).toHaveLength(1);
    const only = parsed.document.entries[0]!.entry;
    expect(only.project.title).toBe("元宵提灯");
    expect(only.pattern?.cells).toEqual([0, 1, -1, 2]);
    expect(only.pattern?.provenance).toEqual({ kind: "PixelArt", ditherApplied: false });
    expect(only.progress).toEqual({ mode: "tile", stepIndex: 2, elapsedMs: 1_000, updatedAt: 9 });
    expect(await minted[0]!.blob.type).toBe("application/json");
  });

  it("画廊项目的卡片没有导出链接（判别式同 D-ED-1）", async () => {
    const fixture = galleryProject("史莱姆");
    renderApp({ route: "/workspace", seed: { projects: [fixture] } });
    const card = (await screen.findByText("史莱姆")).closest("div")!;
    expect(within(card).queryByRole("button", { name: "导出 .beadproj" })).not.toBeInTheDocument();
  });

  it("无来源但读不到文档的项目也没有导出链接", async () => {
    const orphan = converted("丢了网格的");
    renderApp({ route: "/workspace", seed: { projects: [orphan] } });
    await screen.findByText("丢了网格的");
    await waitFor(() =>
      expect(screen.queryByRole("button", { name: "导出 .beadproj" })).not.toBeInTheDocument(),
    );
  });
});

describe("T-IE-18 导出全部", () => {
  const project = converted("元宵提灯");
  const fixture = galleryProject("史莱姆");
  const stock: InventoryEntry[] = [
    { paletteId: "gallery", code: "G07", name: "苔绿", hex: "#4c7a44", beads: 120 },
  ];

  it("内容含全部项目（画廊 entry 无 pattern）、库存与游标", async () => {
    const { user } = await openPanel("import-project", {
      seed: {
        projects: [project, fixture],
        inventory: stock,
        progress: [
          {
            projectId: project.id,
            mode: "row-by-row",
            stepIndex: 1,
            elapsedMs: 30,
            updatedAt: 11,
          },
        ],
      },
      patterns: [docFor(project)],
    });

    await user.click(screen.getByRole("button", { name: "导出全部" }));
    await screen.findByText(/已导出 2 个项目/);
    expect(downloads[0]?.name).toBe("bead-projects.beadproj");

    const parsed = await downloadedFile();
    expect(parsed.ok).toBe(true);
    if (!parsed.ok) return;
    const [first, second] = parsed.document.entries.map((accepted) => accepted.entry);
    expect(first!.project.title).toBe("元宵提灯");
    expect(first!.pattern?.width).toBe(2);
    expect(first!.progress?.mode).toBe("row-by-row");
    expect(second!.project.sourcePatternId).toBe("gal-slime-01");
    expect(second!.pattern).toBeUndefined();
    expect(parsed.document.inventory).toEqual(stock);
  });

  it("导出全程零写盘", async () => {
    const repository = createInMemoryRepository(
      { projects: [fixture], inventory: stock },
      [],
    );
    const save = vi.spyOn(repository, "savePatternDoc");
    const saveProjects = vi.spyOn(repository, "saveProjects");
    const { user } = await openPanel("import-project", { repository });

    await user.click(screen.getByRole("button", { name: "导出全部" }));
    await screen.findByText(/已导出 1 个项目/);
    expect(save).not.toHaveBeenCalled();
    // 水合后的那次回写不算导出写的；导出本身没有再触发一次。
    expect(saveProjects.mock.calls.length).toBeLessThanOrEqual(1);
  });

  it("项目与库存皆空 → 控件不渲染（D-INV-11 镜像）", async () => {
    const { panel } = await openPanel("import-project", { seed: { projects: [], inventory: [] } });
    await waitFor(() =>
      expect(within(panel).queryByRole("button", { name: "导出全部" })).not.toBeInTheDocument(),
    );
  });

  it("只有库存也值得备份：控件在，归档里项目为空", async () => {
    const { user } = await openPanel("import-project", { seed: { inventory: stock } });
    await user.click(await screen.findByRole("button", { name: "导出全部" }));
    await screen.findByText(/已导出 0 个项目/);

    const parsed = await downloadedFile();
    expect(parsed.ok).toBe(true);
    if (!parsed.ok) return;
    expect(parsed.document.entries).toEqual([]);
    expect(parsed.document.inventory).toEqual(stock);
  });

  it("导出的归档能原样导回来（面板到面板的闭环）", async () => {
    const { user } = await openPanel("import-project", {
      seed: { projects: [project], inventory: stock },
      patterns: [docFor(project)],
    });
    await user.click(screen.getByRole("button", { name: "导出全部" }));
    await screen.findByText(/已导出 1 个项目/);
    const text = await minted[0]!.blob.text();

    // 换一台「机器」：卸掉这棵树，导出的字节是唯一带过去的东西。
    cleanup();
    const second = await openPanel("import-project");
    await second.pick(textFile("round-trip.beadproj", text));
    expect(await screen.findByText(/已导入 1 \/ 1/)).toBeInTheDocument();
  });
});
