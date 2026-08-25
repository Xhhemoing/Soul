claude-fable-5-thinking-xhigh

（自报依据：本会话系统身份为 Claude Fable 5，与请求 slug 同系列同代；运行时未向本代理暴露服务端变体串，无任何降级信号。若父代理侧记录的实际 slug 与此不符，以父代理记录为准并视为本行失效。）

# Round 3 — fable-b：文档 vs 代码拓扑交叉核验

分支：`cursor/goal1-close-loop-a073`（未开新分支）。核于 2026-08-25 ~07:00 UTC，HEAD `b210bfd`（含本轮 opus-a/opus-b/gpt-sol 已落的 Round 3 提交）。只写本报告，不改 docs，不碰代码。

## 一句话结论

PR 目标确认无误：本分支必须、且实际已经对着 `cursor/goal1-unblock-a073`（PR #10），不是 `main`。树内文档与代码拓扑基本对齐；发现 **1 条新的跨分支拓扑漂移（P1，需父代理裁决）** 与 **2 条树内残留旧句（P2/P3，单句级）**。历史 D1–D60 措辞按任务豁免，未计入矛盾。

## PR 目标确认（任务第二问）

**必须对 `cursor/goal1-unblock-a073`，不能对 `main`。当前已经如此，无需动作。**

1. **GitHub 实况**（`gh pr view`，核于本轮）：PR #10 OPEN，`headRefName: cursor/goal1-close-loop-a073` → `baseRefName: cursor/goal1-unblock-a073`；PR #7 OPEN，`cursor/goal1-unblock-a073` → `main`。合入链是 #10 → #7 → `main`。
2. **祖先关系**：`origin/cursor/goal1-unblock-a073` 尖端 `6133307` 是本 HEAD 的严格祖先；本分支在其上 34 个提交，`6133307` 又在 `origin/main`（`a0ec14b`）之上 177 个提交。对 base 开 PR，diff 恰好是本轮 34 条；若对 `main` 开，会把 PR #7 还没合的 177 条整体重复一遍，等于绕开 PR #7 另立合入路径。
3. **文档裁决**：D61「主干由 `cursor/soul-goal1-7b1c` 更替为 `cursor/goal1-unblock-a073`（PR #7，合入路径）」；R2-SYNTHESIS「权威只认 `cursor/goal1-close-loop-a073` → 合回 PR #7」「禁止对这些侧枝开对 `main` 的 PR」。与上两条代码事实一致。

## 对拍通过项（docs 声明 ↔ 代码实况）

| 文档声明 | 代码核验 | 结果 |
|---|---|---|
| PLAN_INDEX §3 / FORMAL：`main` 合入前只有计划面 + 冻结算法 crate，无可安装应用 | `git ls-tree origin/main`：只有 `crates/soul-algo-tie`、`crates/soul-algo-trait` + `docs/`，无 `apps/` | ✅ |
| FORMAL 计划面与代码面：本树 WP01–WP11、WP13 落地（桌面壳、加密库、导入、图谱、记忆、审计、smoke） | 18 个 crate（含 `soul-store`、`soul-graph`、`soul-import`、`soul-policy`、`soul-win-dpapi`）+ `apps/desktop`（含 `src-tauri`）在树 | ✅ |
| PLAN_INDEX §2：十份正文 = 九份存储契约 + 一份导入契约，另有 `_defs` 与 `schemas.lock.json` | `docs/schemas/` 恰好 12 个文件：9 存储 + `soul-import-v1` + `_defs` + lock | ✅ |
| PLAN_INDEX §2：`BLOCKERS.md` 仍在 PR #6，本树没有这份文件 | 本树 `docs/` 无 `BLOCKERS.md` | ✅ |
| PLAN_INDEX §2/§4：`GOAL1_PLAN.md` 本树已有；第二份 `PRODUCT.md` 禁止存在 | `docs/GOAL1_PLAN.md` 在；全库 glob 无任何 `PRODUCT.md` | ✅ |
| D61 + STATUS「下一步」第 2 条：ci.yml 自动 `push` 列 `main`、`cursor/goal1-unblock-a073`、历史分支 `cursor/soul-goal1-7b1c`；无 `pull_request` | `.github/workflows/ci.yml` L24–33 逐字一致；头注释（L4–10）与 D61 的主干叙述一致 | ✅（与 R3 gpt-sol-b 探针互证） |
| PLAN_INDEX 表 2 行「D1–D61」、行「D49/D61」 | `docs/DECISIONS.md` 表尾确到 D61，D49 原行未改写（R3 gpt-sol-b 已逐字节验证） | ✅ |
| R2 落地物 | `crates/soul-graph/tests/t4d_product.rs`（CODE-2）在；`last_contact_utc` 逻辑在 `crates/soul-graph/src/build.rs`（CODE-3）在；工作区干净、`Cargo.lock` 相对 base 零 diff | ✅ |

## 残留矛盾

### C1（P1，跨分支拓扑漂移）：`cursor/soul-goal1-7b1c` 不再是「静止的历史祖先」

**文档说**（均为现在时事实句，不属于 D1–D60 历史措辞豁免）：

- PLAN_INDEX §3：「`cursor/soul-goal1-7b1c`：历史 Goal 1 HEAD，**是本分支的祖先**，不是合入路径。」
- FORMAL 计划面与代码面表：「实现主干的祖先……不要从它另开第二条实现线。」
- STATUS「各条线」表：「尖端：**历史祖先**」；D61：「降为历史祖先」。

**代码实况**（核于 2026-08-25 ~06:55 UTC 的 `origin/cursor/soul-goal1-7b1c`，尖端 `650b0f2`）：

- `git merge-base --is-ancestor origin/cursor/soul-goal1-7b1c origin/cursor/goal1-unblock-a073` **失败**。两线 merge-base 为 `80c9011`（2026-08-25 01:19 UTC）；7b1c 在其后有 **46 个主干不可达的提交**，时间跨 01:19–06:48 UTC——最后一条距本轮开工仅数分钟，作者均为 `Cursor Agent <cursoragent@cursor.com>`。
- 这 46 条**不是只有 docs**：改了 `apps/desktop/src/routes/{Wizard,Graph,Collect,Files,Memory,Profile}*`、`src-tauri/{lib,tray,instance}.rs`、`ipc_roundtrip.rs`、`ci.yml`、两份 `Cargo.lock` 等。例：`478f19f` fix: wizard pitch does not claim later E1 stays local、`0de3e90` fix: graph local-only line names the summary as an E1 trigger、`90c2d25` fix: endpoint notice names both draft generate and graph summary。
- **内容真的分叉了**：以 `Wizard.tsx` 为例，7b1c 侧有主干没有的 E1 例外句（「只有你以后自己填写的模型端点例外，发出去的内容会先占位」）与 `data-testid="wizard-locality"`；主干侧是较早的短句版。即那边有本主干缺的文案修正，这边有那边缺的 37 个提交（含算法吸收与本轮全部工作）。

**定性**：这些提交不可能来自本 close-loop 谱系（我们没碰 7b1c）。最像是 D61 拍板前的旧 Goal 1 代理线仍在运行并继续推它自己的分支——恰是 FORMAL「不要从它另开第二条实现线」描述的局面，只是方向相反：不是我们开了第二条线，是旧线没停。文档写下时为真（「降为历史祖先」是裁决），是**世界变了**；但 PLAN_INDEX §3 / FORMAL / STATUS 的「祖先」句现在对着 ref 尖端为假。

**后果与建议（报告级，本轮不执行）**：
1. 父代理需裁决那 46 条的归宿：有价值的文案/测试修正 cherry-pick 进主干，或在 STATUS 按 D57（带分支+提交+核于日期）显式记「7b1c 尖端已漂移至 `650b0f2`，其增量视为被主干取代/待摘取」。二选一，不裁决则两线会继续互不知情地分叉。
2. 连带风险：ci.yml 保留 7b1c 为 push 触发（当时理由是「还有 ref 指着它」），如今那条线**活跃推送**——Actions minutes 恢复后，旧线每次 push 都会花掉 STATUS 说应留给 Goal 1 HEAD 的分钟。是否从触发列表移除 7b1c，同样交父代理裁决。
3. 本条不影响 PR #10 的目标确认：合入路径仍是 #10 → #7，与 7b1c 无关。

### C2（P2，树内单句过期）：STATUS 里程碑段的 CI 触发句自相矛盾

STATUS 第 13 行（里程碑长段）内有：「`.github/workflows/ci.yml` 现在只自动 `push` `main` 与 `cursor/soul-goal1-7b1c`」。而实际 ci.yml（L24–28）与 STATUS 自己「下一步」第 2 条（`f8fe32f` 更新）列的是**三条**分支，含主干 `cursor/goal1-unblock-a073`。R-1 修复（`f8fe32f`）改了 ci.yml 和「下一步」，漏了里程碑段这一句。句中「现在」使其成为现时事实句而非历史吸收句；但 R2 结论「STATUS 不改写历史吸收句」的边界怎么划，交父代理裁决——若判定可改，最小修法是把该句改为与「下一步」第 2 条一致的三分支表述，或删「现在只」二字所在半句。本轮只报不改。

### C3（P3，树内单格过期）：STATUS「各条线」表主干行的「尖端」格

「各条线」表第一行写主干尖端为「吸收 `origin/main`（计划文档 PR #8）**中**」。该 merge 早已完成（`a30bd6f` merge: absorb plan-doc freeze from main 在主干历史里；STATUS 进度表也写「本 merge 完成」）。「中」字过期。同格的「计划权威面 D1–D60」属历史吸收措辞，按任务豁免不计（opus-b 本轮已在表后追加 D61 对齐句 `341c921`，语义上已被纠正，剩的只是这一个字）。

## 按任务豁免、未计入矛盾的历史措辞

FORMAL L16「D1–D60」、STATUS L9「D41–D60」、STATUS 各条线表「D1–D60」、R2-SYNTHESIS 内旧句。均为写作当时为真的吸收记录，且 D61 对齐句已由 opus-b 落在其后。

## 没做什么

- 没改任何 `docs/**`、代码、ci.yml；没有 cherry-pick 7b1c 的任何提交。
- 没开新分支，全程在 `cursor/goal1-close-loop-a073`。
- 没跑测试套件（静态核验；本轮测试归 gpt-sol-a/opus-a）。
- 没动 PR：#10 的 base 已正确，无需改。
- 未启动 Goal 2，未触碰 F04c，无 empty-commit。
