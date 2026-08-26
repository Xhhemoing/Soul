MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 1 / fable-b：SOTA 标准与产品切片验收（只读审计）

核于 2026-08-26。审计对象：

- 本树 = `main` 视角（当前分支 `cursor/project-status-audit-c49c` 与 `main` @ `a0ec14b` 内容零差异，`git diff main` 为空）。
- Goal 1 主干 `origin/cursor/soul-goal1-7b1c` @ `6d1058b`（其 STATUS 自称产品 HEAD `478f19f`，之后为文档提交）——**本树可 fetch，已核验**，不是「本树不可见」。
- 吸收线 `origin/cursor/goal1-unblock-a073`（PR #7）@ `6133307`。
- 对照权威：`docs/PRODUCT_LOCK.md`（13 条切片 + 砍/留 + 不可协商约束）、`docs/FORMAL_WORK_PROMPT.md`（验收矩阵 AC-01…AC-26、AC-28…AC-34）、`docs/algorithms/DECISION.md`（`ALGO_FROZEN`）、`origin/cursor/blockers-analysis-a073:docs/BLOCKERS.md`（`BLOCKERS_FROZEN`，PR #6，尚未在 `main` 树上）。

方法：全部结论来自本树源码/测试实读、`cargo test --workspace`（main 上 249 项全绿）、`cargo clippy`/`cargo fmt --check`（干净），以及对两条远程分支的 `git show`/`git grep` 逐文件核验。未改任何权威文档与 crate 逻辑，未写应用代码。

---

## 一、总判

1. **`main` 上 13 条切片全部不可证明为产品切片**——`main` 只有计划文档 + 两个冻结算法 crate，没有应用代码，甚至没有 `.github`（切片 13 的 CI 在 `main` 上零配置）。这与 `docs/STATUS.md`、FORMAL「计划面与代码面」表的自述一致，不是缺陷，是分工。
2. **Goal 1 主干对 13 条切片的声称绝大部分有代码与测试实证**，且其 STATUS 的自述纪律极好（本机绿 ≠ hosted 绿、crate 绿 ≠ 产品缝绿、CI 绿 ≠ 真机绿，三条缝都如实分开写）。但四个**关闭级缺口**仍开着：主干图谱判档仍是 T0（G1/G1+）、问卷 intake 绕轴锁（G2）、图边不可纠正锁定（G3）、算法 crate 未进主干工作区（M2）。四者的修复**全部已在 PR #7 吸收线上落地**，未回主干。
3. **算法冻结（`soul-algo-tie` + `soul-algo-trait`）达到「可被应用安全依赖」的 SOTA 水准**：DECISION.md 四项已知代价（F04c、群聊不判档、as_of、A1 空转）全部有钉死测试，且多数是性质级/穷举级而非样例级。残余差距见第四节，均为小项且多数已被 BLOCKERS 定价。
4. Goal 1 关闭的两套门（D54：矩阵 + 13 切片同时过）**当前都过不了**，且有两个非代码阻塞：hosted CI 因私有仓库 Billing & plans/spending limit 空 runner（AC-26），作者 Win11 手动清单 76 项未勾（AC-01 及全部「干净 Win11 上证明」前提）。

---

## 二、13 条垂直切片矩阵

「main 可证明」列统一为**否**（main 无应用），只在有算法口径交集处注明。「Goal 1 声称」以该分支 `docs/STATUS.md` 为准；「核验」是我对该分支树的实读结论。

| # | 切片 | main | Goal 1 声称 | 本次核验（分支树实读） | 缺口 |
|---|---|---|---|---|---|
| 1 | 安装、托盘、灵魂向导（权限默认全关） | 否 | 已做（WP09/WP13；单实例互斥量、托盘左键、NSIS 装 `%LOCALAPPDATA%\Programs\Soul`） | `apps/desktop/src-tauri` 全套在；`Wizard.tsx` + 测试在；`install-smoke.ps1`、`installer-hooks.nsh`、`shell_is_local_only` 钉托盘文案；`config_defaults.rs` 钉默认全关 | AC-01 本质是作者手动：真机 NSIS、托盘外观、UAC 观感全未做（清单 76 项未勾）；hosted 在 HEAD 空 runner |
| 2 | 问卷 + `soul-import-v1` JSONL / Telegram `result.json` | 否 | 已做（WP06 + WP09 第四段 `/import` 上屏） | `soul-import` 有 `soul_import_v1.rs`/`telegram.rs`/`questionnaire.rs` 测试；`session_import.rs`、`routes/Import.tsx` 在；问卷回退与 WP03 已并成一套 | 真机 UI 导入没人做过（作者清单第 8 节，标可选）；G4 重复导入不幂等（P1，BLOCKERS 已定价不挡合入） |
| 3 | 可编辑档案 + 人脉图 v0；推断带证据与证据档 | 否（但档位算法本身在 main 可证：T4D crate 级 13/13+） | 已做（WP03/WP05；`/profile`、`/graph` 上屏） | 档案/图谱/证据链在：`record_inference` 无 evidence_ids 不落库；边带证据。**但主干 `soul-graph/src/build.rs` 判档仍是 T0**：本地 3/10/3 常量 + 全场地计数 + 无 180/360 沉寂降档 + 无 as_of | **关闭级缺口 G1**：违反 PRODUCT_LOCK「灵魂层算法」节与约束 13；AC-08/AC-28/AC-30 在主干必红。FORMAL 明写「T0 图谱不能借 26 行全绿关闭 Goal 1」。T4D 接线（含 `t4d_adapt.rs`、全库单一 as_of、分列计数落边）已在 PR #7（`814064e`/`64e0b88`/`t4d_product.rs`），未回主干 |
| 4 | 用户纠正锁定，后续推断不覆盖；起草语气立即改变 | 否 | 部分做（`correct_axis` 锁 + 推断尊重锁 + 语气进 prompt 是 WP09 第八段） | `correct_axis` 锁定√；`record_axis_inference` 对锁定轴 `RefusedAxisLocked`√；语气「热络」实进 prompt（`session_e1.rs` 断言 mock 收到的字节）√。**但主干 `intake` 的 `place_axis` 不查 `axis_is_locked`**——再答问卷会移动锁定轴（service.rs `intake` 分支实读确认）。图边纠正完全不存在：`build.rs:322` 恒写 `user_verdict: Unreviewed` | **关闭级缺口 G2 + G3**：AC-31（intake 不绕锁 + 拒答如实回报）与 AC-32（边纠正锁定 + 机器档另存）在主干必红。两者修复均在 PR #7（`62840ff`、`graph_correction.rs`、GC-9 话术走 D48 加性增补），未回主干 |
| 5 | 自传记忆 CRUD + 遗忘（销毁 CK）+ 影响面预览 | 否 | 已做（WP02/WP04） | 证据充分：`soul-memory` CRUD/重开测试；`soul-store/tests/forget.rs` 预览三数字来自真查询、关库重开后 CK 销毁不可解密、推断 orphaned、审计仍在无正文；崩溃测试（fail point 真 abort）单事务回滚；遗忘话术如实补了 SSD 句（`928ef5a`，对齐 D15） | 真机/hosted 未验（同全局缺口）；无切片内代码缺口 |
| 6 | 单人人事摘要（无 key 统计降级，禁诊断词） | 否（A2 渲染器在 main 可证） | 已做（WP10 + WP09 第十段：`person_summary` 走端点改写，拒绝/超时/像诊断则计数原样留下） | `people_summary.rs`、AC-16/17 产品路径 `session_e1.rs` 在；降级与请求入链√ | 冻结 A2 渲染器未接（换 `a2_render` 与 COPY_ZH 分列句/沉寂句是 G1 同批义务）；AC-33 的 P1b 分列句在主干无从渲染（边上还没有分列计数）。在 PR #7（`7713a53`），未回主干 |
| 7 | 前台采集可选；关闭 1 秒内无新事件；未同意 0 事件 | 否 | 已做（WP07 + WP09 第五段 `/collect` 上屏；同意只活在进程，重启回关） | `soul-collect` 有 `consent_gate.rs`/`collection_lifecycle.rs`/`window_titles_are_not_collected.rs`/`the_tests_do_not_fake_collection.rs`；`session_collect.rs` 读回 `config.json` 字节反证不落盘 | CI 用假源（符合 ASSUMPTION 与 AC-09/10「谁跑=CI」）；真机采集属于作者清单第 6 节，未做 |
| 8 | 粘贴起草不发送；默认 E1 第三人正文占位 | 否 | 已做（WP10 + WP09 第六/七段：端点表单、AC-13「按原文带上」按钮） | `never_sends.rs`、`redactor_leakage.rs`/`redactor_exemption.rs`（豁免按值消费）、`session_e1.rs` 断言回环 mock 实收字节（两次请求体对照）√；壳 `KnownIdentifiers` 已从第三人显示名填上（STATUS 遗留 7 消除） | AC-12 的严格口径「嵌中文名夹具必须在 session 缝变红」是否已有专门夹具在 session 缝上断言，未见独立测试文件名指认（S2 在 BLOCKERS 判 P1，不挡合入但挡「AC-12 全字面达成」）；`soul-egress` 无 `Authorization` 头 → 无 key 输入框（分支已如实记为遗留） |
| 9 | 授权目录只读扫描 + 计划预览；不执行写；未授权 100% 拒绝 | 否 | 已做（WP11；`/files` 无执行按钮） | `soul-fileplan` 五个测试文件齐：`no_write_api.rs`（源码级无写 API）、`execution_is_refused.rs`、`unauthorized_paths.rs`、`file_names_are_data.rs`；拒绝与敌意文件名 `injection.blocked` 已入产品链 | M3：主干 `fc96e46` 的大小写夹具写法（`cfg(unix)` decoy、无条件 `>=25` 守卫）被 BLOCKERS **明判不采纳**——windows 绿是用不合规写法拿到的；合规「运行时文件系统探针」在 PR #7（`7aee812`/`c81c233`），未回主干。大小写敏感 Windows 目录上 `CaseFolded` 接受未授权 `alpha` 是已知产品 P2（预期红，禁止用 cfg 藏） |
| 10 | 研究导出只预览；第三人行数 0；不写文件 | 否 | 已做（WP02） | 证据充分且构造级：`zero_third_party_rows(counted)` 只接受 0 是唯一构造入口；`third_party_rows_excluded > 0` 反证确实查到了并排除；预览前后目录字节不变；另有测试把 `research_preview.rs` 源码读回断言无写文件 API；`/research` 空状态文案已改为只说它知道的事（`f0a2363`） | 无切片内代码缺口；真机未验（同全局） |
| 11 | 云端开关「尚未启用」；零 E0 | 否 | 已做（WP08/WP09；WebView2 自有流量关闭 `928ef5a`） | `CloudToggle` + 测试、`no_egress_path`、`xtask e0-audit`（对合成 `evil.example` 必红的自测）、`deny.toml` 仅 `soul-egress` 可包 reqwest；连点三次仍「尚未启用」已在 Linux 会话走过 | 真机 WebView2 抓包未做（作者清单第 5 节；分支 STATUS 已如实标注）；S3 残余：Windows 上 `/proc` 观察器 `Unsupported`（P2） |
| 12 | 审计覆盖采集/导入/推断/纠正/记忆/遗忘/起草/E1/文件计划/拒绝 | 否 | 已做（WP08 + WP09 第八/十段补齐起草/E1/文件计划三类落链 + `session_matrix_replay.rs` 累计链） | `session_matrix_replay.rs` 是真正的产品级证明：同一 `Session` 依次 13 个动作、一条链、末尾一次回放 `verified` + 逐条 `follows_previous` + 正文反搜（含「`forget.execute` 以 .exe 结尾会永真」这种细节都处理了）；此前「每条链都是同类第一条」的弱点被如实点名并补掉 | `capability.reject` 无产品路径可产生（v0.1 无 `execute` 命令面，注释如实说明——合规）；真机未验 |
| 13 | CI + 安装 smoke | **否（main 连 `.github` 都没有）** | 部分（`2e72ddf` 五门全绿 run 32754617268 含 windows-latest 与 package/asInvoker/install-smoke 15 项；HEAD `478f19f` 起 hosted 全部空 runner） | ci.yml 五门结构在（lint / test-linux / test-windows / sbom / package）；空 runner 归因已被三次复查钉死为私有仓库 Billing & plans/spending limit（同账户公开仓库同时段有真实 runner），不是 workflow 写坏 | **非代码阻塞**：作者处理账单前 AC-26 无法在 HEAD 上绿；NSIS 真机安装作者手动（D56）；那次绿 run 的 package 工件早于 NSIS 目录修复，不能拿来做卸载/keys.dpapi 手动验证（分支已自我标注） |

### 砍/留表与「明确不做」抽查

未发现越界实现：无发送路径（`never_sends.rs`）、无文件写执行（`no_write_api.rs` + HITL 测试断言无 known action 要写 scope + v0.1 不签发写令牌）、无 OAuth、无窗口标题采集（`window_titles_are_not_collected.rs`）、研究不写文件（源码断言）、无云 HTTP（e0-audit + deny.toml）、无诊断词（`assert_non_clinical` + denylist）。「留」的六项（记忆、人事分析、特质轴、人脉图、审计、遗忘）全部有实现与测试——其中人脉图与特质轴的**口径**不合规（G1/G2/G3），见上表。

### 不可协商约束逐条速查

1–12 条在 Goal 1 分支架构上均被尊重（抽查证据见矩阵）。**第 13 条在主干被违反**：`build.rs` 本地 3/10/3 常量即红线 11 禁止的第二套阈值字面量，判档是 T0。DECISION §6.5 给了过渡期语义（现行 band 只是遗留不是规范），所以这不是「静默另立口径」，但它是 Goal 1 关闭的硬门。约束 8（外部内容不是指令）有三路注入测试（导入行/粘贴/文件名）且文件名注入已入产品链。

---

## 三、算法冻结 SOTA 评估：`crates/soul-algo-tie` + `soul-algo-trait`

**结论：已达到「可被应用安全依赖」的 SOTA。** 依据（全部在 main 本树实测）：

- 工程基线：两 crate 均零依赖（`[dependencies]` 为空）、`#![forbid(unsafe_code)]`、`rust-version = 1.83` 钉死；`cargo test --workspace` 249 项全绿；clippy/fmt 干净。`as_of_unix` 是唯一时间输入——「不读墙钟」由类型签名保证，而不只是约定。
- 常量单点：`constants.rs` 是唯一数字来源，T4/T4D/oracle 都从这里读；常量值有回归测试钉死（含「Round 1 的 8 已消失」这种历史防回潮断言）。
- 话术绑定：`explain_zh.rs`（323 行）绑 COPY_ZH 模板；解释文案从计数现算而非从 band 反推（`zh_reason` 注释明说「存档 band 可与计数不一致，句子仍须可数」，且 `explain_zh` 对不一致情形有「按现在的规则应是 X，存档里是 Y」的如实分叉——这是超出验收要求的诚实设计）。
- 禁词：`denylist_scan.rs`（252 行）扫描用户可见输出。
- 墓碑：`tombstones.rs` 把 REJECTED.md 的落选者（T3R 折算、per-peer as_of、direct-only 时钟、milliscale）钉成会红的测试。

### DECISION.md 四项已知代价的覆盖核验

| 已知代价 | 覆盖 | 证据（main 本树） |
|---|---|---|
| **F04c 复燃一响**（§4.2，禁止第三道降档门） | ✓ 钉死 | `revived_after_gap`（私聊 30 次停 191 天 + 最近一来一回）在 `ablation.rs:84` 钉 `Truth::Exactly(Strong)`、`:125` 判定矩阵三算法同 S。谁加第三道门此行必红——「禁止静默修补」被翻译成了会红的测试，不只是文档句 |
| **群聊不判档**（§2.1/§4.1） | ✓✓ 性质级 | 决胜夹具 `group_heavy_plus_one_direct_each_way` 同测试内双断言 T4D=Weak **且** T4=Strong（漏洞实证共存，AC-28 的「改回任一场地必红」语义成立）；`group_only_50` Weak；自愈 `group_heavy_plus_three_directs`→Moderate；单向私聊 `group_heavy_plus_directs_one_way`→Weak；`the_boundaries_are_on_the_direct_count_not_the_total`（9→10 一对一边界）。`direct_gate.rs` 另证两条**性质**：T4D band 恒 ≤ T4 band（单调性），无群行时两规则逐字节相等 |
| **as_of 纪律**（§3） | ✓✓ 陷阱级 | `as_of_discipline.rs` 11 项：`a_peer_local_as_of_would_hide_every_dormant_tie` 把 per-peer 陷阱写成对照测试（dormant_2019 在 per-peer 下错判 Strong / 全库下正确 Weak）；`as_of_max` 全库缺省合法性 + 阈值边缘位移逐条入账（quiet_180/dormant_360 的 ±1 天位移被显式断言而非回避）；负 Unix 秒 floor 除法、UTC 自然日不随机器时区、epoch 前 `last_direct_contact` 仍如实报告 |
| **A1 空转**（§2.4「空转是预期行为」） | ✓✓✓ 穷举级 | `roundx_a1_empty_on_v01.rs`：把 v0.1 可写的行字母表（2 种来源 × 4 位置 × 3 日 = 24）穷举到 4 行日志全空间（24+576+13 824+331 776 条），断言升档分支**不可达**且 Strong 必来自纠正；另有 6 天 × 4^6 问卷组合；编译期 `const assert!(A1_EMPTY_ON_V01)` + 推导函数 + 行为穷举三者钉成同一主张，翻转任何一个必失败。这是「空转是预期而非缺陷」能得到的最强证明形态 |
| **§4.3 任一场地时钟不对称**（附带核验） | ✓ 差分级 | `dormant_direct_group_ping_yesterday` / `dormant_direct_no_ping` 恰差一条群消息（fixture 测试逐行断言两者只差那一条），band 差异被精确归因给任一场地时钟；`a_group_message_yesterday_stops_the_dormancy_step` 同时断言群消息没有加进判档证据（`band_before` 不变） |

### AC-28…AC-33 的夹具已备情况

FORMAL 点名的四个夹具（`group_heavy_plus_one_direct_each_way`、`lilei_12`、`group_heavy_plus_three_directs`、`dormant_2019`）全部在 `soul-algo-tie/src/testing`，A0 锁定用例在 `soul-algo-trait/tests/a0_lock.rs`——Goal 1 侧只需导入。产品边界版本（导入→加密库→rebuild→读边）已在 PR #7 的 `soul-graph/tests/{t4d_band,t4d_product,graph_correction,ego_graph}.rs` 成形；AC-34 的群聊导出样本亦在 PR #7（`90da257` owner 群消息归零归因）。主干上这些行**均不可证**。

---

## 四、SOTA 差距与不完善项

按影响排序。前四项是 Goal 1 关闭级（均非本轮新发现，BLOCKERS 已定价；本轮的贡献是核实「主干未修、PR #7 已修」的当前态）：

1. **G1/G1+（主干图谱 T0 + owner 群消息扇出）**——切片 3 的算法承诺整体不成立；伪造 `last_contact` 还会让 200 天无一对一的关系永不降档（污染降档时钟，比展示计数膨胀严重）。修复在 PR #7。
2. **G2（intake 绕轴锁）**——切片 4 的「后续推断不覆盖」只在 inference 路径成立，问卷重答路径不成立；AC-31 必红。修复在 PR #7（`62840ff`，含 `axis_locked_by_user` 拒答理由）。
3. **G3（图边不可纠正）**——AC-32 必红；且 GC-9 话术受 D48 约束（COPY_ZH 无「由你本人指定」变体前，锁定边不得渲染冻结 P5 原句）。修复与话术处理在 PR #7。
4. **M2（算法 crate 不在主干成员表）**——主干 `Cargo.toml` 16 个成员无 `soul-algo-*`；红线 12 的依赖箭头在主干尚未存在。PR #7 已加。

结构性风险（需要父代理裁决，不是子代理能修的）：

5. **两线合并路径未定**。主干 STATUS 明写「不要合 PR #7 / #10」，而 `main` 的 STATUS 与 BLOCKERS D50 把吸收线标为「待回主干」。两份权威没有当面矛盾（都承认合并是父代理按 BLOCKERS §5 工序做的事），但**没有任何一份文档写明 PR #7 如何回主干**（cherry-pick？merge？谁先谁后 M3/M2？）。这是当前唯一没有 owner 的关闭路径环节。
6. **AC-26 hosted 绿被账单挡死**（非代码）：私有仓库 Billing & plans/spending limit。作者动作。
7. **作者 Win11 手动清单 76 项未勾**（非代码）：13 条切片的总前提「在干净 Windows 11 x64 上证明」目前零项有真机证据。作者动作。

算法冻结侧的残余小项（不影响「可安全依赖」判定，如实入档）：

8. **180 的跨 crate 双字面量**：`soul-algo-tie::DEMOTE_ONE_BAND_DAYS` 与 `soul-algo-trait::DORMANT_AFTER_DAYS` 各自定义 180；两 crate 互不依赖，main 工作区没有钉两者相等的测试（各自 `assert_eq!(…, 180)` 只是间接相等）。DECISION §3 说沉寂句阈值「与降档共用同一常量，不得分裂」——现状是「两处字面量、分别钉数值」。BLOCKERS 已把「工作区测试钉相等」列入 M2 吸收义务、把「合并 crate」列为可后置，所以这是已定价缺口；但那条钉相等测试目前**哪条树上都还没有**。
9. **DECISION.md §4.3 夹具名漂移**：正文写 `direct_quiet_200_group_yesterday`（200 天版），crate 真名是 `dormant_direct_group_ping_yesterday`（300 天版）；200 天版只存在于 `.agent_workspace/round3/fable-a/t4d-verify/` 归档。判决方向一致，plan-polish round2 fable-b 已记录。唯一咬人场景：将来照抄 §4.3 名字写 AC 或探针会指向不存在的夹具——届时必须写 crate 真名（同 D53 精神，但 D53 的真名清单只点了 crate 名与 build.rs，没点这一条）。
10. **常量名漂移**：DECISION §3 的 `DORMANT_NOTE_DAYS` 在代码里叫 `DORMANT_AFTER_DAYS`。D53「实现跟真名」可兜底，但 D53 列举清单同样没含这一条。轻微，建议下次动 DECISION.md 时顺带补进 D53 清单（本轮禁改权威文档，未动）。
11. **A2 无阈值源码扫描按名字扫**：`roundx_a2_no_threshold.rs` 自己的文档注释已如实承认源码扫描「按 Goal 1 用过的名字拼写，换名字的门会走过去」——并用行为 sweep（跨 3/10 边界全组合断言「进什么 band 出什么 band」）补上了。这是自知且已补的弱点，列出仅为完整。
12. **AC-12 严格口径在 session 缝的专用夹具未见指认**（S2，P1）：分支 STATUS 说 `KnownIdentifiers` 已从第三人显示名填上，`session_e1.rs` 断言 mock 实收字节；但「嵌中文名夹具在 session 缝必须变红」这一 FORMAL 原话要求的**专用对抗夹具**没有可指认的测试名。升 P0 条件（BLOCKERS）尚未触发，如实记为差距。

### 与验收矩阵的对应（速查）

- 主干可信绿（crate/产品缝级，本机 + `2e72ddf` hosted）：AC-02…AC-07、AC-09…AC-25 的 CI 侧。
- 主干必红：AC-28、AC-29、AC-30、AC-31、AC-32、AC-33、AC-34（全部依赖 T4D/A0 接线与导入归因，均在 PR #7 才可证）。
- 无法由 CI 关闭：AC-01（作者手动）、AC-26 的 hosted 半边（账单）与 NSIS 半边（作者签名，D56）。

## 五、本轮未做（按指令）

未改 `docs/` 权威与两个 crate 的任何逻辑；未写应用代码；未合并、未触碰任何远程分支。产出仅本报告。
