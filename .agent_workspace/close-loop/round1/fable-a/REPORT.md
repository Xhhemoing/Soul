MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 1 fable-a：Goal 1 收口差距审计（close-gap audit）

审计对象：`cursor/goal1-close-loop-a073` @ `10da234`（等于 trunk `cursor/goal1-unblock-a073` @ `6133307` + 一条 docs 提交）。
**同轮并发修正**：审计进行期间兄弟槽位推了 `9fd6870`（opus-a，R-1 修复）等提交；凡受其影响的结论已按 `9fd6870` 复核并在文中标明，其余 file:line 仍核于 `10da234`。
门禁口径：D54 —— FORMAL 验收矩阵全部 v0.1 行 **与** PRODUCT_LOCK 十三片切片**同时**过。
逐项分类表在同目录 `CLOSE_CHECKLIST.md`；本文只写结论、父代理种子项的核实、与 Round 2 建议。

## 一、总结论

**本树已经没有成片的代码缺口。** 剩余收口项按类计：

| 类 | 数量级 | 内容 |
|---|---|---|
| already-done | 矩阵 33 行中 28 行 + 切片 13 片中 9 片 + 种子项 3 项（含本轮 `9fd6870` 关掉的 R1-LEGACY）+ 历史回归 3 条 | 见检查单 |
| code-now | **2 项**（都是测试收紧，不动产品代码） | 见第三节 |
| docs-now | 2 项（都在 10 行以内） | D49 追认接班 + STATUS 的 D57 复述补半句 |
| author-manual | 切片 1/7/11 的真机半边 + AC-01 + NSIS 签名 + DPAPI 真机确认 | `scripts/author-manual-checklist.md` 七条门禁节 |
| minutes | AC-26 / 切片 13 的 HEAD hosted 五门 | 分钟恢复后 `workflow_dispatch` 本分支，不 empty-commit |
| frozen-wont | 8 项 | 全部有拍板或明确定价（D34/D35/D36/D44/D55、AC-27、9999 摆动、`parse_rfc3339` 双份、COPY_ZH §4 漂移） |
| other-PR | 3 项 | PR #7 合入、PR #6 后合（含 D32 撞号处理）、PR #4 关闭 |

换句话说：**Goal 1 关不上的原因只有两个不归代码管的东西（hosted 分钟、作者 Win11 真机），加上两个一轮 opus 就能收掉的测试小项。**

## 二、父代理种子项核实（两项是错的）

### CI-TRUNK —— **错，已完成（already-done）**

种子说「`.github/workflows/ci.yml` auto-push 仍只列 `main` + `cursor/soul-goal1-7b1c`」。不符事实：

- `.github/workflows/ci.yml:24-28` 的 `push.branches` 是 `main`、**`cursor/goal1-unblock-a073`**（第 27 行）、`cursor/soul-goal1-7b1c`（第 28 行，注释第 7-10 行解释为何保留祖先分支触发）。
- 五个 job 的 `if:` 门（第 51、108、168、241、275 行）都包含 `refs/heads/cursor/goal1-unblock-a073`。
- 修复提交是 `f8fe32f`（`ci: trigger auto push on the Goal 1 trunk cursor/goal1-unblock-a073`）。
- 种子的两条约束现状也满足：无 `pull_request` 触发（第 20-23 行注释写明理由）；`workflow_dispatch` 在第 33 行。

Round 2 **不要**再动 `ci.yml` 触发面。

### R1-LEGACY —— **对（缺口属实），且已在本轮被 `9fd6870` 关闭（already-done）**

缺口在审计基线 `10da234` 上逐条复核成立：

1. `crates/soul-graph/src/model.rs:104-106`：`algorithm_id: String` 带 `#[serde(default)]`、**无** `skip_serializing_if`；分列计数（第 78-92 行）与 `silent_days`（第 98-99 行）同理恒序列化。
2. `crates/soul-graph/src/correct.rs:98-114`（`correct_tie`）与 `:144-160`（`release_tie`）读整包 → 改锁字段 → `rewritten()`（`:207-226`）整包 `serde_json::to_value` 写回 `put_relationship`，无任何 legacy 检查；`machine_reading()` 注释（`:197-204`）明说 legacy 纠正是有意支持的路径。
3. `docs/schemas/relationship.schema.json:31-34`：`algorithm_id` 枚举只有 `T4D`/`T4`，空串必拒；即便跳过空串序列化，`:106-115` 的 `dependentRequired`（`direct_out_count → algorithm_id` 等八条）也会拒掉恒序列化的 `direct_out_count: 0`。
4. trunk 尾部提交 `6133307` 只改了 `soul-schema/tests/schema_wiring.rs` 的标识符遍历（`algorithm_id` 不当 uuid7 查），**没有**触碰本缺口。

**同轮关闭**：opus 槽位提交 `9fd6870`（`soul-graph: score a pre-wiring edge before locking its band (R-1)`）落地了种子要求的形态——`correct.rs` 新增 `scored()`：`algorithm_id` 非空原样返回；为空先跑一次 `rebuild`（审计条目照写）再取重打分的行；重建后仍打不出分的边以新增的 `GraphError::UnscoredEdge` 具名拒绝。schema 未动（无放松枚举、无跳过零值），`graph_correction.rs` +188 行回归测试。**Round 2 只需复核这条提交，不要重新实现。**

### DOC-D49 —— **对，确认为 docs-now**

`docs/DECISIONS.md:60`（D49 行）仍写「唯一实现主干 | `cursor/soul-goal1-7b1c`」，而 STATUS 第 9 行、FORMAL 第 20/27 行、PLAN_INDEX 第 35 行都已把现行唯一主干写成 `cursor/goal1-unblock-a073`（PR #7）。处置：**追加 D61** 记录主干接班（D49 其余三条——停 `agent/dev-sota`、关 PR #4、`cursor/` 前缀——继续有效），不改写 D49 原文。10 行以内，符合父代理直改白名单（FORMAL 11.3）。

### DOC-INDEX —— **错，已完成（already-done）**

种子说 PLAN_INDEX「只写常量名」一行与 D60 冲突。不符现状：`docs/PLAN_INDEX.md:26` 行尾已经写着「`algorithms/COPY_ZH.md` 与 `REJECTED.md` 允许引用常量表已钉的数字（D60）」。这正是 plan-polish R-2（`.agent_workspace/plan-polish/round3/fable-b/REGRESSIONS.md:21-31`）要求的豁免，由提交 `095c1f8`（Round 3 冻结）落地，本树经 `a30bd6f` 吸收。无事可做。

### M1-PR4 / AUTHOR / MINUTES —— 三项都对，分类维持

- PR #4（`agent/dev-sota`）实查仍 open（`gh pr list` 核于 2026-08-25）。本环境 `gh` 只读，关闭是父代理/作者的 GitHub 动作 → other-PR。
- `scripts/author-manual-checklist.md` 在树上（13 节，七条门禁节 + §8-10 可选）→ author-manual。
- hosted 空 runner：STATUS「当前里程碑」已锚定证据（`2e72ddf` 五门绿 run 32754617268；此后 HEAD 全部 0 step 空 run）→ minutes，不是代码任务。

## 三、Round 2 opus 的 code-now 建议（2 项，全部有界）

原第 1 项（R1-LEGACY 守卫，曾编号 CODE-1）已被本轮 `9fd6870` 实现，从建议中移除；Round 2 对它只做复核。剩两项，编号沿用：

**CODE-2（测试收紧，不动产品代码）：具名夹具在产品边界对拍，收严 AC-28/29/30。**
FORMAL 第 162 行原话是「Goal 1 侧应当**导入**它们并断言产品路径与算法 crate 同判」，但 `crates/soul-graph/tests/` 目前没有任何文件 import `soul_algo_tie::testing`（t4d_band/t4d_product 都是手搭等价形状）。补一个对拍测试：把 `group_heavy_plus_one_direct_each_way`、`lilei_12`、`group_heavy_plus_three_directs`、`dormant_2019` 四个具名夹具灌进真库 → rebuild → 断言每条边 band 与 `soul_algo_tie::score` 同判；顺带补上三个现缺断言：AC-28 决胜边的 `direct_active_day_count`（`t4d_band.rs:161-167` 现只断四个分列计数）、AC-29 的 `group_heavy_plus_three_directs`=Moderate 与锚同测、AC-30 的两条边 `as_of_utc` 全等。禁止在测试里复述阈值数字（红线 11——夹具规模数字可写，门槛数字不可写）。

**CODE-3（测试收紧，一处断言）：AC-34 的 `last_contact` 半句。**
AC-34 的 Then 有两半；`crates/soul-import/tests/import_to_graph.rs:157`（`an_owner_group_message_does_not_write_one_outgoing_row_per_speaker`）只断了「无 Outgoing 行」那一半。夹具本身已经把 owner 的消息排在最后（10:00 晚于全部发言人的 09:xx），正好用来断第二半：rebuild 之后每条边的 `tie_strength.last_contact_utc` 等于该 peer 自己那条 09:xx 消息，而不是 owner 的 10:00。一处断言，改一个测试函数。

两项都是纯测试：不触碰冻结 crate、不动 schema、不加命令、不加依赖。除此以外**没有第三个 code-now**：gpt-sol-a 同轮报告里 AC-32/AC-33 的「无单一整合测试」读法比矩阵行文更严——那两行的每个 Then 子句都已各有产品路径断言（见检查单），不构成收口缺口，Round 2 不要为它们加测试。

## 四、明确不在范围内（Round 2/3 谁都不做）

1. **Goal 2**（D28，SHARED_BRIEF 硬禁）。
2. **F04c 第三道降档门**（D44，PRODUCT_LOCK 不可协商约束 13）。
3. **GC-6/7 遗忘 vs 锁**（D34：Goal 1 不实现，不发明墓碑/Option 时间戳）。
4. **GC-9b「由你本人指定」**（D35：COPY_ZH 未冻结该 key 前禁入；现存唯一出现处是允许的反向断言 `locked_tie_summary.rs`）。
5. **COPY_ZH §4 vs `a2.rs` 文案漂移**（不改冻结 crate；R3-SYNTHESIS 已记）。
6. **合法 9999 年摆动 as_of**（G4/P1 已定价；D37 只挡不可表示年）。
7. **v1 按发言人数判 Direct**（D36 维持）。
8. **双份 `parse_rfc3339` 合并**（非 P0 重构，不是 D54 门项）。
9. **AC-27 文件写执行**（v0.1.1，不是 Goal 1）。
10. **导入幂等**（D55：P1 设计目标，不进矩阵不进切片）。
11. **重跑 WP 拆分 / CreateGoal**（FORMAL「开工第一动作」第 3 步）。
12. **`ci.yml` 触发面**（已修好，见 CI-TRUNK；加 `pull_request` 被禁）。
13. **整份拷贝 BLOCKERS.md**（仍在 PR #6，合入顺序在 PR #7 之后；届时处理 D32 撞号，`docs/DECISIONS.md:73` 已写好处置）。
14. **empty-commit / `cargo generate-lockfile`**（SHARED_BRIEF 硬禁）。

## 五、方法与时效

- 全部 file:line 证据核于本树 `10da234`（2026-08-25）。跨 GitHub 的事实（PR #4 open、空 run 编号）核于同日 `gh` 只读查询。
- AC-01…AC-26 的逐行证据以 `.agent_workspace/unblock/round3/fable-b.md` §1（核于 `016951d`）为底，其上列的五个残差（D37/D38/D39/D40、intake 对拍、`--no-fail-fast`、近阈值门、Graph.tsx 档位词）逐项在本树复核**均已落地**：`soul-draft/Cargo.toml:22` + `src/a2_adapt.rs`（D40）、`soulcore/src/commands/profile.rs:284,306`（D39）、`soul-import/src/telegram.rs:316`（D37）、`soul-import/src/instant.rs:32-42` + 负例 `2026-02-31`（D38）、`soul-profile/tests/intake_replay.rs`、`ci.yml:195` 的 `--no-fail-fast`、`t4d_product.rs:303-373` 近阈值门、`Graph.tsx:34-36`「弱/中等/强」。
- 同轮 gpt-sol-a 的 TEST_LOG（本目录旁）在 HEAD 上四包 `cargo test -p soul-graph -p soul-profile -p soul-draft -p soul-schema --offline` 139 过 0 败，`xtask schema-freeze --check` 绿；本报告未重复跑。
- 本槽位只写了 `.agent_workspace/close-loop/round1/fable-a/` 下两个文件，未动任何产品代码与 docs。
