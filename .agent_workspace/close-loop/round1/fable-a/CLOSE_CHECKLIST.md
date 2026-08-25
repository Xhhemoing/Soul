# Goal 1 收口检查单（Round 1 fable-a）

核于 `cursor/goal1-close-loop-a073` @ `10da234`（= trunk `6133307` + docs），2026-08-25。
类别恰好七种：already-done / code-now / docs-now / author-manual / minutes / frozen-wont / other-PR。
「Owner (R2)」指 Round 2 的处置人；`—` = 无人做、按拍板冻结。

## 一、父代理种子项

| ID | 类 | 证据路径 | Owner (R2) |
|---|---|---|---|
| CI-TRUNK | already-done | `.github/workflows/ci.yml:24-28`（第 27 行列 trunk）+ job `if` 门 `:51,108,168,241,275`；提交 `f8fe32f`。种子过时 | —（勿再动触发面） |
| R1-LEGACY | **code-now**（CODE-1） | `crates/soul-graph/src/model.rs:104-106`；`crates/soul-graph/src/correct.rs:98-114,144-160,207-226`；`docs/schemas/relationship.schema.json:31-34,106-115`；`.agent_workspace/plan-polish/round3/fable-b/REGRESSIONS.md:5-19` | opus |
| DOC-D49 | **docs-now**（追加 D61，不改写 D49） | `docs/DECISIONS.md:60` vs `docs/STATUS.md:9`、`docs/FORMAL_WORK_PROMPT.md:20`、`docs/PLAN_INDEX.md:35` | 父代理直改（≤10 行，FORMAL 11.3）或 docs 槽 |
| DOC-INDEX | already-done | `docs/PLAN_INDEX.md:26` 行尾已有 D60 豁免句；提交 `095c1f8`，本树经 `a30bd6f` 吸收。种子过时 | — |
| M1-PR4 | other-PR | `gh pr list` 核于 2026-08-25：PR #4（`agent/dev-sota`）仍 open；本环境 `gh` 只读 | 父代理/作者（GitHub 关闭，不合入） |
| AUTHOR | author-manual | `scripts/author-manual-checklist.md`（§1-7 门禁，§8-10 可选）；`docs/STATUS.md:829` | 作者（Win11 真机） |
| MINUTES | minutes | `docs/STATUS.md:13,828`：`2e72ddf` 五门绿 run 32754617268；此后 HEAD 全部 0 step 空 run（最新 32796349061） | 等分钟恢复 → 父代理 `workflow_dispatch` 本分支；禁 empty-commit |

## 二、验收矩阵（AC-01…AC-26、AC-28…AC-34；AC-27 属 v0.1.1 不占行）

「already-done」= 本树代码+测试在场且本机绿（`docs/STATUS.md` 各 WP 节 + gpt-sol-a 同轮 TEST_LOG）；HEAD 的 hosted 复跑统一归 AC-26 那一行的 minutes，不逐行重复。

| ID | 类 | 证据路径 | Owner (R2) |
|---|---|---|---|
| AC-01 | author-manual（smoke 半边已 done） | `scripts/install-smoke.ps1`；`crates/soulcore/tests/install_smoke_script.rs`；`apps/desktop/src-tauri/tests/shell_is_local_only.rs`（托盘文案/asInvoker/Programs 目录）；真机托盘/UAC = 清单 §1-4 | 作者 |
| AC-02 | already-done | `crates/soulcore/tests/session_collect.rs`、`config_defaults.rs`；`ipc_roundtrip` | — |
| AC-03 | already-done（D39 已闭） | `crates/soul-profile/tests/questionnaire_intake.rs`；`crates/soulcore/src/commands/profile.rs:284,306`（`ignored`/`answered` 口径） | — |
| AC-04 | already-done | `crates/soul-store/tests/no_plaintext_at_rest.rs`；`crates/soulcore/tests/session_import.rs` | — |
| AC-05 | already-done（D37/D38 已闭） | `crates/soul-import/src/telegram.rs:316`（1970–9999）；`src/instant.rs:32-42`（按月天数含闰年）+ `2026-02-31` 负例；`tests/telegram.rs` 缺字段可读失败 | — |
| AC-06 | already-done | `crates/soul-graph/tests/ego_graph.rs`、`graph_correction.rs`（rebuild 后仍 cite 定档行） | — |
| AC-07 | already-done | `crates/soul-profile/tests/correction_lock.rs`；`soulcore/tests/session_screens.rs`（set_voice→文字变）；`session_e1.rs`（mock 实收字节含「热络」）；`soul-profile/tests/intake_replay.rs` | — |
| AC-08 | already-done | `ego_graph.rs`（≥3 节点、边带证据）；`t4d_band.rs:618` 源码无第二套阈值守卫 | — |
| AC-09 | already-done | `crates/soul-collect/tests/consent_gate.rs`；`session_collect.rs`（关=0） | — |
| AC-10 | already-done | `collection_lifecycle.rs`；`session_collect.rs`（开≥1、撤销 1s 内无新事件） | — |
| AC-11 | already-done | `crates/soul-egress/tests/e1_origin.rs`（精确 origin、跨 origin 302 不跟）；`session_e1.rs` | — |
| AC-12 | already-done（session 缝在场） | `crates/soul-policy/tests/redactor_leakage.rs`；`soul-draft/tests/wire.rs`；`soulcore/tests/session_e1.rs:603-777`（中文名在 session 缝对 mock 实收字节断言） | — |
| AC-13 | already-done | `redactor_exemption.rs`；`session_e1.rs`（第一次原文、第二次占位）；确认屏按钮 `apps/desktop/src/routes/Draft.tsx` | — |
| AC-14 | already-done | `soulcore/tests/profile_memory_commands.rs`；审计无内容 | — |
| AC-15 | already-done | `crates/soul-store/tests/forget.rs`（关库重开、CK 消失、orphaned）、`crash_recovery.rs` | — |
| AC-16 | already-done（D40 已闭） | `soul-draft/Cargo.toml:22` + `src/a2_adapt.rs`（冻结 `a2_render`）；`tests/people_summary.rs`；`session_e1.rs`（端点改写/诊断词回落） | — |
| AC-17 | already-done | `soulcore/tests/draft_commands.rs`、`netwatch.rs`（无 key 统计/模板 + 无非回环） | — |
| AC-18 | already-done | `crates/soul-fileplan/tests/{authorized_scan,unauthorized_paths,no_write_api}.rs`；`session_commands.rs` | — |
| AC-19 | already-done | `soul-policy/tests/hitl.rs`（未知拒、hash 变拒、令牌一次性）；`session_screens.rs` `hitl.deny` 落链 | — |
| AC-20 | already-done | `crates/soul-store/tests/research_preview.rs`（`zero_third_party_rows` 只收 0、目录字节不变） | — |
| AC-21 | already-done | `soulcore/tests/netwatch.rs`（/proc）；`install-smoke.ps1` 进程外 TCP 表；xtask e0-audit | — |
| AC-22 | already-done | 云开关恒「尚未启用」；依赖图无 E0（`deny.toml` + e0-audit） | — |
| AC-23 | already-done | `soul-policy/tests/audit_chain.rs`；产品链含 draft.create/egress.request/file.plan 与三路拒绝（`session_screens/e1/commands.rs`） | — |
| AC-24 | already-done | `audit_crash.rs`；`crash_recovery.rs`（COMMIT 中杀、重开链过、最多丢 1 条） | — |
| AC-25 | already-done | `soul-policy/tests/injection.rs`；`soul-draft/tests/injection.rs`；三路 `injection.blocked` 只留 `items` | — |
| AC-26（hosted 五门 @ HEAD） | minutes | 绿基线 `2e72ddf` run 32754617268；HEAD 空 run 证据 `docs/STATUS.md:13,828` | 等分钟 → 父代理 `workflow_dispatch` |
| AC-26（NSIS 签名安装包，D56） | author-manual | `docs/DECISIONS.md:67`（D56）；清单 §0；CI 不下 `tauri build` | 作者 |
| AC-28 | **code-now**（CODE-2 收严；实质已 done） | 已有：`t4d_band.rs:122-174`（Weak+四分列）、`people_summary.rs:305`（分列句）、`soul-algo-tie/tests/ablation.rs:261`（T4 反证）。缺：决胜边 `direct_active_day_count` 断言、具名夹具产品对拍（FORMAL:162） | opus |
| AC-29 | **code-now**（CODE-2 收严；实质已 done） | 已有：`t4d_product.rs:128`（12 次/6 天 Strong）。缺：import `lilei_12`（含三天前收尾）、`group_heavy_plus_three_directs`=Moderate 同测断言 | opus |
| AC-30 | **code-now**（CODE-2 收严；实质已 done） | 已有：`t4d_band.rs:213`（单库 as_of 降休眠不降活跃、as_of/silent_days 落库）、`t4d_product.rs:174`。缺：两条边 `as_of_utc` 全等断言 | opus |
| AC-31 | already-done | `soul-profile/tests/intake_replay.rs:72`（锁轴、skip 理由、replay 全等）；`correction_lock.rs:213` | — |
| AC-32 | already-done（组合覆盖） | `graph_correction.rs:149,198`（纠正过 rebuild、锁边继续计数）；`soulcore/tests/graph_correction_commands.rs:49,109`（落链、双档上屏）；`locked_tie_summary.rs:151,217`（锁边抑制 filed_band、无未冻结变体） | — |
| AC-33 | already-done（组合覆盖） | `people_summary.rs:305,329`（携带才渲染/缺席整句不出）；`t4d_band.rs:618` 源码守卫；`apps/desktop/src` 无阈值字面量（本轮 grep 0 命中）；schema `if(algorithm_id)` = `relationship.schema.json:117-125` | — |
| AC-34 | **code-now**（CODE-3；前半已 done） | 已有：`soul-import/tests/import_to_graph.rs:157`（0 条 Outgoing、每 peer 一条 Incoming）。缺：每条边 `last_contact_utc` 停在 peer 自己的 09:xx、不被 owner 的 10:00 刷新 | opus |

## 三、PRODUCT_LOCK 十三片切片（`docs/PRODUCT_LOCK.md:114-130`）

| # | 切片 | 类 | 证据路径 | Owner (R2) |
|---|---|---|---|---|
| 1 | 安装、托盘、向导默认全关 | author-manual（代码半边 done） | smoke + `shell_is_local_only`（11 项）+ `installer-hooks.nsh` Programs 目录；真机托盘/UAC/真装卸 = 清单 §1-4 | 作者 |
| 2 | 问卷 + 两种导入 | already-done | WP06 + WP09 §4（`/import` 一屏）；D37/D38 已闭（见 AC-05 行） | — |
| 3 | 可编辑档案 + 人脉图 v0 + 证据档 | already-done | WP03/WP05 + T4D 换血；`t4d_product.rs` 近阈值门（`:303-373`） | — |
| 4 | 纠正锁定 + 语气立即改变 | already-done | AC-07/AC-31/AC-32 行证据；WP09 §8（brief 进 prompt） | — |
| 5 | 记忆 CRUD + 遗忘 + 影响面预览 | already-done | `forget.rs`、`profile_memory_commands.rs`；遗忘拒绝落链（`2d2badd`） | — |
| 6 | 单人人事摘要（无 key 统计降级） | already-done（D40 已闭） | `a2_adapt.rs` 走冻结 `a2_render`；`session_e1.rs` 端点改写；`/graph` 渲染 `source` | — |
| 7 | 可选前台采集（关=0、撤销 1s） | author-manual（代码半边 done） | WP07 + WP09 §5（`/collect` 一屏）；真机二十秒窗口 = 清单 §6 | 作者 |
| 8 | 按语气起草不发送 + 第三人占位 | already-done | AC-11/12/13 行证据；确认屏无发送词断言 | — |
| 9 | 只读扫描 + 计划预览 + 未授权 100% 拒绝 | already-done | AC-18/19 行证据；`/files` 无执行按钮、无可绑命令（D31） | — |
| 10 | 研究只预览、第三人行 0、不落盘 | already-done | AC-20 行证据 | — |
| 11 | 云端「尚未启用」、零 E0 | author-manual（代码半边 done） | AC-21/22 行证据；WebView2 流量肉眼 = 清单 §5 | 作者 |
| 12 | 审计全覆盖、无正文 | already-done | AC-23 行证据（采集/导入/推断/纠正/记忆/遗忘/起草/E1/文件计划/拒绝十类都有落链测试） | — |
| 13 | CI + 安装 smoke | minutes | 同 AC-26（hosted）行；smoke 本体在 package job 与 `install_smoke_script.rs` | 等分钟 |

## 四、其余已知开放项（R3-SYNTHESIS / STATUS 下一步 / plan-polish 回归）

| ID | 类 | 证据路径 | Owner (R2) |
|---|---|---|---|
| GC-6/7（遗忘 vs 锁） | frozen-wont | `docs/DECISIONS.md:45`（D34） | — |
| GC-9b「由你本人指定」 | frozen-wont | `docs/DECISIONS.md:46`（D35）；唯一出现处是允许的反向断言 `locked_tie_summary.rs:42` | — |
| COPY_ZH §4 vs `a2.rs` 文案漂移 | frozen-wont | 不改冻结 crate；`.agent_workspace/unblock/R3-SYNTHESIS.md:29` | — |
| 合法 9999 年摆动 as_of | frozen-wont | G4/P1 已定价；D37 只挡不可表示年（`telegram.rs:316`） | — |
| v1 按发言人数判 Direct | frozen-wont | `docs/DECISIONS.md:47`（D36） | — |
| 双份 `parse_rfc3339` | frozen-wont（非 P0 重构，非 D54 门项） | `.agent_workspace/unblock/R3-SYNTHESIS.md:32` | — |
| AC-27 文件写执行 | frozen-wont（v0.1.1） | `docs/FORMAL_WORK_PROMPT.md:160`；`docs/STATUS.md:833` | — |
| 导入幂等 | frozen-wont | `docs/DECISIONS.md:66`（D55：P1，不进矩阵不进切片） | — |
| R-3（STATUS 的 D57 复述缺「同一棵树核于提交号一致」半句） | docs-now | `docs/DECISIONS.md:68` vs `docs/STATUS.md:837`；`REGRESSIONS.md:33-37` | 父代理直改（1 行）或 docs 槽 |
| R-4（FORMAL 夹具溯源句） | already-done | `docs/FORMAL_WORK_PROMPT.md:162`「AC-34 是例外」已在场 | — |
| STATUS 分支失真（unblock R3 §5） | already-done | `docs/STATUS.md:9` 已写 PR #7 为集成分支、7b1c 为历史 HEAD | — |
| Graph.tsx 第三套档位词 | already-done | `apps/desktop/src/routes/Graph.tsx:34-36`「弱/中等/强」（父代理补刀，R3-SYNTHESIS:17） | — |
| DPAPI 真机确认（keys.dpapi 存在、卸载不删） | author-manual | `docs/STATUS.md:830`；清单 §7 | 作者 |
| PR #7 合入 `main` | other-PR | 唯一合入路径（SHARED_BRIEF:5）；D54 两门齐过后由父代理/作者操作 | 父代理/作者 |
| PR #6（BLOCKERS.md）后合 + D32 撞号 | other-PR | `docs/STATUS.md:831`；`docs/DECISIONS.md:73` 已写撞号处置 | 父代理/作者（PR #7 之后） |
| PR #4 关闭 | other-PR | 同种子 M1-PR4 行 | 父代理/作者 |

## 五、Round 2 汇总

- **code-now（恰 3 项，全给 opus）**：CODE-1 legacy 边守卫（R1-LEGACY）；CODE-2 具名夹具产品对拍 + 三个缺失断言（AC-28/29/30 收严）；CODE-3 AC-34 的 `last_contact` 断言。细则见 REPORT.md 第三节。
- **docs-now（2 项，均 ≤10 行）**：追加 D61（主干接班，DOC-D49）；STATUS D57 复述补半句（R-3）。
- 其余全部 already-done / author-manual / minutes / frozen-wont / other-PR，Round 2 **不得**为它们生成工单。
