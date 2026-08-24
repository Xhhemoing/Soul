MODEL: claude-fable-5-thinking-xhigh

# SOTA 复核 — WP10 起草（不发送）+ 人事分析摘要

复核对象：`agent/dev-sota` @ `8676dbd`（`eb74904` 落地 WP10）。
读过的全部代码：`crates/soul-draft/{src,tests}/**`、`crates/soul-draft/Cargo.toml`、
`crates/soulcore/src/commands/draft.rs`、`crates/soulcore/tests/draft_commands.rs`，
以及作为判定依据的 `soul-policy/src/{redactor,e1,clinical,hitl,injection,audit}.rs`、
`soulcore/src/commands/policy.rs`、`soul-testkit/src/{mock_llm,leakage}.rs`、
`xtask/src/egress.rs` + `xtask/tests/self_test.rs`、`deny.toml`、两份 fixture。

本机验证：`cargo test -p soul-draft --all-targets` 11 通过、
`cargo test -p soulcore --test draft_commands` 11 通过、
`cargo tree -p soul-draft -e normal` 无任何 HTTP/网络 crate。

## 1. 结论

**ship-with-must-fix。**

实现本体是真的：redaction 走的是 `soul-policy` 的唯一路径（`RedactedBody`
无公开构造），wire 断言全部对着 `RecordedRequest.body`（mock 的 `content`
恒空，没有一条断言碰模型回复文本），豁免按值消费且有类型级+运行期双证，
模板不是常量（81 种组合互异），摘要数字来自真 `soul_graph::rebuild` 的真库行
（两个人计数不同），粘贴不落库有正反对照。没有发现假实现。
四条 must-fix 里三条是缺失的测试/审计字段（小补丁），一条是 D-01 第 1 层
整层缺席（xtask 名单，ST-01 无权触碰、已如实上报，见 §3.1）。

## 2. D-01..D-09 逐条判定

| 条 | 判定 | 证明它的测试 |
|---|---|---|
| D-01 永不发送（三层） | **partial** | 层2：`no_send_api.rs::the_crate_has_no_send_surface` + `the_scanner_sees_the_draft_functions` + `the_scanner_catches_a_synthetic_send`；层3：`draft_commands.rs::configured_origin_is_the_only_wire_destination`；**层1（依赖图禁消息类 crate + self_test 对照）：MISSING** |
| D-02 精确 origin / 跨 origin 302 拒绝 | **pass** | `a_cross_origin_redirect_is_a_readable_failure`（含清 redirect 后同请求通过的对照、decoy 恒 0、`E1CrossOriginRedirect` 可读、同 host 换端口断言）；`configured_origin_is_the_only_wire_destination` |
| D-03 默认体过 LeakageChecker | **pass** | `the_default_wire_body_passes_the_leakage_checker`（fixture 语料 + 反向占位符/自有正文断言）；`a_mixed_turn_never_reaches_the_wire`（Mixed 按第三人，两个占位符计数） |
| D-04 一次性豁免 | **partial** | `one_exemption_one_original_then_clean_again`（断言1、2 全过；断言3 类型级过——`OneShotExemption` 非 Clone/Copy 且注释在测试里；**运行期"审计记 turn_id"：MISSING**，见 §3.3） |
| D-05 无 endpoint 确定性模板 | **pass** | `tone_templates.rs::same_input_same_voice_is_byte_identical`、`different_directness_and_emoji_change_the_draft`（81 组互异）、`all_81_voice_combinations_render_non_clinical`（全表扫描）；`draft_commands.rs::no_endpoint_means_zero_connections`（loopback decoy 恒 0 + 直接 `e1_generate` 拒 `E1NotConfigured`） |
| D-06 AC-07 用户 voice 即时生效 | **pass** | `set_voice_changes_the_very_next_draft_and_suggest_cannot`（wire 上「适中」→「直接」、`suggest_voice` 返 false 后仍是用户值、回读走 `profile_commands::voice` 链路、wire 无 `user_set`）；`the_tone_line_carries_four_words_and_no_internal_state` |
| D-07 人事摘要证据链 | **partial** | 7.1/7.2：`people_summary.rs::every_claim_has_resolvable_evidence` + `a_dangling_evidence_id_is_an_error_not_an_empty_list`（三种拒绝，不给短列表）；7.5：`two_contacts_get_different_counts` + `no_endpoint_means_zero_connections`；7.3 渲染侧：`summary_carries_the_hypothesis_notice_and_is_non_clinical`，**端点回诊断词被拒：MISSING**；7.4 结构上成立（`summarize` 只抄边上的 id，`answer_text` 只取 `content`），**对抗性测试：MISSING**（见 §3.4） |
| D-08 AC-25 粘贴注入 | **partial** | 8.1/8.2/8.4：`pasted_injection_stays_data`（system 槽与 `DRAFTING_INSTRUCTION` 逐字节相等、材料在栅栏内、每个 URL 过 `NetGuard::closed` + `authorize_e1` 双拒、decoy 恒 0、豁免行落在栅栏内的对照）；8.3：本测试只跑 2/8 个动作，`ActionKind::ALL` 全矩阵在先例 `soul-import/tests/injection_is_data.rs`；**8.5 工具调用形回复：MISSING**（见 §3.4） |
| D-09 审计无正文 | **partial** | 9.2：`the_audit_chain_serialises_clean`（动作矩阵全跑 + 双阈值语料 + 三个"牙齿还在"对照）；9.3：库层先例 `soul-policy/tests/audit_chain.rs::a_prose_field_is_refused_before_it_reaches_the_store`；**9.1 链验证调用：MISSING**——测试只 `list_audit`，从未调 `verify_audit_chain()`（见 §3.2） |

## 3. Must-fix（按代价升序不是重要性）

### 3.1 D-01 层 1：xtask 禁用名单没有扩到消息发送类 crate

- **位置**：`crates/xtask/src/egress.rs:36`（`BANNED_HTTP_CLIENTS`）；`crates/xtask/tests/self_test.rs`（无合成 `lettre` 对照）；`deny.toml:89-105` 同样只禁 HTTP 客户端。
- **为什么红**：D-01.1 的红法是「顺手引入 telegram bot 库→1 红」。今天引入 `lettre`/`teloxide`/`matrix-sdk` 作 normal 依赖，`e0-audit` 与 `cargo deny` 双双放行——三层防线的第 1 层整层不在。ST-01 无权改 xtask，已在其报告 §6.1 上报；这条要派给能动 xtask 的工作包，WP10 在它落地前不算关门。
- **最小补丁**（`egress.rs`）：

```rust
/// Crates that put a message somewhere. Drafting never sends, so none of
/// these may be reachable from any shipped crate — soul-egress included.
pub const BANNED_MESSAGING: &[&str] = &[
    "lettre", "async-smtp", "imap", "async-imap", "teloxide",
    "grammers-client", "matrix-sdk", "tokio-tungstenite", "tungstenite",
];
```

在 `audit_dependencies_from_roots` 现有两行 `banned.extend(...)` 旁加
`banned.extend(BANNED_MESSAGING.iter().copied());`。注意 `sanctioned` 分支
只赦免 `BANNED_HTTP_CLIENTS`，所以经 `soul-egress` 到达的消息类 crate 仍会被报
——不需要额外改动。对照用例仿 `a_second_crate_holding_an_http_client_is_reported`：
克隆 metadata，把某个真实存在的传递依赖（如 `uuid`）改名为 `lettre`，断言
`audit_dependencies_from_roots` 报出 `banned == "lettre"`。

### 3.2 D-09.1：动作矩阵之后没有验证链

- **位置**：`crates/soulcore/tests/draft_commands.rs:896`。
- **为什么红**：D-09.1 要求「链验证通过」。测试只 `list_audit()` 读回条目；
  `SqlCipherStore::verify_audit_chain()`（`soul-store/src/store.rs:304`）存在却没被调。
  seq/prev_hash/entry_hash 由库覆盖是事实，但「WP10 的写入没有破坏链」这半句现在无人断言。
- **最小补丁**：`the_audit_chain_serialises_clean` 里 `let chain = store.list_audit()...` 之前加一行：

```rust
store.verify_audit_chain().expect("the chain verifies after the whole matrix");
```

### 3.3 D-04.3 运行期半条：豁免审计不带 turn_id

- **位置**：`crates/soulcore/src/commands/draft.rs:284-288`（`audit.push(outcome.audit())`）；
  `soulcore/src/commands/policy.rs:246-257`（`E1Outcome::audit()` 无 subject_refs）。
- **为什么红**：D-04.3 原文「审计里记的是 `carries_exempted_original` 的事实**与 turn_id**，没有原文」。
  现在 `THIRD_PARTY_BODY_INCLUDED` 记了事实，但哪一条 turn 被豁免链上无痕。
  裸 UUID 是审计明确允许的（PRODUCT_LOCK「可保留孤立本地 UUID」，`AuditContent::about` 现成）。
- **最小补丁**（不动 soul-policy，改在 draft.rs 编排层）：

```rust
let exempted = body.exempted_turn();   // 在 body 被 move 进 e1_generate 之前取出
// ...
Ok(outcome) => {
    let mut entry = outcome.audit();
    if let Some(turn_id) = exempted {
        entry = entry.about(&[turn_id]);
    }
    audit.push(entry);
    // ...
}
```

并在 `one_exemption_one_original_then_clean_again` 断言 `THIRD_PARTY_BODY_INCLUDED`
那条的 `subject_refs == Some(vec![exempted_turn])`。

### 3.4 D-07.3/7.4/8.5：「模型回复是数据」只有结构证明，没有会红的测试

- **位置**：`crates/soul-draft/src/answer.rs`（全文无测试）；`crates/soul-draft/src/tone.rs:107`
  （`DraftOutcome::new` 的 `assert_non_clinical` 无 E1 侧红测试）。
- **为什么红**：D-07.3「mock 回诊断词时拒绝生成而不是过滤后照发」、D-08.5「回复工具调用形 JSON
  仍只是文本草稿」当前零测试。结构防线确实在（`answer_text` 只取
  `choices[0].message.content`，别的字段读都不读；`summarize` 的 evidence_ids 只抄边），
  但按本仓库自己的标准，无红法的保证不算保证。wire 级版本被 MockLlm 恒回空 `content`
  挡住（改 testkit 越 ST-01 的界，其报告 §6.2 已上报）——**单元级版本不被挡**：
- **最小补丁**（`crates/soul-draft/tests/` 新增一个测试文件，三个用例）：
  1. `answer_text` 喂 `{"choices":[{"message":{"role":"assistant","tool_calls":[…],"content":null}}]}`
     → `Err(DraftError::UnreadableAnswer)`（工具调用形回复没有 content，是可读失败，不是动作）；
  2. `answer_text` 喂 `content` 为工具调用形 JSON **字符串**的应答 → 原样返回 `UntrustedText`（数据）；
  3. `DraftOutcome::new(带 denylist 词的文本, DraftRoute::E1, stats)` → `Err(NonClinical)`
     （拒绝，无任何"洗掉再发"路径可走）。

## 4. Should-fix（SOTA 打磨，不是正确性）

1. **`injection_corpus()` 把载荷行 `### SYSTEM ###` 当注释删了**
   （`draft_commands.rs:939-947`，`!line.starts_with('#')`）。语料实际 14/15 行，
   丢的恰好是 RoleImpersonation 的裸标记行。改成 `!line.starts_with("# ")`
   （fixture 的注释全部是「# + 空格」）即可全量。ST-01 报告 §6 末尾已注意到但没修。
2. **D-08.3 循环补全到 `ActionKind::ALL`**（`draft_commands.rs:680`）：现在只跑
   `DraftReply`/`AnalysePeople` 两个。`ExternalContentNotAuthority` 判定在 `check_action`
   里先于一切且与动作无关，所以这不是漏洞，但先例
   （`injection_is_data.rs:153`）就是 ALL，抄过来是三行。
3. **给 `MockLlm` 加 `set_response(content)`**（soul-testkit 所有者）：解锁 §3.4 的
   wire 级版本与 D-08.5 的 `issued_count` 断言。后者还需要 `PolicySession`
   暴露一个只读 `issued_token_count()`（issuer 现在是私有字段）。
4. **`soul-draft/Cargo.toml` 有两个未使用依赖**：`soul-store-api` 与 `serde`
   （src 里 grep 不到任何 `soul_store_api`/`serde::` 引用；只用了 `serde_json`）。
   ST-00 预写清单的遗产。砍掉能让「这个 crate 连存储 API 的形状都不知道」变成清单事实。
5. **`no_send_api.rs::FORBIDDEN` 可加 `"fn submit"`**：产品锁原文是
   「No send/submit/deliver API」，现名单缺 submit 这个拼法。

## 5. 明确的非问题（看着可疑，实际是对的）

1. **审计泄漏检查不是全语料 `min_ngram(4)`**（`draft_commands.rs::ChainCorpus`）。
   按字面写会系统性假阳性：4-scalar 英文命中 `prev_hash` 的 `prev`、`action` 的 `ctio`、
   hex 摘要里的 `2026`（ST-01 报告 §5.1 列了七条实测命中）。现方案两个 checker 对**同一**
   候选串都断言（中文 4、含 ASCII 的 8 + 标识符任意长度），且有三个对照证明牙齿还在：
   提取值确实含链的 serde 拼写（拼写现取非手抄），拼一条真泄漏进候选串语料仍报得出。
   压平到 4 断言的将是「审计链不许出现 action 这个词」，不是「无正文泄漏」。
2. **审计检查只查值不查键**。键是闭集：schema `additionalProperties: false` +
   `FORBIDDEN_FIELDS` 递归拒绝（`soul-policy/src/audit.rs:306-337`），正文进不了键位。
3. **`people_summary` 不写审计、没有 E1 分支**（`draft.rs:311-320`）。ST-01 定案：
   本地统计路径**就是**摘要，不是降级；链只记出网与决策，读自己的图两者都不是。
   连带地，D-07.3「mock 回诊断词时摘要拒绝生成」对摘要侧是空命题——LLM 输出根本
   没有进入摘要的通道（这正是 D-07.4 想要的世界）。
4. **decoy 没有起在语料 URL 上**（`pasted_injection_stays_data`）。语料 URL 是
   `evil.example`，CI 绑不了这个 host。现做法——每个 URL 过 `NetGuard::closed()` 与
   `session.guard().authorize_e1()` 双拒 + loopback decoy 恒 0——是这条断言可实现的
   诚实等价物。
5. **E1 路由下草稿文本是空串**。mock 恒回空 `content`，所以没有一条测试断言 E1
   草稿"像样"——这是规矩本身（wire 断言只许对着出站字节），不是敷衍。
6. **`known_identifiers` 把密封联系人标签解到明文**（`draft.rs:180-194`）。明文只进
   redactor 的占位字典，别处不去；字典为空才是真 bug（两字中文名规则会无物可配），
   `a_summary_cites_evidence_that_resolves_and_names_nobody` 正断言了非空。
7. **语气指令躺在 QUOTE 栅栏里而不是 system 槽**。system 槽与
   `soul_policy::e1::DRAFTING_INSTRUCTION` 逐字节相等是锁（D-08.2 断言了它），
   语气以 Owner turn 进材料位是唯一合规通道，不是搞错了位置。
8. **`DraftRequest.exemption` 是公开字段**。`OneShotExemption` 非 Clone/Copy，
   连带 `DraftRequest` 也不可复制——字段公开改变不了「留不住、放不回」。
9. **`soul_policy::redactor::Turn::new` 收 `impl Into<String>`**。那是 soul-policy
   自己的构造子且立刻包成 `UntrustedText`；soul-draft 的粘贴入口只有
   `PastedTurn::new(SealedSubject, UntrustedText)`，无 String 直通。
10. **`draft.rs` import 了 `BlobStore`**。用的是 `store.open(sealed)`（读），
    不是 `seal`（写）；粘贴不落库由 `a_paste_is_material_not_a_memory`
    的三计数前后全等 + 「链确实长了」对照钉死。

## 6. 范围声明

本复核未启动也不建议在本轮启动：Goal 2、任何 UI 页面、DPAPI 实现、任何文件写路径。
所列 must-fix 全部是测试/审计字段级补丁，无一条要求重写 crate。
