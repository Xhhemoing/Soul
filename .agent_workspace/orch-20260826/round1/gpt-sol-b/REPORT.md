MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Round 1 边界探索报告：Git / 远程 / PR / 文档缺口

核验时间：2026-08-26 UTC。仓库：`Xhhemoing/Soul`。事实源为 `git fetch/ls-remote/ls-tree/rev-list` 与 `gh pr view/list`；没有修改权威文件。

## 结论摘要

1. `origin/main` 当前是 `a0ec14b`，不是 `docs/STATUS.md` 所写的 `7b35bde`。`STATUS` 的总体边界判断仍成立：`main` 的 `crates/` 只有 `soul-algo-tie`、`soul-algo-trait`，没有桌面壳或应用 crate；但分支尖端、分支自称和文件树摘要已陈旧。
2. `docs/BLOCKERS.md` **不在 `main`**；它存在于 `origin/cursor/blockers-analysis-a073@e669b78`。PR #6 仍 OPEN、CLEAN、MERGEABLE。
3. `origin/cursor/soul-goal1-7b1c` 可以 fetch，当前尖端 `6d1058b`。其根 `Cargo.toml` 有 16 个应用/基础 crate，**没有** `crates/soul-algo-tie` 与 `crates/soul-algo-trait`，所以 M2 在 Goal 1 主干上仍未关闭。两个算法成员只已出现在 `origin/cursor/goal1-unblock-a073@6133307`。
4. PR #2 仍 OPEN，但当前 `CONFLICTING / DIRTY`；它与 `main` 的 merge-base 仍是初始提交 `ea6f62f`，`main...goal1` 为 7 个 main-only、196 个 goal1-only 提交。先前“尚未合入”已升级为实际冲突风险。
5. PR #4 仍 OPEN、`CONFLICTING / DIRTY`，且三个唯一检查项（lint、Ubuntu test、Windows test）均失败；D49 所要求的“停并关闭”尚未执行。
6. 权威文档区有 6 个确定死链，均是 `docs/scan-rounds/R{1,2,3}-SYNTHESIS.md` 中把 Cursor run ID 写成相对路径。`README.md` 与 `docs/PLAN_INDEX.md` 的显式本地链接均可解析。
7. `main` 有 12 个零字节跟踪文件，全部是 `.agent_workspace/**/target/` 内的 Rust 构建产物；`docs/` 无空文件。`main` 共 585 个文件，其中 `.agent_workspace` 508 个、被跟踪的 `target/` 文件 191 个，仓库卫生明显不足。

## 远程事实表

### 指定 PR

| PR | head → base | GitHub 状态 | 合并状态 / 检查 | 结论 |
|---|---|---|---|---|
| [#2](https://github.com/Xhhemoing/Soul/pull/2) | `cursor/soul-goal1-7b1c@6d1058b` → `main` | OPEN | `CONFLICTING / DIRTY`；无上报 checks | Goal 1 未合入，且已冲突 |
| [#4](https://github.com/Xhhemoing/Soul/pull/4) | `agent/dev-sota@24c539f` → `main` | OPEN | `CONFLICTING / DIRTY`；lint、Ubuntu、Windows 三项均 FAILURE（各有两次记录） | 与 D49 的关闭要求不符 |
| [#5](https://github.com/Xhhemoing/Soul/pull/5) | `cursor/algo-verify-opt-a073@d79c8ba` → `main` | MERGED | 2026-08-24 15:48:53Z；无 checks | 算法冻结已合入 |
| [#6](https://github.com/Xhhemoing/Soul/pull/6) | `cursor/blockers-analysis-a073@e669b78` → `main` | OPEN | `MERGEABLE / CLEAN`；无 checks | `BLOCKERS.md` 仍只在该分支 |
| [#7](https://github.com/Xhhemoing/Soul/pull/7) | `cursor/goal1-unblock-a073@6133307` → `main` | OPEN | `MERGEABLE / CLEAN`；无 checks | 已含两个算法 workspace member，但尚未回到 Goal 1 主干 |
| [#8](https://github.com/Xhhemoing/Soul/pull/8) | 合并时 `cursor/polish-project-plan-5280@095c1f8` → `main` | MERGED | 2026-08-25 03:29:45Z；无 checks | 计划文档已合入；远程分支后来前移至 `a0ec14b` |

### 所有远程分支快照

- 最终复核时，`git ls-remote --heads origin` 与刷新后的 remote refs 共 101 个 head；其中两个 Round 1 审计分支是在初始 99-head 快照之后并发推送的。
- 拓扑上只有 `main@a0ec14b` 与 `cursor/polish-project-plan-5280@a0ec14b` 是 `main` 的 ancestor（两者同尖端）；其余 99 个 head 均不是 `main` 的 ancestor。
- “不是 ancestor”不等于 PR 未合并：例如 PR #5 采用 GitHub 合并结果，原 head `d79c8ba` 不必成为 `main` 的祖先。PR 状态应以 GitHub 表为准。

完整 head 清单：

```text
agent/dev-sota@24c539f
cursor/algo-verify-opt-a073@d79c8ba
cursor/bead-r1-algo-align-c441@f9422cf
cursor/bead-r1-algo-c441@e3cb62d
cursor/bead-r1-algo-ts-c441@5ca90c2
cursor/bead-r1-algo-ts-review-c441@24aecaf
cursor/bead-r1-align-review-c441@d436385
cursor/bead-r1-ci-c441@814250f
cursor/bead-r1-core-c441@2a47329
cursor/bead-r1-core-review-c441@75614bc
cursor/bead-r1-map-c441@0c59fb9
cursor/bead-r1-shell-c441@dc35ef0
cursor/bead-r1-shell-review-c441@4e7b990
cursor/bead-r1-shell-sh-c441@35e1047
cursor/bead-r1-ui-c441@7dcc8ad
cursor/bead-r2-al-review-c441@3fac2fd
cursor/bead-r2-al23-c441@f0e6a21
cursor/bead-r2-b04-ia-c441@7277cf3
cursor/bead-r2-b04-impl-c441@b0e34ed
cursor/bead-r2-b04-review-c441@f2f8af0
cursor/bead-r2-b05-ia-c441@86c213d
cursor/bead-r2-b05-impl-c441@33e18f0
cursor/bead-r2-b05-review-c441@20bf70e
cursor/bead-r2-b10-c441@af07100
cursor/bead-r2-b10-review-c441@ad3870f
cursor/bead-r2-build-c441@08b1349
cursor/bead-r2-cov-c441@bffa729
cursor/bead-r2-data-c441@7033dde
cursor/bead-r2-data1-c441@d3aed7a
cursor/bead-r3-al4-c441@417e9be
cursor/bead-r3-al4-review-c441@62ab3ab
cursor/bead-r3-b03-ia-c441@88192cd
cursor/bead-r3-b03-impl-c441@4a1cf4d
cursor/bead-r3-b03-review-c441@2a64186
cursor/bead-r3-b06-fix-c441@8676bc6
cursor/bead-r3-b06-ia-c441@ad42029
cursor/bead-r3-b06-impl-c441@a11358e
cursor/bead-r3-b06-review-c441@8937501
cursor/bead-r3-b07-ia-c441@1eed043
cursor/bead-r3-b07-impl-c441@3dbf43f
cursor/bead-r3-b07-review-c441@ffef9f6
cursor/bead-r3-b08-ia-c441@77956eb
cursor/bead-r3-b08-impl-c441@51dd0cf
cursor/bead-r3-b08-review-c441@1446072
cursor/beadflow-integration-c441@83818fc
cursor/blockers-analysis-a073@e669b78
cursor/blockers-r2-fable-a-7d67@0d8c8a9
cursor/blockers-r2-opus-a-1efe@73f8d6f
cursor/blockers-r2-opus-b-52e4@71f33a3
cursor/blockers-r3-opus-a-52bd@cbbbe9e
cursor/blockers-r3-opus-b-e326@6071240
cursor/collect-event-count-4a8e@0b9208b
cursor/e1-closed-store-4a8e@920fe87
cursor/e1-multiline-provenance-4a8e@654aca9
cursor/e1-netwatch-instrument-4a8e@1f1a258
cursor/e1-response-cap-4a8e@4959a68
cursor/events-source-index-4a8e@5078243
cursor/first-test-candidate-c441@85aae68
cursor/forget-impact-bind-4a8e@9d976f2
cursor/forget-summary-cite-4a8e@e080085
cursor/forget-tie-correct-4a8e@3f3eb60
cursor/g1-owner-fold-4a8e@5891d14
cursor/goal1-build-audit-c441@85aae68
cursor/goal1-close-loop-a073@d4490b0
cursor/goal1-closeout-c441-2d70@9eeef94
cursor/goal1-r2-code2-df52@8671794
cursor/goal1-r2-fable-a-57ad@6079ec6
cursor/goal1-r2-opus-b-4799@17c18dc
cursor/goal1-round2-probe-2d26@6244154
cursor/goal1-unblock-a073@6133307
cursor/graph-correct-ui-4a8e@3d04384
cursor/graph-same-band-4a8e@c49cc3b
cursor/import-tx-wrap-4a8e@5ad6d6e
cursor/intake-ignored-ui-4a8e@fc81246
cursor/intake-transact-4a8e@4d827ac
cursor/key-blob-race-4a8e@1924239
cursor/key-blob-reclaim-4a8e@c0093db
cursor/name-harvest-fold-4a8e@909b762
cursor/orchestrator-prompt-c441@7ca4894
cursor/phone-fullwidth-dot-4a8e@77b7402
cursor/phone-group-shape-4a8e@911e6d0
cursor/phone-unicode-shape-4a8e@a988fe2
cursor/polish-project-plan-5280@a0ec14b
cursor/port-t4d-4a8e@b9d2ec2
cursor/predict-algo-survey-a073@ae8119d
cursor/predict-session-pin-4a8e@1e3a881
cursor/predict-slice-4a8e@b51a7a3
cursor/project-status-audit-c49c@b7702bd
cursor/projection-as-of-none-4a8e@4981c7d
cursor/research-disposition-4a8e@a72e81b
cursor/round1-opus-b-core-impl-audit-ab42@13b322e
cursor/roundx-opus-b-poll-c71f@cb89814
cursor/schema-version-guard-4a8e@59ded11
cursor/soul-goal1-7b1c@6d1058b
cursor/soul-integration-4a8e@a27cfd1
cursor/soul-product-lock-7b1c@a785317
cursor/soul-status-round1-665b@0d57141
cursor/store-forget-4a8e@9f554e8
cursor/telegram-username-alias-4a8e@2a7b740
cursor/wizard-refusal-4a8e@a9f8227
main@a0ec14b
```

## `main` 文件树与 `STATUS` 对照

`origin/main@a0ec14b` 顶层只有：

```text
.agent_workspace/
.gitignore
Cargo.lock
Cargo.toml
README.md
crates/
docs/
rust-toolchain.toml
```

`crates/` 只有：

```text
soul-algo-tie
soul-algo-trait
```

| `STATUS` 声称 | 远程实况 | 判定 |
|---|---|---|
| `main@7b35bde` | `main@a0ec14b`；`7b35bde` 已是历史提交 | 陈旧 |
| `main` 没有应用代码 | 无 `apps/`，无 Goal 1 的 16 个 crate；只有两个算法 crate | 准确 |
| `main` 有两个算法 crate 与算法文档 | 有；但当前 `main` 还已有完整计划文档，且有 508 个 `.agent_workspace` 文件 | 摘要不完整 |
| “本分支”为 `cursor/polish-project-plan-5280` | 文件现在位于 `main`，该远程分支也已前移到 `a0ec14b` | 自称过期 |
| Goal 1 尖端 `df5d2dd` | 当前 `6d1058b` | 陈旧 |
| PR #6 / `BLOCKERS.md` 尚未在本树 | `main` 不存在该文件，PR #6 仍 OPEN | 准确 |
| PR #4 “未见关闭” | PR #4 仍 OPEN，且冲突、检查失败 | 准确但处置未完成 |
| M2：Goal 1 成员表没有两个算法 crate | 当前 `6d1058b` 仍缺这两个 member | 准确，但引用 SHA 陈旧 |
| PR #7 吸收线已有两个算法 member | 当前 `6133307` 的成员表确实含二者 | 准确，但引用 SHA `c81c233` 陈旧 |

同类 SHA 陈旧也出现在 `docs/SECURITY.md`：它仍引用 Goal 1 `df5d2dd`。

## M2 专项

Fetch 成功：

```text
origin/cursor/soul-goal1-7b1c@6d1058b
```

Goal 1 主干 `Cargo.toml` 的 16 个 member 为：

```text
soul-schema, soul-store-api, soul-store, soul-policy, soul-egress,
soul-graph, soul-import, soul-testkit, xtask, soulcore, soul-profile,
soul-memory, soul-collect, soul-draft, soul-fileplan, soul-win-dpapi
```

缺失：

```text
crates/soul-algo-tie
crates/soul-algo-trait
```

因此 M2 结论是 **OPEN**。PR #7 分支虽已把二者加入 workspace，但它基于 `main`，尚未进入 PR #2 的 Goal 1 主干；PR #2 当前冲突状态进一步说明不能把“PR #7 有修复”视为“M2 已关闭”。

## `docs/BLOCKERS.md`

| 树 | 结果 |
|---|---|
| `origin/main@a0ec14b:docs/BLOCKERS.md` | 不存在 |
| `origin/cursor/blockers-analysis-a073@e669b78:docs/BLOCKERS.md` | 存在 |
| PR #6 | OPEN、MERGEABLE、CLEAN |

`docs/STATUS.md`、`docs/PLAN_INDEX.md` 对“尚未合入 main”的描述仍正确。问题是权威阻碍清单仍是跨分支引用，普通 `main` checkout 无法直接打开它。

## 文档死链

### 权威/历史文档区

以下 6 个链接目标均写成裸 `bc-...`，Markdown/GitHub 会把它们解释为 `docs/scan-rounds/` 下的相对文件；仓库中不存在对应文件：

- `docs/scan-rounds/R1-SYNTHESIS.md:4` → `bc-ae1f3731-3cff-57cf-aab3-8e77527156a5`
- `docs/scan-rounds/R1-SYNTHESIS.md:5` → `bc-4055f588-dada-5d05-8326-87bf6bf4a1ef`
- `docs/scan-rounds/R2-SYNTHESIS.md:4` → `bc-32d82525-2c83-5745-a723-d0af183e7c8a`
- `docs/scan-rounds/R2-SYNTHESIS.md:5` → `bc-513459d3-2f20-51ca-af15-5acddd931480`
- `docs/scan-rounds/R3-SYNTHESIS.md:4` → `bc-bea41d97-3b1c-5de9-b306-721b3edf466e`
- `docs/scan-rounds/R3-SYNTHESIS.md:5` → `bc-75c9efdf-2a49-5dbb-8708-75daae90d989`

这些目标若本意是 Cursor run 页面，应使用完整 URL，而不是相对路径。`README.md` 与 `docs/PLAN_INDEX.md` 的显式本地链接均存在。

### 非权威过程材料

`.agent_workspace/plan-polish/round1/opus-b/README.md` 有 12 个死链实例：它在深层目录仍使用 `docs/...` 相对路径（其中 `docs/STATUS.md` 出现两次）。同目录 `PLAN_INDEX.md` 另有 4 个死链：`PRODUCT_LOCK.md`、`DECISIONS.md`、`algorithms/DECISION.md`、`PLAN_VERIFY_PROMPT.md` 均不在该快照旁。它们不是权威文件，但会误导直接浏览该目录的人。

## 空文件与仓库卫生

- `main`：12 个零字节文件，全部位于 `.agent_workspace/round3/fable-a/t4d-verify/target/debug/`，包括 `.cargo-lock`、3 个 `.rmeta` 与 8 个 incremental `.lock`。
- `main:docs/`：0 个空文件。
- `cursor/soul-goal1-7b1c`：全树 0 个空文件，文档区也为 0。
- `main` 有 191 个被 Git 跟踪的 `target/` 文件。构建产物进入历史会放大 clone/diff，并使“main 只有计划与算法源码”的表述失真。

## 不完善项（按优先级）

| 优先级 | 项 | 证据 / 影响 |
|---|---|---|
| P0 | PR #2 与 `main` 冲突，M2 仍未进 Goal 1 主干 | PR #2 `CONFLICTING/DIRTY`；Goal 1 `Cargo.toml` 缺两个算法 member |
| P0 | 双实现线仍并存 | PR #4 仍 OPEN、冲突且三类检查失败；与 D49 直接冲突 |
| P1 | 阻碍项权威文件不在 `main` | `docs/BLOCKERS.md` 只能跨分支读取；PR #6 尚未合入 |
| P1 | `STATUS` / `SECURITY` 的 SHA 与分支自称陈旧 | `main`、Goal 1、PR #7 均已前移；会导致审计基线错误 |
| P1 | 远程分支膨胀 | 最终 101 个 head，99 个不是 `main` ancestor；另有大量 Bead 分支/PR 与 Soul 主线共存，增加误选基线风险 |
| P2 | 6 个权威历史文档死链 | 三轮 synthesis 无法跳到原始 run |
| P2 | 大量构建产物被跟踪 | `.agent_workspace` 508 文件；`target/` 191 文件；12 个零字节文件 |
| P2 | 非权威快照有 16 个死链实例 | 虽非权威，直接浏览仍会误导 |

本轮只报告事实，没有修改 `docs/`、`Cargo.toml` 或其他权威面。
