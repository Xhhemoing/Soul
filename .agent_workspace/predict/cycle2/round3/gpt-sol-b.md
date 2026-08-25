MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# C2R3：闭集 `ActionKind` 与生产调用点——冻结前独立核账

## 结论

`ActionKind` 的 **8 值策略词表**、**生产中真正经过 `check_action` 的 5 值集合**、以及
**用户实际能触发的 8 个同名语义操作**不是同一个集合：

```text
策略域 K（ActionKind::ALL）                  = 8
生产闸门生产者 G（能返回对应 ApprovedAction） = 5
已上线同名语义操作 P                         = 8
持久化的精确 ActionKind 历史 H                = 0

G = {draft.reply, analyse.people, scan.directory, plan.files, egress.generate}
P \ G = {forget.preview, forget.execute, research.preview}
```

所以 R1 的“8 个 token 中只有 5 个有生产者”若冻结，必须改写成：

> 三个 token **不是没有产品功能**；三个功能都已接到桌面 UI，但绕过了 `ActionKind` /
> `check_action`。对动作预测器而言它们不能记成用户选择次数为 0；对策略边界而言这是
> 词表与执行调用图分叉。

其中 `forget.execute` 不是纯记账瑕疵。中央策略明确要求 `ForgetExecute` 能力令牌并在销毁前
拒绝变化后的计划；上线遗忘路径只核对 `preview_id + memory_id`，不铸造、不消费能力令牌，
也不在销毁前比较影响面。当前实现甚至可以先按旧预览执行销毁，再在回执里报告
`matched_preview=false`。这应作为冻结目录中的阻塞项，而不是未来评测时把该 token 排除分母
就算关闭。

本文件独立读取生产源码与测试；未读取其他 Round 3 产出，不改产品 crate。

## 1. 审计口径

为避免继续混用“生产者”，本轮固定三个词：

1. **闸门生产者**：非测试生产代码构造该 `ActionKind` 的 `ActionRequest`，且该路径实际调用
   `check_action`，成功时得到 `Ok(ApprovedAction { kind, .. })`。
2. **语义生产路径**：安装后的桌面命令能执行枚举注释所描述的操作；它是否经过中央闸门另计。
3. **可观测动作事件**：精确保存 `ActionKind`、能供后继预测器读取的一条记录。返回值存在不算；
   调用方丢掉它就没有事件。

源码范围为 `crates/**/src/**/*.rs` 与 `apps/desktop/src{,-tauri}/**`，排除 `tests/`、
`#[cfg(test)]`、`target/`、`node_modules/`。Tauri 注册、Rust IPC wrapper 与前端调用三层都
核过；不是仅凭一个未使用的 `pub fn` 推断“上线”。

## 2. 逐值对账

| `ActionKind` / token | 中央闸门生产点 | 已上线入口 | 冻结判定 |
|---|---|---|---|
| `DraftReply` / `draft.reply` | `soulcore/src/commands/draft.rs:176` → `allow` → `:732` | `Session::draft_pasted`（`session.rs:679-683`）→ `draft_reply` IPC → `Draft.tsx:84` | **G ∩ P** |
| `AnalysePeople` / `analyse.people` | `draft.rs:564` → `allow` → `:732` | `Session::person_summary`（`session.rs:648-669`）→ `person_summary` IPC → `Graph.tsx:81` | **G ∩ P** |
| `ScanDirectory` / `scan.directory` | `soul-fileplan/src/preview.rs:88-96` | `Session::preview` → `preview_plan` IPC → `Files.tsx:70` | **G ∩ P**；同一次手势的内部第一步 |
| `PlanFiles` / `plan.files` | `preview.rs:101-104` | 同上 | **G ∩ P**；紧随 `scan.directory` |
| `PreviewForget` / `forget.preview` | **无**；除 `hitl.rs` 与测试外，生产源码无 `ActionKind::PreviewForget` | `Session::preview_forget`（`session.rs:1097-1110`）→ IPC（`commands.rs:284-291`）→ `Memory.tsx:112` | **P \ G** |
| `ExecuteForget` / `forget.execute` | **无**；除 `hitl.rs` 与测试外，生产源码无 `ActionKind::ExecuteForget` | `Session::forget_memory`（`session.rs:1125-1145`）→ IPC（`commands.rs:293-301`）→ `Memory.tsx:129` | **P \ G；安全语义分叉** |
| `PreviewResearch` / `research.preview` | **无**；除 `hitl.rs` 外，生产源码无 `ActionKind::PreviewResearch` | `Session::research`（`session.rs:1365-1371`）→ IPC（`commands.rs:303-309`）→ `Research.tsx:56` | **P \ G** |
| `GenerateWithUserEndpoint` / `egress.generate` | 真正生产点只有 `PolicySession::e1_generate`（`policy.rs:198-204`） | `generate_draft`；另有人事摘要端点改写（`draft.rs:632-649`） | **G ∩ P** |

两处容易误数：

- `draft.rs:197` 的 `prepare()` 虽传入 `GenerateWithUserEndpoint`，但 `allow` 在
  `kind.needs_capability_token()` 时于 `:727-730` 直接 `Ok(())`，**没有调用
  `check_action`，不产生 `ApprovedAction`**。若事件定义是“已批准动作”，准备期不能算一次
  `egress.generate`；真正一次在 `PolicySession::e1_generate`。
- `scan.directory → plan.files` 是一次 `preview_plan` 手势内的两个连续批准，不是用户连续作了
  两次选择。带端点的 `person_summary` 也可在同一手势内产生
  `analyse.people → egress.generate`。未来事件至少要有 `call_id/user_gesture_id`，否则 MRU /
  转移模型会把调用图学成偏好。

## 3. “闭集”实际闭住了什么

### 3.1 已闭住

- `ActionKind` 枚举与 `as_str()` 的穷尽 `match` 使未知字符串不能越过
  `ActionKind::parse`（`soul-policy/src/hitl.rs:24-70, 457-459`）。
- 当前 `ActionKind::ALL` 明列 8 值；测试把长度钉为 8，并验证切片里的每一项可往返
  （`soul-policy/tests/hitl.rs:70-80`）。
- 中央闸门对 `ExecuteForget` 与 `GenerateWithUserEndpoint` 要求令牌
  （`hitl.rs:74-79, 475-490`）。

### 3.2 没闭住

1. **生产命令面没有被枚举闭住。** 新增 Tauri / `Session` 命令不会迫使作者登记
   `ActionKind`；三个现成反例已经证明。
2. **`ALL` 的穷尽性不是类型事实。** 它是手写切片。新增第 9 个枚举值、补上 `as_str`，
   却漏改 `ALL` 时仍可编译；现有“长度等于 8”测试甚至会继续通过，而 `parse` 会拒绝新值。
3. **令牌 scope 映射有通配臂。** `hitl.rs:486-489` 用
   `ExecuteForget => ForgetExecute, _ => E1Generate`。今天 `_` 只剩 E1；未来若增加一个
   `needs_capability_token()==true` 的动作，编译器不会要求登记其 scope。
4. **批准结果没有形成历史。** `ApprovedAction` 的生产引用只在 `hitl.rs`、`policy.rs` 与
   `soul-fileplan/preview.rs`；产品只取 `plan_hash` / 把对象塞进短命 `Preview`，没有写入
   `kind` 的持久化或可遗忘事件。故今天 `H=0`，不是 `H=5`。

因此“闭枚举”只能证明中央 parser 的输入域，不能证明所有生产操作都经过它，也不能证明
未来加动作会在所有登记点编译失败。

## 4. `forget.execute` 的具体分叉

中央策略与产品路径今天同时各自测试通过，但测试没有把两者接起来：

- `ActionKind::ExecuteForget.needs_capability_token() == true`；
- 无令牌的请求由 `check_action` 返回 `TOKEN_REQUIRED`
  （`soul-policy/tests/hitl.rs:212-219`）；
- 令牌还绑定计划、过期、scope、一次性消费
  （同文件 `:100-113, 127-219`）。

上线调用图却是：

```text
preview_forget(memory_id)
  -> 保存 HeldForget { preview_id, memory_id, impact }

forget_memory({ preview_id, memory_id })
  -> 只比较两个 id
  -> memory_commands::forget(...)       // 已发生不可逆销毁
  -> ForgetReceiptView::of(..., impact)
  -> matched_preview = receipt.impact == quoted  // 事后才比较
```

坐标：`session.rs:1097-1110, 1125-1145`；
`commands/memory.rs:221-253`。`ForgetConfirmation` 类型只有两个字符串，没有 `plan_hash` 或
能力令牌字段。

最小反例不需要伪造 token：

1. 预览记忆 M，得到影响面 Q 与 preview id；
2. 在同一会话编辑 M；编辑会在同一内容密钥下留下新的密封 blob；
3. 交回旧 preview id；
4. 两个 id 仍匹配，代码先销毁，再发现实际影响面不等于 Q，只能返回
   `matched_preview=false`。

这与 D24“plan hash 变拒绝”方向相反：这里是“变化后照做，回执再承认不匹配”。它也说明
`preview_id` 是会话 nonce，不等价于绑定计划的一次性能力令牌。

本轮实跑：

```text
cargo test -p soul-policy --test hitl
15 passed

cargo test -p soulcore --test session_screens \
  a_forget_only_runs_on_the_preview_the_user_read
1 passed
```

两边都绿恰好证明缺的是跨层不变式：中央测试证明“若走 gate 则需令牌”，会话测试证明
“生产遗忘能完成”，没有测试断言“生产遗忘必须走 gate”或“影响面变化时销毁数为 0”。

这不是说当前 UI 能把文件正文伪装成点击：IPC wrapper 固定走用户手势。风险是调用图没有把
该事实编码在统一策略边界里；未来建议器或新的 Rust 调用者可直接调用 `Session` 方法，并且
不会遇到 `RequestOrigin`、plan hash 或 token gate。

## 5. 对动作预测候选目录的冻结

### F-ACTION-1：词表分母

- 策略设计词表仍是 8。
- 若数据源定义为 `Ok(ApprovedAction)`，今天生产可产生的字母表只能是上述 5；三个绕行功能
  必须标为 **unobserved-by-gate**，不得作为 0 次偏好计入分母。
- 今天没有可读的 `ApprovedAction` 历史，所以任何跨启动学习候选继续是
  `TESTABLE-AFTER-GAP`。不能因静态枚举存在就写成“已有 8 类训练数据”。

### F-ACTION-2：事件规范化

- `prepare_draft` 不记 `egress.generate`；它没有 `ApprovedAction`。
- 一次 `preview_plan` 的 `scan.directory + plan.files` 合并为一次用户机会。
- 一次 `person_summary` 内部追加的 E1 改写要用同一 `user_gesture_id` 标成机械后继，不能
  当成第二次自由选择。
- 拒绝不进入批准历史；事件必须由真实生产调用图返回的 `ApprovedAction` 产生，禁止手插 token。

### F-ACTION-3：阻塞项 G19 重写

旧措辞“`forget.*` / `research.preview` 无生产者”太弱且易误读。冻结为：

> **G19-production-gate-split**：三个 `ActionKind` 有已上线同名操作，但这些操作不经过
> 中央 gate；其中 `forget.execute` 绕过枚举声明的能力令牌与销毁前计划变化拒绝。

解除条件必须同时满足：

1. 三条生产路径都在操作前得到对应 `ApprovedAction`；
2. `forget.execute` 的影响面发生任何变化时，在内容密钥销毁前拒绝，销毁数为 0；
3. 每个 token-gated `ActionKind` 到 `CapabilityScope` 是无 `_` 的穷尽映射；
4. 有集成测试从桌面 IPC 入口证明以上性质，而不只直接测 `check_action`；
5. 若要供预测器消费，另建与内容同生命周期、可遗忘的精确动作事件；审计链不代替它。

### F-ACTION-4：闭集同步门

日后实现应让“枚举值 → token → gate scope → 生产 handler → 预测分类”成为一个穷尽登记点，
或至少用无通配 `match` 加编译门逐项覆盖。仅依赖 `ActionKind::ALL.len() == 8` 与全文 `rg`
不足以长期守住同步。

## 最终裁决

动作轴候选可继续保留为研究目录，但当前输入状态应写成：

```text
policy vocabulary:       CLOSED-8
production gated subset: 5
semantic UI coverage:    8, with 3 gate bypasses
forgettable exact log:   absent
productization:          BLOCKED
```

最坏的冻结方式是把三个 token 当作“合法但用户从不选择”，然后让模型从恒零计数学习偏好。
正确冻结是先承认生产调用图与闭集分叉；尤其先修复遗忘的跨层不变式，再谈 8 值 next-action
预测。
