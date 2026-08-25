MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 3 fable-b：HEAD vs 产品锁交叉审计（只读）

审计对象：`cursor/goal1-unblock-a073` @ **`016951d`**（`docs(unblock): Round 2 synthesis and R3 decisions`）。
依据：`docs/PRODUCT_LOCK.md`、`docs/FORMAL_WORK_PROMPT.md` AC 矩阵、`docs/DECISIONS.md` D32–D40、
`docs/algorithms/DECISION.md`（ALGO_FROZEN）、`docs/algorithms/COPY_ZH.md`、`R2-SYNTHESIS.md`。

**审计时效声明**：审计进行期间，R3 兄弟槽位正在同一 worktree 上未提交地改
`crates/soul-draft/**`（新增 `src/a2_adapt.rs`）、`crates/soul-import/**`、
`crates/soul-graph/tests/t4d_product.rs`、`crates/soul-profile/tests/intake_replay.rs`（新文件）、
`.github/workflows/ci.yml`。本报告一律以 `git show HEAD:` 取证；工作区在读取过程中实际变过
（`soul-draft/src/analysis.rs` 两次读取内容不同）。未提交的改动**不计入**任何「已闭合」判定。
在场改动的所有权抽查见 §6。

状态记号：**met** = HEAD 在树有代码+测试；**残差** = R3 槽位应闭合的代码缺口；
**手动** = 作者 Win11 真机清单（`scripts/author-manual-checklist.md`）；
**卡分钟** = hosted Actions 空 runner，代码改不动。
全局事实先钉一条：**HEAD（含本分支任何提交）从未有过 hosted run** —— `ci.yml` 的 `push`
触发只列 `main` 与 `cursor/soul-goal1-7b1c`（HEAD `ci.yml` 第 20–22 行），本分支只能
`workflow_dispatch`；而账户分钟仍然耗尽（§3）。所以下表所有「met」都读作
「在树证据 + `2e72ddf` hosted 绿覆盖当时存在的那一半；HEAD 增量 hosted 未证」。

## 1. 验收矩阵对 HEAD

### 1.1 AC-01 … AC-26

| AC | HEAD 证据（路径 + 测试名） | 判 |
|---|---|---|
| AC-01 | `scripts/install-smoke.ps1`；`crates/soulcore/tests/install_smoke_script.rs`；`apps/desktop/src-tauri/tests/shell_is_local_only.rs`（托盘文案、`asInvoker`、`$INSTDIR=%LOCALAPPDATA%\Programs\Soul`） | met（CI 半）+ **手动**（托盘肉眼/UAC/真装真卸，清单 §1–4）+ 卡分钟（HEAD package） |
| AC-02 | `crates/soulcore/tests/session_collect.rs::a_restart_reopens_a_closed_collection`（快照全关 + `config.json` 字节无 `collect`/`consent`）；`ipc_roundtrip::a_restart_finds_collection_off_again`；`config_defaults.rs` | met |
| AC-03 | `crates/soul-profile/tests/questionnaire_intake.rs`、`one_questionnaire.rs`；`apps/desktop/src/routes/Wizard.test.tsx` | met（字面）+ **残差 D39**（回执诚实性，§2.2） |
| AC-04 | `crates/soul-store/tests/no_plaintext_at_rest.rs`；`crates/soulcore/tests/session_import.rs`（过真库、重开仍在、数据目录搜不到原文） | met |
| AC-05 | `crates/soul-import/tests/telegram.rs::a_message_without_a_unix_timestamp_is_refused_rather_than_guessed_at` 等缺字段可读失败 | met（字面）+ **残差 D37/D38**（时间毒化，§2.3） |
| AC-06 | `crates/soul-graph/tests/ego_graph.rs`；`graph_correction.rs`（R3 证据并集 `9b12268`：rebuild 之后仍 cite 定档行）；`soulcore/tests/import_and_graph_commands.rs` | met |
| AC-07 | `crates/soul-profile/tests/correction_lock.rs`；`soulcore/tests/session_screens.rs`（`set_voice` → 起草文字变）；`session_e1.rs`（mock 收到的字节含「热络」） | met + **残差**（产品 `intake` vs 冻结 `apply_intake` 对拍缺席，§2.5） |
| AC-08 | `ego_graph.rs` ≥3 节点、边带证据 | met |
| AC-09 / AC-10 | `crates/soul-collect/tests/consent_gate.rs`、`collection_lifecycle.rs`；`session_collect.rs`（关=0 且采样数 0；开≥1；撤销 1s 不变） | met（CI 半）+ **手动**（真机前台切换，清单 §6） |
| AC-11 | `crates/soul-egress/tests/e1_origin.rs`（精确 origin、跨 origin 302 不跟）；`session_e1.rs`（填写不访问、批准才到 mock） | met |
| AC-12 | `crates/soul-policy/tests/redactor_leakage.rs`；`crates/soul-draft/tests/wire.rs`（对服务器实收字节断言） | met |
| AC-13 | `redactor_exemption.rs`（按值消费）；`session_e1.rs`（两次请求体：第一次原文、第二次占位，姓名两次占位） | met |
| AC-14 / AC-15 | `crates/soul-store/tests/forget.rs`（关库重开后 CK 消失、推断 orphaned）、`crash_recovery.rs`；`soulcore/tests/profile_memory_commands.rs` | met |
| AC-16 | `crates/soul-draft/tests/people_summary.rs`（每条 cite、无诊断词）；`session_e1.rs`（端点改写 `source=="user_endpoint"`、诊断词改写回落计数） | met（字面）+ **残差 D40**（自制话术非冻结 A2，§2.1） |
| AC-17 | `soulcore/tests/draft_commands.rs`、`netwatch.rs`（无 key 统计/模板 + 无非回环） | met |
| AC-18 / AC-19 | `crates/soul-fileplan/tests/{authorized_scan,unauthorized_paths,no_write_api,execution_is_refused}.rs`；`soul-policy/tests/hitl.rs`（未知拒、hash 变拒、令牌一次性）；`session_commands.rs` | met |
| AC-20 | `crates/soul-store/tests/research_preview.rs`（`zero_third_party_rows` 只收 0；目录字节不变） | met |
| AC-21 | `soulcore/tests/netwatch.rs`（Linux `/proc`）；`install-smoke.ps1` 进程外 TCP 表；`xtask` e0-audit | met（CI 半）+ **手动**（WebView2 流量肉眼，清单 §5） |
| AC-22 | 云开关恒「尚未启用」+ 依赖图无 E0 | met + **手动**（资源监视器一眼，清单 §5） |
| AC-23 / AC-24 | `soul-policy/tests/{audit_chain,audit_crash}.rs`；产品链含起草/E1/文件计划及其拒绝（`session_screens.rs` `draft.create`+`hitl.deny`、`session_e1.rs` `egress.request`、`session_commands.rs` `file.plan`） | met |
| AC-25 | `soul-policy/tests/injection.rs`；`soul-draft/tests/injection.rs`；三路 `injection.blocked` 只留 `items`（`session_commands.rs` / `session_e1.rs` / `session_import.rs`；`ipc_roundtrip` 过真 handler） | met |
| AC-26 | 最后一次 hosted 绿：`2e72ddf` run [32754617268](https://github.com/Xhhemoing/Soul/actions/runs/32754617268)。此后全部空 run，最新 [32799578919](https://github.com/Xhhemoing/Soul/actions/runs/32799578919)（2026‑08‑25 01:58 UTC，`3161e02` on `7b1c`）：5 job、0 step、`runner: null`、~3 秒 | **卡分钟** + 残差（windows `cargo test` 无 `--no-fail-fast`，§2.4）+ 分支触发问题（§5） |

### 1.2 十三个 v0.1 切片

| # | 切片 | 判 |
|---|---|---|
| 1 | 安装、托盘、向导默认全关 | met（smoke + `shell_is_local_only` 11 项）；托盘出现/不提权/NSIS 真装卸 = **手动** |
| 2 | 问卷 + 两种导入 | met；**残差 D37/D38**（导入侧时间校验，§2.3） |
| 3 | 可编辑档案 + 人脉图 v0 + 证据档 | met（T4D 已换血 `814064e`，单一 as_of）；**残差**：store 侧近阈值计数门测试（§2.4 修正后只剩 2/3、9/10、日 2/3） |
| 4 | 纠正锁定 + 语气立即改变 | met（AC-07 两级证据）；**残差 D39 + intake 对拍**（§2.2、§2.5） |
| 5 | 记忆 CRUD + 遗忘 + 影响面预览 | met |
| 6 | 单人人事摘要（无 key 统计降级） | 功能 met；**残差 D40**：话术整套是自制的，不是冻结 A2（§2.1）——这是 R3 最大的一块产品锁失真 |
| 7 | 可选前台采集 | met（CI 半）；真机 = **手动** |
| 8 | 按档案语气起草不发送 + 第三人占位 | met |
| 9 | 只读扫描 + 计划预览 + 未授权 100% 拒绝 | met（D31 保留；无执行按钮、无可绑命令） |
| 10 | 研究只预览、第三人行 0、不落盘 | met |
| 11 | 云端「尚未启用」、零 E0 | met；WebView2 肉眼 = **手动** |
| 12 | 审计矩阵动作全覆盖、无正文 | met（含拒绝与注入三路落链） |
| 13 | CI + 安装 smoke | 在树 met；HEAD hosted = **卡分钟**；`--no-fail-fast` = 残差 |

## 2. R3 槽位应闭合的代码缺口（HEAD 实证）

### 2.1 A2 自制话术（opus-a；D40 / ALGO_FROZEN §6.4）

HEAD `crates/soul-draft`（末次提交 `733af60`）**零依赖** `soul-algo-trait`
（`git show HEAD:crates/soul-draft/Cargo.toml` 无该行），`a2_render` 调用数为 0。取而代之：

- `src/analysis.rs:246 points_for` 自制五句；`:415 band_word` 自制档位词
  「往来不多/往来中等/往来密集」——COPY_ZH §6.5 只准「强/中等/弱」三词；
- `:323 direction_statement` 自制方向句（2:1 阈值碰巧同 COPY_ZH P2，措辞不同）；
- `:338 venue_point`、`:373 recency_point` 自制场合/近因句；
- **无 P1b 分列句**（`personnel.venue.direct_and_group_counts`）、**无休眠句**
  （`personnel.recency.dormant`）。冻结面九个 statement key（`a2.rs:72–80`）产品一个都没走。

供数侧**已就绪**：`soul-graph/src/model.rs:77–106` 的 `TieStrength` 持久化了
`direct_out/in_count`、`group_out/in_count`、`direct_active_day_count`、
`last_direct_contact_utc`、`silent_days`、`as_of_utc`、`algorithm_id`，
且 `t4d_band.rs::split_counts_round_trip_through_the_store` 钉过往返。缺的只是渲染端。

产品路径实达屏幕：`Session::person_summary`（`soulcore/src/commands/session.rs:648`）→
`commands/draft.rs:556 summarize_person` → `soul_draft::analysis::summarize_person`。
自制词就是用户看到的词。

**必须保住的既有语义**：HEAD 在锁定边上已丢归档句（`analysis.rs:281
!strength.is_locked_by_user()`，`tests/locked_tie_summary.rs` 钉死）——接 `a2_render`
后仍须丢 `personnel.tie.filed_band`（GC-9a）。验收检查点（R2 约束）：
`direct/group_count` 仅当 `as_of_utc.is_some()` 才 `Some`（遗留零 ≠ 实测零）；
UUID 证据 id 稠密 intern 成 u64 再映回；`soul-draft` 不得出现 180 字面量
（HEAD 已无，grep 证实）；**不改** `a2.rs` 模板。

### 2.2 `IntakeReceipt.ignored`（opus-a；D39）

HEAD `crates/soulcore/src/commands/profile.rs:251–262 IntakeReceipt` 无 `ignored` 字段；
`:275 answered: outcome.evidence_ids.len()`。而上游 `soul-profile/src/service.rs`
的 `IntakeOutcome` **已有** `ignored: Vec<IgnoredAnswer>`，且被锁轴的拒答**照样落证据行**
（service.rs 注释原话：「what the lock stops is the axis moving, not the evidence table
growing」）。所以对着一条锁轴重答，回执把拒答计进「已答」——soulcore 这一层把上游已经
算好的诚实数字丢掉了。TS 面同病：`apps/desktop/src/core.ts:329 IntakeReceipt` 无
`ignored`；`test/fakeCore.ts:363 anIntakeReceipt` 无 `ignored: []` 默认；
`Wizard.tsx:249` 那句「记下了 {answered} 条」在有拒答时口径失真。
D39 口径：`ignored: Vec<{question_id, reason}>`，reason 恒 `axis_locked_by_user`；
`answered = evidence_ids.len() − ignored.len()`；serde 加法；链上不加新审计枚举。

### 2.3 导入时间毒化（opus-b；D37/D38，R2 P0）

**D37（Telegram）**：HEAD `crates/soul-import/src/telegram.rs:272–279 to_instant`
对 `date_unixtime` 任意 i64 直接 `Timestamp::new(rfc3339_utc(seconds))`，零界检。
毒化链闭合实证：`soul-policy/src/clock.rs:32 rfc3339_utc` 用 `{year:04}` 格式化——
年 >9999 出五位、年 <0 带负号 → `soul-graph/src/t4d_adapt.rs:92 parse_rfc3339`
要求第 4 字节是 `-` 且四位年 → `None` → `build.rs:243
GraphError::UnreadableInteraction` 上的 `?` 掐死**整个** rebuild；fail-hard 行为
本身已被 `t4d_band.rs::a_broken_timestamp_fails_the_rebuild_and_names_the_row` 钉住，
而导入非事务，毒行落库后每次 rebuild 永久失败。修法（D37）：年不在 1970–9999 记
defect、拒该消息，在 `soul-import` 侧，不动冻结契约。

**D38（v1 民事日）**：HEAD `src/instant.rs:13 is_civil_datetime` 的日检是
`(1..=31).contains(&day)`——`2026-02-31` 放行，`is_date_time` 连带放行带 zone 形式；
`soul_import_v1.rs:282 check_instant` 用的就是它。schema 层开着
`should_validate_formats(true)`（`soul-schema/src/validate.rs:213`），jsonschema 的
date-time 检查**可能**兜底，但 D38 的要求正是本地校验独立成立（「关掉 format 校验时
仍拒」）；且一旦漏进，`parse_rfc3339` 的 `days_in_month`（`t4d_adapt.rs:112`）会把
Feb‑31 判 `None` → 同一条 rebuild 永久失败链。需按月天数（含闰年）校验 + 负例。

### 2.4 store 侧近阈值门测试（gpt-sol-a）——**R2 差距表需修正**

HEAD `crates/soul-graph/tests/t4d_product.rs`（190 行）只有四门：
`twelve_reciprocal_direct_exchanges_over_six_days_are_strong`、
`one_direct_exchange_each_way_does_not_let_group_volume_create_strong`、
`group_only_traffic_is_weak_at_any_volume`、`dormant_peer_is_scored_against_the_store_wide_as_of`。

但 R2 列的「179/359 不降」**在 HEAD 已被覆盖**：
`t4d_band.rs::demotion_edges_are_closed_at_the_store` 精确断言
(Strong,179) / (Moderate,180) / (Moderate,359) / (Weak,360) 过真库往返。
**真缺的只剩计数门**：2/3（Moderate 次数）、9/10（Strong 次数）、2 日/3 日
（Strong 自然日）在 store 侧无对应（算法侧存在：
`soul-algo-tie/tests/direct_gate.rs` 的「nine private exchanges … one short」）。
gpt-sol-a 不必重复 179/359，除非有意在 `t4d_product.rs` 双钉。

### 2.5 `intake` vs `apply_intake` 对拍（gpt-sol-b）

冻结侧自洽已有：`soul-algo-trait/tests/a0_lock.rs::intake_matches_replay_on_every_fixture_and_mode`
——但那是冻结 crate 自己对自己。HEAD `crates/soul-profile/tests/`（git ls-tree：五个文件，
无 replay）没有任何「产品 `soul_profile::intake` 过真店后映射到
`soul_algo_trait::a0::apply_intake` 输入、两侧轴态全等」的测试。锁轴、拒答、重答三种
路径的产品/规范一致性目前只靠人读代码。dev-dep 方向（soul-profile →dev→ soul-algo-trait）合法。

### 2.6 CI 可观测（gpt-sol-b）

HEAD `ci.yml:162–163` windows `cargo test --workspace --all-targets` 无
`--no-fail-fast`——一个 crate 红就看不到其余。R2 授权只加该 flag、不改触发分支。

## 3. 明确不是代码任务的（R3 禁止有人去「修」）

1. **作者手动七条**（清单 §1–7）：真装真卸、托盘图标肉眼、不提权、进程名、
   WebView2/云开关流量、真机采集、环境项。§8–10（界面导入/本机端点/语气+审计）可选，
   非门禁。STATUS 原话「CI 不能替，也不要在本文件假装过了」。
2. **hosted Actions minutes**：账户级耗尽。`2e72ddf` 之后每一次 run 都是 5 job/0 step/
   `runner: null`（本审计实查最新 32799578919，2026‑08‑25 01:58 UTC 仍空）；
   `agent/dev-sota` 同形态，非本仓库 workflow 之过。恢复后 `workflow_dispatch`，
   **不要 empty-commit**。
3. **GC-9b**（D35）：「由你本人指定」在 COPY_ZH 冻结该 key 之前禁入产品源码。
   HEAD 唯一出现处是允许的反向断言：`soul-draft/tests/locked_tie_summary.rs:42
   UNFROZEN_VARIANT`。合规，勿动。
4. **GC-6/GC-7**（D34，R4/R5）：遗忘压锁、无观测锁定边 Option 时间戳——Goal 1 不实现，
   STATUS 遗留即可，不发明墓碑。
5. **D36**：v1 按发言人数判 Direct/Group 维持现状；单活跃发言人群可铸 Direct 是钉死的
   已知面，不改冻结契约。
6. **合法未来时间戳摆 as_of**（R2 风险 4）：合法 RFC3339 的 9999 年可把全库降 Weak——
   G4/P1 定价遗留，D37 只挡不可解析年，本轮不修。
7. **Goal 2**：Goal 1 关闭前禁止启动（D28）。

## 4. 第二套 3/10/3 扫描（soul-graph、soul-draft；只报不修）

**结论：产品 crate 源码干净。**

- `soul-graph/src`：判档全部走 `soul_algo_tie`（`build.rs:64,291` `tie_reading`），
  单一全库 `as_of`（`build.rs:214–252`）。grep `>=3|>=10|180|360|*_MIN|FORCE_WEAK|DEMOTE`
  仅命中：`t4d_adapt.rs:180–184` 历法换算（400 年纪元，非阈值）与
  `correct.rs:45 CORRECTION_STRENGTH = Strong`（纠正证据档常量，非判档门）。
  且已有源码级守卫测试：`t4d_band.rs::the_graph_source_holds_no_second_threshold`
  对 `build.rs/model.rs/view.rs` 扫 10 个禁串——注意它**不扫** `correct.rs`，也**不扫
  soul-draft**，守卫面有边界。
- `soul-draft/src`（HEAD）：grep 同串零命中；无 180、无计数比较。自制的是**词**
  （`band_word` 三个档位词、方向句 2:1）不是**阈值**——归 §2.1 的 D40 问题，不是第二套 3/10/3。
- 顺带一处词面漂移（非阈值、非本轮任务）：`apps/desktop/src/routes/Graph.tsx:32–36`
  UI 侧 `BAND` 词表「观察到的往来较少/中等/较多」，与 COPY_ZH「弱/中等/强」及 soul-draft
  自制词构成第三套档位措辞。opus-a 接 `a2_render` 后，建议 STATUS 记一条 UI 词表与
  COPY_ZH 对齐的遗留，勿在 R3 顺手改。

## 5. STATUS 分支失真 + 一个新发现的分叉（父代理 R3 后处理；本报告不改 STATUS）

- `docs/STATUS.md` 第 7 行仍写「Goal 1 已开工：分支 `cursor/soul-goal1-7b1c`」；
  门禁对照表锚在 `2e72ddf`。而 T4D 换血、G3 保全、R3 全部工作都在
  `cursor/goal1-unblock-a073`。
- `ci.yml` push 触发只列 `main` + `cursor/soul-goal1-7b1c` ⇒ **本分支即使分钟恢复也不会
  自动跑**，只有 `workflow_dispatch`。retarget 时父代理要么把分支名换进 workflow，
  要么把本分支并回 7b1c/main——注意 gpt-sol-b 的 R2 授权**不含**改触发分支。
- **分叉实测**：`origin/cursor/soul-goal1-7b1c` 头是 `3161e02`
  （2026‑08‑25 01:57 UTC 推送：`ipc_roundtrip` +411 行、新 `session_crash.rs` 392 行、
  `session_collect/e1/import/screens` 增补、`ci.yml` +22 行），**不在本分支**；
  merge-base `80c9011`，本分支领先 26 commits。retarget 是一次**合并**，不是改个名——
  两边都动了 `.github/workflows/ci.yml` 与 `soulcore/tests/session_*.rs`，冲突可预期。

## 6. 在场未提交改动的所有权抽查（截至本审计写盘时刻）

| 文件 | 推定槽位 | 越界？ |
|---|---|---|
| `soul-draft/{Cargo.toml,src/analysis.rs,src/lib.rs,src/a2_adapt.rs,tests/people_summary.rs}` | opus-a | 否（A2 所有权内） |
| `soul-import/{src/instant.rs,src/telegram.rs,tests/*}` | opus-b | 否 |
| `soul-graph/tests/t4d_product.rs` | gpt-sol-a | 否（只加测试） |
| `soul-profile/{Cargo.toml,tests/intake_replay.rs}` | gpt-sol-b | 否 |
| `soulcore/{src/commands/profile.rs,tests/profile_memory_commands.rs}`、`apps/desktop/src/{core.ts,test/fakeCore.ts}`、`apps/desktop/src-tauri/Cargo.lock` | opus-a | 否（R2 明列这几个文件归 IntakeReceipt 槽位）——写盘后落地：profile.rs +36、测试 +76、`core.ts` +16（`ignored` 字段已入 TS 接口） |
| `.github/workflows/ci.yml` | gpt-sol-b | 否——diff 只加 `--no-fail-fast` 三处，未动触发分支；其中加在 `--no-run` 那条上是无操作，无害，父代理可留可删 |
| `Cargo.lock` | opus-a 连带 | 否——+2 行加法（两处 `soul-algo-trait` 依赖行），不是 regenerate |
| `.agent_workspace/unblock/round1/fable-b-G3.md` | fable-a | 否（R2 明授「修订冲突段」） |
| **`crates/xtask/src/denylist.rs`** | 推定 opus-a | **技术性越界，需父代理明示追认**。diff 只把 `crates/soul-draft/src/a2_adapt.rs` 加进 `EXEMPT_FILES`（逐文件豁免，照抄 `t4d_adapt.rs` 先例）——是 D40 任务的必然连带（调 `a2_render(TieScore)` 就得写出被禁词），soul-draft 其余源码仍受扫。但 `crates/xtask/**` 不在任何 R3 槽位的声明所有权里，且这是在改守卫面本身：收 R3 时单独过目这 6 行 |

五个缺口（§2.1–2.6）截至本次更新**全部**已见在场未提交改动；父代理收 R3 时逐项对本报告
的 HEAD 基线复核，尤其 §2.4 的修正（179/359 勿重复）与 §2.1 的三个验收检查点。
