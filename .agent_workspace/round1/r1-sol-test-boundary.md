MODEL: gpt-5.6-sol-xhigh-fast

# Round 1：测试与未覆盖边界

远程基线：`origin/cursor/soul-goal1-7b1c`，提交 `862e8589b3cd4a95ab038a6073e6bf41978a03dd`（`STATUS: record WP09's first段, the shell`）。以下盘点只来自 `git ls-tree` / `git show` 与 `gh` 的只读输出；没有 checkout Goal 1。

## 测试资产清单（计数）

按题目给定的三种路径模式，共 **58 个文件**：

| 范围 | 文件数 | 说明 |
|---|---:|---|
| `crates/*/tests/**` | 50 | 47 个顶层 Rust integration-test target，加 3 个 `tests/common/mod.rs` 辅助模块 |
| `apps/desktop/src/**/*.test.*` | 4 | Vitest/Testing Library |
| `apps/desktop/src-tauri/tests/**` | 4 | Tauri/Rust integration tests |
| 合计 | **58** | 若只数可独立执行的测试 target/file，则是 55；本报告的主计数按路径文件为 58 |

### Rust crates：50

分 crate：`soul-collect` 5、`soul-egress` 1、`soul-graph` 1、`soul-import` 5、`soul-memory` 3、`soul-policy` 9、`soul-profile` 3、`soul-schema` 2、`soul-store-api` 3、`soul-store` 6、`soul-testkit` 4、`soulcore` 7、`xtask` 1。

```text
crates/soul-collect/tests/collection_lifecycle.rs
crates/soul-collect/tests/common/mod.rs
crates/soul-collect/tests/consent_gate.rs
crates/soul-collect/tests/the_tests_do_not_fake_collection.rs
crates/soul-collect/tests/window_titles_are_not_collected.rs
crates/soul-egress/tests/e1_origin.rs
crates/soul-graph/tests/ego_graph.rs
crates/soul-import/tests/import_to_graph.rs
crates/soul-import/tests/injection_is_data.rs
crates/soul-import/tests/questionnaire.rs
crates/soul-import/tests/soul_import_v1.rs
crates/soul-import/tests/telegram.rs
crates/soul-memory/tests/common/mod.rs
crates/soul-memory/tests/crud_roundtrip.rs
crates/soul-memory/tests/forget_reopen.rs
crates/soul-policy/tests/audit_chain.rs
crates/soul-policy/tests/audit_crash.rs
crates/soul-policy/tests/consent.rs
crates/soul-policy/tests/hitl.rs
crates/soul-policy/tests/injection.rs
crates/soul-policy/tests/net_guard.rs
crates/soul-policy/tests/non_clinical.rs
crates/soul-policy/tests/redactor_exemption.rs
crates/soul-policy/tests/redactor_leakage.rs
crates/soul-profile/tests/axes_and_evidence.rs
crates/soul-profile/tests/correction_lock.rs
crates/soul-profile/tests/questionnaire_intake.rs
crates/soul-schema/tests/roundtrip.rs
crates/soul-schema/tests/schema_wiring.rs
crates/soul-store-api/tests/fake_conformance.rs
crates/soul-store-api/tests/field_aead.rs
crates/soul-store-api/tests/sqlcipher_smoke.rs
crates/soul-store/tests/common/mod.rs
crates/soul-store/tests/conformance_real.rs
crates/soul-store/tests/crash_recovery.rs
crates/soul-store/tests/forget.rs
crates/soul-store/tests/no_plaintext_at_rest.rs
crates/soul-store/tests/research_preview.rs
crates/soul-testkit/tests/crash_harness_demo.rs
crates/soul-testkit/tests/fixture_corpus.rs
crates/soul-testkit/tests/leakage_checker.rs
crates/soul-testkit/tests/mock_llm_server.rs
crates/soulcore/tests/config_defaults.rs
crates/soulcore/tests/endpoint_is_user_supplied.rs
crates/soulcore/tests/import_and_graph_commands.rs
crates/soulcore/tests/policy_commands.rs
crates/soulcore/tests/profile_memory_commands.rs
crates/soulcore/tests/shell_commands.rs
crates/soulcore/tests/store_commands.rs
crates/xtask/tests/self_test.rs
```

### Desktop frontend：4

```text
apps/desktop/src/App.test.tsx
apps/desktop/src/components/CloudToggle.test.tsx
apps/desktop/src/contract.test.ts
apps/desktop/src/routes/Wizard.test.tsx
```

### Tauri shell：4

```text
apps/desktop/src-tauri/tests/command_surface.rs
apps/desktop/src-tauri/tests/ipc_roundtrip.rs
apps/desktop/src-tauri/tests/no_egress_path.rs
apps/desktop/src-tauri/tests/shell_is_local_only.rs
```

### Fixture 种类

题目点名的四类目录都存在，共 13 个文件：

- `fixtures/import/`：7 个。问卷回答 1；`soul-import-v1` 合法、缺字段、三联系人、注入行 4；Telegram 正常与缺字段 2。
- `fixtures/profile/`：4 个。默认五轴档案、完整问卷、部分问卷、应拒问卷。
- `fixtures/memory/`：1 个。基础记忆 CRUD/遗忘语料。
- `fixtures/denylist/`：1 个。诊断词词表。

远程依据：`origin/cursor/soul-goal1-7b1c:fixtures/{import,profile,memory,denylist}/**`；corpus 自检见 `crates/soul-testkit/tests/fixture_corpus.rs`。

## CI 能证明 vs 不能证明

### Workflow 实际覆盖

远程路径：`origin/cursor/soul-goal1-7b1c:.github/workflows/ci.yml`、`origin/cursor/soul-goal1-7b1c:justfile`。

没有 `strategy.matrix`；是三个独立 job：

| Job | Runner | 实际执行 | 边界 |
|---|---|---|---|
| `lint (fmt, clippy, cargo-deny)` | Ubuntu | `cargo fmt`、workspace clippy、`cargo deny check` | 不碰独立的 `src-tauri` workspace |
| `test (ubuntu, headless)` | Ubuntu | `just ci`：lint、schema freeze、E0、denylist、fixture、workspace Rust tests、前端 `ui-lint`、Vitest `ui-test` | 不编译 Tauri shell，不启动 GUI |
| `test (windows-latest)` | Windows | 根 workspace `cargo test --workspace --all-targets`；随后 `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets` | 后一条等价于 `just desktop-test`，但 workflow 没有按名字调用该 recipe；不跑前端 Vitest |

结论：

- **不跑 `tauri build`**，也不产 MSI/NSIS。
- Windows **跑了 `desktop-test` 的等价 cargo 命令**，覆盖 Tauri shell 的编译及 4 个 Rust integration test；Windows 不跑 `pnpm ... test`，前端 4 个测试文件由 Linux `just ci` 跑。
- `apps/desktop/src-tauri` 是独立 Cargo workspace，因此根 workspace 的 lint/test 不会自动覆盖它。

### 当前 CI 结论

查询时点：2026-08-24 13:39 UTC。`gh pr checks 2` 与 `gh run view 32732849726` 对当前 HEAD `862e858` 的结论：

| Check | 当前状态 | gh 证据 |
|---|---|---|
| Linux lint | **pass**，55s | [job 97448789356](https://github.com/Xhhemoing/Soul/actions/runs/32732849726/job/97448789356) |
| Linux headless test | **pass**，3m54s | [job 97448789157](https://github.com/Xhhemoing/Soul/actions/runs/32732849726/job/97448789157) |
| Windows | **pending / in progress** | [job 97448789398](https://github.com/Xhhemoing/Soul/actions/runs/32732849726/job/97448789398) |

Windows 内部进度比 `gh pr checks` 更细：根 workspace 的 `cargo test` 已成功；`cargo test (desktop shell)` 正在执行，所以当前 HEAD 还不能写成 Windows 全绿。此前完整成功的 [run 32728373161](https://github.com/Xhhemoing/Soul/actions/runs/32728373161) 对应 `01de568`，早于 WP07/WP09 大批新增文件，不能替当前 shell 背书。

### CI 已能证明

- workspace Rust 核心、schema/denylist/E0/依赖审计以及前端无头测试在当前 HEAD 的 Ubuntu job 通过。
- 当前 HEAD 的 Windows 根 workspace tests 已通过，含 SQLCipher smoke、导入/档案/记忆/图谱/策略/采集核心测试。
- Tauri 测试在源码层检查 `asInvoker` 清单及其嵌入脚本、`soul` 二进制名、禁 updater、WebView 本地 CSP、current-user NSIS 配置、`webviewInstallMode: skip`、最小 capability 和无 HTTP-client 依赖路径。
- IPC tests 用 Tauri mock runtime 验证三个现有命令、参数命名、ACL/伪造 origin 拒绝；前端测试验证默认全关、云开关仍“尚未启用”且常见 JS 出网 API 未被调用。

### CI 仍不能证明

`origin/cursor/soul-goal1-7b1c:docs/STATUS.md` 的 WP09 Windows 手动缺口 **七项全部仍成立**。静态断言或编译成功只是半证据，未闭合以下运行时边界：

1. **托盘**：Windows 通知区域中图标、tooltip、“打开 Soul/退出 Soul”菜单及关窗入托盘只能真机看；runner 无可观察通知区域。
2. **UAC**：清单是 `asInvoker` 且被 build script 引用，但双击是否无盾牌、无 UAC 提示仍需真机。
3. **`soul.exe` 进程名**：测试钉住 target/产物 stem；任务管理器实际显示仍未观察。
4. **打包/安装**：workflow 没有 `tauri build`，所以 MSI/NSIS 是否成功、安装器是否真的 per-user 且不下载未被执行证明。
5. **WebView2**：配置选择 `skip`，没有在干净 Windows 11 上证明现有 Evergreen runtime 能启动应用，也没有覆盖缺运行时的错误体验。
6. **中文与 DPI**：没有真实 WebView 的字体、150% 缩放、长文截断/滚动/焦点测试。
7. **云开关无系统流量**：依赖图与 JS mock 证明“看不到已知代码路径”，不能替代资源监视器/ETW/防火墙层观察 `soul.exe` 无非回环连接。

补充：`docs/GOAL1_PLAN.md` 仍写“Linux CI 不能自证的仅 AC-01 托盘目视”，这一句已被后来的 WP09 现实扩大；应以 `STATUS.md` 的七项清单为准。

## 空白矩阵（功能 × 测试层）

“核心已有构件测试”不等于“目标功能已测试”。下表按目标能力本身判定：

| 空白 | 核心单元/集成 | IPC / UI 自动化 | Windows / 手动 | 判定 |
|---|---|---|---|---|
| 起草（WP10） | policy、redactor、E1 origin、mock LLM 是已测构件；树中没有 `soul-draft`，无完整 AC-11/12/13/17/25 起草流水线 | `App.test.tsx` 只断言空路由无输入框/发送按钮 | 无 | **无测试（功能）**；现有只是“不得伪装已实现”的负面锁 |
| 文件计划（WP11） | HITL/plan-hash/未知动作拒绝是构件；树中没有 `soul-fileplan`，无 AC-18 授权根扫描与磁盘不变测试 | 只断言空路由没有执行/移动/删除按钮 | 无 | **无测试（功能）** |
| 真 store 接线 | `soul-store` 的加密、契约、崩溃、遗忘、预览测试较强 | Tauri `configure()` 只 `manage(SessionConfig::default())`；现有 IPC 只有配置/向导/云开关，没有 DB | 无启动重开验证 | **有核心测试无 UI**；“壳接线本身”是无测试 |
| DPAPI | SQLCipher/字段 AEAD 已测；`DpapiKeyProvider` 实际两个入口仍返回 `Unsupported`，单测也只证明“不伪造 key” | 无 | 无真实 DPAPI roundtrip | **无测试（实际能力）**；骨架拒绝测试不能算 DPAPI 保护测试 |
| 导入事务 | 解析、正常 commit、落加密库已测 | 无导入 UI | 无崩溃/重启手动门 | **无测试（事务语义）**；`commit` 逐项写，失败可留半份 |
| 导入去重 | 有联系人 digest 匹配测试；重复导入只断言联系人仍为 3 | 无 UI 重导/警告测试 | 无 | **有核心测试无 UI（仅联系人）**；事件/证据去重未实现且未测试，源码明确“同一文件两次写两遍事件” |
| 双问卷 | `soul-import` 八题用真 store 测事件/证据；`soul-profile` 七题用 FakeStore 测非空档案；两套题号不同且互不引用 | 向导 UI 目前只确认默认关闭，不展示任一业务问卷 | 无 | **有核心测试无 UI**；没有共享题库、`UserStatedSink -> soul-profile` adapter、只问一次或端到端非空档案测试 |

关键远程证据：

- 空路由：`apps/desktop/src/App.test.tsx`、`apps/desktop/src/router.tsx`。
- store 未接线：`apps/desktop/src-tauri/src/lib.rs`、`apps/desktop/src-tauri/src/commands.rs`；后者只持有内存 `SessionConfig(pub Config)`。
- DPAPI 骨架：`crates/soul-store/src/keys.rs`；安全文档也明确“缺口，不是已实现但没测”：`docs/SECURITY.md`。
- 导入非事务及事件不去重：`crates/soul-import/src/commit.rs`、`docs/STATUS.md` 的“WP06 的取舍与遗留”第 6/7 条；联系人去重断言见 `crates/soul-import/tests/soul_import_v1.rs::re_importing_matches_the_contacts_that_are_already_there`。
- 双问卷：`crates/soul-import/src/questionnaire.rs` / `tests/questionnaire.rs` 与 `crates/soul-profile/src/questionnaire.rs` / `tests/questionnaire_intake.rs`；`docs/STATUS.md` 的“WP06 与 WP03 的接缝”已明确会让用户被问两遍。

## 对 Goal 1 剩余 WP 的测试建议（不写测试代码）

### WP10：起草与人事摘要

1. 先建一条真正的核心端到端测试：粘贴 `UntrustedText` → 读取用户已锁定语气 → redactor → 精确 E1 mock origin → 返回 draft；断言绝无“发送”副作用。
2. 同一语料覆盖默认第三人占位、一次性原文豁免只生效一次、跨 origin redirect 拒绝、无 key/端点时确定性模板降级且零非回环连接。
3. 用户纠正语气后下一次 draft 必须立即改变；每条人事摘要必须引用可解 evidence 且过非诊断词检查。
4. 增加真实 Tauri IPC 与 UI journey，而不是只测 policy 构件：输入、预览、错误/降级、单次豁免确认；DOM 与 command surface 都不得出现发送动作。
5. 把粘贴注入 fixture 接到完整 draft 流水线，验证不产生工具计划、不访问语料中的 URL。

### WP11：只读文件计划

1. 用临时目录建立授权 A、未授权 B 和哨兵哈希；扫描/计划前后递归比较名称、内容、mtime，证明 A 只读，B 100% 拒绝。
2. Windows 专项覆盖 junction/reparse point、符号链接逃逸、盘符大小写、UNC/长路径、隐藏/系统文件及扫描与预览间 TOCTOU；canonicalization 必须在授权边界内。
3. 文件名注入必须保持数据身份，不能变成 action；计划 hash 改动、未知动作和令牌重放继续拒绝。
4. v0.1 command/API 层只暴露 scan/preview；用源码/command-surface 断言没有 move/rename/delete/write，也没有 UI 执行按钮。

### WP09 功能视图与跨 WP 收口

1. 在 Tauri setup 中只打开一次真实 `SqlCipherStore` 并 `manage` 共享句柄；测试同一进程所有命令使用同一实例、重启可读回、并发采集与 UI 命令不会各开 WAL。
2. Linux/mock 测试注入 `TestKeyProvider`；Windows/release 路径必须拒绝测试 key，并改用真实 DPAPI。IPC 测试至少走一次真实 store 的写入、关闭、重开。
3. 为导入、档案、图谱、记忆/遗忘、研究预览、审计增加 UI journey；UI 测试必须核对核心返回值与错误态，不能用前端假数据替代。
4. 合并两套问卷后，加一条“无文件 → 只问一次 → 真 store 留事件/证据 → 真 profile 非空且每轴可解引用”的跨 crate/IPC/UI 验收。
5. 导入原子性应在存储边界增加事务能力后做 failpoint：联系人、seal、event、evidence、audit 各阶段失败/崩溃，重开后只能全有或全无。
6. 先冻结去重语义再测。若承诺幂等，需覆盖同文件重导零新增、重叠导出只增尾部；若 v0.1 明确允许重复，则测试应钉住重复计数并要求 UI 在确认前明确提示，不能只靠源码注释。

### WP13：Windows 发布门

1. 实现 DPAPI 后在 Windows runner 做 current-user protect/unprotect、进程重启重开 DB、错误用户/损坏 blob 拒绝、磁盘不出现裸 DEK/KEK/CK；禁止 release 路径回退 `TestKeyProvider`。
2. 增加真正的 `tauri build` 与产物 smoke：检查 `soul.exe`、嵌入 manifest、MSI/NSIS current-user 安装/卸载、无 updater/WebView 下载副作用。打包失败不能由普通 `cargo test` 代替。
3. 保留一张作者签字的 Windows 11 x64 真机 release checklist：托盘、UAC、任务管理器进程名、干净机 WebView2、中文 100%/150%/200% DPI、云开关期间系统层零非回环流量。
4. CI 状态必须按提交闭合：Linux、Windows root、Windows desktop shell、package smoke 都要对应同一个 HEAD；不能拿 WP09 之前的全绿 run 替当前提交。

## 结论

当前测试资产对核心数据/策略层覆盖较强，但 Goal 1 的风险已从“算法是否有测试”转移到“真实 Windows 壳是否把这些核心正确串起来”。最高优先级测试债是：真实 store + DPAPI 接线、导入原子性、双问卷合并、WP10/WP11 的功能级端到端，以及 `tauri build`/真机发布门。当前 CI 只能确认 Linux 两 job 通过、Windows 根 workspace 通过；Windows desktop shell 仍在跑。
