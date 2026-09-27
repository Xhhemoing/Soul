MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Round 2 / gpt-sol-a：D52 常量钉死缺失与墙钟探针

审计时间：2026-08-26 UTC  
范围：当前 `/workspace` 的根 workspace；只扫描 `crates/` 与 `docs/`，不把 `.agent_workspace/` 内历史副本混入结果。  
约束：未修改产品代码，未执行 `git checkout` / `git commit` / `git push`。

## 结论

1. **D52 要求的跨 crate 相等性测试不存在。** 两个 crate 各自把自己的常量钉到字面量 `180`，但没有测试直接比较两侧常量；因此任一侧单独漂移时，现有两侧测试都可能继续通过。
2. `crates/` 中有 **56 个** `180` / `360` token，分布在 **43 个匹配行、13 个文件**；`docs/` 中有 **22 个** token，分布在 **12 个匹配行、5 个文件**。合计 **78 个 token / 55 个匹配行**。
3. 真正构成 D52 双源的产品定义是：
   - `crates/soul-algo-tie/src/constants.rs:51`：`DEMOTE_AFTER_SILENT_DAYS: i64 = 180`
   - `crates/soul-algo-trait/src/a2.rs:91`：`DORMANT_AFTER_DAYS: i64 = 180`
   `DORMANT_NOTE_DAYS` 在 `crates/` 中为 **0 命中**。
4. 指定墙钟扫描只命中两处“禁止读取 `SystemTime`”的文档注释；**可执行代码命中 0**。`Utc::now`、`Local::now`、`unix_epoch` 均为 0；补扫大写 `UNIX_EPOCH` 也为 0。
5. `cargo test --workspace` 退出码 **0**，仍为 **249 passed / 0 failed / 0 ignored**。

## 1. `180` / `360` 全量清单

扫描：

```text
rg -o -n '\b(180|360)\b' crates docs
```

下列清单保留同一行的重复 token；`×2` / `×3` 表示该行确有多个相同 token。

### `crates/`：56 token / 43 行 / 13 文件

| 文件 | 全部命中（行号：token） | 性质 |
|---|---|---|
| `crates/soul-algo-trait/tests/a2_render.rs` | `303: 180`; `306: 180` | trait 侧单独钉字面量与边界夹具 |
| `crates/soul-algo-trait/src/a2.rs` | `91: 180` | **D52 敏感：独立产品常量定义** |
| `crates/soul-algo-trait/src/fixtures.rs` | `150: 180`; `176: 180` | 计数/时间夹具 |
| `crates/soul-algo-tie/tests/explain_zh.rs` | `236: 180` | 文案断言 |
| `crates/soul-algo-tie/tests/as_of_discipline.rs` | `38: 180, 360` | 边界夹具 |
| `crates/soul-algo-tie/tests/direct_gate.rs` | `158: 180`; `224: 180, 360` | 性质/边界测试 |
| `crates/soul-algo-tie/tests/roundx_opus_a.rs` | `10: 180, 360`; `432: 180, 360`; `440: 180`; `442: 360` | 注释与边界测试 |
| `crates/soul-algo-tie/src/constants.rs` | `12: 180, 360`; `51: 180`; `54: 360`; `75: 180, 360`; `76: 180`; `77: 360`; `78: 180` | **权威产品定义及其本 crate 单测** |
| `crates/soul-algo-tie/src/tombstones.rs` | `110: 360`; `111: 180` | `#[cfg(test)]` 的落选 T3R 墓碑 |
| `crates/soul-algo-tie/src/t4.rs` | `11: 360, 180`; `196: 180`; `198: 360`; `215: 180, 360` | 模块说明与单测；产品逻辑读取常量 |
| `crates/soul-algo-tie/src/recency.rs` | `6: 360`; `7: 180`; `12: 180, 360`; `59: 180`; `61: 360`; `68: 180`; `69: 360`; `76: 180, 360`; `86: 180×2`; `87: 180`; `88: 360×2` | 模块说明与单测；产品逻辑读取常量 |
| `crates/soul-algo-tie/src/t4d.rs` | `12: 360, 180` | 模块说明；产品逻辑读取常量 |
| `crates/soul-algo-tie/src/testing/mod.rs` | `279: 180`; `281: 180`; `299: 360`; `301: 360`; `367: 180` | 固定夹具数据 |

关键区分：

- `soul-algo-tie` 的实际降档逻辑 `recency.rs:25-45` 使用 `DEMOTE_AFTER_SILENT_DAYS` / `WEAK_AFTER_SILENT_DAYS`，没有在产品分支再次写 `180` / `360`。
- `tombstones.rs` 由 `lib.rs:80-81` 的 `#[cfg(test)]` 限定，不在产品路径。
- 测试、夹具、说明文字中的边界数字很多，但它们不能替代 D52 要求的“两侧常量直接相等”断言。

### `docs/`：22 token / 12 行 / 5 文件

| 文件 | 全部命中（行号：token） |
|---|---|
| `docs/DECISIONS.md` | `71: 180` |
| `docs/algorithms/REJECTED.md` | `37: 180, 360`; `86: 180, 360` |
| `docs/algorithms/R2-SYNTHESIS.md` | `5: 180, 360` |
| `docs/algorithms/COPY_ZH.md` | `27: 360`; `42: 360`; `73: 180×2` |
| `docs/algorithms/DECISION.md` | `35: 180, 360`; `39: 180, 360`; `52: 180×3`; `53: 360×2`; `69: 360, 180` |

`docs/schemas/schemas.lock.json` 中哈希片段里的 `180` 没有被计为数字字面量：它与十六进制字符相连，不匹配上述 token 边界。

## 2. D52 跨 crate 常量相等测试

判定：**不存在。**

证据：

- 根 `Cargo.toml` 的 workspace 只有 `crates/soul-algo-tie` 与 `crates/soul-algo-trait` 两个成员。
- 两个 crate 的 manifest 均无普通依赖；`soul-algo-tie` 的 `[dev-dependencies]` 也为空。
- 在 `soul-algo-trait` 中搜索 `soul_algo_tie|soul-algo-tie` 为 0 命中；反向搜索 `soul_algo_trait|soul-algo-trait` 也为 0 命中。
- 在 tie crate 中搜索 `DORMANT_AFTER_DAYS` 为 0 命中；在 trait crate 中搜索 `DEMOTE_ONE_BAND_DAYS|DEMOTE_AFTER_SILENT_DAYS` 为 0 命中。
- trait 侧只有 `tests/a2_render.rs:303`：

  ```rust
  assert_eq!(DORMANT_AFTER_DAYS, 180);
  ```

- tie 侧只有 `src/constants.rs:76` 的独立字面量钉死：

  ```rust
  assert_eq!(DEMOTE_AFTER_SILENT_DAYS, 180);
  ```

这两个断言是“分别等于 `180`”，不是“两个 crate 常量相等”。它们当前传递性地给出同值，但没有建立 D52 所要求的跨 crate 回归门：将来只修改一侧常量及其本地断言时，另一侧不会让测试失败。

规范证据是 `docs/DECISIONS.md:63`：两侧天数常量应“用工作区测试钉相等”。`docs/algorithms/DECISION.md:54` 与 `COPY_ZH.md:73` 进一步要求 `DORMANT_NOTE_DAYS = DEMOTE_ONE_BAND_DAYS`；但实现侧没有名为 `DORMANT_NOTE_DAYS` 的常量。

若后续获准修复，最低限度门禁应直接比较两侧导出的常量，而不是再次比较字面量；本轮按指令不改代码。

## 3. 墙钟探针

指定扫描：

```text
rg 'SystemTime|Utc::now|Local::now|unix_epoch' crates
```

全部结果只有：

- `crates/soul-algo-tie/tests/as_of_discipline.rs:5`：注释说明 crate 不读 `SystemTime`
- `crates/soul-algo-tie/src/lib.rs:55`：文档说明 crate 不调用 `SystemTime`

逐项统计：

| 模式 | 命中 | 可执行代码命中 |
|---|---:|---:|
| `SystemTime` | 2（均为否定性注释） | 0 |
| `Utc::now` | 0 | 0 |
| `Local::now` | 0 | 0 |
| `unix_epoch` | 0 | 0 |

补充探针：

- 大小写不敏感 `unix_epoch`：0
- `SystemTime::now|Utc::now|Local::now|UNIX_EPOCH`：0
- `std::time|Instant::now|now_utc|chrono|time::OffsetDateTime::now`：0

结论：当前两个算法 crate 没有发现墙钟读取路径；时间仍由调用方通过固定 `as_of` 数据注入。

## 4. Workspace 测试

执行：

```text
cargo test --workspace
```

结果：

- 退出码：`0`
- `soul-algo-tie`：60 个单元测试 + 74 个集成测试 = 134
- `soul-algo-trait`：0 个单元测试 + 114 个集成测试 = 114
- doc-tests：`soul-algo-tie` 1，`soul-algo-trait` 0
- 合计：**249 passed；0 failed；0 ignored；0 measured；0 filtered out**

全绿并不否定 D52 缺口：当前 249 个测试中没有任何一个测试能同时引用两侧常量。
