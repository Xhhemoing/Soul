claude-fable-5-thinking-xhigh

（自报依据：本会话系统身份为 Claude Fable 5，与请求 slug 同系列同代；运行时未向本代理暴露服务端变体串，无任何降级信号。若父代理侧记录的实际 slug 与此不符，以父代理记录为准并视为本行失效。）

# Round 2 fable-b — CODE-2 / CODE-3 范围控制 + D61 时效裁定

核于 2026-08-25。**工作树在审查期间持续移动**：开始时 HEAD 为 `ee4337b`（分支 `cursor/goal1-close-loop-a073`），审查中并发落入 `d3b83f3` / `6244154`（gpt-sol 双探针，仅 `.agent_workspace/round2/` 文档）、**`032374f`（opus-b：CODE-3 落地）**，随后 opus-a 把共享检出切到堆叠分支 `cursor/goal1-r2-code2-df52` 并落入 **`2d96fa9` + `057e318`（CODE-2 落地）** 与 `2eac280`（其自报告）。本报告先按在飞工作稿审 CODE-2，落地后复核：落地版与被审稿在断言与改动面上逐字一致，差异仅 4 行格式重排（`last_direct_contact` 闭包换行），**判定对落地提交成立**。只读审查，未改任何 `docs/` 文件；产出仅本文件。

## 判定总表

| # | 项 | 判定 |
|---|---|---|
| 1 | CODE-2（落地 `2d96fa9`+`057e318`）是否发明新 AC 行 | **否**。三个新测试逐条回溯到 FORMAL 矩阵 AC-28/29/30 既有行的 Then 列，无一行新增；详见 §1 |
| 2 | CODE-2 是否把算法 crate 内部拉进产品侧 | **否**。夹具取自 `soul_algo_tie` 的**公开** `testing` 模块（`lib.rs:79` `pub mod testing;`），依赖箭头维持 Goal 1 → soul-algo（D51 / 红线 12）；`crates/soul-algo-*` 零改动、零新依赖边（`soul-graph/Cargo.toml:11` 早已依赖该 crate）。落地 diff 复核：`032374f..2eac280` 只含两份测试文件与 opus-a 自报告 |
| 3 | CODE-3（已落地 `032374f`） | **范围内**。仅 `crates/soul-import/tests/import_to_graph.rs` +56/−5，纯测试；断言全部回溯 AC-34 第二句 Then（D47 / G1+）；未触碰算法 crate，符合 FORMAL 的 AC-34 例外（导出样本 Goal 1 侧自造，不造第二套阈值） |
| 4 | 是否重开 Goal 2 | **否**。两份 diff 只动 Goal 1 矩阵行的测试文件；`docs/GOAL2_POLISH_PROMPT.md` 与 Goal 2 相关面零触碰 |
| 5 | 三个警戒项（Authorization 头 / 文件写执行 / F04c 门） | **全部未出现**，含对两份 opus diff 的逐行核与全树 grep；详见 §3 |
| 6 | D61（round1/fable-b `DOC_PATCHES.md`） | **APPLY**。`git diff cdca740..HEAD -- docs/` 为空，四个补丁的「旧文本」在现树逐字命中，D61 仍缺席；opus-b 施加即可，本 slot 未代施 |

---

## 1. CODE-2 范围核（落地 `2d96fa9` + `057e318`：`t4d_product.rs` +360/−1、`t4d_band.rs` +4）

**不发明新 AC 行。** 三个新测试与新增断言逐条对号：

| 落地内容 | 回溯到 |
|---|---|
| `the_decisive_group_flood_edge_is_weak_and_carries_its_direct_active_day_count`：决胜夹具入库重建后 Weak、边上落 `direct_active_day_count` 与四分列、`as_of_utc` 断言、末尾 T4 反证（同证据 T4=Strong） | AC-28 Then 逐句（含「此行变红」的反证半句） |
| `the_anchor_stays_strong_while_three_directs_reach_moderate_in_one_rebuild`：`lilei_12` 与 `group_heavy_plus_three_directs` **同库同 rebuild 同测试**，前者 Strong 后者 Moderate，群侧计数与决胜夹具对齐 | AC-29 Then 逐句（「在同一测试内一并断言」） |
| `both_edges_carry_the_same_store_wide_as_of_and_only_the_dormant_one_is_weak`：两条边 `as_of_utc` 集合恰为单元素、休眠边 Weak、per-peer as_of 反证 | AC-30 Then 逐句（「随每条边落库」+「per-peer 必须让此行变红」） |
| `t4d_band.rs` 既有测试补 `direct_active_day_count` 两条断言 | AC-28 Then 的落库字段清单 |

**不重推阈值、不改 algo crate。** 期望读数一律经 `assert_edge_matches_frozen` 向冻结 crate 现问（`soul_algo_tie::score`），不在产品侧复述门槛；稿内出现的字面数字全部是**夹具身份 / 夹具算术**（分列条数、活跃天数、休眠年份差的下界），无一等于任何门槛常量——符合 FORMAL「红线 11 自查」段的许可。`crates/soul-algo-*` 与常量模块零改动；`rfc3339_utc` 辅助函数调用的是冻结 crate 公开的日历函数（格式化方向），不是第二份解析器。

**公开面而非内部。** `pub mod testing` 无 feature 门、无 `#[cfg(test)]`，`t4d_product.rs` 此前就以同一路径消费该模块（本树 `soul-graph/src/t4d_adapt.rs` 亦经常规依赖使用该 crate）。CODE-2 没有要求、也没有实际发生任何「把 crate 内部搬进 soul-graph」或反向依赖。

**一处留白，非范围问题。** AC-28 Then 还有「解释文案同屏报出一对一与群里两个数」半句（round1 AC_MAP 记录的未覆盖片段），落地两提交均未见对应断言。若后续补在 `people_summary` 一侧，仍属**既有行内**收严，不是加行；若不补，属完成度问题，交 AC-REPROBE（gpt-sol-a）复跑时判 PARTIAL/COVERED，不构成本报告的范围警报。

## 2. CODE-3 范围核（`032374f`，已落地）

改动面：单文件 `crates/soul-import/tests/import_to_graph.rs`，在**既有** AC-34 测试内加断言——每位历史发言人的边与节点 `last_contact_utc` 停在其本人 09:xx 那分钟、owner 的 10:00 不出现在任何边的 first/last contact、`last_direct_contact_utc` 为 None。提交说明记录了活性验证（在 `commit.rs` 恢复旧 fan-out 时断言读到 10:00 而非 09:01），验证做完已还原，产品源码零改动。

三点范围结论：不加 AC 行（全部断言是 AC-34 第二句同一 Then 的字面化，附带的 None / first-contact 强化断言仍作用于同一夹具同一行，属收严不属扩权）；不碰算法 crate（AC-34 是 FORMAL:162 明写的例外——样本 Goal 1 侧自造，`soul-algo-tie` 里本来就没有对应夹具，`032374f` 也确实没引入）；不碰 Goal 2。

## 3. 三个警戒项（现树基线 + 两份 opus diff）

- **Authorization 头**：全树 grep `Authorization|Bearer` 仅命中 `soul_fileplan::Authorization`——那是**路径授权类型**（只读扫描的授权名单），与 HTTP 认证头无关，勿在后续轮误报；`apps/` 零命中；两份 opus diff 零命中。E1 面未被触碰。
- **文件写执行**：`soul-fileplan/tests/` 的 `no_write_api.rs` 与 `execution_is_refused.rs` 原样在场，无人为 AC-27（v0.1.1，frozen-wont）翻案；红线 7 / D19 完好。
- **F04c 第三道门**：`crates/` 全树 grep `F04c|第三道|third.*(demotion|gate)` 零命中；`soul-algo-tie/src/recency.rs` 的 `demote` 仍恰好两级、只引用既有两个沉寂常量名，没有第三个分支或第三个常量（D44 完好）。

截至本报告核毕（CODE-2 / CODE-3 均已落地入库），**无警报**。若 opus 后续增补引入上述任一项，按 R1-SYNTHESIS「明确不做」清单直接打回。

## 4. D61 裁定：**APPLY**

`git diff cdca740..HEAD -- docs/` 为空——`DOC_PATCHES.md` 的基准提交与现树在 `docs/` 上无一字差异。逐补丁核对：

| 补丁 | 旧文本现状 | 结论 |
|---|---|---|
| 1（DECISIONS 追加 D61） | 表仍止于 D60（`docs/DECISIONS.md:71`），D61 缺席；:73 的 PR #6 假 D32 备注原样在场，补丁的「空闲 ID 从 D62 起算」连带注意仍成立 | 待施加 |
| 2（PLAN_INDEX:12 跳号） | 该格仍写「已拍板的选择（D1–D60）」，整格逐字命中 | 待施加（与补丁 1 同提交） |
| 3（PLAN_INDEX:26 豁免句） | 末句仍是「`algorithms/COPY_ZH.md` 与 `REJECTED.md` 允许引用常量表已钉的数字（D60）」，整句逐字命中 | 待施加 |
| 4（PLAN_INDEX:25 指针补挂，可选） | 仍写「理由见 D49（唯一实现主干）与 `STATUS.md`」，整句逐字命中 | 待施加（依赖补丁 1） |

补丁文本无一处过时，行号漂移风险已由 DOC_PATCHES 自身的「按旧文本整句匹配」纪律覆盖。**APPLY，由 opus-b 施加**；本 slot 依派单未动 `docs/`。
