MODEL_SLUG: claude-fable-5-thinking-xhigh

# Soul 仓库云端独立审计报告（cloud-fable，2026-08-26）

只审计，不改产品方向，不写应用。本文件是过程材料（`.agent_workspace/**` 非权威，D41），任何要成为权威的结论须由父代理落到 `docs/` 并留痕。

审计基线：`main` @ `a0ec14b`（工作树即此）。所有跨分支事实带分支名 + 提交号，核于 2026-08-26（D57 纪律）。

---

## 0. 已知事实核验结果

| 待核事实 | 结论 |
|---|---|
| main 全历史 ea6f62f → a0ec14b，共 8 提交 | **属实**。`git log --oneline main` 恰为 8 条 |
| main = 计划冻结 + 算法 crate 冻结，无桌面应用 | **属实**。树上只有 `docs/`、`crates/soul-algo-tie`、`crates/soul-algo-trait`、README、workspace 配置；无 apps、无 .github |
| Goal 1 实现在 `cursor/soul-goal1-7b1c`，未合入 main | **属实**。PR [#2](https://github.com/Xhhemoing/Soul/pull/2) OPEN，尖端 `6d1058b`，且该分支 STATUS 自认对 `origin/main`（`a0ec14b`）CONFLICTING |
| PLAN_FROZEN / ALGO_FROZEN 不管实现合入 | **属实**。STATUS / DECISION.md / FORMAL 三处口径一致 |
| `.agent_workspace/**` 非权威 | **属实**（D41、PLAN_INDEX 第四节） |

但「已知事实」描绘的世界**已经过时**：PR #8 之后仓库又长出了 **48 个 PR（#9–#56）和三条 main 权威文档完全不知道的工作线**，见第 2 节。

---

## 1. 已变更摘要（main：ea6f62f → a0ec14b）

1. **`ea6f62f`**（2026-08-24）：Initial commit，仅 1 行 README。
2. **`7b35bde`**（PR [#5](https://github.com/Xhhemoing/Soul/pull/5)，merged 2026-08-24T15:48Z）：算法冻结落 main。带来 `crates/soul-algo-tie`（T4/T4D 判档、常量模块、决胜夹具、6 个测试文件、matrix example）、`crates/soul-algo-trait`（A0/A1/A2/A3、诊断词 denylist）、`docs/algorithms/`（DECISION.md=`ALGO_FROZEN`、COPY_ZH、REJECTED、四轮 synthesis）、rust-toolchain 钉 1.83。479 文件 +49,533 行——大头是 `.agent_workspace` 过程材料。
3. **`c65a0b0` … `095c1f8`**（PR [#8](https://github.com/Xhhemoing/Soul/pull/8)，分支 `cursor/polish-project-plan-5280`，merged 2026-08-25T03:29Z）：计划权威面三轮打磨落 main——
   - `PRODUCT_LOCK.md` 补「灵魂层算法（v0.1）」节，把切片 3/6 的算法口径钉到 `ALGO_FROZEN`，不复抄常量；
   - `DECISIONS.md` 扩到 D1–D60（D41–D60 为本轮收编与新板）；
   - `FORMAL_WORK_PROMPT.md` 验收矩阵扩 AC-28…AC-34（算法吸收行 + 导入归因行），历史段降格为存档；
   - `PLAN_INDEX.md` 新建（30 秒定位权威）；`SECURITY.md` 增补；
   - **`relationship.schema.json` 的 `tie_strength` 从裸 object 类型化为 T4D 可复核面**（D58/D59：band、分列计数、silent_days、as_of、machine_band/user_band/locked_by_user、`if(algorithm_id)` 整包必填），`schemas.lock.json` 重钉该文件哈希；
   - README 重写、STATUS 更新、`.agent_workspace/plan-polish/` 三轮过程稿。
4. **`a0ec14b`**（2026-08-25）：仅改 STATUS——把「本 PR 待合」改为「已合 @095c1f8」，下一步从「合本 PR」推进到「合 PR #6」。

**main 现状验证**：`cargo test --workspace` 在本审计环境（Rust 1.83.0）全绿，19 个 test target 全部 ok（含 ablation、as_of 纪律、explain_zh、goal1_fidelity、a0_lock、a1 空转钉死、a2 无阈值、denylist、doc-test）。夹具 `lilei_12` / `dormant_2019` / `group_heavy_plus_one_direct_each_way` / `group_heavy_plus_three_directs` 确在 `crates/soul-algo-tie/src/testing/mod.rs`。docs 面常量双源抽查干净（STATUS / PLAN_INDEX / README / PRODUCT_LOCK 无 180/360 等阈值字面量）。

### PR #2/#4/#5/#6/#7/#8 与远程尖端核验

| PR | 状态 | 分支 @ 尖端（核于 2026-08-26） | 与 main 权威记载的差异 |
|---|---|---|---|
| [#2](https://github.com/Xhhemoing/Soul/pull/2) Goal 1 | OPEN，CONFLICTING | `cursor/soul-goal1-7b1c` @ `6d1058b` | STATUS 记 `df5d2dd`（已前移；`df5d2dd` 确在其历史中） |
| [#4](https://github.com/Xhhemoing/Soul/pull/4) dev-sota | **仍 OPEN** | `agent/dev-sota` @ `24c539f` | D49 判「停并关闭」，至今未关 |
| [#5](https://github.com/Xhhemoing/Soul/pull/5) 算法冻结 | MERGED @ `7b35bde` | — | 一致 |
| [#6](https://github.com/Xhhemoing/Soul/pull/6) BLOCKERS | **仍 OPEN** | `cursor/blockers-analysis-a073` @ `e669b78` | STATUS「下一步」第 1 条（合入 main）未执行；main 树无 BLOCKERS.md |
| [#7](https://github.com/Xhhemoing/Soul/pull/7) 吸收线 | OPEN | `cursor/goal1-unblock-a073` @ `6133307` | STATUS 记 `c81c233`（已前移）；该线已 merge main 计划冻结（`a30bd6f`）；而主干 STATUS 明写「不要合 PR #7」 |
| [#8](https://github.com/Xhhemoing/Soul/pull/8) 计划面 | MERGED @ `095c1f8` | — | 一致 |

另：PR [#1](https://github.com/Xhhemoing/Soul/pull/1)、[#3](https://github.com/Xhhemoing/Soul/pull/3) 已被 #8 实质取代但仍 OPEN；PR [#11](https://github.com/Xhhemoing/Soul/pull/11)（父编排提示词模板）OPEN 对 main。

---

## 2. main 权威文档的盲区：#8 之后仓库实际长出了什么

远程现有约 100 条分支、PR 到 #56。main 的 STATUS / PLAN_INDEX / README 对以下三条线**零记载**：

1. **`cursor/first-test-candidate-c441`**（PR [#14](https://github.com/Xhhemoing/Soul/pull/14)，base=主干）：主干 @ `5309656` 的 fast-forward + BUILD 审计，自称「第一个正式测试候选」。主干其后又前移（`6d1058b`，文档提交），两者已分叉。
2. **`cursor/soul-integration-4a8e`**（PR [#15](https://github.com/Xhhemoing/Soul/pull/15)，base=first-test-candidate，尖端 `a27cfd1`）：自称「父代理专属整合线」，已跑 17 轮扫描修复。**关键事实：这条线已经把 T4D 移植进产品**（经 `cursor/port-t4d-4a8e`，非合 PR #7）——workspace 成员表含两个算法 crate，`soul-graph/src/build.rs` 判档改走 `soul_algo_tie`，band 话术来自 COPY_ZH，并在产品边界补了 T4D 验收测试（`aac2b39`）。即 **M2 阻碍在这条线上实质完成**。另有：档位纠正产品路径（`correct_tie`/`release_tie`，COMMANDS 36→38，对应 D48/AC-32）、遗忘页级销毁、intake 整批事务、电话号码占位系列、E1 观测仪器。
3. **`cursor/beadflow-integration-c441`**（PR [#16](https://github.com/Xhhemoing/Soul/pull/16)，及 #17–#56 共 40 个 PR）：**姊妹产品 BeadFlow**（拼豆像素工具），195 文件 +35k 行。核验：相对其 base 全部为新增（`docs/bead/`、`apps/bead`、`crates/bead-core`、`.github/workflows/bead.yml`、共享 `pnpm-lock.yaml` 追加），**不改写 Soul 既有文件**；R1–R3 已走完 B01–B08。

对照冻结主干拓扑：D49 说唯一实现主干是 `cursor/soul-goal1-7b1c`；FORMAL「开工第一动作」第 4 条允许「经宣布的后继主干」——但**从未有人在 main 上宣布过后继**。现在实现侧事实上有三个尖端（主干 `6d1058b` / 测试候选 / 整合线 `a27cfd1`），其中最完整的是整合线，而按 main 文档接手的人只会找到主干。

---

## 3. PRODUCT_LOCK 十三片切片对照

图例：✅ 已交付（代码 + 测试）；🟡 部分；❌ 缺。「真机」指作者 Win11 手动清单（76 项，全部未勾）。

| # | 切片 | main | Goal1 主干 @`6d1058b` | 整合线 @`a27cfd1` | 剩余缺口 |
|---|---|---|---|---|---|
| 1 | 安装、托盘、向导（默认全关） | ❌ | ✅ 代码 + 安装 smoke | ✅ | 真机 AC-01 未验 |
| 2 | 问卷 + soul-import-v1 / Telegram 导入 | ❌ | ✅（含 `/import` UI） | ✅ | 真机 UI 导入未走（可选项） |
| 3 | 可编辑档案 + 人脉图 v0 + 推断带证据档 | 🟡 算法权威在 crate | 🟡 图/档案在，但**判档仍是本地 `band()`（T0）** | ✅ T4D 已接线 | 主干未吸收 T4D（M2）——只在整合线闭合 |
| 4 | 用户纠正锁定、推断不覆盖 | 🟡 A0 锁在 crate（a0_lock.rs） | 🟡 特质轴锁 ✅；**边档纠正（D48）无产品路径** | ✅ correct_tie/release_tie | 主干缺 AC-32 产品面 |
| 5 | 记忆 CRUD + 遗忘 + 影响面预览 | ❌ | ✅ | ✅（页级销毁强化） | — |
| 6 | 单人人事摘要（无 key 统计降级） | 🟡 A2 渲染器在 crate | 🟡 摘要在，但话术**未走 `a2_render`**（无 crate 依赖，D40 未落地） | ✅ band 话术来自 COPY_ZH | 主干话术双源风险 |
| 7 | 前台采集，关闭 1s 无事件 | ❌ | ✅（`/collect` UI） | ✅ | 真机采集未验 |
| 8 | 起草不发送 + E1 第三人占位 | ❌ | ✅（AC-12/13 产品路径） | ✅ | — |
| 9 | 只读扫描 + 计划预览 + 未授权 100% 拒绝 | ❌ | ✅ | ✅ | — |
| 10 | 研究预览、第三人行数=0、不写文件 | ❌ | ✅ | ✅ | — |
| 11 | 云端开关「尚未启用」、零 E0 | ❌ | ✅（WebView2 后台联网也已关） | ✅ | 真机抓包未验 |
| 12 | 审计覆盖十类动作 | ❌ | ✅（起草/E1/文件计划/拒绝落链已补） | ✅ | — |
| 13 | CI + 安装 smoke | ❌ **main 无任何 CI** | 🟡 工作流在、`2e72ddf` 曾五门全绿；**HEAD hosted 全部空 runner** | 同左 | GitHub Billing 挡死 AC-26 |

**结论**：main 只交付了切片 3/6 的算法权威半边；主干交付了 13 片的代码大盘但缺三样——T4D 吸收（切片 3/4/6 的算法半边）、hosted CI 绿（切片 13）、真机验收（切片 1 等）；整合线把前一样补齐了，但它不在任何权威记载中。**「两边都缺」的只有：hosted CI 绿、作者真机清单、以及 main 侧的 BLOCKERS.md（仍悬在 PR #6）。**

---

## 4. 不完善清单

### P0（挡住关闭门或正在放大代价）

1. **实现主干歧义**：「唯一实现主干」（D49）事实上已裂成三个尖端；完成度最高的 `cursor/soul-integration-4a8e`（M2 已闭合、D48 产品路径已接）没有任何 main 权威文档承认其地位，「经宣布的后继主干」从未宣布。按 main 文档接手的代理会找错树，或在主干上重做一遍 T4D 吸收。
2. **hosted CI 被 GitHub Billing 挡死**：私有仓库所有产品 run 空 runner（「Billing & plans / spending limit」），AC-26 与 Goal 1 关闭门被**非代码因素**卡住。这只有作者能解，任何代理动作（empty-commit、retrigger）都无效且已被 STATUS 明令禁止。
3. **schema 双源仍在发散**：main 的 `relationship.schema.json` 已类型化（D59），Goal1 系全部分支仍是裸 `{"type":"object"}` + 旧 lock；PR #2 CONFLICTING。整合线每轮还在加产品代码，拖得越久，最终 `xtask schema-freeze` 重算与 D59 收敛的冲突面越大。

### P1（文档漂移 / 双源 / 治理缺口）

4. **STATUS 过期且自指错误**：main `docs/STATUS.md` 第 11 行仍自称「本分支（`cursor/polish-project-plan-5280`……）」——该文件已在 main 上；引用尖端 `df5d2dd` / `c81c233` 均已前移（现 `6d1058b` / `6133307`）；对 #9–#56、三条新线、BeadFlow 零记载。核于 2026-08-25，一天后即失真——D57 纪律执行了「带提交号」，但没有任何机制促使它重核。
5. **过期 PR 未清**：PR #4 按 D49 应关未关；PR #1、#3 已被 #8 取代仍 OPEN；PR #6 是 STATUS「下一步」第 1 条，至今未合（D32 撞号脚注的处理义务也随之悬置）。
6. **main 零机器门禁**：main 无 `.github/workflows`、无 xtask。`schemas.lock.json` 的 11 个 sha256、诊断词 denylist、红线 11（常量双源）在 main 上没有任何自动校验——校验工具全在 Goal1 系分支。今天在 main 上改坏 schema 不会被任何东西拦住。
7. **BeadFlow 未入 repo 级记载**：仓库近半 PR（#16–#56）属于另一个产品，而 README「这个仓库现在有什么」与 PLAN_INDEX「仓库拓扑」仍是两行世界（main / Goal1）。BeadFlow 本身自律（不碰 Soul 文件、自带 `docs/bead/`），但新读者从 main 完全无法得知它存在，也无从判断 `PRODUCT_LOCK.md`「唯一产品权威」的辖域边界（它只锁 Soul，不锁仓库）。
8. **D50 与实际路径背离未留痕**：D50 说算法 crate「在 Goal 1 线上一次 merge 时加进成员表」，实际发生的是整合线 port（且主干 STATUS 明写「不要合 PR #7」）。改道本身合理，但 DECISIONS 没有对应新板（D61+）记录它，违反本仓库自己的留痕纪律（回退链/改道全走 DECISIONS）。

### P2（卫生 / 低风险）

9. `docs/algorithms/DECISION.md` 首行残留 `MODEL_SLUG: claude-fable-5-thinking-xhigh`——子代理输出残渣进了冻结权威文件正文。
10. DECISION.md §1 叙述夹具规模（群聊 100/50）与代码夹具（30/10）不一致——FORMAL 已声明以代码为准，属已知已管理，但冻结文本自身仍带旧数字。
11. 每个算法 crate 各自带 `Cargo.lock`（workspace 成员的 lock 无效）+ 根 lock，双份无效文件。
12. main 树 585 个受控文件中 508 个（87%）是 `.agent_workspace` 过程材料；已声明非权威，但体量随每轮增长，检索/克隆成本在涨。
13. STATUS M3（Windows 夹具「需在尖端复核」）无人认领。
14. 远程约 100 条分支中大量已审完/已合的短命分支未清理。

### 测试缺口

- **main**：算法 crate 测试齐全且本地全绿，但无 CI 承载（见 P1-6）——「冻结」只有社会约束，没有机器约束。
- **主干**：AC-28…AC-34（产品边界的算法验收行）**结构性不可测**——主干没有 `soul-algo-tie` 依赖，这些行只能在整合线（`aac2b39` 已补）上跑。
- **全线**：AC-01（真机安装）、AC-26（hosted 五门）、作者手动清单 76 项——未过，被 P0-2 与真机可用性挡住。
- A1 空转有专门测试钉住（`a1_independence.rs` 等），是预期行为，**不是**缺口。

---

## 5. 建议下一步（不启动 Goal 2）

1. **【作者，唯一非代理可解】** 处理 GitHub Billing & plans，然后 `workflow_dispatch` 在选定实现尖端跑 hosted CI。P0-2 解开之前，AC-26 与关闭门的一切讨论都是空转。
2. **【文档动作，main】** 重核 STATUS（新「核于」日期）：修第 11 行自指、更新各线尖端提交号、如实记载三条新线与 BeadFlow 的存在及地位；同批在 DECISIONS 追一条（D61+）宣布实现收敛路径——建议正面裁决 `cursor/soul-integration-4a8e` 是否为「经宣布的后继主干」（它是主干的合法后代、已闭合 M2 并接通 D48 产品路径），或明确否决。STATUS/DECISIONS 追加不触碰 PLAN_FROZEN 辖域（冻结管计划内容，不管进度记账与新板追加）。
3. **【PR 卫生】** 关闭 PR #4（执行 D49）、PR #1、PR #3；把 PR #6（BLOCKERS）合进 main，按 DECISIONS 脚注处理 D32 撞号。
4. **【合并准备，分支上先演练】** 在第 2 步选定的实现尖端上做 schema 收敛演练：取 main 类型化的 `relationship.schema.json` → 跑 `xtask schema-freeze` 重算 lock → 全量测试。先证明 D59 义务能闭合，再处理 PR #2 的冲突（冲突面是文档 + schema lock，不是产品代码，主干 STATUS 已确认）。
5. **【repo 拓扑一行】** README / PLAN_INDEX 拓扑节补一行 BeadFlow 指引（事实陈述，不动产品锁）。
6. **不做的**：不启动 Goal 2（Goal 1 未关，D54 双门未过）；不做 AC-27（v0.1.1）；不 empty-commit 触发 CI；不在主干与整合线之外再开实现线。

---

## 附录：本次核验的关键证据

- main 历史：`git log --oneline main` → 8 提交（ea6f62f…a0ec14b）。
- 主干未吸收算法 crate：`origin/cursor/soul-goal1-7b1c:Cargo.toml` 成员表 16 个 crate，无 soul-algo-*；`crates/soul-graph/src/build.rs:118` 仍有本地 `fn band`。
- 整合线已吸收：`origin/cursor/soul-integration-4a8e:Cargo.toml` 成员表 18 个 crate（含两个算法 crate）；`soul-graph` 依赖 `soul-algo-tie`，`build.rs` 走 `soul_algo_tie::Band`。
- schema 分叉：`git diff main origin/cursor/soul-goal1-7b1c -- docs/schemas/` → `tie_strength` 整节（约 120 行）在 Goal1 侧退回裸 object，lock 哈希不同。
- BeadFlow 隔离：`git diff origin/cursor/first-test-candidate-c441 origin/cursor/beadflow-integration-c441` → 195 文件全部新增，无 Soul 既有文件修改（共享 `pnpm-lock.yaml` 仅追加）。
- 测试：`cargo test --workspace`（main，Rust 1.83.0）→ 19 个 test target 全部 ok。
- PR 状态：`gh pr view 2 4 5 6 7 8`（详见第 1 节表）；`gh pr list` 全量 56 个 PR。
