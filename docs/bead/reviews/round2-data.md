# ROUND 2 · WP-B09 数据 / 存储 / 权限审查

Reviewer：`claude-fable-5-thinking-xhigh`（实际运行 Claude Fable 5 thinking，无静默降级；只审不实现）。
基线：`origin/cursor/beadflow-integration-c441` @ `dc4b91d`。
输入：`apps/bead/src/stores/**` 全部 10 个文件、`docs/bead/PLAN.md`、`docs/bead/WORK_PACKAGES.md`（WP-B09）、
`docs/bead/reviews/round1-*.md` 全部 7 份（重点：frontend §5 状态所有权 / R3、map §3 数据库·Storage·可靠性·安全面 / §4 B09、
shell-review §2.9 / SH-4）、`apps/bead/src/algo/`（grid / pipeline / framing / decode，取网格尺寸与导入面事实）、
`apps/bead/src/test/isolation.test.ts`（NE-1 现状）、`apps/bead/src/pages/assemble/AssemblePage.tsx`（进度面现状）。
硬禁区自查：本轮只新增本文件；不触碰 Soul 锁、`apps/desktop`、`crates/soul-*`、根配置、`.github/**`。

---

## 0. 结论先行

**分层裁决，两句话：**

1. **v0 板级规模（≤56×56，fixture 画廊 + B04 步进进度 + B05 库存）：localStorage 仍够，
   本轮对「整体迁 IndexedDB」判 NO_HIGH_VALUE_CHANGE_FOUND**——数字见 §2，余量在两个数量级以上，
   且 D-UI-5 的 async Repository 接缝已铺好，现在迁移只是把风险从「以后可能撞墙」换成「现在就引入
   IDB 的打开失败 / 测试桩 / 迁移路径三个新失败面」，负收益。
2. **例外恰好是任务书点名的「大图进度」所在的那一层：逐格载荷（转换网格本体 + 逐格完成位）
   在 B07 的 512×512 上限下必然击穿 localStorage**（两次导入即近 5MB 配额，§2.3），
   所以最小 IndexedDB 契约在本文 §5 一次写死，**触发点钉死为「第一个把 `Grid` 持久化的工作包落地
   的那个 PR」**（B03 的 `/create` 上传接线，或 B07 导入，谁先到谁背）——在那之前不写一行 IDB 代码。

另有一个**现在就该修**的存量缺陷（DATA-1，§8）：repository 在写失败时静默降级到内存备份、
读却仍回旧 localStorage，等价于静默丢档。它与 IDB 无关、约 15 行，不应等 B09。

## 1. 存储现状盘点（基线树逐处核实）

| 键 / 位置 | 内容 | 写者 | 读者 | 当前体量 |
|---|---|---|---|---|
| localStorage `bead.state` | `{projects, favorites, inventory}` 整包 JSON | 仅 `stores/repository.ts`（三个 save 各自整包重写） | 同文件（hydration 一次 + 每次 save 前 read-modify-write） | 项目记录 ≈200 字符/条，纯元数据 |
| localStorage `bead.theme` | `"light" \| "dark"` | 仅 `stores/theme.tsx` | 同文件 | 5 字符 |
| （无） | 网格 / 转换结果 / 进度 / 计时 | — | — | B03/B04 尚未持久化任何逐格数据 |

核实过的合规点（不重复开票）：

- **页面不直碰存储**：全树 grep，localStorage 只出现在 `repository.ts` 与 `theme.tsx`（IA §5 规则成立）。
- **持久化边界有形状校验**：`parsePersistedState` 丢弃畸形记录而非崩溃，SH-4 的 backdrop 校验已补齐并有测试
  （`repository.test.ts` 三用例）。这是「localStorage 是用户可手改边界」的正确先例，§6 的导入消毒直接沿用同一哲学。
- **写回时机**：`store.tsx` 只在 hydration 完成后写回，防止慢加载清空存档——正确。
- **主题写失败不致命**：`theme.tsx` 的 catch 只放弃持久化、不放弃应用主题——正确。
- **NE-1 已落地**：`isolation.test.ts` 含 no-egress 扫描（`fetch(`/XHR/WebSocket/sendBeacon/远程 URL）
  加扫描器自检（种入必中、react-router 路径必放过）。ROUND 1 shell-review 的 SH-1 已闭环。

## 2. localStorage 上限核算

**配额事实**：主流引擎 ≈5MB / origin（bead dev server 钉独立端口，即独立 origin，不与人分摊）；
Chromium 按 UTF-16 计量，ASCII JSON 的有效载荷约 **2.5M 字符**。所有写入同步、发生在主线程。

### 2.1 现状树（B02 + B05 库存 + 收藏）

100 个项目元数据 + 全量 72 色库存 + 收藏 ≈ **<40KB**。距上限两个数量级，无议。

### 2.2 v0 板级网格落库后（B03 fixture 规模）

`Grid.cells` 是行主序 `(索引|null)[]`，JSON 化每格 ≈3–5 字符：

- 28×28 = 784 格 ≈ 3–4KB/张；56×56 = 3136 格 ≈ 13–16KB/张。
- 20 个带网格的项目 + 步进进度游标（§3，O(1) 大小）≈ **300–500KB**。仍在配额十分之一以内。
- 整包重写代价：几百 KB 的 `JSON.stringify` + 同步 `setItem` 在毫秒级；B04 自动保存按**步**为节拍
  （一步 = 一种颜色 / 一块板 / 一行，间隔以分钟计），不是按豆——可接受，前提是 §3 的进度形状钉住。

**结论：v0 范围内 localStorage 够用，这不是猜测是算术。**

### 2.3 击穿点：B07 的 512×512 导入上限（即「大图」）

map §4 B07 已钉导入格数上限 512×512 = 262,144 格：

- 网格 JSON ≈ **1.1MB 字符 ≈ 2.2MB UTF-16**——**两次导入即耗尽整个 origin 配额**；
- 且监护是整包模型：此后**每一次**任何 save（哪怕只是切换收藏）都要重新序列化并同步写入 >1MB，
  主线程卡顿落在沉浸拼装的核心卖点路径上；
- 逐格完成位若也 JSON 化（布尔数组每格 ≈6 字符），单图再加 ≈1.5MB——同一数量级，同罪；
- 二进制表示（`Int16Array` 网格 512KB、`Uint8Array` 位图 32KB）localStorage 根本存不了
  （只收字符串，base64 再 +33% 且照样 UTF-16 翻倍），IndexedDB 结构化克隆原生收。

**这一层就是任务书括号里的例外，也是 §5 契约的全部理由。IDB 的 origin 配额按磁盘比例计（GB 级），
512KB/张的 pattern 文档存几百张不见顶。**

## 3. 进度格子怎么存（B04 尚未动工，形状必须先钉）

现状：进度只有 Workspace 的 `ProgressBar` 恒 0（注明归 B04），`AssemblePage` 无任何会话持久化。
map §4 B04 已钉「步进粒度自动保存、刷新后恢复当前步」。本轮把形状收敛成两层：

1. **步游标（必做，B04 的验收对象）**：`{ projectId, mode, stepIndex, elapsedMs, updatedAt }`。
   O(1) 大小，v0 期间放 localStorage 整包里完全无压力。
   **禁止**把步序列（`Step[]`）落库——四模式拆步是确定性纯函数（G6 并列裁决钉死的意义就在这），
   `grid + mode → steps` 每次现算，和 BOM 一样归「派生数据禁止持久化」（IA §5 规则）。
2. **逐格完成位（可选层，只随 IDB 出现）**：`doneBits: Uint8Array`（width×height 位，行主序）。
   只有当 B04 允许偏离严格步序（悬浮球的「锁定当前行」暗示部分完成的步）或恢复到步中位置时才需要；
   56×56 = 392 字节，512×512 = 32KB，都只在 IDB 二进制层存。
   **在 localStorage 阶段 B04 不得引入任何逐格进度数组**——刷新后恢复到「当前步开头」是 v0 可接受的
   精度损失，换来的是 §2.2 的算术继续成立。

## 4. 裁决依据小结

| 候选动作 | 判定 | 理由 |
|---|---|---|
| 现在整体迁 IndexedDB | 否（NO_HIGH_VALUE_CHANGE_FOUND） | §2.1/2.2：余量两个数量级；迁移引入打开失败、vitest 无原生 IDB、迁移路径三个新失败面，而 B04/B05 在 localStorage 上就能交付 |
| 大图逐格载荷进 localStorage | 禁止 | §2.3：两次 512×512 导入击穿配额；整包重写把 MB 级同步写砸进拼装主循环 |
| 最小 IDB 契约现在**写死**、实现**延迟到触发点** | 是 | B03/B04/B05 三个实现席都压在存储形状上（map §5 早已点名）；契约不先行，就会各自发明 |
| DATA-1 静默降级修复 | 现在就修 | 与 IDB 无关的存量数据完整性缺陷，见 §8 |

## 5. 最小 IndexedDB 契约（触发点：第一个持久化 `Grid` 的 PR；在此之前零实现）

### 5.1 库与对象仓

库名 `bead-v1`（map §4 B09 已钉），`version = 1`，四个 object store：

| store | key | 值 | 说明 |
|---|---|---|---|
| `state` | 显式 key：`"projects"` / `"favorites"` / `"inventory"` | 与今天 `bead.state` 里同形状的三个整数组 | 保持整数组语义 ⇒ `store.tsx` 与现有六个 Repository 方法**零改动**（D-UI-5 的回报） |
| `patterns` | keyPath `projectId` | `PatternDoc`（见 5.2） | 重文档与热元数据分仓：Workspace 列表永不反序列化网格 |
| `progress` | keyPath `projectId` | `ProgressDoc`（见 5.2） | 步游标 + 可选 `doneBits` |
| `meta` | 显式 key | `{ schemaVersion: 1 }` 等 | 迁移标记；预留，不为 v0 造字段 |

### 5.2 文档形状（进 `stores/types.ts`，TypeScript 即 schema）

```ts
interface PatternDoc {
  projectId: ProjectId;
  paletteId: string;            // v0 恒 "generic-5mm"，导入时校验命名空间（§6）
  width: number;
  height: number;
  cells: Int16Array;            // 行主序；-1 = 空格；结构化克隆原生支持
  provenance?: { kind: ImageKind; ditherApplied: boolean };  // 展示用，只读
}

interface ProgressDoc {
  projectId: ProjectId;
  mode: SplitMode;
  stepIndex: number;
  elapsedMs: number;
  updatedAt: number;
  doneBits?: Uint8Array;        // width×height 位，行主序；仅偏离步序时写
}
```

**不持久化**：BOM、`Step[]`、缺口 / 替代建议、个人条计数——全部派生，selector 现算（IA §5 规则原样延伸）。

### 5.3 Repository 接口增量（现有六方法不动）

```ts
loadPatternDoc(id: ProjectId): Promise<PatternDoc | null>;
savePatternDoc(doc: PatternDoc): Promise<void>;
loadProgress(id: ProjectId): Promise<ProgressDoc | null>;
saveProgress(doc: ProgressDoc): Promise<void>;
deleteProject(id: ProjectId): Promise<void>;   // 单事务级联删 patterns + progress，唯一跨仓不变量
```

### 5.4 迁移（一次性、单向、幂等）

首次打开 `bead-v1` 时：用现存 `parsePersistedState` 读 localStorage `bead.state` → 三个数组 put 进
`state` 仓 → 删除 `bead.state` 键。第二次运行读不到键即天然幂等。`bead.theme` 不迁（主题留 localStorage，
体量 5 字符、且 ThemeProvider 首帧同步读，搬进 async IDB 纯属倒退）。后续 schema 变更走
`onupgradeneeded` 单向升级（map 已钉），v0 不写降级路径。

### 5.5 错误政策（与 DATA-1 同一条原则：禁止静默降级）

IDB 打开或写入失败 ⇒ Repository 进入**显式**降级态：读继续（内存态，应用可用），但必须向 store 暴露
`persistenceFailed` 信号，UI 出一条常显横幅「本地存储不可用，本次更改不会保存」。
**任何 catch-后-假装-成功都是缺陷**——这条同样约束今天的 localStorage 实现（§8 DATA-1）。

### 5.6 测试口径

- 单测用 `fake-indexeddb`（devDependency，不进运行时依赖面，不违 D-UI-7）；
- 必测四条：PatternDoc 含 `Int16Array` 往返不变；迁移（种 `bead.state` → 打开 → 三数组在 `state` 仓、原键已删、
  再开不重复迁）；`deleteProject` 级联；打开失败 ⇒ `persistenceFailed` 信号可观察。
- 「导出全部 = `.beadproj` 数组下载」是备份通道（map §4 B09），round-trip 测试归 B07，此处只要求
  各仓可枚举导出。

### 5.7 明确不做（免后续往返论证）

- 不引 Dexie / idb 等包装库：四仓一迁移，裸 IDB ~150 行以内，包装库是运行时依赖面的净增；
- 不做多标签页一致性（整包 last-writer-wins 照旧；单机档案 v0 假设单标签页，记录为已知边界）；
- 不调 `navigator.storage.persist()`：v0 数据量距逐出阈值极远，且 Firefox 会弹权限框，违背 §7 零弹窗面；
- 不做加密 / 不做账号档案隔离：无账号体系是 BD7 拍板不是缺口（round1-map 登录权限面已判 NO_HIGH_VALUE_CHANGE_FOUND，本轮维持）。

## 6. 导入消毒（B07 的存储边界，先例 = `parsePersistedState`）

威胁面就两个：导入文件解析、出网（后者 §7）。消毒清单如下，B07 实现须逐条对照：

1. **`.beadproj`**：schema 校验（`src/schema`，无 `$id`/`$schema` URL——e0 红线）；上限钉死
   512×512 格、20MB 文件（map §4 B07），超限给明确错误，**禁止静默截断**；
2. **id 一律重铸**：导入的 `proj-*`/`gal-*` 只当外来字符串，进库前用 `mintProjectId` 重新分配——
   信任导入 id 会碰撞覆盖现有项目及其 progress（branded 守卫只查前缀，防不了碰撞）；
3. **字符串字段设长**：`title` 等 ≤256 字符——4MB 的标题就是一次配额 DoS；
4. **色板命名空间校验**：v0 只认 `generic-5mm`；`cells` 每格必须是 `-1` 或 `[0, palette.entries.length)`
   的整数，越界即拒收（报错带格坐标），不做静默夹取；
5. **png/jpg**：解码只走 `createImageBitmap`（平台解析器，无第三方解析依赖，`decode.ts` 已钉
   `colorSpaceConversion: "none"`）——但 **`decodeImage` 目前没有源尺寸上限**：恶意 20000×20000 PNG
   头会触发 1.6GB 的 RGBA 分配。接线前须在 `drawImage` 之前按位图头尺寸设上限（建议 ≤4096×4096，
   RGBA 64MB），超限走类型化错误（DATA-3）；
6. **`.pat` / `.gamedev`**：识别魔数、不认识就 `UNSUPPORTED_FORMAT`（map §4 B07 已钉，不逆向臆造格式）；
7. 全树已无 `eval` / `dangerouslySetInnerHTML`，B07 评审时复查一次即可，不新增门。

## 7. 默认不出网 / 权限面

- **不出网已是可执行断言**：`isolation.test.ts` 的 NE-1 扫描 + 自检本轮逐行复核，e0-audit 覆盖
  `apps/bead` 全树 URL 字面量（round1-map §2.1）。IndexedDB 不引入任何网络路径。维持现状，无新门。
- **浏览器权限面 = 零弹窗**，并应保持为零：文件导入走 `<input type="file">`（手势门控、无权限提示）、
  导出走下载（同）、无 getUserMedia / 剪贴板读 / 通知 / persistent-storage。B05 的「导出采购文本」
  用下载或选中复制实现，不引 Clipboard API 权限。任何为 v0 引入权限提示的改动都应在 review 里被拒。
- **无账号 = 单机档案**：授权层不存在也不该存在（BD7）。「档案」的完整性边界就是 §5.5 的错误政策
  加 §6 的导入消毒。

## 8. 发现清单（DATA-*，供实现席引用）

### 需要动代码的

- **DATA-1（中）· 写失败静默降级 = 静默丢档，现在就修，不等 B09。**
  `repository.ts::write` 在 `setItem` 抛出（配额满 / 私隐模式）时落进 `MemoryBacking`，但 `read`
  仍优先读 localStorage——此后：save 全进易失内存、下次 read-modify-write 又拿旧 localStorage 当底，
  刷新即丢失首个失败点之后的一切，全程无任何信号。v0 体量下今天触发不了，但 B03 一落网格就进入
  可触发区，且它违反本线「禁止静默降级」的总原则。修法（~15 行 + 1 测）：写失败后置 `persistenceFailed`
  标志并让后续 read 改读 fallback（读写视图收敛），Repository 把标志暴露给 store，UI 出常显横幅。
  §5.5 的 IDB 错误政策与此同源，先修此处，B09 直接继承。
- **DATA-2（低）· B04 开工前把进度形状钉进类型。**
  在 `stores/types.ts` 定义 §3 的步游标类型并在 review 钉死「localStorage 阶段禁止逐格进度数组、
  禁止持久化 `Step[]`」。零行为变更，纯契约，防三个实现席各自发明。
- **DATA-3（低）· `decodeImage` 无源尺寸上限。**
  见 §6 第 5 条。归 B03 接线或 B07 导入的同一 PR，不单独开 PR。

### 记录性（无需动代码）

- **DATA-4 · B07 消毒清单**：§6 全文即验收清单，B07 的 review 逐条打钩。
- **DATA-5 · 多标签页 last-writer-wins**：已知边界，v0 接受，写进 §5.7 免得当 bug 报。

## 9. NO_HIGH_VALUE_CHANGE_FOUND 子树

- 整体 IndexedDB 迁移（v0 板级规模）：§2 算术 + §4 表。NO_HIGH_VALUE_CHANGE_FOUND（触发点见 §5）。
- 存储抽象再加层（Dexie/idb、ORM、按记录 CRUD 化 `state` 仓）：四仓裸 IDB 足够。NO_HIGH_VALUE_CHANGE_FOUND。
- `navigator.storage.persist()` / StorageManager 配额探测：数据量距逐出阈值极远，还引入权限弹窗。NO_HIGH_VALUE_CHANGE_FOUND。
- 账号 / 档案多用户 / 加密静态数据：BD7 拍板无账号，单机档案。NO_HIGH_VALUE_CHANGE_FOUND（维持 round1-map 判定）。
- 主题存储改造：5 字符、同步首帧读，现状即最优。NO_HIGH_VALUE_CHANGE_FOUND。
- 出网面新增门禁：NE-1 + e0-audit 双层已可执行，IDB 不改变威胁面。NO_HIGH_VALUE_CHANGE_FOUND。

## 10. 禁区自查

本轮只新增 `docs/bead/reviews/round2-data.md`。未改 `apps/bead` 任何源码（DATA-1…3 留给实现席），
未触碰 `docs/PRODUCT_LOCK.md`、`docs/STATUS.md`、`apps/desktop/**`、`crates/soul-*/**`、根 `Cargo.toml`、
`deny.toml`、`.github/**`。
