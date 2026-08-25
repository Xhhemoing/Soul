import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { fireEvent, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { imageFromPixels, type RgbaImage } from "../../algo/image.ts";
import { createInMemoryRepository, type Repository } from "../../stores/repository.ts";
import { currentPath, renderApp } from "../../test/render.tsx";

/**
 * T-UP-9..15: the `/create?entry=upload` workbench, driven through the real
 * route table.
 *
 * jsdom has neither `createImageBitmap` nor a real 2D context, so the decode
 * step is stubbed to hand back a fixture's raw RGBA — the same technique
 * `decode.test.ts` uses. Everything past that point (classification, framing,
 * quantisation, BOM, the save sequence) is the production code.
 */

const UPLOAD_ROUTE = "/create?entry=upload";

/** 4×4 logical pixels blown up 2×, i.e. what a screenshot of pixel art looks like. */
const PIXEL_ART = imageFromPixels(8, 8, (x, y) =>
  (Math.floor(x / 2) + Math.floor(y / 2)) % 2 === 0
    ? [220, 40, 40, 255]
    : [40, 60, 220, 255],
);

/** A smooth gradient: many colours, almost no flat neighbours. */
const PHOTO = imageFromPixels(16, 16, (x, y) => [x * 15, y * 15, (x + y) * 7, 255]);

/** Same gradient, twice as wide as it is tall — the aspect preset needs a非正方形源. */
const WIDE_PHOTO = imageFromPixels(16, 8, (x, y) => [x * 15, y * 30, (x + y) * 7, 255]);

const TRANSPARENT = imageFromPixels(8, 8, () => [0, 0, 0, 0]);

function stubDecoder(image: RgbaImage): void {
  const close = vi.fn();
  vi.stubGlobal(
    "createImageBitmap",
    vi.fn(async () => ({ width: image.width, height: image.height, close })),
  );
  vi.stubGlobal("OffscreenCanvas", undefined);
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
    imageSmoothingEnabled: true,
    drawImage: vi.fn(),
    getImageData: () => ({ data: image.data }),
  } as unknown as CanvasRenderingContext2D);
}

function file(name: string, type: string): File {
  return new File([new Uint8Array([1, 2, 3])], name, { type });
}

async function openWorkbench(image: RgbaImage, repository?: Repository) {
  stubDecoder(image);
  const app = renderApp(repository === undefined ? { route: UPLOAD_ROUTE } : { route: UPLOAD_ROUTE, repository });
  await screen.findByRole("heading", { level: 1, name: "创作与导入" });
  // `applyAccept: false` so user-event's own `accept` filter does not swallow
  // the rejected file before the page sees it — T-UP-9 is about the inline
  // error the page renders, not about the picker's dialog filter.
  const picker = userEvent.setup({ applyAccept: false });
  const pick = (named: File) => picker.upload(screen.getByLabelText(/选择图片/), named);
  return { ...app, pick };
}

function badges(): string {
  return screen.getByTestId("conversion-grid").parentElement!.querySelector("ul")!.textContent!;
}

// jsdom has no object URLs; the manual viewport wants one for its blob: preview.
const urlApi = globalThis.URL as unknown as Record<string, unknown>;
const originalCreate = urlApi["createObjectURL"];
const originalRevoke = urlApi["revokeObjectURL"];

beforeAll(() => {
  let issued = 0;
  urlApi["createObjectURL"] = () => `blob:bead-test/${(issued += 1)}`;
  urlApi["revokeObjectURL"] = () => {};
});

afterAll(() => {
  if (originalCreate === undefined) delete urlApi["createObjectURL"];
  else urlApi["createObjectURL"] = originalCreate;
  if (originalRevoke === undefined) delete urlApi["revokeObjectURL"];
  else urlApi["revokeObjectURL"] = originalRevoke;
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("T-UP-9 文件门（D-UP-2）", () => {
  it("png 收下并出参数面板", async () => {
    const { pick } = await openWorkbench(PIXEL_ART);
    await pick(file("小猫.png", "image/png"));

    expect(await screen.findByRole("group", { name: "转换参数" })).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("jpg 同样收下", async () => {
    const { pick } = await openWorkbench(PHOTO);
    await pick(file("photo.jpg", "image/jpeg"));
    expect(await screen.findByRole("group", { name: "转换参数" })).toBeInTheDocument();
  });

  it.each([
    ["gif", "animation.gif", "image/gif"],
    ["webp", "shot.webp", "image/webp"],
    ["无类型且扩展名不认识", "clipboard.bin", ""],
  ])("%s 被内联拒绝，参数面板不出现", async (_label, name, type) => {
    const { pick } = await openWorkbench(PIXEL_ART);
    await pick(file(name, type));

    expect(await screen.findByRole("alert")).toHaveTextContent("只支持 png / jpg");
    expect(screen.queryByRole("group", { name: "转换参数" })).not.toBeInTheDocument();
    expect(screen.queryByTestId("conversion-grid")).not.toBeInTheDocument();
  });

  it("类型为空但扩展名是 .png 时按扩展名收下", async () => {
    const { pick } = await openWorkbench(PIXEL_ART);
    await pick(file("从相册来的.PNG", ""));
    expect(await screen.findByRole("group", { name: "转换参数" })).toBeInTheDocument();
  });
});

describe("T-UP-10 路径与置信度（D-UP-4 / BD15）", () => {
  it("自动路径把判定与置信度写在徽标上", async () => {
    const { pick } = await openWorkbench(PIXEL_ART);
    await pick(file("blocks.png", "image/png"));

    await screen.findByTestId("conversion-grid");
    expect(badges()).toMatch(/路径：像素图/);
    expect(badges()).toMatch(/置信度：\d+%/);
    expect(screen.getByRole("radio", { name: "自动" })).toBeChecked();
  });

  it("手动覆盖走 options.kind：同一张图改判成照片", async () => {
    const { user, pick } = await openWorkbench(PIXEL_ART);
    await pick(file("blocks.png", "image/png"));
    await screen.findByTestId("conversion-grid");
    expect(badges()).toMatch(/路径：像素图/);

    await user.click(screen.getByRole("radio", { name: "照片" }));
    expect(await screen.findByText(/路径：照片/)).toBeInTheDocument();
  });
});

describe("T-UP-11 抖动开关只在生效路径可交互（D-UP-5）", () => {
  it("照片路径开抖动 → 徽标亮；切像素图 → 开关禁用、徽标灭", async () => {
    const { user, pick } = await openWorkbench(PHOTO);
    await pick(file("photo.jpg", "image/jpeg"));
    await screen.findByTestId("conversion-grid");

    const dither = screen.getByRole("checkbox", { name: /抖动/ });
    expect(dither).toBeEnabled();
    await user.click(dither);
    expect(await screen.findByText(/抖动：已应用/)).toBeInTheDocument();

    await user.click(screen.getByRole("radio", { name: "像素图" }));
    // 展示以结果的 ditherApplied 为准，不以开关状态为准。
    expect(await screen.findByText(/抖动：未应用/)).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: /抖动/ })).toBeDisabled();
    expect(screen.getByText("像素图路径不做抖动")).toBeInTheDocument();
  });
});

describe("T-UP-12 三预设与 ≤56 夹取（D-UP-6 / D-UP-7）", () => {
  it("固定板默认 28×28，可切 56×56", async () => {
    const { user, pick } = await openWorkbench(PHOTO);
    await pick(file("photo.jpg", "image/jpeg"));

    expect(await screen.findByLabelText(/转换预览：28×28/)).toBeInTheDocument();
    await user.selectOptions(screen.getByLabelText("板子尺寸"), "56");
    expect(await screen.findByLabelText(/转换预览：56×56/)).toBeInTheDocument();
  });

  it("按比例适配保留长宽比", async () => {
    const { user, pick } = await openWorkbench(WIDE_PHOTO);
    await pick(file("wide.jpg", "image/jpeg"));
    await screen.findByTestId("conversion-grid");

    await user.click(screen.getByRole("radio", { name: "按比例适配" }));
    expect(await screen.findByLabelText(/转换预览：28×14/)).toBeInTheDocument();
  });

  it("手动视口按输出格数出网格，超过 56 被夹住", async () => {
    const { user, pick } = await openWorkbench(PHOTO);
    await pick(file("photo.jpg", "image/jpeg"));
    await screen.findByTestId("conversion-grid");

    await user.click(screen.getByRole("radio", { name: "手动视口" }));
    const cells = await screen.findByLabelText("输出格数");
    expect(cells).toHaveAttribute("max", "56");

    fireEvent.change(cells, { target: { value: "12" } });
    expect(await screen.findByLabelText(/转换预览：12×12/)).toBeInTheDocument();

    fireEvent.change(cells, { target: { value: "999" } });
    expect(await screen.findByLabelText(/转换预览：56×56/)).toBeInTheDocument();
  });

  it("取景框方向键平移，网格跟着重算", async () => {
    const { user, pick } = await openWorkbench(PHOTO);
    await pick(file("photo.jpg", "image/jpeg"));
    await screen.findByTestId("conversion-grid");
    await user.click(screen.getByRole("radio", { name: "手动视口" }));

    const frame = await screen.findByRole("group", { name: /取景框/ });
    fireEvent.change(await screen.findByLabelText("取景框边长（源像素）"), {
      target: { value: "8" },
    });
    fireEvent.change(screen.getByLabelText("输出格数"), { target: { value: "8" } });
    const before = (await screen.findByTestId("conversion-grid")).innerHTML;

    frame.focus();
    await user.keyboard("{ArrowRight>4/}");
    // 取景框挪了，取到的就是别的像素——预览必须跟着变。
    await vi.waitFor(() =>
      expect(screen.getByTestId("conversion-grid").innerHTML).not.toBe(before),
    );
  });

  it("取景框也能拖拽平移（D-UP-7）", async () => {
    const { pick } = await openWorkbench(PHOTO);
    await pick(file("photo.jpg", "image/jpeg"));
    await screen.findByTestId("conversion-grid");
    fireEvent.click(screen.getByRole("radio", { name: "手动视口" }));

    const frame = await screen.findByRole("group", { name: /取景框/ });
    fireEvent.change(await screen.findByLabelText("取景框边长（源像素）"), {
      target: { value: "8" },
    });
    fireEvent.change(screen.getByLabelText("输出格数"), { target: { value: "8" } });
    const before = (await screen.findByTestId("conversion-grid")).innerHTML;

    // jsdom 没有版面也没有指针捕获：视口给一个 160px 的假矩形，
    // 于是 80px 的拖拽正好是源图 16 像素宽度的一半。
    frame.setPointerCapture = () => {};
    vi.spyOn(frame.parentElement!, "getBoundingClientRect").mockReturnValue({
      width: 160,
      height: 160,
      x: 0,
      y: 0,
      top: 0,
      left: 0,
      right: 160,
      bottom: 160,
      toJSON: () => ({}),
    });

    expect(frame.style.left).toBe("0%");
    fireEvent.pointerDown(frame, { pointerId: 1, clientX: 0, clientY: 0 });
    fireEvent.pointerMove(frame, { pointerId: 1, clientX: 80, clientY: 0 });
    fireEvent.pointerUp(frame, { pointerId: 1 });

    // 16 像素宽的源图，8 像素的框：右移 8 像素就到头，夹在 50%。
    expect(frame.style.left).toBe("50%");
    await vi.waitFor(() =>
      expect(screen.getByTestId("conversion-grid").innerHTML).not.toBe(before),
    );
  });
});

describe("T-UP-13 空网格禁止保存（D-UP-9）", () => {
  it("整张透明图给出说明，两个 CTA 都不可按", async () => {
    const { pick } = await openWorkbench(TRANSPARENT);
    await pick(file("blank.png", "image/png"));

    expect(await screen.findByText(/全是空格/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "加入待拼" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "转入工作台" })).toBeDisabled();
    expect(screen.getByLabelText(/转换预览：28×28 网格，共 0 颗豆/)).toBeInTheDocument();
  });
});

describe("T-UP-14 保存序列（D-UP-10 / D-UP-11）", () => {
  it("「加入待拼」先落文档再建项目，留在原地并给成功提示", async () => {
    const repository = createInMemoryRepository();
    const { user, pick } = await openWorkbench(PIXEL_ART, repository);
    await pick(file("小猫.png", "image/png"));
    await screen.findByTestId("conversion-grid");

    // D-UP-11: 标题预填文件名去扩展名。
    expect(screen.getByLabelText("项目名称")).toHaveValue("小猫");
    await user.click(screen.getByRole("button", { name: "加入待拼" }));

    expect(await screen.findByText("已加入待拼：小猫")).toBeInTheDocument();
    const projects = await repository.loadProjects();
    expect(projects).toHaveLength(1);
    expect(projects[0]).toMatchObject({ title: "小猫", status: "todo", sourcePatternId: null });

    const doc = await repository.loadPatternDoc(projects[0]!.id);
    expect(doc).not.toBeNull();
    expect(doc?.paletteId).toBe("generic-5mm");
    expect(doc?.provenance).toEqual({ kind: "PixelArt", ditherApplied: false });
  });

  it("「转入工作台」建 active 项目并跳 /workspace", async () => {
    const repository = createInMemoryRepository();
    const { user, router, pick } = await openWorkbench(PIXEL_ART, repository);
    await pick(file("小猫.png", "image/png"));
    await screen.findByTestId("conversion-grid");

    await user.click(screen.getByRole("button", { name: "转入工作台" }));

    await vi.waitFor(() => expect(currentPath(router)).toBe("/workspace"));
    expect((await repository.loadProjects())[0]).toMatchObject({ status: "active" });
  });

  it("空标题回落到「未命名转换」", async () => {
    const repository = createInMemoryRepository();
    const { user, pick } = await openWorkbench(PIXEL_ART, repository);
    await pick(file("小猫.png", "image/png"));
    await screen.findByTestId("conversion-grid");

    await user.clear(screen.getByLabelText("项目名称"));
    await user.click(screen.getByRole("button", { name: "加入待拼" }));

    expect(await screen.findByText("已加入待拼：未命名转换")).toBeInTheDocument();
  });

  it("savePatternDoc 失败 → 内联错误，且一个项目都不 mint", async () => {
    const backing = createInMemoryRepository();
    const repository: Repository = {
      ...backing,
      savePatternDoc: () => Promise.reject(new Error("配额满了")),
    };
    const { user, pick } = await openWorkbench(PIXEL_ART, repository);
    await pick(file("小猫.png", "image/png"));
    await screen.findByTestId("conversion-grid");

    await user.click(screen.getByRole("button", { name: "加入待拼" }));

    expect(await screen.findByText("转换结果没有保存")).toBeInTheDocument();
    expect(await backing.loadProjects()).toEqual([]);
    expect(screen.queryByText(/已加入待拼/)).not.toBeInTheDocument();
  });

  it("确认之前一个字节都不落盘（D-UP-8）", async () => {
    const repository = createInMemoryRepository();
    const { user, pick } = await openWorkbench(PHOTO, repository);
    await pick(file("photo.jpg", "image/jpeg"));
    await screen.findByTestId("conversion-grid");

    await user.click(screen.getByRole("radio", { name: "像素图" }));
    await user.selectOptions(screen.getByLabelText("板子尺寸"), "56");
    await screen.findByLabelText(/转换预览：56×56/);

    expect(await repository.loadProjects()).toEqual([]);
  });
});

describe("BOM 摘要与色号文本（D-UP-9）", () => {
  it("每个色块都带码文本与命名空间标签", async () => {
    const { pick } = await openWorkbench(PIXEL_ART);
    await pick(file("blocks.png", "image/png"));

    const bom = await screen.findByRole("list", { name: "用色清单" });
    const rows = within(bom).getAllByText(/^G\d\d /);
    expect(rows.length).toBeGreaterThan(0);
    expect(within(bom).getAllByText("【通用5mm】").length).toBe(rows.length);
  });
});
