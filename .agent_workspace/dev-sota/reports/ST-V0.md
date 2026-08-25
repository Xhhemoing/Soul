MODEL: claude-opus-5-thinking-high-fast

# ST-V0 soulcore:视图 DTO + 宿主函数 —— 完成报告

分支 `agent/dev-sota`,已 commit 并 push(commit `92eb9db`)。

## 1. 结论

`TASK_SPLIT_VIEWS` 第 2.1 节的四件全部落地,零偏离、零越界:`StoreSlot`、`Session` 三个
访问器 + `ViewRefused`/`wall_clock`、`DraftView`/`draft_view`、`FilePlanView`/`fileplan_view`。
新增 10 条烟测(`draft_view.rs` 5 条、`fileplan_view.rs` 5 条)+ `shell_commands.rs` 3 条
访问器测试。完成定义 1–9 逐条可断言,见第 4 节。

**没有需要父代理裁决的偏离。** 一处主动补充与一处已知边界写在第 5 节。

## 2. 改动的文件(全部在 ST-V0 的 glob 内)

| 文件 | 改动 |
|---|---|
| `crates/soulcore/src/commands/store.rs` | +51:`StoreSlot`(`install` / `lock`)与模块文档一段 |
| `crates/soulcore/src/commands/shell.rs` | +148:`ViewRefused` / `ViewRefusedReason`、`NO_STORE_FOR_VIEW_EXPLANATION`、`ENDPOINT_UNUSABLE_EXPLANATION`、`wall_clock()`、`SessionState` 手写 `Default`、`Session::config()` / `Session::draft_profile_id()` |
| `crates/soulcore/src/commands/draft.rs` | +151:`VIEW_MODEL`、`DRAFT_NEVER_SENT_EXPLANATION`、`TEMPLATE_ROUTE_LABEL` / `E1_ROUTE_LABEL`、`EMPTY_PASTE_EXPLANATION`、`DraftView` + `DraftView::of`、`draft_view` |
| `crates/soulcore/src/commands/fileplan.rs` | +190:`PLAN_PREVIEW_ONLY_EXPLANATION`、三个 `*_ACTION_LABEL`、`FilePlanEntryView`、`FilePlanView` + `of`、`fileplan_view`、私有 `encode`/`action_label`/`as_refused` |
| `crates/soulcore/tests/draft_view.rs` | 新建,254 行,5 条 |
| `crates/soulcore/tests/fileplan_view.rs` | 新建,345 行,5 条 |
| `crates/soulcore/tests/shell_commands.rs` | +73:3 条访问器测试(`config()` / `draft_profile_id()` / `wall_clock()`) |

`git diff --stat 215a310..92eb9db` 只有这 7 个文件(本报告自身在其后单独一次 commit)。纯 crate(`soul-draft` / `soul-fileplan` /
`soul-policy` / `soul-store`)、`commands/mod.rs`、两份 `Cargo.toml`、`Cargo.lock`、
`docs/schemas` 与 `schemas.lock`、`apps/desktop` 一个字节未动。

## 3. 关键设计决定(与第 2.1 节的对应)

**`StoreSlot` 用 `OnceLock<Arc<Mutex<SqlCipherStore>>>`,不是 `Mutex<Option<…>>`。** 两个理由:
「只填一次」变成类型自己的规则,`install` 第二次返回 `false` 是 `OnceLock::set` 的语义而不是
一段可以被改写的判断;更实在的是生命周期——`lock()` 要返回 `MutexGuard<'_, SqlCipherStore>`,
借的是槽里那个 `Mutex`,而藏在外层锁后面的值做不到这一点(外层 guard 一 drop,借用就悬了)。
中毒锁按 `Session::lock` 先例 `into_inner` 恢复。`install` 标了 `#[must_use]`:第二次 install
是 bug,壳侧不该静默丢弃返回值。

**`SessionState` 手写 `Default`,`draft_profile_id` 铸 `Uuid::now_v7()`。** 会话内稳定
(`complete_wizard` 只换 `config`,不换身份),会话间互不相同。不走 `profile::intake`——
空问卷是契约雷区;`read_voice` 对任意 id 都给中性 `VoiceProfile`,起草需要的只有这个。

**`ViewRefused` 照抄 `RootRefused` 的形状,但只有 `Serialize`。** 序列化后恰好三个键
`{reason, code, message}`,`reason` 是 snake_case,`#[error("{message}")]` 让 Display 与过
IPC 的那串是同一个字符串(两个测试各自断言了这一点)。没有 `Deserialize`:能被解析回来的
值,它的承诺就来自那份文档而不是这个 build。`code` 来自 `ReasonCode::as_str()`,没有 gate
参与决策时是 `None`。

**`wall_clock()` 一次读取给两个单位。** 毫秒给 token/action 时钟,整秒给审计条目;两者同源,
否则记录和决策会描述两个不同的时刻(`shell_commands.rs` 里有断言:`now_ms / 1000 == at`)。
纪元之前的时钟取零而不是 panic。

**`never_sent` / `written_to_disk` 是构造级的。** 私有字段 + 唯一构造 + 无 setter + 无
`Deserialize`,照抄 `FilePlanPreview::written_to_disk` 的模式。`DraftView::of` 里 `never_sent`
写死 `true`,`FilePlanView::of` 里 `written_to_disk` 写死 `false`,全仓各只有一处赋值。

**拒绝语不回显被拒路径。** `fileplan_view` 的所有拒绝都走 `as_refused`,即
`FilePlanRefusal` 自己的 `Display`;`FilePlanError::PathNotAuthorized` 的错误契约是「只列已
授权 roots」,视图层一个字都不拼。测试正面断言两侧:message **不含** B 的路径,且**含**
A 的规范化路径(否则「不回显」可以靠返回空串蒙混过关)。

**`session_for` 的 `OriginError` 不用 Display。** 这是本包唯一一处**不**用底层错误原文的地方,
理由是 `OriginError` 的五个变体每一个都把 URL 原样回显(`crates/soul-policy/src/net_guard.rs:33`),
而红线写明 `llm_endpoint` 字符串不进任何 DTO/错误文本。改用常量
`ENDPOINT_UNUSABLE_EXPLANATION`(住在 `shell.rs`,两个视图共用)。该分支在本轮不可达:壳里
没有任何写 `llm_endpoint` 的入口,`Session` 的三个写方法也都到不了它。

**宿主函数零新增审计。** `draft_view` 只调 `draft_reply`,`fileplan_view` 只调
`scan_directory` + `plan_files`,审计全部由它们内部落链。空粘贴在**取到 store 之后、任何
落链动作之前**返回,所以链上不多一条(有测试)。

## 4. 完成定义逐条对照

| # | 定义 | 证据 |
|---|---|---|
| 1 | 空槽 `lock()` 为 None;`install` 后为 Some;第二次被拒 | `draft_view.rs::a_slot_is_empty_until_it_is_filled_and_will_not_be_filled_twice`。额外断言:第二次 install 被拒后,槽里**仍是第一个句柄**(经原 `Arc` 读链验证) |
| 2 | `draft_profile_id()` 同会话相等、跨会话不等;`config()` 看得到 root | `shell_commands.rs::a_session_drafts_under_one_profile_and_the_next_one_under_another`(含「过一次 wizard 身份不变」)、`::the_session_hands_the_views_the_directory_it_just_authorised` |
| 3 | 空槽下两个视图都是 `NoStoreOpened` + 逐字常量 | `draft_view.rs::a_draft_without_a_store_is_refused_in_the_core_s_own_words`、`fileplan_view.rs::a_plan_without_a_store_is_refused_in_the_core_s_own_words`。两条都断言了 JSON 三键形状 |
| 4 | 真库 + 默认 Session → `route == "template"`、`never_sent`、`text` 非空、`placeheld_turns >= 1`、`notice` 逐字、链上 `DraftCreate / Allowed` | `draft_view.rs::a_pasted_message_becomes_a_local_template_draft`。全程零 MockLlm。额外断言:草稿里出现 `THIRD_PARTY_PLACEHOLDER` 且**不含**粘贴原文 |
| 5 | 全空/纯空白 → `EmptyPaste`,链上无新增 | `draft_view.rs::an_empty_paste_is_refused_before_anything_is_recorded`,三种空形态(空 Vec / 空串 / 纯空白),前后都断言链为空 |
| 6 | 授权目录 → 成功且 counts 对得上;目录 B → `PATH_NOT_AUTHORIZED` 且不回显 B;空 roots 同样拒绝 | `fileplan_view.rs` 三条:`an_authorised_directory_produces_a_preview_that_touches_nothing`(三个 count 相加等于 entries 长度,每个 `action_label` 与常量比对)、`a_directory_outside_the_authorised_roots_is_refused_without_being_named`(B、B 下的文件、`A/../B` 三种写法,链上三条 Denied)、`a_session_with_no_authorised_root_scans_nothing_at_all` |
| 7 | 三个类型只有 `Serialize` | 源码事实(`draft.rs:*`、`fileplan.rs:*`、`shell.rs:*` 的 derive 行);源码级扫描器按拆单归 V2/V3。字段集本包已用两条测试钉死(`a_draft_view_carries_these_fields_and_no_others`、`a_file_plan_view_carries_these_fields_and_no_others`),V1 的 TS 类型可以直接照抄 |
| 8 | 纯 crate / `commands/mod.rs` / 所有 `Cargo.toml` / schema 零改动 | 第 2 节的 `git diff --stat` |
| 9 | 根树全绿 | 第 6 节 |

## 5. 主动补充与已知边界

**补充一:`E1_ROUTE_LABEL`。** 第 2.1 节只给了模板路线那一句。`DraftView::of` 收的是任意
`DraftOutcome`,`match` 必须穷尽,所以 E1 也有一句常量。写下来比让 label 变成「今天恰好为真
的常量」好:哪天端点录入 UI 落地,改的是一个 `match` 臂而不是有人临时编一句话。本轮该分支
不可达。

**补充二:`ENDPOINT_UNUSABLE_EXPLANATION` 住在 `shell.rs` 而不是 `draft.rs`。** 两个视图共用,
而 `shell.rs` 已经是 `NO_STORE_FOR_VIEW_EXPLANATION` 与 `ViewRefused` 的家。V1 要用
`rustStringConstant` 比对的两句 notice(`DRAFT_NEVER_SENT_EXPLANATION` 在 `draft.rs`、
`PLAN_PREVIEW_ONLY_EXPLANATION` 在 `fileplan.rs`)位置不变,不受影响。

**已知边界(不可达,如实记下)。** `DraftCommandError::Generation` 一路下去的
`EgressDenied` / `EgressError` 的 Display 可能带 URL。视图路径到不了它:`Session` 的 config
永远 `llm_endpoint: None` ⇒ `session_for` 给 closed session ⇒ 路线恒为 Template ⇒ E1 分支不
执行。端点录入 UI(WP13/P1)落地时,这条要连同 `VIEW_MODEL` 的注释一起复查。

**给 V1 的两点。** 其一,`StoreSlot::install` 有 `#[must_use]`,`install_store` 里要处理返回值
(`assert!` 或显式忽略并写明)。其二,`DraftView`/`FilePlanView` 的字段顺序即 TS 类型顺序,
两条 `*_carries_these_fields_and_no_others` 测试里是**按字母序**排过的列表(serde_json 默认
`Map` 是 `BTreeMap`),照抄时注意这是排序后的形状而不是声明顺序。

## 6. 绿灯命令与结果

在 `/workspace`(根树),全部 exit 0:

| 命令 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | 通过 |
| `cargo clippy -p soulcore --all-targets -- -D warnings` | 通过 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 通过 |
| `cargo test -p soulcore --test draft_view` | 5 passed |
| `cargo test -p soulcore --test fileplan_view` | 5 passed |
| `cargo test -p soulcore --test shell_commands` | 26 passed(原 23 + 新 3) |
| `cargo test -p soulcore` | 全绿(9 个测试二进制) |
| `cargo test --workspace --all-targets` | 全绿,0 failed |
| `cargo run -p xtask -- e0-audit` | clean,13 crate / 150 文件 |
| `cargo run -p xtask -- denylist-audit` | clean,94 term / 104 文件 |
| `cargo run -p xtask -- schema-freeze --check` | `docs/schemas matches the lock` |
| `cargo test -q -p soul-testkit --test fixture_corpus`(= `just fixtures-verify`) | 9 passed |

**未跑:`cargo deny check`。** 这台机器上没装 `cargo-deny`(`which cargo-deny` 为空)。本包
没动任何 `Cargo.toml`、没加任何依赖、`Cargo.lock` 未变,所以 deny 的三类检查(licence /
advisory / HTTP 客户端禁令)的输入与合入前完全一致;`e0-audit` 这一路的等价检查是绿的。
建议 ST-V4 在装了工具的环境里补跑一次。

**未跑(不归本包):** desktop 树的 `just desktop-check` / `desktop-test` 与
`pnpm ui-lint` / `ui-test`——ST-V0 不碰 `apps/desktop`,soulcore 的改动全是新增面,
既有 IPC 契约未变。

## 7. 移交给 ST-V1 的面

```rust
soulcore::commands::store::StoreSlot            // Default / install(-> bool) / lock()
soulcore::commands::shell::ViewRefused          // { reason, code: Option<String>, message }
soulcore::commands::shell::ViewRefusedReason    // no_store_opened | empty_paste | refused
soulcore::commands::shell::NO_STORE_FOR_VIEW_EXPLANATION
soulcore::commands::draft::{DraftView, draft_view}          // (&StoreSlot, &Session, Vec<String>)
soulcore::commands::draft::DRAFT_NEVER_SENT_EXPLANATION
soulcore::commands::fileplan::{FilePlanView, fileplan_view} // (&StoreSlot, &Session, String)
soulcore::commands::fileplan::PLAN_PREVIEW_ONLY_EXPLANATION
```

两个 wrapper 的一行体形如:

```
soulcore::commands::draft::draft_view(slot.inner(), &session.0, pasted)
soulcore::commands::fileplan::fileplan_view(slot.inner(), &session.0, target)
```

参数名钉死为 `pasted`(`Vec<String>`)与 `target`(`String`)。
