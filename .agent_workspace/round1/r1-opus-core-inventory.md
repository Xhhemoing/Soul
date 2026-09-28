MODEL: claude-opus-5-thinking-high-fast

# R1 核心实现盘点（opus-fast / 只读实证）

盘点对象：`origin/cursor/soul-goal1-7b1c`（共 293 个受版本控制文件）。
方法：全部结论来自 `git ls-tree -r --name-only origin/cursor/soul-goal1-7b1c` 与 `git show origin/cursor/soul-goal1-7b1c:<path>` 读到的源码本身，未 checkout，未编译，未运行测试。凡是「STATUS.md 这么说」的内容，若我没在树里看到对应源码，一律记在「差异」一节而不写进「已实现功能」。

---

## 一、workspace 成员表

根 `Cargo.toml`（`origin/cursor/soul-goal1-7b1c:Cargo.toml`）的 `[workspace] members` 实际有 **13 个 crate**，全部在 `crates/` 下：

| # | crate | 路径 | 一句话职责（源自各自 `src/lib.rs` 头注释） | 关键模块文件 | `tests/` | 集成测试函数数（`#[test]` 计数） |
|---|---|---|---|---|---|---|
| 1 | `soul-schema` | `crates/soul-schema` | 数据契约唯一事实来源；不做 IO、不开 socket，`docs/schemas/*.json` 编译期内嵌 | `audit.rs` `common.rs` `contact.rs` `event.rs` `evidence.rs` `export_manifest.rs` `inference.rs` `memory.rs` `profile.rs` `relationship.rs` `soul_import_v1.rs` `validate.rs` | 有 | 21 |
| 2 | `soul-store-api` | `crates/soul-store-api` | 存储边界的 trait 合同 + 内存 `FakeStore` + 一套 conformance 套件 | `conformance.rs` `fake.rs` `forget.rs` `research.rs` `types.rs` | 有 | 6 |
| 3 | `soul-store` | `crates/soul-store` | 唯一把业务数据落盘的 crate：SQLCipher 主库 + 二次密封 + 遗忘事务 + 审计链 | `keys.rs` `sql.rs` `store.rs` `forget.rs` `audit.rs` `research_preview.rs` | 有 | 20 |
| 4 | `soul-policy` | `crates/soul-policy` | 权限面：E0/E1 网关、第三人占位、一次性豁免、HITL 令牌、注入隔离、审计无正文 | `net_guard.rs` `redactor.rs` `hitl.rs` `injection.rs` `audit.rs` `consent.rs` `clinical.rs` `e1.rs` `clock.rs` | 有 | 70 |
| 5 | `soul-egress` | `crates/soul-egress` | 全仓唯一持有 HTTP client 的 crate；`send` 必须收到 `soul-policy` 签发的 `EgressPermit` | `lib.rs`（单文件） | 有 | 8 |
| 6 | `soul-graph` | `crates/soul-graph` | 人脉图：证据 → 每人一条边 + 每边一条推断，节点上没有姓名字段 | `interaction.rs` `build.rs` `model.rs` `view.rs` `error.rs` | 有 | 7 |
| 7 | `soul-import` | `crates/soul-import` | v0.1 仅两种导入器（`soul-import-v1` JSONL、Telegram `result.json`）加一个问卷兜底 | `soul_import_v1.rs` `telegram.rs` `questionnaire.rs` `commit.rs` `redact.rs` `model.rs` `instant.rs` `defect.rs` | 有 | 34 |
| 8 | `soul-testkit` | `crates/soul-testkit` | 验收矩阵要用的仪器（崩溃 harness、泄漏检查器、mock LLM），刻意只当 dev 依赖 | `crash.rs` `fixtures.rs` `leakage.rs` `mock_llm.rs` | 有 | 23 |
| 9 | `xtask` | `crates/xtask` | 仓库护栏：URL 出网扫描、denylist 扫描、schema 冻结校验，每条都是可测的纯函数 | `egress.rs` `denylist.rs` `schema_freeze.rs` `main.rs` | 有 | 23 |
| 10 | `soulcore` | `crates/soulcore` | 编排核心：配置默认值 + `commands/` 下一包一文件的命令面 | `config.rs` `commands/*.rs` `bin/soul-headless.rs` | 有 | 29 |
| 11 | `soul-profile` | `crates/soul-profile` | 灵魂档案：五条方向轴、语气、问卷、写入侧承诺、可解引用的读模型 | `axes.rs` `voice.rs` `questionnaire.rs` `numeric.rs` `service.rs` `view.rs` `error.rs` | 有 | 13 |
| 12 | `soul-memory` | `crates/soul-memory` | 自传记忆：一记忆一把内容密钥，标题与摘要密封，CRUD + 遗忘 | `draft.rs` `service.rs` `error.rs` | 有 | 8 |
| 13 | `soul-collect` | `crates/soul-collect` | 只采前台应用使用时长，不采窗口标题；Windows 实现 + Linux 用假源 | `source.rs` `windows.rs` `fake.rs` `session.rs` `collector.rs` `runner.rs` `consent.rs` `lock.rs` `error.rs` | 有 | 16 |

**13/13 都有 `tests/` 目录，没有一个空壳 crate。** 集成测试函数合计 278 个（未含各 `src/*.rs` 内的 `#[cfg(test)]` 单元测试，我没有统计那一层）。

### 不在 workspace 里的 Rust 代码

`apps/desktop/src-tauri` 是**独立 cargo workspace**，不是根 members 之一（根 `Cargo.toml` 的 members 列表里没有它，它自带 `Cargo.lock` 与 `.cargo/config.toml`）。后果是 `cargo test --workspace` 碰不到它，要单独跑 `just desktop-test`。它有 4 个测试文件、21 个 `#[test]`：`command_surface.rs`(4) / `ipc_roundtrip.rs`(7) / `no_egress_path.rs`(3) / `shell_is_local_only.rs`(7)。

### `soulcore` 的依赖面（`crates/soulcore/Cargo.toml`）

`soulcore` normal 依赖了 10 个业务 crate（schema / store-api / store / policy / egress / graph / import / profile / memory / collect），所以它是唯一一个能同时看到全部业务能力的 crate。它另有一个 `[[bin]] soul-headless`（`src/bin/soul-headless.rs`），行为是打印默认配置 JSON 并退出 0，且在打印前检查 `config.open_capabilities()` 为空，非空则 exit failure。

---

## 二、soulcore 命令面表

`crates/soulcore/src/commands/mod.rs` 实际 `pub mod` 声明了 **8 个模块**：

```
pub mod collect;  pub mod graph;   pub mod import;  pub mod memory;
pub mod policy;   pub mod profile; pub mod shell;   pub mod store;
```

> 注意：同一文件的头注释表格里还列了 `draft.rs`（WP10）与 `fileplan.rs`（WP11），**这两个文件在树里不存在**，也没有对应的 `pub mod`。头注释是规划意图，不是现状。

| 模块 | 文件行数 | 公开函数 / 类型（逐条读源码所得） | 性质 |
|---|---|---|---|
| `store.rs` | 66 | `database_path(dir) -> PathBuf`<br>`open_store(dir, keys: &dyn KeyProvider) -> StoreResult<SqlCipherStore>`<br>`open_test_store(dir, seed) -> StoreResult<SqlCipherStore>`<br>`preview_forget(&store, ForgetUnit) -> StoreResult<ForgetImpact>`<br>`execute_forget(&mut store, ForgetUnit) -> StoreResult<ForgetReceipt>`<br>`research_preview(&store, &ResearchPreviewRequest) -> StoreResult<ResearchPreviewReport>` | 纯薄封装，直接转调 `SqlCipherStore` |
| `profile.rs` | 109 | `questions() -> Vec<soul_profile::Question>`<br>`intake(&mut store, profile_id, &QuestionnaireResponse, at_unix_seconds) -> Result<IntakeOutcome, ProfileError>`<br>`read(&store, profile_id) -> SoulProfile`<br>`view(&store, profile_id) -> ProfileView`<br>`render(&store, profile_id) -> String`<br>`voice(&store, profile_id) -> VoiceProfile`<br>`correct_axis(&mut store, profile_id, axis_id, AxisPosition, at) -> SoulProfile`<br>`set_voice(&mut store, profile_id, VoiceSetting, at) -> VoiceProfile`<br>`suggest_voice(&mut store, profile_id, VoiceSetting, at) -> bool`<br>`record_inference(&mut store, profile_id, AxisProposal, at) -> InferenceOutcome` | 薄封装；时钟由调用方以 `at_unix_seconds` 传入 |
| `memory.rs` | 66 | `create(&mut store, &MemoryDraft, at) -> SoulMemory`<br>`read(&store, memory_id) -> MemoryContent`<br>`list(&store) -> Vec<MemoryDigest>`<br>`update(&mut store, memory_id, &MemoryEdit, at) -> SoulMemory`<br>`preview_forget(&store, memory_id) -> ForgetImpact`<br>`forget(&mut store, memory_id, at) -> ForgetReceipt` | 薄封装 |
| `graph.rs` | 57 | `rebuild(&mut store, at) -> GraphBuild`（**非纯转调**：`soul_graph::rebuild` 之后逐条 append 审计）<br>`load(&store) -> SoulGraph`<br>`edge_evidence(&store, &TieEdge) -> Vec<SoulEvidence>`<br>`evidence_for(&store, relationship_id) -> Vec<SoulEvidence>` | 除 `rebuild` 补审计外为薄封装 |
| `import.rs` | 85 | `read_soul_import_v1(text: &str) -> Result<StagedImport, ImportFailure>`（不写盘）<br>`read_telegram(&Value) -> Result<StagedImport, ImportFailure>`（不写盘，调用方自己解析 JSON）<br>`commit(&mut store, &StagedImport, at) -> ImportReceipt`（**非纯转调**：append 全部审计，含 `injection.blocked`）<br>`questionnaire_needed(Option<&StagedImport>) -> bool`<br>`questions() -> &'static [questionnaire::Question]`<br>`record_questionnaire(&mut store, &[Answer], &Timestamp, &mut dyn UserStatedSink, at) -> QuestionnaireReceipt` | 读与写是两个独立调用，读不落盘 |
| `collect.rs` | 72 | `share(SqlCipherStore) -> Arc<Mutex<SqlCipherStore>>`<br>`platform_source() -> Result<Box<dyn ForegroundSource + Send>, SourceError>`（非 Windows 直接失败）<br>`start<F: ForegroundSource + Send + 'static>(source, Arc<Mutex<store>>, &ConsentHandle, CollectorConfig) -> CollectResult<CollectorHandle>`<br>`stop(CollectorHandle) -> CollectResult<CollectorReport>`<br>`forget_unit(content_key_id) -> ForgetUnit` | 刻意**没有** `is_collecting()`；运行态问 `CollectorHandle::is_running`，允许态问同意台账 |
| `shell.rs` | 170 | `struct ConfigSnapshot` + `ConfigSnapshot::of(&Config)`<br>`struct CloudNotice`<br>`cloud_toggle(&Config, requested_on: bool) -> CloudNotice`<br>`config_snapshot() -> ConfigSnapshot`<br>`struct WizardAnswers`<br>`complete_wizard(&WizardAnswers) -> Result<ConfigSnapshot, WizardRefused>`<br>`enum WizardRefused` | **唯一被桌面 UI 调到的模块**；不碰网也不碰盘 |
| `policy.rs` | 257 | `struct PolicySession`：`closed()` / `new(EgressConfig, KnownIdentifiers)` / `with_user_endpoint(..)` / `guard()` / `check_action(..)` / `issue_token(..)` / `redact(&[Turn]) -> RedactedBody` / `redact_with_exemption(..)` / `e1_generate(&mut self, model, RedactedBody, token_id, now_ms) -> Result<E1Outcome, E1Refusal>`<br>自由函数 `e1_plan(model, &RedactedBody) -> serde_json::Value`<br>`struct TokenRefused` `enum E1Refusal`（`reason_code()` / `audit()`）`struct E1Outcome`（`audit()`） | 唯一的 E1 出网路径；`e1_generate` 内部强制顺序：先脱敏 → 再算 plan → `check_action` → 端点存在性 → `guard.authorize_e1` 出 permit → `soul_egress::send` |

命令面的 `soulcore/tests/` 有 7 个文件、29 个测试函数，覆盖情况按文件：`store_commands.rs`(1) `profile_memory_commands.rs`(2) `import_and_graph_commands.rs`(4) `policy_commands.rs`(6) `shell_commands.rs`(10) `config_defaults.rs`(5) `endpoint_is_user_supplied.rs`(1)。**没有 `collect_commands.rs`** —— `collect.rs` 是唯一一个在 soulcore 层没有自己测试文件的业务命令模块（采集的验收测试全在 `crates/soul-collect/tests/`）。

---

## 三、桌面路由表

来源：`apps/desktop/src/router.tsx` 的 `ROUTES` 常量、`apps/desktop/src/App.tsx` 的渲染分支、`apps/desktop/src/components/Pending.tsx`。

`App.tsx` 的判定规则很直白：`route.id === "home"` 渲染 `<Home>`，`route.id === "settings"` 渲染 `<Settings>`，而 `route.ownedBy !== null` 就渲染 `<Pending>`。所以「真 UI」= `ownedBy === null` 的那两条。

| # | `id` | 路径 | 标题 | `ownedBy` | 实际渲染 | 判定 |
|---|---|---|---|---|---|---|
| 1 | `home` | `/` | 概览 | `null` | `routes/Home.tsx` | **真 UI**：渲染 `ConfigSnapshot` 的四项事实（采集开关 / 云端标签 / 端点是否填写 / 已授权目录数）+ 一张「这一版还没有的东西」清单（由 `ROUTES.filter(ownedBy !== null)` 自动生成） |
| 2 | `profile` | `/profile` | 灵魂档案 | `"WP09 功能视图"` | `Pending` | 占位。文案：特质轴、语气与纠正锁定「已经在核心里落地，这个视图还没有接上去」 |
| 3 | `graph` | `/graph` | 人脉图 | `"WP09 功能视图"` | `Pending` | 占位 |
| 4 | `memory` | `/memory` | 自传记忆 | `"WP09 功能视图"` | `Pending` | 占位 |
| 5 | `draft` | `/draft` | 起草 | `"WP10"` | `Pending` | 占位。`App.test.tsx` 有断言钉住此页**没有输入框、没有发送按钮** |
| 6 | `files` | `/files` | 文件计划 | `"WP11"` | `Pending` | 占位。`App.test.tsx` 断言此页**没有任何执行按钮** |
| 7 | `research` | `/research` | 研究预览 | `"WP09 功能视图"` | `Pending` | 占位 |
| 8 | `audit` | `/audit` | 审计 | `"WP09 功能视图"` | `Pending` | 占位 |
| 9 | `settings` | `/settings` | 设置 | `null` | `routes/Settings.tsx` | **真 UI**：`<CloudToggle>` + 一段四条的「这个壳会不会自己上网」事实列表 |

**9 条路由，2 条是真 UI，7 条是 `Pending` 占位。** 路由器是手写 hash router（`routeForHash` 认不出的地址回落到 `ROUTES[0]` 即概览，`App.test.tsx` 有对应断言「认不出来的地址回到概览，而不是白屏」）。

另有两个不在路由表里的界面：
- `routes/Wizard.tsx` —— 首启向导，是 `App.tsx` 里 `finished === false` 时的**全屏门**，不是一条路由。它渲染的四行默认值全部来自 `snapshot`，不是硬编码；唯一的勾选框是「我读过」，点完调 `completeWizard`。
- `components/NavRail.tsx` —— 左侧导航，把 9 条路由全列出来，`ownedBy !== null` 的加 `muted` 类。

前端测试：4 个 vitest 文件、20 个用例（`App.test.tsx` 7 / `contract.test.ts` 4 / `CloudToggle.test.tsx` 4 / `Wizard.test.tsx` 5）。

---

## 四、STATUS 宣称 vs 树实证 差异

先说结论：**`docs/STATUS.md` 在可核对的量化断言上没有说谎**，我抽查的三个数字都对得上。差异集中在「读者容易误读的地方」，而不是「写错的地方」。

| # | STATUS / 文档的说法 | 树里的实证 | 判定 |
|---|---|---|---|
| 1 | `docs/GOAL1_PLAN.md:41` 把 `soul-draft`、`soul-fileplan` 列为 Goal 1 关键 crate；`:82` 还写了「`soul-fileplan` 无写 API」这条禁令 | 全树搜 `soul-draft` / `soul_draft` / `soul-fileplan` / `soul_fileplan`，**命中仅这两行文档**。根 `Cargo.toml` 无此 member，`crates/` 下无此目录 | **两个 crate 都不存在**。STATUS 进度表把 WP10/WP11 归入「v0.1 其余 WP：未开始」，所以 STATUS 本身自洽；但只读 GOAL1_PLAN 的「关键 crate」一行会误以为它们在 |
| 2 | `soulcore/src/commands/mod.rs` 头注释的约定表列了 `draft.rs`（WP10）与 `fileplan.rs`（WP11） | 该目录只有 8 个 `.rs` + `mod.rs`，无 `draft.rs`、无 `fileplan.rs`，`pub mod` 列表也只有 8 项 | 源码内注释是意图声明。**不要把它当成命令面清单** |
| 3 | 树里存在 `crates/soul-memory/src/draft.rs` | 读文件头：它定义的是 `MemoryDraft`（`memory_type` / `title` / `summary` / `subject` / `source_event_ids` / `evidence_ids` / `placeholder`）与两个密封字段名常量，是「一条待存记忆的入参结构」 | **与 WP10「起草」毫无关系**，是同名陷阱。按文件名 grep `draft` 会得出错误结论 |
| 4 | 进度表：WP03 档案 / WP04 记忆 / WP05 人脉图 / WP06 导入 / WP07 采集「完成」 | Rust 侧确实完成：五个 crate 各有源码与测试，`soulcore/src/commands/` 也各有一层薄封装 | **属实，但只到 Rust 层**。这五项能力**一个都没有接到 UI 上**（见下条与第五节） |
| 5 | 进度表：WP09 桌面壳「第一段（壳）完成」；WP09 取舍 8 承认「壳还没有连真的 store」，取舍 9 承认五条路由是空的 | `apps/desktop/src-tauri/src/commands.rs` 只注册 3 个命令；`SessionConfig(pub Config)` 用 `Config::default()`，全文没有 `SqlCipherStore`、没有 `open_store` | **属实**。但这条只写在 WP09 小节的「取舍与遗留」里，进度表顶部看不到。父调度器若只读进度表会高估可用性 |
| 6 | WP09 落地内容称 `apps/desktop/` 有「43 个文件」 | `git ls-tree -r -- apps/desktop \| wc -l` = **43** | 对得上 |
| 7 | WP09 称「vitest 4 个文件 20 项」「cargo 21 项」 | vitest：7+4+4+5 = **20**；`src-tauri/tests` 的 `#[test]`：4+7+3+7 = **21** | 两个都对得上 |
| 8 | WP07 遗留 10 称「没有加 `soulcore/tests/collect_commands.rs`」 | `crates/soulcore/tests/` 7 个文件里确实没有 | 对得上 |
| 9 | WP09 取舍 1 称 `apps/desktop/src-tauri` 是独立 workspace | 根 `Cargo.toml` members 里没有它，它自带 `Cargo.lock` + `.cargo/config.toml` | 对得上。**父调度器注意**：改 `src-tauri` 的 Rust 代码不会被 `cargo test --workspace` 覆盖 |
| 10 | WP07 取舍 8 称整个进程只能有一个 store 句柄，`commands::collect::share` 是那个包装 | `collect.rs` 里 `share(store) -> Arc<Mutex<SqlCipherStore>>` 属实；但**目前没有任何调用方**——桌面壳压根没开 store | 属实且尚未被使用。这是「接 UI」时的第一个约束 |
| 11 | `soulcore/src/lib.rs` 头注释仍写「WP01 ships configuration defaults and nothing else」「依赖列表刻意很短：数据契约和存储边界。No HTTP stack」 | `Cargo.toml` 现在 normal 依赖 10 个业务 crate，**含 `soul-egress`**（HTTP client 就在图里，靠 `deny.toml` + `xtask e0-audit` 约束而非靠依赖缺席） | **lib.rs 头注释已过时**，与同 crate 的 `Cargo.toml` 注释自相矛盾（后者明确解释了 `soul-egress` 为何在）。属文档陈旧，不影响行为 |
| 12 | `soulcore/src/config.rs` 的 `Config` 有 `authorized_roots: Vec<PathBuf>` 字段，UI 也显示「已授权目录 N 个」 | 无任何 crate 消费它（`soul-fileplan` 不存在） | 字段先行、能力未落地。UI 上那个计数目前恒为 0 且没有添加入口 |

---

## 五、UI 是否已调用 soulcore 的业务命令

**没有。UI 只接了三个 shell 命令，业务命令面一个都没上。** 三处独立证据：

1. **前端侧**：`apps/desktop/src/core.ts` 是全前端唯一允许 `import { invoke } from "@tauri-apps/api/core"` 的文件（`eslint.config.js` 的 `no-restricted-imports` 加 `src/contract.test.ts` 双重把关）。它导出的 `COMMANDS` 常量只有三项：
   ```
   configSnapshot: "config_snapshot"
   completeWizard: "complete_wizard"
   cloudToggle:    "cloud_toggle"
   ```
   对应三个函数 `configSnapshot()` / `completeWizard(answers)` / `cloudToggle(requestedOn)`。没有 profile、没有 memory、没有 graph、没有 import、没有 collect、没有 store、没有 policy。

2. **Rust 宿主侧**：`apps/desktop/src-tauri/src/commands.rs` 全文只有三个 `#[tauri::command]`，全部转调 `soulcore::commands::shell::*`，并有 `pub const COMMAND_NAMES: &[&str] = &["config_snapshot", "complete_wizard", "cloud_toggle"]`。`lib.rs` 的 `generate_handler!` 也正好注册这三个。`command_surface.rs` 有一条测试断言每个 wrapper 函数体最多一条语句（放不下分支与循环）。

3. **状态侧**：`SessionConfig(pub Config)` 用 `#[derive(Default)]`，即 `Config::default()`。`src-tauri` 全部源码里没有 `SqlCipherStore`、没有 `open_store`、没有任何 `Arc<Mutex<..>>` 的 store 句柄。**壳根本没有打开过数据库**，所以就算想调业务命令也没有 store 可传（除 `import::read_*`、`profile::questions`、`import::questions`、`collect::forget_unit`、`policy::e1_plan` 这几个不需要 store 的以外，所有业务命令的第一个参数都是 `&SqlCipherStore` 或 `&mut SqlCipherStore`）。

所以准确的说法是：**UI 目前接的就是「shell 快照 + 首启向导 + 云开关」这三样，外加一个纯前端的 hash 路由和 7 个 Pending 占位页。**

---

## 六、父调度器可用的「已实现功能」清单

只写我在这次盘点里亲眼读到源码的东西。**每一条都注明「能从哪里调到」**，因为绝大多数只能从 Rust 调，不能从 UI 调。

### A. Rust 侧可用（有 `soulcore::commands::*` 入口 + 有测试）

| 能力 | 入口 | 前置 | 有 soulcore 层测试 |
|---|---|---|---|
| 打开 / 建立加密主库 | `commands::store::open_store` / `open_test_store` | 需 `KeyProvider`；测试用 `TestKeyProvider::from_seed` | 是（`store_commands.rs`） |
| 遗忘影响面预览 + 执行遗忘 | `commands::store::preview_forget` / `execute_forget` | 打开的 store | 是 |
| 研究预览（只读、不落盘） | `commands::store::research_preview` | 打开的 store | 是 |
| 问卷取题 + 问卷建档 | `commands::profile::questions` / `intake` | store | 是（`profile_memory_commands.rs`） |
| 读档案 / 解引用视图 / 渲染成文 | `commands::profile::read` / `view` / `render` | store | 是 |
| 读语气、设语气、机器建议语气 | `commands::profile::voice` / `set_voice` / `suggest_voice` | store | 是（含「用户设过之后 suggest 返回 false」） |
| 用户纠正轴并锁定 | `commands::profile::correct_axis` | store | 是 |
| 记录轴推断（无证据会被拒） | `commands::profile::record_inference` | store | 是 |
| 记忆 CRUD | `commands::memory::create` / `read` / `list` / `update` | store | 是 |
| 记忆遗忘预览 + 遗忘 | `commands::memory::preview_forget` / `forget` | store | 是 |
| 读 `soul-import-v1` JSONL（不落盘） | `commands::import::read_soul_import_v1` | 无 | 是（`import_and_graph_commands.rs`，含「坏文件写不进任何东西」） |
| 读 Telegram `result.json`（不落盘，调用方给已解析 JSON） | `commands::import::read_telegram` | 无 | 是 |
| 提交导入（写库 + 补审计，含 `injection.blocked`） | `commands::import::commit` | store | 是 |
| 无文件时走问卷兜底 | `commands::import::questionnaire_needed` / `questions` / `record_questionnaire` | store + `UserStatedSink` | 是 |
| 重建人脉图（写库 + 补审计） | `commands::graph::rebuild` | store | 是 |
| 读人脉图 / 解一条边的证据 | `commands::graph::load` / `edge_evidence` / `evidence_for` | store | 是 |
| 前台采集：共享句柄 / 平台源 / 启停 / 遗忘单元 | `commands::collect::share` / `platform_source` / `start` / `stop` / `forget_unit` | store 的 `Arc<Mutex<..>>` + `ConsentHandle`；`platform_source` 非 Windows 必失败 | **否**（soulcore 层无测试文件；验收测试在 `crates/soul-collect/tests/`，16 个） |
| 权限会话：动作检查 / 一次性令牌 / 脱敏 / 一次性豁免 | `commands::policy::PolicySession::{check_action, issue_token, redact, redact_with_exemption}` | `EgressConfig` + `KnownIdentifiers` | 是（`policy_commands.rs` 6 项） |
| E1 出网生成（唯一的出网路径） | `commands::policy::PolicySession::e1_generate` + `e1_plan` | 用户自填端点；令牌一次性；plan 改了会被拒 | 是 |
| 无头入口（CI 无桌面时） | `soulcore` 的 `[[bin]] soul-headless` | 无 | 是（`config_defaults.rs::the_headless_binary_prints_closed_defaults_and_exits_zero`） |
| 默认全关的配置 + `open_capabilities()` | `soulcore::Config` | 无 | 是（5 项） |

### B. UI 侧可用（用户真能看见并操作）

| 能力 | 位置 |
|---|---|
| 首启向导：显示四项默认值（值来自核心快照，非硬编码），勾「我读过」才能完成；核心会拒绝返回任何有开关打开的配置 | `routes/Wizard.tsx` → `complete_wizard` |
| 概览页：显示采集 / 云 / 端点是否填写 / 已授权目录数 + 自动生成的「还没有的东西」清单 | `routes/Home.tsx` |
| 设置页：云开关（点几次都返回「尚未启用」，不出网）+ 四条自查事实 | `routes/Settings.tsx` + `components/CloudToggle.tsx` → `cloud_toggle` |
| 左侧导航 9 条 + hash 路由 + 未知地址回落概览 | `components/NavRail.tsx` + `router.tsx` |
| 7 个写明归属的空路由（不是白屏，也不是假实现） | `components/Pending.tsx` |
| 托盘：两项菜单「打开 Soul」「退出 Soul」；有托盘时关窗收起，无托盘时正常关闭 | `src-tauri/src/tray.rs` + `lib.rs` 的 `on_window_event` |

### C. 明确**不可用**的（父调度器不要往上排任务时假设它在）

- `soul-draft` / `soul-fileplan` 两个 crate：不存在。
- `commands/draft.rs` / `commands/fileplan.rs`：不存在（只在 `mod.rs` 注释里出现）。
- 桌面壳连真库：不存在（`SessionConfig` 只握内存 `Config`）。
- 档案 / 人脉图 / 记忆 / 研究预览 / 审计 五个功能视图：只有 `Pending` 占位。
- 采集能力的 UI 入口与同意开关界面：没有（`Wizard.tsx` 明确说「只有勾『我读过』，没有第二个能打开什么的控件」）。
- Windows 密钥保护（DPAPI）：STATUS WP02 取舍 3 自陈是骨架，两个入口返回 `KeyError::Unsupported`（我未逐行核对 `soul-store/src/keys.rs`，此条转述 STATUS，标记为待核）。

### D. 可跑的检查命令（`justfile` recipes，实证存在）

`default` `setup` `fmt` `lint` `test` `schema` `schema-write` `e0` `denylist` `fixtures-verify` `deny` `ci`（= lint schema e0 denylist fixtures-verify test ui-lint ui-test） `ci-full`（= ci deny） `ui-install` `ui-lint` `ui-test` `ui-build` `desktop-check` `desktop-test` `desktop-dev`。

> 提醒：`just ci` 的 `test` 走的是根 workspace，**不含** `apps/desktop/src-tauri`；后者要 `just desktop-test`。

---

## 附：本次盘点的局限

1. 未编译、未跑测试。「有测试」= 树里有测试文件且有 `#[test]`，不等于当前 HEAD 上它们全绿。
2. 只统计了 `tests/` 下的集成测试函数，各 `src/*.rs` 内的 `#[cfg(test)] mod tests` 未计入，因此真实测试数量高于表中数字。
3. `soul-store/src/keys.rs`、`soul-policy` 各模块、`soul-collect/src/collector.rs` 的内部实现只读了头注释与公开面，没有逐行核实实现是否与注释一致。若父调度器要对某条 AC 下判断，建议单独派一个子代理读该文件全文。
