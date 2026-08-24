# STATUS

单一事实来源。每个子代理完工必须更新本文件。

## 当前里程碑

**`PLAN_FROZEN`**。Goal 1 已开工：分支 `cursor/soul-goal1-7b1c`。文档 PR `#1` 不夹带应用代码。Goal 2 在 Goal 1 关闭前不要启动。批 1（WP01）与批 2（WP02 数据面 + WP08 权限面）已完成。Windows CI 在 schema freeze CRLF 修复后一度全绿；WP11 落地后 `test (windows-latest)` 因 `canonicalize` 的 `\\?\C:\...` 被当成 UNC 而红，筛查已改为只把本地盘的 extended-length 写法剥成盘符路径。WP13 第二段之后，桌面壳握着这个进程唯一的 `SqlCipherStore` 句柄，配置能读回来，`/files`、`/graph` 与起草的端点确认屏都不再是空路由；Windows 上库仍然打不开，因为 DPAPI 还是骨架。

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
| WP03 档案 | 完成。见下节。与 WP06 的两套问卷已并成一套（见「WP06 与 WP03 的接缝」） |
| WP04 自传记忆 | 完成。见下节 |
| WP05 人脉图 | 完成。见下节 |
| WP06 导入 | 完成。见下节。问卷回退与 WP03 的入档路径已合并，`soul-profile` 实现 `UserStatedSink` |
| WP07 前台采集 | 完成。见下节 |
| WP09 桌面壳 | 第一段（壳）完成。见下节。壳里那一个 store 句柄由 WP13 第二段落地（遗留 8 消除） |
| WP10 起草与人事摘要 | 完成。见下节。本机路径与端点路径的确认屏都已接上（遗留 6 消除） |
| WP11 文件计划 | 完成。见下节。`/files` 已接 `PlanPreview`，仍然没有执行按钮（遗留 8 消除） |
| WP13 安装 smoke / CI / SBOM / 壳接库 | 两段都完成。见下节。剩下的是 Windows 真机手动那七条 |
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

`crates/soul-profile` 落地。本机 `cargo test -p soul-profile -p soulcore` 绿，`xtask all`（e0-audit / denylist-audit / schema-freeze --check）绿。问卷合并之后重跑 `cargo test -p soul-profile -p soul-import -p soulcore -p soul-testkit`，绿。

| 交付 | 证据 |
|---|---|
| 五条方向轴，`axis_id` 是固定 uuid7 常量 | `src/axes.rs` 五个 `AxisDefinition` 常量（好奇 / 条理 / 社交能量 / 随和 / 情绪稳度），各带 `label`、两极描述与问卷题号（题面在合并后的那一份题表里，见 WP03/WP06 接缝）；`tests/axes_and_evidence.rs` 断言五个 UUID 是合法 uuid7、互不相同、`label` 非空。fixture `fixtures/profile/default_axes_profile.json` 把这五个 ID 钉住，改号会红 |
| 位置只有四种，`clinical_claim` 恒 false | 位置直接用 `profile.schema.json` 的 `AxisPosition`，没有第五种可写；`clinical_claim` 是 `NotAClinicalClaim` 单值类型，不是字段 |
| 无 evidence 的 inference 不落库 | `service::record_axis_inference` 在碰存储前就返回 `ProfileError::NoEvidence`；真库那一层也拒。两道网，因为档案不能停在「写了一半又被驳回」 |
| AC-03 问卷完成后非空档案，字段来源 user_stated | `tests/questionnaire_intake.rs`：`fixtures/questionnaire/answers_basic.json` 十一题答了十题（留白那题不落任何行），产出十条 `SoulEvidence`，`kind: questionnaire`、`method: user_stated`；五条轴全部离开 `unknown` 且各自 cite 到能解引用的那一条，语气三项钉住 `user_set`，两道散文题落成 `boundaries` / `values` 里的指针。同一批回答同时是十条 `ui_questionnaire` 事件，正文密封、一次运行一把 CK。另有 partial fixture（未答的轴留在 `unknown`，不猜；未答的散文题不留占位）与 rejected fixture（数字位置 / 未知题号 / 题型不符 / 拿散文答选择题 / 空卷五种，全部落空，库里连证据都不留） |
| AC-06 档案侧每条 inference 可解引用 | `view::profile_view` 真去 `get_evidence` 每一个 id，取不回来就报 `DanglingEvidence` 而不是给空列表。`tests/axes_and_evidence.rs` 正反都测：正常情况全部解得开，人为写一条指向不存在证据的 inference 会被拒 |
| AC-07 档案侧纠正锁定 | `tests/correction_lock.rs`：用户纠正后轴 `locked_by_user`，随后更强的推断返回 `RefusedAxisLocked`，轴不动；被拒的推断**仍然入库**，用户有权看见机器还是不同意。锁一条不影响另外四条。语气侧同理：`set_voice` 之后 `suggest_voice` 返回 `false`，`read_voice` 仍返回用户值——这是 WP10 起草要读的那个值 |
| 禁数字、过非临床断言 | `numeric::reject_numeric_rating_value` 对序列化后的 JSON 递归查数字，测试往轴里注入一个 `score` 字段证明它真会红（对着 `TraitAxis` 结构体查是空转的，因为没有字段能放数字）。所有可读字符串过 `soul_policy::assert_non_clinical`；`render` 带 `WORKING_HYPOTHESIS_NOTICE` |

落地内容：`crates/soul-profile/{axes,voice,questionnaire,sink,numeric,service,view,error}.rs` 与五个测试文件；`fixtures/profile/default_axes_profile.json` 与 `fixtures/questionnaire/` 四份；`soulcore/src/commands/profile.rs`。

### WP03 的取舍与遗留

1. **问卷答案不锁轴，纠正才锁。** 两者都是用户说的，但含义不同：轴是灵魂层持有的工作假设，后来的证据有权修正它；语气是代理层要照办的指令。所以问卷的语气回答**立刻**钉住 `user_set`，问卷的轴回答只置 `locked_by_user = false`。要锁轴得走 `correct_axis`。
2. **`evidence_ids` 是替换不是累加。** 一条轴上的列表说的是「支持它**现在**这个位置的证据」。被取代的回答仍留在证据表与审计链里，只是不再被当作它已不支持的那个结论的依据。
3. **denylist 抓到过一次真的。** 语气渲染里「用得少」原本写成「少量表情」，其中「量表」正是 D22 禁的刻度词。改词之后补了一条穷举测试，把 81 种语气组合全部渲染一遍再过断言——这类命中靠人眼复查是抓不住的。
4. **语气问三题，不是两题也不是四题。** 原本只问直接程度与表情用量，理由是完成率。合并之后语域（`q.voice.register`）也进来了——WP06 那一侧本来就在问它，只是落成了一段没人读的散文；同一个问题问一遍并且让它真的钉住字段，比问一遍然后丢掉划算。温度（warmth）仍然不问，留在中性默认等用户在档案页自己改，`voice_question_id(Warmth)` 返回 `None` 而不是指向一道不存在的题。
5. **`profile.voice` 是 schema 里的自由 JSON。** `VoiceProfile` 自己序列化进去，包含一份 `user_set` 名单。这意味着语气的锁定信息不在 `additionalProperties: false` 的保护范围内——档案 schema 没有为它定形状。若 WP09/WP10 要在 UI 上展示锁定态，先考虑把它提成正式字段。
6. **散文题落进 `boundaries` / `values` 的是指针，不是话。** 契约里这两个字段是自由数组，正因为如此往里放什么要自己守规矩：落的是 `{origin, question_id, event_id, evidence_id}`，用户写的那句话留在录制方密封的那条事件里。SECURITY.md 把散文限定在 `sealedText`，`profiles` 表不是那个地方。代价是要读回这句话得开一次 blob，档案视图目前不做这件事——UI 要展示「你说过的边界」时才需要接。
7. **同一道题再答一次是替换，不是叠加。** 与轴上的 `evidence_ids` 同一个规矩：`boundaries` 里一道题只留一条指针，旧的那条事件与证据仍在库里、仍在链上，只是不再被当作现在这条边界的依据。

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

`crates/soul-import` 落地：两个 v0.1 导入器加问卷回退。本机 `cargo test -p soul-import -p soul-graph -p soulcore -p soul-testkit` 绿，`xtask all` 绿。问卷回退此后与 WP03 合并成一套题（见下面的接缝一节），合并后同一组命令重跑仍绿。

| 交付 | 证据 |
|---|---|
| soul-import-v1 逐行校验，合法 fixture 落加密库 | `tests/soul_import_v1.rs`：`valid_basic.jsonl` 解析出 2 个对象 2 个会话，提交后 5 条事件、3 个联系人。重复导入认出已有的人（`contacts_matched` 3，联系人表仍是 3） |
| AC-04 库文件字节无明文 | 同文件 `a_valid_file_lands_sealed_with_no_plaintext_left_on_disk`：`flush` + `close` 之后扫目录下每个文件（含 `-wal`/`-shm`），fixture 里每一句正文都搜不到，**会话 id 也搜不到**（它是第三人标识符，入库前就哈希了）。反向断言正文仍能从 `open()` 取回，避免「什么都没写」也能过 |
| AC-05 Telegram 映射与可读失败 | `tests/telegram.rs`：`result_basic.json` 出 3 个参与者 6 条消息 2 个会话，service 消息（通话）跳过，`text` 的分段数组拼回用户看到的那一句。失败侧对 `result_missing_fields.json` 断言 `messages`/`id`/`from_id`/`date_unixtime`/`date` 都被点名，定位串带 `chat_id=` 与 `message_id=`，且**三个会话标题一个都没出现在消息里**——个人会话的标题就是对方的名字 |
| AC-03 导入侧问卷回退 | `tests/questionnaire.rs`：无文件时 `fallback_needed` 为真，只有 header 的空导出也为真。回答产出事件（`source: ui_questionnaire`、`kind: questionnaire_answer`）与证据（`kind: questionnaire`、`method: user_stated`、`subject: self`），证据的 `source_refs` 指回事件、题号，选择题还带 `option_key`。留白的一题不落任何行；不在选项里的答案当场拒，且拒绝信不复述它。同一个文件把 `QUESTIONS` 逐题对着 `fixtures/questionnaire/v0_1.json` 核一遍——题号、题型、选项 |
| 注入行只进数据通道 | `tests/injection_is_data.rs`：`injection_lines.jsonl` 五行全是合法数据，正常提交、正文能读回来，同时留下 `injection.blocked` 审计（`items: 5`，无正文）。对每一行、对 `ActionKind::ALL` 的每一个动作，以 `RequestOrigin::ExternalContent` 请求全部被拒且理由是 `external_content_not_authority`；语料里的 URL 逐个过 `NetGuard::closed()` 全部拒绝；形如 tool call 的那行解析成 JSON 之后仍然只是 JSON |
| 错误信息可读且不含原文 | `tests/soul_import_v1.rs::a_refusal_reads_like_a_sentence_and_repeats_none_of_the_file`：每条 defect 有位置、有句子，`field` 只能是契约定义的名字；把所有 reason 拼起来，fixture 里每一句正文的任意 12 个 scalar 的窗口都搜不到。另一条测试把整段散文塞进 JSON 的**键**里，断言它不会被回显 |

落地内容：`crates/soul-import/{model,soul_import_v1,telegram,commit,questionnaire,defect,redact,instant}.rs` 与五个测试文件；`fixtures/import/soul-import-v1/three_partners.jsonl`、`fixtures/questionnaire/`（与 WP03 共用）；`soulcore/src/commands/import.rs`。

### WP06 与 WP03 的接缝（已合并）

原先这里写着一条遗留：两侧各有一套问卷，`soul-import::questionnaire::QUESTIONS` 八题、键形如 `voice.directness`，`soul-profile` 七题、键形如 `q.axis.curiosity`；两者互不引用，写的证据也互不覆盖。互不覆盖正是它能一直活着的原因——用户不会看到冲突，只会被问两遍，而其中八题的答案永远进不了档案。现在并成了一套。

**一份题表，问一遍。** 题表在 `soul-import::questionnaire::QUESTIONS`：录制方拥有它，因为拒绝一个没人提供过的选项是录制方的事，而它不知道档案是什么，也不该知道。每道题带一个 `AnswerShape`——`Choice(&[&str])` 或 `Prose`。`soul-profile` 不再自己声明题目，`questionnaire()` 是遍历这张表、逐题问 `target_of()` 拼出来的。

**十一题，不是十五题。**

| 题号 | 形态 | 动什么 |
|---|---|---|
| `q.axis.curiosity` / `q.axis.orderliness` / `q.axis.social_energy` / `q.axis.accommodation` / `q.axis.emotional_steadiness` | 三选一（`leans_low` / `mixed` / `leans_high`） | 五条方向轴 |
| `q.voice.register` / `q.voice.directness` / `q.voice.emoji_use` | 三选一 | 语气三个字段，答了就钉住 |
| `q.boundary.topics` / `q.boundary.availability` | 文本框 | `profile.boundaries`（指针） |
| `q.value.what_matters` | 文本框 | `profile.values`（指针） |

WP06 那八题的去向：`voice.directness` 与 `voice.register` 从文本框变成选择题，现在直接钉语气字段；`boundary.topics` / `boundary.availability` / `value.what_matters` 保留为文本框，但接上了档案；`preference.decision_style`（推进 vs 想清楚）问的是 `q.axis.orderliness` 那条轴，合并；`voice.length` 档案里没有对应字段，而消息长短是导入文件一眼能看出来的东西，删；`relationship.close_circle` 删——人脉图是从证据推出来的（WP05 遗留 1），一段「同事、家人」的散文既建不出节点也建不出边，还会把第三人写进一条 `subject: self` 的密文里。

**接口没变，实现方来了。** `UserStatedSink` 还是那个 trait，`soul-profile::sink::ProfileSink` 实现它。`RecordedAnswer` 多了一个字段 `choice: Option<&'static str>`：选择题带上用户选中的那个选项（借自题目自己的列表，所以只可能是提供过的那几个），文本框是 `None`——**用户写的话不过接缝**。sink 只解释不落库：它把 `q.axis.curiosity` + `leans_high` 变成「好奇轴 + leans_high」，把 `formal` 变成 `VoiceSetting::Register(Formal)`，暂存起来；写库是 `soul_profile::intake` 的事，因为录制的时候 store 正被 `record()` 可变借着，一个也要写的 sink 会是别人借用期里的第二个写者。这样安排还有一个好处：sink 拒绝（没有对应字段、选项解不开）发生在档案被碰之前。

**一条回答只有一份记录。** `soul_profile::intake` 现在是问卷的唯一入口：校验 → `soul_import::questionnaire::record`（密封正文、写事件、写一条 `user_stated` 证据）→ 应用 sink 暂存的东西。轴 cite 的就是录制方写的那条证据，不再另铸一条。证据强度两侧统一成 `moderate`（原先导入侧写 `strong`）：凭记忆描述自己是中等支持，看着结论纠正才是强的，这是 WP03 的读法，也是 `correct_axis` 那条 `strong` 的对照。

**钉住题号的是 fixture，不是自觉。** `fixtures/questionnaire/v0_1.json` 列出每道题的题号、题型、选项与它动的档案字段。`soul-import/tests/questionnaire.rs` 核前三样，`soul-profile/tests/one_questionnaire.rs` 核最后一样并断言两侧的题号列表逐项相等。库里的证据 `source_refs` 指着这些题号，改一个就是让已经写下的证据指空，所以这条断言是硬的。

**还没有人画这十一道题。** 桌面向导目前只做「确认默认全关」（WP09 第一段），命令面 `import::questions()` 与 `profile::questions()` 把题目和选项都备好了，`Answer::for_question(question_id, 选中的那个选项)` 是 UI 只需要知道的那一个调用。headless 主流程走的是编进二进制的那份答卷（`fixtures/questionnaire/answers_basic.json`），AC-03 在 CI 里绿的是这条路。真正的向导界面属于 WP09 的下一段。

### WP06 的取舍与遗留

1. **契约顶层是 `oneOf`，所以字段级的报错是这个 crate 自己说的。** 校验器对一条坏消息行只能说「两个分支都不匹配」。转发 serde 的报错更糟：它的文本会把噎住它的那个值原样抬出来，而这正是拒绝信不能做的事。于是 `soul_import_v1::shape_defects` 把同一套要求写第二遍，句子从那里来，**能不能通过仍由 schema 说了算**。契约改了要同时改这里；`the_malformed_import_corpus_is_rejected_line_by_line` 与本 crate 的测试会一起红。
2. **内容护栏有两道尺度。** 从文件里抬出来的**片段**（契约没定义的字段名）echo 4 个 scalar 就丢掉；本 crate 自己拼的**句子**要重复 12 个 scalar 才算引用。一开始两者都用 4，结果一个正文里恰好写了 `RFC 3339` 的文件，会把「你的时间戳不是 RFC 3339」这句解释本身消音——护栏惩罚了读信的人。
3. **Telegram 的 `contacts.list` 不导入。** 电话簿条目有名字有号码但没有 user id，聊天消息有 user id 没有号码，没有连接键。硬并会造出重复的人或者错的人；只有在聊天里出现过的人才成为联系人。
4. **时间取 `date_unixtime`，缺了就拒。** 旁边的 `date` 是本地墙钟没有偏移量，单靠它只能猜时区。fixture 里这两个字段本来就对不上，测试反过来利用了这一点来证明用的是哪一个。
5. **标识符按来源加盐。** 一个 Telegram 导出里的 user `42` 和一个 `soul-import-v1` 文件里的 user `42` 是两个人，直到有东西把他们连起来。两个节点是用户看得见、能合并的错；一个节点装两个人是看起来对的错图。`import_to_graph.rs::identifiers_are_scoped_to_the_export_they_came_from` 把这个语义连同「两个 self 联系人时图会拒绝构建」一起钉住了。
6. **同一个文件导入两次会写两遍事件。** v0.1 没有外部 id 索引可以去重，造一个就意味着要有一列存平台的消息 id。联系人是去重的（按标识符摘要），事件不是。要不要去重由调用方决定。
7. **提交不是一个事务。** `commit` 逐条写联系人、密封、事件、证据；中途失败会留下写了一半的导入。WP02 的遗忘是单事务的，导入不是——`soul-store-api` 上没有可以让调用方开事务的入口，加一个是存储边界的改动，超出本工作单。重跑同一个文件是安全的（联系人会认回来），只是事件会多一份。
8. **问卷也不是一个事务。** 合并之后 `intake` 是「录制 N 条 → 写档案 → 落审计」，中途失败会留下几条没有档案认领的问卷事件与证据。它们不是坏数据（每条都自洽、都指得回题号），只是没被引用；重跑一遍是安全的，轴上的 `evidence_ids` 是替换语义。要做成原子的，同样得先有一个能让调用方开事务的存储入口。
9. **`soul-profile` 依赖 `soul-import`，方向是定的。** 合并要有一个 crate 拥有题表，而录制方不能知道档案是什么——反过来接就得让 `soul-import` 认识轴与语气字段。代价是 `soul-profile` 的依赖里多了一个不搞存储也不搞策略的 crate，以及选项那几个 token（`leans_high`、`formal`……）在两边各出现一次：录制方声明它们是为了拒掉没提供过的选项，档案侧解释它们。`one_questionnaire.rs` 把每个 token 拿去 `position_by_key` / `VoiceSetting::from_option` 解一遍，解不开就红。

## WP07 完成情况

`crates/soul-collect` 落地。本机 `cargo test --workspace --all-targets` 绿；`xtask all`（e0-audit / denylist-audit / schema-freeze --check）在**干净检出**上绿。链路是 `ForegroundSource → 同意门 → EventStore`，CI 只替换最前面那一段，后面全是 Windows 机器上跑的同一份代码。

| 交付 | 证据 |
|---|---|
| `ForegroundSource` 是唯一入口 | `src/source.rs` 定义 trait 与 `AppIdentity`；`src/windows.rs` 用 `GetForegroundWindow` → `GetWindowThreadProcessId` → `QueryFullProcessImageNameW` 三个调用实现它（`cfg(windows)`，`cargo check`/`clippy --target x86_64-pc-windows-msvc` 过，真机未跑）；`src/fake.rs` 是 CI 那一侧。`platform_source()` 在非 Windows 上返回 `Unsupported` 而不是一个永远报告「无前台」的源——后者会让用户以为采集在工作 |
| AC-09 采集关时事件=0 | `tests/consent_gate.rs`：同意门关着切 10 次应用，`list_events` 无论按 `source` 过滤还是全表都是 0，`Collector` 的 `consent_refusals` 是 10，而 `FakeForegroundSource::samples_taken()` 是 **0**——门在源前面，关着的时候连前台都不看。第二条测试证明 `runner::start` 本身就拒绝启动。第四条是对照：同样 10 次切换、同意打开，写出 10 条事件，所以上面的 0 是门挡下来的，不是管道从来没通过 |
| AC-10 开启期间 ≥1 条、关闭后 1s 内无新事件 | `tests/collection_lifecycle.rs`：真起后台线程、真 `SqlCipherStore`，切应用直到库里出现事件（超时 10s 判失败）；`stop()` 耗时断言 ≤ `STOP_BUDGET`（1s）；停完再切 20 次应用并等满 1 秒，事件数一条不变。撤销同意是第二条路径，也在 1 秒内自停且此后无新事件 |
| 停采集 ≤1s 与轮询无关 | `stopping_does_not_wait_for_the_poll_interval`：用默认 1 秒轮询间隔起停，耗时断言 < 0.5 秒。等待是 `Condvar::wait_timeout`，不是 `sleep`，所以「停得快」不是「轮询得快」的副产品。另有一条：句柄被 drop 时线程也停，不会留下没人能关的采集 |
| 只采时长，不采窗口标题 | `tests/window_titles_are_not_collected.rs` 三层：① 把本 crate 每个 `src/*.rs` 读回来，搜 10 个取标题的 Win32 调用与 7 个键鼠/截屏/剪贴板调用，一个都不能出现（这条测试在写的时候真的红过一次——`windows.rs` 的文档注释里提到了 `GetWindowText`）；② `AppIdentity` 要求可执行映像名，四种真实形状的窗口标题全部被拒；③ 密封体的键恰好是 `["app","duration_ms"]`，关库后扫目录里每个文件都搜不到 `excel.exe` / `outlook.exe` |
| 事件形状 | `source = collector.foreground_app`、`kind = app.foreground`、`actor_subject = self`、`privacy.subject = self`、`egress` 全 deny。应用名与时长在 `body_ref` 指向的密封体里，`content_key_id` 是本次运行的那把 |
| 审计无正文 | `tests/window_titles_are_not_collected.rs::the_audit_chain_records_the_run_and_none_of_what_was_collected`：整条链只有 `collect.start` 与 `collect.stop`，后者带 `items` 计数与 `content_key_id`；把四个应用名当语料、阈值调到 4 个 scalar 过 `LeakageChecker` |
| 测试没有手工 insert 冒充采集 | `tests/the_tests_do_not_fake_collection.rs`：扫本目录每个测试文件，`append_event` / `put_*` / `seal` / `append_audit` / `execute_forget` 一律不许出现（读接口不限）。另有一条控制用例，证明这个扫描器认得出真的写调用 |

落地内容：`crates/soul-collect/{source,fake,session,collector,runner,consent,lock,error,windows}.rs` 与四个测试文件加 `tests/common/mod.rs`；`soulcore/src/commands/collect.rs`。根 `Cargo.toml` 追加了 member 与 `soul-collect` 的 workspace 依赖项，`soulcore/Cargo.toml` 追加一行依赖，`soulcore/src/commands/mod.rs` 追加 `pub mod collect;` 与目录注释里的一句。没有新 fixture，没有动 schema。

### WP07 的取舍与遗留

1. **一次运行一把内容密钥，所以遗忘单元是「这次采集」而不是「这一天」。** `ForgetUnit::ContentKey(id)`，id 由 `CollectorHandle::content_key_id()` 交出，`soulcore::commands::collect::forget_unit` 把它包成遗忘单元。按天分钥对用户更自然（「忘掉昨天」），但那需要重启后还能找回昨天那把钥匙，也就是存储侧要能按天查内容密钥——那是 `soul-store-api` 的改动，超出本工作单。
2. **时长在密封体里，不在事件字段上。** `event.schema.json` 是 `additionalProperties: false`，唯一能装东西的地方是 `body_ref`，而它是密封指针。于是 `{app, duration_ms}` 一起密封，事件的 `ts` 是**会话开始**的那一秒。好处是应用名跟着内容密钥走，遗忘能够到；代价是研究预览目前只能按 `kind` 与小时桶聚合，看不到时长——`export-manifest` 里那个 `duration_bucket` 字段要等有人把密封体解出来聚合，那是研究轨道的活。
3. **`consent_id` 恒为 `None`。** `ConsentLedger` 记的是状态与变更时间，没有 id 可引用。契约要求这里是 uuid7，编一个出来只会让审计指向不存在的行。要填它得先让同意记录有身份。
4. **关闭有两种语义，故意不一样。** `stop()` 是有序收尾，会把用户此刻还在用的那个应用的时长写完；撤销同意则把在飞会话**丢掉**。理由是后者是用户收回授权，「再写一条」不是收回授权的一部分。两条路都在 1 秒内不再产生新事件，测试分别验。
5. **`AppIdentity` 要求可执行后缀（`.exe` / `.com` / `.scr`）。** 这是把「不采标题」从习惯变成规则的那一条：窗口标题几乎不会以 `.exe` 结尾。代价是没有这类后缀的前台进程会被拒（计入 `source_errors`，不写事件）。v0.3 的 Android 是包名不是映像名，要另加构造器，**不要**靠放松这条来支持它。
6. **打不开的进程记为「前台无内容」。** 提权或受保护的进程 `OpenProcess` 会失败，这时返回 `Ok(None)`，那段时长就丢了。反过来从窗口去猜名字，等于放弃「只采应用」的承诺。
7. **时长用墙钟，不是单调钟。** 事件的 `ts` 必须是墙钟，两头用同一个时钟才不会自相矛盾；NTP 回拨时 `duration_ms` 走 `saturating_sub` 记 0，`session.rs` 有测试钉住。
8. **采集线程与调用方共享 `Arc<Mutex<SqlCipherStore>>`。** `soulcore::commands::collect::share` 是那个包装。整个进程只能有一个 store 句柄，否则两个连接会各写各的 WAL。**WP13 第二段落地：** `soulcore::commands::session::Session` 是唯一开库的地方，`Session::store()` 发出去的是同一个 `Arc` 的克隆；`apps/desktop/src-tauri/tests/one_store.rs` 回读壳自己的源码，壳里出现 `open_store` 就红。
9. **`e0-audit` 在本机会被 `apps/desktop/dist/` 命中。** 那是 WP09 的构建产物，`.gitignore` 里有它，但 `xtask` 的 `EXEMPT_DIRS` 没有 `dist`，所以本机跑会报 20 条 URL。干净检出（我在 `/tmp` 克隆 HEAD 验过）三项全绿，CI 也是干净检出。要不要给 `EXEMPT_DIRS` 加 `dist` 由 WP09 或 WP13 决定，本工作单不动 `xtask`。
10. **没有加 `soulcore/tests/collect_commands.rs`。** 工作单允许的 soulcore 面只有 `src/commands/collect.rs`，验收测试因此全部放在 `crates/soul-collect/tests/`，它们本来也需要真库与后台线程。

## WP09 完成情况（第一段：壳）

`apps/desktop` 落地：Tauri 2 宿主 + React/Vite/TypeScript 界面。本机 `just ci` 全绿（`lint / schema / e0 / denylist / fixtures-verify / test / ui-lint / ui-test`，vitest 4 个文件 20 项）；`just desktop-test` 绿（cargo 21 项，MSRV 1.83）。本段只做壳与两个必须落地的开关，**起草 UI 全文是 WP10**，相关路由留空并写明归属。

| 交付 | 证据 |
|---|---|
| AC-01 asInvoker、无 updater、WebView 只本地、进程名按锁文档 | `src-tauri/tests/shell_is_local_only.rs` 7 项：`windows/soul.exe.manifest` 里 `requestedExecutionLevel level="asInvoker"` 且 `uiAccess="false"`，`build.rs` 确实把它嵌进去（读 build 脚本断言，不是相信约定）；`tauri.conf.json` 全文搜不到 updater 端点/公钥，`createUpdaterArtifacts: false`，`webviewInstallMode: "skip"`（安装器既不提权也不下载）；CSP 的 `default-src` 只有 `'self'`，没有任何 `http(s)://` 源；产物名与 `[[bin]] name` 都是锁文档写的 `soul` |
| WebView 的权限面就是它需要的那点 | 同文件 `the_webview_holds_only_core_permissions`：`capabilities/default.json` 只列 `core:default`，断言里逐个排掉 `fs` / `shell` / `http` / `updater` / `dialog` 前缀。`src-tauri/tests/no_egress_path.rs` 从 `cargo metadata` 的 resolve 图出发（Windows 与 Linux 两个 target 各走一遍），normal/build 边上走不到任何被禁的 HTTP client 或 Tauri 网络插件；`reqwest` 唯一的到达路径仍是 `soul-egress` |
| AC-02 向导：采集 / 云 / LLM 默认全关 | Rust 侧 `soulcore/tests/shell_commands.rs::a_finished_wizard_leaves_every_capability_off` 与 `the_wizard_would_refuse_a_configuration_with_a_switch_on`——向导**不能**产出一个有开关是开的快照，`complete_wizard` 拿到这种配置直接报错，所以「默认全关」不是初始值而是后置条件。界面侧 `src/routes/Wizard.test.tsx` 5 项：逐项断言渲染出来的每一项能力都写着关闭、页面上除了「我读过」之外没有第二个勾选框（没有一个能在向导里打开什么的控件）、没勾就点不动而且**一次核心都不问**、核心若说某项是开的向导照实显示（证明它读的是快照不是硬编码） |
| AC-22 云开关可见、写「尚未启用」、点了不出网 | `src/components/CloudToggle.test.tsx` 4 项：开关可见且文案是「尚未启用」；连点五次，五次之后仍然是「尚未启用」；说明文字与 `soulcore` 的 `CLOUD_NOT_YET_AVAILABLE_EXPLANATION` 逐字相等（`src/contract.test.ts` 另有一条跨语言比对，界面自己编一句会红）；整个测试期间 `fetch` / `XMLHttpRequest` / `WebSocket` / `EventSource` / `navigator.sendBeacon` 五个全部被替换成「一被调用就让测试失败」的桩，`forbidNetwork` 在 `src/test/fakeCore.ts`。jsdom 本来就没有真 socket，所以这一条查的是意图不是报文：它拦的是某次改版顺手加上去的一个 `fetch`。Rust 侧 `cloud_toggle` 无视 `requested_on` 返回同一个 `CloudNotice`，`no_configuration_can_claim_the_cloud_is_available` 遍历配置空间断言没有哪一种能让它说可用 |
| UI 无业务：只调 soulcore commands | 三道锁。① `eslint.config.js` 的 `no-restricted-imports` 只放行 `src/core.ts` 引用 `@tauri-apps/api`；② `src/contract.test.ts::只有 core.ts 直接引用 Tauri 的 API` 直接读每个源文件再查一遍（lint 规则被人改宽了它还在）；③ `src-tauri/tests/command_surface.rs::the_command_layer_stays_thin` 读 `src/commands.rs`，断言每个 wrapper 的函数体最多一条语句——放不下分支，也放不下循环，只够转调 `soulcore::commands::shell`。判断全在 Rust：`ConfigSnapshot.fully_closed` 与 `open_capabilities` 由核心算好送过来，界面只渲染 |
| 两侧的命令名是同一套 | `src/contract.test.ts::界面用的命令名和 src-tauri 注册的一模一样` 与 `src-tauri/tests/command_surface.rs` 3 项对着咬：TS 的 `COMMANDS`、Rust 的 `COMMAND_NAMES`、`#[tauri::command]` 的实际注册，三者集合相等。少写一个或多写一个都会红 |
| IPC 真的走通了 | `src-tauri/tests/ipc_roundtrip.rs` 7 项用 `tauri::mock_builder()` 加 **`generate_context!()`**（不是空 context）跑真 `RuntimeAuthority`——ACL 来自真的 `tauri.conf.json` 与 capabilities。三个命令都从 WebView 那一侧发请求、收 JSON：快照过去是 `fully_closed`、向导没勾选回来是错误不是默认值、云开关按哪一下都返回同一个 notice。另有反例两条：未注册的命令被拒、伪造 origin 的调用被拒；还有一条钉住参数在 WebView 侧的拼法（改 `requestedOn` 的 serde 命名会红） |
| 托盘入口 | `src-tauri/src/tray.rs`：两项菜单「打开 Soul」「退出 Soul」，每个平台都编译。关窗默认收进托盘，所以「退出」必须在托盘里够得着 |
| 界面不出现诊断词与量表词 | `src/contract.test.ts::不出现诊断词与量表词` 把 `fixtures/denylist/diagnostic_terms.txt` 读进来扫每个 `.ts`/`.tsx`/`.css`/`.html`。写这条的时候故意往组件里塞过一个禁用词验证它会红 |

落地内容：`apps/desktop/` 43 个文件——界面 `src/{App,main,core,router,styles}` 加 `src/components/{CloudToggle,NavRail,Pending}`、`src/routes/{Wizard,Home,Settings}`、`src/test/{setup,fakeCore}` 与 4 个测试文件；宿主 `src-tauri/src/{lib,main,commands,tray}.rs`、`tauri.conf.json`、`capabilities/default.json`、`windows/soul.exe.manifest`、`build.rs`、图标与 4 个测试文件。仓库面：`soulcore/src/commands/shell.rs` + `soulcore/tests/shell_commands.rs`、`justfile` 的 `ui-*` 与新增 `desktop-*`、根 `package.json` / `pnpm-workspace.yaml` / `pnpm-lock.yaml`、`.github/workflows/ci.yml`、`.gitignore`、`crates/xtask/src/egress.rs`。没有动 schema，没有动产品定义，没有新 fixture。

### WP09 的 Windows 手动缺口

Linux 上能证明的到此为止。下面每一条都要在 Windows 11 x64 真机上由作者过一遍，CI 补不了：

1. **托盘图标真的出现在通知区域（AC-01）。** 编译在 Windows CI 跑，图标在不在、tooltip 是不是「Soul」、右键菜单两项能不能点，没有 runner 有通知区域可看。托盘逻辑本身在 Linux 上用 Xvfb + AppIndicator 手动过了一遍（关窗后进程存活、窗口消失），但那不是 Windows 的实现。
2. **双击启动不弹 UAC（AC-01）。** 清单文本与嵌入都有测试，但「图标上没有盾牌、启动没有提示框」是肉眼的事。
3. **任务管理器里的进程名是 `soul.exe`。** 配置与 `[[bin]]` 有测试钉住，实际显示未看过。
4. **`tauri build` 从来没有在任何 runner 上跑过。** Windows job 只跑 `cargo test`，没做 MSI/NSIS 打包——打包要下载 WiX/NSIS，那是出网。安装器的 asInvoker 与「不下载 WebView2」目前只由 `tauri.conf.json` 的字段保证。
5. **WebView2 运行时。** `webviewInstallMode: "skip"` 意味着安装器不会去下载它。Windows 11 自带 Evergreen 运行时，但「在一台干净的 Windows 11 上双击就能开」要实测；万一开不了，正确的修法是在安装器里说清楚，不是改成让它自己下载。
6. **中文在 WebView 里的字体与 DPI。** 缩放 150% 下向导那段长说明会不会截断，只能看。
7. **点云开关时系统层面没有流量（AC-22）。** 测试证明的是代码里没有这条路径、JS 侧五个出网 API 一次都没被调、依赖图里走不到任何 HTTP client。用资源监视器看一眼进程的网络列是空的，是作者手动那一栏。

### WP09 的取舍与遗留

1. **`apps/desktop/src-tauri` 是独立的 cargo workspace，不是根 workspace 的成员。** 否则 Linux 上的 `cargo test --workspace` 会去编 `webkit2gtk`，而 Soul 不出 Linux 版——为一个不发布的平台给 CI 装一整套 GUI 依赖是错的交换。代价是根 workspace 的 `just lint` / `just test` 碰不到它，所以另开了 `just desktop-check` / `just desktop-test`，Windows job 显式跑后者。**加了新的 Rust 代码要记得它不在 `--workspace` 里面。**
2. **`src-tauri` 有自己的 `Cargo.lock` 和 `.cargo/config.toml`。** Tauri 2 的若干传递依赖（`dlopen2` 等）新版本要 edition 2024，仓库钉 1.83。用 `incompatible-rust-versions = "fallback"` 加一份单独锁文件把它们钉在能编的版本上。`cargo update` 之后要用 1.83 验一遍，不要只看 stable。
3. **`custom-protocol` 没有设成默认 feature。** 开着的话 `cargo check` 会要求 `dist/` 先存在，于是「跑 Rust 测试」就先要跑一次前端构建。打包时 `tauri build` 自己会开它。
4. **CSP 的 `connect-src` 显式放行 `ipc:` 与 `ipc.localhost`。** 严格的 `'self'` 在 Windows 上会掐断 IPC——Tauri 在那边走 `http://ipc.localhost`。这不是放宽出网：两个都是本机协议端点，`default-src` 仍然只有 `'self'`，`shell_is_local_only.rs` 把这两项写成白名单，多一个源就红。
5. **`e0-audit` 的 build output 豁免改成按标记文件认。** WP07 遗留 9 说 `apps/desktop/dist/` 会让本机 e0 红。修法不是把 `dist`/`gen` 加进 `EXEMPT_DIRS`（那样任何目录改个名字就能躲开审计），而是只在旁边有 `package.json` / `tauri.conf.json` 时才跳过。`xtask/tests/self_test.rs` 里有一条写了个手写的 `crates/pretend/src/gen/`，它仍然会被扫到。
6. **托盘装不上时窗口就正常关闭。** 关窗收进托盘只有在真有托盘时才成立；没有通知区域的桌面上，那会变成关不掉又退不出的窗口。`tray::install_or_report` 把这次会话有没有托盘记进 state，关窗处理读它。Windows 11 一定有托盘，这条是给别的环境和调试用的。
7. **`soulcore/src/commands/shell.rs` 里的 `ConfigSnapshot` 是壳自己的视图，不是 `Config` 的序列化。** 它只带界面要显示的那几个布尔与计数，**不带 LLM 端点字符串**（`the_snapshot_carries_no_endpoint_string` 钉住）：界面没有理由拿到那个地址，而每一个跨进程边界的字符串都是一次泄漏机会。要显示端点内容，得先想清楚为什么。
8. ~~**壳还没有连真的 store。**~~ **已消除（WP13 第二段）。** `run` 在 `lib.rs` 里造一个 `Session` 并 `manage` 起来，`configure` 把它当参数收，每个命令拿 `State<'_, SessionState>`。「一个进程一个句柄」因此是调用图上的性质，不是习惯：第二次 `configure` 得有人专门再造一个 session 递给它。
9. **起草 / 文件计划 / 导入 / 记忆 / 人脉这些路由是空的，但不是白屏。** `components/Pending.tsx` 写明这一页归哪个 WP。`App.test.tsx` 里两条断言钉住空路由的形状：起草页没有输入框也没有发送按钮，文件计划页没有任何执行按钮——工作单禁止假实现，测试就是这条禁令的执行者。要在这些页面上加控件的人会先撞到它们。（WP10 已接起草页：那条断言现在读作「有输入框，没有发送按钮」，后半句一个字没改，这正是它当初的用途。）
10. **前端只有 4 个测试文件，没有组件快照。** 断言全是「用户能看见什么」（`getByRole` / 可见文本），不是 DOM 结构。快照测试会在 WP10 改版式的时候整片变红，却挡不住把云开关文案改掉这种真问题。（WP10 加了第 5 个，同样的写法。）

## WP10 完成情况

`crates/soul-draft` 落地：语气简报、无 key 的确定性模板、端点起草、回复解读、人事摘要。`soulcore/src/commands/draft.rs` 是命令面，`apps/desktop` 的 `/draft` 从空路由接上了其中的本机那一半。本机 `just ci` 全绿（`lint / schema / e0 / denylist / fixtures-verify / test / ui-lint / ui-test`；workspace 75 个测试目标 417 项，vitest 5 个文件 31 项），`just desktop-test` 绿（25 项）。

PRODUCT_LOCK 的「出站消息 v0.1 只起草，不发送」是这份工作单的形状，不是它的一条检查项：这个 crate 里没有收件人、没有通讯录、没有一个意思是「送达」的动词。唯一能上 socket 的东西是一次生成请求，去的是 `EgressPermit` 点名的那个 origin。

| 交付 | 证据 |
|---|---|
| AC-17 无 key 时确定性模板 | `tests/voice_and_template.rs`：`with_no_endpoint_the_draft_is_a_template_and_nothing_is_contacted` 让一个真的 `MockLlm` 在旁边听着，`request_count()` 是 0；`the_template_is_a_function_of_its_inputs_and_nothing_else` 用每轮新造的 `Drafter` 跑同一输入 32 遍，全部相等；`changing_any_one_voice_field_changes_the_draft` 证明它不是一句写死的话。走这条路时**根本不构造请求体**——不是构造完了不发，所以没有东西可漏 |
| AC-17 模板说的话产品能说 | `every_voice_combination_renders_something_this_product_may_say`：81 种语气组合 × 两种上下文全部过 `assert_non_clinical` |
| AC-07 推断不覆盖用户锁定 | `a_pinned_voice_field_survives_an_inference_that_disagrees_with_it` 与 `the_prompt_carries_the_users_value_and_not_the_inferred_one`：用户设过的字段，再来一条相反的推断，**送进 prompt 的那份文本**里是用户的值。断言落在渲染出来的简报上而不是内存里的结构体上，因为模型读到的是前者 |
| AC-07 简报只带有证据的轴 | `an_axis_with_no_evidence_stays_out_of_the_prompt`：`SupportedBand::None` 的轴不进简报 |
| AC-12 第三人正文默认占位 | `tests/wire.rs::the_third_partys_words_do_not_reach_the_wire`：断言的对象是 `MockLlm` 实际收到的字节，不是本进程里的中间值。`LeakageChecker` 同时找第三人正文、姓名和手机号 |
| AC-13 单次豁免下次回到占位 | `one_exemption_covers_one_request_and_the_next_is_placeheld_again`：同一个 `Drafter` 连发两次，第一次带原文，第二次自己回到占位。豁免是按值传进去然后就地丢掉的，`ReplyGenerator::generate` 也按值收 `RedactedBody`——「不会被记住」是签名而不是分支。`an_exempted_body_still_placeholds_the_name_and_the_number`：整条豁免只放开这一条正文，姓名与账号仍然占位 |
| AC-11 精确 origin，跨 origin 拒绝 | `a_redirect_off_the_configured_origin_is_refused_and_the_target_never_hears_from_us`：重定向目标是另一个真的 `MockLlm`，它的 `request_count()` 是 0。`a_different_port_on_the_same_host_is_a_different_origin` |
| AC-16 每条摘要都有证据 | `tests/people_summary.rs`：图由 `soul_graph::rebuild` 从证据推导而不是手写。`a_point_with_no_evidence_is_not_a_value_that_can_exist`——`SummaryPoint::new` 收空的证据列表会失败，所以「有证据」是类型层面的；`an_edge_whose_evidence_does_not_resolve_produces_no_summary`——引了一行取不回来的证据就整份拒绝，不是悄悄少给一条 |
| AC-16 过 denylist | `the_summary_says_nothing_a_medical_product_would_say`、`a_point_that_reads_like_a_diagnosis_fails_to_build`、`a_rephrasing_that_reads_like_a_diagnosis_is_dropped_and_the_points_stand`——端点改写回来的叙述过不了就整段丢掉，回到本机计数版，点条目一条不动 |
| AC-16 摘要不点名 | `the_summary_names_nobody`：文本里没有姓名也没有 uuid，只有「这个人」 |
| AC-25 注入不进指令位 | `tests/injection.rs::nothing_from_a_paste_reaches_the_instruction_slot`：对整个语料，`system` 消息逐字节等于 `soul_policy::e1::DRAFTING_INSTRUCTION`。这条能成立是因为**没有任何运行时的值到得了那个位置**，语气简报也走引用材料槽 |
| AC-25 注入不外连 | `no_url_in_the_corpus_is_reachable` 与 `a_server_at_the_address_an_injection_names_never_hears_from_us`：注入点名的地址上真起一个服务器，它一个请求都收不到 |
| AC-25 注入被记下但不被听 | `a_detected_injection_is_written_down_and_the_draft_is_still_produced`、`an_injection_that_comes_back_from_the_endpoint_is_recorded_and_not_obeyed`、`nothing_derived_from_a_paste_can_authorize_an_action` |
| 声明是工作假设不是临床结论 | `PersonSummary::notice` 是 `工作假设，非临床结论`，`PersonSummaryView.clinical_claim` 恒为 false 且没有代码路径设它 |
| 结构上不发送 | `tests/never_sends.rs`：`Draft` 的 `delivery` 是 `NeverSent`，只序列化成 `false` 且没有别的值；`no_field_on_a_draft_could_name_somewhere_to_send_it` 拿 `DRAFT_FIELDS` 逐个比。`soulcore/tests/draft_commands.rs::this_command_surface_offers_no_way_to_send_a_message` 搜命令面的源码 |
| 命令面：批准的就是发出去的 | `soulcore/tests/draft_commands.rs` 17 项：`prepare` 描述、`generate` 执行，中间隔着一个人。`a_prepared_body_can_be_sent_once_and_not_twice`、`approving_a_shape_that_is_not_the_prepared_one_sends_nothing`、`an_approval_for_one_message_does_not_send_a_different_one_of_the_same_shape` |
| 桌面壳没有发送按钮 | `apps/desktop/src/routes/Draft.test.tsx::不管有没有草稿，页面上都没有发送的按钮`（有草稿之后再查一遍，那才是有人会加按钮的时刻）；`src-tauri/tests/ipc_roundtrip.rs::there_is_no_command_that_sends_a_draft_to_anybody`；`contract.test.ts::界面能调用的命令里没有一个是发送` |
| 界面上的话是核心的话 | `contract.test.ts::不发送与本机模板两句话和 soul-draft 里的常量一模一样`：直接读 `crates/soul-draft/src/draft.rs` 的常量，防止测试替身变成一个比真核心更友好的核心 |

落地内容：`crates/soul-draft/{brief,template,draft,reply,analysis,error,lib}.rs`（1368 行）与五个测试文件；`soulcore/src/commands/draft.rs` 与 `soulcore/tests/draft_commands.rs`；`apps/desktop` 的 `src/routes/Draft.{tsx,test.tsx}`、`core.ts` 两个命令、`src-tauri/src/commands.rs` 两个转发、`router.tsx` 一行、`styles.css` 两个类。根 `Cargo.toml` 追加 member 与依赖项，`soulcore/Cargo.toml` 追加一行。没有动 schema，没有动产品定义，没有动 `soul-policy`，没有动 `xtask`。

### WP10 的取舍与遗留

1. **语气简报走「引用材料」槽，不走指令槽。** `soul_policy::e1` 把请求定死成两格：一个常量指令，一个被指令告知「不得当作命令」的引用材料。WP10 没有把它撑开。代价是模型看到的语气说明和第三人正文在同一个信任级别里；换来的是 AC-25 的强形式——注入进不了指令位，不是因为有过滤器认得出它，而是因为**没有任何运行时的值到得了那个位置**。要把语气放进 system 消息的人得改 `soul-policy::e1`，那是权限面的改动，评审注意力本来就该落在那里。简报本身只描述不命令，所以被标成材料没有损失什么。
2. **`prepare` / `generate` 是两步，中间的东西是一个人。** 一次调用会去哈希一份没人看过的计划。请求体在两步之间被**留着**而不是重建：一次性豁免在构造时就消费掉了，重建出来的是另一份体（该带原文的地方成了占位），用户批准的计数就不再描述真的发出去的东西。留着也顺带把爆炸半径钉死——只有一个位置，第二次 `prepare` 顶掉第一次，`generate` 按值取走。
3. **批准要对上两样东西，不只是 `plan_hash`。** 计划里只有计数没有正文（这是故意的），所以两条各含一个第三人 turn 的粘贴哈希一模一样。光靠哈希，用户对 A 的批准能把 B 发出去。`Pending` 因此带一个 `preparation_id`，`Approval` 两样都要echo回来。这条是写测试的时候撞出来的，不是设计时想到的。
4. **`summarize_person` 不写审计。** `docs/schemas/audit.schema.json` 是冻结的，里面没有一个属于人事摘要的动作。它读图和图已经引用的证据行，什么都不改；在这里现编一个动作等于让审计条目声称一件契约没说过的事。要不要加是改 schema 的事。
5. **端点回复不能用时降级，不报错。** 读不出来、空的、或者带了这个产品不说的词，都退回本机模板并在 `Draft::degraded` 里说清楚是哪一种。那是内容问题不是权限问题，用户还是该拿到一份草稿。跨 origin 被拒这类**是**权限问题，照样往上抛——AC-11 要的是它可见，不是被兜住。
6. ~~**桌面壳只接了本机那一半。**~~ **已消除（WP13 第二段）。** 端点路径的确认屏接上去了：`prepare_draft` 给一屏计数与两个标识，`generate_draft` 要把那两样原样 echo 回来。屏上没有第三人的正文——计划里本来就只有计数，把正文放回去等于让人批准一段没有重读过的话。本机路径没有变。
7. **壳里的 `KnownIdentifiers` 是空的。** 填它要通讯录，通讯录要一个打开的 store，那还是 WP13。按形状的清洗照常抓地址、handle 和长数字串，第三人 turn 无论里面是谁都整条占位，所以空名单降低的是精度不是底线。`closed_session()` 把两个 session 一起造出来，就是为了不会有人给它们两份不一样的名单。
8. ~~**人事摘要的视图没有接到界面上。**~~ **已消除（WP13 第二段）。** `/graph` 画节点、边与每条边背后的证据条数，摘要按核心写的那几句原样渲染。屏上没有名字：标签在库里是密封的，这条路径不打开它，人靠标识摘要的前几位区分。`Graph.test.tsx` 把 `xtask` 那份 denylist 跑在**渲染出来的 DOM** 上。
9. **`Draft.test.tsx` 里的注入地址没有写 scheme。** `xtask e0-audit` 扫这棵树的 URL 字面量，豁免的是 `tests/` 目录，而 TypeScript 的测试是贴着源文件放的 `.test.tsx`，不在豁免里。带真 scheme 的语料在 `crates/soul-draft/tests/injection.rs`（那里是 `tests/` 目录）。本工作单不动 `xtask`；要不要给 `.test.ts(x)` 也豁免，由拿到 `xtask` 的工作单决定。
10. **`ReasonCode` 借了 `PLAN_HASH_MISMATCH`。** `NothingPrepared` 和 `NotThePreparedRequest` 在冻结的词表里没有自己的词，而这两种情况说的都是同一句话：你批准的那份东西不是现在要发的这份。和 WP11 借 `CONSENT_MISSING` 是同一个取舍。
11. **人事摘要的档位来自交互计数，不是判断。** `soul-graph` 的 `MODERATE_MIN_INTERACTIONS` / `STRONG_MIN_INTERACTIONS` 是它的全部内容，每条点条目都把自己的计数写在句子里。这很笨，但一份人跟不上的推理不是可以被同意的东西——和 WP11 整理规则故意很笨是同一条理由。
12. **草稿框是可以改的，改完不回传。** 那是用户自己的文字了。`Result` 组件按次数 key，所以第二版草稿到的时候上一版的编辑不会跟着活下来。

## WP11 完成情况

`crates/soul-fileplan` 落地：只读目录扫描、整理计划预览、`plan_hash`，以及一个没有成功分支的执行拒绝面。本机 `cargo test -p soul-fileplan -p soulcore` 绿（44 项），`cargo test --workspace --all-targets` 72 个测试目标全绿，`xtask` 三项（e0-audit / denylist-audit / schema-freeze --check）绿，`cargo clippy --workspace --all-targets --all-features -D warnings` 绿。

D31 是这份工作单的边界：只读预览留在 Goal 1，写执行是 v0.1.1（AC-27）。所以这里没有 `execute`，也没有一个将来可以填上的 `Ok` 分支。

| 交付 | 证据 |
|---|---|
| AC-18 未授权 100% 拒绝 | `tests/unauthorized_paths.rs::every_way_of_naming_the_unauthorized_directory_is_refused`：语料按「一条路径能被写成另一条路径的方式」构造——`..`、`.`、符号链接（出去的和留在里面的）、混用分隔符、大小写、`\\?\` / `\\.\` / `\\server\share`、盘符相对、保留设备名（`NUL` / `con.txt`）、尾随点与空格、备用数据流、控制字符、以及把根名当前缀的 `AlphaExtra`。30 条全部被拒且拒绝理由都属于「授权类」，语料长度本身被断言，缩水会红。同一批语料再走一遍 `preview`（UI 走的那道门），拒绝后目录逐字节不变 |
| AC-18 授权侧真出计划 | `tests/authorized_scan.rs`：四条 move 逐条点名（`budget.csv → 表格/`、`photo.jpg → 图片/`、`report.pdf → 文档/`、`笔记.txt → 文档/`），八条 left-alone 连理由一起点名。没有这一半，上面那个 100% 用「永远返回错误」就能拿到 |
| AC-18 扫描前后快照不变 | 同上：三条独立断言——crate 自己的 before/after `DirectorySnapshot` 相等；测试自己写的 `walk()`（不调用 crate 的哈希）在授权目录上前后逐项相等；同一个 `walk()` 在**整棵树**（含未授权的 `Bravo`）上前后相等。`tests/no_write_api.rs` 里连跑五次预览再比一遍 |
| AC-19 未知动作 | `tests/execution_is_refused.rs`：`plan.execute` / `file.move` / 大小写变体 / 空串全部 `UNKNOWN_ACTION`；另一条对 `ActionKind::ALL` 里**不属于本面**的每一个动作断言同样的答案——文件计划面认得 `forget.execute` 就等于给遗忘开了第二道门 |
| AC-19 plan_hash 变则拒 | 同上：改一条 move 的目的地、改一个计数字段、以及「批准之后用户往目录里存了一个文件再重扫」三条路径，都得到 `PLAN_HASH_MISMATCH` |
| AC-19 令牌重放拒绝 | 同上：先真发一张 `ForgetExecute` 令牌并真消费掉，再拿它来请求执行，得到 `TOKEN_REPLAYED` 且审计动作是 `capability.reject` |
| v0.1 不消费写文件令牌 | `refuse_execution(issuer: &TokenIssuer, …)` **不可变**借用账本，所以「不消费」是签名而不是分支。`a_file_write_token_is_neither_honoured_nor_spent`：发一张 `FileWrite` 令牌，请求执行被拒，`is_spent` 仍为 false，再请求一次仍是同一个 `WRITE_NOT_IMPLEMENTED`（若第一次偷偷烧掉，第二次会变成重放）。`FILEPLAN_ACTIONS` 里没有一个动作 `needs_capability_token` |
| 完美请求也拒 | `a_request_with_nothing_wrong_with_it_is_refused_anyway`：用户发起、令牌有效、计划与批准的哈希一致，返回 `WRITE_NOT_IMPLEMENTED`。返回类型 `ExecutionRefusal` 不是 `Result`，没有成功变体可构造 |
| 全 crate 禁写 API | `tests/no_write_api.rs` 三层：① 扫 `src/*.rs` 每一行（去掉注释）找 28 个写调用与存储写半边；② 读 `Cargo.toml`（去掉注释）断言 normal 依赖里没有 `soul-store` / `soul-collect` / `soul-import` / `tempfile`；③ 跑五次预览再比整棵树。控制用例喂五行真的写调用证明搜索认得出来，另喂三行读调用证明它不会误伤。还有一条断言公共面没有 `fn execute` / `apply` / `perform` / `commit` / `undo`，且 `execute.rs` 里不出现 `Result<` |
| AC-25 文件名通道 | `tests/file_names_are_data.rs`：五个敌意文件名真落在磁盘上，扫描把它们计进 `injection_signals` 并出 `injection.blocked` 审计（只有计数），但仍然当数据留在计划里；对每个名字 × `ActionKind::ALL`，以 `ExternalContent` 发起全部被拒；名字提到的任何 URL 过 `NetGuard::closed()` 全拒。计划里能搜到 `李雷的照片.jpg`（控制），三条审计条目搜不到 |
| 计划哈希稳定且可比 | `the_plan_hash_is_stable_until_the_directory_is_not`：同一目录两次预览哈希与计划完全相等；存进一个文件后哈希变、move 多一条。`executable_in_this_version: false` 在被哈希的值里面，把它改成 true 哈希就变 |
| 目的地不出授权根 | `no_proposed_destination_leaves_the_authorized_root`：每条 move 的目的地都走同一个 `resolve`（对尚不存在的路径也成立），并断言它确实还不存在 |

落地内容：`crates/soul-fileplan/{screen,authorize,refusal,kind,scan,plan,execute,preview}.rs` 与四个测试文件加 `tests/common/mod.rs`；`soulcore/src/commands/fileplan.rs`。根 `Cargo.toml` 追加了 member 与依赖项，`soulcore/Cargo.toml` 追加一行，`soulcore/src/commands/mod.rs` 追加 `pub mod fileplan;` 与目录注释里的一句。没有动 schema，没有动产品定义，没有新 fixture，没有动 `xtask`。

### WP11 的取舍与遗留

1. **路径筛查在原始字符串上做，不在 `Path` 上。** `Path` 每个平台解析得不一样：Linux 上 `\\?\C:\Windows` 是一个无害的相对段，没有根也没有分隔符，Windows 上它是绕过 Win32 规范化的 verbatim 设备路径。照着 `Path::components` 写的规则会在跑测试的机器上说没事、在发布的机器上才出事。所以 `screen.rs` 一律按字符判断，Windows 的那几条（保留设备名、尾随点与空格、备用数据流）在 Linux 上也生效——只在没有测试的平台上生效的规则等于没有规则。代价是 Linux 上一个合法含冒号的文件名会被判为不可计划（`SkipReason::UnplannableName`），这是有意选的保守方向。
2. **包含性判两遍，符号链接一律不跟。** 词法一遍（对着用户授权时给的写法）、规范化一遍（`canonicalize` 解完链接之后）；中间还把根以下的每一段用 `symlink_metadata` 看一眼，是链接就拒，**指回自己目录里的链接也拒**。比包含性要求的更严，但它是「文件系统在检查和读取之间变了」也仍然成立的那条规则。`the_lexical_and_canonical_checks_disagree_about_a_link_on_purpose` 单独证明第二张网不是冗余的：一条出根的链接在词法上完全在根里面。
3. **大小写是唯一不能到处一样的规则。** 折叠大小写会让包含性**更容易**成立，所以把 NTFS 的规则用在大小写敏感的文件系统上，会把 `/A/secret` 判进 `/a` 这个从没被授权的根里。`PathMatching` 因此是显式的两个变体，默认取平台真实行为，两个变体都在 Linux 上跑过。Windows 侧只有 `cfg!(windows)` 这一行没有被真机验过。
4. **`ReasonCode` 借了 `CONSENT_MISSING`。** WP08 冻结的词表里没有 `PATH_NOT_AUTHORIZED`，而扩一个权限词表不是 WP11 该做的事。`CONSENT_MISSING` 对每一次授权类拒绝都是真话：用户从没同意过那个目录，这就是答案是「不」的全部原因。「读不到」和「不是目录」两种不是产品拒绝的情况记 `ROUTINE`。将来若要加词，`Refusal::reason_code` 是唯一要改的地方。
5. **`refuse_execution` 把 HITL 的三步又写了一遍。** 为的是能不可变借用账本（见上表「不消费写文件令牌」那一行）。第二份拼写靠 `the_refusals_agree_with_the_policy_gate` 保持诚实：同一个请求，本面给的理由码必须等于 `soul_policy::hitl::check_action` 给的。
6. **UI 视图没有接，`/files` 仍是 WP09 留的空路由。** 视图类型 `soulcore::commands::fileplan::PlanPreview` 已经就绪并有序列化形状测试（含 `deny_unknown_fields` 往返），接上去只差 `core.ts` / `commands.rs` / `router.tsx` 那几处注册。没有当场接的原因是本工作单与 WP10 起草 UI 在同一个工作树里并行，两边要改的正是同一批文件（`core.ts`、`contract.test.ts`、`src-tauri/src/commands.rs`、`router.tsx`、`App.test.tsx`），并发读改写会互相吞掉改动。`App.test.tsx::文件计划页没有任何执行按钮` 仍然是那条禁令的执行者，接视图的人会先撞到它——这正是它存在的意义。
7. **没有加 `soulcore/tests/fileplan_commands.rs`。** 工作单允许的 soulcore 面只有 `src/commands/fileplan.rs`，所以命令面的六项测试写在模块内的 `#[cfg(test)]` 里。其中「本面没有执行入口」那条要把禁用的名字拼出来才能找它们，第一次跑的时候找到了自己，现在先把测试模块以下的部分切掉再搜。
8. ~~**授权列表不落盘。**~~ **已消除（WP13 第二段）。** 授权的根写进库旁边的 `config.json`，重启之后 `FilePlanSession::from_config` 从它恢复；不再解析得开的根照旧当成拒绝报出来，出现在 `/files` 的「找不到的目录」一栏，而不是从名单里消失。
9. **快照比的是 mtime 与长度，不是 atime。** 在挂了 atime 更新的文件系统上，`read_dir` 会动目录的访问时间。那是「读」的固有代价而不是写，快照要是把 atime 也算进去，每次扫描都会自己判自己失败。所以「磁盘没变」的准确含义是：没有条目增减、没有长度变化、没有修改时间变化。扫描不打开任何文件，所以文件的 atime 也不动。
10. **泄漏检查用了两把尺子。** 中文短名用 4 个 scalar，英文名用产品自己的 `≥8`。原因是 `ctio` 是 `injection.blocked` 的片段而不是用户的内容，`prev` 是审计链自己的 `prev_hash`——在英文上把阈值压到 4，抓到的是契约的字段名。姓名/账号那条规则对两边都是任意长度生效，`李雷` 靠的是它。
11. **整理规则故意很笨。** 只把散在最外层的文件按扩展名分进一层分类文件夹，目录不动、子目录里的不动、认不出的不动、目标已被占用的不动。理由是预览是给人批准的，一个人跟不上的推理不是可以被同意的东西。扩展名是猜测，所以 `kind.rs` 只按名字判断，一个字节都不读——一个仍然会打开每个文件的只读承诺比听上去要小。
12. **`soul-fileplan` 不依赖 `soul-store`，因此扫描结果不落库。** PRODUCT_LOCK 把目录文件元数据采集和写执行一起推到 v0.1.1；没有存储依赖，「扫描不会悄悄变成采集」就不需要靠自觉。预览活在内存里，唯一比它活得久的是一条只带计数与哈希的审计条目。
13. **扫描有上限**（深度 8、条目 20000）。撞上限时 `truncated` 为真并出 `SkipReason::DepthLimit` / `EntryLimit`，不是安静地少显示一些。上限值是拍的，等真机上有人对着家目录跑一次再调。
14. **Windows `canonicalize` 给出 `\\?\C:\...`，那是本地盘，不是 UNC。** `screen` 把 `\\?\` 后紧跟盘符的形式剥成 `C:\...` 再走其余规则；`\\?\UNC\`、`\\.\`、`\\server\share` 仍然拒绝。不剥的话，windows-latest 上临时目录连授权都过不了（`authorized_scan.rs` 在 `authorize Alpha` 红）。未授权语料仍是对着已授权根去 `resolve`，能筛过本地 extended-length 写法不会把 Bravo 放进来。

## WP13 完成情况（第一段：安装 smoke、CI、SBOM）

本机 `just ci` 全绿（`lint / schema / e0 / denylist / fixtures-verify / test / smoke-lint / sbom / ui-lint / ui-test`；workspace 79 个测试目标 444 项，vitest 5 个文件 31 项），`just desktop-test` 绿（25 项，含 `ipc_roundtrip` 11 项——它在 Linux 上跑得起来，红的是 windows-latest，见下）。

这一段做的是**证据链的最后一环**：AC-21 到目前为止只有「每个 crate 各自不出网」，AC-01 与 AC-26 的打包那一栏一直是空的。WP13 的另一半（壳接 `SqlCipherStore` 与配置、`/files` 与 `/graph` 接视图、起草端点的确认屏）见下一节。

| 交付 | 证据 |
|---|---|
| AC-21 主流程，不是逐 crate | `crates/soulcore/src/headless.rs`：向导 → 加密库 → 导入 → 图谱 → 档案（含被推断顶不动的那条轴）→ 记忆 CRUD 与遗忘 → 无 key 起草 → 人事摘要 → 文件计划预览与执行拒绝 → 研究预览 → 采集（同意关）→ 云开关 → 审计链，十三步在**同一个进程**里按顺序跑完，每一步证不出自己声称的事就返回 `HeadlessError` 而不是打一行日志。这是逐 crate 测试看不见的那种泄漏该出现的地方 |
| AC-21 非回环连接 = 0，观察来的不是推理来的 | `crates/soulcore/src/netwatch.rs`：Linux 上读 `/proc/self/fd` 拿到本进程的 socket inode，再对着 `/proc/net/{tcp,tcp6,udp,udp6}` 把它们解析成对端地址，整个主流程期间持续采样。`crates/soulcore/tests/netwatch.rs` 证明这个观察器不是空转——真开一个到文档地址（RFC 5737 / 3849）的 socket，它必须看得见；同时还有一份合成 `/proc/net` 表喂给解析器。平台不支持时报 `Unsupported` 而**不是**报 0：一个总是说「没看到连接」的观察器和一个坏掉的观察器长得一模一样 |
| AC-21 关着的守卫真的拒绝 | 同上，`egress_findings`：三个不可能存在的地址（`.invalid` 加两段文档网段）过 `NetGuard::closed()`，逐个记下拒绝理由码。地址在源码里写成裸 authority 再在运行时拼成 URL——`xtask e0-audit` 扫的就是 URL 字面量，这个文件不豁免 |
| AC-02 装完之后仍然全关 | `soul-headless smoke` 的报告里 `config.fully_closed` 与 `config.open_capabilities`，`install-smoke.ps1` 在真机上断言它们。`soul-headless config` 另有一条：默认配置里有开关是开的就**拒绝启动**，不是打印出来让人自己看 |
| AC-01 静默安装 → smoke → 卸载 | `scripts/install-smoke.ps1`（533 行）：NSIS `/S` 装（`.msi` 走 `msiexec /qn`，留给还不存在的 WiX 目标）→ 卸载项必须在 HKCU（按用户装）→ 装出来的可执行文件叫 `soul.exe` 且 PE 里的清单是 `asInvoker` → `soul-headless smoke` 在 `Get-NetTCPConnection` 盯着它的 TCP 表的情况下跑 → `uninstall.exe /S _?=<dir>` 卸干净且注册项消失。退出码是全部接口，`finally` 保证卸载在前面任何一步失败后照样跑 |
| AC-01 不提权，由脚本自己证明 | 同上：安装前断言**本进程不是管理员**。要提权的安装器在这里会弹 UAC，而 `/S` 压不住它——这正是 AC-01 要抓的那件事。清单检查除了要 `asInvoker`，还逐个排掉 `requireAdministrator` / `highestAvailable` 并要求 `uiAccess="false"` |
| 脚本不下载任何东西 | `crates/soulcore/tests/install_smoke_script.rs` 6 项：`Invoke-WebRequest` / `Invoke-RestMethod` / `Start-BitsTransfer` / `WebClient` / `curl` / `wget` / `winget` / `Install-Module` 一个都不许出现，全文搜不到 `://`。另外 `xtask e0-audit` 现在也扫 `scripts/`（`.ps1` / `.psm1` / `.sh` / `.bat` / `.cmd`），`xtask/tests/self_test.rs::the_url_scanner_reads_packaging_scripts` 喂一个合成的脚本证明它会红 |
| 脚本读的字段真的存在 | 同上：从脚本源码里**扫出**每一个 `$smoke.Report.<path>`，再拿一份真跑出来的报告逐个解析。CI 的 Linux job 不执行这个脚本，所以「报告字段改了名，脚本拿 `$null` 去和 0 比，smoke 在一台正在出网的机器上绿了」是唯一没人会发现的失败模式，这条就是堵它的 |
| SBOM | `crates/xtask/src/sbom.rs`（502 行）：从 `cargo metadata` 的 resolve 图出发，只走 normal 与 build 边，两份 CycloneDX 文档——`soul-core`（215 个组件，13 个 shipped root，测试器材排除在外）与 `soul-desktop`（514 个组件，安装器里装的其实主要是它）。`self_test.rs` 里 7 项：测试器材不进文档、同一个 checkout 两次输出逐字节相等、生成的文档里搜不到 `://`（SBOM 是要发出去的东西，往里放 URL 等于给自己开一个例外）、`Cargo.lock` 的校验和配对、没写许可证的依赖会被点名。`cargo cyclonedx` 没有用：那是又一个要装的二进制，而这份文档描述的正是一张没有 HTTP client 的依赖图 |
| SBOM 的第二份证词 | CI 的 lint job 导出 `cargo deny list` 的许可图（`--layout crate --format json` 与 `--layout license --format tsv`）为产物。`xtask sbom` 说什么 crate 以什么 SPDX 表达式发货，cargo-deny 用它自己的话再说一遍，两边不一致就看得见 |
| AC-09 / AC-10 真机采集有得跑 | `soul-headless collect-probe --i-consent [--seconds N]`（`crates/soulcore/src/collect_probe.rs`）：两段计时，采集关的那段 `start` 被拒且事件数不变，采集开的那段必须写出至少一条——写不出来算失败，「0 条」在这里不是更安全而是探针没看见桌面。然后**只撤销同意、不停止采集器**，隔 1.2 秒读两次事件数，相等才算过 AC-10。临时库跑完删掉，`tests/collect_probe.rs` 断言它用的是 `open_test_store` 而不是真库、且撤销发生在停止之前 |
| 作者手动那一栏有文档了 | `scripts/author-manual-checklist.md`：七节，托盘 / 不提权 / `soul.exe` 进程名 / 云开关的系统层面无流量 / 真机采集 / WebView2 与 DPI / 记录方式。每一条要么是一条带预期读数的命令，要么是一件要看的事加上「看到什么算失败」 |
| AC-26 | `just ci` 多了 `smoke-lint` 与 `sbom` 两步；CI 多了 `sbom` job（CycloneDX 产物）与 `package` job。Windows job 加了 `--test ipc_roundtrip --no-run` |

落地内容：`crates/soulcore/src/{headless,netwatch,collect_probe}.rs` 与 `bin/soul-headless.rs`、四个测试文件（`headless_main_flow` / `netwatch` / `install_smoke_script` / `collect_probe`）；`crates/xtask/src/sbom.rs` 与 `main.rs` / `lib.rs` / `egress.rs` 的接线加 `tests/self_test.rs` 的 8 项；`scripts/install-smoke.ps1` 与 `scripts/author-manual-checklist.md`；`justfile` 三条新 recipe；`.github/workflows/ci.yml`。没有动 schema，没有动产品定义，没有新 fixture，没有加依赖。

### WP13 的 Windows 手动缺口

CI 到此为止。下面每一条都要在 Windows 11 x64 真机上由作者过一遍，步骤在 `scripts/author-manual-checklist.md`：

1. **`tauri build` 仍然没有在任何 runner 上跑过（WP09 缺口 4 未消除）。** `tauri build` 第一次打 NSIS 包时会去取 NSIS 工具链，那是出网。为了证明安装器可信而去下载一个第三方安装器，是把自己绕进去了，所以打包这一步标为**作者手动**。`package` job 走到打包之前为止：前端产物、带 `custom-protocol` 的 release 二进制、从**编出来的 `soul.exe` 里**读清单确认是 `asInvoker`（不是读 `tauri.conf.json` 的字段）、以及 Windows 上的 AC-21 headless smoke，两个二进制作为产物上传，作者拿下来直接打包即可，不用重编一遍。
2. **安装包里没有 `soul-headless.exe`。** Tauri bundle 只放 `mainBinaryName`，所以 `install-smoke.ps1` 的 `-Headless` 指的是同一 commit 编出来的那个，不是安装器放上去的那个。这一步证明的是「这个 build 的核心在这台机器上跑起来不出网」，**不是**「安装器放上去的核心不出网」。要消除它，得让 bundle 带上第二个二进制（`tauri.conf.json` 的 `externalBin`），那是产品打包形状的改动，本工作单不动。
3. **托盘图标、UAC 弹窗、任务管理器里的进程名。** 编译在 Windows CI 跑，清单在 CI 里从二进制里读出来验过，但「通知区域里有没有那个图标」「双击的时候屏幕暗没暗一下」「那一行显示的是不是 `soul.exe`」是肉眼的事。检查清单第 2、3、4 节。
4. **`Get-NetTCPConnection` 只看 TCP。** Windows 的 UDP 端点表没有远端地址，所以脚本看不见 UDP 对端。Linux 侧的 `netwatch` 两个都读，两边合起来才是覆盖。检查清单第 5 节让作者手动看一眼 `Get-NetUDPEndpoint`。
5. **WebView2 的进程不是 `soul.exe`。** `msedgewebview2.exe` 有它自己的网络行为。云开关那一节明确要求把它单独记一条，不要含糊地算进「Soul 出网了」或者「没事」。
6. **AC-09 / AC-10 的真机那一半。** 所有采集测试都驱动 `FakeForegroundSource`，因为 runner 没有桌面。`collect-probe` 是给这一半准备的工具，但它要一个人在键盘前切二十秒窗口，所以结果只能手填回来。
7. **卸载会不会删掉用户数据、Defender/SmartScreen 会不会拦未签名的安装器。** 两条都要实测并记录，都是发布前要处理的事。

### WP13 的取舍与遗留

1. **SBOM 自己写而不是装 `cargo cyclonedx`。** 理由有两条，第二条才是主要的：一是又一个要装的二进制（CI 里 `cargo-deny` 已经是特例，靠下载预编译包解决），二是这份文档存在的意义就是描述一张没有 HTTP client 的图，为它引入一个带 HTTP client 的生成器，逻辑上说不过去。代价是 `sbom.rs` 502 行要自己维护 CycloneDX 1.5 的形状；它只发 `metadata` / `components` / `dependencies` 三块，用到的字段都在 `self_test.rs` 里钉着。
2. **SBOM 里不放 URL，包括 crates.io 的。** CycloneDX 通常带 `externalReferences`（仓库地址、下载地址）。这里一个都不放，因为 `xtask e0-audit` 扫 URL 字面量，而一份把 URL 写进产物的生成器等于给自己开了个例外。crate 的身份靠 purl（`pkg:cargo/serde@1.0.219`）与 `Cargo.lock` 里的 sha256 校验和表达，两样都不是地址。
3. **`soul-desktop` 那份 SBOM 是 `cargo metadata` 解出来的，不是 `tauri build` 产出的清单。** 它列的是「编 `soul.exe` 要用到的 crate」，不是「安装包里有哪些文件」。WebView2 运行时、NSIS 自己放进去的东西、图标资源都不在里面。要一份真正的安装包清单，得在打包之后对着 bundle 生成——那要先解决缺口 1。
4. **`netwatch` 在非 Linux 上是 `Unsupported`，不是 0。** Windows 的等价物是 `GetExtendedTcpTable`，那要么引 `windows-sys` 要么写 `unsafe`，而 `soulcore` 是 `forbid(unsafe_code)`。选择是：Rust 侧诚实地说「这台机器上没看」，Windows 侧的观察交给 `install-smoke.ps1` 的 `Get-NetTCPConnection`。报告里 `egress.observed` 就是这个区别，别把它读成通过。
5. **`collect-probe` 是仪器，不是产品面。** 没有任何 shell 命令到得了它，它只在 `soul-headless` 这个二进制里，而且要 `--i-consent`。这是有意的：界面上还没有采集开关（WP07 落的是 crate 与命令面，壳没接），在壳里现加一个只为了手动测试的开关，等于让测试需求决定产品形状。
6. **`collect-probe` 写的那条 `collect.start` 审计是探针自己补的。** `ConsentHandle::grant` 把审计条目交回给调用方，由拿着 store 的人写；平时那个人是壳，这里是探针。链里少一条「采集被打开过」的记录，不是 AC-23 要的那条链。
7. **`ipc_roundtrip` 在 windows-latest 上仍然不跑，只编译。** 失败发生在测试进程启动阶段（`STATUS_ENTRYPOINT_NOT_FOUND` / 0xc0000139），runner 的 `WebView2Loader.dll` 没有导出 mock IPC runtime 要的符号，一条断言都还没跑到。加了 `--no-run` 之后，这个文件里的编译错误会当场红，而不是等作者本地跑才发现。它在 Linux 上 11 项全过（`just desktop-test`），AC-02 与 AC-22 另有 soulcore 与 vitest 覆盖。
8. **`package` job 会编一次 Tauri 的 release，很慢。** 换来的是「装到用户机器上的那个 `soul.exe` 确实是 `asInvoker`」这句话有二进制层面的证据，而不只是配置字段。要是这个 job 的时间变成问题，先砍的应该是它的触发条件（比如只在 tag 上跑），不是砍掉从二进制里读清单那一步。
9. **`install-smoke.ps1` 用 `.gitattributes` 钉成 CRLF。** PowerShell 对 LF 其实无所谓，但脚本是给 Windows 作者双击着用的，混行尾在 `git diff` 里很吵。`install_smoke_script.rs` 用 `include_str!` 读它，所以断言都写成单行片段，不跨行。
10. **`just ci` 里 `sbom` 是检查不只是产物。** 一个没写许可证的依赖会让它失败——`deny.toml` 判的是 SPDX 名字的允许清单，而「条款根本没人写下来」是那份清单唯一说不上话的情况。目前 `LicenseRef-Soul-Proprietary` 14 个（本仓库自己的 crate）、`LicenseRef-LICENSE` 2 个（依赖自带的非 SPDX 声明）。
11. **`headless::run` 用固定时钟（`AT_UNIX_SECONDS`），`collect_probe` 用墙钟。** 前者是为了两次运行写出同一条审计链，后者不行：它写的是「刚才这一次采集」，时间戳编一个出来就成了假话。`headless::now_unix_seconds` 是这条分界。
12. **主流程用的是编进二进制的 fixture。** `fixtures/` 是仓库目录，装好的 Soul 没有仓库，所以导入语料与问卷答案用 `include_str!` 编进去。它们和 `just fixtures-verify` 检查的是同一批字节。代价是这两个文件改了，`soulcore` 要重编。

## WP13 完成情况（第二段：一个 store 句柄、能读回的配置、三条路由）

本机 `cargo test --workspace --all-targets` 绿（83 个测试目标 472 项），`just desktop-test` 绿（38 项），`just ui-test` 绿（7 个文件 53 项），`just ui-lint`、`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets --all-features -D warnings`、`e0-audit`、`denylist-audit`、`schema-freeze --check` 都过。没有加依赖。

这一段之前，这个壳每次启动都是第一次启动：配置是内存里现造的 `Config`，向导的答案活在一个 React prop 里，`/files` 与 `/graph` 是空路由，因为没有行可画。补的就是中间缺的那一块。

| 交付 | 证据 |
|---|---|
| 一个进程一个 store 句柄（WP07 遗留 8、WP09 遗留 8） | `soulcore::commands::session::Session` 是产品里唯一调 `store::open_store` 的地方，`Session::store()` 发出去的是同一个 `Arc<Mutex<SqlCipherStore>>` 的克隆。`apps/desktop/src-tauri/tests/one_store.rs` 三项：回读壳自己的四个源文件，`open_store` / `open_test_store` / `SqlCipherStore::open` / `Connection::open` 一个都不许出现；每个命令的签名里必须有 `State<'_, SessionState>`；两次 `Session::store()` 做指针相等。回读源码那一半是重点——运行时只能证明「我要到的两个句柄是同一个」，要排掉的却是**没人看的地方冒出第二次开库** |
| 配置落盘，而且落不下能力（AC-02） | 库旁边的 `config.json` 只有两个字段：向导有没有走完、授权过哪些目录。`StoredConfig` 是 `deny_unknown_fields`，所以一份写着采集或端点的文件**根本读不进来**——session 把问题报出来并按全关运行。「重启打不开任何能力」因此是文件形状的性质，不是读文件那段代码的谨慎。写走 `.partial` 再 rename，写到一半断电读到的是上一次的答案。`session_commands.rs` 14 项覆盖首次启动不落盘、向导跨重启、授权跨重启、自定义字段不生效、根目录消失要报出来 |
| 向导只问一次 | `session_status` 从盘上读 `wizard_completed`，`App` 不再拿 prop。`ipc_roundtrip::a_finished_wizard_is_still_finished_after_a_restart` 是真的重启：第一个应用退出，第二个在同一个目录上起来，中间只有那个文件在传话 |
| 密钥来源不含糊 | Windows 上是 `DpapiKeyProvider`，而 SECURITY.md 说它现在仍然是拒绝而不是编一个 key 出来——所以那台机器上库打不开，`SessionStatus.store_notice` 就直说这件事。其它平台是库旁边的种子文件，`KeyProtection::DeveloperKeyFile` 把这一点带到界面上，屏幕没法声称一个这个构建没有的保护 |
| `/files` 接 `PlanPreview`（WP11 遗留 8） | 授权目录、看计划、移动清单 / 原地不动清单 / 计数 / 两个哈希。**没有执行按钮**，而且不是靠藏：`core.ts` 列全了壳能调的命令，没有一个写文件；`PlanPreview.executable_in_this_version` 在 TypeScript 里的类型是字面量 `false`，想分支到执行路径的组件编不过。`Files.test.tsx` 7 项，在**有计划摆在屏幕上**的那一刻搜执行 / 应用 / 移动 / 重命名 / 删除 / 撤销 |
| `/graph` 接 `PersonSummaryView`（WP10 遗留 8） | 节点、边、每条边背后的证据条数，摘要按核心写的那几句原样渲染。屏上没有名字。`Graph.test.tsx` 7 项，其中一项把 `fixtures/denylist/diagnostic_terms.txt` 跑在渲染出来的 DOM 上；库没打开时给的是带理由码的拒绝，不是一张空的图——两者不能长得一样 |
| 起草端点的确认屏（WP10 遗留 6） | `prepare_draft` → 一屏计数 → `generate_draft`，批准要把 `preparation_id` 与 `plan_hash` 两样原样 echo 回来。屏上没有第三人的正文。`Draft.test.tsx` 加了 5 项（共 13），`ipc_roundtrip` 加了 5 项：计划不含粘贴的任何片段、对不上的批准什么都不生成且不留下能批第二次的东西、丢掉之后原来那份批准也失效 |
| 两边的命令名还是同一份 | `core.ts` 的 `COMMANDS` 从 5 个长到 14 个，`command_surface.rs` 四项照旧比对两侧并要求每个命令体只有一条语句。`contract.test.ts` 现在还把只读、确认、非临床三句话对着 Rust 常量核一遍——测试用的那个 double 不能变成一个比真核心更好说话的核心 |

落地内容：`crates/soulcore/src/commands/session.rs` 与两个测试文件（`session_commands` / `session_directory`）；`crates/soulcore/src/commands/graph.rs` 加 `PeopleGraphView`；`apps/desktop/src-tauri/src/{lib,commands}.rs` 与 `tests/{one_store,ipc_roundtrip}.rs`；`apps/desktop/src/` 的 `core.ts`、`App.tsx`、`router.tsx`、`routes/{Files,Graph}.tsx`、`routes/{Draft,Home}.tsx` 与相应测试。没有动 schema，没有动产品定义，没有新 fixture。

### WP13 第二段的取舍与遗留

1. **Windows 上库仍然打不开。** `DpapiKeyProvider` 是 SECURITY.md 里写着的骨架，`KeyError::Unsupported`。session 不去替它兜底：那台机器上 `store_opened` 是 false，`/graph` 给拒绝，`/files` 照常工作（文件计划不碰库）。**要它变绿得先实现 DPAPI**，那是安全面的工作单，不是这里现编一个 key 派生。
2. **session 的命令不写审计。** 授权目录、看计划、看摘要这三件事里，只有文件计划本来就有自己的审计条目（`FilePlanSession` 写）。「用户授权了一个目录」在冻结的 `audit.schema.json` 里没有对应动作，和 WP10 遗留 4 是同一条理由：现编一个动作等于让审计条目声称一件契约没说过的事。
3. **`config.json` 是明文。** 它只有一个布尔和一串路径，没有一个字节是内容。把它放进库里意味着「读配置」要先「开库」，而库开不开正是配置要报告的事情之一——那是个环。路径本身算不算隐私是可以讨论的，讨论的结果如果是「算」，那要改的是把它挪进库并接受首次启动读不到它。
4. **`SOUL_DATA_DIR` 是一个真的环境变量，不是只在测试里生效。** 它在平台规则**之前**读，所以一个测试没法半躲开平台规则。代价是任何人都能用它把 Soul 指到别处；这和「数据目录在哪里得看得见」是同一件事的两面，且它不会打开任何能力。
5. **`Home` 上的「已授权目录」计数在这次会话里可能过期。** 那个数来自启动时读的一次 `config_snapshot`，授权一个新目录之后 `/files` 会更新，概览不会。`/files` 才是那份名单的现场视图。
6. **`ipc_roundtrip` 的每个用例都新起一个应用。** Tauri 的 mock runtime 便宜，但这意味着「重启」和「同一个 session 上的两步」得分开表达——`Shell` 这个小结构体就是那条分界，`Shell::restart` 是前者，同一个 `Shell` 上调两次是后者。
7. **`soul-headless` 没有接 `Session`。** 它照旧用 `open_test_store` 走临时库，因为它证明的是 AC-21 的主流程，不是安装后的那个目录。两条路都只经过 `store::open_store`，但它们不是同一个句柄，也不该是。
8. **`one_store` 进了 windows-latest 那份点名清单，`ipc_roundtrip` 没有。** 后者链 WebView2 的 mock runtime，在 runner 上一条断言都跑不到（见 WP13 第一段遗留 7），只 `--no-run` 编译。`one_store` 不碰 mock runtime，而且它的运行时那一半正好是 Windows 的情况——DPAPI 拒绝，没有句柄可比，session 必须直说而不是蒙混过去。

## 下一步

批 3 的档案与记忆（WP03+WP04）、人脉图与导入（WP05+WP06）都已完成，批 4 的 WP07 前台采集与 WP09 桌面壳第一段也已完成。批 5 的 WP10 起草与人事摘要、WP11 文件计划都已完成并接到界面上。WP13 两段都完成：安装 smoke / CI / SBOM 是第一段，一个 store 句柄、能读回的配置、`/files` 与 `/graph` 与端点确认屏是第二段。

剩下的三件事，一件是界面，两件是人在真机前面：

1. **向导还没有画那十一道题。** `profile::questions()` 给出题面、选项和形状，`profile::intake` 收答案，`fixtures/questionnaire/v0_1.json` 钉住题号。store 句柄这个挡路的东西已经没有了——向导现在有 session 可用，缺的只是那一屏。同一批里还有 `/profile`、`/memory`、`/research`、`/audit` 四条 WP09 功能视图的空路由，核心与命令面都在，接法和 `/files`、`/graph` 一样。
2. **`scripts/author-manual-checklist.md` 要在一台 Windows 11 真机上过一遍**，七条结果填回「WP13 的 Windows 手动缺口」。托盘图标、UAC、任务管理器里的进程名、WebView2 的网络行为、真机采集这几条没有任何 CI 能替，也不要在文档里假装它们过了。
3. **DPAPI 要真的实现**，否则 Windows 上库打不开、`/graph` 只会给拒绝。这是 Goal 1 在目标平台上能不能读自己数据的前提。

不要启动 Goal 2。文件写入仍是 v0.1.1（AC-27）：`/files` 有计划、有哈希、没有执行按钮，也没有可以绑执行按钮的命令。
