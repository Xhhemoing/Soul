# STATUS

单一事实来源。每个子代理完工必须更新本文件。

## 当前里程碑

**`PLAN_FROZEN`**。Goal 1 已开工：分支 `cursor/soul-goal1-7b1c`。文档 PR `#1` 不夹带应用代码。Goal 2 在 Goal 1 关闭前不要启动。

## 进度

| 项 | 状态 |
|---|---|
| R1 双模型扫描 | 完成，曾 `PLAN_BLOCKED` |
| R2 规范修复 | 完成并落盘 |
| R3 复核冻结 | 完成。仲裁见 `docs/scan-rounds/R3-SYNTHESIS.md`，结论 `PLAN_FROZEN` |
| Goal 1 规划 | 完成。DAG 见 `docs/GOAL1_PLAN.md` |
| WP01 骨架 | 完成。见下节 |
| v0.1 其余 WP | 未开始 |

## WP01 完成情况

`GOAL1_PLAN.md` 的五条完成定义在本机 Linux 上满足，`just ci` 绿。GitHub Actions 尚未在本推送上跑完；Windows SQLCipher 冒烟以 `test-windows` job 为准。

| 完成定义 | 证据 |
|---|---|
| `just ci` 本地绿 | `lint / schema / e0 / denylist / fixtures-verify / test / ui-*` 全过；`just ci-full` 另跑 `cargo deny check`，四项 ok |
| 九份 `$ref` 接到 `_defs`，lock 已钉 | `crates/soul-schema/tests/schema_wiring.rs`：记录型 retriever 观察到九份编译时都拉取了 `_defs`，且扣掉 `_defs` 后九份全部编译失败；`docs/schemas/schemas.lock.json` 钉 11 份 sha256 |
| crash harness 绿、leakage 对 Unicode fixture 全过 | `crash_harness_demo.rs` 子进程真 abort，已提交行存活、未提交行丢失，并有「不武装 fail point 就不死」的对照；`leakage_checker.rs` 7 项全过 |
| SECURITY 加密落地与实证一致 | `SECURITY.md`「加密落地」小节；`sqlcipher_smoke.rs` 在 ubuntu 与 windows-latest 都跑 |
| xtask 断言器非空转 | `crates/xtask/tests/self_test.rs` 17 项：URL 扫描器对合成的 `evil.example` 必红，denylist 对合成的 `score` 字段必红，schema-freeze 对改一个字节的副本必红，依赖遍历在把 `soul-testkit` 当成品根时确实找得到 `hyper` |

落地内容：workspace + 工具链钉版本、`crates/{soul-schema,soul-store-api,soul-testkit,xtask,soulcore}`、`fixtures/`、`deny.toml`、`justfile`、`.github/workflows/ci.yml`（lint / test-linux / test-windows）。无业务功能。

### WP01 的取舍与遗留

1. **加密路线：SQLCipher，未回退。** `rusqlite` 的 `bundled-sqlcipher-vendored-openssl` 在 Linux 编过并实证加密（cipher 4.5.7）。Windows job 跑同一测试，但 vendored OpenSSL 需要 perl 与 NASM；若 windows-latest 镜像日后去掉其一，WP02 需改用预编译 SQLCipher 或按 `SECURITY.md` 走回退门。
2. **`axis_id` 收紧为 uuid7。** 工作单要求「各 `*_id` 裸 string → uuid7」。`profile.trait_axes[].axis_id` 因此也是 uuid7；轴的人类可读名字放 `label`。WP03 建默认轴时要用固定 UUID 常量。
3. **`evidence_band` / `strength` / `self_trait_band` 用 `allOf` 接 `_defs`。** `_defs` 的 `evidenceBand` 含 `none`，而这三处内联枚举原本只有弱/中/强。直接 `$ref` 会放松约束，所以写成 `allOf: [$ref evidenceBand, {enum: [weak, moderate, strong]}]`：既真的引用了 `_defs`，又保持只收紧。
4. **`e0-audit` 的 dev 豁免是按「成品根」实现的。** 遍历只从 `soul-testkit` 与 `xtask` 以外的 workspace 成员出发，走 normal/build 边。`soul-testkit` 因此可以持有 axum/hyper，但只能被 dev 边引用。`crates/xtask/**` 同时豁免源码 URL 扫描与 denylist 扫描——它必须把这些字面量写出来才能执行禁令。
5. **泄漏检查器的 `>=8` 下限是显式的。** 短于 8 个 scalar 的第三人短句（如「好的没问题」）默认规则抓不到，只能靠姓名/账号规则。fixture 与测试把这一点写死并演示了把阈值调到 4 就能抓到，避免后续 WP 误以为是缺陷。
6. **`jsonschema` 钉在 0.26.2，若干传递依赖在 `Cargo.lock` 里降级钉死**（`idna_adapter` 1.2.0、`uuid` 1.11.1、`zeroize` 1.8.1、`getrandom` 0.3.1）。更新的版本要求 Rust 1.85/1.88 或 edition 2024，本仓库钉 1.83。升级工具链时这几条一起解。
7. **`just` 钉 1.46.0**（1.83 能编的最后一版）。justfile 每条 recipe 的注释里写了等价 cargo 命令，不装 `just` 也能跑。
8. **`cargo-deny` 用预编译二进制，钉 0.18.6。** 0.16.x 解析不了当前 advisory 数据库的 CVSS 4.0；能在 1.83 上从源码编出来的版本都太老。CI 直接下载 musl 二进制。
9. **UI 面全是占位。** `package.json` / `pnpm-workspace.yaml` / `just ui-*` 空转退出 0，等 WP09 建 `apps/desktop`。

## 阻塞

无。

## 下一步

批 2：WP02 数据面（真 SQLCipher 存储、遗忘、研究预览不写文件）∥ WP08 权限面（策略、审计链、net_guard、redactor、HITL 令牌）。两者都对着 `soul-store-api` 的 trait 与 `conformance::run_conformance` 写，新后端必须跑通同一套。
