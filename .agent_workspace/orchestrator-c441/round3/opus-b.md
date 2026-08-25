[Model: claude-opus-5-thinking-high-fast]

# BUILD R3 — opus-b：人事摘要的出处标签与 E1 用途指令

- round: `BUILD R3`
- role: `implement`
- branch: `cursor/goal1-build-audit-c441`
- 起点：`7be8279`（工作树期间被 opus-a 推进到 `e14425c`，无冲突）
- 提交：`7c921be`（crates）、`2312dec`（Graph 界面）、以及一条文档收窄
- 状态：**P1 关闭**（在下面「没做到的那一半」写明它关的是什么、没关什么）

## 缺口原文

`Graph.tsx` 说 `这一份是你自己的端点根据本机统计改写的。`，同页出网提示说 `外加一句固定的改写要求`。
实际上：`soul-policy/src/e1.rs` 只有一条系统指令 `只根据用户档案起草回复`，摘要请求发的就是它；
`soul-draft/src/reply.rs` 只要求非空、非诊断；`analysis.rs::phrase_with` 拿到什么就放进 `narrative`
并把 `source` 标成 `UserEndpoint`。端点回 `这个人最喜欢榴莲。`，屏幕上就是一句「根据本机统计改写」的
无证据断言，而 AC-16 要的是每一条看起来像结论的话都拴在 evidence_ids 上。

## 落笔

**1. 按用途分指令（`crates/soul-policy/src/e1.rs`）**

新增 `E1Purpose { Draft, PersonSummary }` 与 `PERSON_SUMMARY_INSTRUCTION`：
「只能改写这些计数、不得添新事实、不得改动或新增任何数字、改写不出来就照抄」。
`E1RequestPlan::new` / `chat_completions` 仍是 `Draft`（`soul-egress`、`soul-draft` 既有测试逐字节
对着 `DRAFTING_INSTRUCTION`，一条都没动），另加 `for_purpose` / `chat_completions_for`。
指令位仍然只放本 crate 的常量，没有任何运行时的值到得了那里。

**2. 用途按请求走，不按会话走（`soulcore/src/commands/policy.rs`，新增；`draft.rs`，改一处调用）**

`PolicySession::e1_generate_for(purpose, …)`，`e1_generate` 转发 `Draft`。摘要那条路
（`Rephraser::generate`）传 `PersonSummary`。守卫、令牌、脱敏体一个都没变——新用途买不到任何
它本来没有的 socket。`e1_plan`（计划哈希）没有动，因为摘要路径本来就没有确认屏，起草那条的哈希语义
不该在这一票里变。

**3. 回来的字不再照单全收（`soul-draft/src/reply.rs`）**

`read_grounded_in(raw, material)` + `is_grounded_in(text, material)`，`ReplyDefect::Ungrounded`：

- **数字**：`text` 里每一段数字（阿拉伯、全角、中文数词）都必须在 `material` 里出现过。这一半是判定，
  不是估计——摘要就是计数，计数里没有的数字就是端点编的。
- **跑题**：`text` 与 `material` 至少要共享 3 个不同的相邻字符对。`这个人最喜欢榴莲` 与摘要只共享
  `这个` / `个人`。

`material` 是**真正发出去的那份体**（`summary_body` 的 `RedactedBody`），不是第二次渲染出来的副本。

**4. 出处标签说实话（`soul-draft/src/analysis.rs`）**

`phrase_with` 现在四种情况退回计数：读不出来、空、诊断词、不 grounded。活下来的那一句由
`ENDPOINT_LINE_PREFIX` 引出：`整体来看（这一句是你自己的端点写的，不是本机算出来的，也没有证据支持；
本机只挡下了新出现的数字和跑题的回答，没有替你核对它说得对不对）：`。
点仍然是点：`SummaryPoint` 的证据、档位、非临床检查一个字节没动。

**5. 屏幕（`apps/desktop/src/routes/Graph.tsx`）**

- `user_endpoint` 那一行改成「带证据的每一条仍然是本机根据往来次数算的；最后『整体来看』那一句是你自己的
  端点写的，本机只挡下了新出现的数字和跑题的回答，没有替你核对它说得对不对。」——不再出现「根据本机统计改写」。
- 出网提示里的 `外加一句固定的改写要求` 改成 `外加一句固定的系统指令（只许把这些计数改写成一段话，
  不许添新事实、不许改数字）`——现在这句话描述的东西真的在请求体里。
- 摘要正文加 `className="reading"`（`white-space: pre-wrap`，样式表里既有的类）。此前核心按行写的文本
  在 `<p>` 里被折成一段，「每一行自带 依据 N 条记录」在屏幕上其实不成立。

## 没做到的那一半（**请复核时按这条读**）

`is_grounded_in` **不是忠实性判定**，代码与文档都明说了。`你们最近往来比较稳定，多数时候是一对一说话。`
里的「稳定」是计数没有的断言，它照样通过——而这句话正是 `session_e1.rs`（我不能改）与
`ipc_roundtrip.rs` 钉住必须是 `user_endpoint` 的那一句。我实测过：它与 `这个人最喜欢榴莲。` 在
字符二元组重合率上是 0.33 对 0.29，**任何阈值都分不开这两句**。所以这一版的保证是两条，不是三条：

1. 编数字 → 丢掉；跑题 → 丢掉（退回计数，与既有诊断词那条同一条路）；
2. 剩下的那一句，屏幕与正文都点名它是端点写的、没有证据、本机没核对。

要更强就得换机制（例如只允许端点在计数句之间做模板级重排，或者把叙述做成可回指的句-证据对），
那是产品面的改法，不是这一票能顺手加的。**AC-16 现在成立的读法是**：看起来像结论的每一行都拴着
evidence_ids，唯一不拴的那一行在自己那一行上写明了它不拴。

## 测试

| 命令 | 结果 |
|---|---|
| `cargo test -p soul-draft` | 绿（`people_summary` **17**，从 12 长上来的 5 条见下） |
| `cargo test -p soul-policy` | 绿 |
| `cargo test -p soulcore --test session_e1` | **28 绿**（未改动该文件） |
| `cargo test -p soulcore --test session_summary`（新文件） | **3 绿** |
| `cargo test --workspace` | **588 绿 / 0 红** |
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test ipc_roundtrip` | **54 绿**（含 opus-c 本轮在同一工作树里的改动） |
| `npx vitest run`（桌面壳全量） | **14 文件 172 项绿**；`Graph.test.tsx` **10** |
| `npm run lint`（tsc + eslint） | 绿 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 绿 |
| `cargo fmt --all -- --check` / `schema-freeze --check` / `e0-audit` / `denylist-audit` | 全绿 |

新增的钉子：

- `soul-draft`：`an_answer_about_something_else_is_dropped_and_the_counts_remain`（探针那句榴莲，
  `source` 回 `counts`、正文里没有那句、也没有端点行前缀）、
  `an_answer_that_states_a_count_nobody_computed_is_dropped`（`40 次往来` → 丢）、
  `a_chinese_numeral_the_counts_do_not_have_is_a_figure_too`（`四十次` → 丢）、
  `the_endpoint_s_line_says_on_the_line_that_it_is_the_endpoint_s`（前缀逐句钉，且**断言前缀里没有
  「根据本机统计改写」**）、`the_summary_instruction_asks_for_a_rewrite_and_forbids_new_facts`。
- `soulcore/tests/session_summary.rs`：产品路径。榴莲那次**真的发出去了**（`request_count()==1`）
  而屏幕退回计数、链上仍有 `egress.request`、链里搜不到那句；端点写的那句被显示时正文里有前缀；
  以及一条会话里先摘要再起草，**两次请求分别带自己那条系统指令**（用途不粘会话）。
- `Graph.test.tsx`：端点那句「写明是端点写的且不说是本机统计改写出来的」、核心退回计数时页面不提端点、
  出网那一行说的固定指令就是摘要那一条。

## 动过的文件

派给我的：`soul-policy/src/e1.rs`、`soul-draft/src/{analysis,reply}.rs`、
`soul-draft/tests/people_summary.rs`、`soulcore/src/commands/draft.rs`（只有 `Rephraser` 那一处调用与
两段文档，**`E1_PLAN_NOTICE` 一个字节没碰**）、`soulcore/tests/session_summary.rs`（新）、
`apps/desktop/src/routes/Graph.tsx` / `Graph.test.tsx`。

派单里没写、但改了的三处，请父代理核一眼：

1. `crates/soulcore/src/commands/policy.rs` — **必须**。`E1RequestPlan` 在这里建，用途到不了 `soul-egress`
   就等于指令常量白加。改动是纯新增（`e1_generate_for`），既有签名与调用点不变。
2. `crates/soul-draft/src/draft.rs` — 6 行。新增的 `ReplyDefect::Ungrounded` 让 `Degradation::of` 的
   `match` 不再穷尽；起草路径不调 `read_grounded_in`，所以那条分支到不了，映射成 `ReplyUnreadable`
   而不是 `unreachable!()`（端点不该能让本机 panic）。没有给 `Degradation` 加取值，`core.ts` 与
   `fakeCore.ts` 因此不用动。
3. `docs/STATUS.md` 第 755 行那一条与 `scripts/author-manual-checklist.md` 第 9 节 AC-16 那一步 —
   两处都逐字写着旧的「根据本机统计改写」，不改就是新的 copy-vs-behavior 缺口。只动了这两处，
   STATUS 的其余部分留给 fable / 父代理。

MUST NOT 清单（`redactor.rs`、`session_e1.rs`、`Wizard.tsx`、`Memory.tsx`、`ipc_roundtrip.rs`、
`fakeCore.ts`、`net_guard.rs`、soul-graph 阈值）一个都没动。

## 环境备注

本轮三个 opus 共用同一个工作树（我提交前 `git status` 里有 opus-a 已提交的 `e14425c` 与 opus-c
未提交的 `Memory*` / `ipc_roundtrip.rs` / `fakeCore.ts` / `session_screens.rs`）。我只 `git add` 了
自己的文件，两次提交都逐个点名，没有 `-A`。**`git push` 四次全部失败**：
`remote: Invalid username or token`（`origin` 里内嵌的 token 认证失败，不是网络错误）。
提交都在本地 `cursor/goal1-build-audit-c441` 上，需要有推送凭据的一方接手推。
