# fable-b — BUILD R3 未关闭 DAG（READONLY on product code）

[Model: claude-fable-5-thinking]（请求 slug `claude-fable-5-thinking-xhigh`；同系列 thinking 在跑，未静默降级、未换系列）

- round: BUILD R3
- role: plan
- scope: remaining/unclosed DAG — 本树 HEAD `2bd56a2`（分支 `cursor/goal1-build-audit-c441`，基线 `cursor/soul-goal1-7b1c` @ `5309656`）之后仍未关闭的项
- 已读: STATUS「BUILD+AUDIT 第 2 轮」「下一步」与 AC 对照表、GOAL1_PLAN、FORMAL_WORK_PROMPT 验收矩阵（本树止于 AC-26，AC-27 是 v0.1.1）、R2-SYNTHESIS、round2/fable-closeout、`5309656..2bd56a2` 全量 32 提交、`origin/cursor/blockers-analysis-a073:docs/BLOCKERS.md`
- 方法: 收口稿核到 `e9cf29b`；其后只有 5 个提交（`1e20445` 收口稿、`ad69d07`/`2bd56a2` STATUS、`882b814` 向导、`99bdd30` store-api 测试基建），逐个 `--stat` 核过触碰面；两条跟进提交在 HEAD 代码里逐行核过并本机重跑钉住它们的测试

## 一句话

fable-closeout 留下的两件可选单**都已真闭合**（`882b814` 一单收掉向导 P1+P2 两句，`99bdd30` 按收口稿开的 ①②③ 方子收掉 FakeStore 视差，两处都有钉死测试且本机绿）；停车 P2 群的路径自 R2 裁决以来**零改动**，判定不变，无一升级为 P0/P1。**本树上剩余代码 P0/P1：零。** 未关闭的只有三件 blocked-on-user（N4/N5/N6）。建议本轮为存量 DAG 派 opus：**0 名**。

## done（不要重派）

| 项 | 证据 |
|---|---|
| WP01–WP13 全部 | GOAL1_PLAN 完成定义全勾；STATUS AC 对照表。**不重派。** |
| R2 已闭合 11 项（2 P0 + 5 P1 代码 + 5 文案/流程） | fable-closeout 在 `e9cf29b` 逐条核过，0 误报；其后无人动这些路径 |
| 收口余项 A：向导欢迎段「发出去的内容会先占位」（P1 文案，原 `Wizard.tsx:168`） | **`882b814` 闭合**。现写明并不全是占位符：档案摘要与图统计按原样、第三人正文默认占位、二次确认单次原文、姓名账号恒占位；`Wizard.test.tsx:148` 钉 `并不全是占位符` |
| 收口余项 B：向导「要用生成能力，得填地址」（原停车 P2 文案，`Wizard.tsx:76`） | **同一单 `882b814` 顺手闭合**。现说不填也能起草（本机确定性模板）并点名两个真正出网的按钮；`Wizard.test.tsx:170` 钉 `不填也能起草` |
| 收口余项 C：FakeStore `keys_of` 视差（P2 测试基建） | **`99bdd30` 闭合**，形状与收口稿方子逐条对上：① `fake.rs` `SealedBlob` 记 `row_id`；② `keys_of(Contact)` UNION 标签键与行锚键、`contacts_under` 读同两路；③ `conformance.rs:334` `forgetting_a_nameless_contact_destroys_the_bodies_under_their_key` 进 `run_conformance`（`:38`），fake 与 SQLCipher 两个跑器自动双向钉死。不碰产品文件、不碰 schema |

## open（按优先级；全部非代码）

| # | 项 | AC | 路径 | opus 可触 / 禁触 | 建议 opus |
|---|---|---|---|---|---|
| 1 | **N4** hosted 五门在 HEAD 真跑。GitHub Billing & plans 处理后对本分支 `workflow_dispatch`；不要 empty-commit；HEAD package 绿之前不要拿 `2e72ddf` 工件做卸载/`keys.dpapi` | AC-26（hosted 半） | run [32818145279](https://github.com/Xhhemoing/Soul/actions/runs/32818145279) 等空 runner；`.github/workflows/ci.yml` 无须改 | 无代码可写；禁触 workflow（已收窄，改它不解决账本） | **0**（blocked-on-user） |
| 2 | **N5** 作者 Win11 真机清单：托盘/UAC/进程名（AC-01 1–4）、WebView2 流量与资源监视器（AC-21/22 清单 5）、真机采集（清单 6）、HEAD `tauri build` 真装真卸、`keys.dpapi` 在且卸载不删、语气/审计/二次确认走查。gpt-sol-b P1-2（AC-21 自动化看的是 `soul-headless` 不是装出来的 `soul.exe`）按 R2 裁决归入此件，本 Linux 主干不落地 | AC-01、AC-21/22 肉眼半；清单 8/9/10 可选非门禁 | `scripts/author-manual-checklist.md`（结果回填「WP13 的 Windows 手动缺口」） | 无代码可写；禁触清单判据本身（`7cb9d24` 已同步过遗忘判据） | **0**（blocked-on-user） |
| 3 | **N6** 集成线裁决：T4D/A0 产品化归 PR #7 线，由用户/父代理裁 | — | PR #7 | 本主干**禁止**重实现 T4D/A0（D49；STATUS「不要合 #7」）；禁合 #7 | **0**（blocked-on-user/parent） |
| 4 | **停车 P2 群**（维持停车；自 R2 裁决后这些路径零提交，无新 repro，无一件升级）：fable-b P2-1 单 mutex+120s egress 超时挡撤回（`crates/soulcore/src/session.rs`）；P2-2 目录换符号链接 TOCTOU（`crates/soul-fileplan/src/scan.rs`）；P2-3 NTFS $UpCase 大小写折叠（本就要真机）；P2-4 首启键 blob 竞态（`crates/soul-store/src/keys.rs`）；gpt-sol-b P2-1 e0-audit 按客户端名枚举（`crates/xtask/src/egress.rs`）、P2-2 IPv6 origin 丢括号（fail-closed，方向安全）、P2-3 netwatch 采样窗 | — | 见各条 | 本轮任何 opus **禁触**上述文件，除非带 repro 把某条证成 P0/P1 | **0**（parked） |

不存在的代码 P0 没有被发明：我把 `e9cf29b..2bd56a2` 的触碰面逐个核过，没有新缺陷面；R3 的新缺陷发现权归 gpt-sol 探针，不由本稿预支。D55（重复导入去重索引）维持 v0.1 矩阵之外，诚实警告路线已落（`c8cebce`），不是 Goal 1 项。

## tests（本机 HEAD `2bd56a2` 树上亲跑，非转述）

- `cargo test -p soul-store-api`：`fake_conformance` 3（含新的无名联系人遗忘检查）、`sqlcipher_smoke` 2、单元 2+1 — 全绿
- `cargo test -p soul-store --test conformance_real`：4 绿（真库被同一条新检查钉住）
- `npx vitest run src/routes/Wizard.test.tsx`：16 绿（含两句新钉）
- **本机绿不是 hosted 绿**（AC-26 hosted 半仍空 runner = N4）

## p0/p1/p2

- **代码 P0：0。代码 P1：0。**（收口稿判定 + 两条跟进闭合 + 其后触碰面核查）
- **非代码 P0：N4、N5**（Goal 1 关闭门禁）；**N6** 是裁决不是缺陷
- **P2：停车群维持停车**，判定沿 R2/收口稿原样继承；FakeStore 视差已从群里销账

## assumptions

- 本树验收矩阵止于 AC-26；AC-27 与 PR #7 线的 AC-28+ 不作本主干事实
- 工作区里 `PROGRESS.md` 的未提交改动是父代理的派单稿，不由我提交
- 「深度优化并确定架构与计划」的架构半边归 fable-a（`ARCHITECTURE_LOCK.md`），本稿不重复

## do_not_touch

- 未动任何产品文件（本文件是唯一落笔）；未合 PR #4/#7/#10/#13；未在本主干重开/重实现 T4D、A0；未启 Goal 2；LOOP20 维持 QUEUED；无 empty-commit
- 若后续任何 opus 领单：向导两句只活在 `Wizard.tsx`（`882b814` 确认无 core 常量载体），**禁**把它们搬进 soulcore；停车 P2 的七个文件无 repro 不许碰

## next

1. 父代理向用户转三件：GitHub Billing → dispatch（N4）；Win11 清单（N5）；PR #7 裁决（N6）。三件任一未回，Goal 1 不能关、LOOP20 不动
2. 本轮存量 DAG 派 opus 0 名。若 gpt-sol-a/b 探针带 repro 报出新缺陷，按惯例数据/权限面（`crates/*`：soulcore/store/policy/fileplan）与 UI 面（`apps/desktop/src`）分人，禁止两人同文件
3. STATUS「BUILD+AUDIT 第 2 轮」第 6 条已如实记账 `882b814`/`99bdd30`，无需再写码；R3 综合稿由父代理落 `round3/`
