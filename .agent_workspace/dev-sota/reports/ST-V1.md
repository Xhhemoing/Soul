MODEL: Cursor Grok 4.6

# ST-V1 壳接线:IPC + 路由 + 骨架页 —— 完成报告

分支 `agent/dev-sota`。本包只改 TASK_SPLIT_VIEWS 第 3 节列出的 glob。

## 1. 结论

两条新命令 `draft_view` / `fileplan_view` 过了三方一致(COMMANDS ↔ COMMAND_NAMES ↔ `generate_handler!`)。`configure()` 托管空 `StoreSlot`，`install_store` 填槽且使用 `install` 的布尔返回值；mock 运行时槽空，视图命令返回可读的 `no_store_opened`。`#/draft` 与 `#/files` 从 Pending 变成骨架页：生成草稿、扫描并预览，没有发送、没有执行。

**没有需要父代理裁决的偏离。** D32(QQ/微信客户端读取)本包未做、也未加任何采集/RPA 入口。

## 2. 改动的文件(全部在 ST-V1 glob 内)

| 文件 | 改动 |
|---|---|
| `apps/desktop/src-tauri/src/lib.rs` | `configure` 托管 `StoreSlot::default()`；注册两条新命令；`install_store` 填槽，不再 `manage` 句柄 |
| `apps/desktop/src-tauri/src/commands.rs` | 两个一行 wrapper；`COMMAND_NAMES` 七条 |
| `apps/desktop/src-tauri/tests/command_surface.rs` | 新增「命令名无发送/执行词汇」测试(含 `send_draft` 合成反例) |
| `apps/desktop/src-tauri/tests/ipc_roundtrip.rs` | 空槽拒绝、缺参拼写钉死、伪 origin 加上两条新命令 |
| `apps/desktop/src/core.ts` | `COMMANDS` 七条；`DraftView`/`FilePlanView`/`ViewRefused`；`never_sent: true` 与 `written_to_disk: false` 为字面量类型 |
| `apps/desktop/src/test/fakeCore.ts` | 两个 case：定值 + 空粘贴 / 未授权 target；拒绝形状 `{reason, code, message}` |
| `apps/desktop/src/router.tsx` | draft/files `ownedBy: null` |
| `apps/desktop/src/App.tsx` | 渲染 `<Draft />` / `<Files />` |
| `apps/desktop/src/App.test.tsx` | 起草有 textbox 与「生成草稿」；无 `/发送\|send\|submit\|deliver/`；文件页保留无执行按钮循环 |
| `apps/desktop/src/contract.test.ts` | 两句 notice 与 Rust 常量逐字比对 |
| `apps/desktop/src/routes/Draft.tsx` | 骨架(移交 V2) |
| `apps/desktop/src/routes/Files.tsx` | 骨架(移交 V3) |

`tests/store_session.rs` **一个字节未改**；既有六条继续绿(`open_store` 仍只在 `lib.rs` 出现一次；`commands.rs` 无 `Arc`/`Mutex`/`SqlCipherStore` 字样)。

## 3. 关键决定

**槽而不是第二份 `manage`。** mock 运行时只跑 `configure()`，空槽让 `draft_view`/`fileplan_view` 返回 `ViewRefused`，而不是 `State<Arc<Mutex<…>>>` 缺失时的 panic。`install` 标了 `#[must_use]`，第二次填槽走 `Err`。

**wrapper 一行、体内无 `}`。** `the_command_layer_stays_thin` 数的是第一个 `{` 到第一个 `}` 之间的非注释行。`&store` 靠 `State` 的 Deref 强制转换成 `&StoreSlot`；`&*store` 会被 clippy `explicit_auto_deref` 拒绝。

**按钮叫「生成草稿」。** `queryByRole("button", { name: /发送/ })` 会命中「不发送」。整页按钮可达名不含「发送」二字。文件页建议表是纯文本行，行内零 button；「扫描并预览」不匹配 `/执行|应用|移动|重命名|删除/`。

**假核心不比真核心仁慈。** 空粘贴用 `EMPTY_PASTE_EXPLANATION`；未授权 target 用 `FilePlanError::PathNotAuthorized` 空 roots 的 Display，**不含**被拒路径。

## 4. 完成定义对照

| # | 定义 | 证据 |
|---|---|---|
| 1 | configure 托管空槽；install 填槽；store_session 不改一字继续绿 | `lib.rs`；`store_session.rs` 6/6 |
| 2 | wrapper 各一行；COMMAND_NAMES 七条；禁发送/执行名 | `command_surface.rs` 5/5 |
| 3 | core.ts 七条命令 + 字面量类型 + notice 常量比对 | `contract.test.ts` |
| 4 | fakeCore 两 case + default 抛错保留 | `fakeCore.ts` |
| 5 | 骨架页可用 | `Draft.tsx` / `Files.tsx` |
| 6 | App.test 更新，旧承诺保住 | 起草/文件两条；向导/设置/未知路由/核心不回应未动 |
| 7 | ipc 空槽拒绝 + 缺参 + 伪 origin | `ipc_roundtrip.rs` 13/13 |
| 8 | forbidNetwork 在新页路径上空 | `App.test.tsx` 两条都包了 |
| 9 | desktop + ui 全绿 | 见下 |

## 5. 命令结果

- `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings` 通过(不用 `--all-features`：`custom-protocol` 需要 `dist/`)
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets` 全绿(command_surface 5、ipc 13、store_session 6、其余原样)
- `pnpm --filter @soul/desktop lint` 通过
- `pnpm --filter @soul/desktop test` 5 files / 31 tests 通过

根树未改 soulcore，本包不重跑根 `just ci`。

## 6. 移交

- `Draft.tsx` → ST-V2
- `Files.tsx` → ST-V3
- 共享文件(`core.ts`、`commands.rs`、`lib.rs`、`fakeCore.ts`、`App.tsx`、`App.test.tsx`、`router.tsx`、`contract.test.ts`)自本包合入后冻结
