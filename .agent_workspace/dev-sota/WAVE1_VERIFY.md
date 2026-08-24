MODEL: gpt-5.6-sol-xhigh-fast

# 波次 1（ST-00 + ST-03）收口验证清单

本清单只把根 `justfile` 与 `.github/workflows/ci.yml` 中的命令展开为无 `just` 环境可直接运行的 `cargo` / `pnpm` 等价命令。父代理应在 ST-00、ST-03 都合入后，从仓库根目录 `/workspace` 按顺序执行。根 workspace、Tauri 独立 workspace、前端三棵树必须分别验证；根 `cargo test --workspace` 不覆盖 `apps/desktop/src-tauri`。

## 0. 一次性准备

```bash
export CARGO_TERM_COLOR=always
export CARGO_NET_RETRY=3
rustup component add rustfmt clippy
pnpm install --frozen-lockfile
```

通过判据：

- 使用仓库钉住的 Rust 1.83，`rustfmt`、`clippy` 均安装成功。
- `pnpm install --frozen-lockfile` 退出码为 0，且没有因 lockfile 与 manifest 不一致而改写锁文件。

若环境没有 `cargo-deny`，按 CI 的固定版本安装预编译二进制（不要在 Rust 1.83 下从源码安装新版）：

```bash
set -euo pipefail
version=0.18.6
url="https://github.com/EmbarkStudios/cargo-deny/releases/download/${version}/cargo-deny-${version}-x86_64-unknown-linux-musl.tar.gz"
curl -fsSL "$url" | tar xz -C /tmp
export PATH="/tmp/cargo-deny-${version}-x86_64-unknown-linux-musl:$PATH"
```

通过判据：安装命令退出码为 0，随后 `cargo deny --version` 可执行并报告 0.18.6。

## 1. 根 workspace：等价于 `just ci`

### 1.1 格式

```bash
cargo fmt --all -- --check
```

通过判据：退出码为 0；所有 workspace Rust 文件均已符合 rustfmt，命令不要求写回格式。

### 1.2 Clippy

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

通过判据：退出码为 0；所有根 workspace 成员、target 和 feature 组合均无 Clippy 警告。以 recipe 实际命令中的 `--all-features` 为准。

### 1.3 冻结契约

```bash
cargo run -q -p xtask -- schema-freeze --check
```

通过判据：退出码为 0；冻结 schema 的摘要与 `docs/schemas/schemas.lock.json` 完全一致，ST-00 没有造成未批准的契约漂移。

### 1.4 E0 审计

```bash
cargo run -q -p xtask -- e0-audit
```

通过判据：退出码为 0；发布依赖图中没有被禁 HTTP client，URL 字面量均在允许范围内。

### 1.5 非临床词与数字评分审计

```bash
cargo run -q -p xtask -- denylist-audit
```

通过判据：退出码为 0；扫描面中没有被禁止的临床词汇或数字评分表达。

### 1.6 fixture 语料

```bash
cargo test -q -p soul-testkit --test fixture_corpus
```

通过判据：退出码为 0；fixture corpora 全部可解析，验收测试依赖的语料特征仍存在。

### 1.7 根 workspace 全测试

```bash
cargo test --workspace --all-targets
```

通过判据：退出码为 0；所有根 workspace 测试与 target 全绿。该命令应覆盖 ST-00 的新 crate/`ReasonCode` 接线以及 ST-03 的 `soulcore` 壳命令测试，但不覆盖 Tauri 独立 workspace。

### 1.8 前端静态检查

```bash
pnpm --filter @soul/desktop lint
```

通过判据：退出码为 0；该 package 的 `tsc --noEmit` 与 `eslint .` 均通过，包括新增 IPC 名单和设置页类型检查。

### 1.9 前端测试

```bash
pnpm --filter @soul/desktop test
```

通过判据：退出码为 0；Vitest 全绿，包括 TS/Rust 命令名单契约、设置页授权目录入口、无网络调用及诚实密钥文案断言。

## 2. 供应链检查：补齐 `just ci-full`

```bash
cargo deny check
```

通过判据：退出码为 0；第三方许可证、advisory 与 HTTP client 禁令全部通过。此项在 GitHub CI 的独立 `lint` job 中执行，不能因 `just ci` 本身未串入而省略。

## 3. Tauri 独立 workspace

### 3.1 编译检查

```bash
cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets
```

通过判据：退出码为 0；Tauri 壳全部 target 可编译，ST-03 新增 managed state、IPC wrapper 与 `soulcore` 接线类型一致。

### 3.2 壳测试

```bash
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets
```

通过判据：退出码为 0；独立 workspace 全部测试通过，特别包括单一开库调用点、同一 store 句柄、密钥回退描述、授权目录 IPC roundtrip、命令三方名单一致和薄 wrapper 门禁。

GitHub CI 在 `windows-latest` 上执行 3.2；Windows 是产品目标平台，也是该项最终通过判据。若在 Linux 本地复跑 3.1/3.2，需先具备 `justfile` 注释列出的 WebKit/AppIndicator/RSVG/Xdo/build-essential 系统库。不得用根 workspace 测试代替这两项，也不要在本波次运行 `tauri build`。

## 4. 收口总判据

以上所有命令均以退出码 0 结束，且：

1. 根 workspace、前端、Tauri 独立 workspace 三棵树没有任何一棵被漏测。
2. `cargo fmt --check`、Clippy `-D warnings`、三项 xtask 审计、fixture、根测试、前端 lint/test、`cargo deny`、desktop check/test 全绿。
3. Windows CI 的根测试与 desktop shell 测试全绿；Linux CI 的 `just ci` 与独立 cargo-deny job 全绿。
4. 不运行 `tauri build`，不把测试失败用跳过、忽略或缩小 target/feature 范围的方式掩盖。
