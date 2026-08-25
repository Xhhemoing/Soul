[Model: claude-fable-5-thinking-xhigh]
round: BUILD R3
role: review（只读复核）
scope: 复核本地 HEAD `c0a6295`（`cursor/goal1-build-audit-c441`）对 `7be8279`（R3 synthesis）的全部 12 个提交（`8cded4c..c0a6295`），即 opus-a/b/c 三路落笔。逐项判 P0/P1 闭合/残留/误报，重跑实现者声明的测试。未改任何产品文件；未推送（token 失效，遵嘱不重试）。

## 基线事实

- 工作树干净，HEAD = `c0a6295`。`origin/cursor/goal1-build-audit-c441` 停在 `9b4bcd4`（opus-a 前两个提交已上远端，其余 10 个提交只在本地，含 opus-a 的 `e14425c`）。
- 逐提交核过暂存面：12 个提交各自只含本人分工的文件，无交叉污染。冻结面未动：`PRODUCT_LOCK.md` / `DECISIONS.md` / `GOAL1_PLAN.md` / schemas 在本区间零改动（`schema-freeze --check` 亦绿）。
- `Graph.tsx` / `STATUS.md` 在 diff 视图里出现的「���」是终端显示伪影，两文件逐字节验证为合法 UTF-8、零替换符。

## 判定总表

| 项 | 派单 | 判定 |
|---|---|---|
| P0 未导入姓名 + 原文豁免泄漏（gpt-sol-a） | opus-a | **残留**（大幅收窄，见下） |
| P1 摘要「端点根据本机统计改写」假出处（gpt-sol-a） | opus-b | **闭合**（带已声明且上屏的边界） |
| P1-1 IPC `return;` 假绿（gpt-sol-b） | opus-c | **闭合** |
| P1-2 遗忘重试合同停在 Session（gpt-sol-b） | opus-c | **闭合** |
| P1-3 研究预览只比路径（gpt-sol-b） | opus-c | **闭合** |

## P0 — 残留（不得称收口）

**闭合的一半（真实、三层钉死）**：`Redactor::build` 现把豁免那一条送进 `scrub_exempted_original` = `scrub_identifiers` + `scrub_spaced_label_shapes`（redactor.rs:305/311）。带空格的汉字显示名（1–2 字/组、2–4 组、≤6 字、锚在汉字连串头部、超长整串放弃而非截断）在豁免条里占位。探针的原始 repro（空图 + `李 雷` + 二次确认）在三层被钉死：`redactor_exemption.rs::an_exempted_turn_placeholds_a_display_label_nobody_registered`（断言集合为空）、`redactor_leakage.rs::a_display_label_nobody_registered_is_placeheld_inside_an_exempted_turn`、`session_e1.rs::with_nothing_imported_the_same_name_is_placeheld_by_its_shape`（真回环端点收到的字节：无 `李 雷`、有 `[姓名已占位]`、`场地` 仍在、无第三人占位符）。把泄漏当「not a bug」的旧测试被反转而非删除；被它遮住的图谱布线控制换成 `a_display_name_with_no_shape_is_placeheld_because_the_graph_learned_it`（`Wang Xiao`，标签先从库里读出）。默认路径逐字节不变（`the_default_path_is_unchanged_by_the_label_rule`），豁免仍买到正文（五串散文逐字节回来，含 owner 名 `Roy`、`下午 3 点，第 2 会议室，预算 45000`），研究路径无豁免入口不变。以上我全部重跑，绿。

**残留的一半（与原 P0 同类，范围更窄）**：界面与 PRODUCT_LOCK 76 行的承诺仍是无条件的「姓名与账号两种情况下都占位」，但空图下豁免条里：①不带空格的汉字名（`李雷说…`，自然中文写法）照常出网——`scrub_spaced_label_shapes` 只认 1–2 字的组，≥3 字连串不成组；②拉丁显示名（`Wang Xiao`）照常出网——形状规则限 Han。两条我按源码核实（空集合下 `scrub_identifiers` 只剩邮箱/@handle/7+ 位数字三类形状），opus-a 报告亦如实自陈。gpt-sol-a 派单时点名「copy-only softening 不够，需要能对未入图标识符维持全称承诺的机制（如出网前逐字预览/改写步）」——该机制没有做。此外通知文案一字未动：opus-a 写好的更强条款因 `E1_PLAN_NOTICE` 在 `fakeCore.ts` 有逐字节孪生（当轮属 opus-c）而回退，该约束随本轮结束已失效。残留触发面窄（须用户主动二次确认豁免 + 名字恰为无形状写法），但它就在做出「姓名恒占位」承诺的那块屏上，性质仍是错误知情同意。**按 R3-SYNTHESIS「残留 P0 不得称收口」执行。**

给父代理的两条闭合路径（二选一，非我裁量）：(a) 按探针原方案给豁免加出网前逐字预览/标记步（产品面，能维持全称承诺）；(b) 承认形状规则的边界，把 `E1_PLAN_NOTICE` + Wizard 文案 + `fakeCore.ts` 孪生一票改成与 PRODUCT_LOCK 相容的准确表述——但这需要先裁 PRODUCT_LOCK 76 行「姓名/账号同样占位」是否允许限定语，动锁面不是 opus 单能拍的。无论选哪条，残留应有一条测试或 STATUS 记账钉住现状（现在只写在 redactor.rs 的文档注释里）。

## P1 摘要出处 — 闭合

探针要的两件事都做了、且都到了线上而不只在散文里：

1. **按用途分指令**：`E1Purpose { Draft, PersonSummary }` + `PERSON_SUMMARY_INSTRUCTION`（只许改写计数、不得添事实、不得改数字、改不出就照抄），指令位仍只放本 crate 常量；`e1_generate_for` 按请求传用途、不粘会话。`session_summary.rs::the_summary_asks_for_a_rewrite_and_the_draft_beside_it_still_asks_for_a_draft` 在同一会话对真回环端点连发摘要+起草，断言两个请求体各带各的指令——这是产品路径的线上证据，不是常量自证。
2. **回答不再照单全收 + 标签说实话**：`reply::read_grounded_in` 丢「说出材料里没有的数字」（阿拉伯/全角/中文数词，判定性）与「与材料共享 <3 个相邻字符对」（跑题）两类；material 是真正发出的 `RedactedBody` 字节。榴莲 repro 端到端钉死：`session_summary.rs::an_answer_about_something_else_leaves_the_counts_standing` 断言 `request_count()==1`（请求真的发生了）、退回 counts、点位逐条带 evidence、链上有 `egress.request` 而搜不到那句话。活下来的句子由 `ENDPOINT_LINE_PREFIX` 在正文行内点名「端点写的、无证据、本机没核对」；`Graph.tsx` 的 `user_endpoint` 来源行同样说法，且「根据本机统计改写」全仓只剩注释/历史记述/否定断言（我逐处核过六个文件）。`className="reading"`（样式表既有）让按行的「依据 N 条记录」在屏上真的按行。

**已声明的边界（接受）**：`is_grounded_in` 不是忠实性判定——`稳定` 这类计数没说的定性词照样通过。opus-b 在代码、屏幕、STATUS、清单四处如实写明，这正是探针给的备选补救「若任意散文仍可见，就标成未核实的端点输出」，故按闭合记，不算残留。三处派单外改动核过均正当：`commands/policy.rs`（纯新增入口，守卫/令牌/脱敏体不变，用途买不到 socket）、`soul-draft/draft.rs` 6 行（新枚举值映射为 `ReplyUnreadable` 而非 `unreachable!`，端点不该能索取 panic，方向正确）、STATUS/清单两处旧句（不改就是新的 copy-vs-behavior 缺口）。

## P1-1 / P1-2 / P1-3 — 闭合

- **P1-1**：`ipc_roundtrip.rs` 的 `return;` 从 14 个到 **0 个**（新旧文件各 `rg -c` 核过；现存三个 "return" 均为注释/函数名）。前置全部 `expect`。被容忍的那个状态成为独立测试 `a_store_that_will_not_open_is_a_coded_refusal_rather_than_an_empty_answer`：先断 `store_opened==false`，再要求 7 条命令各以 `ROUTINE` + `STORE_UNAVAILABLE_NOTICE` + 恰好两字段拒绝。54/54 我重跑绿。
- **P1-2**：IPC 主链路测试改为拒绝后**用同一份预览**确认成功、再重放被 `PLAN_HASH_MISMATCH` 拒（读过测试体，2748–2867 行）；新测试补两半错配 + 链上 2×`hitl.deny` + 1×`forget.execute`。产品修复一处且正确：`Memory.tsx` 的 `setPreview(null)` 移入成功分支，拒绝分支只置 refusal（读过现行代码 135–151 行，拒绝后面板保留）。`fakeCore.ts` 用 `heldForget` 学 `Session::held_forget` 的持有/两半匹配/花掉语义。UI 钉：一次点击恰一次 `forget_memory`、拒绝后同一 `preview_id` 仍在且按钮可用、成功后重放被拒。
- **P1-3**：`session_screens.rs` 与 `ipc_roundtrip.rs` 都改为对整个数据目录做排序 `(相对路径, 长度, SHA-256)` 前后快照，且先断快照非空。`-wal` 计入（审计追加会落在那里），`-shm` 排除且理由成立（WAL 索引、读锁即戳、无数据）。手写 SHA-256 由 `the_digest_agrees_with_the_published_vectors` 钉 FIPS 180-4 三个向量（三个期望值我对过公开值，正确），常量哈希混不过去。store 层原测试未动。

## 我重跑的测试（全部在 HEAD `c0a6295`，与实现者声明数逐一相符）

- `cargo test -p soul-policy --test redactor_exemption --test redactor_leakage` — **9/9 + 9/9**
- `cargo test -p soulcore --test session_e1` — **28/28**；`--test session_screens` — **11/11**；`--test session_summary` — **3/3**
- `cargo test -p soul-draft` 全部绿（其中 `people_summary` **17/17**）
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test ipc_roundtrip` — **54/54**
- `cargo test --workspace` — **588 passed / 0 failed**
- `pnpm exec vitest run`（apps/desktop 全量） — **14 文件 172/172**
- `npm run lint`（tsc + eslint）绿；`cargo clippy --workspace --all-targets --all-features` 无 warning/error 行；`cargo fmt --all -- --check` 绿
- `xtask e0-audit`（14 crates / 186 文件）clean；`schema-freeze --check` 匹配锁；`denylist-audit` clean

数字差异说明：opus-c 报 vitest 170、opus-b 报 172——opus-b 的 `Graph.test.tsx` 在 opus-c 之后落 +2，最终 172 正确。opus-a 报 ipc 51 是其落笔时点（opus-c 未落），最终 54 正确。共享工作树导致的中途不编译已自愈，最终树以我的重跑为准。

assumptions:
- 「两种情况」按字面读=默认路径与豁免路径（探针与 synthesis 的读法），故拉丁/无空格汉字名在豁免条出网记 P0 同类残留而非新开 P2。
- 判定对象是探针原文的 finding；实现者自陈且探针备选补救覆盖的边界（grounded≠忠实）不降闭合等级。
- opus-c 报告里两条 mutation 验证（`setPreview(null)` 前移变红、快照间插 `write_memory` 变红）未实际复演——只读约束下按静态读码确认测试确实咬得住（面板断言依赖 preview 存续；快照是字节元组相等）。

do_not_touch（遵守情况）: 我只写了本文件。未推送、未合 PR、未动 Goal 2、未动冻结文档。三个 opus 的 MUST-NOT 清单核过均守住；唯一同文件重叠是 opus-a/opus-b 都动了 `crates/soulcore/src/commands/draft.rs`（synthesis 未把它列进 opus-b 派单也未列进禁触），两次改动内容不相交、按提交串行、无冲突——记流程观察，不记缺陷。

next:
- P0 残留是本轮唯一不能称收口的项：父代理在上文 (a)/(b) 两条路径里裁一条派下轮；无论哪条，先把残留钉进一条测试或 STATUS 记账。
- 本地 10 个提交等有效凭据后推送（origin 停在 `9b4bcd4`）。gpt-sol-b 的 P2-1/P2-2 本轮未派、未动，留后批。N4/N5/N6 仍 blocked-on-user；LOOP20 仍 QUEUED；本地绿不是 hosted 绿。
