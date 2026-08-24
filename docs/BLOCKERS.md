# Soul 阻碍项

状态：`BLOCKERS_FROZEN`（Round 1 六路独立 + Round 2 六路交叉验证后由父代理冻结）。
日期：2026-08-24。父代理 run `bc-b296c4d9-0feb-4f6e-b844-aeaca7a8a073`。
权威：Goal 1 线上的 `docs/PRODUCT_LOCK.md`、`docs/FORMAL_WORK_PROMPT.md`；`main` 上的 `docs/algorithms/DECISION.md`（`ALGO_FROZEN`）。本分析分支从 `main` 长出，前两份要到 M2 之后才出现在同一棵树上。
过程稿：`.agent_workspace/blockers/`。进度仍以 Goal 1 线的 `docs/STATUS.md` 为准。

**一句话：** 产品定义和算法选型都已冻结。现在挡住项目的是：两条 Goal 1 实现线合不拢、`main` 与应用工作区四个 add/add、Windows 夹具让 CI 红、冻结的 T4D/A0 没接到产品、灵魂层纠正在产品面不成立。

Goal 2 在 Goal 1 关闭前不要启动。本文件合进 `main`（PR #6）之后，再按 M2 把最新 `main` merge 进 Goal 1——不要把集成规则只放在被集成分支上。

关闭 Goal 1 需要两套门同时过：`FORMAL_WORK_PROMPT` 验收矩阵 **和** `PRODUCT_LOCK` 的 v0.1 切片。矩阵没有的行不能拿来否决切片；切片没有的行不能拿来否决矩阵。§3 的项挡的是「诚实的产品关闭」，不一定挡「矩阵某一格」。

---

## 0. 不是阻碍（禁止当新工作单重开）

| 项 | 为什么不是阻碍 | 权威 |
|---|---|---|
| 产品定义 / `PLAN_FROZEN` | 已拍板 | `PRODUCT_LOCK.md` |
| 算法选型（T4D + A0） | `ALGO_FROZEN`；保留 1–2 个已完成 | `DECISION.md` |
| T0 / T1 / T2 / T3 / A3 | 墓碑 | `REJECTED.md` |
| 为 F04c 加第三道降档门 | 静默修补，禁止 | `DECISION.md` §4.2 / §5 |
| WP12 | 已删除 | `FORMAL_WORK_PROMPT.md` |
| 文件写执行 / 撤销 | v0.1.1 AC-27 | D31 |
| OAuth、云端 E0、Linux 桌面、微信/QQ 非官方抓取 | 锁外或 D32 | `PRODUCT_LOCK.md` |
| 单独合 PR #1 | 内容已是 Goal 1 祖先 | PR #2 历史 |
| 诊断词 / 量表分数 | 禁令，不是缺口 | D22 |
| 在 CI 里跑完整 `tauri build` / 下 NSIS | 出网循环；打包作者手动 | AC-26 措辞见 S5 |
| 钉 `OPENSSL_SRC_PERL` / 强制 NASM | 缺工具时编译自己会红 | Round 2 gpt-sol-b |
| 为「CLI 没跑过」加 `--no-bundle` 门禁 | 与现有 release 编重复 | 同上 |
| 合并两个算法 crate / 改冻结 crate 源码做 180 别名 | 合并义务可后置；产品 crate 禁止第三个 `180` | `DECISION.md` §0、M2 |

STATUS 里「阻塞：无」只表示「写代码的前置文档已齐」，**不是**可以合进 `main`，也不是 Goal 1 可以关闭。

DECISION.md §6.1 仍写 `crates/soul-algo` 与 `graph_build.rs`：真名是 `soul-algo-tie` / `soul-algo-trait` 与 `crates/soul-graph/src/build.rs`。实现跟真名。

---

## 1. 仓库拓扑

核对：2026-08-24 16:45 UTC，非浅克隆。`git rev-parse --is-shallow-repository` 必须先查。

| 线 | 尖端 | 与 `main` | 说明 |
|---|---|---|---|
| `main` | `7b35bde`（PR #5 squash） | — | 只有算法 crate + 过程稿 |
| `cursor/soul-goal1-7b1c` PR #2 | `3e88b48` | 祖先 `ea6f62f`；**CONFLICTING**；**draft** | **主干。** WP01–WP13 两段、问卷已合并、`/files` `/graph` 已接 |
| `agent/dev-sota` PR #4 | 仍在推（R2 时 `fc9836e`） | 祖先 `ea6f62f`；**CONFLICTING** | 在 `862e858`（WP09）与 Goal 1 **分叉**后各自重做 WP10/WP11。PR 正文「#2 已是祖先」为假。最新推送在重复 goal1 已有的 Draft/Files 视图。违反 `cursor/` 前缀 |
| PR #1 | `a785317` | MERGEABLE draft | Goal 1 祖先 |
| PR #3 | 现状盘点 | CONFLICTING | 关 |
| PR #5 | 已合 | — | 算法冻结 |
| PR #6 | 本文件 | 对 `main` 干净 | **先合进 `main`** |

`git merge-tree origin/main origin/cursor/soul-goal1-7b1c` 恰好四个 add/add：`.gitignore`、`Cargo.lock`、`Cargo.toml`、`rust-toolchain.toml`。

**主干裁决：`cursor/soul-goal1-7b1c`。** P0 做到「宣布唯一主干 + 停止/关闭 PR #4」。从 dev-sota 樱桃摘 e0 发送 crate 禁令 / SOTA 评论 / D32 是 **P1**，不挡 Goal 1 合入。禁止整树 merge/rebase，禁止采用它的 `ci.yml`（会在 Windows 真跑 `ipc_roundtrip`）。

---

## 2. 合入阻塞（不解决则应用进不了 `main`）

### M1 — 两条实现线（P0）

停 `agent/dev-sota`。关 PR #4。独有项事后审，不挡 #2。

### M2 — 把 `main` 并进 Goal 1（P0）

在 `cursor/soul-goal1-7b1c` 上 **merge 一次**，不要 rebase 96 个共享提交：

```text
git checkout cursor/soul-goal1-7b1c
git merge origin/main
```

把 merge 提交直接推进该分支，使 PR #2 变为 MERGEABLE；不要另开长期吸收分支却不写清如何成为 #2 的尖端。

| 文件 | 取谁 |
|---|---|
| `Cargo.toml` | Goal 1 成员表 **加上** 两个算法 crate |
| `.gitignore` | Goal 1 |
| `rust-toolchain.toml` | Goal 1（`1.83` + components） |
| `Cargo.lock` | Goal 1 的锁，再 `cargo update -w` |
| 工作区 `license` | 保持 Goal 1 的 `LicenseRef-Soul-Proprietary`；算法 crate 可 `license.workspace = true` |

算法 crate 的判档/问卷规则 **不改**。接口加宽（A2 计数 `u32`→`u64`、证据 ID 泛型）允许，因为那是冻结后的合并义务，不是新算法。`src/` 里再写一个 `180` 字面量禁止；两 crate 现有的 `DEMOTE_ONE_BAND_DAYS` 与 `DORMANT_AFTER_DAYS` 用工作区测试钉相等，不在本阶段合并 crate。

合 #2 进 `main` 前先 **mark ready**（现在是 draft）。**强烈偏好 merge commit**（#1 会自动标已合，历史可二分）。若被 squash：手动关 #1。这是 GitHub 记账，不是正确性门禁。

### M3 — Windows 夹具：decoy 写进已授权根（P0，AC-18 / AC-26）

不是 `\\?\C:\` 剥前缀（`9ba160d` 已修授权）。`Tree::build` 往 `base/alpha` 写 `decoy.txt`，NTFS 上即 `Alpha`。失败点：`authorized_scan.rs:65`、`:165`。产品行为正确。

`unauthorized_paths` 因 fail-fast **从未在 windows-latest 跑到**；探针还要改四处（约 `:50` `:53` `:237` `:247`）。语料长度断言 `>= 27` 在 Windows 上刚好饱和：探针拿掉 `lower_alpha` 两条后会变成 25 然后**红在守卫上**。守卫必须改成精确式：

```text
25 + (probe ? 2 : 0) + (cfg!(unix) ? 3 : 0)
```

**Bravo 无条件留在语料里。** `case_is_decided_by_the_filesystem_rather_than_assumed` 不要改、不要跟探针走。问文件系统，不要问 `cfg!(windows)`（APFS 会折、NTFS 可标大小写敏感）。不要为绿把守卫放宽成无条件 `>= 25`（Linux 会默默丢条目）。不要 `#[ignore]`。

`test-windows` 里**每一次** `cargo test`（工作区 + desktop shell）建议带 `--no-fail-fast`，与夹具 **同一次推送**即可。它是可观测性，不是第二道 P0 门；夹具修好后默认也会跑完。desktop 步现在因工作区步失败从未开始，`one_store` / `ipc_roundtrip --no-run` 在尖端上零次执行。

大小写敏感的 Windows 目录上，默认 `PathMatching::CaseFolded` 仍可能接受无人授权的 `alpha`：那是产品 P2，预期红，禁止改回 `cfg` 来藏。

Windows 测试在 DPAPI 落地前会在 `Session::open` 处继续红。不要把「每步全绿」读成「DPAPI 之前什么都不许做」。在夹具修复之后立刻加测试入口 `Session::open_with_keys(dir, provider)`，让 `soulcore` 在 Windows 上用显式 `TestKeyProvider` 跑完，发货路径的 `Session::open` 保持诚实。用签入的豁免名单也可以，但入口更便宜。

---

## 3. 产品关闭阻塞（锁 / 冻结义务 / 切片）

权威是 PRODUCT_LOCK 切片与 `DECISION.md` §6，不是「矩阵没这行所以不是 P0」。

### G1 — 人脉图仍是 T0（P0）

`Tally::band` 全场地 3/10/3，无 180/360，无全库单一 `as_of`。群扇出 + T0 会把 `group_heavy_plus_one_direct_each_way` 判 Strong；冻结答案 Weak。§6.5：现行 band 只是遗留。

接线：`soul_algo_tie::score`（或 `score_ego_network`）；peer **和** `conversation_id` 都映射到本次 rebuild 的稠密 `u64`；`as_of` = 全库 `max(occurred_at)`，在丢掉「联系人解不开」的行**之前**算，禁止 per-peer、禁止墙钟。持久化分列计数、任一场地 `last_contact`、`last_direct_contact`、`silent_days`、`as_of_utc`、`algorithm_id`。合并计数只由分列相加。

A2：`soul-draft::points_for` 已读生效 `band`，没有第二套 3/10/3。换 `a2_render` 是为了 `COPY_ZH.md` 的分列句与沉寂句，不是去重常量。适配器必须把分列填成 `Some`，否则 A2 静默丢掉冻结句。A2 的 `u32` 与 UUID 证据：优先在 Goal 1 适配（重建期映射）；需要时允许加宽冻结 crate 接口。产品 crate 禁止第三个 `180`。源码守卫继续扫 `soul-draft` 与 A2。

夹具（产品边界，不只算法 crate）：`lilei_12`；群洪 + 每向 1 条一对一 → Weak 且分列精确；仅群聊 Weak；2/3、9/10、日 2/3；179/180/359/360 闭区间；休眠 peer 用全库 `as_of`（per-peer 对照必须不同）。

已知代价禁止静默修补：仅群聊 Weak、F04c、真实群聊可挡住降档。

### G2 — intake 绕过轴锁（P0）

锁轴不 `place_axis`；答案仍落证据；`ignored` 理由 `axis_locked_by_user`；与 `apply_intake` replay 在映射 ID 后全等。保持 `LastWriteWins`。不要开 A1 / `NoDowngrade`。

### G3 — 人脉图不可纠正（P0）

`rebuild` 把 `user_verdict` 写成 `Unreviewed`。照 GRAPH_CORRECTION.md 的 GC-1…GC-8、GC-10。生效档 = 锁定时的用户档；机器档另存；A2/图只消费生效档。

**GC-9 话术不可照抄：** 冻结 `COPY_ZH.md` 没有「由你本人指定」变体。先经父代理 DECISIONS **加性**增补 COPY_ZH（新 P5 key），再改渲染。变体落地前，锁定边 **不得** 渲染现冻结 P5 原句（那是对锁定档撒谎）。

工序：先接线 T4D 机器档，再持久化生效档，再让 A2 读生效档。

### G4 — 导入观测质量（P1，与 T4D 同批更干净，但不挡合入）

矩阵与 PRODUCT_LOCK 都没有「重复导入必须幂等」。STATUS 已记录同一文件会写两遍。去重键需要身份/事务设计，不能当小修 P0。

群聊 N:1 扇出会伪造 `last_contact`、挡住降档。T4D 次数门已挡住「刷 Strong」。Telegram `result.json` 通常没有逐条成员表，「没有同场集合就零 Outgoing」可能过纠。**标 P1 / 已知限制**：T4D 接线后如实入档；有成员集合再收紧。不要在算法 crate 里加权重补偿。

去重（P1）：`sha256("soul.import.dedup.v1|source|canonical_external_id")`。JSONL 的 `id` 必须带上会话/账号命名空间。只存摘要。同文件两次导入不改变档位——作为设计目标，不是矩阵行。

### G5 — 壳只接了切片的一部分（P0 关闭，非合入）

AC-03 的「谁跑」是 CI，喂 fixture 合规。锁的是切片 2/3：无文件则问卷回退、可编辑档案。向导没有十一题，问卷命令未注册。`COMMAND_NAMES` 14 个；`profile` / `memory` / `import` / `collect` / `policy` 没接到壳。采集开关检查清单自己写着界面没有。

关闭 Goal 1 前至少：向导十一题（来自 `questions()`）+ `/profile`（`correct_axis`，G2 之后）。导入 / 记忆 / 采集 / 研究预览是同一欠债，按 WP09 视图补，不要另起 Goal 2。

---

## 4. 发货与诚实缺口

### S1 — DPAPI 骨架（P0 发货 / P1 合入）

矩阵 AC-04/AC-08 的「谁跑」是 CI，用 `TestKeyProvider` / `open_test_store`，**挡不住任何矩阵行**。这与 S5 一样是措辞洞：按字面 Goal 1 可以在「唯一发货平台库打不开」时合法关闭。关闭清单必须把「Windows 能打开自己的库」写成产品条件，或改矩阵。

实现放进新 crate **`soul-winkeys`**（仅 `cfg(windows)`，零依赖），**不要**把 `soul-store` 的 `forbid(unsafe_code)` 降成 `deny`。失败 → `Unavailable`，禁止新铸钥匙清空用户库。明文密钥文件不是发货路径。

### S2 — KnownIdentifiers 空（P1）

STATUS：「降低精度不是底线」（第三人 turn 整段占位 + shape scrub）。升 P0 的条件：一条能穿过现有占位、让 AC-12 红的夹具（owner 文本里嵌 2–3 字中文名）。图上解封姓名是另一件事，不要绑在清洗上。

### S3 — AC-21 观察器（P2）

`EgressFindings` 已有 `observed`；`headless` 没拿它做 `require`。`install-smoke.ps1` 已经在采 `Get-NetTCPConnection`。补一行 Rust `require` 即可，不挡合入。

### S5 — 打包（措辞，非代码门禁）

`package` job 已从编出来的 `soul.exe` 读 `asInvoker`。AC-01 不提权的一半在 CI。NSIS 仍作者手动。AC-26「打包绿」改为「package job 绿 + 作者签名安装包」，或接受 Goal 1 不能只靠 CI 关闭。不要在 CI 下 NSIS。

作者手动：托盘、UAC 观感、进程名、云开关系统层（含 WebView2 进程）、真机采集、WebView2/DPI。AC-09/AC-10 矩阵写的是 CI（假源）；真机是加分，不是矩阵行。

---

## 5. 推荐工序

全程 `cursor/` 前缀。不要开 Goal 2。

「每步再做下一步」对 **合入门禁**（M1–M3、M2）成立。对 Windows 全绿：**夹具 + `open_with_keys` 之后**，允许 DPAPI 豁免名单上的红，直到 S1。

| 序 | 项 | 门禁 |
|---|---|---|
| 0 | 本文件 `BLOCKERS_FROZEN` 合进 `main`（PR #6）；停 `agent/dev-sota` | 合入 |
| 1 | M3 夹具探针 + 精确语料守卫；可选同推 `--no-fail-fast`；`Session::open_with_keys` | 合入 |
| 2 | M2：`main` merge 进 `cursor/soul-goal1-7b1c`；PR #2 mark ready | 合入 |
| 3 | G1 T4D 机器档 + 边界夹具 | 关闭 |
| 4 | G2 A0 intake | 关闭 |
| 5 | G3 纠正（COPY_ZH 先加 P5 变体）+ A2 读生效档 | 关闭 |
| 6 | G5 向导 + `/profile`（及其余未接切片按 WP09） | 关闭 |
| 7 | S1 `soul-winkeys` | 发货 |
| 8 | G4 去重/扇出（P1）、S2 夹具（P1） | 不挡 |
| 9 | PR #2 merge commit 进 `main`；关 #3 #4 | 合入 |
| 10 | 作者签 `scripts/author-manual-checklist.md` | 关闭 |

---

## 6. 父代理直改说明

文档与过程稿。未改业务逻辑。算法规则未改。Round 2 改判见 `.agent_workspace/blockers/R2-SYNTHESIS.md`。
