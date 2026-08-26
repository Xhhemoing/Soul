`claude-fable-5-thinking-xhigh`

# ROUND 2 · WP-B10 CI/CD 复审

Reviewer：`claude-fable-5-thinking-xhigh`（实际运行 slug，无静默降级；只审查，不实现）。
被审对象：`9b2d65a`（ci(bead): absorb isolated Bead workflow，PR #41 head `af07100` 与吸收提交的 `bead.yml` 零字节 diff），基线 `origin/cursor/beadflow-integration-c441`。
审查依据：`docs/bead/reviews/round2-build.md`（可实现工作流 + 验收清单）、`docs/bead/reviews/round1-cicd.md`（I-1…I-7 / CI-1…CI-7 / §0.5 / §4）、`docs/agent-decisions.md` BD3 / BD4 / BD17。
本轮只新增本文件；未改任何 workflow、产品文件、Soul 树。`gh` 只读，仅用于查 run 历史与 PR #41 描述。

---

## 0. 结论先行

**接受。无 HIGH、无 MED 缺陷。** `bead.yml` 与 `round2-build.md` 的可实现形状逐行等价（差异仅为注释与 step 命名），
七条 CI 不变量全部成立，且其中最关键的三条（CI-1 零字节、CI-2 反向不干扰、CI-6 不 dispatch Soul）都有**实证**而非只有文本核对：
blob 哈希三点一致、Actions run 历史两条流水线各自守界、PR #41 明拒 runner 试探。下面列 4 条 LOW，均为记录性质，不构成回退理由。

## 1. 逐项核对

### 1.1 CI-1 / I-1 · Soul `ci.yml` 零字节 diff ✅（blob 级实证）

`ci.yml` 的 blob 在三个点完全同一：`0f745b6`

- 专属线分叉点 `origin/cursor/first-test-candidate-c441:.github/workflows/ci.yml`
- B10 前一提交 `730b351:.github/workflows/ci.yml`
- 当前 HEAD `.github/workflows/ci.yml`

`git log <fork>..HEAD -- .github/workflows/ci.yml` 为空——整条 bead 线从未有任何提交碰过 Soul workflow。
零 trigger 改动、零 job 改动、零 paths 改动，不是「diff 很小」而是 diff 为 0 字节。

### 1.2 CI-2 · `push.branches` 只有 `cursor/bead*` ✅（含实证）

```yaml
push:
  branches:
    - "cursor/bead*"
```

该模式匹配不到 `main`、`cursor/soul-goal1-7b1c`、`cursor/first-test-candidate-c441`、`cursor/goal1-*` 中任何一个。
Actions run 历史（`gh run list`）交叉验证：`Bead` 只在 `cursor/bead-r2-b10-c441` 与 `cursor/beadflow-integration-c441`
上跑过（两个都命中 `cursor/bead*`）；`CI` 只在 `cursor/soul-goal1-7b1c` 上跑过；两边零串线、零 dispatch 事件。

### 1.3 CI-3 · 无 `pull_request` 触发 ✅

`on:` 下只有 `push` 与 `workflow_dispatch`。头注释原文复述了 `ci.yml` 花钱买过的理由
（GitHub 按整个 PR 而非最新提交求值 paths，docs-only 追加提交会重开全部 job；push checks 已挂 PR 页面）。

### 1.4 CI-4 · 并发组与 Soul `CI` 隔离 ✅

`name: Bead` ≠ Soul 的 `CI`；`concurrency.group: ${{ github.workflow }}-${{ github.ref_name }}` 含 `github.workflow`。
两条流水线的组名分别以 `Bead-`、`CI-` 打头，命名空间不相交，互不 cancel。bead 侧不带 `github.head_ref ||`
是正确简化——没有 `pull_request` 触发，`head_ref` 恒空。

### 1.5 CI-5 · paths 收窄且刻意去掉 `docs/bead/**` ✅

paths = `apps/bead/**`、`crates/bead-core/**`、`pnpm-lock.yaml`、`.github/workflows/bead.yml`，与 round1 §4.2 /
round2-build 完全一致。`docs/bead/**` 与 `docs/agent-progress.md` 都不在 paths 里——本复审文件的 push 本身
就是这条过滤的实测：只动 `docs/bead/reviews/**`，不应触发 `Bead`。

### 1.6 CI-6 / BD17 · 不从 bead ref dispatch Soul `CI` ✅

- `bead.yml` 全文无对 Soul workflow 的任何引用：无 `workflow_call`、无 `gh workflow run`、无 repository_dispatch。
- 头注释原文点名 BD17/CI-6 并解释后果（Soul job 守卫放行 dispatch，会把五门真跑在 bead ref 上）。
- run 历史实证：全部 `CI` run 的 event 均为 `push` 且 headBranch 均为 `cursor/soul-goal1-7b1c`，零 dispatch。

### 1.7 CI-7 · 无 empty-commit 试探 ✅

触发过 `Bead` 的两次 push（`af07100` 新增 bead.yml、`9b2d65a` 吸收提交）都携带真实内容。两次 run 均按
仓库已知 Billing 状况在 ~3s 空 steps / 空 runnerName 失败，PR #41 如实记录并明确「不以 empty-commit 探 runner，
本机命令仍是门」。I-7 四条验证在 PR #41 描述里齐备：app 358 绿、bead-core 153 绿、`just ci` / e0-audit / denylist 绿。

### 1.8 BD4 / I-6 · 根 pnpm 面零改动 ✅

根 `package.json` scripts 仍只 `--filter @soul/desktop`（lint/test/build/dev 四条原样）；`pnpm-workspace.yaml`
仍只 `apps/*`。`bead.yml` 两个 job 只跑包级命令（`pnpm --filter @bead/app …`、`--manifest-path crates/bead-core/Cargo.toml`），
不调根脚本、不调 `just ci`、不碰根 workspace，与头注释承诺一致。

### 1.9 BD3 / I-2 · bead-core 仍在根 Cargo workspace 外 ✅

根 `Cargo.toml` members 仍是 15 个 soul crate + xtask，无 `crates/bead-core`；`crates/bead-core/Cargo.toml`
自带空 `[workspace]`，`Cargo.lock` 已提交，`[dependencies]` 仍为空（I-3 顺带成立）。

### 1.10 job 守卫 ✅

`bead-app` 与 `bead-core` 两个 job 都带 `if: startsWith(github.ref, 'refs/heads/cursor/bead')`——从非 bead ref
手动 dispatch 时两个 job 全 skip，不会把 bead 命令跑在 Soul 分支上。守卫语义与 `cursor/bead*` 分支过滤一致。

### 1.11 变更面与工程一致性 ✅

- `9b2d65a` 只动两个文件：新增 `.github/workflows/bead.yml`（+104）与进度账本 `docs/agent-progress.md`（本线惯例，
  不属产品面，不在 bead.yml paths 内，不触发任何流水线）。`justfile`、`deny.toml`、`.gitignore`、Soul 树零触碰。
- Action 版本与 Soul `ci.yml` 逐个同款：`actions/checkout@v4`、`pnpm/action-setup@v4`（pnpm 版本由根
  `packageManager: pnpm@10.15.0` 钉）、`actions/setup-node@v4` + node 22 + `cache: pnpm`、
  `dtolnay/rust-toolchain@1.83`（与 `rust-toolchain.toml` 的 1.83 一致）、`Swatinem/rust-cache@v2`。
- 比 Soul 更严的两处：workflow 级 `permissions: contents: read`（Soul `ci.yml` 无 permissions 块，用默认 token 权限）；
  两 job 均 `timeout-minutes: 15`（Soul job 无 timeout）。
- `bead.yml` 全文零外网 URL（I-4 防御纵深；`.yml` 本就不在 e0 扫描扩展名内）、零 denylist 词。

## 2. 缺陷清单

### HIGH

无。

### MED

无。

### LOW

- **LOW-1 · coexist job 按简报后置，bead 分支暂无 e0/denylist 自动网。** round1 §4.2 参考实现含第三个
  `coexist` job（`xtask e0-audit` / `denylist-audit`），round2-build 第 6 条明示可后置、实现照办。当前唯一
  自动覆盖是本机 `just ci` 记录（I-7）。Billing 恢复后建议按 round1 §4.2 的独立 `coexist` job 形状补上
  （`-p xtask` 只编译 xtask，不碰 soul 业务 crate 测试），不要折进两个包 job。
- **LOW-2 · action 按 tag 而非 commit SHA 钉。** 与 Soul `ci.yml` 全款一致，是仓库既有约定；bead 单方面
  改 SHA 钉会造成两个 workflow 的维护口径分裂。若将来做供应链加固应仓级统一。`contents: read` 已把
  token 爆炸半径压到最低，是现状下的正确缓解。
- **LOW-3 · paths 不含根 `package.json` 与 `rust-toolchain.toml`。** 理论上 bead 分支单独 bump pnpm 钉版或
  工具链而不动锁文件/workflow 时不会重触发。实践中这两类 bump 必然连带 `pnpm-lock.yaml` 或
  `bead.yml` 里的 `@1.83`（两者都在 paths 内）。记录即可，不建议预防性放宽——CI-5 的收窄理由优先。
- **LOW-4 · Billing 恢复前 bead PR 上会持续挂红 X。** 两个 job 每次触发都在 ~3s 空跑失败（与 Soul `CI`
  同因），这是 round1 §0.5 已预告并接受的状态；红 X ≠ 代码红，验收以 I-7 本机四条记录为准。落地即完成
  任务（CI-7），无需任何补救动作。

## 3. 禁区自查

本审查只新增 `docs/bead/reviews/round2-cicd-review.md`。未触碰：`.github/workflows/ci.yml`、
`.github/workflows/bead.yml`、根 `Cargo.toml` / `package.json` / `pnpm-workspace.yaml` / `justfile` / `deny.toml`、
`apps/**`、`crates/**`、`docs/PRODUCT_LOCK.md`、`docs/STATUS.md`。无 empty commit；未从任何 ref dispatch
任何 workflow；`gh` 仅读。
