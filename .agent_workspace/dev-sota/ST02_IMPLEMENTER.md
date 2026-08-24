MODEL: claude-fable-5-thinking-xhigh

# ST-02 实现者清单 — WP11 只读扫描 + 计划预览

压缩自:TASK_SPLIT ST-02、SURFACES_WP11_STORE、SOTA_BARS F-01~F-06、TEST_PATTERNS。冲突时以本文为准。

## 0. 边界(先读)

- **依赖 ST-00 合入后开工**。ST-00 骨架已在工作树(尚未 commit):`ReasonCode::PathNotAuthorized` 三处已加、`soul-fileplan` 空壳、`commands/fileplan.rs` 存根、`fixtures/fileplan/`(约定 README + `sample_tree.json`)都已备好。ST-02 **不碰**任何 `Cargo.toml`、`soul-policy`、UI、`docs/STATUS.md`。
- 独占 glob:`crates/soul-fileplan/**`、`crates/soulcore/src/commands/fileplan.rs`、`crates/soulcore/tests/fileplan_commands.rs`、`fixtures/fileplan/**`(如需)、`.agent_workspace/dev-sota/reports/ST-02.md`。越界即停,上报父代理。
- 分层:`soul-fileplan` 纯逻辑(roots 作参数、无 Config、无库句柄、无网);`soulcore::commands::fileplan` 编排(check_action + 审计)。
- **不需要令牌**:`ScanDirectory`/`PlanFiles` 的 `needs_capability_token() == false`。源码里出现 `TokenIssuer`/`FileWrite` 就是走错路。
- 不做:执行/apply/undo(v0.1.1)、UI(`/files` 的 Pending 与"无执行按钮"断言不许动)、`authorized_roots` 写入口(归 ST-03)。

## 1. 类型(`soul-fileplan`,建议模块 authorize / scan / plan)

| 类型 | 要点 |
|---|---|
| `AuthorizedRoots` | newtype;唯一构造 `canonicalized(&[PathBuf]) -> Result<Self, FilePlanError>`:逐个 canonicalize、须存在且是目录、去重。空表合法但授权不了任何路径 |
| `ScanEntry` | `rel_path: PathBuf`(相对 root)、`bytes: u64`、`kind: EntryKind{File,Dir}`。文件名是数据,原样保留 |
| `ScanReport` | `root_id: Uuid`、`entries: Vec<ScanEntry>`(按 rel_path 显式排序)、`skipped_escaping_links: usize` |
| `PlanAction` | `enum { Move, Rename, Group }`,纯描述值,无方法 |
| `PlanEntry` | `source_rel: PathBuf`、`action: PlanAction`、`target_rel: Option<PathBuf>`。target 只是建议名,不是承诺 |
| `FilePlanPreview` | 字段私有;`written_to_disk()` 恒返回 `false`,**唯一构造入口不收该值**(仿 `zero_third_party_rows` 构造级强制);`entries()` 按 (source_rel, action, target_rel) 排序;`to_plan_json() -> serde_json::Value`(见 §3) |
| `FilePlanError` | thiserror。变体至少:`PathNotAuthorized { roots: Vec<PathBuf> }`(**不含请求路径**,消息只说"不在授权根内"+授权根)、`RootUnreadable`、`NotADirectory` |

公开入口只有两个:`scan(&AuthorizedRoots, target: &Path) -> Result<ScanReport, FilePlanError>` 与 `plan(&ScanReport) -> FilePlanPreview`。没有收 `String` 的直通入口;文件名进任何下游走 `UntrustedText`(`ExternalChannel::FileName`)。

`soulcore::commands::fileplan` 薄封装(签名对齐 `commands/policy.rs` 风格):`scan_directory`、`plan_files`。各自:`check_action` → 调 `soul-fileplan` → `append_or_store_error` 落审计。E1/出网:本包完全无。

## 2. 授权判定算法(顺序即安全,逐条照做)

1. roots 预处理在 `AuthorizedRoots::canonicalized` 一次做完:`fs::canonicalize` 每个 root,失败 → 可读错误;去重。
2. roots 为空 → 任何请求直接 `PathNotAuthorized`。
3. 请求目标 `fs::canonicalize(target)`;**失败即拒**(不存在的路径无法证明在根内,不得先前缀后规范化)。
4. 前缀判定:`canonical_target.starts_with(canonical_root)`,用 **`Path::starts_with`(组件级)**,对任一 root 命中即过。**禁止字符串 `starts_with`**(经典洞:授权 `/data/a` 放过 `/data/ab`)。
5. **canonicalize 必须在前缀判定之前**——`A/../B` 与"A 内 symlink 指向 B"只有规范化后才现形。显式以逃逸路径为扫描目标的请求 → 整体拒。
6. 遍历期:`walkdir::WalkDir::new(root)`(默认 `follow_links(false)`,保持);entry 是 symlink 时 canonicalize 其目标,逃出 root → 不进 entries、不进计划、`skipped_escaping_links += 1`。结果中 B 下任何文件的路径/名字/字节零出现。
7. B 是 A 的父目录:canonicalize 后不以任何 root 为前缀 → 第 4 步自然拒,单独写测试钉住。
8. 大小写/Unicode:判定只基于 canonicalize 后的字节;NFD 变体、大小写变体在 Linux CI 上要么 canonicalize 失败要么前缀不命中 → 拒;测试把这个决定写死(Windows junction 进手动清单,留 `cfg(windows)` 位)。
9. 拒绝输出:返回 `FilePlanError::PathNotAuthorized`(带类型,非空计划、非 panic);编排层落审计 `FilePlan / Denied / PATH_NOT_AUTHORIZED`。**100% 是字面义**:拒绝矩阵逐条断言,请求数 == 拒绝数,无部分成功。

## 3. HITL 接线与计划 hash

- 编排层每次请求过 `check_action`:`ActionRequest::new(ActionKind::ScanDirectory.as_str(), RequestOrigin::User)`(plan 侧同理用 `PlanFiles`)。`RequestOrigin::ExternalContent` → 被 `ExternalContentNotAuthority` 拒,这是文件名注入的必测路径。
- 批准流:`.with_plan(plan_json).approved_as(PlanHash::of(&plan_json))`;计划改动任一语义字段 → `PlanHashMismatch`。
- `plan_json` 形状(仿 `e1_plan`:计数与类别):`root_id`、`entry_count`、按 `PlanAction` 的分类计数、`entries` = 逐条 (source_rel, action, target_rel) 的规范化编码,**显式排序**(不依赖遍历顺序,否则同一棵树两次扫 hash 不同 → 误拒)。hash 覆盖全部语义字段(漏 target 的 hash 等于没 hash)。禁止 hash 算在 `format!("{:?}")` 上。
- `FilePlanPreview` 是唯一真源:用户看的渲染与被 hash 的 JSON 同源派生,改显示层骗不过批准。
- 审计:成功 `FilePlan / Allowed`,内容只有 uuid 与计数(`items` = 文件数、拒绝数);**JSON 本体与任何文件名/路径不落审计**(预览给用户看可以含路径,审计不行)。落链用 `append_or_store_error(store, content, now)`。
- 令牌:全流程 `TokenIssuer::issued_count() == 0`;手工 `issue_token(CapabilityScope::FileWrite, hash, now)` → `WriteNotImplemented` 拒(既有测试,不许动)。

## 4. 无写 API 扫描词表(源码级自查用)

扫描方式:运行时枚举 `crates/soul-fileplan/src/` 下**全部** `.rs`(`env!("CARGO_MANIFEST_DIR")` + 递归 read_dir,测试里读文件系统允许),**不用 `include_str!` 钉死名单**——新加文件不能躲。只扫 `src/`,不扫 `tests/`(tests 允许 tempfile 与写 setup),不扫 `Cargo.toml`。

```text
fs::write        File::create      File::options     OpenOptions
create_dir       remove_file       remove_dir        fs::rename
fs::copy         hard_link         os::unix::fs::symlink   symlink_file
symlink_dir      set_permissions   set_len           .truncate(
.append(         .create_new(      BufWriter         io::Write
write!           writeln!          tempfile
```

- `create_dir` 同时覆盖 `create_dir_all`;`fs::rename`/`fs::copy` 按调用形态列,裸 `rename`/`copy` 会误伤 `PlanAction::Rename` 与 `Copy` trait;裸 `symlink` 会误伤授权逻辑的标识符/注释,故列创建 API 三兄弟。
- 渲染字符串用 `format!`,不用 `write!`(词表禁了它)。
- **明确不禁**:`PathBuf`、`Path`、`read_dir`、`walkdir`、`fs::canonicalize`、`metadata`——`research_preview.rs` 那张表含 `PathBuf`/`tempfile`,**不能照抄**。
- 三件配套断言缺一不可:① 锚点——扫到的文件数 ≥ 实际模块数,且读到 `fn scan` 与 `fn plan`(防文件挪走后空绿);② 对照——往合成字符串塞 `fs::write` 证明扫描器会红;③ 公开面无 execute/apply/undo 命名的函数(编译期事实,测试注释写明)。

## 5. 测试函数名

`crates/soul-fileplan/tests/authorized_scan.rs`(AC-18 正向 + 磁盘不变;建树按 `fixtures/fileplan/README.md` 约定读 `sample_tree.json` 在 tempfile 现场创建,不提交实体树)
- `authorized_root_scans_real_entries` — 真 tempdir 两层文件,scan 返回集合与磁盘相对路径一致且非空;禁手工构造 ScanEntry 冒充
- `scan_and_preview_leave_the_tree_byte_for_byte_unchanged` — 递归快照 (相对路径, 类型, 字节数, 内容 SHA-256),setup 全部写完后取 before,scan+plan 各跑 3 遍,after 相等;**不断言 atime/mtime**(SOTA F-01:假警报源;fixtures README 提了 mtime,以本条为准);反向:预览非空且引用到快照里真实存在的文件
- `preview_is_derived_from_the_real_scan` — 每条 PlanEntry 可追溯到 ScanEntry
- `the_same_tree_scanned_twice_yields_the_same_plan_hash` — 排序稳定性,防误拒
- `changing_a_real_file_changes_the_plan_hash` — 防常量计划
- `a_preview_can_never_claim_a_disk_write` — `written_to_disk()` 恒 false + 构造入口不收该值

`crates/soul-fileplan/tests/unauthorized_is_refused.rs`(拒绝矩阵,逐条,不抽样)
- `empty_roots_refuse_everything`
- `a_path_outside_every_root_is_refused` — 直接绝对路径 B
- `dotdot_traversal_out_of_the_root_is_refused` — `A/../B`
- `a_symlink_escaping_the_root_is_refused` — A/link→B 为扫描目标整体拒;扫 A 时 B/secret 的内容与元数据零出现、skipped 计数 +1
- `a_parent_directory_of_the_root_is_refused`
- `a_sibling_sharing_the_root_prefix_is_refused` — 授权 `/data/a` 拒 `/data/ab`
- `a_case_or_nfd_variant_outside_the_root_is_refused`
- `refusal_names_no_requested_path` — 错误消息含授权根、不复读 B
- `the_refused_tree_is_untouched` — 对 B 也做前后快照
- `an_authorized_request_still_passes` — 对照,防"全拒"假绿

`crates/soul-fileplan/tests/no_write_api.rs`
- `production_source_has_no_write_surface` — §4 词表
- `the_scanner_saw_the_real_modules` — 锚点断言
- `the_scanner_recognises_a_synthetic_write_call` — 对照必红
- `no_execute_or_apply_entry_point_exists`
- `no_token_machinery_appears_in_the_source` — 无 `TokenIssuer`/`FileWrite` 字面量

`crates/soul-fileplan/tests/filename_injection.rs`(语料 `fixtures/injection/filenames.txt`,经 `read_lines` 取、滤 `#` 注释;只取 portable 可创建子集真建文件,其余做纯 `UntrustedText` 单测并如实注明)
- `portable_injection_filenames_are_scanned_as_data` — 照常列出,不多不少,不产生额外计划条目
- `filename_urls_never_receive_a_connection` — `urls_in(UntrustedText)` 全部过 `NetGuard::closed()` 拒,decoy 计数 0
- `the_fixture_and_real_scan_controls_are_nonempty` — 语料行数与真建成功数都有下限,防过滤后空过

`crates/soulcore/tests/fileplan_commands.rs`(真库端到端,`open_test_store` + tempdir)
- `a_scan_through_the_session_leaves_a_counted_audit_entry` — `FilePlan / Allowed`,只有 uuid 与计数
- `a_refused_path_is_audited_as_path_not_authorized` — `FilePlan / Denied / PATH_NOT_AUTHORIZED`
- `external_content_cannot_request_a_scan_or_plan` — `ExternalContent` × `ScanDirectory`/`PlanFiles` → `ExternalContentNotAuthority`
- `an_edited_plan_fails_the_approved_hash` — 换 target、交换 entry 顺序 → `PlanHashMismatch`;原计划带对 hash 通过(对照)
- `a_file_write_token_buys_no_execution` — `issue_token(FileWrite, …)` → `WriteNotImplemented`;全流程 `issued_count() == 0`
- `the_audit_chain_carries_no_file_name` — 链验证通过后整链序列化,以全部扫描到的文件名(可能含人名)为语料过 `LeakageChecker`

## 6. 必用现有 API 与完工门禁

- API:`PolicySession`/`check_action`、`ActionRequest`/`RequestOrigin`/`ActionKind::{ScanDirectory,PlanFiles}`/`PlanHash`;`ReasonCode::PathNotAuthorized`、`AuditAction::FilePlan`、`append_or_store_error`;`UntrustedText`/`urls_in`;`NetGuard::closed()`(仅测试);`walkdir`;dev:`open_test_store`、`LeakageChecker`、`soul_testkit::fixtures::read_lines`。
- 全绿后交付:`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace --all-targets`、`cargo run -p xtask -- e0-audit` / `denylist-audit` / `schema-freeze --check`、`cargo deny check`;结果写 `.agent_workspace/dev-sota/reports/ST-02.md`。
- schema 与 `schemas.lock` 零改动(`file.plan` 已冻结在案,reason_code 只有模式约束)。
