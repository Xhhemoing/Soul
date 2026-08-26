# STATUS

计划线的单一事实来源。每个子代理完工必须更新本文件。写法纪律见本文件末节（D48）。

核于 2026-08-25。

## 当前里程碑

**`PLAN_FROZEN` + `ALGO_FROZEN`。** 这两个冻结管的是**计划与算法选型**，不是实现，也不是合入。

本分支（`cursor/polish-project-plan-5280`，从 `main` @ `7b35bde` 长出）只做一件事：**把计划权威面打磨好、落进 `main`**。本分支不发布应用，不改任何应用代码，也不改冻结算法的规则与常量。

## 各条线各自到哪了

引用他线事实一律带提交号；下表核于 2026-08-25，各分支仍在推进。

| 线 | 尖端 | 有什么 | 状态 |
|---|---|---|---|
| `main` | `7b35bde` | 两个算法 crate（`soul-algo-tie` / `soul-algo-trait`）与 `docs/algorithms/`（`ALGO_FROZEN`） | **`main` 上还没有应用代码** |
| 本分支（计划面） | 本 PR | `PRODUCT_LOCK` / `DECISIONS` / `FORMAL_WORK_PROMPT` / 九份 schema / `SECURITY` / 本文件 | 进行中，未合入 `main` |
| `cursor/soul-goal1-7b1c`（PR #2） | `3161e02` | **Goal 1 实现主干**：按该分支自己的 `docs/STATUS.md`，WP01–WP11、WP13 与 DPAPI 均已落地 | **进行中：未合入 `main`，也未关闭** |
| `cursor/blockers-analysis-a073`（PR #6） | `e669b78` | `docs/BLOCKERS.md`（`BLOCKERS_FROZEN`） | 待合进 `main`，尚未在本树 |
| `cursor/goal1-unblock-a073`（PR #7） | 远程进行中 | Goal 1 吸收 T4D 的那一批改动 | 进行中；本分支不碰那条线的代码 |
| `agent/dev-sota`（PR #4） | 远程 | 与主干分叉后重做的 WP10/WP11 | 按 D40 应停并关闭 |

**不要再把「尚未写应用代码」当成全局事实。** 准确说法是：`main` 上没有应用代码；应用代码在 `cursor/soul-goal1-7b1c` 上，覆盖面已经很宽，但**没有合入，也没有关闭**。实现面的权威是那条分支上的同名文件，比本文件细得多。

## 进度

| 项 | 状态 |
|---|---|
| R1 双模型扫描 | 完成，曾 `PLAN_BLOCKED` |
| R2 规范修复 | 完成并落盘 |
| R3 复核冻结 | 完成，结论 `PLAN_FROZEN`。仲裁见 `docs/scan-rounds/R3-SYNTHESIS.md` |
| 算法三轮 + Round X | 完成，结论 `ALGO_FROZEN`：人脉 T4D、特质轴 A0。裁决见 `docs/algorithms/DECISION.md` |
| 阻碍项分析 | 完成，`BLOCKERS_FROZEN`（PR #6，核于 Goal 1 `2e72ddf`），尚未在本树 |
| 计划面打磨（本分支） | 进行中：算法口径进锁与拍板、STATUS 诚实化、单源收敛 |
| Goal 1 实现 | **进行中，在 `cursor/soul-goal1-7b1c` 上。未合入 `main`，未关闭** |
| Goal 1 关闭门 | 未过。关闭需矩阵与十三片切片同时过（D45） |
| Goal 2 | 未启动，且 Goal 1 关闭前不得启动 |

## 阻塞

**计划面无阻塞**：写码的前置文档已齐。按 D48，这句话只覆盖本树，**不等于可以合进 `main`，也不等于 Goal 1 可以关闭**。

合入与关闭的阻碍项权威是 `docs/BLOCKERS.md`（`BLOCKERS_FROZEN`；PR #6 合进 `main` 后与本文件同树）。本文件不复抄它的全表，只记本分支能核到的状态：

| 项 | 类别 | 核于 2026-08-25 的状态 |
|---|---|---|
| M1 两条实现线 | 合入 | 未见 PR #4 关闭 |
| M2 `main` 并进 Goal 1 | 合入 | 未做：`3161e02` 的 `Cargo.toml` 成员表里没有两个算法 crate |
| M3 Windows 夹具 | 合入 | 需在尖端复核；`BLOCKERS.md` 已判定主干 `fc96e46` 的写法不采纳 |
| G1 人脉图仍是遗留判档 | 关闭 | 未接：`3161e02` 的 `crates/soul-graph` 不依赖 `soul-algo-tie`，`build.rs` 仍自带 `band()`。PR #7 正在做 |
| G1+ owner 群消息扇出 | 关闭 | 见 D38，随 G1 同批 |
| G2 intake 绕轴锁 | 关闭 | 见 D37 |
| G3 人脉图不可纠正 | 关闭 | 见 D39；GC-9 话术要先加性增补 `COPY_ZH.md` |
| G5 壳的残余入口 | 关闭 | `BLOCKERS.md` 核于 `2e72ddf` 时缺导入入口与采集开关；该分支 STATUS 称此后已接上，需按尖端复核 |
| S1 DPAPI 的 Windows 证据 | 发货 | 取决于 M3 |

## 下一步

工序权威是 `docs/BLOCKERS.md` 第 5 节，本文件不复抄整表。本分支只负责其中计划的那一半：

1. 本分支的计划面合进 `main`：`PRODUCT_LOCK` 的「灵魂层算法（v0.1）」、`DECISIONS` D32–D48、验收矩阵的算法相关行、本文件。
2. PR #6（`BLOCKERS_FROZEN`）合进 `main`。
3. 之后按 `BLOCKERS.md` 第 5 节走 M3 → M2 → G1/G1+ → G2 → G3 → G5 → S1 → PR #2 合入。**那些都在 Goal 1 线上做，不在本分支做。**
4. Goal 2 在 Goal 1 关闭前不要启动。

## 本文件的写法纪律（D48）

1. 只写本文件所在这棵树可验证的事实。跨分支的事实必须带分支名与提交号，并标明「核于」日期。
2. 「阻塞：无」只覆盖本树，不构成合入或关闭的结论。
3. 冻结标记要写清管辖范围：`PLAN_FROZEN` 管计划，`ALGO_FROZEN` 管算法选型，`BLOCKERS_FROZEN` 管阻碍项清单；三者都不管实现是否完成。
4. `cursor/soul-goal1-7b1c` 上另有一份更细的同名文件。PR #2 合入时把两侧合成一份，`main` 上只保留一份 `docs/STATUS.md`。
