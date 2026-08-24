MODEL: claude-fable-5-thinking-xhigh

# Goal 1 剩余 P0 拆分 — TASK_SPLIT

**分支**:`agent/dev-sota`(基于 `862e858`,WP01–WP09 壳已在)。
**下游**:每个 ST 派一个 `claude-opus-5-thinking-high-fast`,一人一包,包内文件所有权互斥。
**本文件只拆任务,不含实现代码。** 所有 API 名均经源码核实(见第 1 节),不是转抄简报。

---

## 0. 范围与原则

P0 = 用户指定的五件事:WP10 起草+人事摘要、WP11 只读文件计划、壳接真 `SqlCipherStore`、soulcore 薄封装、授权目录配置入口。

拆分原则:

1. **文件所有权互斥**:任何两个可并行的 ST 的 glob 交集为空。共享文件(根 `Cargo.toml`、`soulcore/Cargo.toml`、`commands/mod.rs`、`soul-policy/src/audit.rs`)全部收进串行先行的 ST-00,后续并行 ST 一律不碰。
2. **`docs/STATUS.md` 不归任何并行 ST**。仓库惯例是"每个子代理完工更新 STATUS",但它是全员冲突点;本轮改为:各 ST 把完成报告写到 `.agent_workspace/dev-sota/reports/ST-0X.md`(各自独占),由收口 ST-04 统一合入 STATUS。
3. **根 `Cargo.lock` 与 `apps/desktop/src-tauri/Cargo.lock` 是机器生成文件**,不计入互斥(cargo 会重算);合并冲突由 ST-04 重新解析处理,任何 ST 不得手改 lock 内容。
4. 职责切分沿用仓库既有分层:新 crate 是纯逻辑(无 IO 决策、无网),`soulcore::commands::*` 只编排(持库句柄、过 policy、落审计),`apps/desktop` 只渲染与转调。
5. 若某 ST 发现必须改动自己 glob 之外的文件(例如需要新的第三方依赖进根 `Cargo.toml`),**停下上报父代理**,不得自行改共享文件。

---

## 1. 源码勘探结论(已核实,含勘误与新发现)

给定关键面逐条核实结果:

| 关键面 | 核实结果 |
|---|---|
| `PolicySession::{redact, redact_with_exemption, e1_generate, issue_token, check_action}` | ✅ `crates/soulcore/src/commands/policy.rs`。注意正确调用顺序:`redact(turns)` → `e1_plan(model, &body)` → `PlanHash::of` → `issue_token(CapabilityScope::E1Generate, hash, now)` → `e1_generate(model, body, token_id, now)`(内部自己重算 plan hash、consume 令牌、经 NetGuard 授权、调 `soul_egress::send`)。`E1Outcome::audit()` / `E1Refusal::audit()` 返回现成的 `AuditContent`,调用方负责落链 |
| `soul_profile::service::read_voice` / `set_voice` | ✅ 泛型 over `ProfileStore`/`AuditLog`;`soulcore::commands::profile::{voice, set_voice}` 已有 `SqlCipherStore` 版薄封装,WP10 直接用后者 |
| `soul_testkit::mock_llm::MockLlm` | ✅ dev-only。`start/base_url/chat_completions_url/requests/request_count/set_redirect`。`requests()` 返回原始字节 body,配 `soul_testkit::leakage::LeakageChecker`(`with_min_ngram`/`add_third_party_body`/`add_known_identifier`/`assert_clean`) |
| `soul_policy::assert_non_clinical` | ✅ re-export 于 crate 根;**勘误:`WORKING_HYPOTHESIS_NOTICE` 未 re-export 到根**,路径是 `soul_policy::clinical::WORKING_HYPOTHESIS_NOTICE` |
| `ActionKind::{DraftReply, AnalysePeople, ScanDirectory, PlanFiles, GenerateWithUserEndpoint}` | ✅ 全在。前四个 `needs_capability_token() == false`(过 `check_action` 即可);只有 `GenerateWithUserEndpoint`/`ExecuteForget` 要令牌。`RequestOrigin::ExternalContent` 一律拒 |
| `Config.authorized_roots` 无写入命令 | ✅ 字段在、`is_fully_closed`/`open_capabilities` 已计入;整个仓库无任何写入口。桌面侧 `SessionConfig(pub Config)` 是**不可变** managed state,要加写入口必须 Mutex 化 |
| `apps/desktop/src-tauri` 独立 workspace | ✅ 有自己的 `Cargo.lock` 与 `.cargo/config.toml`;根 `cargo test --workspace` 碰不到它,要跑 `just desktop-check` / `desktop-test` 等价命令 |
| 根 `Cargo.toml` members / workspace.dependencies | ✅ 新 crate 两处都要加;第三方版本全部钉在根,成员用 `workspace = true` 引用。`walkdir 2.5.0` **已钉**(xtask 在用),WP11 可直接引用,不用动根 |
| `soul-memory/src/draft.rs` | ✅ 是 `MemoryDraft`(记忆入参),与 WP10 无关。同名陷阱:WP10 的类型命名避开 "MemoryDraft",且不 import 该模块 |

拆分必须知道的**新发现**:

1. **`ReasonCode` 是闭集,没有"路径未授权"码。** `audit.schema.json` 只限形状(`^[A-Z][A-Z0-9_]{2,63}$`),所以加变体是纯 Rust 改动(`crates/soul-policy/src/audit.rs` 的枚举 + `as_str` + `ALL`),**不动 schema、不动 schemas.lock**。收进 ST-00,避免 WP11 的 ST 碰 soul-policy。
2. **`AuditAction` 已有 `DraftCreate`("draft.create")与 `FilePlan`("file.plan")**,frozen schema 已备好这两个动作,WP10/WP11 无需任何 schema 变更。人事摘要不落库、无专属审计动作:本地统计路径不写审计;走 E1 时由 `E1Outcome::audit()` 的 `EgressRequest` 条目覆盖。**禁止为它新造 AuditAction(那是 schema 变更,需批准)。**
3. **审计落链用 `soul_policy::audit::append_or_store_error(store, content, now)`**(profile/memory 已用);链字段(`seq`/`prev_hash`/`entry_hash`)由存储覆盖,调用方只构造内容。
4. **单句柄工具已存在**:`soulcore::commands::collect::share(store) -> Arc<Mutex<SqlCipherStore>>`。壳接线复用它,不要再造包装。
5. **`TestKeyProvider::in_dir(dir)`** 把随机 seed 存 `dir/soul-test-keys.bin`(明文文件),重启后同一把钥匙——壳接真库应该用它而不是 `from_seed`(固定 seed 等于把密钥硬编码进二进制)。`DpapiKeyProvider` 两个入口现在都返回 `Unsupported`,**壳必须回退并如实标注**。
6. **`soulcore::commands::store` 没有 re-export `TestKeyProvider`/`KeyProvider` 类型**,只有 `open_store(dir, &dyn KeyProvider)` 与 `open_test_store(dir, seed)`。壳要用 `in_dir` 模式,要么在 `commands/store.rs` 加一个入口,要么 desktop 直接依赖 `soul-store`——建议前者,保持"壳只依赖 soulcore"。`commands/store.rs` 因此划给 ST-03。
7. **源码级断言是本仓库的既有模式**(research_preview 无写 API、collect 不采标题、command_surface thin-wrapper 都是读回源码断言),WP11 的"无写 API"与壳的"单一 open 调用点"照抄这个模式,并带"扫描器能命中合成反例"的对照。
8. **注入与泄漏语料已有**:`fixtures/injection/`、`fixtures/leakage/`。优先复用;新增 fixture 若含 URL/敏感词要先看 `crates/xtask` 的豁免规则(fixtures 有既有豁免先例),不要自行改 xtask。
9. **desktop 前端三方一致性测试会咬人**:TS `COMMANDS`(`src/core.ts`)、Rust `COMMAND_NAMES`(`src-tauri/src/commands.rs`)、`generate_handler!` 注册三者集合相等,由 `contract.test.ts` 与 `command_surface.rs` 双向断言;加 IPC 命令三处都要动,且 wrapper 函数体 ≤1 条语句(`the_command_layer_stays_thin` 钉住)。
10. **起草/文件页现在被测试钉住"没有输入框/没有执行按钮"**(`App.test.tsx`)。本轮不做这两页 UI,别碰那些断言。

---

## 2. 子任务总览与所有权矩阵

```
波次1:  ST-00 骨架注册(串行,小)     ST-03 壳接真库+授权目录(与 ST-00 并行)
              │
波次2:  ST-01 WP10 起草+摘要   ∥   ST-02 WP11 只读文件计划
              │                          │
波次3:  ST-04 收口回归 + STATUS(串行,等全部合入)
```

| ST | 独占 glob(唯一允许改动) |
|---|---|
| ST-00 | 根 `Cargo.toml`;`crates/soul-draft/**`(空壳);`crates/soul-fileplan/**`(空壳);`crates/soulcore/Cargo.toml`;`crates/soulcore/src/commands/mod.rs`;`crates/soulcore/src/commands/draft.rs`(存根);`crates/soulcore/src/commands/fileplan.rs`(存根);`crates/soul-policy/src/audit.rs`(仅加 ReasonCode 变体);`.agent_workspace/dev-sota/reports/ST-00.md` |
| ST-01 | `crates/soul-draft/**`;`crates/soulcore/src/commands/draft.rs`;`crates/soulcore/tests/draft_commands.rs`;`fixtures/draft/**`(如需);`.agent_workspace/dev-sota/reports/ST-01.md` |
| ST-02 | `crates/soul-fileplan/**`;`crates/soulcore/src/commands/fileplan.rs`;`crates/soulcore/tests/fileplan_commands.rs`;`fixtures/fileplan/**`(如需);`.agent_workspace/dev-sota/reports/ST-02.md` |
| ST-03 | `apps/desktop/**`;`crates/soulcore/src/commands/shell.rs`;`crates/soulcore/src/commands/store.rs`;`crates/soulcore/src/config.rs`;`crates/soulcore/src/lib.rs`;`crates/soulcore/tests/shell_commands.rs`;`docs/SECURITY.md`(仅密钥小节);`.agent_workspace/dev-sota/reports/ST-03.md` |
| ST-04 | `docs/STATUS.md`;`docs/GOAL1_PLAN.md`;`.agent_workspace/dev-sota/**`;合并破损的最小修复(见工作单) |

互斥性说明:ST-00 与 ST-03 并行成立,因为 ST-00 碰 `soulcore/Cargo.toml`+`commands/mod.rs`+两个存根,ST-03 碰 `shell.rs`/`store.rs`/`config.rs`/`lib.rs`,零交集。ST-01/ST-02 各自的存根文件在 ST-00 落地后所有权移交(串行,无冲突)。两个新 crate 的 dev 依赖(`soul-testkit`/`tempfile`)与 normal 依赖由 ST-00 一次写全(清单见工作单),ST-01/02 不再碰任何 `Cargo.toml`。

---

## 3. 工作单

### ST-00 骨架注册(串行先行)

**目标**:把两个新 crate 与共享文件的全部改动一次做完,让 ST-01/02 的 glob 可以完全不含共享文件。

**允许改动**:见矩阵。

**完成定义**(可测断言):

1. `crates/soul-draft` 与 `crates/soul-fileplan` 存在:各含 `Cargo.toml`(`workspace = true` 继承版本/许可)与 `src/lib.rs`(只有 `#![forbid(unsafe_code)]`、`#![deny(missing_debug_implementations)]` 与一段文档注释写明归属 WP10/WP11,**零业务代码**,仿 `Pending.tsx` 的"空得诚实"模式)。
2. 依赖一次写全,ST-01/02 不再碰 Cargo.toml:
   - `soul-draft`:normal = `soul-schema`、`soul-policy`、`soul-profile`、`soul-graph`、`soul-store-api`、`serde`、`serde_json`、`thiserror`、`uuid`;dev = `soul-testkit`、`soul-store`、`tempfile`。**不含 `soul-egress`、`reqwest`、`tokio`、`axum`。**
   - `soul-fileplan`:normal = `soul-schema`、`soul-policy`、`soul-store-api`、`walkdir`、`serde`、`serde_json`、`thiserror`、`uuid`;dev = `soul-testkit`、`soul-store`、`tempfile`。**不含 `soul-egress`。**
3. 根 `Cargo.toml`:members 加两条,`[workspace.dependencies]` 加 `soul-draft`/`soul-fileplan`(path + version,与现有条目同款)。
4. `crates/soulcore/Cargo.toml` 加 `soul-draft`/`soul-fileplan` 两行 normal 依赖;`commands/mod.rs` 加 `pub mod draft;`、`pub mod fileplan;`(头部注释里 WP10/WP11 两行本来就在);`commands/draft.rs`、`commands/fileplan.rs` 为存根(只有文档注释:归 ST-01/ST-02 填)。
5. `crates/soul-policy/src/audit.rs`:`ReasonCode` 加一个变体 `PathNotAuthorized`,`as_str` 返回 `"PATH_NOT_AUTHORIZED"`,加入 `ReasonCode::ALL`。不加其它变体,不动任何现有变体。
6. 全绿:`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace --all-targets`、`cargo run -p xtask -- e0-audit`、`denylist-audit`、`schema-freeze --check`、`cargo deny check`。

**必须调用的现有 API**:无(本包只注册,不写逻辑)。

**红线**:清单之外零文件;不写任何函数体;不动 schema 与 schemas.lock;`Cargo.lock` 只由 cargo 再生;存根不得假装功能(不放 `todo!()`,放空模块+文档注释)。

**建议测试文件名**:无新测试(现有 `soul-policy` 测试覆盖 `ReasonCode::ALL` 一致性;若 clippy 对空 crate 报 warning,修 crate 自身而不是加 allow)。

**依赖**:无(第一波)。

---

### ST-01 WP10:`crates/soul-draft` 起草不发送 + 人事分析摘要

**目标**:粘贴消息按档案语气起草(不发送);单人人事分析摘要(有 key 走 E1,无 key 统计/模板降级);默认 E1 请求体第三人占位;一次性豁免;注入串只是数据。对齐 PRODUCT_LOCK 切片 6 与 8。

**允许改动**:`crates/soul-draft/**`、`crates/soulcore/src/commands/draft.rs`、`crates/soulcore/tests/draft_commands.rs`、`fixtures/draft/**`(优先复用 `fixtures/injection`、`fixtures/leakage`)、`reports/ST-01.md`。

**职责切分**(必须遵守,这是文件互斥的前提):

- `soul-draft` = 纯逻辑:粘贴文本 → `Turn` 序列(带 `SealedSubject` 标注,粘贴内容一律按第三人/mixed 保守分类);`VoiceProfile` → 确定性语气模板与 E1 系统提示;人事摘要的统计侧(吃 `soul-graph` 的 `TieEdge`/`TieStrength` 计数与解引用后的证据行,产出每条 claim 带 `evidence_ids` 的结构化摘要);所有可读输出过 `assert_non_clinical` 并携带 `WORKING_HYPOTHESIS_NOTICE`。无 IO,无网,无库句柄。
- `soulcore::commands::draft` = 编排:持 `SqlCipherStore` 读 voice 与图、持 `PolicySession` 走 redact→plan→token→e1_generate、用 `append_or_store_error` 落 `DraftCreate` 审计。E1 调用**只能**发生在这一层。

**完成定义**(可测断言,对齐 AC):

1. **AC-07 起草侧**:`soulcore::commands::profile::set_voice` 把 directness 设为 `Direct` 后,同一粘贴文本再起草,模板输出与 E1 请求体里的语气指令都反映用户值;随后 `suggest_voice` 提议相反值返回 `false`,再起草输出不变。断言读的是 `read_voice` 链路的返回值,不是测试自己存的变量。
2. **AC-11 起草路径**:`soul-draft` 公开面不存在任何名带 send/submit/deliver 的 API(源码级断言,仿 research_preview 模式,含合成反例对照);起草全程 `MockLlm` 只收到打到配置 origin 的请求;`set_redirect` 到另一 origin 后 `e1_generate` 返回拒绝(reason `E1_CROSS_ORIGIN_REDIRECT`)且 `request_count` 不再增加。
3. **AC-12**:含第三人正文/姓名/账号的 fixture 默认起草,`MockLlm.requests()` 的每个原始 body 过 `LeakageChecker`(第三人正文 ≥8 scalar 子串为零命中;`KnownIdentifiers` 里的姓名/账号零裸露)。
4. **AC-13**:`ExemptionRequest::for_turn(id).confirm(true)` 得到 `OneShotExemption`,`redact_with_exemption` 起草一次:该次请求体含原文、`E1Outcome.carries_exempted_original == true`、审计 reason 是 `THIRD_PARTY_BODY_INCLUDED`;同一 session 紧接着默认起草,请求体回到占位、审计 reason 回到 `THIRD_PARTY_BODY_PLACEHELD`。
5. **AC-16**:人事摘要每条 claim 的 `evidence_ids` 非空且逐个 `get_evidence` 解引用成功(取不回即失败,不得吞);渲染文本过 `assert_non_clinical`(以及对 `fixtures/denylist` 全表的直接扫描)并含 `WORKING_HYPOTHESIS_NOTICE`;强度只用 `SupportedBand`/交互计数,无数字评分。
6. **AC-17**:`PolicySession::closed()`(或 `llm_endpoint: None`)时,起草返回确定性语气模板结果、摘要返回统计降级结果,两者非空且可读;全程零网络调用(不起 MockLlm,并断言 closed session 的任何 `e1_generate` 都在 `NotConfigured` 处拒绝)。
7. **AC-25 粘贴路径**:`fixtures/injection` 语料逐行粘贴起草:产出只是草稿文本(`soul-draft` 无工具执行 API,源码级断言);`soul_policy::injection::urls_in` 提出的每个 URL 过 `NetGuard::closed()` 全拒;以 `RequestOrigin::ExternalContent` 请求 `ActionKind::DraftReply`/`AnalysePeople` 被拒 `ExternalContentNotAuthority`;检出注入标记时留 `InjectionBlocked` 审计(无正文)。
8. **审计**:成功起草留 `DraftCreate / Allowed` 条目(只有 id 与计数);整条链序列化后以粘贴正文+姓名为语料过 `LeakageChecker`;E1 成功/拒绝的条目由 `E1Outcome::audit()`/`E1Refusal::audit()` 产出后落链。
9. 粘贴内容**不落库**(本包不调 `EventStore::append_event`/`BlobStore::seal`;想存就走 WP04 的记忆入口,不在本包)。

**必须调用的现有 API**(不得重写):`PolicySession` 全套与 `e1_plan`(顺序见第 1 节);`soulcore::commands::profile::{voice, set_voice, suggest_voice}`;`soulcore::commands::graph::{load, edge_evidence}`(或 `soul_graph::view::{load, resolve_evidence}`);`soul_policy::redactor::{Turn, KnownIdentifiers, ExemptionRequest, RedactedBody}`;`soul_policy::injection::{UntrustedText, scan, looks_like_injection, urls_in}`;`soul_policy::{assert_non_clinical}` 与 `soul_policy::clinical::WORKING_HYPOTHESIS_NOTICE`;`soul_policy::audit::{AuditContent, append_or_store_error}`;`soul_policy::hitl::{ActionRequest, RequestOrigin, ActionKind}`;dev:`soul_testkit::{mock_llm::MockLlm, leakage::LeakageChecker}`、`soulcore::commands::store::open_test_store`。

**红线**:`soul-draft` 不依赖 `soul-egress`/`reqwest`(ST-00 已把依赖钉死,不得加回);占位符逻辑不得自造(只经 `Redactor` 产出的 `RedactedBody`,它是唯一能进 `e1_generate` 的类型);不得出现诊断词与数字评分(denylist-audit 会扫);命名避开 `MemoryDraft`,不 import `soul_memory::draft`;不动 `soul-memory`/`soul-policy`/`soul-egress` 任何文件;不做 UI;不发送——公开面连"可发送"的状态位都不许有。

**建议测试文件**:`crates/soul-draft/tests/tone_templates.rs`(模板与降级,纯逻辑)、`crates/soul-draft/tests/people_summary.rs`(AC-16 证据与非临床)、`crates/soul-draft/tests/no_send_api.rs`(源码级)、`crates/soulcore/tests/draft_commands.rs`(AC-07/11/12/13/17/25 的 MockLlm + 真库端到端)。

**依赖**:ST-00。与 ST-02、ST-03 真正并行。

---

### ST-02 WP11:`crates/soul-fileplan` 只读扫描 + 计划预览

**目标**:授权根内只读扫描、生成整理计划预览;未授权路径 100% 拒绝;源码级不存在写 API。对齐 PRODUCT_LOCK 切片 9 与 D31(执行是 v0.1.1)。

**允许改动**:`crates/soul-fileplan/**`、`crates/soulcore/src/commands/fileplan.rs`、`crates/soulcore/tests/fileplan_commands.rs`、`fixtures/fileplan/**`(如需)、`reports/ST-02.md`。

**职责切分**:

- `soul-fileplan` = 授权判定(`roots: &[PathBuf]` 作参数传入,canonicalize 后前缀判定)、`walkdir` 只读扫描、计划构造(建议的移动/重命名/归类,纯描述值)、`FilePlanPreview` 类型带 `written_to_disk` 字段且**只有恒为 false 的构造入口**(仿 `zero_third_party_rows` 的构造级强制)。不持 Config,不持库句柄,不出网。
- `soulcore::commands::fileplan` = 编排:`check_action`(`ScanDirectory`/`PlanFiles`)、成功与拒绝都落审计(`FilePlan` 动作;拒绝用 ST-00 加的 `PATH_NOT_AUTHORIZED`)。

**完成定义**(可测断言,对齐 AC):

1. **AC-18 正向**:tempdir A 授权后扫描 → 计划预览非空(fixture 目录树足够产生至少一条建议);扫描+预览前后对 A 逐文件比对 `(路径, 字节数, 内容哈希, mtime)` 快照,完全不变。
2. **AC-18 拒绝**:以下每种对 B 的请求全部拒绝并返回 `PATH_NOT_AUTHORIZED` 语义:roots 为空;B 不在 roots;`A/../B` 相对穿越;A 内指向 B 的 symlink(canonicalize 后逃出 root 即拒);B 是 A 的父目录。拒绝路径同时留 `FilePlan / Denied / PATH_NOT_AUTHORIZED` 审计。**100% 是字面义**:对拒绝用例矩阵逐一断言,没有"抽样"。
3. **源码级无写 API**:测试读回 `crates/soul-fileplan/src/` 每个文件,断言不出现 `fs::write`、`File::create`、`OpenOptions`、`rename`、`remove_file`、`remove_dir`、`copy`、`hard_link`、`set_permissions`、`create_dir`(含 `create_dir_all`);带对照用例证明扫描器对合成文本真的会红(仿 WP07 三层模式)。
4. **计划哈希可钉**:计划的 JSON 描述经 `PlanHash::of` 产 hash;改动计划任一字段后携旧 hash 过 `check_action` 被拒 `PlanHashMismatch`(AC-19 底座在 fileplan 上的实例化)。
5. **写令牌换不来执行**:`PolicySession::issue_token(CapabilityScope::FileWrite, …)` 对本计划 hash 返回 `WriteNotImplemented` 拒绝;`soul-fileplan` 类型面上不存在 execute/apply 入口(编译期事实,测试注释写明)。
6. **文件名注入**(AC-25 文件名路径的 fileplan 侧):fixture 里文件名含注入串与 URL;扫描结果照常列出(数据不是指令);`urls_in(UntrustedText::new(文件名))` 的 URL 过 `NetGuard::closed()` 全拒;`RequestOrigin::ExternalContent` 的 `ScanDirectory`/`PlanFiles` 被拒。
7. **审计无路径**:成功条目只有计数(`items` = 文件数)与 uuid;把扫描到的每个文件名当语料,审计链序列化过 `LeakageChecker`(文件名可能含人名,不得进链)。计划预览本身可以含路径(它是给用户看的本地值),审计不行。

**必须调用的现有 API**:`soul_policy::hitl::{check_action 经 PolicySession, ActionKind::{ScanDirectory, PlanFiles}, ActionRequest, RequestOrigin, PlanHash}`;`soul_policy::audit::{AuditContent, ReasonCode::PathNotAuthorized, append_or_store_error}`;`soul_policy::injection::{UntrustedText, urls_in}`;`soul_policy::net_guard::NetGuard`(仅测试);`walkdir`(workspace 已钉);dev:`soulcore::commands::store::open_test_store`、`soul_testkit::leakage::LeakageChecker`。

**红线**:全 crate 无写 API(上面清单是最低集合,发现新写法一并禁);`#![forbid(unsafe_code)]`;不新增 `CapabilityScope` 变体、不实现 FileWrite 消费;类型面无 execute/apply/undo(那是 v0.1.1 的 AC-27);roots 只作参数,不读全局状态;不依赖 `soul-egress`;不动 `soul-policy`(`PATH_NOT_AUTHORIZED` 已由 ST-00 备好);审计不含文件名/路径;不做 UI(文件页的"没有执行按钮"断言不许动)。

**建议测试文件**:`crates/soul-fileplan/tests/authorized_scan.rs`(AC-18 正向+磁盘不变)、`crates/soul-fileplan/tests/unauthorized_is_refused.rs`(拒绝矩阵)、`crates/soul-fileplan/tests/no_write_api.rs`(源码级+对照)、`crates/soul-fileplan/tests/filename_injection.rs`、`crates/soulcore/tests/fileplan_commands.rs`(HITL+审计端到端)。

**依赖**:ST-00。与 ST-01、ST-03 真正并行。

---

### ST-03 壳接真 `SqlCipherStore`(单句柄)+ 授权目录配置入口

**目标**:桌面壳启动时打开一次真加密库并作为唯一句柄托管;无 DPAPI 时用 `TestKeyProvider` 并在 UI 与 SECURITY 如实写;给 `Config.authorized_roots` 一个最小写入口(核心校验 + IPC + 设置页入口),否则 AC-18 的"授权 A"没有用户入口。**不做**档案/记忆/人脉/起草/文件等功能视图。

**允许改动**:`apps/desktop/**`、`crates/soulcore/src/commands/shell.rs`、`crates/soulcore/src/commands/store.rs`、`crates/soulcore/src/config.rs`、`crates/soulcore/src/lib.rs`、`crates/soulcore/tests/shell_commands.rs`、`docs/SECURITY.md`(仅密钥落地小节)、`reports/ST-03.md`。

**完成定义**(可测断言):

1. **单句柄真库**:`lib.rs` 的 setup 解析数据目录(用 Tauri 自带 path resolver 的 app-local-data 目录;Windows 语义即 `%LOCALAPPDATA%`,不加插件)、选 KeyProvider、`open_store` 一次、经 `soulcore::commands::collect::share` 包成 `Arc<Mutex<SqlCipherStore>>` 后 `manage`。源码级断言:`apps/desktop/src-tauri/src/` 全部文件里 `open_store`/`open_test_store` 的调用点恰好一个(仿 thin-wrapper 断言模式)。
2. **密钥选择与诚实标注**:优先探测 `DpapiKeyProvider`(当前必然 `Unsupported`)→ 回退 `TestKeyProvider::in_dir(数据目录)`;回退事实通过 `KeyProvider::describe()` 进入一个新的核心侧快照字段(如 key_protection 描述),UI 设置页逐字渲染核心给的句子(仿 `CLOUD_NOT_YET_AVAILABLE_EXPLANATION` 的"文案住在 Rust"模式),内容如实:密钥文件未受 DPAPI 保护。断言:全仓库(本 ST 改动面内)不出现"已受 DPAPI 保护"的宣称;`SECURITY.md` 密钥小节同步一段如实说明。
3. **授权目录写入口**:核心侧新增 `shell::authorize_root`(校验:存在、是目录、canonicalize、去重;错误可读)与列出入口;`SessionConfig` Mutex 化;IPC 新增两条命令(授权/读取),`COMMANDS`(core.ts)/`COMMAND_NAMES`/`generate_handler!` 三处同步,`contract.test.ts` 与 `command_surface.rs` 的三方一致断言继续绿;wrapper 仍 ≤1 条语句。
4. **AC-02 不回归**:向导完成后 `fully_closed == true`、`authorized_root_count == 0`;`complete_wizard` 拒绝任何有开关打开的配置的断言原样保留。授权动作只能来自设置页,不进向导。
5. **快照如实**:授权一个 tempdir 后 `config_snapshot` 的 `authorized_root_count == 1`、`open_capabilities` 含 `authorized_roots`;设置页显示已授权路径列表(用户自己输入的路径,允许回显)与"本次会话有效,尚无持久化"的如实文案(配置持久化归 WP13,不在本包做)。
6. **IPC roundtrip**:mock runtime + 真 `generate_context!` 下,授权命令走通(tempdir);非法路径(不存在/是文件)返回可读错误;未注册命令仍拒;伪造 origin 仍拒。
7. **前端约束保持**:`forbidNetwork` 五桩全程无调用;设置页新控件只有文本输入+按钮(不加 dialog/fs 插件);denylist 扫描(contract.test.ts)继续绿。
8. 桌面与根两棵树都绿:`cargo test --workspace --all-targets`(根)、`just desktop-check`/`desktop-test` 等价命令(desktop 树)、`pnpm ui-lint`/`ui-test`。

**必须调用的现有 API**:`soulcore::commands::store::{open_store, database_path}`(如需 in_dir 入口,加在 `commands/store.rs`,签名对齐现有风格);`soul_store::{KeyProvider, TestKeyProvider, DpapiKeyProvider}`(经 soulcore re-export,不让 desktop 直接依赖 soul-store);`soulcore::commands::collect::share`;`soulcore::commands::shell::{ConfigSnapshot, complete_wizard, cloud_toggle}`(扩展不重写);tauri `Manager::manage`/`State`。

**红线**:不加任何 tauri 插件,capabilities 只保 `core:default`;CSP 不放宽(`ipc:`/`ipc.localhost` 白名单之外不加源);LLM 端点字符串仍不进快照(`the_snapshot_carries_no_endpoint_string` 不许动);不写 DPAPI 的 unsafe 实现(P1);不宣称 KEK 受保护;不在命令处理器里二次开库;不跑 `tauri build`;不做起草/文件/档案等功能视图,`App.test.tsx` 钉住的"起草页无输入框、文件页无执行按钮"断言不许动;业务判断(路径校验)全在 Rust 核心,WebView 只传字符串;不动 `soul-store`/`soul-policy` 源码。

**建议测试文件**:扩展 `crates/soulcore/tests/shell_commands.rs`(authorize_root 校验矩阵、快照计数、AC-02 后置条件);`apps/desktop/src-tauri/tests/store_session.rs`(单 open 调用点源码断言+密钥回退描述);扩展 `apps/desktop/src-tauri/tests/ipc_roundtrip.rs` 与 `command_surface.rs`;前端 `apps/desktop/src/routes/Settings.test.tsx`(授权入口、诚实文案、forbidNetwork)。

**依赖**:无(第一波,与 ST-00 并行)。ST-01/02 不等它;后续功能视图(P1)等它。

---

### ST-04 收口回归 + STATUS(串行殿后)

**目标**:三条并行分支合入后,让整棵树(根 workspace + desktop workspace + 前端)回到全绿,并把进度如实写回文档。

**允许改动**:`docs/STATUS.md`、`docs/GOAL1_PLAN.md`(补 WP10/WP11/壳接线的完成定义与勾选)、`.agent_workspace/dev-sota/**`;两份 `Cargo.lock` 重解析;**其它文件仅限修复合并引入的编译/测试破损**,每处修复单独说明归因,超过 10 行或涉及权限/数据面判断的问题回报父代理重派原 ST 责任人,不得自行改语义。

**完成定义**:

1. `just ci` 等价全绿:fmt --check、clippy -D warnings、`cargo test --workspace --all-targets`、xtask 三项(e0/denylist/schema-freeze --check)、fixtures-verify、`cargo deny check`。
2. desktop 树全绿:`just desktop-check`/`desktop-test` 等价、`pnpm ui-lint`/`ui-test`。
3. `docs/STATUS.md` 增"WP10 完成情况""WP11 完成情况""壳接真库与授权目录"三节(交付/证据/取舍表,风格对齐现有章节),内容从 `reports/ST-0*.md` 合成,不夸大:凡 Windows 真机才能验的照 WP09 先例列入手动清单。
4. `GOAL1_PLAN.md` 补 WP10/WP11 完成定义(R1 已点名的缺口)。
5. `.agent_workspace/dev-sota/PROGRESS.md` 循环表更新。

**红线**:不加功能、不顺手重构、不动 schema、不跑 tauri build、不启动 Goal 2/P1 项。

**依赖**:ST-01 + ST-02 + ST-03 全部合入。

---

## 4. 本轮不做(写死,实现者不得"顺手"越界)

| 项 | 去向 |
|---|---|
| Goal 2 全部 | Goal 1 关门后 |
| 文件写执行与撤销(AC-27)、`CapabilityScope::FileWrite` 的任何消费路径 | v0.1.1 |
| E0 / 云端 HTTP / 任何非 E1 出网 | 无代码路径,恒禁 |
| OAuth、微信/QQ 抓取 | v0.2 / 不做 |
| `DpapiKeyProvider` 的 unsafe Win32 实现 | P1(单独工作单,与壳接线解耦;补齐前不宣布 AC-01) |
| `tauri build` / MSI / 安装 smoke / SBOM | WP13(P1) |
| 档案/记忆/人脉/导入/研究/审计功能视图(WP09 功能视图) | P1;本轮 UI 只加设置页授权入口 |
| 双问卷合并(soul-import 8 题 vs soul-profile 7 题) | P1,按 R1 仲裁走契约改动,不是十行适配器 |
| 配置持久化(重启后记住 roots/endpoint) | WP13 决定位置;本轮 UI 如实写"会话有效" |
| 导入事件去重 | P1 带债关门,UI 提示归功能视图 |

---

## 5. 派工表

每格一个 `claude-opus-5-thinking-high-fast`(降级链:`claude-opus-5-thinking-high` → `claude-sonnet-5-thinking-high`;子代理第一行自报 slug)。分支名建议 `cursor/st0X-<名>-ea3a` 系列,基于 `agent/dev-sota`。

| 波次 | ST | 包 | 必须等谁 | 可与谁同时跑 |
|---|---|---|---|---|
| 1 | ST-00 | 骨架注册(共享文件一次改完) | 无 | ST-03 |
| 1 | ST-03 | 壳接真库 + 授权目录入口 | 无 | ST-00,以及波次 2 的两个 |
| 2 | ST-01 | WP10 soul-draft + soulcore 封装 | ST-00 合入 | ST-02、ST-03 |
| 2 | ST-02 | WP11 soul-fileplan + soulcore 封装 | ST-00 合入 | ST-01、ST-03 |
| 3 | ST-04 | 收口回归 + STATUS/GOAL1_PLAN | ST-01+ST-02+ST-03 全部合入 | 无(独占) |

合并顺序建议:ST-00 → ST-03 → ST-01/ST-02(任意先后,文件互斥)→ ST-04。ST-03 与 ST-00 虽并行开工,合并时先 ST-00(根 Cargo.lock 变更大,后合的 ST-03 只带 desktop 自己的 lock)。

每个 ST 的通用要求(写进派工提示词):第一行自报 slug;只改自己 glob;完成后跑本工作单"完成定义"里点名的命令并把结果写进 `reports/ST-0X.md`;发现必须越界的改动就停下上报,不得自行动共享文件;commit 信息按仓库现有风格(一句话说清为什么)。
