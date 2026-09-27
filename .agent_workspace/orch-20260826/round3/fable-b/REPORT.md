MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 3 / fable-b — 交叉矛盾清单（REPORT）

只读轮。读毕 `orch-20260826/round1`（6 份）与 `round2`（6 份）全部 REPORT，对每处报告间分歧回到一手证据复核（`git show` 远程分支、`gh` 只读、PRODUCT_LOCK 原文行号）。零 git 写操作；产出仅本目录 `CROSS_CHECK.md` 与本文件。

## 最重要的一条：本轮新钉死的事实

**D61 撞号已实际发生**（R1 fable-a P1-3 预言的事故，R2 两份报告各只见一半）：

- `soul-integration-4a8e` 的 D61 = 空图/姓名残差登记（其后还有 D62/D63）；
- `goal1-close-loop-a073`（PR #10 链）的 D61 = **主干由 `soul-goal1-7b1c` 更替为 `goal1-unblock-a073`**。

两条支线各自分配了内容完全不同的 D61。后果：R2 fable-b「D61 只存在于 PR #10 链」不完整；R2 fable-a「跳号至 D64+」的建议依据失效（D62/D63 也被单线占用）；「主干更替」这条全案最关键的拍板坐在撞号行上，落 main 前必须重编号。PR #6 的 D32 撞号处理只剩**括注方案**一条安全路径。

## 四个点名问题的裁断

1. **BeadFlow 是否违反 PRODUCT_LOCK：否。** 采信 R2 fable-a（定性）+ R2 gpt-sol-b（实证：#16–#56 零触锁、锁 blob 与 base 同一、全仓无第二份 PRODUCT.md）。PRODUCT_LOCK 第 4 行只禁 Soul 自建第二份 PRODUCT.md（本轮实读原文确认）。真正违锁的是 **PR #15（soul-integration）无 D 号改 Telegram 导入口径**——追责方向不要指错线。BeadFlow 的真实问题是治理登记缺位 + 经 first-test-candidate 搭车进主干的风险，处置走 R2 fable-a 的 1C（登记 + 冻结 #16 合入）。
2. **schemas.lock 无校验器严重度：低（R1 记「中」应降级），但必须与配对升级捆绑采信。** 采信 R2 opus-b：校验器存在两个（`xtask schema-freeze --check` 在 Goal 1 CI；`soul-schema` 实例校验器挂在 `put_relationship` 写库路径），只是不在 main 树；main 侧 11/11 哈希吻合且与 Goal 1 线字节同一。前提（main 不单独改 schema、收紧按 D58 同批）破掉则回「中」。**同一发现使 S5（D32 真值表放行，静默丢用户纠正）从「中」升「高」**——两条必须一起转述，拆开即读反。
3. **PR #7 该不该合：今天不合。** 采信 R2 fable-b：D61 主干缺主干侧 17 个代码提交（含六连诚实文案修复与 IPC 级 AC 补证），今天合入 = main 实质退步；MERGEABLE/CLEAN（本轮 gh 复核仍成立）是机械事实不是裁决。前置顺序：D61 落 main（先解撞号）→ 拓扑归一（16 待摘 + 5 文件冲突）→ 若走 2A 再做与 soul-integration port 的语义对账（无人验证过等价）。**#6 与 #7 严格区分：#6 今天可按 3A 带三件套合入。**
4. **「中」vs「中等」是否违反产品锁：是。** 采信 R2 opus-a 的升级定性：`PRODUCT_LOCK.md:78`「用户可见文本只有『强 / 中等 / 弱』三个词」是产品锁本体承诺（本轮实读第 78 行与 `types.rs:87` 双确认），一处字面共踩三线（+COPY_ZH §0.1、§5.5）。修复是一处字面替换，无需产品裁决、需留痕；与 tie crate「强联系」系列（需按 DECISION §6.4 先裁归属）严格分开，勿捆绑。

## 真矛盾 vs 表面差异（详表见 CROSS_CHECK.md）

**真矛盾 6 条**，全部裁定采信 R2 一侧（R2 每条带一手反证或完整枚举）：上述四问对应的 4 条，加上——墙钟护栏（R1「加 `SystemTime::now()` 不会红」被 R2 反证为 28 红，但 DECISION §6.3 禁止的缺省入口形态确可 135/135 全绿混入，源码扫描是唯一护栏）；「躲过筛子」的安全含义（R2 实测 42/42 通过 peer 筛，今天无安全问题，结构缺口保留）。

**表面差异 8 条**，无需裁决：M2「OPEN vs 已修」（基线不同）、「不改写 Soul」的三个口径、合并路径 owner 的时序升级、PR #2 提交数 196 vs 49（不同 merge-base）、schema 份数 12 vs 11（是否含 lock/_defs）、测试数全一致、常量双源三级证据强度递进、R1 证据文件收集丢失（流程问题，R2 opus-b 已重验其中 24 条无分歧）。

## 给父代理的转述纪律

- R1 与 R2 冲突处一律采信 R2，但 R2 内部互有盲区（两个半 D61、降级/升级配对），请按 CROSS_CHECK.md 合并口径引用，勿单引孤立句子。
- 两个易混对勿压成一句：#6 ≠ #7（可合/不可合）；「中」≠「强联系」（即修/先裁）。

## 未做声明

未执行任何 `git add/commit/push/checkout`，未改 `docs/**`、`crates/**` 或任何既有文件；本目录两个新文件是本轮唯一产出。
