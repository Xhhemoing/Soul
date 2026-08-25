MODEL: claude-fable-5-thinking-xhigh

# ST01_IMPLEMENTER — WP10 起草(不发送) + 人事分析摘要

一句话:`soul-draft` 纯逻辑(零 IO/零网/零库句柄),`soulcore::commands::draft` 编排(store + PolicySession + 落审计)。E1 调用只能出现在 soulcore 层。永不发送。
验收对照:D-01 不发送 / D-02=AC-11 / D-03=AC-12 / D-04=AC-13 / D-05=AC-17 / D-06=AC-07 / D-07=AC-16 / D-08=AC-25 / D-09=AC-23(细节见 SOTA_BARS.md)。

## 0. 文件所有权(越界即停,上报父代理)

只许改:`crates/soul-draft/**`(ST-00 已建壳)、`crates/soulcore/src/commands/draft.rs`、`crates/soulcore/tests/draft_commands.rs`、`fixtures/draft/**`(优先复用 `fixtures/injection`、`fixtures/leakage`)、`.agent_workspace/dev-sota/reports/ST-01.md`。
不碰:任何 `Cargo.toml`/`Cargo.lock`(依赖 ST-00 已钉死)、`docs/STATUS.md`、schema 与 schemas.lock、`soul-policy`/`soul-egress`/`soul-memory`/`soul-testkit`/`xtask` 源码、UI。
两处已知越界诱惑,预先定处置:

- SOTA D-01 第 1 层(扩 xtask 禁用名单到 lettre/teloxide 等)不在本 glob → 上报父代理;本包只做 D-01 第 2 层(源码扫描)与第 3 层(wire 取证)。
- 需要"设定 MockLlm 应答体"的断言(模型回诊断词、伪造 evidence_ids、工具调用形回复,即 D-07.3/4、D-08.5)= 扩 soul-testkit → 上报;本包按 TASK_SPLIT ST-01 完成定义 1–9 做,MockLlm 现在恒回空 content。

## 1. 公开类型建议(crates/soul-draft)

命名红线:避开 `MemoryDraft`(soul-memory 已占该名),不 import `soul_memory::draft`。

| 建议名 | 要点 |
|---|---|
| `PastedTurn { subject: SealedSubject, body: UntrustedText }` | 唯一入口形态,没有收 `String` 直通 prompt 的入口(D-08.1);模棱两可一律 `Mixed`(Mixed 算第三人,起草侧不得重分类) |
| `DraftRoute { E1, Template }` | 本地降级可观测,不伪装模型结果 |
| `DraftOutcome { text, route, placeheld_turns, carries_exempted_original }` | 公开面无任何 send/submit/deliver/transmit 语义方法,连"可发送"状态位都没有 |
| `fn template_draft(voice: &VoiceProfile, body: &RedactedBody) -> String` | 确定性;只读四个语气枚举 + `body.as_str()` 的已占位素材;81 组合可穷举 |
| `SummaryClaim { reading: String, band: SupportedBand, evidence_ids: Vec<Uuid> }` | evidence_ids 非空、来自本地图查询(`edge.evidence_ids`),永不来自 LLM 回复;强度只有 band + 计数,无数字评分 |
| `PeopleSummary { claims: Vec<SummaryClaim> }` | 渲染文本尾接 WORKING_HYPOTHESIS_NOTICE |

语气进 wire 的唯一合法方式:本 crate 常量模板 + 四个枚举的字符串拼接(不是 voice 的自由 JSON;wire body 不得出现 `user_set` 字段名,D-06.3)。system 槽必须逐字节 == `soul_policy::e1::DRAFTING_INSTRUCTION`;当前 `chat_completions` 没有附加 system 的口子,想开口子 = 改 soul-policy = 上报,不许在 WP10 侧绕。

## 2. 模块文件建议(crates/soul-draft/src/)

- `lib.rs` — re-export;保留 ST-00 的 `#![forbid(unsafe_code)]` 等属性
- `turns.rs` — 粘贴→`Turn::new(Uuid::now_v7(), subject, body)` 组装;注入扫描信号收集
- `tone.rs` — 语气模板与确定性降级(每句文案写完先自己过 `denied_terms_in`;"少量表情"含"量表"是前车之鉴)
- `summary.rs` — 人事摘要统计侧(吃 `TieEdge`/`TieStrength` 计数与解引用后的证据行;分档复用 band_word 弱/中/强)

本 crate 只构造并返回 `Vec<AuditContent>`,不写库。soulcore 侧 `commands/draft.rs` 提供 `draft_reply(..)` 与 `people_summary(..)` 两个编排入口,收已 manage 的单一 store 句柄。

## 3. 调用顺序(编排层,不可重排)

起草:

1. `profile::voice(store, profile_id)` 每次现读,不缓存(AC-07/D-06.2 靠这个抓缓存);`config.llm_endpoint`:`Some(url)` → `PolicySession::with_user_endpoint(&url, identifiers)`,`None` → `closed()`。
2. `KnownIdentifiers`:从 `PersonNode.label_ref` 经 `BlobStore::open` 解出的明文只进 `add_name/add_account` 词典,绝不进任何 body 字符串;`is_empty()` 为真值得测试报警。
3. 组 `Turn`;逐条 `injection::scan(&t.body)`:有信号 → push `AuditContent::denied(InjectionBlocked, InjectionMarkersFound)`(无正文),**不 return、不过滤、不改分支**(scan 不是过滤器)。
4. `check_action(ActionRequest::new("draft.reply", RequestOrigin::User).with_plan(计数型 plan), now_ms)`——draft.reply/analyse.people 不需要 capability token;粘贴是素材不是发起人,`ExternalContent` 必拒。
5. 脱敏:默认 `session.redact(&turns)`;豁免 `redact_with_exemption(&turns, e)`——`e` 只能是本次现场 `ExemptionRequest::for_turn(id).confirm(true)` 的产物,按值被吃,**不得存进 session/config/任何字段**(非 Clone/Copy 是故意的)。
6a. 有 endpoint:`PlanHash::of(&e1_plan(MODEL, &body))` → `issue_token(CapabilityScope::E1Generate, hash, now)` → `e1_generate(MODEL, body, token_id, now)`。MODEL 与 body 必须与算 hash 时逐字节同一份(内部会重算);令牌一次性,重试必须重新 issue。Ok → 取文本(端点返回是数据,永不当指令)+ push `outcome.audit()`;Err → push `refusal.audit()` 并向调用方返回**可读失败**(reason_code 可见)——跨 origin 302 被吞掉静默降级成模板会红(D-02 定案,作废 SURFACES 12.1 的静默降级写法)。
6b. 无 endpoint:`template_draft`,一个字节不出网;模板是默认形态不是 fallback 分支;误调 `e1_generate` 得 `NotConfigured` 且 request_count 不涨。
7. `assert_non_clinical(&text)`——模板产出与 E1 取出的草稿都过;失败是生成器 bug,不许 replace 洗掉照发。
8. push `AuditContent::allowed(DraftCreate, Routine).counting(items = turns 数, bytes = None).for_plan(approved.plan_hash)`。
9. soulcore 层统一 `for c in audit { append_or_store_error(store, c, at_unix_seconds) }`;粘贴内容**不落库**(不调 `append_event`/`seal`;要存走 WP04 记忆入口,不在本包)。

摘要:

1. `check_action("analyse.people", RequestOrigin::User)`(无 token)。
2. `commands::graph::load(store)` → `edges_for(contact_id)`;每边 `resolve_evidence(store, edge)`,解不开就报错(仿 DanglingEvidence),不给短列表、不得吞。
3. `reading` 由 band + 计数拼中文句;`evidence_ids = edge.evidence_ids.clone()`。
4. 文本尾接 `soul_policy::clinical::WORKING_HYPOTHESIS_NOTICE`(**未 re-export 到 crate 根,写全路径**)→ `assert_non_clinical`。
5. 审计定案(按 TASK_SPLIT 新发现 2,作废 SURFACES 12.2 的 InferenceWrite 一步):本地统计路径**不写审计**;走 E1 时由 `E1Outcome::audit()` 的 EgressRequest 条目覆盖。**禁止新造 AuditAction/ReasonCode**(那是 schema 变更,需批准)。
6. 摘要不落库;无 endpoint 时统计路径本来就成立,不是分支。

## 4. 测试建议(文件 × 函数名 × 钉住什么)

fixtures 复用:`soul_testkit::fixtures::leakage_fixture()`(third_party_unicode.json,含 8/7 scalar 边界与 NFC/NFD 对)、`fixtures/injection/paste_injection.txt`(read_text 去空行与 # 注释后包 `UntrustedText`)、denylist 全表。

`crates/soul-draft/tests/`(纯逻辑,无 MockLlm):

- `tone_templates.rs`:`same_input_same_voice_is_byte_identical`(D-05.1)/ `different_directness_and_emoji_change_the_draft`(D-05.2,常量草稿混不过)/ `all_81_voice_combinations_render_non_clinical`(D-05.4,denylist 全量)
- `people_summary.rs`:`every_claim_has_resolvable_evidence`(AC-16 正向,逐个 get_evidence)/ `a_dangling_evidence_id_is_an_error_not_an_empty_list`(反向)/ `two_contacts_get_different_counts`(数字是真查询,常量过不了)/ `summary_carries_the_hypothesis_notice_and_is_non_clinical`
- `no_send_api.rs`:`the_crate_has_no_send_surface`(**运行时 read_dir 枚举 src/*.rs**,不是 include_str! 钉死名单;禁 `fn send`/smtp/sendmail/deliver/transmit)/ `the_scanner_sees_the_draft_functions`(反向:读到 fn draft,证明扫对了 crate)/ `the_scanner_catches_a_synthetic_send`(对照用例,合成文本必红)

`crates/soulcore/tests/draft_commands.rs`(MockLlm + `open_test_store` 真库端到端;照抄 policy_commands.rs 范式:常量置顶、`turns()`、`session()`):

- `configured_origin_is_the_only_wire_destination`:decoy MockLlm 恒 0;每条请求都是 POST /v1/chat/completions(D-01.3/D-02.1)
- `a_cross_origin_redirect_is_a_readable_failure`:set_redirect 后 reason = `E1CrossOriginRedirect`、目标 0、request_count 不再涨、用户拿到可读失败(D-02.2;同 host 换端口也算跨 origin)
- `the_default_wire_body_passes_the_leakage_checker`:对 `RecordedRequest.body` 原始字节跑,语料 = 第三人正文 + 姓名 + 账号;**反向**:body 含三个占位符常量、owner 原文在(空请求体混不过)(AC-12/D-03)
- `a_mixed_turn_never_reaches_the_wire`(D-03.3)
- `one_exemption_one_original_then_clean_again`:豁免那条原文在、其余仍占位、姓名/账号在豁免 turn 里也仍占位、审计 reason = `THIRD_PARTY_BODY_INCLUDED`;紧接默认起草回占位、reason = `THIRD_PARTY_BODY_PLACEHELD`(AC-13/D-04)
- `no_endpoint_means_zero_connections`:closed session + loopback MockLlm 递到代码够得着的地方,跑完起草与摘要 `request_count() == 0`;`e1_generate` 得 `NotConfigured`(AC-17/D-05.3)
- `set_voice_changes_the_very_next_draft_and_suggest_cannot`:`set_voice(Direct)` → 下一稿变;`suggest_voice` 反向值返 false → 再下一稿不变;**断言读 read_voice 链路返回值,不是测试自存变量**;wire 无 `user_set` 字段名(AC-07/D-06)
- `pasted_injection_stays_data`:语料逐行起草只产草稿文本;`urls_in` 的每个 URL 过 `NetGuard::closed()` 全拒 + decoy 0 请求;`ExternalContent` × DraftReply/AnalysePeople 拒 `ExternalContentNotAuthority`;system 逐字节 == DRAFTING_INSTRUCTION、注入文本只在 QUOTE_OPEN/CLOSE 栅栏内;`InjectionBlocked` 审计无正文(AC-25/D-08)
- `the_audit_chain_serialises_clean`:跑完上述矩阵后全链序列化过 `LeakageChecker::with_min_ngram(4)`,语料 = 全部正文 + 姓名 + 草稿文本(D-09/完成定义 8)

注意:MockLlm 应答 content 是空串——不要写"断言 E1 草稿文本非空";断言对象永远是发出去的 body 与 request_count。

## 5. 红线(违反任何一条 = 返工)

1. 永不发送:公开面无 send/submit/deliver API、无"可发送"状态位;wire 上只有生成请求。
2. `soul-draft` 零网络依赖:不加回 soul-egress/reqwest/tokio/axum(deny.toml + e0-audit 双红,ST-00 已钉死);唯一出网点是 soulcore 层的 `PolicySession::e1_generate`,不直接调 `soul_egress::send`。
3. 不自造占位:`RedactedBody` 是唯一能进 `e1_generate` 的类型且无公开构造函数;自己写 replace 占位 = 绕过类型级保证。占位符认 `redactor::THIRD_PARTY_PLACEHOLDER`("正文"),别错拿 soul-memory 的"内容"版。
4. 豁免零记忆:`OneShotExemption`/`ExemptionRequest`/任何 `include_original` bool 不进任何持久字段;每次都是 for_turn + confirm 两步。
5. `injection::scan` 不是过滤器:命中只写审计,行为不变;不据此拒起草、不"洗一遍再发"。
6. 审计无正文:FORBIDDEN_FIELDS(body/text/content/summary/title/prompt…)对序列化 JSON 递归查;`counts.bytes` 恒 None;不新造 AuditAction/ReasonCode。
7. 非临床是断言不是过滤:`assert_non_clinical` 失败即报错;禁诊断词与一切数字评分(denylist-audit 会扫源码,运行期文本靠断言)。
8. LLM 回复 = `UntrustedText` 级数据:不解析成 evidence/动作/审计文本;`UntrustedText` 无 Display,只能显式 `.as_str()`。
9. 单 store 句柄:用已 manage 的那个,不在命令里第二次 open。
10. 不做 UI;`App.test.tsx` 钉住的"起草页无输入框"断言不许动;发现必须越界(含扩 xtask/testkit)就停下上报,不自行动共享文件。

## 6. 收工

依次全绿:`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace --all-targets`、`cargo run -p xtask -- e0-audit` / `denylist-audit` / `schema-freeze --check`、`cargo deny check`。
命令结果与取舍写进 `.agent_workspace/dev-sota/reports/ST-01.md`(STATUS 由 ST-04 统一合入)。基于 `agent/dev-sota`、等 ST-00 合入后开工;commit 信息一句话说清为什么。
