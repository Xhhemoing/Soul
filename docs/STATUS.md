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
| WP05 人脉图 | 完成。见下节 |
| WP06 导入 | 完成。见下节 |
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

## WP05 完成情况

`crates/soul-graph` 落地。本机 `cargo test -p soul-graph -p soul-import -p soulcore` 绿，`xtask all` 绿。图是**推导**出来的，不是导入进来的：导入方写互动证据，图这边一遍扫过去得出边。

| 交付 | 证据 |
|---|---|
| 节点=人，边=互动强度/关系类型/最近接触/证据 | `model.rs` 的 `TieEdge` 四样齐全，全部由观察推出：`TieStrength` 是计数（往/来/会话数/活跃天数/首末接触），不是评分，`TieType` 只说观察到的形状（direct / group_only / reciprocal / one_sided），不编造「同事」「家人」。`tests/ego_graph.rs::an_edge_carries_strength_type_last_contact_and_evidence` 从库里读回来断言，而不是断言 build 的返回值 |
| AC-08 三个对话对象 → 节点 ≥3，边有 evidence | `tests/ego_graph.rs::three_conversation_partners_produce_three_evidence_backed_edges`（手写观察）与 `soul-import/tests/import_to_graph.rs::a_four_partner_export_becomes_four_evidence_backed_edges`（从 fixture 文件走完整条链）。两边都对每条边真去 `resolve_evidence` |
| AC-06 图侧推断可解引用 | `tests/ego_graph.rs::a_tie_inference_dereferences_to_the_observations_behind_it`：每条 tie inference 的 `evidence_ids` 逐个 `get_evidence` 取回，`target.relationship_id` 也真去 `get_relationship`，并断言边与推断引的是同一批证据。`view::resolve_evidence` 取不回来就报错，不返回短一截的列表 |
| 第三人节点默认 local_only，不进 E1/研究行 | `tests/ego_graph.rs::every_third_party_node_and_edge_is_local_only`：`SoulGraph::third_party_data_is_local_only()` 对整张图成立，库里每条 relationship 的 `egress_scope` 都是 `local_only`，每条互动证据 `exportable_to_research: false`。节点上没有任何字段装名字——`label_ref` 是密封指针，标识符是哈希 |
| 重建幂等 | `rebuild` 按 (owner, peer) 认边、按 (relationship_id, 语句前缀) 认推断。`tests/ego_graph.rs::rebuilding_updates_the_same_edges_instead_of_duplicating_them`：第二次导入后 id 一模一样，边数与推断数不变，强度与最近接触更新了 |
| 证据指向已消失的人不静默丢 | 同文件 `evidence_naming_a_contact_that_is_gone_is_counted_rather_than_dropped`：`GraphBuild.peers_unresolved` 计数，不给不存在的人建边 |

落地内容：`crates/soul-graph/{interaction,model,build,view,error}.rs` 与 `tests/ego_graph.rs`；`soulcore/src/commands/graph.rs`；`soul-store-api`/`soul-store` 新增 `list_evidence` 与 `list_relationships`（含一条 conformance 用例）。

### WP05 的取舍与遗留

1. **v0.1 是自我中心网络，每条边都有用户在一端。** Soul 只看见用户参与过的对话，两个第三人之间的边只能靠猜，而 PRODUCT_LOCK 要求推断带证据。真要做人与人之间的边，得先有一个说得出证据的来源。
2. **强度是计数不是评分。** D22 禁掉了心理模型的数字刻度；同一直觉用在这里，所以对外的概括值是 `SupportedBand`，底下是用户自己数消息就能核对的计数。阈值（`MODERATE_MIN_INTERACTIONS = 3`、`STRONG_MIN_INTERACTIONS = 10` 且活跃天数 ≥3）写成常量摆在 `build.rs` 顶部，不藏在表达式里。
3. **`relationship.types` 与 `tie_strength` 在契约里是自由 JSON。** 图这边定了自己的读法，读不出来就报 `UnreadableEdge` 而不是编造一个强度。手改过的行或旧版本写的行会走这条路。
4. **群聊里只给「说过话的人」建边。** 一个五百人的群里潜水的人不该因为用户发了一条消息就长出一条边。代价是真的只潜水的熟人不会出现在图里。
5. **`GraphBuild.audit` 由调用方落链。** 与 WP08 的做法一致：`soul-graph` 构造内容，握着打开的库的人写进去。`soulcore/src/commands/graph.rs` 就是这么做的。

## WP06 完成情况

`crates/soul-import` 落地：两个 v0.1 导入器加问卷回退。本机 `cargo test -p soul-import -p soul-graph -p soulcore -p soul-testkit` 绿，`xtask all` 绿。

| 交付 | 证据 |
|---|---|
| soul-import-v1 逐行校验，合法 fixture 落加密库 | `tests/soul_import_v1.rs`：`valid_basic.jsonl` 解析出 2 个对象 2 个会话，提交后 5 条事件、3 个联系人。重复导入认出已有的人（`contacts_matched` 3，联系人表仍是 3） |
| AC-04 库文件字节无明文 | 同文件 `a_valid_file_lands_sealed_with_no_plaintext_left_on_disk`：`flush` + `close` 之后扫目录下每个文件（含 `-wal`/`-shm`），fixture 里每一句正文都搜不到，**会话 id 也搜不到**（它是第三人标识符，入库前就哈希了）。反向断言正文仍能从 `open()` 取回，避免「什么都没写」也能过 |
| AC-05 Telegram 映射与可读失败 | `tests/telegram.rs`：`result_basic.json` 出 3 个参与者 6 条消息 2 个会话，service 消息（通话）跳过，`text` 的分段数组拼回用户看到的那一句。失败侧对 `result_missing_fields.json` 断言 `messages`/`id`/`from_id`/`date_unixtime`/`date` 都被点名，定位串带 `chat_id=` 与 `message_id=`，且**三个会话标题一个都没出现在消息里**——个人会话的标题就是对方的名字 |
| AC-03 导入侧问卷回退 | `tests/questionnaire.rs`：无文件时 `fallback_needed` 为真，只有 header 的空导出也为真。八题回答产出事件（`source: ui_questionnaire`、`kind: questionnaire_answer`）与证据（`kind: questionnaire`、`method: user_stated`、`subject: self`），证据的 `source_refs` 指回事件与题号。留白的一题不落任何行 |
| 注入行只进数据通道 | `tests/injection_is_data.rs`：`injection_lines.jsonl` 五行全是合法数据，正常提交、正文能读回来，同时留下 `injection.blocked` 审计（`items: 5`，无正文）。对每一行、对 `ActionKind::ALL` 的每一个动作，以 `RequestOrigin::ExternalContent` 请求全部被拒且理由是 `external_content_not_authority`；语料里的 URL 逐个过 `NetGuard::closed()` 全部拒绝；形如 tool call 的那行解析成 JSON 之后仍然只是 JSON |
| 错误信息可读且不含原文 | `tests/soul_import_v1.rs::a_refusal_reads_like_a_sentence_and_repeats_none_of_the_file`：每条 defect 有位置、有句子，`field` 只能是契约定义的名字；把所有 reason 拼起来，fixture 里每一句正文的任意 12 个 scalar 的窗口都搜不到。另一条测试把整段散文塞进 JSON 的**键**里，断言它不会被回显 |

落地内容：`crates/soul-import/{model,soul_import_v1,telegram,commit,questionnaire,defect,redact,instant}.rs` 与五个测试文件；`fixtures/import/soul-import-v1/three_partners.jsonl`、`fixtures/import/questionnaire/answers_basic.json`；`soulcore/src/commands/import.rs`。

### WP06 与 WP03 的接缝

问卷答案在这一侧只落成**事件 + `user_stated` 证据**，不碰档案。交接点是 `questionnaire::UserStatedSink`：

```rust
pub trait UserStatedSink {
    fn accept(&mut self, answer: &RecordedAnswer) -> Result<(), SinkError>;
}
```

`RecordedAnswer` 只带 `event_id`、`evidence_id`、`method`（恒 `user_stated`）与 `question`（借自 `QUESTIONS`，所以题号一定是这套构建认得的）。哪一条答案动哪一根轴，由实现方决定——`soul-import` 不知道档案是什么，也不该知道。`CollectingSink` 是给 WP03 落地之前和测试用的。

WP03 已经落地了自己的问卷入档路径（`soul-profile::questionnaire`），两条路并存且**题号不同**：`soul-import::questionnaire::QUESTIONS` 是八题，键形如 `voice.directness`；`soul-profile` 是七题，键形如 `q.axis.curiosity`。两者目前互不引用，写的证据也互不覆盖，但 v0.1 收尾前应该并成一套题号，否则用户会被问两遍。并的时候 `UserStatedSink` 就是那个接口——`soul-profile` 实现它即可，`soul-import` 一行不用改。

### WP06 的取舍与遗留

1. **契约顶层是 `oneOf`，所以字段级的报错是这个 crate 自己说的。** 校验器对一条坏消息行只能说「两个分支都不匹配」。转发 serde 的报错更糟：它的文本会把噎住它的那个值原样抬出来，而这正是拒绝信不能做的事。于是 `soul_import_v1::shape_defects` 把同一套要求写第二遍，句子从那里来，**能不能通过仍由 schema 说了算**。契约改了要同时改这里；`the_malformed_import_corpus_is_rejected_line_by_line` 与本 crate 的测试会一起红。
2. **内容护栏有两道尺度。** 从文件里抬出来的**片段**（契约没定义的字段名）echo 4 个 scalar 就丢掉；本 crate 自己拼的**句子**要重复 12 个 scalar 才算引用。一开始两者都用 4，结果一个正文里恰好写了 `RFC 3339` 的文件，会把「你的时间戳不是 RFC 3339」这句解释本身消音——护栏惩罚了读信的人。
3. **Telegram 的 `contacts.list` 不导入。** 电话簿条目有名字有号码但没有 user id，聊天消息有 user id 没有号码，没有连接键。硬并会造出重复的人或者错的人；只有在聊天里出现过的人才成为联系人。
4. **时间取 `date_unixtime`，缺了就拒。** 旁边的 `date` 是本地墙钟没有偏移量，单靠它只能猜时区。fixture 里这两个字段本来就对不上，测试反过来利用了这一点来证明用的是哪一个。
5. **标识符按来源加盐。** 一个 Telegram 导出里的 user `42` 和一个 `soul-import-v1` 文件里的 user `42` 是两个人，直到有东西把他们连起来。两个节点是用户看得见、能合并的错；一个节点装两个人是看起来对的错图。`import_to_graph.rs::identifiers_are_scoped_to_the_export_they_came_from` 把这个语义连同「两个 self 联系人时图会拒绝构建」一起钉住了。
6. **同一个文件导入两次会写两遍事件。** v0.1 没有外部 id 索引可以去重，造一个就意味着要有一列存平台的消息 id。联系人是去重的（按标识符摘要），事件不是。要不要去重由调用方决定。
7. **提交不是一个事务。** `commit` 逐条写联系人、密封、事件、证据；中途失败会留下写了一半的导入。WP02 的遗忘是单事务的，导入不是——`soul-store-api` 上没有可以让调用方开事务的入口，加一个是存储边界的改动，超出本工作单。重跑同一个文件是安全的（联系人会认回来），只是事件会多一份。

## 下一步

批 3 的档案与记忆（WP03+WP04）、人脉图与导入（WP05+WP06）都已完成。不要启动 Goal 2。文件写入仍是 v0.1.1。
