MODEL: claude-fable-5-thinking-xhigh

# SOTA 验收条 — WP10 起草 / WP11 只读文件计划 / 壳接库

写给批 5/6 的实现者与复核者。每条验收条都是「测试能红」的断言：条目里的【红法】说明哪一种假实现、偷懒或绕路会让它变红。写不出红法的条目没有资格进本文件。

先例约定（写测试前先读这四个文件，别另起炉灶）：

- **wire 级取证**：`crates/soul-testkit/src/mock_llm.rs` 保存的是原始请求字节（`RecordedRequest.body`），泄漏检查必须对着它跑，不许对着重新序列化的本地字符串跑（该文件注释明说后者是错的对象）。
- **源码级自查**：`crates/soul-store/tests/research_preview.rs::the_research_module_cannot_reach_the_filesystem`——`include_str!` 读回源码禁字面量，并**反向断言**扫的是对的文件。
- **磁盘不变快照**：同文件 `a_preview_leaves_no_new_file_behind` 的 `entries()`（文件名+字节数）。
- **测试自身不许作弊**：`crates/soul-collect/tests/the_tests_do_not_fake_collection.rs`——扫描测试目录禁写调用，并带「扫描器认得出真写调用」的对照用例。

---

## 一、WP10：起草（不发送）+ 人事分析摘要

落点：`crates/soul-draft`（新）+ `crates/soulcore/src/commands/draft.rs`（薄封装）。地基全部就绪：redactor/permit（`crates/soul-policy/src/{redactor,net_guard,e1}.rs`）、`soul_egress::send`（`crates/soul-egress/src/lib.rs`）、`read_voice`（`crates/soul-profile/src/voice.rs`）、`MockLlm`（`crates/soul-testkit/src/mock_llm.rs`）。

### D-01 永不发送：无 SMTP/IM 依赖、无发送 API、wire 上只有生成请求

【断言】三层，缺一不可：

1. **依赖图**：`crates/xtask/src/egress.rs` 的禁用名单扩到消息发送类 crate（至少 `lettre`、`async-smtp`、`imap`、`async-imap`、`teloxide`、`grammers-client`、`matrix-sdk`、`tokio-tungstenite`、`tungstenite`），normal/build 边遍历（既有 `e0-audit` 机制）从每个成品 crate 出发都走不到；`crates/xtask/tests/self_test.rs` 加对照用例：把 `soul-testkit` 当成品根时确实找得到 `axum/hyper` 的同款手法，对合成的 `lettre` 依赖必红。
2. **源码**：`soul-draft` 的每个 `src/*.rs`（运行时 `read_dir` 枚举，不是 `include_str!` 钉死名单——新加文件不能躲）里不得出现 `fn send`、`smtp`、`sendmail`、`deliver`、`transmit` 字面量；对外草稿类型（如 `DraftReply`）上没有任何以发送为语义的方法。反向断言扫描器读到了 `fn draft`（证明扫的是对的 crate）。
3. **wire**：跑完「粘贴→起草→拿到草稿」全链路后，`MockLlm::requests()` 里每条的 `path` 都是 `/v1/chat/completions`、方法都是 POST；没有第二个 origin 收到过任何字节（用第二个 decoy `MockLlm` 断言 `request_count() == 0`）。

【红法】实现者给草稿加「一键发送」方法→2 红；顺手引入 telegram bot 库→1 红；把草稿 POST 给别的路径或别的端口→3 红。只做 3 不做 1/2 的话，「加了发送 API 但测试没调它」就能混过——所以三层都要。

### D-02 mock LLM 只打精确 origin；跨 origin 302 拒绝且目标零接触

【断言】沿用 `crates/soul-egress/tests/e1_origin.rs` 的五件套，但从**起草入口**（`soul-draft` 的公开 API 或 `soulcore::commands::draft`）触发，不是直接调 `soul_egress::send`——WP08 已经证明过 send 本身，WP10 要证明的是起草链路没有绕开它：

1. 配置 origin A（`MockLlm`），另起 decoy B。起草后 A 收到 ≥1 条、B 恒 0。
2. `A.set_redirect(B.chat_completions_url())` 后再起草：返回错误且 `reason_code` 是 `ReasonCode::E1CrossOriginRedirect`，B 仍是 0，**并且用户拿到的是可读失败而不是静默空草稿**。
3. 同 host 换端口的 302 也是跨 origin（`e1_origin.rs::a_redirect_to_another_port_on_the_same_host_is_still_cross_origin` 的同款断言，从起草入口跑）。

【红法】起草层自己 new 一个 reqwest client（`cargo deny` + `e0-audit` 红）；跨 origin 失败被吞掉降级成模板且不报错（断言 2 的「可读失败」红）；重定向后把 B 当新端点重试（B 的 0 红）。

### D-03 默认请求体过 LeakageChecker：第三人 ≥8 字、姓名、账号一个都不上线

【断言】fixture 对话含：一段 ≥8 scalar 的第三人正文、一个两字中文姓名、一个 `@handle` 与一串 11 位号码、一条 `SealedSubject::Mixed` 的混合消息。默认起草后，对 `MockLlm::requests()` 里**每一条**的原始 `body`：

1. 以 fixture 全部第三人正文与标识符为语料的 `LeakageChecker`（`crates/soul-testkit/src/leakage.rs`，默认阈值 8 + 姓名/账号规则）`assert_clean`；
2. **反向**：body 里必须出现 `soul_policy::redactor::THIRD_PARTY_PLACEHOLDER` / `NAME_PLACEHOLDER` / `ACCOUNT_PLACEHOLDER`，且用户自己那条消息的原文**在**——空请求体或压根没发请求也能过 1，反向断言堵死这条路；
3. mixed 消息按第三人占位（`Turn::is_third_party` 已把 Mixed 算第三人，起草侧不得自己重新分类——断言 mixed 那条的原文不在 body 里）。

【红法】自己拼 prompt 绕过 `Redactor`（`RedactedBody` 无公开构造函数，绕的人只能改 `soul-policy`，review 必见）；只对占位后的本地字符串跑检查而 wire 上是另一个字符串（对着 `RecordedRequest.body` 跑就抓得住）；把 mixed 当 owner（断言 3 红）。

### D-04 一次性豁免只生效一次，且豁免 ≠ 放行标识符

【断言】用 `ExemptionRequest::for_turn(id).confirm(true)` 拿到 `OneShotExemption`（`crates/soul-policy/src/redactor.rs`）起草第一稿：

1. wire body 含被豁免那**一条** turn 的原文；其余第三人 turn 仍是占位；姓名/账号即使在被豁免的 turn 里也仍是占位（redactor 已实现，wire 级再证一遍）；
2. 紧接着**不带新豁免**起草第二稿：wire body 对同一 LeakageChecker 全干净——豁免没有被任何状态记住；
3. 类型级：`OneShotExemption` 不是 `Clone`/`Copy`，按值被 `redact_for_e1_with_exemption` 消费——测试想复用它会**编译失败**，这条写进注释即可；运行期断言审计里记的是 `carries_exempted_original` 的事实与 `turn_id`，没有原文。

【红法】起草层把豁免存进 session/config「方便用户」→2 红；豁免时顺带把电话号码放行→1 红；审计写「用户豁免了这句话：……」→3 红（也撞 D-09）。

### D-05 无 endpoint：确定性语气模板，零连接（loopback 也不许）

【断言】`EgressConfig::closed()` 下：

1. 起草仍返回草稿，同一输入调两次**逐字节相等**（确定性）；
2. 模板不是常量：`VoiceDirectness::Direct` 与 `Reserved`、`EmojiUse::Never` 与 `Frequent` 产出**不同**草稿（否则「返回固定字符串」就能过 1）；
3. 零连接的取证是个陷阱题：起个 loopback `MockLlm` 并把它的 URL 递到起草代码够得着的地方，断言 `request_count() == 0`。注意 `NetGuard::authorize` 对 loopback 会发 **L 类 permit**（`crates/soul-policy/src/net_guard.rs::authorize`），起草的 E1 路径必须走 `authorize_e1`——无 endpoint 时它必拒（`EndpointNotConfigured`）。用 `authorize` 的实现会在这条上红；
4. 模板产出过 `soul_policy::assert_non_clinical`（`crates/soul-policy/src/clinical.rs`）与 `fixtures/denylist/diagnostic_terms.txt` 全量扫描——81 种语气组合穷举渲染（先例：WP03 取舍 3，denylist 抓到过一次真的）。

【红法】自动探测本机 Ollama「提升体验」→3 红；模板写死一句话→2 红；模板里出现「量表」类词→4 红。

### D-06 AC-07：用户 set_voice 后下一稿用用户值，推断不覆盖

【断言】接 `crates/soul-profile/src/voice.rs` 的真值，两条路径都测：

1. 用户 `set_voice(Directness::Direct)` 后：模板路径的下一稿与 `Reserved` 时的产出可区分（复用 D-05 断言 2 的可区分性）；E1 路径的下一稿 wire body 里承载的语气描述来自 `read_voice` 的用户值，不是 `VoiceProfile::default()`；
2. 随后一条「更强的推断」`suggest_voice` 返回 `false`（WP03 已证），**再下一稿**与用户值那稿一致——起草层不许缓存旧 voice：set_voice 与下一次起草之间不重启进程，靠缓存的实现会红；
3. 语气值进 prompt 的方式是本 crate 的常量模板拼接，不是把 `profile.voice` 的自由 JSON 整段塞进去（断言 wire body 里没有 `user_set` 字段名——那是内部锁定名单，不该上线）。

【红法】起草读 `Config` 里的副本而不是档案→2 红；把 voice JSON 原样进 prompt→3 红。

### D-07 人事摘要：每条 evidence_ids 可解引用；非临床；无证据不落库；LLM 输出不是证据来源

【断言】对真 `SqlCipherStore`：

1. 摘要的每条 claim 携带 `evidence_ids`，测试逐个 `get_evidence` 真解引用；指向不存在证据的 claim 被拒而不是给空列表（仿 `crates/soul-profile/src/view.rs` 的 `DanglingEvidence` 语义，正反都测）；
2. 无 evidence 的 claim 在碰存储之前就被 service 层拒，真库层再拒一遍（两道网，仿 `soul-profile::service::record_axis_inference`）；
3. 所有可读字符串过 `assert_non_clinical` + denylist fixture；mock LLM 故意回诊断词时，摘要**拒绝生成**而不是过滤后照发（`clinical.rs` 的文档明说这是断言不是过滤器）；
4. **evidence_ids 必须来自本地查询**：mock LLM 回复里伪造一批 uuid 当「证据」，断言摘要不引用它们——LLM 输出是 `UntrustedText`，不是结构化事实；
5. 无 key 降级：`EgressConfig::closed()` 下摘要走统计路径（计数、时间跨度、互动形状——`crates/soul-graph` 的 `TieStrength` 语义），仍满足 1–3，且数字必须是真查询（同一库第二个人的计数不同——常量过不了，先例：WP02 遗忘预览）。

【红法】把 LLM 回复 json 解析出来直接入库→4 红；摘要数字写死→5 红；诊断词靠 replace 洗掉再发→3 红。

### D-08 AC-25 粘贴注入：不能变指令、不能外连、只进引用栏

【断言】以 `fixtures/injection/paste_injection.txt` 全量为语料（含指令覆盖、角色伪装、工具调用形状、伪造批准、URL、shell 命令——`crates/soul-policy/src/injection.rs` 的六类信号）：

1. 粘贴只能经 `UntrustedText` 进入起草（类型层面：`soul-draft` 的公开入口收 `UntrustedText` 或构造 `Turn`，没有收 `String` 直通 prompt 的入口）；
2. wire body 里注入文本只出现在 `QUOTE_OPEN`/`QUOTE_CLOSE` 栅栏**内部**，system 消息与 `soul_policy::e1::DRAFTING_INSTRUCTION` 常量**逐字节相等**（断言整条 system content == 常量，任何拼接都会红）；
3. 对每一行语料、对 `ActionKind::ALL` 每个动作，以 `RequestOrigin::ExternalContent` 请求全部被 `check_action` 拒且理由是 `ExternalContentNotAuthority`（先例：`crates/soul-import/tests/injection_is_data.rs`）;
4. 语料里的 URL：在该 URL 起 decoy 服务，断言 0 请求；`NetGuard` 对它拒绝；
5. mock LLM **回复**一个工具调用形状的 JSON 时，起草结果仍只是文本草稿——没有任何动作被执行、没有连接、没有 HITL 令牌被消费（`TokenIssuer::issued_count()` 不变）。

【红法】把粘贴内容拼进 system→2 红；对模型回复做「智能解析并代办」→5 红；起草 UI 把粘贴文本先给 TS 层「预处理」→撞壳接库 S-04 的三道锁。

### D-09 审计无正文

【断言】跑完 D-02～D-08 的动作矩阵（默认起草、豁免起草、模板降级、摘要、注入被拒）后回放整条审计链：

1. 链验证通过（WP02 语义：`seq`/`prev_hash`/`entry_hash` 由库覆盖，`crates/soul-store/src/audit.rs`）；
2. 全链序列化后过 LeakageChecker，语料 = fixture 里每一句正文 + 用户自己的草稿文本 + mock LLM 的回复文本，阈值调到 **4**（先例：`crates/soul-memory` 的做法）——草稿正文也是正文，审计只许记 id、计数、reason_code、`carries_exempted_original`；
3. 带散文字段的条目在入库前被拒（先例：`crates/soul-policy/tests/audit_chain.rs`）。

【红法】审计里写「为 xx 起草了回复：……」→2 红；把 LLM 回复摘要进审计 detail→2 红（语料里有它）。

---

## 二、WP11：只读目录扫描与计划预览

落点：`crates/soul-fileplan`（新）。红线原文：「`soul-fileplan` 无写 API」（`docs/GOAL1_PLAN.md` 红线节）；D31 只留只读预览，写执行是 v0.1.1（`docs/DECISIONS.md`）。

### F-01 授权根扫描前后磁盘文件名 + 字节完全不变

【断言】fixture 树：多层子目录、中文/emoji/超长文件名、零字节文件、大文件。扫描 + 生成计划预览各跑 3 遍，前后对整棵树做**递归**快照比对：相对路径全集相等，且每个文件的**内容哈希**相等（升级 `research_preview.rs::entries()`：那边只有单层「文件名+字节数」，字节数相等骗得过「改一个字节」，哈希骗不过）。不断言 atime/mtime——读操作在部分文件系统上会动 atime，把它写进断言是让测试为假警报红。**反向**：计划预览必须非空且引用到 fixture 里真实存在的文件（相对路径能在快照里找到）——「什么都不扫」也能过前半条，反向堵死。

【红法】扫描时写缓存/索引文件→路径全集红；「dry-run」实现顺手 touch 了目标目录→哈希红；返回空计划混过→反向红。

### F-02 未授权路径 100% 拒绝

【断言】授权 A、不授权 B，对抗性路径语料至少覆盖：

1. 直接绝对路径 B；
2. `A/../B` 遍历；
3. A 内指向 B 的符号链接（Linux CI 用 symlink 真建；Windows junction 进作者手动清单，测试里留 `cfg(windows)` 位）；
4. 前缀撞名：授权 `/data/a` 时 `/data/ab` 必拒（字符串 `starts_with` 的经典洞）；
5. 大小写变体（大小写不敏感文件系统上 `/DATA/A` 与授权路径的关系要显式决定并测死）；
6. Unicode 变体（NFD 拼法的同名路径）。

每条：返回**带类型的**拒绝错误（不是空计划、不是 panic），且计划输出里 B 下任何文件的路径/名字/字节零出现。symlink 那条额外断言：A/link→B/secret 的场景里 `B/secret` 的内容与元数据都不在计划里——「跟着链接走出去」是这条最真实的失败模式。

【红法】用 `starts_with` 判授权→4 红；canonicalize 前判授权→2、3 红；拒绝时把 B 的路径回显进错误信息且被 UI 展示→与 F-04 的审计断言冲突（未授权路径本身也是不该扩散的信息，错误信息只说「不在授权根内」+ 授权根，不复读请求路径）。

### F-03 源码自查：无 write/rename/remove/create 文件 API

【断言】仿 `research_preview.rs::the_research_module_cannot_reach_the_filesystem`，但两点升级：

1. **运行时枚举** `crates/soul-fileplan/src/` 下每个 `.rs`（先例：`crates/soul-collect/tests/window_titles_are_not_collected.rs` 扫全目录），禁：`fs::write`、`File::create`、`OpenOptions`、`create_dir`、`create_dir_all`、`remove_file`、`remove_dir`、`rename`、`fs::copy`、`hard_link`、`symlink`、`set_permissions`、`set_len`、`truncate`、`BufWriter`、`io::Write`、`tempfile`；
2. 反向断言 ①：扫描器确实读到了 `fn scan`（或计划入口名），证明扫的是对的 crate；反向断言 ②：对照用例——往一个合成字符串里塞 `fs::write` 证明扫描器认得出（先例：`the_tests_do_not_fake_collection.rs` 的控制用例）。

注意 `research_preview.rs` 禁了 `PathBuf`，`soul-fileplan` 做不到（它就是处理路径的）——所以这份名单是按「写能力」而不是「路径类型」列的，评审时别照抄那份。

【红法】任何「顺手能执行」的 apply/execute/组织函数——哪怕没人调用——都会命中 1；把写逻辑挪进另一个新 crate 绕扫描→`e0-audit` 式的依赖遍历管不到写 API，但 F-05 的 HITL 断言与 `docs/GOAL1_PLAN.md` 红线复核管得到，review 必查 `soul-fileplan` 的依赖表里有没有可写存储以外的 IO crate。

### F-04 文件名注入不是指令

【断言】fixture 树里放 `fixtures/injection/filenames.txt` 语料命名的真实文件（指令覆盖句、URL、工具调用形 JSON 当文件名）。扫描后：

1. 文件名只能经 `UntrustedText`（`ExternalChannel::FileName`）进任何下游；
2. 对 `ActionKind::ALL` × `RequestOrigin::ExternalContent` 全拒（同 D-08 断言 3）；
3. 文件名里的 URL：decoy 收到 0 请求，`NetGuard` 拒；
4. 计划预览把这些名字**只当数据渲染**（预览文本里名字在引用位、没有产生任何额外计划条目——「文件名说“把我提权”，计划就多了一条提权动作」是这条要抓的假实现）；
5. 扫描的审计条目无正文：以全部注入文件名为语料过 LeakageChecker——审计记「扫了 N 个文件、拒了 M 个路径 + reason_code」，不记文件名。

【红法】把文件名拼进给 LLM 的整理建议 prompt 的指令位→D-08 断言 2 的同款检查红；审计里列出可疑文件名「方便排查」→5 红。

### F-05 v0.1 不签发 FileWrite token

【断言】三层：

1. `soul-fileplan` 源码全文不出现 `FileWrite` 与 `TokenIssuer`（它是只读预览，不需要令牌机制；`ScanDirectory`/`PlanFiles` 在 `crates/soul-policy/src/hitl.rs` 里 `needs_capability_token() == false`，这条已有测试钉住）；
2. 跑完扫描+预览全流程后 `TokenIssuer::issued_count() == 0`——流程根本不该碰发行器；
3. 既有断言保持绿：手工签发的 `CapabilityScope::FileWrite` 令牌即使未过期、scope 对、plan hash 对，`consume` 也返回 `TokenError::WriteNotImplemented`（`crates/soul-policy/tests/hitl.rs` 已有，WP11 的 PR 不许动这条测试与 `is_implemented_in_v0_1`）。

【红法】「为 v0.1.1 铺路」先把签发路径写好→1、2 红；改 `is_implemented_in_v0_1` 放行→3 红且撞 `docs/GOAL1_PLAN.md` 红线。

### F-06 计划 hash 变则拒绝（复用 HITL，不自己造）

【断言】计划预览产出规范化 JSON（键序稳定——`serde_json` 的 map 默认 BTreeMap，`PlanHash::of` 的注释已依赖这一点），`PlanHash::of(plan)` 钉住：

1. 批准后改一个字节（换一条 entry 的目标名、交换两条 entry 顺序）→ `check_action` 返回 `PlanHashMismatch`（机制已在 `crates/soul-policy/src/hitl.rs`，WP11 只需把计划接上去并证明**接对了**：hash 算在用户看到的那份计划上，不是算在内部中间结构上——两者若不同，改显示层就能骗过批准）；
2. **hash 稳定性反向题**：同一棵 fixture 树扫两遍（目录遍历顺序可能不同），计划序列化后 hash 相等——entry 必须显式排序。不排序的实现会让「用户重启后重新打开同一份计划」被误拒，这是拒错方向的红；
3. 计划里 entry 的排序键不含未授权信息，且 hash 覆盖计划**全部**语义字段（漏掉 target 字段的 hash 等于没 hash——测试改 target 断言 hash 变）。

【红法】hash 算在 `format!("{:?}")` 上（Debug 输出不稳定）→2 红；hash 只盖 entry 数量→3 红。

---

## 三、壳接库（WP09 第二段第一步：壳接真 `SqlCipherStore`）

落点：`apps/desktop/src-tauri/src/{lib,commands}.rs` + `crates/soulcore/src/commands/shell.rs`。纪律来源：WP07 遗留 8 + WP09 取舍 8（`docs/STATUS.md`）——整个进程只能有一个 store 句柄。

### S-01 setup 只 open 一次；所有命令同一 `Arc<Mutex<SqlCipherStore>>`

【断言】

1. **源码级**：`include_str!` 检查（与 `apps/desktop/src-tauri/tests/command_surface.rs` 同款手法）——`SqlCipherStore::open` / `open_store` / `open_test_store` 字面量在 `src/lib.rs` 出现**恰好一次**（setup 路径），在 `src/commands.rs` 出现**零次**；`.manage(` 管理的是 `Arc<Mutex<SqlCipherStore>>`（或包着它的单一 Session 类型）。
2. **运行期**：`tauri::mock_builder()` + 真临时库（沿用 `tests/ipc_roundtrip.rs` 的真 `generate_context!()` 路数），命令 A 写、命令 B 读，跨命令可见——两个独立连接各写各的 WAL 时这条会以脏读/锁冲突的方式红。
3. **与采集共句柄**：若本单接了 collect 命令，`collect_start` 必须 clone 被 `manage` 的那个 Arc（`crates/soulcore/src/commands/collect.rs::share` 是包装入口），源码级断言 `commands.rs` 里没有第二个 `Arc::new(Mutex::new(`。

【红法】每个 command 里「就地开一下反正有缓存」→1 红；为测试方便在 command 里开第二个只读连接→1、2 红。

### S-02 open 的选择逻辑在 soulcore，不在壳

【断言】开库路径解析与 KeyProvider 选择（`TestKeyProvider` vs `DpapiKeyProvider`，`crates/soul-store/src/keys.rs`）属于安全决策，必须住在 `soulcore::commands`（新入口，形如 `open_session_store(app_data_dir)`）：源码级断言 `apps/desktop/src-tauri/src/` 全目录不出现 `TestKeyProvider` / `DpapiKeyProvider` / `KeyProvider` 字面量；`lib.rs` 的 setup 对 store 的参与仅限「拿目录、调 soulcore、manage 返回值」。

【红法】壳里写 `if cfg!(windows) { Dpapi } else { Test }`→红。这不是洁癖：密钥策略写在 UI 面，改 UI 的人就能改密钥策略，而 UI 面的 PR 没人按数据面标准审。

### S-03 无 DPAPI 时不得在 UI 文案声称已保护

【断言】`DpapiKeyProvider` 两个入口现仍返回 `KeyError::Unsupported`（`crates/soul-store/src/keys.rs`，`SECURITY.md` 明文「补齐前不要在 UI 上声称 Windows 的 KEK 已受保护」）。三道锁，仿 AC-22 云开关的既有结构：

1. **核心算，界面渲染**：快照（`crates/soulcore/src/commands/shell.rs` 的 `ConfigSnapshot` 或新的 `StoreStatus`）带 `kek_protected: bool`，由核心按实际 KeyProvider 计算；Rust 测试断言 TestKeyProvider 路径下它是 `false`，且没有任何配置组合能让它变 `true`（穷举，先例：`no_configuration_can_claim_the_cloud_is_available`）。
2. **文案跨语言逐字比对**：未保护提示文案是 soulcore 常量（仿 `CLOUD_NOT_YET_AVAILABLE_EXPLANATION` 与 `apps/desktop/src/contract.test.ts` 的逐字比对），TS 测试断言设置页在 `kek_protected=false` 时渲染的正是这个常量，界面自己编一句会红。
3. **禁词扫描**：扫 `apps/desktop/src/**` 的每个 `.ts/.tsx/.css/.html`（`contract.test.ts` 已有同款扫描器扫诊断词），「已受保护」「已加密保护」「DPAPI 保护」类字样不得以硬编码字面量出现——文案只能来自核心常量。

【红法】前端写死「你的密钥已由 Windows 保护」→3 红；核心为了「体验」把 `kek_protected` 默认 true→1 红。DPAPI 真补齐（P1-1）时改的是核心计算，三道锁全不用动——这是这个结构的意义。

### S-04 wrapper 函数体仍 ≤1 语句，三方名单咬合保持

【断言】`apps/desktop/src-tauri/tests/command_surface.rs` 的四条既有测试（`both_sides_name_the_same_commands` / `every_named_command_is_actually_a_command` / `no_command_exists_outside_the_list` / `the_command_layer_stays_thin`）对**新增的每个** store 命令继续绿：新命令必须同时进 `core.ts` 的 `COMMANDS`、`COMMAND_NAMES`、`#[tauri::command]` 注册三处，且函数体 ≤1 语句——`State<Arc<Mutex<SqlCipherStore>>>` 的取锁、错误映射全都放不下，只能转调 `soulcore::commands::*` 的一个函数。`ipc_roundtrip.rs` 对每个新命令补正反两例（合法请求过、伪造 origin 拒）。

【红法】在 wrapper 里 `let store = state.lock().unwrap(); store.xxx()` 两句→`the_command_layer_stays_thin` 红，正确做法是 soulcore 提供收 `&Arc<Mutex<SqlCipherStore>>` 的单函数。

---

## 四、实现者最容易犯的 10 个错

1. **假实现骗过存在性断言**。「起草返回了非空字符串」「计划预览非空」都能用常量满足。本仓库的解法从来是**可区分性反向断言**：语气不同则草稿不同（D-05.2）、同库第二条记忆的预览数字不同（WP02 先例）、计划引用得到真实文件（F-01 反向）。写测试先问：常量能过吗？
2. **常量 0 / 常量 true**。`third_party_rows: 0` 若是字面量就是没验过的谎——WP02 用只收 0 的构造函数 `zero_third_party_rows` 堵死（`crates/soul-store-api/src/research.rs`）。WP10 的「豁免只一次」、S-03 的 `kek_protected` 同理：值必须由「查出来再验证」的路径产生，不是写上去。
3. **泄漏检查跑在错误的字符串上**。对本地 `RedactedBody` 跑检查、而 wire 上是另一次序列化的产物——`mock_llm.rs` 特意保存原始字节并在注释里警告了这一点。一切 E1 断言对着 `RecordedRequest.body` 跑。
4. **重写 redactor / 泄漏检查器 / 审计链**。`RedactedBody` 无公开构造函数、`EgressPermit` 不可伪造、审计链字段由库覆盖，全是故意的。在 `soul-draft` 里自己写占位 replace、自己拼 `seq/prev_hash`，等于把三年的类型级保证换成一段没人审的字符串操作。发现类型「挡路」时，正确动作是读它的模块注释——挡的就是你正要做的事。
5. **HTTP/网络进错 crate**。`soul-draft` 直接依赖 reqwest、或给 `apps/desktop` 加 `tauri-plugin-http`。`deny.toml` + `crates/xtask/src/egress.rs` 会红；错上加错的反应是去改 `deny.toml` 或 `EXEMPT_CRATES`——那是绕审计。唯一出网点是 `soul_egress::send(&E1RequestPlan)`，没有第二个。
6. **把业务逻辑写进 TS**。「有没有 endpoint」「该不该豁免」「计划能不能批准」在前端判断。三道锁（eslint `no-restricted-imports`、`apps/desktop/src/contract.test.ts`、`command_surface.rs` ≤1 语句）就是为此而设；判断加不进 wrapper 时，答案是给 soulcore 加命令返回判断结果，不是把 wrapper 写长。
7. **第二个 store 句柄**。command 里就地 open、collector 线程自己 open、测试图省事再 open——两个连接各写各的 WAL（WP07 遗留 8 原话）。整个进程一个 `Arc<Mutex<SqlCipherStore>>`，setup 开一次。
8. **把 LLM 输出当结构化事实或指令**。从 mock 回复解析 evidence_ids 入库（D-07.4）、按回复里的工具调用形状「代办」（D-08.5）、把回复文本写进审计。模型回复与粘贴、导入行、文件名同级：`UntrustedText`，只能读，不能信。
9. **`soul-fileplan` 顺手写执行**。「apply 写好但没接线」「dry-run 用 tempfile 演练一遍」都命中 F-03 的源码自查——red line 是**无写 API**，不是「不调用写 API」。同族错误：扫描时落缓存文件（F-01 红）、用 `starts_with` 判授权（F-02.4 红）。
10. **拒错方向：把不稳定当安全**。plan hash 算在 Debug 输出或未排序遍历上（F-06.2），用户重启后同一份计划被误拒；atime 写进磁盘不变断言（F-01），CI 随机红。误拒与假警报不是「更安全」，它们训练用户无视拒绝——每一条拒绝断言旁边都要有一条「合法路径通过」的对照（先例：`e1_origin.rs::clearing_the_redirect_lets_the_same_request_through`）。

---

## 附：验收条与 AC 矩阵的对应

| 验收条 | AC（`docs/FORMAL_WORK_PROMPT.md`） |
|---|---|
| D-01 | 产品锁「只起草不发送」 |
| D-02 | AC-11 |
| D-03 | AC-12 |
| D-04 | AC-13 |
| D-05 | AC-17 |
| D-06 | AC-07 |
| D-07 | AC-16（+ AC-06 的摘要侧） |
| D-08 | AC-25 起草侧 |
| D-09 | AC-23 起草/摘要侧 |
| F-01～F-06 | AC-18（F-06 兼 AC-19 的计划侧复用） |
| S-01～S-04 | AC-26 前置 + WP09 功能视图/WP10 UI 的共同地基 |
