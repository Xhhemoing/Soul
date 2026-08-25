[Model: claude-opus-4-1-20250805]
round: BUILD R3（残留收口）
role: implement（opus-d）
branch: `cursor/goal1-build-audit-c441`（本地 4 个提交，未推送——凭据仍失效，按嘱不重试）
scope: 只做 fable R3 §P0 的残留一半：让空图豁免条里「不带空格的汉字名」与「拉丁显示名」也占位。未开新产品屏（路径 (a) 未走），未动 PRODUCT_LOCK（路径 (b) 未走），未改任何上屏文案。

## 落笔的四个提交

| 提交 | 内容 |
|---|---|
| `f53c3ae` | `soul-policy/src/redactor.rs` 新增 `scrub_attributed_name_shapes` + 两个测试文件的正例/反例 |
| `92623b1` | `soulcore/tests/session_e1.rs`：真回环端点上的三条（两条闭合、一条记账） |
| `fa0c9e1` | `soulcore/src/commands/draft.rs`：只改 `E1_PLAN_NOTICE` 的文档注释，常量字符串逐字节未动 |
| `9910118` | `redactor_exemption.rs`：把两条新规则钉死在豁免条内、钉出用户本人话语之外 |

## 做了什么

原来的 `scrub_spaced_label_shapes` 认的是**拼法**（导出文件写的 `李 雷`）。残留的两例没有拼法可认：`李雷说…` 是连着的四个汉字，`Wang Xiao` 是两个首字母大写的词而英文本来就这么写。它们共同有的是**位置**——聊天记录就是「谁说了什么」，名字站在说话动词前面。

新规则要求两个信号同时成立，宁窄勿宽（用户二次确认买的是这条消息，满屏占位符就不是那条消息了）：

- 汉字：Han 连串的**串头** + 首字是常见单姓（约 100 个）或复姓（欧阳/司马/上官…） + 全名 2–3 字（复姓 3–4 字） + 不在 `NOT_A_NAME`（于是/马上/方才/高兴/向来/万一/严重/石头/毛病/白天）里 + 紧跟说话动词（说/说道/讲/问/表示/告诉/提到/回复/写道/提醒/通知/发来）。取**最短**候选，少吃一个字算一个字。
- 拉丁：词头 + 连续 2–3 个首字母大写词（首字母大写且至少一个小写字母，`SYSTEM`/单字母缩写不算） + 紧跟说话动词（said/says/wrote/writes/asked/asks/told/tells/mentioned/replied/replies/texted/messaged，或跨语言的 `说` 一类）。
- 只在豁免条上跑（`scrub_exempted_original`），默认路径、研究路径、用户本人话语一律不碰。

姓氏表 + 停用词表是为了不误伤：没有姓氏锚，`大家说`/`老板说`/`对方说`/`同事说` 全会中招；有了它，只剩 `于是说`/`马上说` 这种以姓氏字起头的副词，用一张十条的停用表挡掉，并各写了一条断言。

## 断言（都在真字节上，不在计划的自我复制上）

新增 6 条，改写 1 条：

- `redactor_exemption.rs::an_exempted_turn_placeholds_a_name_written_in_front_of_a_verb_of_saying` — 空标识符集，`李雷说周五的场地他已经订好了，你直接过来就行` **整串逐字节**等于 `[姓名已占位]说周五的场地他已经订好了，你直接过来就行`：占位的是名字，不是从句，也不是前后多一个字。
- `redactor_exemption.rs::an_exempted_turn_placeholds_a_latin_display_label_in_front_of_a_verb_of_saying` — 三种写法（`Wang Xiao said…` / `Wang Xiao 说…` / `Wang Xiao texted…`）都不带 `Wang Xiao` 出网，且各自点名必须活下来的词（venue is booked / 方案 / deposit）。
- `redactor_leakage.rs::a_name_in_front_of_a_verb_of_saying_is_placeheld_inside_an_exempted_turn` — 同两例走 `LeakageChecker`，checker 问的是名字不是句子，所以「把整条吃掉」也过不了。
- `soulcore/tests/session_e1.rs::with_nothing_imported_a_name_without_a_space_is_placeheld_by_where_it_stands` — 空图 Session + `MockLlm` 真 socket：端点收到的 body 无 `李雷`、有 `[姓名已占位]`、有 `场地`、无第三人正文占位符。
- `session_e1.rs::with_nothing_imported_a_latin_display_label_is_placeheld_by_where_it_stands` — 同上，`Wang Xiao said Friday's venue is booked…`，body 无 `Wang Xiao`、有占位符、有 `venue is booked`。
- `redactor_exemption.rs::the_default_path_is_unchanged_by_the_label_rule` 里加一句：同样两个名字放进 **Owner** 那一条走默认路径，逐字节原样返回——两条新规则确实只长在豁免条上。
- `the_label_shape_leaves_the_prose_the_exemption_was_for_alone` 加三串反例：`于是说好了周五在会议室碰头`、`马上说定，我这边没问题`（两个信号都在，答案仍是不动）、`Roy said the venue is booked already`（单个大写词不是显示名，账号主人不是第三人）。

**变异验证（实做，不是读码）**：把 `scrub_exempted_original` 里对新函数的调用摘掉重跑——`redactor_exemption` 2 红、`redactor_leakage` 1 红、`session_e1` 2 红，正是上面五条；记账那条按设计仍绿。随后从备份恢复，`git status` 干净。

## 残留（本轮没有假装收口的部分）

`session_e1.rs::the_name_shapes_cannot_reach_a_label_standing_outside_an_attribution` 是记账条，按 fable「残留应有一条测试或 STATUS 记账钉住现状」写的：空图 + 二次确认 + `这周的方案我已经发给 Wang Xiao，别拖到下周`，端点**确实收到** `Wang Xiao`，测试断言它收到了。文档注释里写明这不是背书，是记账；测试失败信息直接写「如果它现在被占位了，说明比本测试描述的更好，请把这条挪到上面两条旁边并把报告里的这项划掉」。同一条测试的后半段断言默认路径上该名字**不出网**（整条第三人正文占位），把缺口范围钉在「豁免条 + 空图 + 名字不在归属位」这一格里。

两个形状合起来仍够不着的，源码 `scrub_attributed_name_shapes` 的 `# What this still does not catch` 逐条列了：

1. 不在说话动词前的名字（`方案下周交给李雷`）——那几个字和普通词没有区别；
2. 姓氏不在表里，或朋友之间的写法（`小王`、`老李`）；
3. 同一 Han 连串里的第二个名字（`李雷说张伟明天也过来` 里的 `张伟`）——只看串头，而串不在名字处结束。

`redactor_exemption.rs::the_names_the_shapes_still_cannot_see_are_written_down_here` 把这三条 + 拉丁的对应情形写成断言（现在带名字出网），并在同一条测试里证明：只要导入过任何东西、`KnownIdentifiers` 里有这几个名字，它们**全部**被占位。即缺口只存在于「什么都没导入 + 豁免条 + 名字不在归属位」。

## 文案面（按派单边界处理）

`E1_PLAN_NOTICE` 字符串一字未动，`fakeCore.ts` 孪生因此不必动，Wizard 文案不必动，vitest 无关。改的只有它的**文档注释**：opus-a 那版写的是「豁免条被额外要求过显示名形状，所以这句承诺在空图下成立」——现在这话仍偏大，改成「对于拼法或位置像名字的名字成立」，并写明剩下那格、指向源码与记账测试，最后说明为什么不在这里软化文案：PRODUCT_LOCK 第 76 行还立着，只在一块屏上加限定语比两边都不动更糟——那是路径 (b)，动锁面不是我能拍的。

## 我跑过的（全部本地，HEAD 为本文四个提交之后）

- `cargo test -p soul-policy --test redactor_exemption --test redactor_leakage` — **12/12 + 10/10**（原 9/9 + 9/9）
- `cargo test -p soulcore --test session_e1` — **31/31**（原 28/28）；`cargo test -p soulcore` 全量 **169/169**
- `cargo test --workspace` — **595 passed / 0 failed**（R3 复核时 588，新增 7 条）
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` — **87/87**（其中 `ipc_roundtrip` 54/54）
- `pnpm exec vitest run`（apps/desktop 全量）— **14 文件 172/172**；`npm run lint`（tsc + eslint）绿
- `cargo fmt --all -- --check` 绿；`cargo clippy --workspace --all-targets --all-features` 无 warning/error 行
- `xtask e0-audit`（14 crates / 186 文件）clean；`schema-freeze --check` 匹配锁；`denylist-audit` clean

## 边界与自陈

- 姓氏表是数据，不是判定：表外的姓（含少数民族姓名、外文音译名的汉字写法）落进上面第 2 条缺口。停用词表同理，是一张十条的手写表，不是词法分析。
- 拉丁规则会连带吃掉句首大写词的边缘情形（`Yesterday Wang said…` 会占位 `Yesterday Wang`）——多吃一个词，方向偏保守；名字不会漏。
- 「归属位」只认列出的动词。`李雷回…`、`李雷称…` 这类故意没收（`回`/`称`/`道` 在别的位置是普通词，代价是散文被吃）。
- 本轮没有做探针原方案的出网前逐字预览步；父代理若裁路径 (a)，本轮的两个形状可以整体留作它的下层默认，互不冲突。

do_not_touch（遵守情况）: 只改了派单 MAY 里的四个文件（`redactor.rs`、`redactor_exemption.rs`、`redactor_leakage.rs`、`session_e1.rs`）加 `commands/draft.rs` 的注释。`Graph.tsx` / `Memory.tsx` / `ipc_roundtrip.rs` / `analysis.rs` / `e1.rs` / `session.rs` / PRODUCT_LOCK / schemas / `fakeCore.ts` / `Wizard.tsx` 零改动。未推送、未开 PR、未动 Goal 2、未动 STATUS.md。倒转过的 `李 雷` 测试与它的图谱控制两条都还在，后者的 paste 改成把标签放在非归属位（`这周的方案我已经发给 Wang Xiao`），这样它继续只能由联系人图占位——不改的话新规则会把它遮住，它就不再是控制了。

next:
- 缺口那一格要真封上，仍是父代理在 (a)/(b) 里裁一条：(a) 出网前逐字预览/标记步；(b) 裁 PRODUCT_LOCK 76 行是否允许限定语，再一票改 `E1_PLAN_NOTICE` + Wizard + `fakeCore.ts` 孪生。现状已有测试记账，两条路径都能在它上面接着走。
- 本地 17 个提交等有效凭据后推送（origin 仍停在 `9b4bcd4`，我落笔时另有一条 `67b0f4d` STATUS 记账已在树上，非我所写）。本地绿不是 hosted 绿。
