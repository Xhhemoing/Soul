MODEL_SLUG: claude-opus-5-thinking-high-fast

# Round 3 / opus-b：Goal 1 线残余工序终表（只读）

核于 2026-08-26 约 03:40 UTC。承接 R2 fable-b（Goal 1 关闭门）、R2 fable-a（治理与分支合法性）、R1/R2 opus-b（算法与 schema 接缝）。

**任务边界**：列出仍不完善的六类事项（M2、G1/G2/G3、CI 账单、作者手动清单、冲突文件、D61 是否在 `main`），不实现任何代码。

**纪律遵守声明**：未执行任何 `git checkout` / `commit` / `push` / `branch` / `merge`。全部结论来自 `git show` / `git ls-tree` / `git rev-parse` / `git merge-base` / `git rev-list` / `git cherry` / `git merge-tree --write-tree`（不落盘试合并）与只读 `gh`。执行过一次 `git fetch origin --no-tags` 以取得当日尖端；它只更新远端跟踪引用，未动本地分支、工作区与 `main`。`/workspace` 只新增本文件。

---

## 0. 终表（先看这一张）

「仍不完善」按**谁能修**分层，每行的「今天差什么」都是本轮实测，不是转述。

| # | 事项 | 状态 | 今天差什么 | owner |
|---|---|---|---|---|
| 1 | **CI 账单** | **未解除** | 08-24 17:05 之后所有 run 均 0 step、3 秒内 failure。**唯一一次五门全绿在 `2e72ddf`**，它是四条线的**共同祖先**，因此没有任何一条线的尖端有 hosted 证据 | 作者（唯一外部动作） |
| 2 | **作者手动清单** | **未开始，且已分叉** | 四棵树上勾选数都是 **0**；但条目数不同：主干 76 / D61 主干 69 / close-loop 69 / integration 77。**没有「76 项」这个单一数字** | 作者（勾选）+ 代理（归一条目） |
| 3 | **M2**（算法 crate 进成员表） | 主干**仍开**；另三线**已关且落法忠实** | 主干 `Cargo.toml` 16 成员；吸收线/close-loop/integration 18 成员。三线的 `soul-algo-{tie,trait}` 的 `src/` 与 `tests/` 与 `main` **树哈希逐字节相同**，只改了打包（删每 crate `Cargo.lock`、加 `license.workspace`） | 已完成，待随拓扑归一进 `main` |
| 4 | **G1**（图谱仍 T0） | 主干**仍开**；吸收线/integration **已修** | 主干 `build.rs:40–44` 仍是本地常量 `MODERATE_MIN_INTERACTIONS=3` / `STRONG_MIN_INTERACTIONS=10` / `STRONG_MIN_ACTIVE_DAYS=3`；另两线经 `t4d_adapt` 调 `soul_algo_tie` | 已完成，待归一 |
| 5 | **G2**（intake 绕轴锁） | 主干**仍开**；吸收线/integration **已修** | 主干 `soul-profile/src/service.rs` 内 `axis_is_locked` 只出现 2 次（都在 `record_axis_inference` 侧），`IntakeSkip::AxisLockedByUser` **0 次**；另两线分别 4 次 / 3 次 | 已完成，待归一 |
| 6 | **G3**（图边不可纠正） | 主干**仍开**；吸收线/integration **已修，但是两套实现** | 主干无 `crates/soul-graph/src/correct.rs`；吸收线 302 行、integration 345 行，**45 行有别**，试合并时是 add/add 冲突 | 已完成，但**两份需要对账后二选一** |
| 7 | **冲突文件** | **三个不同的冲突面，无人做过任何一个** | 主干×吸收线 = **5 文件**；主干×`main`（PR #2）= **13 文件**（全是文档/锁，无产品代码）；**integration×吸收线 = 22 文件**（含 `correct.rs`、`a2_adapt.rs`、`locked_tie_summary.rs` 三个 add/add）；integration×`main` = 14 文件 | 代理，无 owner |
| 8 | **D61 是否在 `main`** | **不在，且是两条互不相容的 D61** | `main` 的 `DECISIONS.md` 止于 **D60**。close-loop 的 D61 = 主干更替（拓扑）；integration 的 D61 = 空图/姓名残差（产品）。**同号异义，两条都自称只追加** | 父代理/作者裁决 |
| 9 | **拓扑归一** | **未开始** | **全仓库没有任何一条分支同时包含主干尖端与吸收线尖端**（107 个远端分支逐条实测，命中 0） | 代理，无 owner |
| 10 | **D52 跨 crate 常量钉相等** | **只在一条线上关闭** | `crates/soul-draft/tests/day_constants_agree.rs` **只存在于 `soul-integration-4a8e`**，且两半都做全了。吸收线与 close-loop 上仍不存在 | 已完成，但载体是 BLOCKED 分支 |
| 11 | **PRODUCT_LOCK 权威面** | **integration 上是过期分叉 + 一处无 D 号私改** | 吸收线的 PRODUCT_LOCK 与 `main` **同一个 blob**；integration 的 = 主干旧版（**整节「灵魂层算法（v0.1）」与约束 13 缺失**）+ 两行 Telegram 导入口径翻转 | 父代理/作者裁决 |
| 12 | **COPY_ZH（`ALGO_FROZEN` 面）** | **integration 私增第 6 节** | integration 给冻结话术文件加了 61 行「降档时钟投影（AD-13）」，`main` / 吸收线均无 | 父代理裁决 |

**一句话**：Goal 1 的**代码工序其实已经全部有人做完了，但成品散在两棵互不相容的树上，而权威面跟着其中一棵、代码跟着另一棵**。剩下的不是写代码，是四件裁决 + 一次归一合并 + 两件只有作者能做的事。

---

## 1. 五棵树的坐标

本轮 fetch 后实测尖端：

| 线 | 尖端 | 提交时间 (UTC) | 相对 R2 |
|---|---|---|---|
| `main` | `a0ec14b` | 08-25 03:30 | 未动 |
| 主干 `cursor/soul-goal1-7b1c`（PR #2 → `main`，CONFLICTING） | `6d1058b` | 08-25 17:59 | 未动 |
| 吸收线 `cursor/goal1-unblock-a073`（PR #7 → `main`，MERGEABLE） | `6133307` | 08-25 05:02 | 未动 |
| close-loop `cursor/goal1-close-loop-a073`（PR #10 → 吸收线，MERGEABLE） | `d4490b0` | 08-25 07:08 | 未动 |
| integration `cursor/soul-integration-4a8e`（PR #15 → first-test-candidate，MERGEABLE） | `a27cfd1` | **08-26 00:45** | **+2 提交，仍在长** |

祖先关系（`git merge-base --is-ancestor` 实测）：

| 是否祖先 | → 主干 | → 吸收线 | → close-loop | → integration |
|---|---|---|---|---|
| `main` | 否 | **是** | **是** | 否 |
| 主干 | — | 否 | 否 | 否（差 1 个提交） |
| 吸收线 | 否 | — | **是** | 否 |
| first-test-candidate | 否 | 否 | 否 | **是** |

关键读法：

- **`main` 在吸收线里，不在 integration 里。** 这就是 PR #7 对 `main` CLEAN、而 integration 对 `main` 有 14 个 add/add 冲突的全部原因——integration 从来没有合过 `main`，它是**搬运**内容而不是合并。
- **主干只差 1 个提交就被 integration 完全包含**（差的那个是 `6d1058b`，纯 docs：hosted CI 是账单不是 Actions 故障）。而 D61 主干（吸收线）差主干 **49 个提交**。

---

## 2. 逐项核验

### 2.1 M2 — 算法 crate 进工作区成员表

`Cargo.toml` 成员数与算法 crate 在列情况：

| 线 | 成员数 | `soul-algo-tie` / `soul-algo-trait` |
|---|---|---|
| 主干 `6d1058b` | 16 | **均不在列** |
| 吸收线 `6133307` | 18 | 在列 |
| close-loop `d4490b0` | 18 | 在列 |
| integration `a27cfd1` | 18 | 在列 |
| first-test-candidate `85aae68` | 16 | **均不在列** |

R2 fable-b 记「吸收线已修」，我复现，并补一条 R1/R2 都没做的**保真核验**：吸收进去的是不是冻结的那份？

```
crates/soul-algo-tie/src      main == 吸收线 == integration   （树哈希相同）
crates/soul-algo-tie/tests    main == 吸收线 == integration
crates/soul-algo-trait/src    main == 吸收线 == integration
crates/soul-algo-trait/tests  main == 吸收线 == integration
```

四个目录在三棵树上树哈希**逐字节相同**。crate 根的差异只有两处，且都是入工作区必需的打包动作：删掉每 crate 自己的 `Cargo.lock`（-7 行），加一行 `license.workspace = true`。

**判定**：M2 的实质在三线已关闭，且**落法忠实，没有夹带算法改动**。这条可以从「待办」里划掉，剩下的只是「它跟着哪棵树进 `main`」——那是第 9 项拓扑归一的事，不是 M2 自己的事。

一处与 D50 字面的偏离照旧成立（R2 fable-b 已记）：D50 说「在 Goal 1 线上一次 merge 时加」，实际是在独立吸收分支上加的，事后由 close-loop 的 D61 追认拓扑。

### 2.2 G1 / G2 / G3 — 三个关闭级缺口

逐项源码级实读，外加五条具名验收测试的存在性扫描：

| 缺口 | 探测点 | 主干 `6d1058b` | 吸收线 `6133307` | integration `a27cfd1` |
|---|---|---|---|---|
| **G1** | `soul-graph/src/build.rs` | `MODERATE_MIN_INTERACTIONS=3`（:40）、`STRONG_MIN_INTERACTIONS=10`（:43）、`STRONG_MIN_ACTIVE_DAYS=3`（:44），`band()` 用全场地计数 = 逐字 T0 | `use crate::t4d_adapt::{tie_reading, ...}`，`soul_algo_tie::Band` 三档映射，无本地阈值 | 同吸收线（行号 +2） |
| **G2** | `soul-profile/src/service.rs` | `axis_is_locked` 命中 2 次（均在 `record_axis_inference` 侧）；`AxisLockedByUser` **0 次** | `axis_is_locked` 4 次；`AxisLockedByUser` 3 次 | 同吸收线 |
| **G3** | `soul-graph/src/correct.rs` | **文件不存在** | 存在，302 行 | 存在，**345 行** |

具名验收测试（`git grep -l` 全树）：

| 测试名 | 主干 | 吸收线 | integration |
|---|---|---|---|
| `what_the_user_says_to_a_group_is_attributed_to_nobody`（G1+ / AC-34） | 无 | 有 | 有 |
| `an_owner_group_message_does_not_write_one_outgoing_row_per_speaker`（G1+） | 无 | 有 | 有 |
| `re_answering_the_questionnaire_does_not_move_a_corrected_axis`（G2 / AC-31） | 无 | 有 | 有 |
| `a_corrected_band_and_its_verdict_survive_a_rebuild`（G3 / AC-32） | 无 | 有 | 有 |
| `a_corrected_edge_is_not_filed_under_anything`（GC-9 / D48） | 无 | 有 | 有 |

**判定**：G1/G2/G3 在主干**全部仍开**（与 R1、R2 一致，本轮第三次独立确认，主干自 R1 起零新提交）。三者在吸收线与 integration 上**各自独立修好了两遍**。

**本轮新增的判定**：G3 的两份实现**不是同一份**。`correct.rs` 302 行 vs 345 行、45 行有别，`git merge-tree` 把它报成 **add/add 冲突**。同类的还有 `soul-draft/src/a2_adapt.rs` 与 `soul-draft/tests/locked_tie_summary.rs`。所以「两条线都修好了」不等于「随便挑一条就行」——归一时必须对这三个文件做一次**语义对账**，不能机械取一侧。R2 fable-a 说「AD-2 的 port 是否等价于 unblock 线，目前没有人验证过」，本轮给出了这句话的量化下界：**至少 22 个文件需要人看**。

### 2.3 CI 账单

`gh run list --limit 200` 实测：

- **最新 run**：`32915840344`，08-26 00:37:41Z，Bead 线。两个 job 都 `"steps":[]`，`startedAt` 到 `completedAt` 间隔 **2–4 秒**，conclusion=failure。这是 runner 从未分配的签名，不是构建失败。
- **近 200 个 run 里成功的共 7 个，全部在 2026-08-24，全部在 `cursor/soul-goal1-7b1c` 上。**（R2 fable-b 写「只有历史那次」，那是 limit=100 的窗口效应；实际有 7 次，但结论不变，见下。）
- 其中只有 `32754617268`（08-24 17:05:33Z）跑满**五门**：`test (windows-latest)` / `test (ubuntu, headless)` / `sbom (cyclonedx)` / `lint (fmt, clippy, cargo-deny)` / `package (as far as CI can honestly go)`，步数 11/13/9/13/17。其余 6 次只跑了 lint + 两个 test。
- 那次 run 的 headSha = **`2e72ddf`**。实测它是**主干、吸收线、close-loop、integration 四条线的共同祖先**。

**判定**：AC-26 的 hosted 半边有且只有一份证据，它落在一个四线共有的、早于所有当前尖端的提交上。**没有任何一条线的尖端有 hosted 证据**，「D61 主干上零证据」这个说法要精确成「所有线的尖端都零证据」。

一条 R2 未记的、会挡住「账单一解除就能跑」的**结构性阻碍**——各线 `ci.yml` 的 push 触发列表：

| 线 | push 触发分支 | 自触发？ |
|---|---|---|
| `main` | **没有 `.github/workflows/ci.yml`** | — |
| 主干 | `main`, `cursor/soul-goal1-7b1c` | 是 |
| 吸收线（D61 主干） | `main`, `cursor/soul-goal1-7b1c` | **否** |
| close-loop | `main`, `cursor/goal1-unblock-a073` | 否（但让吸收线自触发） |
| integration | `main`, `cursor/soul-goal1-7b1c` | **否** |

也就是说：**账单解除的那一刻，D61 主干仍然不会自动跑自己的 CI**——把触发改到它身上的那个提交（`d4490b0`）在 close-loop 上，而 close-loop 尚未合入。所以顺序是 **PR #10 先合，吸收线才能自动跑五门**；在那之前只能靠 `workflow_dispatch` 手点。这条排在账单后面，但排在「在归一树上跑五门」前面。

### 2.4 作者手动清单

`scripts/author-manual-checklist.md` 逐树实数：

| 线 | 未勾选 `- [ ]` | 已勾选 `- [x]` |
|---|---|---|
| 主干 `6d1058b` | **76** | 0 |
| 吸收线 `6133307` | **69** | 0 |
| close-loop `d4490b0` | **69** | 0 |
| integration `a27cfd1` | **77** | 0 |

**这是本轮对 R2 的一处实质修正。**「76 项未勾」只在主干上为真。D61 主干（吸收线）的清单**比主干少 7 条**，少掉的恰好是主干那 6 个诚实文案修复与 2 个壳修复对应的验收步骤：

1. 窗口关掉后再从开始菜单启动一次 Soul（单实例托盘，`88cf931`）
2. 左键单击托盘图标窗口回来且不弹菜单（同上）
3. `msedgewebview2.exe` 进程照上面看一遍（WebView2 自有流量，`928ef5a`）
4. 首次向导欢迎那段要点名模型端点是例外（`478f19f`）
5. 「关系与证据」那行要写节点/边/密封姓名不进研究预览（`0de3e90`）
6. 输入框下那句与「这个壳会不会自己上网」端点那条（`90c2d25`）
7. 「记忆」页遗忘说明要写 SSD 块（`928ef5a`）

另有 1 条措辞不同（采集条数从「按『看现在的条数』」改成「刷新」）。76 − 7 = 69，对得上。

**这条把 R2 fable-b 的「合入即退步」从推断升级成了直接证据**：退步不只发生在代码里，它已经写进了作者要照着做的那张纸——按 PR #10 → #7 → `main` 合入，作者拿到的清单会**比今天主干的清单少 7 项真机验证**，而那 7 项正是「UI 如实写」义务（D15 / PRODUCT_LOCK 切片 1/6/8/10/11）的验收动作。

integration 的 77 = 主干 76 + 1（在特质轴页做一次纠正、看到「你纠正过」徽章——G2/G3 的真机验收步骤），并保留了主干全部 7 条。

**判定**：清单本身需要一次归一，而且归一方向已经明确——**以主干/integration 的条目为准，不能以吸收线为准**。勾选进度三棵树都是 0，这一项对作者的实际工作量是 77 项，不是 69。

### 2.5 冲突文件

`git merge-tree --write-tree`（不落盘）实测四个组合：

**(a) 主干 × 吸收线 = 5 个文件**（R2 fable-b 数字复现，逐字相同）

```
.github/workflows/ci.yml
apps/desktop/src-tauri/tests/ipc_roundtrip.rs
crates/soul-import/tests/telegram.rs
crates/soulcore/tests/session_e1.rs
docs/STATUS.md
```

分叉点 `80c9011`；主干侧独有 **49** 个提交（**18** 个碰代码），吸收线侧独有 **37**。`git cherry` 确认 patch 等价的恰好 1 个（`3161e02`），所以**待摘 17 个**。这三个数与 R2 fable-b 完全一致。

**(b) 主干 × `main`（PR #2）= 13 个文件**

```
.gitignore  Cargo.lock  Cargo.toml  README.md  rust-toolchain.toml
docs/DECISIONS.md  docs/FORMAL_WORK_PROMPT.md  docs/PLAN_VERIFY_PROMPT.md
docs/PRODUCT_LOCK.md  docs/SECURITY.md  docs/STATUS.md
docs/schemas/relationship.schema.json  docs/schemas/schemas.lock.json
```

**全部是文档与锁，一个产品代码文件都没有。** 主干 STATUS 自称「冲突面是文档与 schema 锁，不是产品代码」——这句自述**实测为真**，可以采信。

**(c) 吸收线 × `main` = CLEAN；close-loop × 吸收线 = CLEAN。** 这条链上没有冲突，PR #7 与 PR #10 的 MERGEABLE 状态是真的。

**(d) integration × 吸收线 = 22 个文件**（本轮新测，R2 未覆盖）

```
add/add：crates/soul-graph/src/correct.rs
         crates/soul-draft/src/a2_adapt.rs
         crates/soul-draft/tests/locked_tie_summary.rs
         crates/soulcore/tests/session_crash.rs
         docs/algorithms/COPY_ZH.md
content：.github/workflows/ci.yml  Cargo.lock  apps/desktop/src-tauri/Cargo.lock
         apps/desktop/src-tauri/tests/ipc_roundtrip.rs  apps/desktop/src/core.ts
         crates/soul-draft/Cargo.toml  crates/soul-draft/src/analysis.rs
         crates/soul-import/src/instant.rs  crates/soul-import/tests/import_to_graph.rs
         crates/soul-import/tests/soul_import_v1.rs  crates/soul-import/tests/telegram.rs
         crates/soulcore/Cargo.toml  crates/soulcore/tests/session_collect.rs
         crates/soulcore/tests/session_e1.rs  crates/soulcore/tests/session_screens.rs
         docs/DECISIONS.md  docs/STATUS.md
```

**(e) integration × `main` = 14 个文件**，形状与 (b) 同类（文档 + 锁 + 两个算法 crate 的 `Cargo.toml` + `COPY_ZH.md`），无产品代码冲突。

**判定**：冲突面不是一个数字，是三个，且**没有任何一个被处理过**。哪条路线成本最低取决于第 9 项怎么裁：走 PR #10 → #7 → `main` 只需要处理 (c)（零冲突）但要付第 2.4 与 2.6 节的退步代价；走「承认 integration」要处理 (e) 的 14 个；想要一棵真正的并集树，要处理 (d) 的 22 个。

### 2.6 D61 是否在 `main`

**不在。** `main` 的 `docs/DECISIONS.md` 最后一行是 **D60**。

逐树实测 D 号上界与 D61 行数：

| 线 | `DECISIONS.md` 末行 | D61 行 |
|---|---|---|
| `main` | **D60** | 0 |
| 主干 | **D31** | 0 |
| 吸收线 | **D60** | 0 |
| close-loop | **D61** | 1 |
| integration | **D63** | 1 |

**两条 D61 内容完全不同**：

- close-loop `a29f792`：「唯一实现主干是哪条分支（承接 D49）——主干由 `cursor/soul-goal1-7b1c` 更替为 `cursor/goal1-unblock-a073`（PR #7，合入路径）……`cursor/soul-goal1-7b1c` 降为历史祖先，不是合入路径。」
- integration：「空图 + 二次确认豁免 + 不在发言动词前的姓名——记成残差，不在 v0.1 加姓名识别。」

两条都写着「只追加」，两条都以为自己是 D60 的下一号。R1/R2 fable-a 记过「D61–D63 撞号」，但记的是**号段占用**；本轮把它精确成了**同号异义**：D61 这一个号今天有**两种互斥的语义**，其中一种还是决定「哪条分支是主干」的元决策。任何机械的「合入时改写为下一个空闲 ID」都会把这两条中的一条改成 D64，而**被改的那条如果是拓扑那条，主干更替这件事的编号就会在引用它的三份文档里全部失效**（PR #7 正文、close-loop R3 综合、close-loop 的 `ci.yml` 注释都写着「D61」）。

另外主干自己的 `DECISIONS.md` 止于 **D31** —— 主干这棵树上根本没有 D32–D61，所以「主干不知道 D61」不是它拒绝承认，是它的登记簿停在计划面合并之前。

**判定**：D61 不在 `main`，且在进 `main` 之前必须先解决同号异义。R2 fable-a 建议的「括注方案」只解决 BLOCKERS 里 dev-sota 那个旧 D32，**解决不了这一个**——这两条 D61 都是新写的、都在活跃分支上、都被别处引用。

### 2.7 拓扑归一：没有任何一棵树是并集

对全部 107 个远端分支逐条测「是否同时包含主干尖端与吸收线尖端」，**命中 0**。

进一步的分工事实（本轮新测，是本报告最重要的一条）：把主干独有的 18 个碰代码提交逐个测「是否已在 integration 里」：

| 结果 | 数量 |
|---|---|
| 已在 `soul-integration-4a8e` | **18 / 18** |
| 缺失 | **0** |

清单（全部 in-integ）：`478f19f` 向导承诺、`0de3e90` 图谱 local-only 行、`90c2d25` 端点提示、`17b56e9` 向导授权措辞、`f0a2363` 研究空状态、`928ef5a` 遗忘 SSD + WebView2、`88cf931` 单实例托盘、`6e358b3` Explorer 引号路径、`8cf5da5` 采集重读、`401a032` 采集页自读、`8e23e7f`/`c597358`/`4ed7695`/`79a1990`/`a0c329b`/`85d1b1a`/`28f106c`/`3161e02` 第三证明轮 IPC 级 AC 补证。

**这改写了 R2 fable-b 的第 B3 条。** R2 把「17 个提交待摘」记为「当前唯一能推进 D54 的代码工序，没有 owner」。实测：**这 17 个提交已经有人摘过了，摘进了 `soul-integration-4a8e`。** 那条线上同时还有 M2、G1、G1+、G2、G3、GC-9、第 10 项的 D52 测试，以及主干那 7 条手动清单项。

所以 Goal 1 的代码面并集**今天已经存在**，它叫 `soul-integration-4a8e`。它不是候选合入树，原因**全部在权威面而非代码面**：

1. 它的 PR #15 base 是 `first-test-candidate`，不是 `main`；它自封「exclusive parent branch」并把 → `main` 标成 BLOCKED，而 R2 fable-a 已经查明「宣布后继主干」的程序在任何权威面上都不存在。
2. 它的 `PRODUCT_LOCK.md` 是**主干的旧版**（blob `06b3d3f`，源自主干的 `5f85c42`），整节「灵魂层算法（v0.1）」与约束 13「口径以 `ALGO_FROZEN` 为准」**都不在里面**；而吸收线的 PRODUCT_LOCK 与 `main` 是**同一个 blob**（`7c028c6`）。
3. 在那份旧版之上还有一处**无 D 号的产品语义改动**：v0.1 Telegram 导入口径从 `main` 锁定的「Export chat history → Machine-readable JSON」翻转成「Settings → Advanced → Export Telegram data 全量导出；单聊 Export chat history 是另一形状，**拒收**」，连同 ASSUMPTION 行重写。`main` 明文承诺接收的形状，这条线宣布拒收。（R2 fable-a 已发现这条；本轮补上了「它同时还缺整节算法口径」这个更重的部分。）
4. 它给 `docs/algorithms/COPY_ZH.md` **加了一整个第 6 节**（61 行，「降档时钟投影（AD-13）」）。`docs/algorithms/` 其余 7 份文件与 `main` 逐字节相同，只有 `COPY_ZH.md` 分叉。COPY_ZH 是 `ALGO_FROZEN` 面。附带后果：R1 opus-b 记过 `DECISION.md` §6.4 交叉引用「COPY_ZH.md 第 6 节」而该文件只有 §0–§5（差一节的笔误）——现在 integration 上真有了一个第 6 节，内容与那条引用想指的「绑定测试断言」毫无关系，**那个笔误从「找不到」变成了「找到错的」**。
5. 它的 `FORMAL_WORK_PROMPT.md` 与主干同一个 blob（`fcf6b4c`），而 `main`（`876c93f`）与吸收线（`72008f4`）各是一份。四棵树三个版本。

对照之下，吸收线在权威面上是干净的：PRODUCT_LOCK ≡ `main`，`docs/algorithms/` ≡ `main`，`schemas.lock.json` ≡ `main`（blob `ad8c25c`，主干是 `feab8f7`）。

**这就是残余的真实形状：代码在 integration 上，权威面在吸收线上，两者 22 个文件冲突，没有人拥有把它们并起来这件事。**

### 2.8 D52 跨 crate 常量钉相等

R2 fable-b 记「三线皆无」。**实测有一线有，而且做全了。**

`git grep -l DORMANT_AFTER_DAYS` 全树扫描：吸收线与 close-loop 上该常量只出现在 `soul-algo-trait` 自己的三个文件（外加若干 `.agent_workspace` 归档稿）；**integration 上多出一个 `crates/soul-draft/tests/day_constants_agree.rs`**。

读其内容，D52 的两半都在：

- `the_demotion_clock_and_the_dormancy_clock_are_the_same_day`：`assert_eq!(DEMOTE_ONE_BAND_DAYS, DORMANT_AFTER_DAYS)`，两个名字直接相等，**测试里不写数字**（写了就是决议禁止的第三份拷贝）。
- `no_product_crate_writes_a_day_threshold_of_its_own`：扫 `soul-graph` / `soul-draft` / `soul-profile` / `soul-import` / `soulcore` / `apps/desktop` 六处 `src`，`.rs`/`.ts`/`.tsx`，跳过注释与 `.test.` 文件，用整数边界匹配（`1800` 不算命中）禁止任何第三处字面量。

选址也对：放在 `soul-draft`（同时依赖两个 crate 的下游产品 crate），而不是 R1 opus-b 建议的「给 tie 加 dev-dependency」——后者会让判档 crate 反向依赖渲染 crate。

**判定**：D52 从「已拍板未做」改判为「**已做，但只在一条无合入路径的分支上**」。它跟着第 9 项一起走：不裁 integration 的归属，这条测试就进不了任何会合入 `main` 的树。

---

## 3. 对前几轮的三处修正

| 出处 | 原说法 | 本轮实测 | 性质 |
|---|---|---|---|
| R2 fable-b §四.B3 | 「17 个提交没有吸收进 D61 主干……这是当前唯一能推进 D54 的代码工序，没有 owner」 | 18/18 已在 `soul-integration-4a8e`。工序已完成，缺的是**承认它的裁决**，不是**做它的人** | 定性改变：从「缺工」改判为「缺裁决」 |
| R2 fable-b §四.C5 | 「D52 的跨 crate 常量钉相等测试三线皆无」 | integration 上有 `day_constants_agree.rs`，两半齐全 | 事实修正 |
| R2 fable-b §四.A2 | 「作者 Win11 手动清单实数 76 个未勾选框」 | 76 只在主干为真；D61 主干 69，integration 77。清单本身已分叉，少的 7 条正是诚实文案的验收步骤 | 精度修正 + 新风险 |

另有两处**精度补正**（不改结论）：

- R2 fable-b「近 100 个 run 里成功的只有历史那次」——limit=200 下是 7 次，但只有 1 次跑满五门，且那次的 `2e72ddf` 是四线共同祖先，所以「所有线的尖端零 hosted 证据」比原说法更强也更准。
- R2 fable-a「soul-integration 的 `docs/algorithms/*` 整目录从 `main` 搬入」——8 份里 7 份逐字节相同，`COPY_ZH.md` 多了一整节。「正在向 `main` 收敛」这个信号成立，但 `COPY_ZH` 是反向的。

---

## 4. 残余工序终表（按依赖顺序）

不含实现建议，只列「谁、要决定或做什么、被什么挡住」。

### 只有作者能做

| 序 | 事项 | 被什么挡住 |
|---|---|---|
| A1 | 解除 GitHub Actions 账单 / spending limit | 无（外部动作）。解除后 hosted AC-26 与切片 13 才有可能拿到当前尖端的证据 |
| A2 | 走完 Win11 手动清单（77 项，今天 0 项） | 需要先有安装包与一棵定稿的树；且清单本身要先做 P2（见下） |

### 只有父代理/作者能裁（挡住后面所有代码动作）

| 序 | 裁决 | 不裁的后果 |
|---|---|---|
| P1 | **谁是并集树的载体**：追认 `soul-integration-4a8e`，还是把它的内容反向摘回 D61 主干 | 今天没有任何一棵树是并集；按现路径合入会丢 17 个代码提交、7 项手动清单、1 个 D52 测试 |
| P2 | **两条 D61 谁保号**（拓扑 vs 产品残差），另一条改到哪个号 | 同号异义。改错一边会让 PR #7 正文、close-loop R3 综合、`ci.yml` 注释里的「D61」三处引用同时失效 |
| P3 | **integration 的 PRODUCT_LOCK**：Telegram 导入口径翻转（拒收单聊导出）是追认补 D 号，还是回滚；缺失的整节「灵魂层算法（v0.1）」与约束 13 是否补回 | 这是全案唯一一处「改产品锁不留痕」。不处理就没法把那棵树当合入候选 |
| P4 | **`COPY_ZH.md` 第 6 节**（integration 私增的降档时钟投影）是否进 `ALGO_FROZEN` 面 | 冻结话术文件在两棵树上分叉；且会让 `DECISION.md` §6.4 那条本来只是笔误的交叉引用指向错误内容 |

### 代理可做，但被 P1–P4 全部挡住

| 序 | 工序 | 已量化的成本 |
|---|---|---|
| B1 | 拓扑归一 | 视 P1 而定：走 PR #10→#7→`main` 零冲突但退步；承认 integration 要处理 14 个文件（对 `main`）；做真并集要处理 22 个（对吸收线），其中 `correct.rs` / `a2_adapt.rs` / `locked_tie_summary.rs` 三个 add/add 必须语义对账，不能机械取一侧 |
| B2 | 权威面对齐 | D61 上 `main`；主干 STATUS 与 `main` STATUS 的旧拓扑句改写；`docs/BLOCKERS.md` **今天不在任何一棵树上**（只在 PR #6 分支 `cursor/blockers-analysis-a073`，OPEN 未合），而 D54 关闭门的规格正出自那里 |
| B3 | 手动清单归一 | 以 77 项（integration）为准，不能以 69 项（吸收线）为准 |
| B4 | `ci.yml` 触发归一 | 归一树必须让自己在 push 触发列表里。今天吸收线不触发自己，把它加进去的那个提交在未合入的 close-loop 上。这一步必须在 A1 之后、B5 之前 |
| B5 | 在归一树上跑五门 hosted | 依赖 A1 + B4 |

### 已定价、不挡关闭判定（滚动）

- COPY_ZH ↔ `a2.rs` 六类句漂移与「中」/「中等」词表违反（R1 opus-b §5.1，D40 承诺的「另记」至今不存在）。
- D32 `locked ⟺ user_band` 在 schema 层放行 3 格、误拒 1 格；D48 生效档三条放行（R2 opus-b §4）。
- `last_contact_evidence_id` 判档侧不生产、schema 侧无落点（R2 opus-b §5）。
- relationship 的 schema fixture 语料对 D59 棘轮与 D32 三字段零覆盖（R2 opus-b §4.4）。
- S2（AC-12 嵌中文名 session 缝夹具）、G4（重复导入幂等，D55 不进矩阵）。

---

## 5. 结论

**D54 今天在任何一棵树上都不可关**，这点与 R2 一致。但「差什么」的结构变了：

R2 的四件是「账单 → 76 项清单 → 拓扑归一（17 提交，缺人）→ 权威面对齐」。本轮实测后是：

> **账单（作者）→ 77 项清单（作者）→ 四件裁决（P1–P4，缺的是决定不是人手）→ 一次 14 或 22 文件的归一合并 + `ci.yml` 触发归一（代理，被裁决挡住）→ 在归一树上跑五门。**

最要紧的一句：**Goal 1 的代码工序没有欠账。** M2、G1、G1+、G2、G3、GC-9、D52、主干的 17 个诚实性与 IPC 补证提交——全部有人写完了、测过了。它们只是躺在一条自封 exclusive、PRODUCT_LOCK 是过期分叉、还带着一处无痕改锁和一个撞号 D61 的分支上。再派代理去写代码不会推进 D54；能推进它的是 P1–P4 那四个签字。

---

## 6. 复现

```bash
cd /workspace && git fetch origin --no-tags

# 尖端与祖先
for b in main cursor/soul-goal1-7b1c cursor/goal1-unblock-a073 \
         cursor/goal1-close-loop-a073 cursor/soul-integration-4a8e; do
  git log -1 --format="$b %h %cI" origin/$b; done
git merge-base --is-ancestor origin/main origin/cursor/soul-integration-4a8e; echo $?   # 1 = 不是祖先

# M2 与冻结 crate 保真
for b in cursor/soul-goal1-7b1c cursor/goal1-unblock-a073 cursor/soul-integration-4a8e; do
  git show origin/$b:Cargo.toml | grep -c 'crates/'; done
git rev-parse origin/main:crates/soul-algo-tie/src origin/cursor/goal1-unblock-a073:crates/soul-algo-tie/src

# G1 / G2 / G3
git show origin/cursor/soul-goal1-7b1c:crates/soul-graph/src/build.rs | grep -n 'MIN_INTERACTIONS'
git show origin/cursor/soul-goal1-7b1c:crates/soul-profile/src/service.rs | grep -c 'AxisLockedByUser'   # 0
git cat-file -e origin/cursor/soul-goal1-7b1c:crates/soul-graph/src/correct.rs; echo $?                  # 1 = 不存在

# 三个冲突面
git merge-tree --write-tree origin/cursor/soul-goal1-7b1c origin/cursor/goal1-unblock-a073 | grep '^CONFLICT'
git merge-tree --write-tree origin/cursor/soul-goal1-7b1c origin/main                      | grep '^CONFLICT'
git merge-tree --write-tree origin/cursor/soul-integration-4a8e origin/cursor/goal1-unblock-a073 | grep -c '^CONFLICT'  # 22

# 主干 18 个代码提交是否已在 integration
git rev-list origin/cursor/goal1-unblock-a073..origin/cursor/soul-goal1-7b1c | while read c; do
  git show --stat --format= --name-only $c | grep -qvE '^(docs/|.*\.md$|\.agent_workspace/)' || continue
  git merge-base --is-ancestor $c origin/cursor/soul-integration-4a8e && echo "in-integ $c"; done | wc -l   # 18

# 没有并集树
for r in $(git branch -r --format='%(refname:short)' | grep -v HEAD); do
  git merge-base --is-ancestor origin/cursor/soul-goal1-7b1c $r 2>/dev/null &&
  git merge-base --is-ancestor origin/cursor/goal1-unblock-a073 $r 2>/dev/null && echo $r; done   # 空

# D61 同号异义 / main 止于 D60
for b in main cursor/goal1-close-loop-a073 cursor/soul-integration-4a8e; do
  echo "## $b"; git show origin/$b:docs/DECISIONS.md | grep -E '^\| D6[0-9]'; done

# 手动清单四棵树
for b in cursor/soul-goal1-7b1c cursor/goal1-unblock-a073 \
         cursor/goal1-close-loop-a073 cursor/soul-integration-4a8e; do
  printf "%s " $b; git show origin/$b:scripts/author-manual-checklist.md | grep -c '^\s*- \[ \]'; done
diff <(git show origin/cursor/soul-goal1-7b1c:scripts/author-manual-checklist.md   | grep '^\s*- \[ \]') \
     <(git show origin/cursor/goal1-unblock-a073:scripts/author-manual-checklist.md | grep '^\s*- \[ \]')

# CI 账单
gh run view 32915840344 --json jobs --jq '[.jobs[]|{name,steps:(.steps|length)}]'      # steps 全 0
gh run view 32754617268 --json headSha,jobs --jq '.headSha'                            # 2e72ddf...
gh run list --limit 200 --json conclusion,headBranch,createdAt \
  --jq '.[]|select(.conclusion=="success")|"\(.createdAt) \(.headBranch)"'             # 7 条，全 08-24

# D52 只在 integration
git grep -l 'DORMANT_AFTER_DAYS' origin/cursor/soul-integration-4a8e -- crates/soul-draft
git show origin/cursor/soul-integration-4a8e:crates/soul-draft/tests/day_constants_agree.rs

# PRODUCT_LOCK / COPY_ZH 分叉
for b in main cursor/soul-goal1-7b1c cursor/goal1-unblock-a073 cursor/soul-integration-4a8e; do
  printf "%s " $b; git rev-parse origin/$b:docs/PRODUCT_LOCK.md; done
diff <(git show origin/main:docs/algorithms/COPY_ZH.md) \
     <(git show origin/cursor/soul-integration-4a8e:docs/algorithms/COPY_ZH.md)
```
