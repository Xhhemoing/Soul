# Soul 阻碍项

状态：`BLOCKERS_DRAFT`（Round 1 六路独立审查已齐，待 Round 2 交叉验证后改 `BLOCKERS_FROZEN`）。
日期：2026-08-24。父代理 run `bc-b296c4d9-0feb-4f6e-b844-aeaca7a8a073`。
权威：`docs/PRODUCT_LOCK.md`、`docs/FORMAL_WORK_PROMPT.md`、`docs/algorithms/DECISION.md`（`ALGO_FROZEN`）。
过程稿：`.agent_workspace/blockers/`。本文件是阻碍项的单一事实来源；进度仍以 Goal 1 线的 `docs/STATUS.md` 为准。

**一句话：** 产品定义和算法选型都已冻结。现在挡住项目的是三条实现线合不拢、Windows CI 红、冻结算法没接到产品、灵魂层「可查看和纠正」在产品面不成立。

Goal 2 在 Goal 1 关闭前不要启动。

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

STATUS 里「阻塞：无」只表示「写代码的前置文档已齐」，**不是**仓库可以合进 `main`、也不是 Goal 1 可以关闭。

---

## 1. 仓库拓扑（先读这一节）

核对时间 2026-08-24 16:35 UTC。本工作区曾是浅克隆，把 `cursor/soul-goal1-7b1c` 误报成「与 main 无关历史」——`git fetch --unshallow` 之后：

| 线 | 尖端（当时） | 与 `main` | 说明 |
|---|---|---|---|
| `main` | `7b35bde`（PR #5 算法冻结） | — | 只有 `soul-algo-tie` / `soul-algo-trait` + 过程稿，几乎无应用 |
| `cursor/soul-goal1-7b1c` PR #2 | `3e88b48` | 共同祖先 `ea6f62f`；**CONFLICTING** | Goal 1 主干候选。WP01–WP13 两段都在。问卷已并成十一题。`/files` `/graph` 已接 |
| `agent/dev-sota` PR #4 | 仍在推 | 共同祖先 `ea6f62f`；**CONFLICTING** | 在 `862e858`（WP09 壳）与 Goal 1 **分叉**，随后**各自重做了 WP10/WP11**。PR 正文写「#1/#2 已是祖先」对 #2 **为假**。分支名违反 `cursor/` 前缀 |
| PR #1 | `a785317` | MERGEABLE | 计划冻结，Goal 1 祖先；合 #2 用 **merge commit** 会自动标已合 |
| PR #3 | 现状盘点 | CONFLICTING | 被 PR #5 的 `.agent_workspace` 盖掉，关闭即可 |
| PR #5 | 已合 | — | 算法冻结 |

`git merge-tree origin/main origin/cursor/soul-goal1-7b1c` 冲突恰好四个 add/add：`.gitignore`、`Cargo.lock`、`Cargo.toml`、`rust-toolchain.toml`。原因是 PR #5 把两 crate 工作区 squash 进近空的 `main`，Goal 1 早已有 15 成员工作区。

**裁决：主干是 `cursor/soul-goal1-7b1c`（PR #2）。** 它是 Goal 1 的超集。`agent/dev-sota` 停推，只樱桃摘它独有的东西（e0 禁发送类 crate、SOTA 评论文档、D32），然后关 PR #4。

---

## 2. 合入阻塞（不解决则没有任何实现能进 `main`）

### M1 — 两条 Goal 1 实现线并行（P0）

**证据：** `merge-base(goal1, dev-sota) = 862e858`。之后 goal1 36 个提交（起草、文件计划、WP13、问卷合并、壳接库）；dev-sota 另写一套 `soul-draft` / `soul-fileplan`。分析期间 dev-sota 仍在推。

**影响：** 合任何一边，另一边整树不可合。AC-26 是「一个主干 CI 绿」，不是两条线各自绿。

**解法：**

1. 立刻停止向 `agent/dev-sota` 推送。
2. 在 Goal 1 上开 `cursor/` 前缀短分支，樱桃摘 D32、SOTA 评论、e0 发送 crate 禁令（代码冲突则按 goal1 实现手工移植）。
3. 关 PR #4，分支改名为 `cursor/archive-dev-sota-a073` 或删除。
4. **禁止**把 `agent/dev-sota` 整树 merge/rebase 进 Goal 1（WP10/WP11 会变成没人审过的嵌合体）。不要用它的 `ci.yml`（它会在 Windows 上真跑 `ipc_roundtrip`，Goal 1 已经用 `--no-run` 绕开 WebView2Loader 的 `STATUS_ENTRYPOINT_NOT_FOUND`）。

### M2 — 把 `main` 并进 Goal 1（P0）

**解法（在 `cursor/soul-goal1-7b1c` 上，一次 merge，不要 rebase 96 个共享提交）：**

```text
git checkout -b cursor/goal1-absorb-main-a073 origin/cursor/soul-goal1-7b1c
git merge origin/main
```

冲突处理：

| 文件 | 取谁 |
|---|---|
| `Cargo.toml` | Goal 1 的成员表 **加上** `crates/soul-algo-tie`、`crates/soul-algo-trait` |
| `.gitignore` | Goal 1 |
| `rust-toolchain.toml` | Goal 1（已有 `1.83` + components） |
| `Cargo.lock` | Goal 1 的锁，再 `cargo update -w`（算法 crate 零依赖） |

算法 crate 的 `src/` / `tests/` **一个字节都不改**。若 `cargo deny` 要 license 字段，只给两个 manifest 加 `license.workspace = true`。然后 `cargo test -p soul-algo-tie -p soul-algo-trait` 必须仍全绿。

合进 `main` 时用 **merge commit，不要 squash**，否则 PR #1 的祖先关系断掉，96 个 STATUS 引用的提交哈希全部失效。

### M3 — Windows CI 红：夹具把 decoy 写进已授权根（P0，AC-18 / AC-26）

**不是** `canonicalize` 的 `\\?\C:\` 问题。`9ba160d` 已经修好授权；剩余失败是：

`crates/soul-fileplan/tests/common/mod.rs` 往 `base/alpha` 写 `decoy.txt`。NTFS 上 `alpha` **就是** `Alpha`，decoy 落在已授权根里。`authorized_scan.rs:65` 与 `:165` 的精确列表因此多出一项。产品行为正确。

连带伤害：`cargo test --workspace` 无 `--no-fail-fast`，Windows job 在 `soul-fileplan` 第一个失败二进制处停掉。自 WP11 起，`soul-graph` 及之后约 10 个 crate（含 `sqlcipher_smoke`、`soulcore`）在这条尖端上**从未在 windows-latest 跑完**。`SECURITY.md`「两边 CI 都跑加密冒烟」目前对这条分支为假。

**解法（先 B，再 A，各一次推）：**

1. `.github/workflows/ci.yml` 的 `test-windows` 给 `cargo test` 加 `--no-fail-fast`（先看见完整红表，不要一次修一个 crate）。
2. 夹具问文件系统，不要问 `cfg!(windows)`：只有 `canonicalize(alpha) != canonicalize(Alpha)` 时才写 decoy。`unauthorized_paths` 里「从未点名的 sibling」两条同样受探针控制；**Bravo 必须无条件留在语料里**。加一条「已授权根里没有 decoy」的夹具自检。
3. 不要：删 decoy、`#[ignore]` Windows、改产品代码去藏同目录文件、为了绿而缩短语料（有长度断言）。

Windows junction / reparse 逃逸仍是手动清单项，不是这两条红测试的根因。不要把「Linux 上用 Windows 字符串」当成 AC-18 在目标平台已证。

---

## 3. Goal 1 关闭阻塞（不解决则不能声称验收矩阵通过）

下列在两条实现线上**证据相同**（`soul-graph/src/build.rs`、`soul-import` 扇出、`soul-profile` intake 同病）。修在主干 Goal 1 上即可。

### G1 — 人脉图仍是 T0，冻结 T4D 未接线（P0）

`Tally::band`（`crates/soul-graph/src/build.rs:118`）按**全场地**次数做 3/10/3，无 180/360 时钟，无全库单一 `as_of`。本地又写了一份常量，违反 `DECISION.md` §3。群聊扇出 + T0 = 决胜夹具 `group_heavy_plus_one_direct_each_way` 会判 Strong；冻结答案是 Weak。

`DECISION.md` §6.5：替换完成前，现行 `Tally::band` 只是遗留，不是规范。把 T0 档位展示给用户并宣布 Goal 1 关闭，即违规。

**解法（照 §6，禁止新设计）：**

1. `soul-graph` 依赖 `soul-algo-tie`；`Tally::band` 整体换成 `soul_algo_tie::score`（或一次 `score_ego_network`）。
2. 每条 `InteractionRef` 在边界译成算法 crate 的 `Interaction`（方向、是否一对一、Unix 秒）。peer UUID → 本次 rebuild 的稠密 `u64`，禁止截断哈希。
3. **一次 rebuild 一个 `as_of`** = 全库 `max(occurred_at)`（或测试显式传入）。禁止 per-peer 取该人自己的最大时间戳。不读墙钟。
4. 持久化一对一/群聊分列、`last_contact`（任一场地）、`silent_days`、`algorithm_id`。合并计数由分列相加，禁止第二套加法。
5. `soul-draft` 的 `points_for` 换成 `soul_algo_trait::a2_render`。源码级禁止第二套 3/10/3。A2 的「半年」阈值必须是 `DEMOTE_ONE_BAND_DAYS` 的别名，禁止第二个 `180` 字面量。
6. **禁止**把 SQLCipher / `soul-store` / Tauri / HTTP / 墙钟拉进算法 crate。

已知代价原样入档，禁止静默修补：仅群聊者 Weak、F04c 复燃一响、群聊可挡住降档时钟。

### G2 — 问卷 intake 绕过轴锁（P0）

`intake` 对每条轴回答无条件 `place_axis(..., locked_by_user: None)`。锁旗保留，位置被问卷覆写——挂锁还亮着，锁住的内容没了。推断路径 `propose_axis` 有检查；`correction_lock.rs` 只测推断。冻结参照：`soul_algo_trait::apply_intake`。

**解法：** 锁轴不调用 `place_axis`；答案仍落证据与事件；回执列出 `ignored`（理由 `axis_locked_by_user`）。测试：`correct` → 再 intake → 位置不动、锁仍在、被忽略答案可见，且与 `apply_intake` replay 全等。约 10 行 + 1 测试。不要顺手改成 `NoDowngrade` 或打开 A1。

### G3 — 人脉图不可纠正；rebuild 抹掉裁决（P0）

PRODUCT_LOCK：「用户可查看和纠正」含人脉图。现状：`rebuild` 把 `user_verdict` 写成 `Unreviewed`，`tie_strength` 整行覆写；全仓库零个 `UserVerdict::Corrected` 构造点；图 UI 只读。规格已在 `.agent_workspace/round2/fable-b/GRAPH_CORRECTION.md`。

**解法：** 照抄 GC-1…GC-10。生效档 = 锁定时的用户档，机器档另存。rebuild 更新计数与 `machine_band`，不碰用户档。遗忘压过锁。A2/图只消费生效档。这与 T4D 接线回答的是不同问题，**不能**降成「T4D 之后再说」。

### G4 — 导入去重（P0，与 T4D 正确性绑定）

同一 Telegram / JSONL 再导一次会把一对一次数翻倍，从而跨过 3/10 门；群聊重复行还会刷新任一场地 `last_contact`。算法 crate 吃什么评什么，垃圾进是导入器的。

**解法：** 原子去重键 `sha256("soul.import.dedup.v1|source|external_id")`，只存摘要。重复则整条跳过并在回执里计数。验收：同一文件导两次，事件数、互动证据、档位、`last_contact` 不变。

群聊 N:1 扇出（owner 一条群消息 → 对每个历史发言人一条 Outgoing）**不制造 Strong**（T4D 次数门只看一对一），但会污染展示计数和降档时钟。扇出改成「有同场成员集合才写 peer 行」标 **P1**，不要在算法里加权重来「补偿」。

### G5 — 问卷没有 UI（P0 关闭 / 非合入）

AC-03 Given/When 是「无导入 / 完成问卷」。CI 绿的是把 `answers_basic.json` 喂给 `intake`。向导只有「默认全关」确认表，没有十一题，也没有注册问卷命令。无导出文件的用户造不出档案。

**解法：** 命令面已有 `questions()` / `intake`。向导加一屏，题面全部来自 `questions()`，留白合法。`/profile` 接 `profile_view` + `correct_axis`（G2 修完锁态才是真话）。Windows 上还依赖库能打开（见 S1）。

---

## 4. 发货阻塞与诚实缺口（Windows 真机产品）

### S1 — `DpapiKeyProvider` 仍是骨架（P0 发货 / P1 合入）

`forbid(unsafe_code)` 下两个 cfg 臂都返回 `KeyError::Unsupported`。Goal 1 session 在 Windows 上不开库：托盘可以亮，`/graph`、记忆、档案、导入全部拒绝。CI 绿是因为测试走 `TestKeyProvider`。

对照：`soul-collect/src/windows.rs` 已经手写 Win32 且允许 `unsafe`。`SECURITY.md` 把 stub 归因于「不能 unsafe」与工作区既有决定矛盾。

**解法：** 实现 DPAPI（`CryptProtectData` / `CryptUnprotectData`，用户范围，`UI_FORBIDDEN`）。失败返回 `Unavailable`（不要新铸一把钥匙把用户库变空）。明文密钥文件只能当开发机/CI 的显式测试入口，**不能**当发货路径——种子文件和 `soul.db` 一起被拷走就等于没加密。

合代码可以先保持「库打不开 + 界面直说」；**关闭 Goal 1 / 声称 AC-04、AC-08 在真机成立** 之前必须落地。

### S2 — KnownIdentifiers 恒空（P1）

WP13 已经打开 store，名单仍空。第三人整段占位兜住大头；owner 自己的话里出现的中文名 shape-scrub 抓不到。session 打开后解封联系人显示名，只进占位词典，不进 body。图 UI 对本机 owner 显示解封姓名是 P1（出网仍占位）。

### S3 — AC-21 在 Windows 上空转（P1）

`netwatch` 非 Linux 报 `Unsupported`。`egress_findings` 只要求连接数 = 0，观察器没看见也算过。修法：不能观察就必须显式 waiver，不能 `ok: true`；`install-smoke.ps1` 在 `observed=false` 时改信自己的 `Get-NetTCPConnection` 采样。不要为了绿而假装看到 0。

### S4 — perl/NASM 步骤不会失败（P1）

`|| echo missing` 使前置检查恒绿；bash 里的 Cygwin perl 不是 `openssl-src` 真正用的那一个。改成 pwsh 门禁，钉 `OPENSSL_SRC_PERL` 为 Windows 原生 perl，`OPENSSL_RUST_USE_NASM=1`。

### S5 — `tauri build` 从未在 runner 上跑（P1 诚实 / 非合入）

`package` job 停在打包前是有意的（下 NSIS 等于出网）。可加 `tauri build --no-bundle` 让 CLI 真解析配置，打包仍作者手动。AC-26「打包绿」要么改成「package job + 作者签名安装包」，要么 Goal 1 无法只靠 CI 关闭——这是矩阵措辞问题，不是偷偷在 CI 里下 NSIS。

作者手动七条（托盘、UAC、进程名、云开关系统层流量、真机采集、WebView2/DPI、清单）**不应**挡代码合入。

---

## 5. 推荐工序

每一步 CI 绿再做下一步。全程 `cursor/` 前缀。不要开 Goal 2。

| 序 | 项 | 产出 |
|---|---|---|
| 0 | 停 `agent/dev-sota`；本文件 Round 2 冻结 | 单一主干 |
| 1 | `--no-fail-fast` | Windows 红表完整 |
| 2 | 夹具探针修 AC-18 | windows-latest 能跑完工作区 |
| 3 | merge `main` → Goal 1（M2） | 算法 crate 进工作区 |
| 4 | 导入去重 +（可选同期）扇出收紧 | T4D 吃到的是唯一观测 |
| 5 | 接线 T4D + A2；删本地 3/10/3 | 产品档位 = 冻结档位 |
| 6 | A0 intake 不绕锁 | 挂锁是真话 |
| 7 | 图纠正 GC-1…GC-10 | 灵魂层可纠正 |
| 8 | 向导十一题 + `/profile` | AC-03 有用户路径 |
| 9 | DPAPI | Windows 能打开自己的库 |
| 10 | 填 KnownIdentifiers；AC-21/perl 门禁 | 发货诚实 |
| 11 | PR #2 **merge commit** 进 `main`；关 #3 #4；#1 随祖先闭合 | 一条历史 |
| 12 | 作者按 `scripts/author-manual-checklist.md` 签 AC-01 等 | Goal 1 关闭 |

樱桃摘自 dev-sota 的文档/e0 钉子可插在 3 之后任何空隙。

---

## 6. 父代理直改说明

本文件与 `.agent_workspace/blockers/` 为文档，落在 `cursor/blockers-analysis-a073`。未改业务逻辑。算法 crate 未动。
