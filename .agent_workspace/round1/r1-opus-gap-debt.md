MODEL: claude-opus-5-thinking-high-fast

# Round 1 — 接缝、技术债与剩余 WP 盘点

只读核实。所有路径均为远程分支上的路径，代码面来自 `origin/cursor/soul-goal1-7b1c`（HEAD `862e858`），文档基线来自 `origin/cursor/soul-product-lock-7b1c`（HEAD `a785317`）。

**分支关系已核实**：`origin/cursor/soul-product-lock-7b1c` 是 `origin/cursor/soul-goal1-7b1c` 的严格祖先（`git merge-base --is-ancestor` 为真，merge-base 就是 `a785317`）。两分支的 `docs/PRODUCT_LOCK.md`、`docs/DECISIONS.md`、`docs/FORMAL_WORK_PROMPT.md` 逐字节相同；`docs/STATUS.md` 与 `docs/SECURITY.md` 只在 goal1 上前进。文档 PR 分支没有夹带代码，也不会与 goal1 冲突——这一项**不是**债务。

---

## 一、已确认债务表

严重度：`严重` = 会让用户在真机上撞墙或让门禁失真；`中` = 影响正确性或后续工作量；`低` = 语义或工程整洁问题。
关门判断：`阻塞` = v0.1 不能带着它关门；`可带债` = 可关门但必须在 STATUS 与 UI 上如实写；`可忽略` = 记录即可。

| # | 债务 | 严重度 | 是否阻塞 Goal 1 | 证据路径 |
|---|---|---|---|---|
| D1 | **`DpapiKeyProvider` 仍返回 `Unsupported`**，Windows 上没有任何可用的真 `KeyProvider` | 严重 | **阻塞** | `origin/cursor/soul-goal1-7b1c:crates/soul-store/src/keys.rs`（`unprotect()` 的 `cfg(windows)` 与 `cfg(not(windows))` 两个分支都 `Err(KeyError::Unsupported(..))`；单测 `the_dpapi_provider_constructs_and_refuses_rather_than_inventing_a_key` 把这个行为钉住了）；`docs/SECURITY.md:44` |
| D2 | **桌面壳没有连真 `SqlCipherStore`**，`SessionConfig` 仍是内存 `Config` | 严重 | **阻塞** | `origin/cursor/soul-goal1-7b1c:apps/desktop/src-tauri/src/lib.rs`（`builder.manage(commands::SessionConfig::default())`，`setup` 里只 `manage` 了 `tray::install_or_report`，没有 `open_store`）；`apps/desktop/src-tauri/src/commands.rs`（`pub struct SessionConfig(pub Config)`） |
| D3 | **配置完全没有持久化**，也没有任何写入配置的命令 | 严重 | **阻塞** | `crates/soulcore/src/config.rs`（`Config` 有 `Serialize`/`Deserialize` 但无 load/save）；`crates/soulcore/src/commands/shell.rs`（`complete_wizard` 忽略入参里的当前配置，重新 `Config::default()`）；`apps/desktop/src/App.tsx`（`finished` 只是 React state，注释自陈"哪个文件回答这个问题属于 WP13"） |
| D4 | **WP06 与 WP03 两套问卷仍未合并**，且分歧比 STATUS 记载的深 | 中 | 可带债（但合并成本被低估，见 §3.1） | `crates/soul-import/src/questionnaire.rs`（`QUESTIONS` 八题，键 `voice.directness` / `boundary.topics` / `value.what_matters` …，答案是自由文本 `Answer { question_key, text }`）；`crates/soul-profile/src/questionnaire.rs`（`questionnaire()` 七题 = 五条轴 + `VOICE_QUESTIONS` 两题，键 `q.voice.directness` 与 `axis.question_id`，答案是封闭枚举 `Answer::Axis{position} / Answer::Voice{setting}`）；`crates/soul-profile/Cargo.toml` 不依赖 `soul-import`，全仓无 `impl UserStatedSink for` 的 WP03 实现 |
| D5 | **导入不是事务**，中途失败留下写了一半的导入 | 中 | 可带债 | `crates/soul-import/src/commit.rs`（模块注释列出四类行按序写入）；`crates/soul-store-api/src/lib.rs` 的 trait 表面**没有任何** `begin`/`transaction`/`with_tx` 入口——补事务是存储边界改动 |
| D6 | **导入事件不去重**，同一文件导入两次写两遍事件 | 中 | 可带债（但见 §3.3：会污染 AC-08 的强度带） | `crates/soul-import/src/commit.rs` 模块注释；`crates/soul-store-api/src/lib.rs` 的 `EventStore` 无按外部 id 查询的入口 |
| D7 | **`consent_id` 恒为 `None`** | 低 | 可忽略 | `crates/soul-import/src/questionnaire.rs`（`consent_id: None`）；`crates/soul-collect/src/consent.rs`（`ConsentHandle` 只有 state/grant/revoke/snapshot，没有 id）。**契约允许**：`docs/schemas/event.schema.json:7` 的 `required` 不含 `consent_id`，`:34` 的 `oneOf` 显式接受 `null`。没有任何 AC 要求它 |
| D8 | **采集内容密钥按"这次运行"而不是按天** | 低 | 可带债（UI 必须如实写"忘掉这次采集"而不是"忘掉昨天"） | `crates/soul-collect/src/collector.rs` + `crates/soulcore/src/commands/collect.rs::forget_unit`；`crates/soul-store-api/src/forget.rs:16-21`（`ForgetUnit` 只有 `Memory` / `Contact` / `ContentKey` 三种，没有按时间的单元） |
| D9 | **`apps/desktop/src-tauri` 是独立 cargo workspace**，导致三个 CI 面完全没覆盖它 | 中→严重（因 AC-26） | 打包一项**阻塞**，其余可带债 | 见 §1.1 |

### 1.1 D9 展开：独立 workspace 对 CI / `just ci` 的真实影响

`origin/cursor/soul-goal1-7b1c:Cargo.toml` 的 `members` 列出 13 个 `crates/*`，**不含 `apps/`**。由此产生四条实际缺口，其中前三条 STATUS 没有点名：

1. **`apps/desktop/src-tauri` 的 Rust 代码没有 fmt / clippy 门禁。** `.github/workflows/ci.yml` 的 `lint` job 只跑 `cargo fmt --all -- --check` 与 `cargo clippy --workspace --all-targets --all-features`，两条都作用于根 workspace。`justfile` 的 `desktop-check` 只是 `cargo check`（不是 clippy），而且**从未被任何 CI job 调用**——Windows job 调的是 `desktop-test`。
2. **`cargo deny` 完全看不到桌面依赖树。** `deny.toml` 的 `[graph]` 针对根 manifest 解析；Tauri 2 的整棵传递依赖（数百个 crate）没有过许可证检查、没有过 advisory 检查。这条直接落在 WP13 的 SBOM 上：只对根 workspace 出 SBOM 等于漏掉实际发行的那一半。
3. **`xtask e0-audit` 的依赖面同样只覆盖根 workspace。** `crates/xtask/src/egress.rs:207` 的 `MetadataCommand::new().manifest_path(repo_root.join("Cargo.toml"))` 只解析根图。桌面依赖图的 E0 保证**只**来自 `apps/desktop/src-tauri/tests/no_egress_path.rs`，而那个测试只在 Windows job 跑。源码 URL 扫描这一半是覆盖的：同文件 `:217` 的 `for tree in ["crates", "apps"]` 会扫 `apps/`，`tauri.conf.json` 里的 `devUrl: "http://127.0.0.1:1420"` 靠 `ALLOWED_URL_PREFIXES`（`:67-72`）放行。
4. **`tauri build` 从未在任何 runner 上跑过**（这条 STATUS 记了，`docs/STATUS.md:270`）。`apps/desktop/src-tauri/tauri.conf.json` 的 `bundle.targets: ["nsis"]`、`installMode: "currentUser"`、`webviewInstallMode: {type: "skip"}` 目前全部只是 JSON 字段，没有产物验证过。AC-26 的门禁文本是「lint/test/schema/红线/**打包**绿」，PRODUCT_LOCK v0.1 切片第 13 条是「CI + **安装 smoke**」——这一条现在是 0。

另外 `apps/desktop/src-tauri/Cargo.lock` 是仓库里的第二份锁文件，任何只看根锁文件的升级/审计流程都会漏掉它。

---

## 二、剩余 WP 拆解

`docs/GOAL1_PLAN.md` 只写了 WP01 / WP02 / WP08 三份「完成定义」（`:43-71`）；**WP10 / WP11 / WP09 功能视图 / WP13 在 GOAL1_PLAN 里没有完成定义**，只在 DAG（`:23`）、批次表（`:37`）、仓库布局（`:41`）和红线（`:82`）里被点名。它们的验收依据只能来自 `docs/FORMAL_WORK_PROMPT.md:74-105` 的 AC 矩阵与 `docs/PRODUCT_LOCK.md:87-103` 的 13 条切片。**这本身是一处规划债：后四个 WP 的开工者没有和前八个同等粒度的完成定义可对。**

### 空 crate 情况（明确核实）

`crates/` 下**没有** `soul-draft`，也**没有** `soul-fileplan`。全仓对这两个名字的引用只有两处，都在文档里：`docs/GOAL1_PLAN.md:41`（目标态布局）与 `:82`（红线「`soul-fileplan` 无写 API」）。落地点已经预留：`crates/soulcore/src/commands/mod.rs` 的模块目录注释里写着 `draft.rs`（WP10）与 `fileplan.rs`（WP11），但 `pub mod` 声明只有八个，这两个不在其中。

### WP10 起草（不发送）+ 人事分析摘要

覆盖 AC-07 起草侧、AC-11、AC-12、AC-13、AC-16、AC-17，以及 PRODUCT_LOCK 切片第 6、8 条。

要交付：
- 新 crate `crates/soul-draft`。
- 确定性语气模板路径：从 `soul_profile::service::read_voice` 取用户钉住的 `VoiceProfile`（`crates/soul-profile/src/voice.rs`，STATUS WP03 已明说这就是 WP10 要读的那个值），无 LLM key 时纯模板出稿（AC-17）。
- E1 路径：请求体过 `soul_policy::redactor` 产出 `RedactedBody`，向 `soul_policy::net_guard` 取 `EgressPermit`，组 `E1RequestPlan` 交 `soul_egress::send`。这三样已经落地（`crates/soul-egress/src/lib.rs:83`、`crates/soul-policy/src/{e1,net_guard,redactor}.rs`），WP10 是**装配**不是新建。
- 一次性豁免（AC-13）的调用面：`redactor` 已实现按值消费，WP10 要给它一个不会被记住的入口。
- 人事分析摘要（AC-16）：每条结论带 `evidence_ids`，过 `soul_policy::assert_non_clinical`，且**不落无证据的 inference**（红线 6）。
- HITL：`ActionKind::DraftReply` / `AnalysePeople` / `GenerateWithUserEndpoint` 已在 `crates/soul-policy/src/hitl.rs:44-52` 定义，WP10 要真的走这道门。
- 测试基座已就绪：`crates/soul-testkit/src/mock_llm.rs` 有 `chat_completions_url()`、`requests()` 与 `set_redirect()`，AC-11/12 可以在线上验。
- `crates/soulcore/src/commands/draft.rs` 薄封装 + `pub mod draft;`。
- 前端 `apps/desktop/src/routes/` 的 `/draft` 路由（现在是 `Pending`，`apps/desktop/src/router.tsx` 标 `ownedBy: "WP10"`，且 `apps/desktop/src/App.test.tsx` 有一条断言钉住「这页没有输入框也没有发送按钮」——加控件会先撞红它，要连测试一起改）。

### WP11 只读目录扫描与计划预览

覆盖 AC-18、AC-19（AC-19 的令牌侧 WP08 已过），PRODUCT_LOCK 切片第 9 条。D31 钉死：写执行是 v0.1.1（AC-27），不在 Goal 1。

要交付：
- 新 crate `crates/soul-fileplan`，**红线要求它没有任何写 API**（`docs/GOAL1_PLAN.md:82`）——需要一条像 WP02 `research_preview.rs` 源码级自查那样的测试来证明（读回自己的源码，断言不含写文件 API）。
- 授权根之外 100% 拒绝：依据来自 `Config::authorized_roots`（`crates/soulcore/src/config.rs`）。**这里撞上 D3**：`authorized_roots` 今天永远是空 `Vec`，没有任何命令能往里加，所以 AC-18 的「授权 A」这一半在真机上无路可走。
- 「A 磁盘不变」的实证：扫描前后目录文件名与字节数完全不变的断言，可照抄 WP02 研究预览的做法。
- `ActionKind::ScanDirectory` / `PlanFiles` 已定义（`crates/soul-policy/src/hitl.rs:47-48`），`CapabilityScope::FileWrite` 已定义且 `hitl.rs:106` 明确 v0.1 不签发。
- `crates/soulcore/src/commands/fileplan.rs` + `pub mod fileplan;`。
- 前端 `/files` 路由（`apps/desktop/src/router.tsx` 里 `ownedBy: "WP11"`，文案已写「这里不会出现『执行』按钮」；`App.test.tsx` 有对应断言）。

### WP09 功能视图

DAG（`docs/GOAL1_PLAN.md:23`）把它排在壳之后、WP13 之前。它没有独立的 AC 编号——它是让**已经通过 CI 的核心能力真正被用户够到**的那一段。范围由 `apps/desktop/src/router.tsx` 里标 `ownedBy: "WP09 功能视图"` 的路由界定，共五条：

| 路由 | 要接的核心 |
|---|---|
| `/profile` 灵魂档案 | `soul_profile::view::profile_view`、`correct_axis`、`set_voice`（AC-06 / AC-07 的 UI 侧） |
| `/graph` 人脉图 | `soul_graph::view` / `build`（AC-08 的 UI 侧） |
| `/memory` 自传记忆 | `soul_memory::service` CRUD + `preview_forget` / `execute_forget`（AC-14 / AC-15 的 UI 侧） |
| `/research` 研究预览 | `soulcore::commands::store::research_preview`（AC-20 的 UI 侧，且必须只在屏幕上出现） |
| `/audit` 审计 | `AuditLog::list_audit`（切片第 12 条） |

**没有列出但必须有的两块**（见 §3.2）：导入页与采集同意开关。

这一段的前置条件是 D1 + D2 + D3 三条一起解决：没有真 store 就没有东西可显示，没有 DPAPI 就在 Windows 上开不了真 store，没有配置持久化就没有采集开关和授权根。

工程约束照抄壳的三道锁：`eslint.config.js` 的 `no-restricted-imports`、`src/contract.test.ts` 的源码复查、`src-tauri/tests/command_surface.rs::the_command_layer_stays_thin`（每个 wrapper 函数体最多一条语句）。新增的每个命令都要同时进 `COMMAND_NAMES`（`apps/desktop/src-tauri/src/commands.rs`）与 `src/core.ts` 的 `COMMANDS`，否则契约测试红。**注意 store 句柄**：`docs/STATUS.md` WP07 遗留 8 说整个进程只能有一个 store 句柄（`soulcore::commands::collect::share` 是那个 `Arc<Mutex<..>>` 包装），功能视图接线时必须在 `lib.rs` 的 `setup` 里开一次、`manage` 起来。

### WP13 安装 smoke、CI、SBOM

覆盖 AC-01 的可自证部分、AC-26，PRODUCT_LOCK 切片第 13 条。

要交付：
- **配置文件**：`%LOCALAPPDATA%\Soul\` 下的配置落盘与回读，这是 D3 的正解。`crates/soulcore/src/commands/store.rs:22` 已经把 `DATABASE_FILE_NAME` 和「解析这个目录是 WP09 的活」写在注释里，配置文件应与 `soul.db` 同目录。同时决定「向导是否已完成」存在哪里（`apps/desktop/src/App.tsx` 的注释已把这个问题指名给 WP13）。
- **NSIS 打包在 CI 上真跑一次**，产出安装器工件。这是 AC-26「打包绿」唯一的实现路径。
- **安装 smoke**：安装器以 `currentUser` 装、进程名 `soul.exe`、不提权。
- **SBOM**：必须覆盖两个 workspace（见 §1.1 第 2 点）。
- **补齐 CI 缺口**：把 `desktop-check`（应升级为 clippy）接进某个 job；给 `cargo deny` 加第二份 manifest；考虑把 `e0-audit` 的依赖遍历也指向桌面 manifest。
- **迁移器**：`docs/STATUS.md` WP02 遗留 8 说 `meta.schema_version = 1` 还没有迁移器，"表结构变了要么加迁移，要么开发期删库重来"。第一个可安装版本发出去之后就没有"删库重来"这个选项了——这个决定必须在 WP13 关门前做掉。
- `xtask` 的 `EXEMPT_DIRS` 归属问题（WP07 遗留 9）已由 WP09 用标记文件的办法解掉（`crates/xtask/src/egress.rs:91` 的 `EXEMPT_BUILD_OUTPUT`），WP13 不用再管。

---

## 三、STATUS 未记载但我发现的缝

### 3.1 两套问卷的分歧不是"题号不同"，`UserStatedSink` 合不上

`docs/STATUS.md:203` 写「并的时候 `UserStatedSink` 就是那个接口——`soul-profile` 实现它即可，`soul-import` 一行不用改」。核实下来这句站不住，有三层障碍：

1. **答案的类型不同。** `soul-import` 的答案是自由散文（`Answer { question_key: String, text: String }`，`crates/soul-import/src/questionnaire.rs`）；`soul-profile` 的答案是封闭枚举（`Answer::Axis { position: AxisPosition }` / `Answer::Voice { setting: VoiceSetting }`，`crates/soul-profile/src/questionnaire.rs`）。后者 `check()` 对未知题号、题型不符、重复作答一律报错，**没有接收自由文本的入口**。把八题散文喂给 WP03 需要一个从散文到 `AxisPosition` 的映射——那要么是 LLM（AC-17 说无 key 也要能用），要么就是把题型改成选择题。这是产品决定，不是接口实现。
2. **`RecordedAnswer` 不携带答案内容。** 它只有 `event_id` / `evidence_id` / `method` / `question`（`crates/soul-import/src/questionnaire.rs`）。一个实现 `UserStatedSink` 的 `soul-profile` 拿到 `voice.directness` 的 `RecordedAnswer`，**无法知道用户到底偏哪一边**，除非它去 `BlobStore::open` 解那个密封体——而 `soul-profile` 现在既不依赖 `soul-store` 也不在 trait bound 里要 `BlobStore`（`intake` 的 bound 是 `ProfileStore + AuditLog`）。所以「`soul-import` 一行不用改」是错的：至少 `RecordedAnswer` 要扩，或者 sink 的签名要扩。
3. **两条路写出的证据行形状不兼容。** 同样是 `EvidenceKind::Questionnaire`，`source_refs` 的键完全不同：`soul-import` 写 `{"ref_kind": "questionnaire_answer", "event_id": .., "question_key": ..}`；`soul-profile` 写 `{"origin": "questionnaire", "question_id": .., "axis_id": .., "position": ..}`（`crates/soul-profile/src/service.rs::put_answer_evidence`）。任何要把问卷证据解引用给用户看的视图（WP09 功能视图的档案页正是）必须认两种形状，或者先做一次数据迁移。

**建议改称**：这不是"收尾前并成一套题号"，而是"两条 AC-03 实现并存，其中只有 `soul-profile` 那条能产出非空档案"。合并前应先决定保留哪一条。

### 3.2 UI 上不存在导入页，也不存在采集同意开关

`docs/STATUS.md` WP09 遗留 9 写「起草 / 文件计划 / **导入** / 记忆 / 人脉这些路由是空的」。核实 `apps/desktop/src/router.tsx` 的 `RouteId` 联合类型，九条路由是 `home / profile / graph / memory / draft / files / research / audit / settings`——**没有 `import` 这一条**。也没有任何路由承接问卷。

配套地，`ConfigSnapshot`（`crates/soulcore/src/commands/shell.rs`）只有读，`COMMAND_NAMES` 只有 `config_snapshot` / `complete_wizard` / `cloud_toggle` 三个，其中没有一个能修改配置。结果是：

- **AC-03 / AC-04 / AC-05（问卷与两种导入）在 UI 上没有入口**，PRODUCT_LOCK 切片第 2 条在真机上不可达。
- **AC-09 / AC-10（采集开关）在 UI 上没有入口**——`Home.tsx` 只是渲染 `snapshot.collect_enabled ? "开" : "关"`，永远显示"关"，没有能打开它的控件。切片第 7 条「可选前台应用采集」在真机上不可选。
- **AC-11 / AC-17（LLM 端点）没有填写入口**，`Settings.tsx` 只显示"未填写"。
- **AC-18（授权目录）没有授权入口**。

所以「剩余 WP」的真实缺口比路由表大：WP09 功能视图除了那五条 `ownedBy: "WP09 功能视图"` 的路由，还要**新增导入/问卷页**，并在设置页补上四组写入控件（采集同意、LLM 端点、授权根、以及配置持久化的落点）。这四组写入控件在今天的 WP 划分里**没有明确归属**——WP09 功能视图和 WP13 都可以争，需要父代理指派。

### 3.3 导入事件不去重会直接污染人脉图的强度带，不只是"多一份事件"

`docs/STATUS.md` WP06 遗留 6 把这条描述成「事件会多一份」，语气是可接受的冗余。但 `crates/soul-graph/src/build.rs` 的 `TieStrength` 是**计数**（往/来/会话数/活跃天数），阈值 `MODERATE_MIN_INTERACTIONS = 3`、`STRONG_MIN_INTERACTIONS = 10`（STATUS WP05 取舍 2）。同一个 Telegram 导出被导两次，每条边的互动计数直接翻倍——一条本该是"弱"的边会跳到"中"，本该"中"的会跳到"强"。而 `rebuild` 是幂等的（按 `(owner, peer)` 认边），所以**用户看不到重复的边，只会看到被夸大的强度**，这比看到重复更难发现。

PRODUCT_LOCK 的不可协商约束里有「推断带证据」，被翻倍的证据在技术上仍然可解引用，但它支撑的那个强度结论是错的。这条应从「低」提到「中」，并且在 WP09 功能视图接导入页之前，最低限度要在 UI 上警告重复导入，或者由调用方在导入前做一次"这个文件是否导过"的检查。

### 3.4 D1（DPAPI）与 D2（壳接真库）是同一条阻塞链，不是两条独立遗留

STATUS 把它们分别记在 WP02 遗留 3 与 WP09 遗留 8，读起来像两件可以并行的事。实际上 `crates/soulcore/src/commands/store.rs::open_store(directory, keys: &dyn KeyProvider)` 要求一个 `KeyProvider`，而 Windows 上唯一名义上的真 provider 就是 `DpapiKeyProvider`，它两个入口都 `Err`。剩下的选项只有 `TestKeyProvider`——名字里就写着只作测试用，用它发版等于把 KEK 明文放在 `%LOCALAPPDATA%` 里，直接违反 `docs/SECURITY.md:44`「补齐 DPAPI 前不得声称 Windows 上 KEK 已受保护」和 PRODUCT_LOCK 数据面「DPAPI 保护 KEK」。

**结论：DPAPI 是 WP09 功能视图的硬前置。** 而它自身还有一个未解的工程约束：`crates/soul-store` 是 `#![forbid(unsafe_code)]`，Win32 绑定需要 `unsafe`。`crates/soul-collect` 已经解过同一类问题（commit `8b780e0` "confine the unsafe to the Windows file"），可以照抄那个模式，但这需要在派工时明确允许改 `soul-store` 的 crate 级属性。

### 3.5 `soul-profile` 的问卷证据既没有事件也没有内容密钥，因此永远无法被遗忘

`intake` 的 bound 是 `ProfileStore + AuditLog`，`put_answer_evidence` 只调 `put_evidence`——不写 `SoulEvent`，不 `seal`，因此没有 `content_key_id`。`ForgetUnit` 只有 `Memory` / `Contact` / `ContentKey` 三种（`crates/soul-store-api/src/forget.rs:16-21`），所以这些证据行落在任何遗忘单元之外。

危害有限（存的是题号 + 封闭枚举位置，没有散文，`put_answer_evidence` 的注释也明说了"没有一个字是人写的"），但它确实是关于用户的一句陈述，而且「用户可以忘掉自己说过的话」是 PRODUCT_LOCK 的承诺之一。对照之下，`soul-import` 那条路是有 CK 的（`QuestionnaireReceipt.content_key_id`，注释里明写「一个 run 一把钥匙，所以『忘掉我告诉向导的话』是单个 `ForgetUnit::ContentKey`」）。**这是同一个功能的两条路在遗忘覆盖面上的不对称**，应在 §3.1 的合并决定里一并解掉。严重度低，不阻塞，但 UI 上不能声称档案里的一切都能忘掉。

### 3.6 `soulcore::commands::shell::config_snapshot()` 与 Tauri 命令走的不是同一条路

自由函数 `config_snapshot()` 内部构造 `Config::default()`；Tauri 命令 `commands::config_snapshot` 走的是 `ConfigSnapshot::of(&config.0)`（managed state）。今天两者结果相同（因为 managed state 就是 default），一旦 D3 的配置持久化落地，那个自由函数就会变成一个悄悄返回默认值的陷阱。同理 `complete_wizard(&answers)` 完全不读当前配置。属于低严重度整洁问题，但要在接持久化的那个 PR 里一起清掉，否则会变成一个很难查的 bug。

### 3.7 后四个 WP 没有完成定义

已在 §2 开头说明。`docs/GOAL1_PLAN.md` 对 WP01/02/08 各写了 5–6 条可勾选的完成定义并逐条勾了；WP03–WP07、WP09 的完成情况只在 STATUS 里以「交付/证据」表事后追认。WP10 / WP11 / WP09 功能视图 / WP13 现在既没有事前定义也没有事后记录。派工前应先补齐这四份完成定义，否则「Goal 1 完成 = AC 矩阵全绿」这条判据缺少中间层。

---

## 四、建议的修补顺序

不写代码，只排序。理由是依赖关系与"错误发现得越早越便宜"。

**第 0 步（先于任何派工，父代理直做）**
补 `docs/GOAL1_PLAN.md` 里 WP10 / WP11 / WP09 功能视图 / WP13 的完成定义（§3.7）；并对 §3.2 里那四组配置写入控件指派归属。这是文档改动，不涉及业务逻辑。

**第 1 步 — 解开 D1（DPAPI）**
理由：它是唯一一条阻塞在 D2、D3、WP09 功能视图、WP13 之前的死结，而且是全项目里唯一需要引 `unsafe` 与新依赖的改动，风险最高，应当最先暴露。派工时要一并授权修改 `crates/soul-store` 的 `#![forbid(unsafe_code)]`（照 `soul-collect` 把 unsafe 关进 Windows 文件的做法）。它在 Windows CI 上可自证（`CryptProtectData` 往返 + 关库重开），不必等真机。

**第 2 步 — D3 配置持久化 + D2 壳接真库，同一个 PR**
理由：两者互为前提。配置文件与 `soul.db` 同目录，都需要先解析 `%LOCALAPPDATA%\Soul\`；store 句柄按 WP07 遗留 8 的要求在 `lib.rs` 的 `setup` 里开一次并 `manage`。这一步顺手清掉 §3.6 的双路径，并把"向导是否已完成"从 React state 挪到配置文件。做完这一步，`Home.tsx` 上显示的才是真配置而不是常量。

**第 3 步 — §3.2 的四组写入控件 + 导入/问卷页**
理由：这是把已经通过 CI 的能力交到用户手里的最小一步，也是 AC-03/04/05/09/10/18 从"CI 绿"变成"真机可达"的分界。同时它会立刻逼出 §3.1 的问卷决定——导入页上必须只有一套问卷。

**第 4 步 — §3.1 问卷合并决定（先决定，再实现）**
理由：放在第 3 步之后是因为 UI 一落地就无法再回避"用户被问两遍"；放在 WP10 之前是因为 WP10 起草要读 `VoiceProfile`，而语气值现在有两条写入路径。建议先出一份短决定（保留 `soul-profile` 的封闭枚举题，把 `soul-import` 的八题降级为纯证据留存，或反之），再派实现。合并时一并解 §3.5 的遗忘不对称。

**第 5 步 — WP10 与 WP11 并行**
理由：DAG 就是这么排的（批 5），且两者依赖的 `soul-policy` / `soul-egress` / `soul-testkit::mock_llm` 全部已就位。WP11 依赖第 3 步交付的授权根写入口；WP10 依赖第 4 步定下的语气单一来源。两个 WP 面向不同的红线（WP10 是出网面，WP11 是文件面），符合"权限/数据面与 UI 面分人"。

**第 6 步 — D5（导入事务）与 D6/§3.3（事件去重），一起做**
理由：两者都要动 `soul-store-api` 的边界（前者加事务入口，后者加按外部 id 的查询或唯一约束），一次改边界比两次便宜；而且 §3.3 的强度污染在导入页交付（第 3 步）之后才会被真实用户碰到，所以不能拖到第 6 步之后。如果时间紧，去重可以退化为"导入前查一次同源文件是否导过"并在 UI 上警告，事务则不宜退化。

**第 7 步 — WP13**
理由：收尾。此时才有值得打包的东西。内容按 §2 的 WP13 一节，其中 §1.1 的三条 CI 缺口（desktop 无 clippy、`cargo deny` 漏桌面树、`e0-audit` 依赖面漏桌面树）应与 SBOM 一起解决；`meta.schema_version` 的迁移器决定必须在出第一个安装器之前落定。

**始终不动的**
D7（`consent_id = None`）与 D8（采集 CK 按次）在 v0.1 内不修。前者契约允许且无 AC 要求；后者只要 UI 文案如实写"忘掉这次采集"而不是"忘掉昨天"即可。两条都保留在 STATUS 的遗留里。
