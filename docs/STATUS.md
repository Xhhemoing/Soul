# STATUS

单一事实来源。每个子代理完工必须更新本文件。

## 当前里程碑

**`PLAN_FROZEN`**。Goal 1 已开工：分支 `cursor/soul-goal1-7b1c`。文档 PR `#1` 不夹带应用代码。Goal 2 在 Goal 1 关闭前不要启动。批 1（WP01）与批 2（WP02 数据面 + WP08 权限面）已完成。Windows CI 在 schema freeze CRLF 修复后全绿。

## 进度

| 项 | 状态 |
|---|---|
| R1 双模型扫描 | 完成，曾 `PLAN_BLOCKED` |
| R2 规范修复 | 完成并落盘 |
| R3 复核冻结 | 完成。仲裁见 `docs/scan-rounds/R3-SYNTHESIS.md`，结论 `PLAN_FROZEN` |
| Goal 1 规划 | 完成。DAG 见 `docs/GOAL1_PLAN.md` |
| WP01 骨架 | 完成。见下节 |
| WP02 数据面 | 完成。见下节 |
| WP08 权限面 | 完成。见下节 |
| WP03 档案 | 完成。见下节 |
| WP04 自传记忆 | 完成。见下节 |
| v0.1 其余 WP | 未开始 |

## WP01 完成情况

`GOAL1_PLAN.md` 的五条完成定义在本机 Linux 上满足，`just ci` 绿。第一次 Windows CI 里 SQLCipher 冒烟已通过；失败原因是 schema freeze 把 CRLF 检出当成内容变更。哈希已改为按 LF 计算，并加了 `.gitattributes`。

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

## WP02 完成情况

真加密主库 `crates/soul-store` 落地，`just ci-full` 在本机 Linux 上绿（`lint / schema / e0 / denylist / fixtures-verify / test / ui-*` 加 `cargo deny check` 四项 ok）。GitHub Actions 未在本次推送上跑完；Windows 侧仍以 `test-windows` job 为准。

| 交付 | 证据 |
|---|---|
| 真库跑同一套契约 | `crates/soul-store/tests/conformance_real.rs`：`soul_store_api::conformance::run_conformance` 对 `SqlCipherStore` 全过（每个检查一个独立库文件），另测关库重开、错密钥打不开、事件过滤走 SQL |
| SQLCipher 而不是明文 SQLite | `SqlCipherStore::open` 读不到 `PRAGMA cipher_version` 就报错退出，不静默降级；feature 与 WP01 冒烟一致（`bundled-sqlcipher-vendored-openssl`） |
| AC-04 存储侧无明文残留 | `tests/no_plaintext_at_rest.rs`：三个 needle（ASCII 与中文）写进去后扫目录下**每个**文件（含 `-wal`/`-shm`）都搜不到；反向断言正文仍能从 `open()` 取回，避免「什么都没写」也能过；不带密钥的 `rusqlite` 连接连表都列不出来 |
| AC-15 遗忘 | `tests/forget.rs`：预览三个数字（3 个 blob / 2 把 CK / 1 条推断）来自真查询——同一个库里第二条记忆只有 1 个 blob、1 把 CK，常量过不了；执行后**关库重开**，两把 CK 消失、正文 `ContentKeyDestroyed`、记忆变墓碑、推断 `orphaned`、另一条记忆完好、审计三条仍在且链验证通过、审计 JSON 里搜不到正文。联系人单元另测（经关系边的证据 orphan 推断） |
| AC-15 中途崩溃 | `tests/crash_recovery.rs::a_crash_between_content_key_deletions_leaves_no_half_forgotten_memory`：`FORGET_CK_DELETE_MID` 在第一把 CK 删掉之后触发，子进程真 abort。重开后断言「整个单元的 CK 要么全在且可解密，要么全销毁且不可解密」，本实现落在回滚一侧（整个遗忘一个事务）；审计链仍通过。有不武装 fail point 的对照运行：遗忘走完、正文不可解 |
| AC-20 研究预览 | `tests/research_preview.rs`：fixture 里放了第三人事件（含密封正文）、mixed 事件、第三人联系人显示名。`rows` 来自 `GROUP BY kind, 小时桶, privacy_subject` 的真聚合（早上两条同类事件合成 `aggregate_count = 2`）；`third_party_rows` 由「已发布行里再数一遍第三人」得到，`zero_third_party_rows` 只接受 0，把过滤器改成放行第三人会让预览直接构造失败（已实测）；`third_party_rows_excluded > 0` 证明确实查到了并排除；输出过 `export-manifest.schema.json` 校验、过 leakage checker（第三人正文 + 姓名 + 账号入语料）；只有第三人占用的时段不出现在 rows 里；预览前后目录文件名与字节数完全不变；另有一条测试把 `research_preview.rs` 源码读回来，断言其中没有任何写文件的 API |
| AC-24 存储侧 | `tests/crash_recovery.rs::a_crash_mid_commit_loses_one_event_and_leaves_the_chain_verifiable`：`STORE_EVENT_COMMIT_MID` 用 `2*off->panic` 放过前两条、第三条在 INSERT 之后 COMMIT 之前杀掉子进程。重开后 `integrity_check` 为 ok、正好丢 1 条、已提交的两条都在、审计链验证通过。对照运行不武装则三条全写入 |
| fail point 名字没有另起一套 | `soul_store::failpoints` 与 `soul_testkit::crash::failpoints` 逐条相等，由 `the_fail_point_names_match_the_test_kit` 断言。soul-store 必须自己写字面量，因为 testkit 只能是 dev 依赖 |

落地内容：`crates/soul-store/{keys,sql,store,forget,audit,research_preview}.rs` 与五个测试文件；`soul-store-api` 新增 `research.rs`；`soulcore/src/commands/store.rs` 薄封装（`open_test_store` / `preview_forget` / `execute_forget` / `research_preview`）与 `soulcore/tests/store_commands.rs`。

### WP02 的取舍与遗留

1. **`soul-store-api` 只加不减，加的是 `research.rs`。** 合同里原本没有任何研究面，AC-20 无法对着存储边界表达。新增 `ResearchPreview` trait 故意**不**并进 `SoulStore`：研究轨道只读，写路径不该能碰到它。同一模块还提供 `zero_third_party_rows(counted)`——这是 `third_party_rows` 唯一的构造入口，只接受 0，所以那个字段不可能是没人验过的字面量。`run_conformance` 一行未改，`FakeStore` 未实现 `ResearchPreview`（真库以外没有事件表可查）。
2. **审计链的三个字段由库写，不由调用方写。** `append_audit` 覆盖 `seq` / `prev_hash` / `entry_hash`：调用方能自带链接的话，链就只对它自己自洽，AC-24 的「重开后链通过」也就不成立。`conformance` 传进来的占位哈希因此被覆盖，它只断言条数与无正文，不受影响。WP08 若要接管审计，接的是这套语义：链归存储，内容归调用方，`additionalProperties: false` 挡正文。
3. **`DpapiKeyProvider` 是骨架，Windows 密钥保护仍是缺口。** 见 `SECURITY.md`。Win32 绑定要 `unsafe`，本 crate `#![forbid(unsafe_code)]`，两个入口宁可返回 `KeyError::Unsupported` 也不编造密钥。补齐前不要在 UI 上声称 Windows 的 KEK 已受保护。
4. **遗忘是单事务，所以中途崩溃是回滚而不是半毁。** 工作单要求「要么 CK 还在且可解密，要么已销毁且不可解密」，两侧都合格；实现选了前者，并把这个选择写成断言，将来若改成分批提交，测试会立刻红。
5. **时间桶按 UTC 小时切。** `ts` 以 `Z` 结尾时取 `substr(ts,1,13)`；带偏移量的时间戳降级成日期级桶，而不是把本地小时贴上 `time_bucket_utc` 的标签。v0.1 写入方一律用 UTC，这条是防御性的。
6. **`zeroize` 不开 `derive`。** `zeroize_derive` 1.5 需要 edition 2024，工作区钉 1.83。`SecretKey` 手写 `Drop` 调 `[u8; 32]::zeroize`，`Debug` 打印 `<redacted>`，有测试。
7. **`soulcore` 现在通过 `soul-store` 间接依赖 `rusqlite` + vendored OpenSSL。** `e0-audit` 与 `cargo deny` 都还是绿的（禁的是 HTTP client），但 `cargo build -p soulcore` 从此要编 OpenSSL；WP01 遗留里关于 windows-latest 需要 perl 与 NASM 的那条现在也适用于主二进制，不只是测试。
8. **`meta.schema_version = 1`，还没有迁移器。** 表结构变了要么加迁移，要么在开发期删库重来。WP03–WP06 加列前先决定是哪一种。

## 阻塞

无。

## WP08 完成情况

`soul-policy` 与 `soul-egress` 落地。本机 `cargo test -p soul-policy -p soul-egress -p soulcore` 绿；`xtask e0-audit` / `denylist-audit` 绿。HTTP client 只出现在 `soul-egress`，且必须持有 `soul-policy` 签发的 `EgressPermit`。

| 交付 | 证据 |
|---|---|
| AC-11 精确 origin / 跨 origin 重定向拒绝 | `crates/soul-egress/tests/e1_origin.rs`：请求只打到配置 origin；跨 origin 与换端口 302 都不跟；无 endpoint 时发不出 permit |
| AC-12/13 第三人占位与一次性豁免 | `redactor_leakage.rs` / `redactor_exemption.rs`：默认路径过 leakage checker；豁免按值消费、下次回到占位；研究路径无豁免入口 |
| AC-19 HITL | `hitl.rs`：未知动作拒绝、plan hash 变拒绝、令牌一次性、写文件令牌即使「完美」也拒绝 |
| AC-23 审计无正文 | `audit_chain.rs`：矩阵回放过 leakage checker；散文字段在入库前被拒 |
| AC-24 审计侧崩溃 | `audit_crash.rs`：提交前死丢整条、写入后死保留，链都能重开验证 |
| AC-25 三路注入 | `injection.rs`：导入行 / 粘贴 / 文件名都进 `UntrustedText`，不能授权动作，不能外连语料里的 URL |
| E0 无代码路径 | `net_guard.rs`：`EgressClass` 没有 E0；`e0-audit` 6 个成品 crate 清洁；`deny.toml` 仅 `soul-egress` 可包 `reqwest` |

### WP08 取舍

1. **`EgressPermit` 无公开构造函数。** 字段对本模块私有，另一个 crate 无法伪造。
2. **写文件能力令牌在 v0.1 不签发也不消费。** HITL 测试断言没有 known action 要这个 scope。
3. **审计链字段仍由存储覆盖**（WP02 取舍 2）。WP08 只构造无正文内容。

## WP03 完成情况

`crates/soul-profile` 落地。本机 `cargo test -p soul-profile -p soulcore` 绿，`xtask all`（e0-audit / denylist-audit / schema-freeze --check）绿。

| 交付 | 证据 |
|---|---|
| 五条方向轴，`axis_id` 是固定 uuid7 常量 | `src/axes.rs` 五个 `AxisDefinition` 常量（好奇 / 条理 / 社交能量 / 随和 / 情绪稳度），各带 `label`、两极描述与问卷题；`tests/axes_and_evidence.rs` 断言五个 UUID 是合法 uuid7、互不相同、`label` 非空。fixture `fixtures/profile/default_axes_profile.json` 把这五个 ID 钉住，改号会红 |
| 位置只有四种，`clinical_claim` 恒 false | 位置直接用 `profile.schema.json` 的 `AxisPosition`，没有第五种可写；`clinical_claim` 是 `NotAClinicalClaim` 单值类型，不是字段 |
| 无 evidence 的 inference 不落库 | `service::record_axis_inference` 在碰存储前就返回 `ProfileError::NoEvidence`；真库那一层也拒。两道网，因为档案不能停在「写了一半又被驳回」 |
| AC-03 问卷完成后非空档案，字段来源 user_stated | `tests/questionnaire_intake.rs`：`fixtures/profile/questionnaire_answers_basic.json` 七个回答产出七条 `SoulEvidence`，`kind: questionnaire`、`method: user_stated`；五条轴全部离开 `unknown` 且各自 cite 到能解引用的那一条。另有 partial fixture（未答的轴留在 `unknown`，不猜）与 rejected fixture（数字位置 / 未知题号 / 题型不符 / 空卷四种，全部落空且不写库） |
| AC-06 档案侧每条 inference 可解引用 | `view::profile_view` 真去 `get_evidence` 每一个 id，取不回来就报 `DanglingEvidence` 而不是给空列表。`tests/axes_and_evidence.rs` 正反都测：正常情况全部解得开，人为写一条指向不存在证据的 inference 会被拒 |
| AC-07 档案侧纠正锁定 | `tests/correction_lock.rs`：用户纠正后轴 `locked_by_user`，随后更强的推断返回 `RefusedAxisLocked`，轴不动；被拒的推断**仍然入库**，用户有权看见机器还是不同意。锁一条不影响另外四条。语气侧同理：`set_voice` 之后 `suggest_voice` 返回 `false`，`read_voice` 仍返回用户值——这是 WP10 起草要读的那个值 |
| 禁数字、过非临床断言 | `numeric::reject_numeric_rating_value` 对序列化后的 JSON 递归查数字，测试往轴里注入一个 `score` 字段证明它真会红（对着 `TraitAxis` 结构体查是空转的，因为没有字段能放数字）。所有可读字符串过 `soul_policy::assert_non_clinical`；`render` 带 `WORKING_HYPOTHESIS_NOTICE` |

落地内容：`crates/soul-profile/{axes,voice,questionnaire,numeric,service,view,error}.rs` 与三个测试文件；`fixtures/profile/` 四份；`soulcore/src/commands/profile.rs`。

### WP03 的取舍与遗留

1. **问卷答案不锁轴，纠正才锁。** 两者都是用户说的，但含义不同：轴是灵魂层持有的工作假设，后来的证据有权修正它；语气是代理层要照办的指令。所以问卷的语气回答**立刻**钉住 `user_set`，问卷的轴回答只置 `locked_by_user = false`。要锁轴得走 `correct_axis`。
2. **`evidence_ids` 是替换不是累加。** 一条轴上的列表说的是「支持它**现在**这个位置的证据」。被取代的回答仍留在证据表与审计链里，只是不再被当作它已不支持的那个结论的依据。
3. **denylist 抓到过一次真的。** 语气渲染里「用得少」原本写成「少量表情」，其中「量表」正是 D22 禁的刻度词。改词之后补了一条穷举测试，把 81 种语气组合全部渲染一遍再过断言——这类命中靠人眼复查是抓不住的。
4. **语气只问两题。** 问卷问直接程度与表情用量，另外两个字段（语域、温度）留在中性默认，等用户在档案页自己改。多问两题的收益不如少问两题的完成率。
5. **`profile.voice` 是 schema 里的自由 JSON。** `VoiceProfile` 自己序列化进去，包含一份 `user_set` 名单。这意味着语气的锁定信息不在 `additionalProperties: false` 的保护范围内——档案 schema 没有为它定形状。若 WP09/WP10 要在 UI 上展示锁定态，先考虑把它提成正式字段。

## WP04 完成情况

`crates/soul-memory` 落地。本机 `cargo test -p soul-memory -p soulcore` 绿，`xtask all` 绿。两个验收测试都跑真 `SqlCipherStore` 并且**关库重开**后才断言。

| 交付 | 证据 |
|---|---|
| 标题/摘要走 sealed blob | `service::create` 只经 `BlobStore::seal`；`SoulMemory` 上存的是 `SealedText` 指针。列表视图 `MemoryDigest` 没有任何字符串字段，「不泄正文」是类型层面的事实 |
| AC-14 读写一致 | `tests/crud_roundtrip.rs`：三条 fixture 记忆逐字节读回（含全角引号与长中文段），关库重开后再读一遍仍一致——进程内一致证明不了盘上的密文能不能开。编辑后按同一把 CK 重新密封，`content_key_id` 不变 |
| AC-14 审计无内容 | 同上文件：审计条目只有 id 与计数（`items: 2` 即密封了两个字段），`bytes` 恒空以免成为正文长度的旁路。整条链序列化后过 leakage checker，阈值调到 4 个 scalar，语料是 fixture 里每一句正文 |
| 遗忘走 ForgetOps，预览数字来自真查询 | `tests/forget_reopen.rs`：第一条记忆 2 个 blob、第二条也是 2 个但 CK 不同，编辑第一条后变成 3 个（被取代的密文仍在同一把 CK 下，会一起死）。任何常量都过不了这三个数。回执与预览逐字段相等 |
| AC-15 记忆侧 | 同上文件：关库重开后 CK 不在、密文 `ContentKeyDestroyed`、`read` 返回 `Forgotten`（不是 `NotFound`——用户有权知道自己忘掉过东西）、行留作墓碑仍在列表里、依赖该记忆证据的推断 `orphaned`、另一条记忆完好、三条审计仍在且链验证通过、审计 JSON 里搜不到任何一句正文 |
| 没有重造存储引擎 | `soul-memory` 只依赖 `soul-store-api`，不依赖 `soul-store`（后者只是 dev 依赖，供测试开真库）。加密、CK 生命周期、遗忘事务全在 WP02 那边 |

落地内容：`crates/soul-memory/{draft,service,error}.rs` 与两个测试文件；`fixtures/memory/memories_basic.json`；`soulcore/src/commands/memory.rs`。

### WP04 的取舍与遗留

1. **一条记忆一把 CK，编辑复用。** 这是「一条记忆 = 一个遗忘单元」的前提。编辑若新铸一把，遗忘之后旧标题还读得出来。代价是被取代的密文留在 blob 表里；测试直接断言它跟着同一次遗忘消失，而不是把这一点留作假设。
2. **遗忘的审计条目写在销毁**之后**。** PRODUCT_LOCK 说审计链绝不能挡住遗忘，所以可能失败的那次写发生在第二步。
3. **重复遗忘不报错，但也不是幂等的回执。** 第二次的 `content_key_ids` 仍列出那把钥匙（映射是墓碑的一部分），但 `sealed_blobs_destroyed` 变 0——真的没东西可销毁了。崩溃后重试因此是安全的。测试把这个语义写死了。
4. **`MemoryDraft.subject` 只有 owner / third_party / mixed 三档，占位符按整条记忆走。** 一条 mixed 记忆里哪一句是别人说的，v0.1 不区分，整条带同一个占位符。要做到句级，得在 blob 层加结构，不是这个 crate 能单独决定的。
5. **`preview_forget` 与 `forget` 之间没有令牌。** 用户看到的数字与实际销毁的数字由「回执必须等于预览」这条断言保证，但两次调用之间若有别的写入，数字会变而没人拦。WP09 接 UI 时若要严格，需要一个把预览钉住的短期令牌。

## 下一步

批 3 的档案与记忆（WP03+WP04）已完成，人脉图与导入（WP05+WP06）见对应小节。不要启动 Goal 2。文件写入仍是 v0.1.1。
