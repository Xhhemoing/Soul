# Round 3 · opus-a 冻结确认

MODEL_SLUG: claude-opus-5-thinking-high-fast
对象：`docs/BLOCKERS.md`（`BLOCKERS_FROZEN`）× FORMAL_WORK_PROMPT AC-01..AC-26 × `origin/cursor/soul-goal1-7b1c`（`fc96e46`）+ `crates/soul-algo-tie/tests/goal1_fidelity.rs`。
矩阵原文与主干 `docs/FORMAL_WORK_PROMPT.md` 第 74–106 行逐字节相同，无漂移。
只报三类：矛盾 / 错误降档 / 假 P0。

**根因（导致下面 1 与 2）：** §1 写「核对：2026-08-24 16:45 UTC」，但按 `3e88b48`（15:58:42）盘点。16:22–16:44 之间主干已推 7 个提交，冻结时未计入。

## 1. 假 P0 — S1 DPAPI 已落地（冻结前 23 分钟）

`40b3474`（16:22:41）新增 `crates/soul-win-dpapi`（`sys.rs` 四个 `extern "system"`、一个 `unsafe` 块）；`2f323d5`（16:22:54）让 `DpapiKeyProvider` 真的用 `CryptProtectData` 保护 KEK、并用 XChaCha20-Poly1305 把 DEK 包在 KEK 下写进 `keys.dpapi`。
`crates/soulcore/src/commands/session.rs:215 key_provider()` 在 `cfg!(windows)` 分支返回它；`crates/soul-store/src/lib.rs:25` 仍是 `#![forbid(unsafe_code)]`；`KeyError::Unavailable` 在 `keys.rs:41`。§4 S1 的三条约束全部被满足，只有 crate 名不是文中写的 `soul-winkeys`，是 `soul-win-dpapi`。
**改判：** S1 从「P0 发货」改为「已落地，未验证」。剩下为真的只有一句：`crates/soul-store/tests/dpapi_key_chain.rs` **在 windows-latest 上一次都没跑过**——工作区 `cargo test` 仍死在字母序更靠前的 `soul-fileplan`。它的验证归 M3，不是独立工作单。
连带：§5 步骤 7 作废；§2 M3 末段「DPAPI 落地前会在 `Session::open` 处继续红」前提消失，`Session::open_with_keys` 不必再加。

## 2. 假 P0（按其自述口径）— G5 的关闭门已达成

`46bace8`（16:34:12）向导从 `questionnaire()` 取十一题渲染（`apps/desktop/src/routes/Wizard.tsx:87,202`）；`f799ffb` / `4829671` 补齐四屏。
`COMMAND_NAMES` 从 14 变 27，含 `questionnaire` / `answer_questionnaire` / `profile_screen` / `correct_axis` / `set_voice` / `memory_*` / `preview_forget` / `forget_memory` / `research_preview` / `audit_chain`；路由新增 `Profile` / `Memory` / `Research` / `Audit`。
§3 G5 写死的关闭门「向导十一题 + `/profile`（`correct_axis`）」**两项都已在树上**，§5 步骤 6 同样作废。
**仍为真的残余只有两项**：导入没有用户路径（`COMMAND_NAMES` 无 `import_*`），采集开关没有界面（`Settings.tsx` 只有 `CloudToggle`，无 `collect_*` 命令）。G5 应重写为这两项，不要继续以「问卷 UI 缺失」为题——那会让实现者去做已经做完的事。

## 3. 矛盾 — M3 的在途修复走了 §2 明令禁止的那条路

`fc96e46`（16:52:02，CI run 32753307746 进行中）：decoy 移进 `#[cfg(unix)]`、四处探针（原 `:50` `:53` `:237` `:247`）照改，根因与范围与 §2 完全一致。但两处直接违反冻结文本：

1. 语料守卫改成 **无条件 `corpus.len() >= 25`**。§2 原文：「不要为绿把守卫放宽成无条件 `>= 25`（Linux 会默默丢条目）。守卫必须改成精确式：`25 + (probe ? 2 : 0) + (cfg!(unix) ? 3 : 0)`」。现在 Linux 上语料是 30 条（25 + `lower_alpha` 2 + 符号链接 3），守卫掉 5 条也不会红。
2. `case_is_decided_by_the_filesystem_rather_than_assumed` 被改成按 `cfg!(windows)` 选探针（Windows 用 `alpha/photo.jpg`，Unix 用 `alpha/decoy.txt`）。§2 原文：「不要改、不要跟探针走。问文件系统，不要问 `cfg!(windows)`（APFS 会折、NTFS 可标大小写敏感）」。这条测试的名字说的就是它现在不做的事。

二者择一：要么把守卫改回精确式、把该测试的探针改成问文件系统；要么由父代理对 §2 这两句作显式改判。**默认不许静默采纳**——这正是冻结文本预先点名的「为绿放宽」。

## 4. 错误降档理由 — S2 的升 P0 条件按字面不可能满足

§4 S2 写「升 P0 的条件：一条能穿过现有占位、让 AC-12 红的夹具（owner 文本里嵌 2–3 字中文名）」。这样的夹具**在 AC-12 现有的两处测试里都红不了**，因为两处都自带身份表：
`crates/soul-policy/tests/redactor_leakage.rs:34-43` 从夹具的 `known_identifiers` 现建 `KnownIdentifiers`（注释自称「as the contact graph would supply them」），`names_and_accounts_are_placeheld_even_inside_the_users_own_words` 用的正是 owner 文本里嵌「李雷」；`crates/soulcore/tests/draft_commands.rs:28-32` 直接 `.with_name("王小明")`。
空集只在发货缝上：`crates/soulcore/src/commands/draft.rs:316 closed_session()` 与 `crates/soulcore/src/headless.rs:489` 都用 `KnownIdentifiers::new()`，而这条缝上没有任何泄漏断言（headless 的 draft 段只查模板与「未发送」提示）。
**改判理由不改结论：** S2 仍可留 P1，但依据要从「还没有夹具」改成与 S1 / S5 同类的「谁跑」措辞洞——AC-12 在 crate 边界验的是注入的身份表，不是产品实际持有的那一份。升级条件应写成「在 session 缝（`closed_session` / headless 起草）上加一条泄漏断言」，那是一条断言，不是一条夹具。

## 5. 复核成立、不改判（供父代理免于重开）

- G1 / G2 / G3 三条 P0 在 `fc96e46` 上依然成立：`crates/soul-profile/src/service.rs:179 intake` 仍不查 `axis_is_locked`（`record_axis_inference` 在 `:406` 查了）；`crates/soul-graph/src/build.rs:322` 仍写 `UserVerdict::Unreviewed`；`build.rs` 仍是全场地 3/10/3——`goal1_fidelity.rs` 的 T0 oracle 与之逐格相符，2000 例随机 + 跨 epoch 全等，说明 §3 G1 对现行判档的描述准确。
- G1 接线不会撞现有 goal1 测试：`crates/soul-graph/tests/ego_graph.rs:209,225` 断言 lilei 12 条一对一 → Strong、赵七仅群聊 → Weak，与 T4D 冻结答案同向。`soul_algo_tie::score` / `score_ego_network` / `silent_days` / `last_direct_contact_unix` / `as_of_max` 均已在冻结 crate 的公开面上，A2 的 `venue_split()` 确实要求分列都是 `Some`（`a2.rs:194`），§3 G1 的接线清单可照做。
- G4 扇出维持 P1：`crates/soul-import/src/commit.rs:198-207` 确实把 owner 的群消息扇给该会话**全文件**的发言者，确会刷新 `last_contact` 并让 T4D 的 ≥180 天降档失效（T4D 的近因时钟读任一场地，`t4d.rs:90-93`）。但矩阵没有任何一行约束档位或近因（AC-05 只要「映射事件/联系人；缺字段失败可读」，AC-08 只要「节点≥3、边有证据」），且 §3 G4 已把「伪造 `last_contact`、挡住降档」原样写进文档。这是已定价的已知代价，不是被错误降档的项。
