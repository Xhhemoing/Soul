# `.beadproj` v1

BeadFlow 的项目库归档格式，由 WP-B07 的导入导出面使用。**规范的可执行版本是同目录的
`beadproj.ts`**：那里的校验器就是 schema（D-IE-4）。本文写散文，说明形状、边界与拒收政策，
供人阅读与评审对照，不参与运行。

本仓不发行 JSON Schema 文件，也不引入 ajv / zod：手写的「重建式」校验器已经 fail closed，
而 schema 文件正是 `$schema` / `$id` 之类外网 URL 的高发地（BD15 / BD17）。

## 1. 容器

一个 UTF-8 的 JSON 文本文件，扩展名 `.beadproj`，MIME `application/json`。无 ZIP、无压缩、
无加密、无二进制块。文件里**没有时间戳、没有导出者信息、没有任何 URL**：同一份状态导出两次，
字节完全相同（这条性质被测试直接断言）。

顶层：

```jsonc
{
  "format": "beadproj",   // 字面量判别符；不是 URL
  "version": 1,           // 整数；≠1 一律拒绝，不猜测未来版本
  "projects": [ /* Entry，见 §2 */ ],
  "inventory": [ /* 可选，见 §3 */ ]
}
```

单项目导出与「导出全部」共用这一个形状：**单项目导出就是长度 1 的归档**（D-IE-3）。
`projects` 可以是空数组——只备份库存是合法的。`inventory` 缺席表示「这份归档不带库存」，
与空数组同义。

## 2. `projects[]`（Entry）

```jsonc
{
  "project": {
    "title": "元宵提灯",          // string，≤256 字；trim 后为空 → 导入成「未命名导入」
    "sourcePatternId": null,      // null，或 gal- 开头的画廊图纸 id
    "status": "todo",             // todo | active | draft | done
    "createdAt": 1730000000000,   // 有限正数
    "backdrop": "black",          // black | white | custom
    "backdropColor": "#101014"
  },
  "pattern": { /* 见下；与 sourcePatternId 互为双射 */ },
  "progress": { /* 可选，见下 */ }
}
```

**Entry 里没有项目 id。**嵌套即关联：`pattern` 与 `progress` 挂在自己的 entry 里，不需要 id
相认；导入端一律现铸新 id（D-IE-8）。因此重复导入同一份文件得到的是两套彼此独立的项目，
不会互相覆盖，也不存在 id 碰撞这一类缺陷。

**双射（D-IE-9）**：`sourcePatternId === null` ⟺ `pattern` 字段在场。
画廊来源的项目其网格来自构建期图纸库，带 `pattern` 会被永远无视；
无来源又没有 `pattern` 的项目是一条悬空引用。两者都整条拒绝，并在汇总里报出序号与原因。

### `pattern`

```jsonc
{
  "paletteId": "generic-5mm",   // v0 唯一可索引命名空间
  "width": 28,                  // 正整数
  "height": 28,                 // 正整数
  "cells": [-1, 0, 12, ...],    // 行主序，长度 = width × height
  "provenance": { "kind": "PixelArt", "ditherApplied": false }  // 可选
}
```

`cells` 是明文整数数组：`-1` 表示空格，其余取值 ∈ `[0, 48)`，即 `generic-5mm` 封闭 48 码的
下标。不做 base64、不做 RLE——56×56 是 3136 个数、约 12KB，明文可人工查验，与 20MB 的文件门
差三个数量级。落盘时经 `stores/patterns.ts` 既有的 `Grid ↔ Int16Array` 编解码转换，
本格式不带第二套 codec。

全空网格（`cells` 全 `-1`）是合法的：`patterns` 仓的不变量不含「非空」，「空网格禁保存」只是
上传工作台的规则。导入这样的项目会得到一个能进编辑器、在拼装侧显示空网格态的项目。

**尺寸三层门（D-IE-6）**：

| 层 | 条件 | 结果 |
|---|---|---|
| ① 文件门 | `File.size > 20MB` | `FILE_TOO_LARGE`，**读取字节之前**就拒绝 |
| ② 契约效度 | `width` / `height` 非正整数，或 > 512 | `SCHEMA_INVALID` |
| ③ v0 接收门 | 任一轴 > 56（但 ≤512） | `GRID_TOO_LARGE_FOR_V0`，文案点名实际尺寸与 56×56 |

57–512 之间是「格式合法但本版本没有能渲染它的消费端」，所以是独立错误码而不是格式错误：
日后放宽只需要改 ③，已经发出去的文件不用迁移。三层门**都不静默截断、不提议裁剪**。

### `progress`

```jsonc
{ "mode": "color-by-color", "stepIndex": 7, "elapsedMs": 61000, "updatedAt": 1730000000000 }
```

BD19 的五字段游标去掉 `projectId`（由嵌套关系代替）。`mode` 必须是四种分步模式之一，
校验规则直接复用 `toProgressCursor`。`stepIndex` 越界不在导入端夹取——既有语义是消费端夹取。
`doneBits` 不进 v1：v0 没有写它的人，也没有读它的人。

导入落盘走既有的 `upsertProgress` action，而该 action 按 BD19 的规定由自己盖 `updatedAt`
时间戳，因此**导入后的 `updatedAt` 是导入时刻，不是文件里的值**；其余四项原样恢复。

## 3. `inventory[]`（可选，仅归档层）

```jsonc
{ "paletteId": "generic-5mm", "code": "G07", "name": "Silver", "hex": "#b7bfc6", "beads": 500 }
```

每一条**必须自带 `paletteId`**，且必须是本版本认得的命名空间（`gallery` / `generic-5mm`）。
缺失或不认得的一律**丢弃该条并计数，绝不盖章**（D-IE-12 / BD20）：给一条丢了命名空间的
`G07 Silver` 盖上 `gallery` 章，就是亲手把它和画廊的 `G07 苔绿` 撞在一起。
这个格式是本仓自己发行的，没有需要宽容的历史文件。

合并政策是**已存在的 `(paletteId, code)` 跳过**：本地为准，重复导入幂等，不求和、不覆盖。
汇总会报「新增 x / 跳过 y / 丢弃 z」，所以「跳过」不会看起来像「没生效」。

## 4. 未知键

校验器是重建式的：每个字段单独校验后重新组装，文件里多出来的键天然被丢弃，
既不会进入内存，也不会在下一次导出时被写回去。v1 之内宽进字段、严出形状。

## 5. 版本政策

- `format !== "beadproj"` → `SCHEMA_INVALID`
- `version !== 1` → `UNSUPPORTED_VERSION`（fail closed，不尝试解析未来版本）
- 六角板 / 圆板 / 多板拼接不进 v1：schema 里没有板型字段，方板由 `width` / `height` 隐含。
  几何扩展是 version 升级的事，不是加一个可选键。

## 6. 错误码（封闭集）

`FILE_TOO_LARGE` · `SCHEMA_INVALID` · `UNSUPPORTED_VERSION` · `GRID_TOO_LARGE_FOR_V0` ·
`UNSUPPORTED_FORMAT`

前四个来自 `.beadproj` 解析；`UNSUPPORTED_FORMAT` 来自「导入已有豆图」面的魔数嗅探
（`.pat` / `.gamedev` 等 v0 无解析路径的格式）。每一个都有面板内联呈现，
没有「catch 之后假装成功」的路径。

## 7. 导出侧的两个约定

- **文档枚举由 `projects` 数组驱动**：逐个无来源项目读它自己的 `PatternDoc`。
  仓里若有不属于任何项目的孤儿文档，故意不出现在归档里。
- 无来源项目若读不到文档（存储真的丢了它），该项目**不写进归档**并在导出提示里计数——
  写一条自己的校验器都会拒收的 entry，比少一条更糟。
