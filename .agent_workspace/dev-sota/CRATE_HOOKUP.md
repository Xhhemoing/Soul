MODEL: gpt-5.6-sol-xhigh-fast

# 新增 workspace crate 工装接线探针

## 结论

当前 checkout 确为 `agent/dev-sota`，根 workspace 现有 13 个 member，尚无
`soul-draft` / `soul-fileplan`。新增后不需要手工维护 e0-audit、rustfmt、clippy
的 crate 清单：它们都从根 workspace 或 `crates/` 目录动态取范围。

真正容易踩红的点有三个：

1. 根 `Cargo.toml` 必须同时登记 `members` 和带 `version = "0.1.0"` 的
   `[workspace.dependencies]` 路径依赖；成员内部一律写 `{ workspace = true }`。
2. `soul-draft` 不得直接依赖 `reqwest` / `hyper` 等 HTTP client。需要 E1 时只依赖
   `soul-egress`；`soul-fileplan` 不应依赖 `soul-egress`，更不能持有 HTTP client。
3. 两个 path package 会进入 `Cargo.lock`。保留现有 lock 后跑定向
   `cargo check -p soul-draft -p soul-fileplan` 即可增量更新；不要先
   `cargo generate-lockfile`，后者会重解整棵第三方图，容易制造无关版本漂移。

## 必须修改的清单

### 1. 根 `Cargo.toml`

在 `[workspace].members` 增加：

```toml
  "crates/soul-draft",
  "crates/soul-fileplan",
```

在 `[workspace.dependencies]` 的本地 crate 区增加：

```toml
soul-draft = { path = "crates/soul-draft", version = "0.1.0" }
soul-fileplan = { path = "crates/soul-fileplan", version = "0.1.0" }
```

这里的 `version` 虽对 path 定位是冗余的，但根文件第 59–60 行明确说明它用于保持
cargo-deny 的 wildcard 检查有意义，不能只写裸 `path`。

若新 crate 引入现有列表之外的第三方 crate，还必须先在根
`[workspace.dependencies]` 统一声明版本，再由成员使用 `{ workspace = true }`。
本次接线本身不需要新增第三方依赖。

### 2. `crates/soul-draft/Cargo.toml`

包元数据和普通领域 crate 的布局仿 `crates/soul-profile/Cargo.toml`：

```toml
[package]
name = "soul-draft"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish = false
description = "..."

[dependencies]
# 仅列实现实际使用的依赖，全部 `{ workspace = true }`

[dev-dependencies]
# 测试工具同样走 workspace
```

原因：`soul-profile` 是当前最接近的“领域库 + policy 边界 + workspace 统一依赖 +
真实 store/testkit 仅作 dev-dependency”模板。起草若要走 E1，HTTP 接线规则则仿
`soulcore`：只声明

```toml
soul-egress = { workspace = true }
```

绝不能照抄 `soul-egress` 自身的 `reqwest` 声明。`soul-egress` 是全 workspace
唯一允许直接持有 HTTP client 的 gateway。`soul-draft -> soul-egress -> reqwest`
会被 e0-audit / cargo-deny 识别为获准路径；`soul-draft -> reqwest` 会失败。

### 3. `crates/soul-fileplan/Cargo.toml`

包元数据和平台能力边界的布局仿 `crates/soul-collect/Cargo.toml`：同样继承全部
workspace package 字段、只声明实际使用的 workspace 依赖，并把 fake/test store
等测试设施放入 `[dev-dependencies]`。

`soul-collect` 是更合适的模板，因为它把平台来源隔在 trait 后，并让 policy
守门；文件计划也应把“授权根内的只读枚举”隔在窄接口后。只仿 manifest 结构，
不要机械复制其业务依赖。

该 crate 的约束更严：

- 不加 `soul-egress`；
- 不加任何 HTTP client；
- 不增加写文件库或写 API；
- 第三方依赖若确有必要，先进入根 `[workspace.dependencies]`，并确认许可证在
  `deny.toml` 允许范围内。

### 4. `crates/soulcore/Cargo.toml`

在 `[dependencies]` 增加：

```toml
soul-draft = { workspace = true }
soul-fileplan = { workspace = true }
```

这是编排层引用两个新库所需的唯一 manifest 改动。对应命令模块已经在
`crates/soulcore/src/commands/mod.rs` 的规划注释中预留，但目前尚无文件和
`pub mod` 声明。

### 5. `deny.toml`

**正常情况下不改。**

- `[graph]` 没有固定产品 crate allowlist；根 workspace 增加 member 后，
  cargo-deny 会解析新 member。
- `[bans]` 已将 `reqwest` 限定为只能由 `soul-egress` wrapper 引入，并封禁其他
  HTTP client 与 Tauri HTTP/updater 插件。
- `soul-draft` 通过 `soul-egress` 使用既有 E1 gateway 不需要新增 wrapper。
- `soul-fileplan` 不应有任何网络依赖，因此也不应获得例外。

只有确实引入新第三方许可证、registry/source 或新的安全例外时才考虑改
`deny.toml`；不能为了让新 crate 过检查而扩宽 HTTP wrapper/skip。

## 自动覆盖范围

### xtask `e0-audit`

会自动包含两个新 member 作为成品根，无需改 `crates/xtask/src/egress.rs`。

证据：

- `audit()` 对根 `Cargo.toml` 执行 `cargo metadata`；
- `audit_dependencies_from_roots()` 从 `metadata.workspace_members` 取根；
- 唯一排除的根是常量 `TEST_TOOLING = ["soul-testkit", "xtask"]`；
- 只遍历 normal/build 边，dev-only 的 `soul-testkit` 不进入成品闭包；
- `audit_gateway()` 还会检查除 `soul-egress` 外是否有 member 直接声明 HTTP client；
- URL 扫描直接遍历 `crates/` 和 `apps/`，所以新 crate 源码即使尚未成为 member
  也会被扫描（`tests/` 除外）。

因此：

- 两个新 crate 一旦成为 member，会自动成为第 14、15 个 workspace package，
  并自动成为 e0 成品根；
- `soul-draft -> soul-egress` 可行；
- 两个新 crate 任何一个直接声明 `reqwest` / `hyper` 等都会被报
  `SecondHolder` 或 banned dependency path；
- 源码中的非白名单 `http://` / `https://` 字面量也会报错。

补充：`denylist-audit` 同样直接遍历 `crates/**/*.rs`，新 crate 的非测试 Rust
源码也自动受临床词汇/数字评分门禁约束。

### rustfmt / clippy

会自动覆盖，无需改 `justfile` 或 CI：

- rustfmt：`cargo fmt --all -- --check`，`--all` 覆盖根 workspace 全部 member；
- clippy：`cargo clippy --workspace --all-targets --all-features -- -D warnings`；
- 根 `rustfmt.toml` 自动作用于两个新 crate 的 Rust 源码。

这里不包含独立 workspace `apps/desktop/src-tauri`，但两个新 crate 位于根
`crates/` 并登记为根 member，所以不受该已知缺口影响。

## `Cargo.lock` 的处理

需要提交更新后的根 `Cargo.lock`。即使不新增第三方 crate，workspace 的本地
package 也会作为 `[[package]]` 写入 lock，并记录其依赖名；现有
`soul-collect`、`soul-profile` 等均已有对应 lock 项。

推荐顺序：

```bash
cargo check -p soul-draft -p soul-fileplan
cargo check -p soulcore
```

第一条在保留既有 `Cargo.lock` 的前提下完成解析并增量写入两个 path package；
第二条确认编排层接线。只跑 `cargo check -p soul-draft` 时 Cargo 通常也会为整个
workspace 解 lock，但显式列出两个新包更清楚，也同时证明两包可编译。

不需要、也不建议先跑：

```bash
cargo generate-lockfile
```

它会从头解析整个 lock；根第三方版本多数是兼容范围而非 `=` 精确版本，重生成
可能更新与新 crate 无关的 registry package。最终可再用 `--locked` 复查，确保
报告/提交遗漏 lock 时直接失败。

## `just` 与 cargo 等价命令

仓库有 `justfile`，但当前机器没有 `just`（探测输出为 `just: not found`）。
可直接运行其等价命令：

```bash
# 定向编译
cargo check -p soul-draft -p soul-fileplan
cargo check -p soulcore

# 格式与 lint
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# 守卫
cargo run -q -p xtask -- e0-audit
cargo run -q -p xtask -- denylist-audit
cargo deny check
```

`cargo deny check` 仍要求机器已安装兼容的 cargo-deny；CI 使用预编译 0.18.6。
按要求本探针没有运行全仓 `cargo test`。

## 实现者落地新 crate 的最小 diff 清单

仅让两个库成为可编译的 workspace member，并让 `soulcore` 能依赖它们，最小文件
集合是：

```text
Cargo.toml
Cargo.lock
crates/soul-draft/Cargo.toml
crates/soul-draft/src/lib.rs
crates/soul-fileplan/Cargo.toml
crates/soul-fileplan/src/lib.rs
crates/soulcore/Cargo.toml
```

若同一 diff 还要把功能接入已预留的 soulcore 命令面，则再加：

```text
crates/soulcore/src/commands/draft.rs
crates/soulcore/src/commands/fileplan.rs
crates/soulcore/src/commands/mod.rs
```

功能测试文件、fixture 与 UI 接线取决于 WP10/WP11 的实现，不属于“crate 工装
hookup”的机械最小集。

## 命令输出摘要

```text
$ rustc --version
rustc 1.83.0 (90b35a623 2024-11-26)

$ cargo metadata --no-deps --format-version 1
workspace_root: /workspace
target_directory: /workspace/target
workspace members: 13
soul-schema, soul-store-api, soul-store, soul-testkit, soul-policy,
soul-egress, soul-graph, soul-import, xtask, soulcore, soul-collect,
soul-memory, soul-profile

$ ls -1 crates
13 个目录；不存在 soul-draft、soul-fileplan

$ command -v just / just --version 探测
just: not found

$ .git/HEAD
ref: refs/heads/agent/dev-sota
```

探针仅运行只读/元数据命令；未改业务代码，未运行全仓测试，未 commit。
