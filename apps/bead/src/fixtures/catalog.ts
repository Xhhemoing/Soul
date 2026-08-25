import { asCreatorId, asPatternId } from "../stores/ids.ts";
import type { Creator, Pattern } from "../stores/types.ts";

// Small on purpose (R3): localStorage is the B02 backing, so the fixture keeps
// palettes short. WP-B08 grew this set to nine patterns and three creators and
// is the last work package that does; growth here is append-only (D-GAL-2), so
// no id, palette or grid above ever changes name — an archive exported by WP-B07
// carries `sourcePatternId` and has to keep hitting the catalog (R-IE-4).
//
// D-ASM-2: where a pattern has a grid in `grids.ts`, that grid is the authority
// for `beads` / `beadCount` — the numbers here are derived from it and
// `grids.test.ts` fails if the two drift. Patterns without a grid keep their
// hand-set estimates.
//
// BD15: no field here is a URL and none ever becomes one. There is no social
// backend behind this catalog (BD7), so a creator has a name, a bio and a
// static check-in count and nothing that points off the machine.
//
// Every board here is square: map §3 defers hex and round past v0, and these
// patterns are instantiable, so a non-square one would reach WP-B04's assembler
// as a shape it cannot lay out.

export const TAGS = ["二次元", "像素游戏", "立体拼豆", "节日限定"] as const;

export const CREATORS: Creator[] = [
  {
    id: asCreatorId("cr-mira"),
    name: "Mira 拼豆铺",
    bio: "偏爱像素游戏与节日限定的小图，出图快、配色克制。",
    checkIns: 128,
  },
  {
    id: asCreatorId("cr-tonoya"),
    name: "灯野 Tonoya",
    bio: "做立体拼豆与二次元角色，图纸都标注了板型拼接顺序。",
    checkIns: 74,
  },
  {
    id: asCreatorId("cr-nagi"),
    name: "凪 Nagi",
    bio: "画对称的和风小图与几何挂饰，喜欢把配色压到四色以内。",
    checkIns: 52,
  },
];

export const PATTERNS: Pattern[] = [
  {
    id: asPatternId("gal-slime-01"),
    title: "史莱姆小队",
    creatorId: asCreatorId("cr-mira"),
    tags: ["像素游戏", "二次元"],
    difficulty: 1,
    beadCount: 300,
    estimatedMinutes: 35,
    board: "square-28",
    palette: [
      { code: "H02", name: "薄荷绿", hex: "#7fd6a2", beads: 126 },
      { code: "H10", name: "深松绿", hex: "#2f7d55", beads: 48 },
      { code: "C01", name: "纯白", hex: "#ffffff", beads: 18 },
      { code: "B05", name: "墨黑", hex: "#1b1b1f", beads: 108 },
    ],
  },
  {
    id: asPatternId("gal-torii-02"),
    title: "夏日鸟居",
    creatorId: asCreatorId("cr-tonoya"),
    tags: ["二次元", "节日限定"],
    difficulty: 3,
    beadCount: 1_776,
    estimatedMinutes: 170,
    board: "square-56",
    palette: [
      { code: "R04", name: "朱红", hex: "#d8412f", beads: 790 },
      { code: "Y03", name: "暖砂", hex: "#e8c97a", beads: 190 },
      { code: "G07", name: "苔绿", hex: "#4c7a44", beads: 530 },
      { code: "B05", name: "墨黑", hex: "#1b1b1f", beads: 266 },
    ],
  },
  {
    id: asPatternId("gal-cakebox-03"),
    title: "立体蛋糕盒",
    creatorId: asCreatorId("cr-tonoya"),
    tags: ["立体拼豆", "节日限定"],
    difficulty: 4,
    beadCount: 2_260,
    estimatedMinutes: 240,
    board: "square-28",
    palette: [
      { code: "P02", name: "樱粉", hex: "#f2a7c3", beads: 900 },
      { code: "C01", name: "纯白", hex: "#ffffff", beads: 760 },
      { code: "Y03", name: "暖砂", hex: "#e8c97a", beads: 360 },
      { code: "N06", name: "浅灰", hex: "#c9ccd4", beads: 240 },
    ],
  },
  {
    id: asPatternId("gal-lantern-04"),
    title: "元宵提灯",
    creatorId: asCreatorId("cr-mira"),
    tags: ["节日限定"],
    difficulty: 2,
    beadCount: 340,
    estimatedMinutes: 70,
    board: "square-28",
    palette: [
      { code: "R04", name: "朱红", hex: "#d8412f", beads: 112 },
      { code: "Y01", name: "明黄", hex: "#f5d13b", beads: 76 },
      { code: "B05", name: "墨黑", hex: "#1b1b1f", beads: 152 },
    ],
  },
  {
    id: asPatternId("gal-arcade-05"),
    title: "街机手柄",
    creatorId: asCreatorId("cr-mira"),
    tags: ["像素游戏"],
    difficulty: 2,
    beadCount: 1_238,
    estimatedMinutes: 90,
    board: "square-56",
    palette: [
      { code: "N06", name: "浅灰", hex: "#c9ccd4", beads: 684 },
      { code: "B05", name: "墨黑", hex: "#1b1b1f", beads: 471 },
      { code: "R04", name: "朱红", hex: "#d8412f", beads: 83 },
    ],
  },
  {
    id: asPatternId("gal-gift-06"),
    title: "立体礼盒侧板",
    creatorId: asCreatorId("cr-tonoya"),
    tags: ["立体拼豆", "节日限定"],
    difficulty: 2,
    beadCount: 502,
    estimatedMinutes: 55,
    board: "square-28",
    palette: [
      { code: "L03", name: "湖蓝", hex: "#3f7fbf", beads: 274 },
      { code: "Y01", name: "明黄", hex: "#f5d13b", beads: 112 },
      { code: "B05", name: "墨黑", hex: "#1b1b1f", beads: 98 },
      { code: "C01", name: "纯白", hex: "#ffffff", beads: 18 },
    ],
  },
  {
    id: asPatternId("gal-mochi-07"),
    title: "团子三兄弟",
    creatorId: asCreatorId("cr-nagi"),
    tags: ["二次元", "节日限定"],
    difficulty: 1,
    beadCount: 138,
    estimatedMinutes: 20,
    board: "square-28",
    palette: [
      { code: "W02", name: "竹签棕", hex: "#a9713f", beads: 8 },
      { code: "P02", name: "樱粉", hex: "#f2a7c3", beads: 20 },
      { code: "C01", name: "纯白", hex: "#ffffff", beads: 20 },
      { code: "H05", name: "抹茶绿", hex: "#9fc55a", beads: 20 },
      { code: "B05", name: "墨黑", hex: "#1b1b1f", beads: 70 },
    ],
  },
  {
    id: asPatternId("gal-mush-08"),
    title: "像素蘑菇",
    creatorId: asCreatorId("cr-mira"),
    tags: ["像素游戏"],
    difficulty: 2,
    beadCount: 236,
    estimatedMinutes: 30,
    board: "square-28",
    palette: [
      { code: "R04", name: "朱红", hex: "#d8412f", beads: 64 },
      { code: "C01", name: "纯白", hex: "#ffffff", beads: 48 },
      { code: "B05", name: "墨黑", hex: "#1b1b1f", beads: 78 },
      { code: "Y03", name: "暖砂", hex: "#e8c97a", beads: 46 },
    ],
  },
  {
    id: asPatternId("gal-quilt-09"),
    title: "星纹挂毯",
    creatorId: asCreatorId("cr-nagi"),
    tags: ["节日限定", "像素游戏"],
    difficulty: 3,
    beadCount: 1_105,
    estimatedMinutes: 110,
    board: "square-56",
    palette: [
      { code: "I07", name: "靛蓝", hex: "#2f3a72", beads: 576 },
      { code: "N02", name: "米白", hex: "#f2ead8", beads: 160 },
      { code: "Y05", name: "金黄", hex: "#e0a526", beads: 324 },
      { code: "R04", name: "朱红", hex: "#d8412f", beads: 45 },
    ],
  },
];
