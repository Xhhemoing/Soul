# Round 2 opus-b — CODE-3（AC-34 第二句）与 DOC-D61（补丁 1–4）

**实际 slug：`claude-opus-5-thinking-high-fast`（请求值同）。无静默降档。** 自证据：模型自称 Claude Opus 5、思考通道开启。补充：`cursor-cloud-run-info` 的 `originalModelName` 为 `null`，服务端没有回吐字面 slug，所以上面这行是「模型身份 + 请求值一致」的判断，不是服务端字段的原文回显；bcId `bc-827d63e7-3c1e-569e-9b82-7ef5528d4799`。

分支：`cursor/goal1-r2-opus-b-4799`，基线 `d3b83f3`（= `cursor/goal1-close-loop-a073` 当时的头，含 gpt-sol-b 的 LOCK-HYGIENE 提交，见 §5）。三个提交：

| 提交 | 内容 |
|---|---|
| `9ccadb6` | CODE-3：`crates/soul-import/tests/import_to_graph.rs` 扩测 |
| `a29f792` | 文档补丁 1 + 2 + 4（同一提交，补丁 4 依赖补丁 1） |
| `0418983` | 文档补丁 3（独立） |

---

## 1. CODE-3 — AC-34 第二句已成为活门禁

AC-34 的 Then 有两句。第一句（不产生 owner→A、owner→B 的 Outgoing 行）此前已由
`an_owner_group_message_does_not_write_one_outgoing_row_per_speaker` 覆盖。第二句
（「A/B 的 `last_contact` 不因这条消息刷新」）此前**无断言**。

按派单要求**扩展既有测试**，未新建第二份夹具、未引入第二条归因规则：

- 把两处时间戳字面量收成单一来源：`OWNER_SENT_AT` 常量与 `peer_spoke_at(speaker)`
  函数，构造导出行与断言期望值共用同一份。六个 peer 在 `09:01`–`09:06` 各说一句，
  owner 在 `10:00` 说一句——owner 的时刻**晚于全部 peer**，所以一旦被借用就会成为
  边上最新的那个瞬间，不会被 peer 自己的流量掩盖。
- 新增断言（在原有 rebuild + load 之后）：
  1. 每个 peer 恰有一条边；该边 `tie_strength.last_contact_utc` == 该 peer 自己发言的分钟；
  2. `graph.node(peer).last_contact_utc` 读到同一个瞬间（节点侧是 `view.rs` 对
     touching 边取 max，独立于边侧字段，所以这条不是重复断言）；
  3. `last_direct_contact_utc` 为 `None`（这六人从未一对一）；
  4. 全图**没有任何一条边**的 `first_contact_utc` / `last_contact_utc` 等于 `OWNER_SENT_AT`。

**未改任何产品代码。** `commit.rs` 的 `(true, true) => Vec::new()`（G1+ 禁止出向扇出）
原样保留；`soul-graph/src/build.rs` 的 `PeerEvidence::absorb` 原样保留。第二句之所以
现在就成立，正是因为 owner 的群消息根本不产生 peer 行，`absorb` 也就永远看不到
`10:00`；本次工作是把这个已成立的性质钉成门禁，不是修行为。

### 负控（证明断言不是空转）

只断言「现在是绿的」不能证明它挡得住回归。做了一次一次性变异实验并已完全回滚：

- 变异：`crates/soul-import/src/commit.rs` 的 `(true, true)` 分支改回旧的花名册扇出
  （`speakers.get(&conversation_id)`），即 G1+ 之前的行为；
- 为了绕开先触发的旧断言（出向计数为 0 等），临时另建 `tests/tmp_probe.rs`，只含新增的
  last_contact 断言；
- 结果：

```
thread 'last_contact_only' panicked at crates/soul-import/tests/tmp_probe.rs:68:9:
assertion `left == right` failed: u-p1 last contact
  left: "2026-08-20T10:00:00Z"
 right: "2026-08-20T09:01:00Z"
```

owner 的 `10:00` 确实会顺着扇出爬到 A、B 的 `last_contact` 上。**第二句不是第一句的
推论，它有自己要挡的东西。** 变异与探针文件均已删除（`git status` 干净，见 §4）。

### 未做（守派单边界）

- 未新造第二套阈值、未碰 `soul-algo-*`；
- 未引入第二条归因规则；owner 的群消息仍然是「有 event、无 evidence」，与 D47 / G1+ 一致；
- 未碰 `crates/soul-graph/src/correct.rs`（Round 1 已关）、未碰 `.github/workflows/ci.yml`；
- 未动 `crates/soul-graph/tests/t4d_product.rs`（那是 opus-a 的 CODE-2，见 §5）。

---

## 2. 文档补丁 1–4 — 四处全部按原文施加

施加前逐条用「旧文本」整句匹配定位，四处**全部一次命中**，无行号漂移导致的歧义。

| 补丁 | 文件 | 结果 |
|---|---|---|
| 1（必做） | `docs/DECISIONS.md` | D60 行之后追加 D61 整行，逐字符照抄 |
| 2（必做，与 1 同提交） | `docs/PLAN_INDEX.md` :12 | `D1–D60` → `D1–D61` |
| 3（必做，独立提交） | `docs/PLAN_INDEX.md` :26 | 末句替换；替换后整格与补丁文件给的「供施加者核对」全文逐字相同 |
| 4（可选，依赖 1） | `docs/PLAN_INDEX.md` :25 | `D49（唯一实现主干）` → `D49/D61（唯一实现主干）` |

补丁 4 是可选项，取「做」：D61 落地后单看 D49 会读到旧主干名，而 :25 正是新人查询入口。

### 补丁文件明令「刻意不改」的地方，一处没动

- **D49 原行**：一个字符未改（`git diff docs/DECISIONS.md` 是纯 `+1` 行，无 `-`）；
- `FORMAL_WORK_PROMPT.md:16` 的「计划权威面（D1–D60…）」：未改（描述的是历史 merge 吸收事件）；
- `STATUS.md` :9 / :23 的「D41–D60」与 :817 的「D1–D60」：未改；
- `DECISIONS.md:73` 关于「PR #6 合入时把 BLOCKERS 假 D32 改写为下一个空闲 ID」那段：未改
  （D61 占用后空闲 ID 从 D62 起算，届时按表尾取号，本轮不预写）；
- **`docs/algorithms/` 三份冻结文档（`DECISION.md` / `COPY_ZH.md` / `REJECTED.md`）：一字未动**
  ——派单点名的 `COPY_ZH.md` 尤其确认未触碰（`git diff --stat` 里没有它）；
- `ci.yml`、PR #4 的 GitHub 记账：按补丁文件「明确不提补丁的项」，不动。

补丁 3 值得说明它**没有**放宽什么：主句「判档阈值只有两处定义」原样保留，新句只是把
豁免的依据从「点名两个文件」换成「`docs/algorithms/` 三份是裁决记录本身」，从而把
`DECISION.md` §1 自己的「180/360 整数降档」纳入豁免（R-2 残余），同时把 D60 的引用
收窄回它真正钉的事（P4 比较符），不再让它兼职当「可引用数字」的一般授权。

---

## 3. 测试

在隔离 worktree 里跑（原因见 §5），基线是本分支 `0418983`：

```
$ cargo test -p soul-import --offline
   unittests src/lib.rs        4 passed
   tests/import_to_graph.rs    6 passed   ← 含 CODE-3
   tests/injection_is_data.rs  6 passed
   tests/questionnaire.rs      8 passed
   tests/soul_import_v1.rs    11 passed
   tests/telegram.rs          10 passed
   doc-tests                   0
   总计 45 passed; 0 failed
```

`soul-graph` 作为 `soul-import` 的 dev-dependency 一并编译通过（`import_to_graph.rs`
直接调 `soul_graph::rebuild` / `load`）。另跑 CI 会跑的两道：

- `cargo fmt --all -- --check` — 干净；
- `cargo clippy -p soul-import --all-targets --offline -- -D warnings` — 干净。

文档面无 CI 门禁：`crates/xtask` 不读 `PLAN_INDEX.md` / `DECISIONS.md`，`ci.yml` 里也没有
文档 lint，所以补丁 1–4 是纯文本改动，不会改变任何 job 的红绿。

---

## 4. 工作树卫生

本分支相对基线 `d3b83f3` 只动三个文件：

```
 crates/soul-import/tests/import_to_graph.rs | 61 ++++++++++++++++++++++++++---
 docs/DECISIONS.md                           |  1 +
 docs/PLAN_INDEX.md                          |  6 +--
```

`Cargo.lock` 未动，`schemas/` 未动，`crates/soul-algo-*` 未动，`.github/` 未动，
`docs/algorithms/` 未动。负控用的 `commit.rs` 变异与 `tests/tmp_probe.rs` 都已回滚/删除，
`git status` 干净。

---

## 5. 需要父代理知道的一件事：本轮多个槽位共用同一个 worktree

`/workspace` 被 Round 2 的多个槽位同时使用，期间发生了互相踩踏，值得记一笔：

1. 我在 `/workspace` 建了 `cursor/goal1-r2-opus-b-4799` 并提交 CODE-3。提交落地后
   `git branch --show-current` 已经变成 `cursor/goal1-r2-code2-df52`（opus-a 的 CODE-2 分支）
   ——另一个槽位在我提交的间隙 `checkout -b` 走了 HEAD。**结果：CODE-3 的原始提交
   `032374f` 落在了 opus-a 的 CODE-2 分支上。** 我没有 reset 那条分支（会撞上 opus-a
   正在进行的编辑），改为 cherry-pick 到自己分支（`9ccadb6`）。两条分支上会有同一份
   patch 的两个提交，内容逐字相同，合并时其中一个会成为空提交，不会冲突。
2. 与此同时，gpt-sol-b 的 LOCK-HYGIENE 提交 `d3b83f3`（"docs: record Round 2 lock hygiene
   probe"）是在 HEAD 指着**我的分支**时打下的，所以它同时在 `cursor/goal1-r2-opus-b-4799`
   和 `cursor/goal1-close-loop-a073` 上。两条分支在这一点上不分叉，我没有把它剔掉
   （剔掉要 force-push）。
3. 我随后 `git worktree add /tmp/wt-opus-b cursor/goal1-r2-opus-b-4799` 隔离，§1–§4 的
   全部编辑、测试与提交都在那里做，`/workspace` 从那一刻起没有再被我改过。

**建议**：后续多槽位并发轮次给每个槽位单独的 worktree，或至少约定不在共享树上
`checkout`。本轮的踩踏没有造成内容丢失，但「我的提交出现在别人分支上」这种事下一次
未必这么无害。

---

## 6. 遗留 / 交接

- CODE-3 关闭：AC-34 两句 Then 现在都有断言，第二句经负控证明是活门禁。
- DOC-D61 关闭：补丁 1–4 全部施加，D49 未被改写，`COPY_ZH.md` 未被触碰。
- 给 AC-REPROBE（gpt-sol-a）：重跑映射时 AC-34 可从 PARTIAL 改判——两句 Then 都在
  `crates/soul-import/tests/import_to_graph.rs::an_owner_group_message_does_not_write_one_outgoing_row_per_speaker`
  一个测试里，跑在产品边界（导入 → 加密库 → rebuild → 读边/读节点），不是直调算法 crate。
- 给父代理：本分支需要叠回 `cursor/goal1-unblock-a073`，不得对 `main` 开第二条合入 PR
  （R1-SYNTHESIS 与 fable-b 的裁定）。
