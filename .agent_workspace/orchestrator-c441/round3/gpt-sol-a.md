[Model: gpt-5.6-sol-xhigh-fast]

# BUILD R3 — copy-vs-behavior / redline probe

- round: `BUILD R3`
- role: `probe`
- audit target: `cursor/goal1-build-audit-c441` @ `2bd56a25758290d8e2025f451e8cd7dc84034b68`
- mode: report-only; no `apps/` or `crates/` edits
- result: **1 P0, 1 P1, 1 P2**

## Findings

### P0 — 「姓名两种情况下都占位」对未导入姓名是假的；单次原文豁免会把姓名送出

- **file:line / quoted sentence**
  - `apps/desktop/src/routes/Wizard.tsx:186-189`: `姓名与账号两种情况下都占位。`
  - `crates/soulcore/src/commands/draft.rs:91-96`（由 `Draft.tsx:235` 渲染）: `姓名与账号两种情况下都占位。`
- **why false**
  - `crates/soul-policy/src/redactor.rs:275-285,288-305` 只替换 `KnownIdentifiers` 中已有的姓名，以及邮箱、`@handle`、七位以上数字这几类形状。普通的两字中文姓名没有通用形状规则。
  - 产品只从库中已有第三人 `display_label_ref` 填 `KnownIdentifiers`；用户刚粘贴、尚未导入的姓名不在集合里。
  - 仓库已有一条产品级控制用例把泄漏钉死：`crates/soulcore/tests/session_e1.rs:1044-1077` 明写 `name arrives at the endpoint intact`，并断言未导入任何联系人时，二次确认原文豁免后 `李 雷` 原样出现在 mock endpoint 的请求体。测试注释称其为 “not a bug”，但它直接违反 PRODUCT_LOCK `73-78` 与两个确认界面的绝对承诺。
  - 默认路径仍安全：整段第三人正文会占位。破口只在用户按「这一条按原文带上」后；也正因为界面此时承诺姓名仍会占位，属于错误的出网知情同意。
- **repro**
  1. 新建空数据目录，不导入联系人。
  2. 配置一个记录请求体的 OpenAI-compatible mock endpoint。
  3. 在起草页粘贴 `李 雷 说周五的场地他已经订好了，你直接过来就行`。
  4. 按「用你自己的模型端点写」→「这一条按原文带上」→「确认，开始生成」。
  5. 请求体包含 `李 雷`；仓库现有 `with_nothing_imported_the_same_name_is_a_word_like_any_other` 已证明同一路径。
- **priority**: **P0** — privacy / informed-consent boundary; this is an explicit redline-6 placeholder promise, not a cosmetic imprecision.
- **proposed opus scope (paths)**
  - `crates/soul-policy/src/redactor.rs`
  - `crates/soul-policy/tests/redactor_exemption.rs`
  - `crates/soulcore/src/commands/draft.rs`
  - `crates/soulcore/tests/session_e1.rs`
  - `apps/desktop/src/routes/Draft.tsx`, `Draft.test.tsx`
  - `apps/desktop/src/routes/Wizard.tsx`, `Wizard.test.tsx`
  - A copy-only softening is insufficient while PRODUCT_LOCK still requires names/accounts to remain placeheld. The exemption needs an exact outbound preview/edit-or-mark step, or another mechanism that can uphold the universal claim for identifiers not already in the graph.

### P1 — 人事摘要把任意端点文本标成「根据本机统计改写」，且该新增叙述没有证据

- **file:line / quoted sentence**
  - `apps/desktop/src/routes/Graph.tsx:48-50`: `这一份是你自己的端点根据本机统计改写的。`
  - `apps/desktop/src/routes/Graph.tsx:139-142`: `外加一句固定的改写要求`
- **why false**
  - 摘要请求复用了起草系统指令：`crates/soul-policy/src/e1.rs:24-26,86-100` 固定发送的是 `只根据用户档案起草回复`，并不是人事摘要改写指令。`crates/soul-draft/src/analysis.rs:219-233` 只在引用材料里放一句描述性的 `供改写参考`。
  - 返回值没有做“是否只改写已有统计”的约束。`crates/soul-draft/src/reply.rs:50-72` 接受任意非空、非诊断文本；`analysis.rs:200-215` 随即把它放进 `narrative` 并把 source 标成 `UserEndpoint`。
  - `analysis.rs:170-186` 把这段自由文本渲染为 `整体来看：…`，但只有原来的 `SummaryPoint` 带 `evidence_ids`。因此 `crates/soulcore/src/commands/draft.rs:708` 的内部承诺 `Every line, each naming how many rows are behind it` 也不成立。
  - 外部端点是明确的不可信边界；不能因为 prompt 希望它改写，就把任意答复在 UI 上陈述成由本机统计支持。
- **repro**
  1. 导入可生成一条关系的 fixture，配置 mock endpoint。
  2. 让 endpoint 对任何请求返回合法 OpenAI JSON，assistant content 为 `这个人最喜欢榴莲。`。
  3. 在人脉图按「看这个人的摘要」。
  4. 页面显示该句，同时 source 显示「端点根据本机统计改写」；该句不在发出的统计中，也没有对应 evidence id。
- **priority**: **P1** — materially false provenance/evidence label on the people-analysis path; conflicts with AC-16’s evidence-backed summary intent.
- **proposed opus scope (paths)**
  - `crates/soul-policy/src/e1.rs`
  - `crates/soul-draft/src/analysis.rs`, `reply.rs`
  - `crates/soul-draft/tests/people_summary.rs`
  - `crates/soulcore/src/commands/draft.rs`
  - `crates/soulcore/tests/session_e1.rs`
  - `apps/desktop/src/routes/Graph.tsx`, `Graph.test.tsx`
  - Give summary generation a purpose-specific instruction and do not present unconstrained endpoint prose as evidence-backed. If arbitrary prose remains visible, label it as unverified endpoint output rather than a supported rewrite.

### P2 — IPv6 endpoint is accepted as configured, then rendered into an invalid request URL

- **file:line / quoted sentence**
  - `apps/desktop/src/routes/Settings.tsx:81-87`: `只取地址里的协议、主机和端口：生成的时候请求发到同一个来源下的 /v1/chat/completions`
- **why false**
  - `crates/soul-policy/src/net_guard.rs:142-151` explicitly accepts bracketed IPv6 authorities such as `[::1]:11434` and stores the host without brackets.
  - `Origin`’s `Display` at `net_guard.rs:127-138` writes that host back as `http://::1:11434`, not `http://[::1]:11434`.
  - `crates/soul-policy/src/e1.rs:74-82` builds the actual request URL from that display value. The endpoint is reported as configured, but reqwest receives a malformed URL and cannot send the promised request.
- **repro**
  1. Save `http://[::1]:11434/v1` in Settings; `Origin::parse` accepts it and the UI reports `已填写`.
  2. Approve endpoint drafting.
  3. Observe the generated target is `http://::1:11434/v1/chat/completions` and generation fails before reaching the IPv6 listener.
- **priority**: **P2** — endpoint availability/copy precision; no E0 escape, but an address class the parser deliberately accepts cannot work.
- **proposed opus scope (paths)**
  - `crates/soul-policy/src/net_guard.rs`
  - `crates/soul-policy/tests/net_guard.rs`
  - `crates/soul-egress/tests/e1_origin.rs`
  - `crates/soulcore/tests/session_e1.rs`

## Clean checks

- **Dual threshold literals / redline 11:** clean. Production tie-band thresholds exist only at `crates/soul-graph/src/build.rs:39-44`; classification uses those constants at `:118-130`. `Graph.tsx` only maps returned enum words, and store SQL contains no tie-band `CASE`, `HAVING`, or numeric cutoff. Test fixtures contain example counts/bands, not a second classifier.
- **E0 paths:** clean in the scanned product tree. `soul-policy::EgressClass` has only E1/L, `EgressPermit` has no public constructor, and `soul-egress::send` requires both a permit and `RedactedBody`. No second shipped HTTP/TCP/WebSocket path was found; cross-origin redirect remains denied.
- **Audit containing body text:** clean. `AuditContent` can carry only closed enums, UUIDs, counts, hashes/token id and class (`crates/soul-policy/src/audit.rs:191-208`); reason codes are closed; `audit.schema.json` is `additionalProperties: false`; the SQL audit table stores that typed document. No body/text/content/quote/summary/prompt channel was found.
- **R2 closeouts:** Files empty state, wizard directory scope, import preview audit caveat, repeated-import warning, forget/SSD wording, graph outbound payload disclosure, and draft request-body notice all remain narrowed at HEAD.
- **Other UI claims checked:** import stays local and non-authoritative; file planning has no write execution; research preview remains memory-only with zero third-party rows; collection remains process-local consent with duration-only events; endpoint address is not persisted; message sending remains absent.

## Tests

- Static source trace only in the first report pass; no product file was changed.

## Assumptions / do-not-touch

- PRODUCT_LOCK is authoritative over a test comment that calls unknown-name leakage “not a bug”.
- “姓名与账号两种情况下都占位” is read literally because it appears at the exact one-shot-exemption consent boundary.
- Did not edit `apps/`, `crates/`, schemas, lock documents, `PROGRESS.md`, or Goal 2.
