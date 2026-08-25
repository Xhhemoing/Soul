MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 3 fable-b — Round 2 落地独立交叉审计

绑定读物：`R2-SYNTHESIS.md` 全文 + `round2/fable-a/RESIDUAL.md`。
审计对象：提交 `c3d5960`（Round 2 落地）。
只读审计；只写本目录；未 commit。产物：本文件、`CLOSED.md`、`REGRESSIONS.md`、`probe/verify_r2_landing.py`、`probe/RESULTS.txt`。

**同树并发声明**：审计开始时（03:08 UTC）工作树干净。审计进行中，另一 Round 3 槽位于 03:10–03:12 在飞修改了 `FORMAL_WORK_PROMPT.md`/`PLAN_INDEX.md`/`STATUS.md`（未提交），03:13 另有 `27b4060` 提交（只加 `round3/opus-b/` 报告，不碰 `docs/`）。处置：本轮对那三份文件的逐行判读用的是提交版字节（读取发生在其落笔之前，可由内容差异证明）；探针在 `c3d5960` 干净 worktree 与在飞树上**各跑一遍，均 93/93 且 PASS/FAIL 逐条一致**（`probe/RESULTS.txt` 尾注）。`docs/schemas/**`、`DECISIONS.md`、`SECURITY.md` 两树字节相同。故全部判定对提交版成立，且不被在飞改动推翻。

## 结论

**Round 2 声称关闭的 P0/P1 全部经独立实测确认关闭；落地未复活任何 R1 缺陷；四条新暴露的回归（R-1…R-4）无一条阻塞本 PR 合 `main`**（其中 R-4 已被同树在飞改动修正，待提交）。计划面可以进入合并流程——按既有口径，这不等于 Goal 1 关闭。

## 本轮做了什么（与 Round 2 证据链的独立性）

1. **不采信 Round 2 的探针结果，全部重跑。** 关键理由：`round2/opus-a/RESULTS.txt` 的「57/57 passed」跑在中间稿上——其记录的 relationship 哈希 `350a7ffc…` 不等于落地的 `ebf049a9…`，其 schema 副本里根本没有 `machine_band`（P0-1 的三字段是在其探针跑完之后、落地之前补的）。本轮探针（`probe/verify_r2_landing.py`）自写、跑在最终字节上，**93/93 过**，覆盖：lock 哈希独立重算（11/11 相符且覆盖目录全量）、`tie_strength` 结构断言、28 条实例接受/拒绝探针（Draft 2020-12 + 本地 registry）、文档面 42 条（AC-28–34 存在性、AC-27 无表行、`180|360` 核心八文件零泄漏、D40/D52/D57/D59/脚注逐条、STATUS↔SECURITY 尖端一致、引导面注记、D27 无第二份 PRODUCT.md、L4 横幅）。
2. **跨分支事实用远程实测而非文档互证。** `git ls-remote`：`cursor/soul-goal1-7b1c = df5d2dd6…`，与 STATUS/SECURITY 同写的 `df5d2dd` 一致——P1-6 的「核于」在审计时刻真实成立，不是两份文档抄了同一个错值。
3. **P0-1 的关闭不止验 schema，还逐字段比对了会撞上它的代码。** 取 unblock 线 `c81c233` 的 `model.rs`/`build.rs`/`correct.rs`/`store.rs` 与主线 `df5d2dd` 的 `model.rs`：主线八字段边、unblock 重建边（未锁/锁定/释放三态）、`t4d_band.rs` legacy put 夹具、`roundtrip.rs` 的 `{"band":"moderate"}` 夹具——在最终 schema 下全部 valid；纠正类测试全部先 rebuild 再纠正，合并不会把那条线的 CI 打红。**唯一残余红点是 legacy 边直接走 `correct_tie` 的运行期路径**（`algorithm_id: ""` 被枚举拒），无测试覆盖、无现实数据能触发，定级 P2 并给了一句话修法——详见 `REGRESSIONS.md` R-1。

## 判定总表（细节与证据行号见 CLOSED.md）

| 类 | 关闭 | 未关闭 |
|---|---|---|
| R2-SYNTHESIS 表内（L1–L6、D52、G1+、D59） | L1（实证）、L2、L3、L5 本分支侧、D52、G1+ 计划面、D59 | L4 按拍板保留快照（非缺陷）；L6 维持原判（非动作项） |
| RESIDUAL P0 | P0-1（实证） | — |
| RESIDUAL P1 | P1-1、P1-2、P1-3、P1-4（本分支侧留痕齐）、P1-5、P1-6 | — |
| RESIDUAL P2 | P2-1、P2-2、P2-3、P2-4（取水印行备选） | P2-5（a/b 加固均未做）、P2-6（M2 行仍准确但精度注记未补）、P2-7（维持，无动作） |

## 回归（详见 REGRESSIONS.md）

- **R-1（P2）**：schema 收紧后，legacy 边经 `correct_tie`/`release_tie` 写回会被 `algorithm_id` 枚举拒——CI 不红（纠正测试全在 rebuild 之后）、现实无 legacy 库，但 `correct.rs` 有意支持该路径。修法一句话，归 Goal 1 合并检查单（D59 已有的 `schema-freeze` 义务旁补一条即可）。
- **R-2（P2）**：PLAN_INDEX 新增「其余文档写出数字即为缺陷」按字面会把冻结的 `COPY_ZH.md`（180×2）、`REJECTED.md`（180/360×2）、`DECISION.md` §1 判成缺陷，且把 FORMAL 红线 11（管产品 crate 源码）的辖域扩到了 docs。下次触碰 PLAN_INDEX 时给 `docs/algorithms/` 三份权威加豁免措辞。
- **R-3（P3）**：D57 在原行内增补（同 PR 里 D58 却坚持「原文不改、另立 D59」），且 STATUS 61–66 行的 D57 复述没同步新增的半句。不构成矛盾（复述让位于权威），但属于 Round 2 刚在 P1-2 修完的那类「同树不同步」账。
- **R-4（P2，已被在飞修正）**：Round 2 把 FORMAL 夹具溯源句的范围从「AC-28…AC-33」顺手扩成「AC-28…AC-34 …夹具不是新造的」，但 `crates/soul-algo-tie/src/testing/` 实测只有 `mod.rs`/`oracle.rs`，没有 AC-34 需要的群聊导出样本（synthesis 风险 3 本来就说 AC-34 依赖导入归因）。同树在飞改动已补「AC-34 是例外，样本在 Goal 1 侧新造」，待提交生效。

在飞改动与本报告的交集：它顺带关闭了 P2-4 残余（原话与 1–6 步整体入引用块）与 P2-6（M2 行补 `c81c233` 注记，与本轮实测一致），并修正 R-4；R-1/R-2/R-3 涉及的行在飞 diff 均未触碰，四条回归判定不变。

## 对下一步的意见

1. 计划面无阻塞，本 PR 可合 `main`。R-2/R-3 都是一行措辞的事，若父代理本轮还有落笔窗口可顺手带走；带不走也不该拦合并。
2. R-1 与 D32 撞号（DECISIONS 72 行脚注）一起，构成 Goal 1 / PR #6 合并当天的两条检查单——建议父代理在 Round 3 综合稿里把两条并排列出，免得散在两份文件里被分别漏掉。
3. P2-5 维持开放：快照目录已从「双源风险」降级为「旧版噪音」，最便宜的收口仍是删七份正文只留 README——留给合并后的清扫轮即可。
