# ROUND 1 · F5 测试 / 构建 / CI 隔离清单（WP-B10）

Reviewer：`claude-fable-5-thinking-xhigh`（实际运行 slug，无静默降级；只审 CI/构建/可靠性隔离，不写产品代码）。
基线：`origin/cursor/beadflow-integration-c441` @ `0c3871e`。
输入：`docs/bead/reviews/round1-map.md`（F1 地图 §2/§4）、`docs/bead/WORK_PACKAGES.md` WP-B10、`docs/agent-decisions.md` BD14–BD16、`.github/workflows/ci.yml`、`justfile`、`deny.toml`、`crates/xtask/src/egress.rs`、`crates/xtask/src/denylist.rs`。

**用途**：这是 O1（`cursor/bead-r1-core-c441`）与 O2（`cursor/bead-r1-shell-c441`）交付的验收门。ROUND 2 审查按编号逐条打钩；任何一条红即不合入。所有条目都从两支审计的**源码**核实过，不是转述 WP。

---

## 0. 门禁事实（核实自源码，O1/O2 必须先读这一节）

以下行为决定了 bead 树会被 Soul 的哪些门禁扫到。逐条附出处。

### 0.1 `xtask e0-audit`（`crates/xtask/src/egress.rs`）

- **URL 扫描扫三棵树**：`crates`、`apps`、`scripts`（`audit()`，L225）。`apps/bead/**` 与 `crates/bead-core/**` 都在射程内。
- **可扫描扩展名**（`is_scannable`，L457）：`rs ts tsx js jsx json html css toml conf ps1 psm1 sh cmd bat`。
  **不扫**：`.md`、`.yaml`/`.yml`、`.lock`、`.svg` —— 引用出处/论文链接一律写进 README/`.md`；`Cargo.lock`、`pnpm-lock.yaml` 里的 registry URL 无碍。
- **扫描是裸文本逐行**（`find_url_literals`）：**注释里的 URL 一样命中**。这与 denylist（跳过注释）不同，别混。
- **允许名单只有**（`ALLOWED_URL_PREFIXES`，L71）：`https://soul.local/schemas/` 与回环地址（`127.0.0.1`/`localhost`，按 host 锚定匹配）。其余任何 `http(s)://` 字面量都红。
- **目录豁免**（`EXEMPT_DIRS`，L80）：目录名字面为 `fixtures`、`tests`、`target`、`node_modules`、`.git` 的整棵子树。**与 `src` 并置的 `*.test.ts(x)` 不豁免**。
- **`dist` 只在旁边有 `package.json` 时豁免**（`EXEMPT_BUILD_OUTPUT`，L95）——但 bead 的 dist 本来就不该提交（见 LK-2）。
- **crate 级豁免只有 `xtask`**（`EXEMPT_CRATES`，L100）。bead-core 全量被扫，含它的 `Cargo.toml`。
- **依赖图那一半看不见 bead-core**：`audit()` 用根 `Cargo.toml` 的 `cargo metadata` 起步（L211），bead-core 自带 `[workspace]` 就不在图里。含义：**没有任何自动网拦得住 bead-core 引 HTTP client**（cargo-deny 同样不覆盖，见 0.4）——这就是 WS-3 存在的原因。

### 0.2 `xtask denylist-audit`（`crates/xtask/src/denylist.rs`）

- **只扫 `crates/**` 下的 `.rs`**（`audit()`，L119）。TS 不扫，`apps/**` 不扫。`crates/bead-core/src/**/*.rs` 在射程内。
- **扫字符串字面量与标识符，跳过注释**（`scan_source` 文档注释，L160–163）。
- **匹配规则**：ASCII 词按词边界，标识符先按 `_`/驼峰拆词再整词比对（`PixelArtScore` → `[pixel, art, score]` → 命中 `score`；`underscore` 是单词不命中）；**CJK 按子串**（`百分数` 命中 `分数`）。
- **豁免**（L24）：路径中含 `fixtures`/`tests`/`target`/`node_modules`/`.git` 任一段，或 `crates/xtask`。**`src/` 里的内联 `#[cfg(test)]` 模块不豁免**。
- 对 bead 最危险的词（`fixtures/denylist/diagnostic_terms.txt`）：`score`、`percentile`、`量表`、`得分`、`分数`、`评分`、`计分`、`常模`、`标准分`、`百分位`。

### 0.3 `just ci` 与锁文件（`justfile`）

- `just ci` = `lint schema e0 denylist fixtures-verify test smoke-lint sbom ui-lint ui-test`（L106）。
- `ui-lint`/`ui-test` 依赖 `ui-install` = **`pnpm install --frozen-lockfile`**（L116–117）。
- `pnpm-workspace.yaml` 只有 `apps/*` 一个 glob → `apps/bead` 一落地即被收进 pnpm workspace；它的 `package.json` 不进锁文件，则本线**所有**分支上的 `just ci` 立即红。这就是 BD16。
- pnpm 由根 `package.json` 的 `packageManager` 钉在 **10.15.0**；重生成锁文件必须用这个版本，避免 lockfileVersion 漂移。

### 0.4 根 cargo 门禁的边界

- 根 workspace 16 个成员（15 soul + xtask），`fmt --all` / `clippy --workspace` / `test --workspace` / `cargo deny` / `sbom` / `schema-freeze` 都以它为界。bead-core 自带 `[workspace]` 即隔离（先例：`apps/desktop/src-tauri/Cargo.toml` L15，且它的 `Cargo.lock` 已提交）。
- 代价：**没有任何现有门禁替 bead-core 跑 fmt/clippy/test**，也没有任何网管它的依赖 —— 由本清单 WS-3/WS-5 与 §4 的 bead workflow 补上。
- 工具链仓级钉死：`rust-toolchain.toml` = 1.83；根 `[workspace.package]` edition 2021。bead-core 必须在这套上编译。

### 0.5 Soul hosted CI（`.github/workflows/ci.yml`）

- push 只触发 `main` 与 `cursor/soul-goal1-7b1c`（L19–22）+ `paths-ignore: docs/** / *.md`。**bead 分支的 push 永远不会触发它**——这个性质是本清单要守住的第一不变量。
- 各 job 有 `if: workflow_dispatch || ref == main || ref == goal1` 守卫——注意 **`workflow_dispatch` 可以从任意 ref 跑**：从 bead 分支手动 dispatch Soul CI 会真的把五门跑在 bead ref 上。禁止这么做（CI-6）。
- hosted runner 因 Billing 全线空跑（STATUS 已记）。含义见 §5：workflow 落地是为了 Billing 修好后即时生效，落地前的门禁 = 本机跑 + 文档记录，**禁止 empty-commit 试探**。

---

## 1. O1 验收清单（`crates/bead-core`，分支 `cursor/bead-r1-core-c441`）

### 隔离（WS）

- [ ] **WS-1** `crates/bead-core/Cargo.toml` 自带 `[workspace]`（空表即可，照抄 `apps/desktop/src-tauri/Cargo.toml` L15 先例）；根 `Cargo.toml` 的 `members` **零字节改动**（BD3）。少了 `[workspace]` 的后果：cargo 向上找到根 workspace、发现自己不在 members 里，根侧与包内构建**双双报错**。
- [ ] **WS-2** 提交 `crates/bead-core/Cargo.lock`（先例：src-tauri 的锁文件已提交；`.lock` 不被 e0 扫，registry URL 无碍）。
- [ ] **WS-3** 依赖近零：`[dependencies]` 建议仅 `serde` + `serde_json`，版本与根钉一致（1.0.217 / 1.0.135；独立 workspace 用不了根的 `workspace = true`，要在自己的 Cargo.toml 里写死）。**每加一个新依赖必须在 PR 描述里给一行书面理由**——cargo-deny 与 e0 依赖图都看不见这个 workspace（0.1/0.4），书面理由就是唯一的网。HTTP client（`reqwest`/`hyper`/`ureq`/`curl`/`isahc`/`attohttpc`/`surf`）与任何网络栈**绝对禁止**。不引 `image` crate（解码归浏览器，F1 §4 B01.1）。
- [ ] **WS-4** 不 path-depend 任何 `soul-*` crate，也不被任何 soul crate 反向依赖；`deny.toml` 零字节改动。
- [ ] **WS-5** 自建门禁三连在 PR 里有记录（命令 + 结果），因为没人替它跑：
  ```bash
  cargo fmt --manifest-path crates/bead-core/Cargo.toml --check
  cargo clippy --manifest-path crates/bead-core/Cargo.toml --all-targets -- -D warnings
  cargo test --manifest-path crates/bead-core/Cargo.toml
  ```
- [ ] **WS-6** 在 Rust 1.83 / edition 2021 上编译（`rust-toolchain.toml` 对包内所有 cargo 调用生效，选依赖先看 MSRV）。

### e0-audit（E0，对 O1）

- [ ] **E0-1** `Cargo.toml` 不写 `repository` / `homepage` / `documentation`（`.toml` 被扫，任何外网 URL 直接红）。
- [ ] **E0-2** `src/**` 的 `.rs` 里零外网 URL 字面量——**注释也算**（0.1）。Sharma 论文、参考实现的出处写进 `crates/bead-core/README.md`（`.md` 不扫）或 `tests/`/`fixtures/` 目录下。
- [ ] **E0-3** golden 导出落 `crates/bead-core/fixtures/golden/*.json`（目录名 `fixtures` 在两支审计里天然豁免）；golden JSON 里也不要写 `$schema`/`$id` URL——豁免是防御纵深，不是许可。

### denylist-audit（DL，对 O1）

- [ ] **DL-1** 置信度命名 `confidence`；禁止 `score` / `match_score` / `PixelArtScore` 等任何拆词后含 `score` 的标识符（0.2 的拆词规则）。
- [ ] **DL-2** `src/**/*.rs` 的字符串字面量里不含 `分数/得分/评分/计分/量表/常模/标准分/百分位`（CJK 子串匹配：`百分数` 也命中 `分数`）。错误信息、日志文案都算字符串字面量。
- [ ] **DL-3** 想引用这些词的测试放 `crates/bead-core/tests/`（路径豁免）；**`src/` 内联 `#[cfg(test)]` 不豁免**，别放那里。

---

## 2. O2 验收清单（`apps/bead`，分支 `cursor/bead-r1-shell-c441`）

### 锁文件与 gitignore（LK，= BD16 的可验证形式）

- [ ] **LK-1** `apps/bead` 落地的**同一个提交**里包含重生成的根 `pnpm-lock.yaml`（用钉死的 pnpm 10.15.0 生成）。验证：干净 checkout 上 `pnpm install --frozen-lockfile` 绿。分开提交 = 中间提交上全线 `just ci` 红，不接受。
- [ ] **LK-2** 同一提交 `.gitignore` 增加 `apps/bead/dist/`（现有条目只豁免 `apps/desktop/dist/`；`node_modules/`、`target/` 已通用，够了）。`dist` 永不提交。
- [ ] **LK-3** 锁文件 diff 只允许**新增** bead importer 与其新包；`@soul/desktop` 的既有解析不得被顺带升级/改写。验证：`just ci` 尾部的 `ui-lint`/`ui-test`（Soul 桌面壳）仍绿。
- [ ] **LK-4** 根 `package.json` 的 scripts 零改动（仍只 `--filter @soul/desktop`，BD4）；`pnpm-workspace.yaml` 零改动（`apps/*` 已覆盖，BD14 明令不加 `packages/*`）。

### e0-audit（E0，对 O2）

- [ ] **E0-4** `package.json` / `tsconfig.json` **不写 `$schema`**（schemastore URL 直接命中）。
- [ ] **E0-5** `src/**` 的 `ts/tsx/js/json/html/css` 零外网 URL——**与 `src` 并置的 `*.test.ts(x)` 一样被扫**（只有目录名字面为 `tests`/`fixtures` 的树豁免）；注释也算。
- [ ] **E0-6** `index.html` 无 CDN/字体外链；CSS 无 `@import url(http…)`。
- [ ] **E0-7** `apps/bead/src/schema/`（B07 预留）若建占位，schema 文件不写 `$id`/`$schema` URL。
- [ ] **E0-8** vite 配置钉 `port: 1520, strictPort: true`（Soul 钉死 1420；这是共存条款不是风格条款）。

### no-egress 源码断言（NE）

- [ ] **NE-1** 落一个源码断言测试（模式照抄 `apps/desktop/src/contract.test.ts` 的思路）：`apps/bead/src/**` 不得出现 `fetch(`、`XMLHttpRequest`、`WebSocket`、`sendBeacon`、外网 URL。这是「图纸默认不出网」（WP-B09 红线）的可执行形式，也是 e0 绿的第二道保险。

### 交付验证（对 O2）

- [ ] **V-O2** `pnpm --filter @bead/app lint && pnpm --filter @bead/app test` 绿，**且**在仓库根跑一次 `just ci` 绿——后者是共存回归的唯一入口，第一次实测就在 O2 这个 PR 上。

---

## 3. 每个实现 PR 的固定验证四条（O1、O2、以及之后所有实现席）

PR 描述里逐条记录命令与结果（hosted runner 空跑期间，这份记录**就是**门禁）：

1. 包内测试绿（O1：`cargo test --manifest-path crates/bead-core/Cargo.toml`；O2：`pnpm --filter @bead/app test`）。
2. 仓库根 `just ci` 绿（共存回归）。
3. `cargo run -p xtask -- e0-audit` 输出 `clean`。
4. `cargo run -p xtask -- denylist-audit` 输出 `clean`。

3、4 已含在 `just ci` 里，单列是因为它们是 bead 最容易踩的两个雷（§0.1/§0.2），红了要能单独定位。

---

## 4. 推荐的隔离 bead workflow（不改 Soul push 过滤）

### 4.1 不变量（CI，ROUND 2 按此审 workflow PR）

- [ ] **CI-1** 新建 `.github/workflows/bead.yml`；**`ci.yml` 零字节 diff**（不改 push 过滤、不加 paths、不加 job）。
- [ ] **CI-2** bead workflow 的 `push.branches` 只有 `cursor/bead*`——该模式永远匹配不到 `main` 与 `cursor/soul-goal1-7b1c`，反向不干扰成立。
- [ ] **CI-3** **不加 `pull_request` 触发**，理由与 `ci.yml` 头注相同：GitHub 对整个 PR 而非最新提交求值 paths；push 事件已在 PR 页面显示为 checks。
- [ ] **CI-4** `concurrency.group` 含 `github.workflow`，且 workflow `name` 与 Soul 的 `CI` 不同——两条流水线互不 cancel。
- [ ] **CI-5** paths 过滤**收窄到会改变测试结果的文件**：`apps/bead/**`、`crates/bead-core/**`、`pnpm-lock.yaml`、workflow 自身。**刻意偏离 F1 地图 §4 B10.1 的 `docs/bead/**`**：docs-only push 触发 CI 正是 `ci.yml` 头注里花钱买过的教训（`paths-ignore: docs/**`），本线 review 文档提交频繁，不重蹈。
- [ ] **CI-6** 禁止从 bead 分支对 Soul `CI` workflow 做 `workflow_dispatch`（0.5：job 守卫放行 dispatch，会真跑五门、真烧分钟数）。
- [ ] **CI-7** Billing 修好前**禁止 empty-commit 试探 runner**；workflow 落地即完成任务，绿不绿以本机四条记录（§3）为准。

### 4.2 参考实现（示意，实现席可微调步骤但不得违反 CI-1…CI-7）

```yaml
name: bead

on:
  push:
    branches:
      - "cursor/bead*"
    paths:
      - "apps/bead/**"
      - "crates/bead-core/**"
      - "pnpm-lock.yaml"
      - ".github/workflows/bead.yml"
  workflow_dispatch:

env:
  CARGO_TERM_COLOR: always
  CARGO_NET_RETRY: 3

concurrency:
  group: ${{ github.workflow }}-${{ github.ref_name }}
  cancel-in-progress: true

jobs:
  bead-core:
    name: bead-core (fmt, clippy, test)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.83
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
        with:
          key: bead-core
          workspaces: crates/bead-core
      - run: cargo fmt --manifest-path crates/bead-core/Cargo.toml --check
      - run: cargo clippy --manifest-path crates/bead-core/Cargo.toml --all-targets -- -D warnings
      - run: cargo test --manifest-path crates/bead-core/Cargo.toml

  bead-ui:
    name: bead-ui (lint, test)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
      - run: pnpm install --frozen-lockfile
      - run: pnpm --filter @bead/app lint
      - run: pnpm --filter @bead/app test

  # 两个最易踩的跨界雷（§0.1/§0.2），在 bead 自己的流水线里再断言一次。
  # `-p xtask` 只编译根 workspace 里的 xtask 及其依赖，不碰 soul 业务 crate 的测试。
  coexist:
    name: coexist (e0-audit, denylist-audit)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.83
      - uses: Swatinem/rust-cache@v2
        with:
          key: coexist
      - run: cargo run -p xtask -- e0-audit
      - run: cargo run -p xtask -- denylist-audit
```

落点归属：`bead.yml` 属 WP-B10 实现工作，由实现席在 O1/O2 合入后落地（workflow 引用的 manifest 与 filter 目标要先存在，否则 push 一次红一次）。本文只钉不变量与参考形状。

---

## 5. 隔离规则汇总（一句话版，给父代理与后续席位引用）

| # | 规则 | 出处 |
|---|------|------|
| I-1 | `ci.yml` 零字节改动；bead 走独立 `bead.yml`，branches 只匹配 `cursor/bead*` | WP-B10、CI-1/CI-2 |
| I-2 | bead-core 永不进根 `Cargo.toml` members；自带 `[workspace]` + 自带 Cargo.lock | BD3、WS-1/WS-2 |
| I-3 | bead-core 依赖近零且逐个书面说明——它在 cargo-deny 与 e0 依赖图之外 | WS-3 |
| I-4 | bead 全树（含注释、含并置测试文件）零外网 URL 字面量；出处只进 `.md` 或 `fixtures/`/`tests/` | BD15、E0-1…E0-8 |
| I-5 | `crates/**/*.rs` 禁 denylist 词；`confidence` 不叫 `score`；例外只进 `tests/` | BD15、DL-1…DL-3 |
| I-6 | `apps/bead` 落地提交 = 代码 + 重生成 `pnpm-lock.yaml` + `.gitignore` 加 `apps/bead/dist/`，三合一 | BD16、LK-1/LK-2 |
| I-7 | 每个实现 PR 固定四条验证记录；hosted 空跑期间记录即门禁；禁 empty-commit 试探、禁从 bead ref dispatch Soul CI | §3、CI-6/CI-7 |

## 6. 禁区自查

本审查只新增 `docs/bead/reviews/round1-cicd.md`。未触碰：`.github/workflows/ci.yml`、`docs/PRODUCT_LOCK.md`、`docs/FORMAL_WORK_PROMPT.md`、`docs/STATUS.md` 的 Soul 事实、`apps/desktop/**`、`crates/soul-*/**`、根 `Cargo.toml`、`deny.toml`、`justfile`、根 `package.json`。无产品代码，无 workflow 文件落地（那是 WP-B10 实现席的活）。
