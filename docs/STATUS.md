# STATUS

计划线的单一事实来源。每个子代理完工必须更新本文件。写法纪律见 D57。

核于 2026-08-25。

## 当前里程碑

**`PLAN_FROZEN` + `ALGO_FROZEN`。** 这两个冻结管的是**计划与算法选型**，不是实现，也不是合入。

本分支（`cursor/polish-project-plan-5280`，从 `main` @ `7b35bde` 长出）只做一件事：**把计划权威面打磨好、落进 `main`**。本分支不发布应用，不改任何应用代码，也不改冻结算法的规则与常量。

## 各条线各自到哪了

| 线 | 尖端（核于 2026-08-25） | 有什么 | 状态 |
|---|---|---|---|
| `main` | `7b35bde` | 两个算法 crate 与 `docs/algorithms/` | **`main` 上还没有应用代码** |
| 本分支（计划面） | 已合 `main` @ `095c1f8`（PR #8） | 产品锁 / 拍板（D1–D60）/ FORMAL / `PLAN_INDEX.md` / 已类型化的 `tie_strength` / SECURITY / 本文件 | **`PLAN_DOCS_FROZEN_FOR_MAIN`（已合入）** |
| `cursor/soul-goal1-7b1c`（PR #2） | `df5d2dd` | Goal 1 实现主干；WP01–WP11、WP13 与 DPAPI 均已落地（以该分支自己的 STATUS 为准） | **未合入 `main`，也未关闭** |
| `cursor/blockers-analysis-a073`（PR #6） | 远程 | `docs/BLOCKERS.md`（`BLOCKERS_FROZEN`） | 待合进 `main` |
| `cursor/goal1-unblock-a073`（PR #7） | 远程进行中 | Goal 1 吸收 T4D | 本分支不碰那条线的代码 |
| `agent/dev-sota`（PR #4） | 远程 | 与主干分叉后重做的 WP10/WP11 | 按 D49 应停并关闭 |

**不要再把「尚未写应用代码」当成全局事实。** 准确说法是：`main` 上没有应用代码；应用代码在 `cursor/soul-goal1-7b1c` 上，覆盖面已经很宽，但**没有合入，也没有关闭**。

## 进度

| 项 | 状态 |
|---|---|
| R1 双模型扫描 | 完成，曾 `PLAN_BLOCKED` |
| R2 规范修复 | 完成并落盘 |
| R3 复核冻结 | 完成，结论 `PLAN_FROZEN`。仲裁见 `docs/scan-rounds/R3-SYNTHESIS.md` |
| 算法三轮 + Round X | 完成，结论 `ALGO_FROZEN`：人脉 T4D、特质轴 A0。裁决见 `docs/algorithms/DECISION.md` |
| 阻碍项分析 | 完成，`BLOCKERS_FROZEN`（PR #6），尚未在本树 |
| 计划面打磨（本分支） | Round 3 完成并已合 `main`：`PLAN_DOCS_FROZEN_FOR_MAIN`。不是 Goal 1 关闭 |
| Goal 1 实现 | **进行中，在 `cursor/soul-goal1-7b1c`。未合入 `main`，未关闭** |
| Goal 1 关闭门 | 未过。关闭需矩阵与十三片切片同时过（D54） |
| Goal 2 | 未启动，且 Goal 1 关闭前不得启动 |

## 阻塞

**计划面无阻塞**：写码的前置文档已齐。按 D57，这句话只覆盖本树，**不等于可以合进 `main`，也不等于 Goal 1 可以关闭**。

合入与关闭的阻碍项权威是 `docs/BLOCKERS.md`（PR #6 合进 `main` 后与本文件同树）。本文件不复抄全表。

| 项 | 类别 | 核于 2026-08-25 |
|---|---|---|
| M1 两条实现线 | 合入 | 未见 PR #4 关闭 |
| M2 `main` 并进 Goal 1 | 合入 | 主干 `cursor/soul-goal1-7b1c` @ `df5d2dd` 未做：`Cargo.toml` 成员表里没有两个算法 crate。吸收线 `cursor/goal1-unblock-a073` @ `c81c233` 已把两个 crate 加进成员表，待回主干（D50） |
| M3 Windows 夹具 | 合入 | 需在尖端复核；`BLOCKERS.md` 已判定主干 `fc96e46` 的写法不采纳 |
| G1 人脉图仍是遗留判档 | 关闭 | PR #7 进行中；以该线 STATUS 为准，本树不复验源码 |
| schema 倒退 | 计划 | **已关闭**：Goal 1 接线正文 + `tie_strength` 类型化（D58/D59），含 `machine_band` / `user_band` / `locked_by_user` |

## 下一步

1. PR #6（BLOCKERS）合进 `main`（合入时处理 BLOCKERS 文中的 D32 撞号，见 DECISIONS 脚注）。
2. 之后按 `BLOCKERS.md` 第 5 节在 Goal 1 线上走剩余项。Goal 1 merge 本 `main` 后跑 `xtask schema-freeze`（D59）。
3. Goal 2 在 Goal 1 关闭前不要启动。

## 本文件的写法纪律（D57）

1. 只写本文件所在这棵树可验证的事实。跨分支的事实必须带分支名与提交号，并标明「核于」日期。
2. 「阻塞：无」只覆盖本树，不构成合入或关闭的结论。
3. 冻结标记要写清管辖范围：`PLAN_FROZEN` 管计划，`ALGO_FROZEN` 管算法选型，`BLOCKERS_FROZEN` 管阻碍项清单。
4. `cursor/soul-goal1-7b1c` 上另有一份更细的同名文件。PR #2 合入时把两侧合成一份。
