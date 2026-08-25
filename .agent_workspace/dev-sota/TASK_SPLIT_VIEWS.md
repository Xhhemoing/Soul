MODEL: claude-fable-5-thinking-xhigh

# WP09 功能视图拆分(仅起草页 + 文件计划页)— TASK_SPLIT_VIEWS

**分支**:`agent/dev-sota`(基于 `9705fb4`,WP10/WP11 核心与 SOTA 复审已合入)。
**下游**:每个 ST 派一个 `claude-opus-5-thinking-high-fast`,一人一包,包内文件所有权互斥。
**本文件只拆任务,不含实现代码。** 所有 API 名均经源码核实(第 1 节),不是转抄简报。

---

## 0. 范围与原则

本轮只做两个视图:`#/draft`(粘贴 → 起草,永不发送)与 `#/files`(授权目录扫描 + 计划预览,永不执行)。核心已存在,本轮是**把已验证的核心接到屏幕上**,并为此给 soulcore 长出可序列化的视图 DTO 与宿主函数。

沿用 TASK_SPLIT 的拆分原则,外加本轮特有的三条:

1. **文件所有权互斥**:任何两个可并行的 ST 的 glob 交集为空。共享文件(`core.ts`、`commands.rs`、`lib.rs`、`fakeCore.ts`、`App.tsx`、`App.test.tsx`、`router.tsx`、`contract.test.ts`、soulcore 的 `shell.rs`/`store.rs`)全部收进串行先行的波次 1;波次 2 的两个页面 ST 一律不碰。波次 1 内部再分两个串行 ST(先核心侧、后壳侧),避免一个超大包。
2. **所有权移交**:`commands/draft.rs`、`commands/fileplan.rs` 与两个新路由组件由波次 1 建好骨架后,所有权移交给波次 2 对应的 ST(串行,无冲突)——同 TASK_SPLIT 里 ST-00 存根移交的先例。
3. **DTO 字段集在波次 1 冻结**。`core.ts` 的 TS 类型、`fakeCore` 的假值、Rust DTO 三处必须同形;波次 2 发现字段不够用,**停下上报父代理**,不得自行改共享文件。
4. `docs/STATUS.md` 不归任何并行 ST;各 ST 写 `.agent_workspace/dev-sota/reports/ST-V*.md`(各自独占),由 ST-V4 收口合入。两份 `Cargo.lock` 是机器生成文件,不计入互斥,冲突由 ST-V4 重解析。
5. 若发现必须改动自己 glob 之外的文件,停下上报父代理。

---

## 1. 源码勘探结论(已核实,含陷阱)

| 关键面 | 核实结果 |
|---|---|
| `soulcore::commands::draft` | ✅ 已有 `draft_reply(&mut SqlCipherStore, &mut PolicySession, DraftRequest) -> Result<DraftOutcome, DraftCommandError>`、`session_for(&Config, KnownIdentifiers) -> Result<PolicySession, OriginError>`(`llm_endpoint: None` ⇒ closed session ⇒ `DraftRoute::Template`)、`known_identifiers(&SqlCipherStore)`、`people_summary`。`DraftRequest::new(profile_id, pasted, model, now_ms, at_unix_seconds)`,exemption 默认 `None`。审计(`DraftCreate`、`InjectionBlocked`、E1 条目)全部在 `draft_reply` 内部落链,宿主函数**不需要也不得**再写审计 |
| `soulcore::commands::fileplan` | ✅ 已有 `scan_directory(session, store, roots, target, origin, now_ms, at)` 与 `plan_files(session, store, report, approved: Option<&PlanHash>, origin, now_ms, at)`,两者内部各落一条 `FilePlan` 审计(拒绝带 `ReasonCode::PathNotAuthorized`)。`approved: None` 即"首次出示计划",现有测试如此使用 |
| `DraftOutcome` / `FilePlanPreview` **非 Serialize** | ✅ `soul-draft/src/tone.rs` 只 derive `Debug, Clone, PartialEq, Eq`;`soul-fileplan/src/plan.rs` 同。`ScanReport`/`PlanEntry`/`PeopleSummary` 亦然。⇒ 必须在 soulcore 造 DTO;**纯 crate 一个字都不改**(no_send_api / no_write_api 源码级测试盯着它们) |
| `FilePlanPreview::written_to_disk()` | ✅ 恒 `false`,字段私有、`plan()` 是唯一构造。DTO 照抄这个"构造级强制"模式 |
| `PastedTurn::unattributed(UntrustedText)` | ✅ `soul-draft/src/turns.rs`:Mixed 主体,保守按第三人处理,redactor 会占位。`UntrustedText::new(String)` 是入口 |
| profile id | ✅ **不需要 intake**。`soul_profile::read_profile` 对不存在的 id 返回 `blank_profile`(五轴 unknown + 默认 voice),`read_voice` 因此对任意 id 都给出中性 `VoiceProfile`。`profile::intake` 需要真实 `QuestionnaireResponse`,空问卷是契约雷区——**不碰**。宿主用存在 `shell::Session` 里的稳定 UUIDv7(每会话一枚,SessionState 手写 Default 时 `Uuid::now_v7()` 铸造) |
| 壳的托管 store | ⚠️ 现状:`install_store`(`lib.rs`)在 `setup` 里 `app.manage(session.into_handle())`,而 mock 运行时只跑 `configure()`,**没有**这份 state——命令直接取 `State<Arc<Mutex<…>>>` 在 mock 测试里会炸而不是可读拒绝。且 `tests/store_session.rs::no_command_opens_a_store_of_its_own` 禁止 `commands.rs` 出现字符串 `open_store`/`SqlCipherStore`/`Mutex`/`Arc`;`the_store_is_opened_in_exactly_one_place` 要求全壳 `open_store` 出现次数 == lib.rs 恰好 1 次、无 `Arc::new`/`Mutex::new`。⇒ 需要 soulcore 侧的**空槽类型**(下文 `StoreSlot`):`configure()` 里 `manage` 一个默认空槽,`install_store` 填一次,mock 运行时槽为空 → 宿主函数返回可读拒绝 |
| 薄封装测试 | ⚠️ `command_surface.rs::the_command_layer_stays_thin` 数的是函数体第一个 `{` 到**第一个** `}` 之间的非注释**行数** ≤1。⇒ 新 wrapper 的函数体必须是**一行**(rustfmt 100 列内),体内不得出现 `}`(不许写结构体字面量) |
| 三方一致 | ✅ `contract.test.ts` 用正则读 `commands.rs` 的 `#[tauri::command] pub fn NAME` 与 `core.ts` 的 `COMMANDS` 值比对;`command_surface.rs` 再比 `COMMAND_NAMES` 与 `generate_handler!`(lib.rs 需含 `commands::NAME` 字样)。加命令三处 + `COMMAND_NAMES` 四处同步 |
| `fakeCore.ts` | ✅ `mockIPC` 的 switch 对未知命令 default 抛错 ⇒ 新命令不加假实现,`/draft`、`/files` 的所有 UI 测试直接红 |
| `App.test.tsx` 现状 | ✅ `/draft`:`pending-owner` 含 "WP10 未落地"、无 textbox、无 `/发送/` 按钮;`/files`:`pending-owner` 含 "WP11 未落地"、全部按钮不匹配 `/执行\|应用\|移动\|重命名\|删除/`。本轮:draft 加粘贴框、**保留无发送按钮**;files 的按钮循环**原样保留** |
| ⚠️ 按钮命名陷阱 | 断言是 `queryByRole("button", { name: /发送/ })`——**"起草(不发送)"这种label 也会命中 /发送/**。触发按钮叫「生成草稿」之类,整页按钮可达名不得含"发送"二字,也不得含 send/submit/deliver |
| `router.tsx` / `App.tsx` | ✅ `RouteDefinition.ownedBy: null` 即"已填上";`App.tsx` 对 `ownedBy !== null` 渲染 `Pending`。接视图 = 两条路由 `ownedBy` 置 null + `App.tsx` 加两个 `route.id === …` 分支 |
| 拒绝的形状 | ✅ `RootRefused { reason, message }`(Serialize + thiserror)是既有 IPC 错误先例;`core.ts::refusalText` 已会读 `.message`。新 `ViewRefused` 照抄 |
| 路径 oracle 属性 | ✅ `FilePlanError::PathNotAuthorized` **故意不回显被拒路径**(error.rs 模块注释:拒绝语只列已授权 roots)。视图层的拒绝语必须保持这一点——直接用错误的 `Display`,不自己拼 |
| `AuthorizedRoots::canonicalized(&[PathBuf])` | ✅ 逐根 canonicalize,坏根即错;空表合法(什么都不授权 ⇒ `scan` 可读拒绝)。`Session` 里的 roots 已是 canonicalize 过的(`shell::authorize_root`),二次解析必成 |
| `ScanReport` 口径 | ✅ `file_count()`、`dir_count()`、`skipped_escaping_links()`、`scan_id()`;`PlanEntry::{source_rel, action, target_rel}`、`PlanAction::as_str`(`group/move/rename`)、`FilePlanPreview::{len, count_of, entries}` |
| 单句柄工具 | ✅ `soulcore::commands::collect::share(SqlCipherStore) -> Arc<Mutex<SqlCipherStore>>`;`store::SessionStore::{into_handle, key_protection}`、`open_store_for_session` 不动 |
| 时钟 | ✅ 既有命令层全部由调用方传 `now_ms`/`at_unix_seconds`。视图宿主函数**在 soulcore 里自己读时钟**(SystemTime),否则 wrapper 超一行 |
| 前端边界 | ✅ eslint `no-restricted-imports` 只许 `core.ts` import `@tauri-apps/api`;新页面只 import `../core`。样式复用 `styles.css` 既有 class(`panel`/`facts`/`refusal`/`badge`/`muted`/`field-row`/`primary`),**不改 css 文件** |
| 绿灯命令 | ✅ 根树 `just ci` 等价(fmt/clippy/test + xtask e0/denylist/schema-freeze --check + fixtures-verify)+ `cargo deny check`;desktop 树 `just desktop-check`/`desktop-test`;前端 `pnpm ui-lint`/`ui-test` |

新发现汇总(拆分依赖的判断):

1. **people_summary 本轮不开 IPC。** 它需要 `contact_id`,而联系人列表属于人脉图视图(WP09 功能视图,明确 out)。没有选人入口的摘要视图不是"免费又互斥的小只读"。核心继续 headless,由 soulcore 测试覆盖。
2. **起草页在本轮永远走 Template 路线。** `Config.llm_endpoint` 在壳里没有任何写入口(全仓核实),`session_for` 必然给 closed session。`DraftRequest.model` 传常量即可(E1 分支不可达,常量到不了线上);端点录入 UI 属 WP13/后续,本轮不做。
3. **豁免(OneShotExemption)不过 IPC。** 逐条转发的同意流程需要按 turn 的 UI,超出本轮;`DraftRequest.exemption` 从视图路径恒为 `None`,红线钉死。
4. **宿主函数零新增审计动作。** 复用 `draft_reply`/`scan_directory`/`plan_files` 的内部落链,schema/schemas.lock 一个字节不动。
5. **无新依赖**:根 `Cargo.toml`、desktop `Cargo.toml`、`package.json` 全部不碰;`no_egress_path.rs` 与 `cargo deny` 自动看住。

---

## 2. 必做设计(冻结,实现者不得另起炉灶)

### 2.1 soulcore 侧新面(波次 1 落地)

**`commands/store.rs` — `StoreSlot`**(名字可微调,契约不可):
- `Default` 为空槽;`install(Arc<Mutex<SqlCipherStore>>)` 只成功一次(第二次返回错误或 false,测试可断言);`lock() -> Option<MutexGuard<'_, SqlCipherStore>>`,中毒锁按 `shell::Session::lock` 先例 `into_inner` 恢复。
- 壳在 `configure()` 里 `manage(StoreSlot::default())`,`install_store` 用 `session.into_handle()` 填槽(替换现在的 `app.manage(session.into_handle())`)。mock 运行时槽恒空。

**`commands/shell.rs` — Session 扩展 + 拒绝类型**:
- `Session::config(&self) -> Config`(单锁下 clone;endpoint 是 `Option<String>`,只在 Rust 内部流转,不进任何 DTO)。
- `Session::draft_profile_id(&self) -> Uuid`:`SessionState` 增加字段,手写 `Default` 铸 `Uuid::now_v7()`,同会话内稳定。
- `ViewRefused { reason: ViewRefusedReason, code: Option<String>, message: String }`,`ViewRefusedReason ∈ { NoStoreOpened, EmptyPaste, Refused }`(snake_case serde),`#[error("{message}")]`,Serialize。`code` 放 `ReasonCode::as_str()`(来自 `DraftCommandError::reason_code()` / `FilePlanRefusal::reason_code()`),没有就 None。
- 常量 `NO_STORE_FOR_VIEW_EXPLANATION`(如实:本次会话没打开本机数据库,起草与文件计划需要它;真机在启动时打开,测试的 mock 运行时不开)。文案住 Rust,壳逐字渲染——`CLOUD_NOT_YET_AVAILABLE_EXPLANATION` 同款模式。
- 一个时钟助手(如 `pub fn wall_clock() -> (u64, i64)`),两个宿主模块共用,使 wrapper 不碰时钟。

**`commands/draft.rs` — `DraftView` + 宿主函数**:

```
DraftView(字段私有,唯一构造 DraftView::of(&DraftOutcome),只 derive Serialize,不 Deserialize):
  text: String                       ← DraftOutcome::text
  route: String                      ← DraftRoute::as_str()("template" | "e1")
  route_label: String                ← Rust 常量句(模板:"本机模板写成,没有任何字节出网")
  turns / third_party_turns / placeheld_turns: usize   ← DraftStats
  carries_exempted_original: bool    ← 恒 false(视图路径无豁免)
  never_sent: bool                   ← 构造函数里写死 true,无 setter、无第二构造
  notice: String                     ← 常量 DRAFT_NEVER_SENT_EXPLANATION(如实:这是草稿,
                                        本产品没有发送它的代码路径,用不用、发不发由你)
```

`pub fn draft_view(slot: &StoreSlot, session: &Session, pasted: Vec<String>) -> Result<DraftView, ViewRefused>`,顺序固定:
1. `slot.lock()` 为空 → `NoStoreOpened` + `NO_STORE_FOR_VIEW_EXPLANATION`;
2. trim 后全空 → `EmptyPaste`(可读中文句);非空项逐条 `PastedTurn::unattributed(UntrustedText::new(…))`;
3. `session.config()` → `known_identifiers(&store)` → `session_for(&config, identifiers)`;
4. `shell::wall_clock()` 读时钟(**在 soulcore 内**);
5. `DraftRequest::new(session.draft_profile_id(), pasted, VIEW_MODEL, now_ms, at)`(`VIEW_MODEL` 为模块常量;E1 分支在本轮不可达,注释写明);exemption 恒 None;
6. `draft_reply(&mut store, &mut policy, request)` → `DraftView::of(&outcome)`;错误 → `Refused` + `reason_code` + Display 文本(soul-draft 的错误契约保证不含粘贴正文)。

**`commands/fileplan.rs` — `FilePlanView` + 宿主函数**:

```
FilePlanView(字段私有,唯一构造 FilePlanView::of(&ScanReport, &FilePlanPreview),只 Serialize):
  scan_id: String
  file_count / dir_count / skipped_escaping_links: usize      ← ScanReport
  entry_count / group_count / move_count / rename_count: usize ← FilePlanPreview
  entries: Vec<FilePlanEntryView { source_rel: String, action: String,
           action_label: String, target_rel: Option<String> }>
           (action_label 是 Rust 里的中文词:group→归类 / move→移动 / rename→重命名,
            WebView 只渲染,不翻译)
  written_to_disk: bool   ← 构造函数写死 false,无 setter(照抄 FilePlanPreview 模式)
  notice: String          ← 常量 PLAN_PREVIEW_ONLY_EXPLANATION(如实:只是建议,
                            没有执行它的代码路径;文件写入是 v0.1.1)
```

`pub fn fileplan_view(slot: &StoreSlot, session: &Session, target: String) -> Result<FilePlanView, ViewRefused>`,顺序固定:
1. 槽空 → `NoStoreOpened`;
2. `AuthorizedRoots::canonicalized(&session.config().authorized_roots)`;
3. `session_for` + `known_identifiers` 建 PolicySession(与 draft 同一条路,fileplan 反正不出网);
4. `scan_directory(…, Path::new(&target), RequestOrigin::User, now, at)` → `plan_files(…, &report, None, RequestOrigin::User, now, at)`;
5. 任一拒绝 → `Refused` + `reason_code`(未授权即 `PATH_NOT_AUTHORIZED`)+ Display 文本(**保持不回显被拒路径的 oracle 属性**,直接用 `FilePlanError` 的 Display)。

### 2.2 壳侧新面(波次 1 落地)

- 新命令**只有两条**:`draft_view`、`fileplan_view`。wrapper 一行体:`soulcore::commands::draft::draft_view(store.inner(), &session.0, pasted)` 之类。`COMMAND_NAMES` += 两条;`generate_handler!` += 两条;`core.ts` `COMMANDS` += `draftView`/`fileplanView` 及两个类型化包装;TS 类型把 `never_sent: true`、`written_to_disk: false` 写成**字面量类型**。
- **永不注册发送/执行类命令**:`command_surface.rs` 加一条测试,断言 `COMMAND_NAMES` 无任何名字含 `send|submit|deliver|execute|apply|write|move|rename|remove|delete`,并带合成反例证明扫描器会红。
- 路由:`router.tsx` 把 draft/files 的 `ownedBy` 置 null(pending 字段删除);`App.tsx` 加 `<Draft />`、`<Files />` 分支。
- `fakeCore.ts`:两个 case,返回与 Rust DTO 同形的定值(含 `never_sent: true`/`written_to_disk: false` 与 notice 常量副本);各留一条拒绝路径(空粘贴 / 未授权 target 常量),形状 `{ reason, code, message }`,不许比真核心仁慈。
- `contract.test.ts`:用现成的 `rustStringConstant` 把 fakeCore 里的两句 notice 副本与 `draft.rs`/`fileplan.rs` 的 Rust 常量比对(KEY_PROTECTION 同款防漂移)。

---

## 3. 子任务总览与所有权矩阵

```
波次1(串行): ST-V0 soulcore 视图与宿主函数
                 ↓
              ST-V1 壳接线(IPC + 路由 + 骨架页 + 假核心)
                 ↓
波次2(并行): ST-V2 起草页深化   ∥   ST-V3 文件计划页深化
                 ↓
波次3(串行): ST-V4 收口回归 + STATUS
```

| ST | 独占 glob(唯一允许改动) |
|---|---|
| ST-V0 | `crates/soulcore/src/commands/shell.rs`;`crates/soulcore/src/commands/store.rs`;`crates/soulcore/src/commands/draft.rs`;`crates/soulcore/src/commands/fileplan.rs`;`crates/soulcore/tests/shell_commands.rs`(仅加 accessor 测试);`crates/soulcore/tests/draft_view.rs`(新,烟测);`crates/soulcore/tests/fileplan_view.rs`(新,烟测);`.agent_workspace/dev-sota/reports/ST-V0.md` |
| ST-V1 | `apps/desktop/src-tauri/src/{lib.rs, commands.rs}`;`apps/desktop/src-tauri/tests/{ipc_roundtrip.rs, command_surface.rs, store_session.rs}`;`apps/desktop/src/{core.ts, App.tsx, App.test.tsx, router.tsx, contract.test.ts}`;`apps/desktop/src/test/fakeCore.ts`;`apps/desktop/src/routes/Draft.tsx`(骨架,移交 V2);`apps/desktop/src/routes/Files.tsx`(骨架,移交 V3);`.agent_workspace/dev-sota/reports/ST-V1.md` |
| ST-V2 | `crates/soulcore/src/commands/draft.rs`(V1 合入后接管);`crates/soulcore/tests/draft_view.rs`;`apps/desktop/src/routes/Draft.tsx`;`apps/desktop/src/routes/Draft.test.tsx`(新);`fixtures/draft/**`(优先复用现有);`.agent_workspace/dev-sota/reports/ST-V2.md` |
| ST-V3 | `crates/soulcore/src/commands/fileplan.rs`(V1 合入后接管);`crates/soulcore/tests/fileplan_view.rs`;`apps/desktop/src/routes/Files.tsx`;`apps/desktop/src/routes/Files.test.tsx`(新);`fixtures/fileplan/**`(如需);`.agent_workspace/dev-sota/reports/ST-V3.md` |
| ST-V4 | `docs/STATUS.md`;`.agent_workspace/dev-sota/**`;两份 `Cargo.lock` 重解析;合并破损的最小修复(单处 >10 行或涉权限/数据面判断 → 上报重派) |

互斥性说明:ST-V2 与 ST-V3 的 glob 零交集;两者都不含任何共享文件(共享文件在 V1 合入后冻结)。`draft.rs`/`fileplan.rs`/`Draft.tsx`/`Files.tsx` 是串行移交,不构成并行冲突。`styles.css`、`eslint.config.js`、`NavRail.tsx`、`Pending.tsx`、纯 crate(`soul-draft`、`soul-fileplan`)、`soul-policy`、`soul-store`、schema 与 schemas.lock:**本轮无主,任何 ST 不得改**。

---

## 4. 工作单

### ST-V0 soulcore:视图 DTO + 宿主函数(串行第一)

**目标**:按第 2.1 节落地 `StoreSlot`、`Session` 扩展、`ViewRefused`、`DraftView`/`draft_view`、`FilePlanView`/`fileplan_view`,配烟测。让 ST-V1 的 wrapper 有东西可调。

**完成定义**(可红断言):

1. `StoreSlot::default().lock()` 为 None;`install(share(open_test_store(dir, seed)?))` 后为 Some;第二次 `install` 被拒(测试断言返回值)。
2. `Session::draft_profile_id()` 同一 Session 两次调用相等;两个新建 Session 互不相等。`Session::config()` 在 `authorize_root` 之后能看到该 root(`crates/soulcore/tests/shell_commands.rs` 加断言)。
3. 空槽下 `draft_view`/`fileplan_view` 各返回 `ViewRefused { reason: NoStoreOpened }` 且 `message == NO_STORE_FOR_VIEW_EXPLANATION`(逐字)。
4. 真库(tempdir + `open_test_store` + `share` + `install`)+ 默认 Session:粘贴一段第三人文本 → `draft_view` 成功,`serde_json::to_value` 后 `route == "template"`、`never_sent == true`、`text` 非空、`placeheld_turns >= 1`、`notice` 等于常量;库的审计链上出现 `DraftCreate / Allowed`(烟测,深断言归 V2)。全程不起任何 MockLlm。
5. 全空/纯空白粘贴 → `EmptyPaste`,可读中文 message,审计链无新增条目。
6. 真库 + `authorize_root(tempdir A)` 的 Session:对 A 建一棵有可整理项的树(照 `fileplan_commands.rs::build_tree` 手法)→ `fileplan_view(A)` 成功,`written_to_disk == false`、`entry_count >= 1`、counts 与 entries 对得上;对 A 之外的 tempdir B → `Refused` 且 `code == Some("PATH_NOT_AUTHORIZED")`,message **不含** B 的路径字符串(oracle 属性);空 roots(默认 Session)同样拒绝。
7. `DraftView`/`FilePlanView`/`ViewRefused` 的 derive 行只有 Serialize 没有 Deserialize(源码级断言可放 V2/V3,本包保证事实)。
8. 纯 crate、`commands/mod.rs`、所有 `Cargo.toml`、schema 零改动(git diff 范围即断言)。
9. 根树全绿:`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets --all-features -- -D warnings`、`cargo test --workspace --all-targets`、`cargo run -p xtask -- e0-audit` / `denylist-audit` / `schema-freeze --check`、`cargo deny check`。

**必须调用的现有 API**(不得重写):`draft_reply`、`session_for`、`known_identifiers`、`DraftRequest::new`、`DraftOutcome::{text, route, stats}`、`DraftStats::{turns, third_party_turns, placeheld_turns, carries_exempted_original}`、`DraftCommandError::reason_code`;`scan_directory`、`plan_files`、`FilePlanRefusal::reason_code`、`AuthorizedRoots::canonicalized`、`ScanReport::{scan_id, file_count, dir_count, skipped_escaping_links}`、`FilePlanPreview::{len, count_of, entries, scan_id}`、`PlanEntry::{source_rel, action, target_rel}`、`PlanAction::as_str`;`PastedTurn::unattributed`、`UntrustedText::new`;`collect::share`、`store::open_test_store`;`soul_policy::hitl::RequestOrigin::User`。dev:`tempfile`、`soul_store_api::AuditLog`(读链)。

**红线**:不碰 `soul-draft`/`soul-fileplan`/`soul-policy`/`soul-store` 任何文件(不给纯 crate 加 serde——DTO 就够);不新增审计动作/ReasonCode/schema;宿主函数不额外落审计;`DraftRequest.exemption` 恒 None,公开面不提供传豁免的口;不给 people_summary 造 DTO 或宿主函数;`never_sent`/`written_to_disk` 无 setter、无第二构造;`Config.llm_endpoint` 字符串不进任何 DTO/错误文本;不动 `commands/mod.rs`;时钟只在 soulcore 读;不加依赖。

**依赖**:无(第一波)。

---

### ST-V1 壳接线:IPC + 路由 + 骨架页(串行第二)

**目标**:两条新命令过三方一致;mock 运行时拿到可读的"没开库"拒绝;`/draft`、`/files` 从 Pending 变成能用的骨架页;`App.test.tsx` 按新现实更新且旧承诺(无发送、无执行)原样保住。

**完成定义**(可红断言):

1. `configure()` 同时 `manage` `SessionConfig::default()` 与 `StoreSlot::default()`;`install_store` 改为填槽,不再直接 `manage` 句柄。`store_session.rs` 现有五条测试**不改一字**继续绿(`open_store` 出现次数仍恰好 lib.rs 一处;commands.rs 仍无 `Arc`/`Mutex`/`SqlCipherStore` 字样——wrapper 只 import `StoreSlot`/`ViewRefused`/两个 View 类型)。
2. `commands.rs` 两个新 wrapper 函数体各一行;`COMMAND_NAMES` 七条;`command_surface.rs` 三条既有测试绿,并新增"命令名无发送/执行词汇"测试(见 2.2,含合成反例)。
3. `core.ts`:`COMMANDS` 七条;`DraftView`/`FilePlanView`/`ViewRefused` TS 类型与 Rust DTO 同形,`never_sent: true`、`written_to_disk: false` 为字面量类型;`draftView(pasted)`/`fileplanView(target)` 包装。`contract.test.ts` 的命令名比对自动覆盖,新增两句 notice 的常量比对(`rustStringConstant` 读 `draft.rs`/`fileplan.rs`)。
4. `fakeCore.ts` 新增两个 case(定值 + 拒绝路径,形状 `{reason, code, message}`);default 抛错分支保留。
5. `router.tsx`:draft/files `ownedBy: null`;`App.tsx` 渲染 `Draft`/`Files`。骨架页可用:Draft = 一个 textarea + 「生成草稿」按钮 + 结果区(`text`、`route_label`、`placeheld_turns` 计数、`notice` 逐字)+ `refusalText` 告警区;Files = `authorizedRoots()` 列表(空则提示去设置页授权,句子不许出现被禁词)+ 选中一个 root + 「扫描并预览」按钮 + 结果表(`action_label`/`source_rel`/`target_rel`,纯文本行,行内零交互元素)+ counts + `notice` 逐字 + 告警区。
6. `App.test.tsx` 更新:`/draft` 有 textbox、`queryByRole("button", {name: /发送|send|submit|deliver/i})` 为 null、无 `pending-owner`;`/files` 的"全部按钮不匹配 `/执行|应用|移动|重命名|删除/`"循环**原样保留**、无 `pending-owner`、有扫描按钮。向导/设置/未知路由/核心不回应四条不动。
7. `ipc_roundtrip.rs` 新增:mock 运行时下 `draft_view`(带 `{"pasted": ["…"]}`)与 `fileplan_view`(带 `{"target": "…"}`)都返回 Err,`reason == "no_store_opened"`、`message` 等于 Rust 常量;缺参数报错(参数拼写钉死);伪 origin 循环加上两条新命令。**不在 mock 测试里开任何 SQLCipher**。
8. 前端 `forbidNetwork` 五桩在新页面测试路径上零调用(骨架烟测即可,深测归 V2/V3)。
9. 全绿:`just desktop-check`/`desktop-test` 等价、`pnpm ui-lint`/`ui-test`;根树不动(soulcore 无改动)。

**必须调用的现有 API**:`soulcore::commands::draft::{draft_view, DraftView}`、`soulcore::commands::fileplan::{fileplan_view, FilePlanView}`、`soulcore::commands::store::StoreSlot`、`soulcore::commands::shell::ViewRefused`(全部 V0 产物);`tauri::State::inner`;前端 `core.ts::refusalText`、`mockIPC`、`forbidNetwork`。

**红线**:不注册第三条命令;命令名与 wrapper 里不得出现发送/执行词汇;不加 tauri 插件、capabilities 只保 `core:default`、CSP 不放宽;WebView 零业务判断(空粘贴、路径合法性都由核心拒绝,页面只渲染 message);按钮可达名不含"发送"二字(注意「不发送」也命中正则);不改 `styles.css`/`eslint.config.js`/`NavRail.tsx`/`Pending.tsx`/`Settings*`/`Wizard*`/`Home.tsx`;`Cargo.toml`/`package.json`/lockfile(除 src-tauri Cargo.lock 由 cargo 再生)不动;不跑 `tauri build`。

**依赖**:ST-V0 合入。

---

### ST-V2 起草页深化(与 ST-V3 并行)

**目标**:把 `/draft` 从骨架变成可信的产品面:结果与拒绝都可读、"永不发送"三层钉死(类型层、源码层、测试层),soulcore 侧视图路径补齐 AC 级断言。

**完成定义**(可红断言):

1. **soulcore 深测**(`crates/soulcore/tests/draft_view.rs` 扩):
   a. 用 `fixtures/draft/third_party_paste.json` 的正文/姓名/账号拼粘贴,`draft_view` 后把整条审计链序列化,`LeakageChecker`(≥8 字 scalar + KnownIdentifiers)零命中;
   b. 含注入串的粘贴(`fixtures/injection` 复用):照常出草稿,链上有 `InjectionBlocked` 条目且无正文;
   c. `never_sent` 源码级:读回 `commands/draft.rs`,断言 `never_sent` 只在唯一构造处被赋 `true`、无 `pub` 字段、derive 无 Deserialize;合成反例证明扫描器会红(WP07 三层模式);
   d. 多段粘贴(Vec 多项)→ `turns` 计数与项数一致;
   e. 拒绝可读:`ViewRefused` 的 message 不含粘贴正文(以粘贴串为语料扫错误文本)。
2. **前端**(`Draft.test.tsx` 新建):粘贴→点「生成草稿」→ 屏幕出现 fakeCore 的 text、notice 逐字、占位计数;空粘贴→拒绝语上屏(role=alert);`forbidNetwork()` 全程空;整页 `queryAllByRole("button")` 无一命中 `/发送|send|submit|deliver/i`;结果区无任何把草稿递出去的控件(断言只存在「生成草稿」与可能的「清空」)。
3. 页面渲染核心原话:notice、route_label、refusal 一律逐字,不改写、不翻译。
4. 全绿:根树 + `pnpm ui-lint`/`ui-test`(desktop rust 树不受影响,V2 不碰它)。

**必须调用的现有 API**:V0 宿主面;`soul_testkit::fixtures::read_json`、`soul_testkit::leakage::LeakageChecker`;`soul_store_api::AuditLog`;`store::open_test_store` + `collect::share` + `StoreSlot`。

**红线**:不碰共享文件(第 3 节冻结清单);DTO 字段集冻结,想加字段先上报;不引入 MockLlm 到视图测试(视图路径无 E1 可达);不加发送/复制到剪贴板之类的"递出"控件;不碰 `soul-draft`;测试不得自造占位符逻辑。

**依赖**:ST-V1 合入。与 ST-V3 真正并行。

---

### ST-V3 文件计划页深化(与 ST-V2 并行)

**目标**:把 `/files` 变成可信的只读预览:拒绝矩阵、磁盘不变性、`written_to_disk` 构造级、审计无文件名,页面零执行控件。

**完成定义**(可红断言):

1. **soulcore 深测**(`crates/soulcore/tests/fileplan_view.rs` 扩):
   a. 磁盘不变:视图前后对授权树逐文件 `(路径, 字节数, 内容哈希, mtime)` 快照比对全等;
   b. 拒绝矩阵逐一断言(不抽样):roots 空;B 不在 roots;`A/../B`;A 内指向 B 的 symlink 目标(canonicalize 后逃出即拒,Linux CI 可建);B 是 A 的父目录——全部 `code == Some("PATH_NOT_AUTHORIZED")` 且 message 不含被拒路径;每次拒绝链上多一条 `FilePlan / Denied`;
   c. 文件名注入:树里放注入串/人名文件名 → 照常列出;把全部文件名当语料,序列化审计链过 `LeakageChecker` 零命中;
   d. `written_to_disk` 源码级:读回 `commands/fileplan.rs`,断言只在唯一构造处赋 `false`、无 setter、derive 无 Deserialize;合成反例;
   e. `entries` 与 `ScanReport`/`FilePlanPreview` 口径一致(counts 相加、`target_rel` 对 move/rename 必有)。
2. **前端**(`Files.test.tsx` 新建):roots 列表上屏;点「扫描并预览」→ 建议表 + counts + notice 逐字;空 roots → 指路设置页的句子;拒绝 → message 上屏(role=alert);`forbidNetwork()` 空;整页 `queryAllByRole("button")` 无一命中 `/执行|应用|移动|重命名|删除/`,建议行内零 button/link/input(`within(row).queryAllByRole(…)` 为空)。
3. 页面渲染核心原话:action_label、notice、refusal 逐字。
4. 全绿:根树 + `pnpm ui-lint`/`ui-test`。

**必须调用的现有 API**:V0 宿主面;`AuthorizedRoots`、`ScanReport`、`FilePlanPreview` 的读口;`LeakageChecker`;`AuditLog`;`open_test_store`/`share`/`StoreSlot`;`shell::Session::authorize_root`(建授权态)。

**红线**:不碰共享文件;DTO 冻结;不加执行/撤销/写入的任何入口(类型面、命令面、UI 面三层都无);不消费 `CapabilityScope::FileWrite`;不碰 `soul-fileplan`/`soul-policy`;审计断言不许放宽("无路径、无文件名"是硬线);预览可以显示相对路径(用户自己的盘),审计不行。

**依赖**:ST-V1 合入。与 ST-V2 真正并行。

---

### ST-V4 收口回归 + STATUS(串行殿后)

**目标**:V2/V3 合入后整棵树回到全绿,进度如实写回文档。

**完成定义**:

1. `just ci` 等价全绿 + `cargo deny check`;`just desktop-check`/`desktop-test` 等价全绿;`pnpm ui-lint`/`ui-test` 全绿。
2. `docs/STATUS.md` 增"WP09 功能视图(起草页/文件计划页)"一节:交付/证据/取舍,从 `reports/ST-V*.md` 合成,不夸大;Windows 真机才能验的(真窗口里的两页)照 WP09 先例进手动清单。
3. `.agent_workspace/dev-sota/PROGRESS.md` 循环表更新。
4. 两份 `Cargo.lock` 重解析;合并破损最小修复,每处归因,>10 行或数据面判断 → 上报重派。

**红线**:不加功能、不顺手重构、不动 schema、不跑 tauri build、不启动本轮 out 清单里的任何项。

**依赖**:ST-V2 + ST-V3 全部合入。

---

## 5. 本轮不做(写死,实现者不得"顺手"越界)

| 项 | 去向 |
|---|---|
| **D32 作者锁**:QQ/微信客户端读取 | **OUT**。QQ 日后走既有 OSS、微信日后走 wechat-rpa,都在 soul.exe **之外**;Soul 只吃 `soul-import-v1` 文件。无采集 UI、无 OAuth、无 RPA,连按钮占位都不许 |
| Goal 2 全部 | Goal 1 关门后 |
| 文件写执行/撤销、`FileWrite` 消费、执行按钮 | v0.1.1 |
| 发送/递出草稿的任何形态(含"复制并发送"式按钮) | 无代码路径,恒禁 |
| people_summary 的 IPC/视图 | 等人脉图视图给出选人入口(P1) |
| 豁免(OneShotExemption)过 IPC | 需按 turn 的同意 UI,P1 |
| LLM 端点录入 UI(`llm_endpoint` 写入口) | WP13/P1;本轮起草恒走模板路线并如实标注 |
| profile/graph/memory/import/research/audit 视图 | P1(简报明说:宁可不做"顺手的小只读") |
| DPAPI Win32、`tauri build`/MSI、配置持久化、双问卷合并 | 照 TASK_SPLIT 第 4 节原样,归属不变 |

---

## 6. 派工表

每格一个 `claude-opus-5-thinking-high-fast`(降级链:`claude-opus-5-thinking-high` → `claude-sonnet-5-thinking-high`;子代理第一行自报 slug)。分支建议 `cursor/st-v<N>-<名>-a1a4` 系列,基于 `agent/dev-sota`,完工合回 `agent/dev-sota`。

| 波次 | ST | 包 | 必须等谁 | 可与谁同时跑 |
|---|---|---|---|---|
| 1 | ST-V0 | soulcore 视图 DTO + 宿主函数 | 无 | 无(串行) |
| 1 | ST-V1 | 壳接线:IPC + 路由 + 骨架页 | ST-V0 合入 | 无(串行) |
| 2 | ST-V2 | 起草页深化 | ST-V1 合入 | ST-V3 |
| 2 | ST-V3 | 文件计划页深化 | ST-V1 合入 | ST-V2 |
| 3 | ST-V4 | 收口回归 + STATUS | ST-V2+ST-V3 合入 | 无(独占) |

合并顺序:V0 → V1 → V2/V3(任意先后,glob 互斥)→ V4。

每个 ST 的通用要求(写进派工提示词):第一行自报 slug;只改自己 glob;完成后跑本工作单"完成定义"点名的命令并把结果写进 `reports/ST-V*.md`;发现必须越界就停下上报;commit 按仓库风格一句话说清为什么;**共享文件在 V1 合入后即冻结,V2/V3 一个字都不碰**。
