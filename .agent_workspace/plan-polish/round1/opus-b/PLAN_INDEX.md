# 计划索引（30 秒定位权威文件）

> 目标落位：`docs/PLAN_INDEX.md`。本页所有链接按该落位写成相对路径。

新来的人只需要这一页。**同一个事实只有一个权威文件**；下表左列以外的任何地方写到同一件事，都是复述，冲突时以左列为准。

## 一、先读四份（按顺序，约 15 分钟）

| # | 文件 | 它是什么的唯一权威 | 冻结标记 |
|---|---|---|---|
| 1 | [`PRODUCT_LOCK.md`](PRODUCT_LOCK.md) | 产品是什么、不是什么；v0.1 的 13 条垂直切片；砍/留；出网分级 E0/E1/L；不可协商约束 | `PLAN_FROZEN` |
| 2 | [`DECISIONS.md`](DECISIONS.md) | 已拍板的选择（D1–D31）。想重开某个方向战，先在这里找它有没有被拍过 | `PLAN_FROZEN` |
| 3 | [`algorithms/DECISION.md`](algorithms/DECISION.md) | 灵魂层算法：人脉 = T4D，特质 = A0，摘要 = A2 渲染器，A1 默认规则；常量表；as_of 纪律；已知代价；回退链 | `ALGO_FROZEN` |
| 4 | [`FORMAL_WORK_PROMPT.md`](FORMAL_WORK_PROMPT.md) | Goal 1 怎么开工：工作包清单、实现者红线、**验收矩阵（唯一门禁）** | 随 `PLAN_FROZEN` |

## 二、按问题查

| 你想知道 | 去哪 |
|---|---|
| v0.1 到底做不做某个功能 | `PRODUCT_LOCK.md`「v0.1 最小垂直切片」+「砍 / 留」 |
| 某个决定为什么是这样 | `DECISIONS.md`（产品/工程）、`algorithms/DECISION.md` 第 4–5 节（算法代价与回退） |
| 一件事算不算做完 | `FORMAL_WORK_PROMPT.md` 验收矩阵。**散文不作门禁**（D29） |
| 现在到哪一步了、谁没合入 | `STATUS.md` |
| 数据长什么样 | `schemas/`（九份 + `_defs.schema.json`）。Goal 1 分支另有 `schemas.lock.json` 钉 sha256 |
| 加密、密钥、遗忘、审计的规范 | `SECURITY.md` |
| 关系强度为什么是这个档 | `algorithms/DECISION.md` 第 3 节；中文话术在 `algorithms/COPY_ZH.md`；被否决的候选在 `algorithms/REJECTED.md` |
| Goal 1 的 DAG 与实现细节 | `origin/cursor/soul-goal1-7b1c:docs/GOAL1_PLAN.md`（只读对照，尚未合入 `main`） |
| 已知阻塞 | `origin/cursor/blockers-analysis-a073:docs/BLOCKERS.md`（`BLOCKERS_FROZEN`，尚未合入 `main`） |
| Goal 2 | `GOAL2_POLISH_PROMPT.md`。Goal 1 关闭前不要打开 |

## 三、仓库拓扑（读代码前先看这三行）

- `main`：计划权威面 `docs/` + 冻结算法 crate `crates/soul-algo-tie`（T4D）、`crates/soul-algo-trait`（A0/A1/A2/A3）。**没有可安装的应用。**
- `cursor/soul-goal1-7b1c`：Goal 1 实现主干（桌面壳、加密库、导入、图谱、记忆、审计、安装 smoke）。唯一实现线，尚未合回 `main`。
- 计划冻结 ≠ Goal 1 关闭 ≠ `main` 已有应用。三件事在 `STATUS.md` 里分开写。

## 四、不是权威（别拿它当依据）

- `.agent_workspace/**`：过程材料（扫描轮次、探针、子代理草稿）。**任何权威结论必须落到 `docs/`**；`.agent_workspace/context/plan/` 已不再是权威面。
- `scan-rounds/**`：历史仲裁记录。解释「为什么」，不定义「是什么」。
- 第二份 `PRODUCT.md`：**禁止存在**（D27）。看到就删。

## 五、改这些文件的规矩

| 想改 | 前置 |
|---|---|
| 产品方向 | 先改 `PRODUCT_LOCK.md`，并在 `DECISIONS.md` 追一条 |
| 算法判档 | 只走 `algorithms/DECISION.md` 第 5 节回退链，`DECISIONS.md` 留痕。禁止为 F04c 加第三道门 |
| 验收矩阵 | 只增不删已通过的门禁；新增行写清 Given/When/Then 与「谁跑」 |
| 工作包 | 只减不增（D30）。WP12 保持删除 |
| `schemas/**` | 同一 PR 里重算 Goal 1 的 `schemas.lock.json`，并说明对已落库数据的影响 |
