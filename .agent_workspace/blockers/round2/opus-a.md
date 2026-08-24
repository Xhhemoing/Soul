# Round 2 · opus-a 交叉验证增量

MODEL_SLUG: claude-opus-5-thinking-high-fast
对象：`docs/BLOCKERS.md`（`BLOCKERS_DRAFT`）× FORMAL_WORK_PROMPT AC-01..AC-26 × `origin/cursor/soul-goal1-7b1c`（`3e88b48`）。
只写增量。核对后成立、不再重复的：M1、M2、M3 根因、G2、G3、G4、S4。

## 1. 四条判定

- 快速失败 P0：**成立，且被低估**（§2）。
- DPAPI「P0 发货 / P1 合入」：**排序成立，AC 挂钩不成立**（§5）。
- G5 问卷 UI：**P0 关闭成立，但依据是 PRODUCT_LOCK 切片 2/3，不是 AC-03**，且范围少写一半（§4）。
- §3 标题「不解决则不能声称验收矩阵通过」：**不成立**（§3）。

## 2. M3 的连带伤害比稿子大（CI run 32748412486 实测）

Windows job 停在 `cargo test`，后面两步 **`cargo test (desktop shell)` 与 `cargo test --no-run (ipc_roundtrip)` 状态是 `-`，从未开始**。
所以 `one_store`、`command_surface`、`no_egress_path`、`shell_is_local_only` 在 windows 上一次都没跑过；
尖端提交 `3e88b48` 刚加的 `--no-run` 编译闸也从未执行过——它挡的正是「WebView2 那个文件烂掉没人知道」。
实测红的恰好两条，与稿子一致：`authorized_scan.rs:65`、`:165`。
但 `unauthorized_paths.rs` 排在 `authorized_scan` 之后，**根本没跑到**：它的 `lower_alpha` 用例是预测红不是实测红，
且受影响的是**四处**（`:50`、`:53`、`:237`、`:247`），稿子 §2 只点了「两条」。夹具探针按四处改。
`lint` / `test-linux` / `sbom` / `package` 四个 job 全绿，唯一红的是 windows job。

## 3. G1–G5 一条也没落在失败的 AC 行上

逐行核过：AC-08 只要「节点≥3、边有证据」，不提档位——G1 不挡它；AC-03 的「谁跑」栏就是 CI，喂 `answers_basic.json` 合规——G5 不挡它；
AC-04/AC-05 不要求幂等——G4 不挡它；人脉图纠正在矩阵里没有对应行——G3 不挡它。
G1–G5 的权威是 `PRODUCT_LOCK.md` 的 13 条切片与 `DECISION.md` §6 第 5 条，**不是**验收矩阵。
FORMAL_WORK_PROMPT 第 42 行「Goal 1 完成 = 矩阵全部 v0.1 行通过」与第 18 行「切片以 13 条为准」并存，
稿子选后者是对的，但 §3 现在的标题写成了前者，会被一句「矩阵没这行」整节推翻。**改标题，别改内容。**

## 4. G5 的范围少写一半，§5 第 8 步据此重估

壳的 `COMMAND_NAMES` 只有 14 个：config / session / wizard / cloud / files / authorize / preview / graph / summary + 五个起草。
`crates/soulcore/src/commands/` 有 `profile.rs`、`memory.rs`、`import.rs`、`collect.rs`、`policy.rs`、`store.rs`——**一个都没接到壳上**。
路由只有 Home / Wizard / Settings / Files / Graph / Draft，`rg questionnaire apps/desktop` 零命中。
没有 UI 的不止问卷：**导入（切片 2）、可编辑档案（3）、记忆 CRUD 与遗忘预览（5）、采集开关（7）、研究预览（10）** 全部没有用户路径。
采集开关这条 `scripts/author-manual-checklist.md` 自己就写着「界面上还没有采集开关」。
G5 标题应是「壳只接了 13 条切片里的 6 条」；第 8 步「向导十一题 + `/profile`」严重低估：它是四五屏加一层命令注册，不是一屏。

## 5. DPAPI：排序留着，理由换掉

矩阵里 AC-04/AC-08 的「谁跑」是 CI，没有任何一行要求真机开库——所以 DPAPI **挡不住任何 AC 行**，
S1 写的「声称 AC-04、AC-08 在真机成立之前必须落地」在矩阵文本上没有落点。
这和 S5 是同一个洞：**把 DPAPI 并进 S5 的「矩阵措辞问题」**，否则 Goal 1 可以在「唯一发货平台上库打不开」的状态下合法关闭。
补两条稿子没写的证据：`crates/soulcore/src/headless.rs:272` 走 `open_test_store` + 常量种子 `SCRATCH_SEED`；
`one_store.rs:120` 对 `(None, None)` 只断言 `!store_opened` 就算过。**没有任何 Windows job 碰过真的 KeyProvider**——
stub 对 CI 不可见是结构决定的，不是巧合。这条比「CI 绿是因为走 TestKeyProvider」更该写进 S1。

## 6. G1 第 5 步是错的，且与 M2 打架

`soul-algo-trait/Cargo.toml` 的 `[dependencies]` **是空的**（注释写明「Zero runtime dependencies」），
A2 在物理上无法引用 `soul_algo_tie::DEMOTE_ONE_BAND_DAYS`；要引用就得给冻结 crate 加依赖，
直接违反 M2「算法 crate 的 `src/` / `tests/` 一个字节都不改」。
且 `a2.rs:85-91` 的 `DORMANT_AFTER_DAYS = 180` 的文档注释明说「**This is not a band rule**，band 原样透传」——
它与降档时钟是两个语义，别名化会把它们焊死，再也拆不开。
另外 `soul-draft` 里**没有**第二套 3/10/3，也没有 180：`analysis.rs:242 points_for` 直接读 `edge.tie_strength.band`。
改写：第 5 步的理由不是「删重复常量」，而是**文案对齐 `COPY_ZH.md` §6**；换 `a2_render` 会给用户**新增**一句沉寂描述，是加文案不是去重。

## 7. 三条杂项修正

1. 「权威」行引 `docs/PRODUCT_LOCK.md` 与 `docs/FORMAL_WORK_PROMPT.md`，但 `cursor/blockers-analysis-a073` 上只有
   `docs/BLOCKERS.md` 与 `docs/algorithms/`——**这两份在本分支不存在**，M2 之后才有。要么 M2 之后再冻，
   要么加脚注说明它们在 `cursor/soul-goal1-7b1c` 上。同理 `DECISION.md` §6.1 仍写 `crates/soul-algo` 与 `graph_build.rs`，
   两个名字在任何分支都不存在（真名 `soul-algo-tie` / `soul-algo-trait`、`soul-graph/src/build.rs`）；稿子 G1 用的是真名，
   但实现者会照 DECISION 去找，值得一行脚注。
2. AC-09/AC-10 在矩阵里「谁跑」是 **CI**（`FakeForegroundSource` 已覆盖）。`author-manual-checklist.md` 开头写
   「AC-09 / AC-10 在验收矩阵里就写着作者手动」是**误引**，§4 的「作者手动七条」继承了这个错。
   真机那一段是加分证据，不是矩阵行。反向的一条：AC-01 的「不提权」半边**已经在 CI 里**——`package` job 从编出来的
   `soul.exe` 的 PE 里读清单断言 `asInvoker` 并排除 `requireAdministrator`。S5 现在的措辞让人以为打包那栏整栏是手动，
   实际只有 NSIS 那一下是。
3. S3 请降级：`install-smoke.ps1:303-317` 自己就在采 `Get-NetTCPConnection`，第 482 行独立断言「Windows 也没看见非回环连接」，
   而 `package` job 每次都跑它。剩下的洞只有一处：`headless.rs:757` 已经输出 `observed`，第 473 行的断言却不读它。
   一行断言的事，P1 偏重，建议 P2 / 措辞。
