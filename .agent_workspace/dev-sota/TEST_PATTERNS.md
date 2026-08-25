MODEL: gpt-5.6-sol-xhigh-fast

# WP10 / WP11 可复制的测试手法

范围：只摘录现有仓库里的测试模式，并给 WP10、WP11 设计红灯门禁；不把底层构件已经通过的测试冒充新功能端到端。

## 1. 精确 E1 origin 与跨 origin 302

来源：`crates/soul-egress/tests/e1_origin.rs`

- `a_request_reaches_the_configured_origin_and_only_that_one`：同时启动两个 `MockLlm`，只用第一个的 `base_url()` 配置 `NetGuard`；经 `authorize_e1` → `E1RequestPlan::chat_completions` → `send` 后，断言配置端恰好收到 1 次、另一端为 0，并核对线上真实收到的 `POST /v1/chat/completions`。
- `a_cross_origin_redirect_is_refused_and_the_target_is_never_contacted`：首端 `set_redirect` 到第二端；必须得到 `EgressError::CrossOriginRedirect` 和 `ReasonCode::E1CrossOriginRedirect`，首跳为 1，跳转目标为 0。
- `a_redirect_to_another_port_on_the_same_host_is_still_cross_origin`：专测“同主机、不同端口”；origin 必须按 scheme/host/port 精确比较，不能只比 host。
- `clearing_the_redirect_lets_the_same_request_through`：反向控制。清掉 302 后同一请求成功，避免“客户端对所有请求都失败”也误过红线。
- `the_body_on_the_wire_carries_no_third_party_prose`：检查 `MockLlm::requests()[0].body` 的原始线上正文，不检查本地重新序列化的副本；同时核对固定系统指令位、`QUOTE_OPEN`/`QUOTE_CLOSE` 数据围栏和占位符。
- `with_no_endpoint_configured_there_is_no_permit_to_send_with`：`NetGuard::closed()` 必须在开连接前以 `E1NotConfigured` 拒绝，mock 计数仍为 0。

WP10 应复制“双 mock + 真实 loopback wire + 目标计数”的手法，但入口必须换成 `soul-draft` 的公开起草函数；只重新调用 `soul_egress::send` 不能证明 WP10 已正确装配。

## 2. redactor 泄漏与一次性豁免

来源：`crates/soul-policy/tests/redactor_leakage.rs`

- `no_third_party_body_from_the_corpus_survives_the_default_path`：用 `LeakageFixture` 全语料逐条走 `Redactor::redact_for_e1`，`LeakageChecker::assert_clean` 后还断言确实出现 `THIRD_PARTY_PLACEHOLDER`、计数为 1；不能靠返回空串过关。
- `the_same_corpus_leaks_when_the_redactor_is_bypassed`：同一语料的正向控制，证明 checker 真会报漏，避免 vacuous pass。
- `names_and_accounts_are_placeheld_even_inside_the_users_own_words`：第三人姓名、邮箱、手机号、handle 即使出现在 owner 文本中也要占位。
- `mixed_subject_prose_is_treated_as_third_party`：`SealedSubject::Mixed` 按第三人处理。
- `the_research_path_emits_no_third_party_line_at_all`：研究路径是“删行”而不是放第三人占位行；同时断言实际排除了 1 条。
- `a_decomposed_spelling_is_caught_by_the_composed_corpus`：NFD/NFC 两边都归一，先证明原文会被 checker 抓到，再证明 redactor 后干净。

来源：`crates/soul-policy/tests/redactor_exemption.rs`

- `a_confirmed_exemption_carries_exactly_one_original_and_the_next_draft_does_not`：默认干净；二次确认后仅指定 turn 原文出现，另一第三人 turn 仍隐藏；同一 `Redactor` 的下一次普通调用与豁免前完全相同。
- `declining_the_second_confirmation_yields_no_exemption`：`ExemptionRequest::confirm(false)` 返回 `None`。
- `identifiers_stay_placeheld_inside_an_exempted_turn`：豁免的是一条消息正文，不是消息里的姓名/电话。
- `an_exemption_for_an_absent_turn_placeholds_everything`：turn id 不匹配时不得宽松放行。
- `the_redactor_carries_no_memory_between_calls`：用过豁免的实例与全新实例下一次结果一致。
- `the_research_path_has_no_exemption_entry_point`：读回源码，证明 research 只有一个公开入口且没有 exemption 参数；并用“必须找到预期函数”的正向控制防止文件移动后空过。

WP10 必须在完整起草入口重测一次性语义，因为“redactor 本身无状态”不等于上层 service/session 没有缓存豁免。

## 3. 注入内容必须保持 `UntrustedText`

来源：`crates/soul-policy/tests/injection.rs`

- `import_lines`、`pasted_lines`、`file_names`、`all_channels`：统一把 import/paste/filename 语料转成 `UntrustedText`，并保留 `ExternalChannel` 来源。
- `nothing_in_any_corpus_can_authorize_an_action`：每条外部内容都以 `RequestOrigin::ExternalContent` 进 `check_action`；必须是 `UnknownAction` 或 `ExternalContentNotAuthority`，且 `TokenIssuer::issued_count() == 0`。
- `no_url_from_any_corpus_can_be_reached`：提取语料中的 URL，分别用 closed guard 和配置了其他 origin 的 guard 拒绝；另由 `the_url_extractor_finds_what_the_corpus_planted` 证明测试确实找到了 URL。
- `pasted_injection_reaches_the_draft_as_placeheld_material`：粘贴内容作为第三人 `Turn`，结果只能是 `THIRD_PARTY_PLACEHOLDER`，URL 不得进入请求正文。
- `a_file_name_is_scrubbed_like_any_other_external_string`：文件名仍是字符串，不是 action/path 授权；标识符被 scrub，`ActionKind::parse` 不得产生动作。
- `the_scanner_recognises_what_the_corpora_are_doing`：`scan` 只供审计信号，不承担安全边界；普通文本应无信号。
- `untrusted_text_cannot_be_formatted_into_an_instruction`：源码断言 `UntrustedText` 没有 `Display`。调用点必须显式写 `.as_str()`，便于审查其是否进入了数据位。

WP10/WP11 不应把 `injection::scan` 当过滤器；即使 scanner 漏报，类型、HITL 与 origin/授权根边界仍必须拒绝执行。

## 4. 磁盘不变与源码无写 API

来源：`crates/soul-store/tests/research_preview.rs`

- `the_preview_reports_real_rows_and_excludes_the_third_party_ones`：fixture 有第三人专属 kind/hour；断言 `third_party_rows_excluded > 0`、`candidate_total == excluded + emitted`，并确认第三人专属 bucket 不在结果中。这样 `third_party_rows: 0` 不能靠写常量伪造。
- `the_preview_validates_against_the_frozen_export_contract`：输出先过冻结 schema，再检查 `third_party_rows == 0`、`written_to_disk == false`，最后把完整渲染结果交给 `LeakageChecker`。
- `a_preview_leaves_no_new_file_behind`：在同一临时目录中先完成 seed/flush，再取目录快照；连续调用 3 次公开 preview 后快照必须完全相同。
- `the_research_module_cannot_reach_the_filesystem`：`include_str!` 读回生产模块，逐项禁止 `fs::write`、`File::create`、`OpenOptions`、`std::io::Write` 等写入口；同时断言源码确实含目标函数，防止扫描错文件。
- `entries`：当前实现只比较“文件名 + byte length”。WP11 可以复制结构，但应加强为递归快照，至少含相对路径、类型、内容 SHA-256、长度、mtime；否则同长度覆写抓不到。

WP11 的运行时测试必须创建真实临时目录和真实文件，然后调用公开扫描/预览 API。禁止手工构造 `ScanEntry`、手工 `insert` 计划行或数据库行来冒充“扫描过磁盘”。

## 5. `soul-testkit::leakage` 公共 API

来源：`crates/soul-testkit/src/leakage.rs`

- 常量/函数：`DEFAULT_MIN_NGRAM`（8）、`normalize`（NFC）。
- 类型：`LeakageKind::{ThirdPartyNgram, KnownIdentifier}`、`LeakageFinding`、`CorpusEntry`、`LeakageCase`、`LeakageFixture`、`LeakageChecker`。
- 构造：`LeakageChecker::new`、`Default`、`with_min_ngram`、`from_fixture`。
- 录入：`add_third_party_body`、`add_known_identifier`。
- 检查：`inspect`（返回去重排序 findings）、`is_clean`、`assert_clean`；`min_ngram` 可用于边界断言。
- 根 re-export 只有 `LeakageChecker`、`LeakageFinding`、`LeakageKind`；`LeakageFixture` 等需从 `soul_testkit::leakage` 引入。标准 fixture 入口是 `soul_testkit::fixtures::leakage_fixture()`。

应同时保留“预期干净”和“故意绕过后预期泄漏”的控制组。仅写 `assert_clean` 会让空输出或空 corpus 误过。

## 6. AC-07 档案侧语气锁；起草侧要对称

来源：`crates/soul-profile/tests/correction_lock.rs`

- `a_corrected_axis_is_not_overwritten_by_a_later_stronger_inference`：用户纠正后轴锁定；更强 inference 返回 `RefusedAxisLocked`，轴仍引用 correction evidence，但被拒 inference 本身仍入库供审计。
- `locking_one_axis_does_not_lock_the_others`：锁的粒度是单字段，不是整个档案。
- `a_voice_field_the_user_set_is_what_the_profile_read_returns`：`set_voice(Reserved)` 后 `read_voice` 返回用户值；随后 `suggest_voice(Direct)` 返回 false 且值不变；未被用户碰过的 register 仍可接受 suggestion。

WP10 的对称测试不能只再次测试 `read_voice`。应在同一个 draft service/store 上：先起草一次，调用 `set_voice` 改值，再起草一次；断言第二次的模板结果或线上 prompt 立即使用新值。再用 `suggest_voice` 尝试覆盖并起草第三次，结果仍使用用户值。这会抓到 WP10 启动时缓存 `VoiceProfile` 的错误。

## 7. Tauri command surface

来源：`apps/desktop/src-tauri/tests/command_surface.rs`

- `both_sides_name_the_same_commands`：解析 `apps/desktop/src/core.ts` 的 `COMMANDS`，与 Rust `COMMAND_NAMES` 排序后完全相等。
- `every_named_command_is_actually_a_command`：每个名字都必须有紧邻的 `#[tauri::command]`，并出现在 `src/lib.rs` 的 builder 注册里。
- `no_command_exists_outside_the_list`：反向扫描所有 `#[tauri::command]`，禁止未入清单的隐藏 IPC。
- `the_command_layer_stays_thin`：每个 wrapper 函数体最多 1 个非空、非注释源码行，迫使业务逻辑留在 `soulcore`/业务 crate。

新增 draft/fileplan command 时必须同步四处：`src-tauri/src/commands.rs` wrapper、`COMMAND_NAMES`、`src-tauri/src/lib.rs` 注册、`apps/desktop/src/core.ts::COMMANDS`。现有计数器按“行”而非 Rust AST 计数；实现者应写单一转发表达式，不要把多个分号语句挤到同一行规避门禁。`apps/desktop/src/contract.test.ts::界面用的命令名和 src-tauri 注册的一模一样` 还会从 TS 侧再咬一次。

## 8. 可复用 fixtures

### WP10

- `fixtures/leakage/third_party_unicode.json`：直接用 `fixtures::leakage_fixture()`。覆盖 8/7 scalar 边界、短回复已知下限、NFC↔NFD、ZWJ emoji、姓名/handle/手机/邮箱；最适合 AC-12 wire body。
- `fixtures/import/soul-import-v1/three_partners.jsonl`：可用于“从真实 import 结果选对话/证据再起草或摘要”。注意文件名不可当断言：当前内容有 9 条 `third_party` 消息、4 个不同第三人 sender id，不是恰好 3 个。
- `fixtures/import/soul-import-v1/injection_lines.jsonl`：5 条第三人注入消息；用 `read_jsonl` 取 `text` 后包成 `UntrustedText`，适合 import→draft 完整链。
- `fixtures/injection/paste_injection.txt`：用 `read_text`、去空行和 `#` 注释后包成 `UntrustedText`；适合粘贴→draft。
- `fixtures/denylist/diagnostic_terms.txt`：人事摘要运行时必须经 `soul_policy::assert_non_clinical`；不要只靠源码 audit，因为模型/模板输出是运行时文本。

### WP11

- `fixtures/injection/filenames.txt`：可复用，但其中含 Windows 保留名、路径分隔符和非法字符，不能假装每行都能在每个平台真实创建。跨平台 AC-18 测试应从中挑明确可创建的 portable 子集，创建真实文件后走公开 scan；其余行可做纯解析/`UntrustedText` 单测，并明确不宣称完成了磁盘扫描。
- 同一文件里的 `$(curl ...)`、反引号、`..`、绝对路径、设备名、Unicode/零宽字符分别适合测“名称只是数据”“路径逃逸拒绝”“平台边界可读失败”。
- `fixtures/schemas/invalid/export-manifest/third_party_rows_nonzero.json` 与 `research_preview_written_to_disk.json` 是 schema 反例，不是 WP10/WP11 的行为 fixture；不要拿 schema 拒绝代替真实运行时断言。

结论：WP10 可直接复用第三人泄漏、import 注入、paste 注入和 denylist 四组语料；WP11 可复用 filename 注入语料，但 AC-18 仍必须由临时目录中的真实文件驱动。

## 建议新增的 WP10 测试

以下文件都放 `crates/soul-draft/tests/`。函数名可随最终 API 调整，但调用必须从 WP10 的公开入口开始。

### `draft_wire.rs`

1. `configured_origin_is_the_only_wire_destination`：完整 `draft_reply` 后配置 mock 为 1、无关 mock 为 0；原始 `RecordedRequest.body` 经 fixture `LeakageChecker::assert_clean`，且有第三人占位。
2. `cross_origin_redirect_never_reaches_target`：经完整起草入口得到 `E1CrossOriginRedirect`；首端 1、目标端 0。
3. `no_endpoint_uses_template_without_any_request`：无 endpoint 时返回非空、确定性的模板降级；两个 mock 均为 0，且结果标明本地 fallback 而非伪装模型结果。

### `draft_voice.rs`

1. `user_voice_change_affects_the_very_next_draft`：同一 store/service 先后用两种用户语气起草，第二次可观察 prompt/模板立即变化并包含最新封闭枚举语义。
2. `inferred_voice_cannot_displace_user_voice_in_a_draft`：`suggest_voice` 返回 false 后再次起草，仍与用户值对应。
3. `draft_reads_only_voice_not_the_whole_profile_as_prose`：线上正文不得出现 trait inference 的展示文本/证据说明，防止把整个 profile 塞入 prompt。

### `draft_exemption.rs`

1. `one_shot_exemption_affects_only_one_turn_and_one_call`：默认请求 clean；豁免请求只含指定正文；下一请求恢复 clean，且与默认请求的 redacted material 相同。
2. `exempted_body_still_hides_names_and_accounts`：原文片段可出现，但姓名/手机号/handle 仍不得出现。
3. `declined_or_mismatched_exemption_changes_nothing`：拒绝二次确认或 turn id 不匹配时，请求体保持默认占位。

### `draft_injection.rs`

1. `paste_and_import_injections_remain_untrusted_data`：加载两组 fixture，全部经 `UntrustedText` 和完整 draft；系统 message 必须恒等于固定指令，外部文字不得进入 instruction 位。
2. `an_injected_url_is_never_contacted`：把注入 URL 指向第二个 loopback mock；即使第一个 mock 是合法 E1 endpoint，第二个请求数仍为 0。
3. `external_content_mints_no_action_or_token`：处理 corpus 后不得产生 tool/action plan，`TokenIssuer::issued_count()` 为 0，并有 `injection.blocked` 审计信号而非执行结果。

### `people_summary.rs`

1. `every_summary_claim_has_resolvable_evidence`：至少 seed 一条真实 evidence；每条输出 `evidence_ids` 非空且逐个能从 store 解引用。
2. `unsupported_claim_is_not_persisted`：无 evidence 的候选结论被拒，store 中 inference/summary 行不增加。
3. `summary_is_non_clinical_and_has_local_fallback`：模型和本地统计结果都过 `assert_non_clinical`；无 endpoint 时仍有确定性统计摘要且 mock 为 0。

## 建议新增的 WP11 测试

以下文件都放 `crates/soul-fileplan/tests/`。每个行为测试都走公开 scan/preview，不手工塞扫描结果。

### `authorization.rs`

1. `authorized_root_scans_real_entries`：在临时 A 创建至少两层真实文件，授权 A 后调用 scan，返回项与磁盘相对路径集合一致且非空。
2. `unauthorized_root_is_refused_before_observation`：同级临时 B 未授权；scan/preview B 返回明确 authorization reason，且错误/审计不泄漏 B 的文件名或正文。
3. `symlink_or_parent_escape_cannot_leave_the_root`：A 内链接或 `..` 指向 B 时必须拒绝/不跟随，结果中绝无 B 项；Windows reparse/junction 另加平台专项。

### `readonly_disk.rs`

1. `scan_and_preview_leave_the_authorized_tree_byte_for_byte_unchanged`：真实 A 的递归强快照（路径、类型、长度、SHA-256、mtime）在多次 scan+preview 前后完全相等。
2. `refused_b_tree_is_also_unchanged`：对 B 的拒绝调用前后同样比较强快照。
3. `production_source_has_no_write_surface`：扫描 `src/`，禁止 create/write/append/truncate/remove/rename/copy/set_permissions 等入口和 `execute/apply` 公共函数；同时正向断言预期 scan/preview 函数确实存在。

### `plan_preview.rs`

1. `preview_is_derived_from_the_real_scan`：创建不同扩展名/大小/子目录的文件，公开 scan 后公开 preview；每个计划项都可追溯到 scan entry，且计划非空，不能返回固定 fixture。
2. `changing_a_real_file_changes_the_preview_or_plan_hash`：第一次完成后由测试夹具修改文件，再重新 scan；摘要或 `PlanHash` 必须变化，证明不是常量计划。
3. `preview_exposes_no_execute_capability`：返回类型/公开 API 不含 execute token、write action 或“已执行”状态；`CapabilityScope::FileWrite` 即使伪造也仍由现有 HITL 以 `WriteNotImplemented` 拒绝。

### `filename_injection.rs`

1. `portable_injection_filenames_are_scanned_as_data`：从 `filenames.txt` 选 portable 子集并真实创建文件；scan/preview 后名称仍是数据，不产生 `ActionKind`、shell 或 URL fetch。
2. `filename_urls_never_receive_a_connection`：可创建文件名中嵌入第二个 mock 的编码后 URL，合法 scan/preview 全程该 mock 为 0。
3. `the_fixture_and_real_scan_controls_are_nonempty`：断言语料攻击行数和实际成功创建/扫描的 portable 行数都大于约定下限，避免过滤器把所有案例跳过后空过。

## 两条不可妥协的反作弊控制

1. 不要用常量“第三人为 0”。先放入可观测第三人输入，再断言实际排除数 `> 0`、总数守恒、第三人专属 sentinel 不出现，并用 `LeakageChecker` 检查最终文本；只有输出字段等于 0 没有证明力。
2. 不要手工 `insert` 或直接构造 entry 冒充扫描。AC-18 必须由测试创建真实目录/文件，经授权配置调用生产 scan，再由 scan 结果生成 preview；手工构造只可作为序列化单测，不能标成扫描验收。
