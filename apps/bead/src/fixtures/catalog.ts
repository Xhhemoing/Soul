import { asCreatorId, asPatternId } from "../stores/ids.ts";
import type { Creator, Pattern } from "../stores/types.ts";

// Small on purpose (R3): localStorage is the B02 backing, so the fixture keeps
// palettes short. WP-B08 grows this set; nothing else here should change.
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
];

export const PATTERNS: Pattern[] = [
  {
    id: asPatternId("gal-slime-01"),
    title: "史莱姆小队",
    creatorId: asCreatorId("cr-mira"),
    tags: ["像素游戏", "二次元"],
    difficulty: 1,
    beadCount: 412,
    estimatedMinutes: 35,
    board: "square-28",
    palette: [
      { code: "H02", name: "薄荷绿", hex: "#7fd6a2", beads: 168 },
      { code: "H10", name: "深松绿", hex: "#2f7d55", beads: 96 },
      { code: "C01", name: "纯白", hex: "#ffffff", beads: 84 },
      { code: "B05", name: "墨黑", hex: "#1b1b1f", beads: 64 },
    ],
  },
  {
    id: asPatternId("gal-torii-02"),
    title: "夏日鸟居",
    creatorId: asCreatorId("cr-tonoya"),
    tags: ["二次元", "节日限定"],
    difficulty: 3,
    beadCount: 1_540,
    estimatedMinutes: 150,
    board: "square-56",
    palette: [
      { code: "R04", name: "朱红", hex: "#d8412f", beads: 520 },
      { code: "Y03", name: "暖砂", hex: "#e8c97a", beads: 380 },
      { code: "G07", name: "苔绿", hex: "#4c7a44", beads: 340 },
      { code: "B05", name: "墨黑", hex: "#1b1b1f", beads: 300 },
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
    beadCount: 760,
    estimatedMinutes: 70,
    board: "square-28",
    palette: [
      { code: "R04", name: "朱红", hex: "#d8412f", beads: 320 },
      { code: "Y01", name: "明黄", hex: "#f5d13b", beads: 250 },
      { code: "B05", name: "墨黑", hex: "#1b1b1f", beads: 190 },
    ],
  },
  {
    id: asPatternId("gal-arcade-05"),
    title: "街机手柄",
    creatorId: asCreatorId("cr-mira"),
    tags: ["像素游戏"],
    difficulty: 2,
    beadCount: 980,
    estimatedMinutes: 90,
    board: "square-56",
    palette: [
      { code: "N06", name: "浅灰", hex: "#c9ccd4", beads: 420 },
      { code: "B05", name: "墨黑", hex: "#1b1b1f", beads: 330 },
      { code: "R04", name: "朱红", hex: "#d8412f", beads: 230 },
    ],
  },
];
