MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Round 1 自动化探针与基准报告

## 总结

- `cargo test --workspace` 成功：共 **249** 个测试通过，0 失败，0 ignored。
- `matrix` example 可用并成功运行：输出 21 个夹具，列出 4 个 T4/T4D 分歧。
- workspace 只有 `soul-algo-tie`、`soul-algo-trait` 两个成员；二者均无外部依赖、网络依赖或 path 依赖。
- 两个 crate 均在源码和 manifest 层禁止 `unsafe`，未发现 `unsafe` 代码。
- 未发现产品 crate 读取 `SystemTime`、`Utc::now`、`Local::now`、`Instant::now` 等墙钟；时间由调用方以 `as_of_unix` 注入。
- 未发现实际 `#[ignore]`、`todo!`、`unimplemented!`、`TODO` 或 `FIXME`。

## 命令与退出码

| 命令 | 退出码 | 摘要 |
|---|---:|---|
| `cargo test --workspace` | 0 | 249 passed；0 failed；0 ignored；包含 248 个 `#[test]` 和 1 个 doc-test |
| `cargo run -p soul-algo-tie --example matrix` | 0 | 成功生成固定 `as_of = 1787580000` 的完整矩阵 |
| `cargo metadata --locked --offline --no-deps --format-version 1` | 0 | 离线解析成功；两个 package 的 `dependencies` 均为空 |
| `rustc --version --verbose && cargo --version --verbose` | 0 | `rustc 1.83.0`，`cargo 1.83.0`，与 pin 一致 |
| `git status --short`（生成报告前） | 0 | 无产品代码改动 |

## 测试统计

`cargo test --workspace` 的通过数分解：

- `soul-algo-tie`：60 个单元测试 + 74 个 integration tests = 134。
- `soul-algo-trait`：0 个单元测试 + 114 个 integration tests = 114。
- doc-tests：`soul-algo-tie` 1 个，`soul-algo-trait` 0 个。
- 合计：**249 passed，0 failed，0 ignored**。

静态扫描 `crates/**/*.rs`：

- 实际 `#[ignore]` 属性：0。
- `todo!` / `unimplemented!`：0。
- `TODO` / `FIXME`：0。

## Toolchain、workspace 与依赖

- `rust-toolchain.toml` 只固定 `channel = "1.83.0"`。
- 根 `Cargo.toml` 使用 resolver 2、edition 2021、MSRV 1.83。
- members：
  - `crates/soul-algo-tie`
  - `crates/soul-algo-trait`
- 两个成员的 `[dependencies]` 均为空，`Cargo.lock` 只包含这两个本地 package。
- `cargo metadata --locked --offline` 成功，因此当前 workspace 构建/测试不需要下载网络依赖。

## unsafe 检查

- `crates/soul-algo-tie/src/lib.rs` 与 `crates/soul-algo-trait/src/lib.rs` 均有 `#![forbid(unsafe_code)]`。
- 两个 crate 的 `Cargo.toml` 也设置 `unsafe_code = "forbid"`。
- 除禁止规则及说明文字外，未发现 `unsafe` 使用。

## 墙钟检查

- 对 `crates/**/*.rs` 扫描 `SystemTime`、`Utc::now`、`Local::now`、`now_utc`、`Instant::now`、`std::time`、`chrono` 等模式，未发现读取墙钟的代码。
- `SystemTime` 仅出现在“不读取墙钟”的文档说明中。
- `soul-algo-tie` 的评分入口由调用方传入 `as_of_unix`；matrix 使用固定时间，测试可复现。

## Matrix 摘要

- example 成功输出 21 个夹具。
- T4 与默认 T4D 在 4 个夹具上分歧：
  - `group_only_50`
  - `group_heavy_plus_one_direct_each_way`
  - `group_heavy_plus_three_directs`
  - `group_heavy_plus_directs_one_way`
- 180/360 天边界在输出中按闭区间触发降档。

## 不完善与风险

1. **群消息刷新关系新鲜度。** `dormant_direct_group_ping_yesterday` 中，一对一往来已停 300 天，但昨天一条群消息使 T4D 保持 Strong。`roundx_opus_a.rs` 还用通过中的测试固定了“一条收到的群消息可让任意久远的私聊历史保持 Strong”。若产品语义要求“一对一新鲜度”，这是当前主要风险。
2. **测试文件头注释已过时。** `crates/soul-algo-tie/tests/roundx_opus_a.rs` 第 44–48 行声称存在 `#[ignore]` 的 `bug_*` 测试，并建议用 `--ignored` 查看开放缺陷；当前实际没有 ignored 属性，也没有 `bug_*` 测试函数。该注释会误导缺陷盘点。
3. **全绿不等于无已知行为缺口。** suite 中有若干通过测试明确用于固定已知/历史缺陷行为，例如上述群消息 recency 语义以及 rollback/oracle 的缺陷基线。
4. **时间正确性依赖调用方。** crate 自身不读墙钟，有利于确定性；但生产调用方必须为整次 rebuild 提供一致且足够新的 `as_of_unix`，否则静默产生过旧或按 peer 偏移的结果。
5. **工具链维护风险。** 工具链精确固定在 2024-11 发布的 Rust 1.83.0；结果可复现，但应由上层维护流程确认是否需要定期升级安全/编译器补丁。
