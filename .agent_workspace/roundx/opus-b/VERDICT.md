# Round X · opus-b · VERDICT

模型 slug：`claude-opus-5-thinking-high-fast`
范围：`crates/soul-algo-trait` @ `752cdec`（`cursor/algo-verify-opt-a073`）
命令：`cargo test -p soul-algo-trait --offline` → **114 passed / 0 failed**
（找到的树 108 passed；本轮新增 6）· `fmt --check` 干净 · `clippy --all-targets` 无 warning

## 判定

**PASS — 四条断言全部成立。不建议改判，不启动 DECISION 第 5 节回退链。**

| # | 断言 | 判定 | 主要证据 | 变异验证：改坏后有几个测试响 |
|---|---|---|---|---|
| A0 | 纠正锁成立，intake 不绕锁 | **确认** | `place()` 一处收口；`intake_matches_replay_on_every_fixture_and_mode` 在 2 种写入模式 × 15 夹具上钉住 intake 与 replay 等价 | 去掉 `place()` 的锁分支 → **11 个现有测试失败**，跨 3 个文件 |
| A1 | 默认 `TwoKindsAcrossDays`，`EMPTY_ON_V01` | **确认** | 常量、`Default` 实现、审计拼写、`v3` 标识四项齐；`EMPTY_ON_V01` 由 `a1_can_fire_on_v01_data` 从可达 `EvidenceKind` 推导而非写死 | 默认改回 `KindAndDay` → **12 个测试失败**（9 个是现有的） |
| A2 | 没有 `STRONG_MIN` | **确认** | `a2_render` 原样抄 `TieScore::band`；唯一的数字 `DORMANT_AFTER_DAYS=180` 只决定多一句描述，不碰档位 | 见下：现有守卫有一类绕过方式，已补 |
| A3 | 拒绝从正文推断 | **确认** | 两个入口无条件 `Err`，参数只借不读；`A3_ALGORITHM_ID` 以 `.rejected` 结尾并被登记 | 改成返回 `Ok(AxisState)` → 2 个现有测试失败 |

## 本轮新增（只加 `tests/`，未碰 `src/`）

两个文件、六个测试，各自对应一个现有 108 个测试全部放过的具体改动：

**`tests/roundx_a1_empty_on_v01.rs`（3）** — `EMPTY_ON_V01` 此前只有「数来源种类」的推导和 15 条手写夹具，两者之间没有接缝。v0.1 对一条轴只能写问卷和纠正两种行，空间可穷举：24 个符号上长度 1–4 的 **346 200** 条日志，逐条断言升档分支从不进入、`Strong` 必然带锁；另加 6 天 × 4 方向的 4096 条问卷日志覆盖「组数远超阈值」一侧。0.33 秒。

**`tests/roundx_a2_no_threshold.rs`（3）** — 现有守卫是对 `src/a2.rs` 搜六个字面量（都是 Goal 1 用过的名字），行为侧靠 `tie_score_matrix`，而那个矩阵在边界上很薄：15 个夹具的 `active_day_count` 里**没有 3**（`STRONG_MIN_ACTIVE_DAYS` 的原值），`direct_count` 只有 `None`/0/2/12/90，整个 `3..10` 窗口空着。两个绕过方式经实测**现有测试全绿**：

1. 改名后的门闩 `const THIN_DIRECT_RECORD: u32 = 10;`，`direct_count` 落在 `3..10` 时降档——半个 T4D，六个字面量一个不沾。
2. 档位反过来决定说哪句话：`band == Strong` 时不出「只在群聊里见过」。现有测试只检查出来的 bullet 带的档位对不对，从不检查出来的是不是同一批 bullet。

第二条是两者中更要紧的：它是「A2 不形成第二意见」被漏掉的那半个方向。

## 遗留给下一轮，不构成改判

- **`the_two_write_modes_do_differ_in_what_they_cite`** 是 Round 3 明确甩过来的题。同一份问卷填五次，冻结模式只引用第 5 行。我的判断是**不动**：方向/档位/锁三项一致，证据表五行俱在，档案页要展示「基于哪几次回答」可以按轴查证据表，这是取数问题，不值得动 Goal 1 的回归基线。
- **`a0_trusts_the_band_on_a_row_it_is_handed`** 记录的缺口仍然为真：A0 从不给 `EvidenceRef::band` 封顶，「只有纠正能到 Strong」是 v0.1 数据面的性质而非代码的性质。开始写行为行的版本必须二选一（非纠正行封在 Moderate，或启用 A1）。已有测试、已署名，不是本轮发现。
- A2 的六个字面量列表下次改 A2 时一并检讨。我加的是行为侧补充而不是往列表里塞更多名字——名字列表追不上重命名，而扫描挂在文件上的价值应当保留。

## 合规

未 `git commit` / `git push` / 开 PR。未改任何 `src/`（`TEST_LOG.txt` RUN 5 的 `git diff --stat -- crates/` 为空）。未写他人目录。离线、无新依赖、无 `unsafe`。

同一 checkout 里另一个 Round X 代理正在往 `crates/soul-algo-tie/tests/` 写文件，`cargo test --workspace` 因此可能红；那是它的包，`cargo test -p soul-algo-trait` 不构建它。
