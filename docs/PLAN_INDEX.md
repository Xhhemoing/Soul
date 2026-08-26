# 计划索引（30 秒定位权威文件）

> 目标落位：`docs/PLAN_INDEX.md`。本页所有链接按该落位写成相对路径。

新来的人只需要这一页。**同一个事实只有一个权威文件**；下表左列以外的任何地方写到同一件事，都是复述，冲突时以左列为准。

## 一、先读四份（按顺序，约 15 分钟）

| # | 文件 | 它是什么的唯一权威 | 冻结标记 |
|---|---|---|---|
| 1 | [`PRODUCT_LOCK.md`](PRODUCT_LOCK.md) | 产品是什么、不是什么；v0.1 的 13 条垂直切片；砍/留；出网分级 E0/E1/L；不可协商约束 | `PLAN_FROZEN` |
| 2 | [`DECISIONS.md`](DECISIONS.md) | 已拍板的选择（D1–D60）。想重开某个方向战，先在这里找它有没有被拍过 | `PLAN_FROZEN` |
| 3 | [`algorithms/DECISION.md`](algorithms/DECISION.md) | 灵魂层算法：人脉 = T4D，特质 = A0，摘要 = A2 渲染器，A1 默认规则；常量表；as_of 纪律；已知代价；回退链 | `ALGO_FROZEN` |
| 4 | [`FORMAL_WORK_PROMPT.md`](FORMAL_WORK_PROMPT.md) | Goal 1 怎么开工：工作包清单、实现者红线、**验收矩阵（唯一门禁）**。开工路径只认末尾的「开工第一动作」；中段「历史段」是作者开工原话的存档，**不是工作指令** | 随 `PLAN_FROZEN` |

## 二、按问题查

| 你想知道 | 去哪 |
|---|---|
| v0.1 到底做不做某个功能 | `PRODUCT_LOCK.md`「v0.1 最小垂直切片」+「砍 / 留」 |
| 某个决定为什么是这样 | `DECISIONS.md`（产品/工程）、`algorithms/DECISION.md` 第 4–5 节（算法代价与回退） |
| 一件事算不算做完 | `FORMAL_WORK_PROMPT.md` 验收矩阵。**散文不作门禁**（D29）。问「Goal 1 整体算不算关闭」是另一回事：矩阵与 `PRODUCT_LOCK.md` 十三片切片必须同时过（D54） |
| 现在到哪一步了、谁没合入 | `STATUS.md` |
| 数据长什么样 | `schemas/`（十份正文 = 九份存储契约 + 一份导入契约，另有 `_defs` 与 `schemas.lock.json`）。`tie_strength` 收紧见 D58/D59 |
| 接手时该不该重开 Goal 1 / 重派 planner | 不该。`FORMAL_WORK_PROMPT.md`「开工第一动作」；理由见 D49（唯一实现主干）与 `STATUS.md`。同文件「历史段」写于 Goal 1 开工之前，照做即重开第二条线 |
| 判档阈值到底是多少 | 只有两处：`algorithms/DECISION.md` 第 3 节常量表（语义）与 `crates/soul-algo-tie` 常量模块（取值）。产品锁 / 拍板 / FORMAL / STATUS / README / PLAN_INDEX / SECURITY 只写常量名。`algorithms/COPY_ZH.md` 与 `REJECTED.md` 允许引用常量表已钉的数字（D60） |
| 加密、密钥、遗忘、审计的规范 | `SECURITY.md` |
| 关系强度为什么是这个档 | `algorithms/DECISION.md` 第 3 节；中文话术在 `algorithms/COPY_ZH.md`；被否决的候选在 `algorithms/REJECTED.md` |
| Goal 1 的 DAG 与实现细节 | `origin/cursor/soul-goal1-7b1c:docs/GOAL1_PLAN.md`（只读对照，尚未合入 `main`） |
| 已知阻塞 | `origin/cursor/blockers-analysis-a073:docs/BLOCKERS.md`（`BLOCKERS_FROZEN`）。**仍在 PR #6，尚未合入 `main`，本树也没有这份文件**；本计划 PR 不整份拷贝它。按 `STATUS.md` 的合入顺序，PR #6 在本 PR 之后合 |
| Goal 2 | `GOAL2_POLISH_PROMPT.md`。Goal 1 关闭前不要打开 |

## 三、仓库拓扑（读代码前先看这三行）

- `main`：计划权威面 `docs/` + 冻结算法 crate `crates/soul-algo-tie`（T4D）、`crates/soul-algo-trait`（A0/A1/A2/A3）。**没有可安装的应用。**
- `cursor/soul-goal1-7b1c`：Goal 1 实现主干（桌面壳、加密库、导入、图谱、记忆、审计、安装 smoke）。唯一实现线，尚未合回 `main`。
- 计划冻结 ≠ Goal 1 关闭 ≠ `main` 已有应用。三件事在 `STATUS.md` 里分开写。

## 四、不是权威（别拿它当依据）

- `.agent_workspace/**`：过程材料（扫描轮次、探针、子代理草稿）。**任何权威结论必须落到 `docs/`**；`.agent_workspace/context/plan/` 已不再是权威面。
- `scan-rounds/**`：历史仲裁记录。解释「为什么」，不定义「是什么」。
- [`PLAN_VERIFY_PROMPT.md`](PLAN_VERIFY_PROMPT.md)：**一次性提示词，已执行完毕**（三轮双模型扫描，结论 `PLAN_FROZEN`，落盘在 `scan-rounds/`）。存档以便追溯当时的风险清单，**不要当新工单重跑**。
- `FORMAL_WORK_PROMPT.md` 的「历史段」：作者开工原话，`CreateGoal：Goal 1` 与派 planner 都已执行过。可执行路径只有同文件的「开工第一动作」。
- 第二份 `PRODUCT.md`：**禁止存在**（D27）。看到就删。

## 五、改这些文件的规矩

| 想改 | 前置 |
|---|---|
| 产品方向 | 先改 `PRODUCT_LOCK.md`，并在 `DECISIONS.md` 追一条 |
| 算法判档 | 只走 `algorithms/DECISION.md` 第 5 节回退链，`DECISIONS.md` 留痕。禁止为 F04c 加第三道门 |
| 验收矩阵 | 只增不删已通过的门禁；新增行写清 Given/When/Then 与「谁跑」。行文里只准出现夹具身份的数字，不准复述判档阈值（红线 11） |
| 工作包 | 只减不增（D30）。WP12 保持删除 |
| `schemas/**` | 同一 PR 里重算 Goal 1 的 `schemas.lock.json`，并说明对已落库数据的影响 |
