MODEL: claude-fable-5-thinking-xhigh

# ST-03 对照 SOTA_BARS.md 壳接库四条（S-01～S-04）核对报告

核对人立场：独立复核，不信 `reports/ST-03.md` 的自述，逐条对照工作树源码与测试原文，并在本机重跑了三组测试取证。核对对象是当前工作树（分支 `agent/dev-sota`，ST-03 改动已在 `2920d2f` 内，工作树干净）。

**本机复跑结果（2026-08-24，Linux）：**

```
cargo test -p soulcore --test shell_commands                     → 22 passed, 0 failed
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets
    command_surface.rs 4 | ipc_roundtrip.rs 11 | no_egress_path.rs 3
    shell_is_local_only.rs 7 | store_session.rs 6                → 全部 0 failed
pnpm test（apps/desktop, vitest）                                → 5 files, 29 passed
```

**总判**：S-02、S-03、S-04 达标；S-01 部分达标——断言 2（运行期跨命令共享库句柄）是已承认的缺口，根因是本包不接任何触库命令、无可测面，属结构性顺延而非偷工。

---

## S-01 setup 只 open 一次；所有命令同一 `Arc<Mutex<SqlCipherStore>>`

**判定：部分达标（断言 1 达标且超标，断言 2 缺口，断言 3 条件未触发但防线已设）**

### 断言 1（源码级）——达标，且强于条文

条文要 `include_str!` 手法；实现改用**运行时递归枚举** `src/**/*.rs`（新增文件躲不掉），这是 SOTA_BARS 先例（`soul-collect` 扫全目录）的同款升级，方向正确。

- `open_store` 字面量在整个壳 `src/` **恰好一次**且在 `lib.rs`（实际命中的是 `open_store_for_session` 调用，`lib.rs:62`）；`open_test_store` / `SqlCipherStore::open` 零次：
  - 证据：`apps/desktop/src-tauri/tests/store_session.rs::the_store_is_opened_in_exactly_one_place`（82–100 行）
- `commands.rs` 里 `open_store` / `SqlCipherStore` / `Mutex` / `Arc` 全部零次：
  - 证据：`store_session.rs::no_command_opens_a_store_of_its_own`（105–113 行）
- 反向锚点（防「扫错文件全绿」）：断言扫到 `open_store_for_session` / `fn install_store` / `fn run` / `fn configure`，另有合成字符串对照用例证明扫描器认得出禁词：
  - 证据：`store_session.rs::the_scanner_notices_the_things_it_is_looking_for`（136–150 行）
- `.manage(` 管理的类型：代码合规——`app.manage(session.into_handle())`（`apps/desktop/src-tauri/src/lib.rs:66`），`into_handle()` 返回 `Arc<Mutex<SqlCipherStore>>`（`crates/soulcore/src/commands/store.rs:97–99`）。
  **小缺口**：没有任何测试钉住「open 出来的句柄确实被 `.manage`」——一个「开了库但把句柄丢掉」的实现能过现有全部源码断言。风险低（`configure_opens_nothing_and_setup_does` 已钉 `run()` 里 `install_store(app)` 必在 `setup` 内，`store_session.rs:156–169`），建议第一条触库命令落地时补一行 `.manage(` 存在性断言。

### 断言 2（运行期：命令 A 写、命令 B 读，跨命令可见）——缺口（已承认，结构性）

本包一条触库命令都没接（功能视图归 P1），`Arc<Mutex<SqlCipherStore>>` 被 manage 后无人消费，所以「两个连接各写各的 WAL 时会红」的运行期取证**没有可测面**。替代证据只到 session 层：

- `apps/desktop/src-tauri/tests/ipc_roundtrip.rs::authorising_a_directory_is_visible_to_the_next_command`（187–223 行）：同一 app 连发四条命令（读→写→读→读），证明 managed state 是同一个 `Session` 而非每次一份默认值——`invoke_all` 特意在一个 app 上串行调用（49–83 行，注释明说建新 app 会掩盖这件事）。
- 库句柄本身的最近取证在核心侧：`crates/soulcore/tests/shell_commands.rs::the_session_store_falls_back_to_a_key_file_and_says_so`（258–261 行）`Arc::ptr_eq(session.handle(), session.handle())`——同一 SessionStore 每次给同一句柄，但这不等于「两条 IPC 命令拿到同一句柄」。
- 缺口自述：`reports/ST-03.md` §1.1「已知未覆盖」与 §4 第一行，如实记录。
- **遗留动作**：第一条触库命令（P1 功能视图）落地时必须补齐 S-01.2 原文断言，届时这条从缺口转达标。

### 断言 3（与采集共句柄）——条件未触发，防线已设

条文是条件句「若本单接了 collect 命令」；本包没接（`commands.rs` 全文只有 5 条配置/授权命令）。条文要的源码级防线「没有第二个 `Arc::new(Mutex::new(`」已经落了，且比条文更严——整个壳 `src/` 禁 `Arc::new` 与 `Mutex::new`：

- 证据：`store_session.rs:90–94`；共享包装的唯一入口在核心：`crates/soulcore/src/commands/store.rs:124,128` 调 `crate::commands::collect::share`。

---

## S-02 open 的选择逻辑在 soulcore，不在壳

**判定：达标**

- 选择逻辑住在 `soulcore::commands::store::open_store_for_session(directory)`（条文「形如 `open_session_store(app_data_dir)`」的同形入口）：**每次都先问** `DpapiKeyProvider::key_encryption_key()`，`Ok` 走平台 provider，`Err` 回退 `TestKeyProvider::in_dir` 的明文密钥文件——不是 `cfg!(windows)` 静态分支：
  - 证据：`crates/soulcore/src/commands/store.rs:118–132`（及 107–117 行注释点名「security decision 不进壳」的理由）
- 壳的参与仅限「拿目录、调 soulcore、manage 返回值」（外加把核心观察到的 `KeyProtection` 事实转交 session——转交事实不是决策）：
  - 证据：`apps/desktop/src-tauri/src/lib.rs::install_store`（58–68 行，正文三个语句）
- 源码级断言：壳 `src/` 全目录（运行时枚举）零出现 `KeyProvider` / `TestKeyProvider` / `DpapiKeyProvider` / `key_encryption_key` / `database_key` / `cfg!(windows)`——`KeyProvider` 是后两个 provider 名的子串，一条断言盖三个名：
  - 证据：`apps/desktop/src-tauri/tests/store_session.rs::the_shell_does_not_choose_the_key_provider`（117–131 行）
- 壳没有绕开 soulcore 的第二条依赖路径：`store.rs:26–28` re-export 存储类型，`apps/desktop/src-tauri/Cargo.toml` 保持单条 path 依赖（`reports/ST-03.md` §1.2 自述，与 `no_egress_path.rs` 3 条全绿相符）。

---

## S-03 无 DPAPI 时不得在 UI 文案声称已保护

**判定：达标（一处扫描器覆盖面注脚，不构成缺口）**

前提复核：`DpapiKeyProvider` 两个入口今天确实仍返回 `KeyError::Unsupported`——`crates/soul-store/src/keys.rs:231–246`（Windows/非 Windows 两个 `unprotect` 都 `Err(Unsupported)`），配套测试 `keys.rs:307–316`。

### 锁 1：核心算，界面渲染——达标

- `ConfigSnapshot.kek_protected: bool` 由 `KeyProtection::kek_protected()` 算出（`matches!(PlatformKeyStore)`），`KeyProtection` 只能来自 `open_store_for_session` 观察到的 provider 应答，`Config` 改不动它：
  - 证据：`crates/soulcore/src/commands/shell.rs:82–113`（类型 + 注释「Not a setting」）、`150–165`（`of_session` 组装）
- 穷举断言：16 种 `Config` 组合 × 2 种可达 `KeyProtection`，`kek_protected` 恒 false，且 `key_protection` 句子恒等于 `keys.explanation()`：
  - 证据：`crates/soulcore/tests/shell_commands.rs::no_configuration_can_claim_the_key_is_protected`（209–238 行，先例 `no_configuration_can_claim_the_cloud_is_available` 的同款结构）
- 反向（防常量 false）：`PlatformKeyStore.kek_protected() == true`：
  - 证据：`shell_commands.rs::the_protection_flag_follows_the_provider_that_answered`（196–204 行）
- TestKeyProvider 路径的真库取证：tempdir 真开库 → `UnprotectedKeyFile`、`!kek_protected`、`soul.db` 与 `soul-test-keys.bin` 真的落盘（文案点名的文件必须存在）：
  - 证据：`shell_commands.rs::the_session_store_falls_back_to_a_key_file_and_says_so`（242–268 行）
- session 只报被告知的保护等级：`shell_commands.rs::the_session_reports_only_the_protection_it_was_told_about`（295–307 行）

### 锁 2：文案跨语言逐字比对——达标

- 文案是 soulcore 常量 `KEY_FILE_NOT_PROTECTED_EXPLANATION`（仿 `CLOUD_NOT_YET_AVAILABLE_EXPLANATION`）：`crates/soulcore/src/commands/shell.rs:58–61`
- TS 侧读 Rust 源码、还原反斜杠续行、与 `fakeCore.KEY_PROTECTION` 逐字相等：
  - 证据：`apps/desktop/src/contract.test.ts::密钥说明的文案和 soulcore 里的常量是同一句话`（97–100 行；`rustStringConstant` 49–54 行）
- 屏幕上渲染的正是这个常量、一字不差：`Settings.tsx:115` 原样渲染 `{snapshot.key_protection}`；
  - 证据：`apps/desktop/src/routes/Settings.test.tsx::密钥说明是核心给的那句话，一个字都没改`（48–54 行，`textContent` 全等）
- 常量本身不夹带宣称：`shell_commands.rs::the_key_sentence_never_claims_a_protection_that_does_not_exist`（272–293 行，禁 6 种宣称 + 必含 DPAPI/明文/文件名）

### 锁 3：禁词扫描——达标（附注脚）

- 扫 `apps/desktop/src/**` 的 `.ts/.tsx/.css`（测试文件除外），`DPAPI`、`已受保护`、`已加密保护`、`受到保护`、`已经保护`、`安全保管` 一律零命中——禁整个 `DPAPI` 一词，严于条文的「DPAPI 保护」：
  - 证据：`apps/desktop/src/contract.test.ts::界面自己不声称密钥受到任何保护`（142–155 行）
- **注脚**：条文写的扩展名清单是 `.ts/.tsx/.css/.html`，扫描器少了 `.html`。现状无实害——`src/` 下没有任何 html，仓库唯一的 `apps/desktop/index.html` 在条文 glob（`src/**`）之外且已人工核对无宣称字样（只有 title 和挂载点）。若日后 `src/` 下出现 html 会漏扫，建议顺手把 `".html"` 加进 `sourceFiles` 的扩展名数组（一行改动，非本次动作）。

---

## S-04 wrapper 函数体仍 ≤1 语句，三方名单咬合保持

**判定：达标**

- 四条既有测试对新增两条命令继续绿（本机复跑 `command_surface.rs` 4 passed）：
  - `both_sides_name_the_same_commands` / `every_named_command_is_actually_a_command` / `no_command_exists_outside_the_list` / `the_command_layer_stays_thin`
  - 证据：`apps/desktop/src-tauri/tests/command_surface.rs`（31–113 行；thin 检查逐命令数非空非注释行 ≤1）
- 三处名单同步齐：
  - `core.ts` `COMMANDS`（`apps/desktop/src/core.ts:67–73`，含 `authorize_root` / `authorized_roots`）
  - `COMMAND_NAMES`（`apps/desktop/src-tauri/src/commands.rs:59–65`）
  - `#[tauri::command]` + `generate_handler!` 注册（`commands.rs:23–52`、`lib.rs:35–41`）
  - TS 侧再比一遍：`contract.test.ts::界面用的命令名和 src-tauri 注册的一模一样`（72–78 行）
- 五个 wrapper 函数体全部恰好 1 语句、只转调 `Session` 的一个方法——取锁、canonicalize、错误映射全在 `soulcore::commands::shell::Session`（`shell.rs:397–454`），壳里放不下也确实没放：
  - 证据：`apps/desktop/src-tauri/src/commands.rs:24–52`（逐个目检：`session.0.snapshot()` / `.complete_wizard(&answers)` / `.cloud_toggle(requested_on)` / `.authorize_root(&path)` / `.authorized_roots()`）
- `ipc_roundtrip.rs` 对每个**新**命令正反两例齐备：
  - `authorize_root` 正：`authorising_a_directory_is_visible_to_the_next_command`（187–223 行，真临时目录、断言回读的是 canonicalize 后的路径）；另有三种可读拒绝（228–256 行）与缺参必错（260–263 行）
  - `authorized_roots` 正：同一用例的第 1、3 次调用（先空后一条）
  - 反（伪造 origin 拒）：`a_call_claiming_a_different_origin_is_refused`（269–289 行）对 `config_snapshot` / `authorized_roots` / `authorize_root` 三条全拒
  - 本机复跑 `ipc_roundtrip.rs` 11 passed

---

## 缺口清单（汇总）

| 编号 | 严重度 | 内容 | 归属动作 |
|---|---|---|---|
| S-01.2 | 中（已承认） | 无触库命令 → 运行期「跨命令共享同一库句柄」无可测面；替代证据只覆盖到 Session 层 | 第一条触库命令（P1 功能视图）落地时补原文断言 |
| S-01.1 附 | 低 | 无测试钉「open 出来的句柄确实被 `.manage`」，「开了就丢」能过现有源码断言 | 与上一条同时补一行 `.manage(` 存在性断言 |
| S-03.3 附 | 极低 | 禁词扫描器扩展名少 `.html`；现状 `src/` 下无 html、无实害 | 择机把 `".html"` 加进 `contract.test.ts` 的 `sourceFiles` 扩展名 |

除上述外，S-02、S-03、S-04 三条按 SOTA_BARS 原文逐款达标，且 S-01.1 的扫描手法（运行时枚举）与 S-03.3 的禁词范围（禁整词 `DPAPI`）两处严于条文。
