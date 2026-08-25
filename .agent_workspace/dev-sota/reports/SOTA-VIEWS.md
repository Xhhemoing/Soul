MODEL: claude-fable-5-thinking-xhigh

# SOTA 复核 — WP09 功能视图：`#/draft` 与 `#/files`

分支 `agent/dev-sota`（HEAD `a3c8f92`）。范围：起草页与文件计划页两个功能视图及其核心侧视图层（ST-V0～ST-V3 已落地）。产品权威：`docs/PRODUCT_LOCK.md`；D32（QQ/微信客户端读取不进本仓库）。只读复核，未改任何被审文件。

## 结论

**verdict: ship**

六条验收线全部成立，且每条都不止一层锁。本机全绿：vitest 38/38（含 Draft.test.tsx 3、Files.test.tsx 4）、`tsc --noEmit` + eslint 干净、`cargo test -p soulcore --test draft_view` 10/10、`--test fileplan_view` 9/9、`apps/desktop/src-tauri` 全部 34 条（command_surface 5、ipc_roundtrip 13、no_egress_path 3、shell_is_local_only 7、store_session 6）通过。

无 must-fix。四条非阻塞观察见文末。

---

## 逐条验收

### 1. 起草永不发送 — 通过

**无发送控件。** `Draft.tsx` 全页恰好两个按钮：生成草稿、清空，无任何链接。测试从两个方向钉死：`Draft.test.tsx:59` 断言按钮全集 `["生成草稿", "清空"]`（多一个控件即红），`:60` 反查 `/发送|send|submit|deliver/i`，`:61` 断言零链接。壳层再钉一遍（`App.test.tsx:48`，经真实路由 `#/draft` 进入）。

**无发送命令。** 命令面只有 7 个名字，`command_surface.rs:95-115` 扫描 `COMMAND_NAMES` 禁 send/submit/deliver/execute/apply/write/move/rename/remove/delete，并带合成反例证明扫描器能红。三处名单（`core.ts` COMMANDS、`commands.rs` COMMAND_NAMES、`lib.rs` generate_handler）由 `command_surface.rs` 与 `contract.test.ts:79-85` 双向对账，两侧都不能单独长出新命令。

**`never_sent` 构造级为真。** 三层：
- 类型层：TS 侧 `core.ts:81` 声明字面量类型 `readonly never_sent: true` —— 一个想传 `false` 的值过不了编译；Rust 侧字段私有、唯一构造 `DraftView::of` 写死 `true`（`draft.rs:474`）、只 `Serialize` 不 `Deserialize`（`draft.rs:446`），值不能从文档里被解析回来。
- 源码层：`draft_view.rs::never_sent_is_true_by_construction` 用 `include_str!` 读回 `draft.rs`，断言 derive 无 Deserialize、无 `pub never_sent`、无 setter、无 `never_sent: false`，并用合成文档证明每条断言都能红。
- 运行层：`a_pasted_message_becomes_a_local_template_draft` 对真加密库断言序列化后 `never_sent == true`、`route == "template"`（视图路径的会话由 `session_for` 从无 endpoint 配置构造，桌面没有写 `llm_endpoint` 的入口，E1 分支不可达 —— `draft.rs:400-411` 把这一点写成了常量而不是习惯）。

**说明句逐字上屏。** 链条闭合：`DRAFT_NEVER_SENT_EXPLANATION` 是 `draft.rs:420` 的常量 → Rust 测试断言视图 `notice` 等于它（`draft_view.rs:154-158`）→ 页面 `{view.notice}` 原样渲染（`Draft.tsx:75`）→ UI 测试用 `.toBe` 精确比对 textContent（`Draft.test.tsx:32`）→ `contract.test.ts:109-114` 用 `rustStringConstant` 把 fakeCore 的副本钉回 Rust 源文件。任何一端改一个字，至少一条测试红。

### 2. 文件计划永不执行 — 通过

**无执行控件。** `Files.tsx` 全页一个按钮：扫描并预览。`Files.test.tsx:98-100` 断言按钮全集，`:95-97` 禁 执行/应用/移动/重命名/删除；建议表逐行断言零按钮、零链接、零输入框（`:66-70`）；空 roots 提示句本身也过禁词（`:28`）。壳层再钉一遍（`App.test.tsx:65-75`）。

**`written_to_disk` 构造级为假。** 与 never_sent 同构的三层：TS 字面量类型 `readonly written_to_disk: false`（`core.ts:110`）；Rust 字段私有、唯一构造写死 `false`（`fileplan.rs:255`）、Serialize-only；源码级自查 `fileplan_view.rs::written_to_disk_is_false_by_construction` 带合成反例。

**且不止声明，有取证。** `a_preview_leaves_the_authorised_tree_untouched` 对整棵授权树做递归快照（相对路径全集 + 字节内容 + mtime），预览前后逐字节相等。`PLAN_PREVIEW_ONLY_EXPLANATION`（`fileplan.rs:177-179`）的逐字链条与起草页同构闭合（`Files.test.tsx:50` 用 `.toBe`；`contract.test.ts:116-121`，`rustStringConstant` 正确还原了 Rust 的反斜杠换行）。动作词（归类/移动/重命名）是核心常量而非前端翻译表，`cargo test` 读得到（`fileplan.rs:181-189`）。

### 3. WebView 无业务逻辑；IPC 包装一行；只有 core.ts 引 @tauri-apps/api — 通过

**边界双锁。** eslint `no-restricted-imports`（`eslint.config.js:22-33`，仅豁免 core.ts 与测试支撑）+ `contract.test.ts:70-77` 不依赖 linter 的文件系统扫描。两页确实只从 `../core` 导入。

**页面不决策。** 值得表扬的一处：`Draft.tsx` 连"粘贴框是否为空"都不判 —— 原样发给核心，渲染核心的拒绝句（`Draft.test.tsx:40-51` 证明空粘贴的拒绝语来自核心而非前端自编）。`Files.tsx` 同样：target 原样过去，roots 从 `authorized_roots` 读回。页面里剩下的只有显示状态（useState 三件套、live 标记），无一处分支产生业务后果。

**包装一行。** `core.ts` 每个包装函数是单条 `return invoke(...)`；Rust 侧七个 `#[tauri::command]` 各一条语句转调 `soulcore::commands::*`，`command_surface.rs::the_command_layer_stays_thin` 钉 ≤1 语句。`draft_view`/`fileplan_view` 的取锁、trim、拒绝映射全部在 soulcore（`draft.rs:499-536`、`fileplan.rs:293-333`），命令层放不下也确实没放。

**StoreSlot 安装正确。** `lib.rs::configure` manage 空槽（mock 运行时因此能以句子拒绝而非 panic），`install_store` 在 setup 里开一次库、`OnceLock` 拒绝二次安装（`store.rs:126-153`），键提供者选择留在 soulcore（`open_store_for_session`）。`store_session.rs` 六条 + `draft_view.rs::a_slot_is_empty_until_it_is_filled_and_will_not_be_filled_twice`（第二个句柄拒绝后，写入走的仍是第一个 Arc）把 WP07 遗留 8 的"一个进程一个句柄"钉死。IPC 往返层补齐了两页面在 mock 运行时的拒绝、参数名契约（`the_view_commands_need_the_arguments_they_are_given` 区分"参数错在 IPC"与"视图拒绝"）与伪造 origin 拒绝。

### 4. UI 测试 forbidNetwork — 通过

`Draft.test.tsx` 3/3、`Files.test.tsx` 4/4，每条测试第一行 `forbidNetwork()`、最后断言 `attempts` 为空；`App.test.tsx` 的 draft/files/settings 三条同样。stub 覆盖 fetch/XHR/WebSocket/EventSource/sendBeacon（`fakeCore.ts:229-265`），经 `vi.stubGlobal` 可复原。

### 5. 拒绝不回显粘贴正文与未授权路径 — 通过

**起草侧。** 可达的拒绝形态全部是定句或不含正文的组合：no-store 与 empty-paste 是常量（`a_view_refusal_does_not_echo_the_paste` 直接断言不含种入的粘贴行）；`refused` 变体在模板路径上可达的错误源查了一遍 —— `HitlDenial` 只有 reason 词、`StoreError` 是后端句、`NonClinicalViolation` 只带禁词本身（`clinical.rs:42-44`）、`DraftError` 各变体只带 UUID —— 无一携带粘贴正文。`ENDPOINT_UNUSABLE_EXPLANATION` 特意用定句替换会复读 URL 的 `OriginError`（`shell.rs:388-396`）。审计链侧由 `a_fixture_paste_leaves_no_third_party_prose_on_the_chain`（LeakageChecker 带反向对照）与注入用例覆盖。

**文件侧。** `FilePlanError::PathNotAuthorized` 从错误类型的 Display 上就不含被拒路径（`soul-fileplan/src/error.rs:20-21`，只列授权根）。拒绝矩阵覆盖面好：外部目录、外部文件、`A/../B` 遍历、父目录（含"父路径是授权根前缀"的 substring 陷阱，`fileplan_view.rs:415-419` 处理正确）、Unix 逃出 symlink（连逃出目标也不回显）、空 roots 不确认路径存在。`RootUnreadable`/`NotADirectory` 确实带路径，但仅对**已通过授权**的路径报出（`error.rs:27-29` 注释明说非存在性预言机），不触本条红线。

### 6. D32：无 QQ/微信/RPA/采集 UI — 通过

对 `apps/desktop/src` 全树扫 QQ/微信/WeChat/RPA/采集/collect：两个被审页面及其测试零命中。两页无导入控件、无采集开关、无任何客户端读取入口。UI 树里唯一出现处是 `Home.tsx:37-38` 的**否定式披露**（"本版不从 QQ / 微信客户端读取……Soul 只吃文件"）—— 这是对 D32 的如实陈述而非功能，且在本次范围之外。

---

## 非阻塞观察（不要求本轮修）

1. **单条粘贴永远是一个 turn。** `Draft.tsx:20` 把整个 textarea 作为一个元素发（`draftView([paste])`），多条消息粘进来计数显示"占位 1 段"。核心支持多元素（`each_nonempty_paste_item_is_one_turn` 已证），但唯一调用方只发单例。在 WebView 里切分粘贴会是业务逻辑，所以现状方向正确；若产品上要按消息计数，正确落点是给核心加切分并让页面提供多输入，不是前端 split。
2. **UI 侧缺一条"拒绝不回显路径"的屏上断言。** `Files.test.tsx:76-87` 的拒绝用例用空 target 触发；没有一条测试输入 `REFUSED_TARGET` 后断言告警文本不含该路径。核心侧已钉死且页面逐字渲染 `message`，链条实质闭合，但屏上多一条断言能把"前端顺手把 target 拼进告警"这类回归也挡住。
3. **`fileplan-row` 的 React key 潜在碰撞。** `Files.tsx:104` 用 `${entry.action}:${entry.source_rel}` 作 key；当前规划器一文件一建议，碰不上。若 v0.1.1 让同一文件产生同动作多条目，会出现重复 key 警告。
4. **`PathNotAuthorized` 的拒绝句是英文。** `fakeCore.ts:49-50` 逐字复制了真核心的句子，保真度是对的；但中文优先的产品里这句英文最终会上屏。改动落点在 `soul-fileplan`（本次范围之外），顺带提及。

## 与 SOTA_BARS 的对齐

本包对应 SOTA_BARS 的 S-01～S-04 与 D-01/F-03/F-05 的视图侧延伸：S-01（一个句柄）由 store_session + StoreSlot 测试覆盖；S-02（键选择在 soulcore）由 `the_shell_does_not_choose_the_key_provider` 覆盖；S-04（wrapper ≤1 语句、三方名单咬合）由 command_surface 五条覆盖且对新命令继续绿。视图层新增的"构造级布尔 + 源码自查 + 逐字文案链"三件套沿用了 bars 里"测试能红"的纪律——每条源码扫描都带合成反例。

## 验证记录

| 套件 | 结果 |
|---|---|
| `pnpm --filter @soul/desktop test` | 7 文件 38/38 通过 |
| `pnpm --filter @soul/desktop lint`（tsc + eslint） | 干净 |
| `cargo test -p soulcore --test draft_view` | 10/10 |
| `cargo test -p soulcore --test fileplan_view` | 9/9 |
| `cargo test`（apps/desktop/src-tauri，全部五个测试文件） | 34/34 |
