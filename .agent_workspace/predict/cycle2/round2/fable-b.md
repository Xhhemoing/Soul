MODEL_SLUG: claude-fable-5-thinking-xhigh

# Cycle 2 Round 2 fable-b — 可建议子集 vs 令牌门控动作：三分而非二分，边界写进类型

定位：下一动作子轨 Round 2。R1 里两份独立产出撞到了同一个 4 动作可建议集（本人 R1 fable-b §二白名单；opus-b R1 §2.2「建议集 ⊊ 预测集」表），本轮把这条边界从「一张要人记得维护的表」升级为**结构主张**：可建议性与令牌门控是两个不同问的谓词，禁止合并；并对本人 R1 的 A2 条款做一次自我收紧。读入：`hitl.rs` 全文、`docs/PRODUCT_LOCK.md`、`docs/STATUS.md`（AC-19 与 echo 机制段）、cycle2 R1 全部六份与 R1-SYNTHESIS。本轮独立，未依赖任何其他 Round 2 产出。不重开 P0–P8，不写产品 crate。

---

## 0. 一句话结论

**可建议集 ≠ ¬令牌门控集。** 8 个 `ActionKind` 分三类而不是两类：可建议 4 个、免令牌但仅限用户自发 2 个、令牌门控 2 个（v0.1.1 起 +1）。把 `!needs_capability_token()` 当可建议判据用，今天碰巧对 6 个、错 2 个，且错的方式是**静默漂移**——所以边界必须是独立的、穷尽匹配的、加新动作就编译失败的第二谓词。另：R1 fable-b A2 的「执行类建议携带 plan_hash」条款本轮**出局**，卡片载荷里不再允许出现任何批准链工件。

---

## 1. 冻结事实（源码坐标，任一被推翻则依赖它的论证重判）

| # | 事实 | 坐标 |
|---|---|---|
| F1 | `ActionKind` 8 值闭枚举；仅 `ExecuteForget`、`GenerateWithUserEndpoint` 需能力令牌 | `hitl.rs:24-41, 74-79` |
| F2 | `CapabilityScope` 三值：`E1Generate` / `ForgetExecute` / `FileWrite`；`FileWrite` v0.1 见牌即拒 | `hitl.rs:85-107, 308-310` |
| F3 | 闸门顺序：parse → 外部内容非权威 → plan_hash 比对（**仅当带了 approved hash**）→ 令牌 | `hitl.rs:452-497` |
| F4 | **免令牌动作的闸门是轻的**：`approved_plan_hash == None` 时，parse + origin 两关即放行 | `hitl.rs:465-481` |
| F5 | 令牌只在批准屏 echo `preparation_id` + `plan_hash` 两样之后铸造；`preview_id`/`preparation_id` 只活在 session 内存 | `STATUS.md` L349、L465、L538 |
| F6 | 令牌门控动作的 scope 映射含 `_ =>` 通配臂：`ExecuteForget => ForgetExecute, _ => E1Generate` | `hitl.rs:486-489` |
| F7 | `forget.preview` / `forget.execute` / `research.preview` 三个 token 全仓无生产者（G19） | opus-b R1 §1.2 |

F4 是本轮新抬出来的一条，它是整个边界问题的物理基础：**免令牌动作过闸便宜，所以「哪些动作可以被卡片主动递到用户面前」必须与闸门强度协变**——闸门轻的那一侧，值域必须只装后果最轻的动作。反过来，令牌门控动作的闸门重，但重闸门防的是执行完整性，不防「用户被推着走向那道闸门」。两侧各缺一半，边界谓词补的就是这一半。

---

## 2. 三分法（本轮的中心主张）

| `ActionKind` | 令牌 | 可建议 | 类 | 理由（一句话） |
|---|---|---|---|---|
| `draft.reply` | 否 | **是** | S | 只起草不发送；产物本机可逆 |
| `analyse.people` | 否 | **是** | S | 只读已存证据 |
| `scan.directory` | 否 | **是**（与 `plan.files` 打包为一次点击） | S | 只读 |
| `plan.files` | 否 | **是** | S | 只读预览，`executable_in_this_version: false` |
| `forget.preview` | 否 | **否** | U | 建议「看看忘掉 X 的代价」= 朝销毁方向的推力；遗忘是用户权利（#5），不是代理目标 |
| `research.preview` | 否 | **否** | U | 同意压力；研究轨是第三条轨道，助手轨不拉人（#7，「向导不弹一堆权限」的延伸） |
| `forget.execute` | ForgetExecute | 否 | T | 不可逆 |
| `egress.generate` | E1Generate | 否 | T | 出网花钱；产品锁 E1 原文「仅用户触发的生成」 |
| （v0.1.1 文件执行） | FileWrite | 否（**本轮预登记**） | T | 写文件系统；见 §3 |

三类的定义不共享判据：

- **S（可建议）**：卡片可以主动出现并在点击后构造 `origin=User` 的 `ActionRequest`。
- **U（免令牌但仅限用户自发）**：合法、免令牌、随时可从菜单进入，但**任何卡片不得指向它，连导航都不给**。排除理由是推力方向（销毁压力 / 同意压力），**不是**「未实现」——F7 的三个孤儿 token 日后补上生产者，U 类判定一个字不变。这是 G19 修复时最容易被静默翻掉的一格，§6 的 T6 为它立回归钉。
- **T（令牌门控）**：永不可建议，永不进卡片载荷；至多经**导航卡**到达自己的屏（fileplan 计划屏可以；遗忘屏与研究同意屏按 U 类纪律连导航也不给）。

**两个不塌缩方向都有反例，所以谓词必须是两个**：

1. `T ⊂ ¬S` 是硬蕴含（可性质测试：∀k, suggestable(k) ⇒ !needs_capability_token(k)——opus-b R1 §2.2 已立此式，本轮保留）；
2. `¬S ⊄ T`：`forget.preview`、`research.preview` 是活反例。若把可建议性实现为 `!needs_capability_token()` 一行，明天有人加一个无害的免令牌 preview 动作，它就**自动**变成可建议——没有任何测试会红。边界的敌人不是今天写错，是明天漂移。

---

## 3. 对 R1 的自我收紧：执行类建议整族出局

R1 fable-b A2 写过：「执行类建议必须携带所指计划的 `plan_hash`；目录一变，卡自动降级为重新预览。」本轮裁定这整句**出局**，v0.1.1 也不复活。三条独立理由：

**回声论证（决定性）**。F5：令牌铸造以批准屏原样 echo `preparation_id` + `plan_hash` 为前提，而这两样只在用户**看过那一屏**之后存在、只活在 session 内存。一张卡片没让用户看屏，就没有可 echo 之物。于是「指向 T 类动作的建议捷径」只有两种存在方式：要么点击后老老实实打开完整批准屏——那它省了零次点击，等价于导航卡；要么它试图带着旧 hash 绕过屏——那是 R1 X7（TOCTOU 族），AC-19 现有测试直接拒。**中间态在机器上不存在，所以禁令的产品代价恰好是零。**一条零成本就能换来「可建议 ∩ 令牌门控 = ∅ 永真」的不变式，没有理由不买。

**爆炸半径论证**。卡片的触发因子是白名单计数（授权根新增文件数、沉寂天数），但**计数本身是外部可影响的**——别人给你发 14 个文件，就改变了你的触发事实。这与 X5 不同：X5 禁的是内容语义进建议，这里即便只吃合规计数，计数仍是攻击面。值域封顶后，触发操纵的最坏收益 = 弹出一张指向只读动作的卡片；不封顶，它是指向不可逆动作的远程推力通道。

**习惯化论证**。HITL 链验证计划完整性，不验证用户注意力。反复出现的「执行」按钮训练的是点击反射，同意疲劳是链看不见的攻击面。缺一张建议卡的代价是用户多一次导航；多一次被推着做出的错误批准的代价是不可逆损失——不对称成立，从严侧赢。

**随之收紧的载荷纪律**：卡片类型里没有 `plan_hash`、`preparation_id`、`token_id` 字段——不是「不填」，是**类型上没有槽**。R1 A3 的「已批准未执行的整理计划」提醒降级为导航卡：打开计划屏（屏会重算当前 hash，陈旧性问题在屏上消解），卡上只有存在性事实，零批准链工件。

---

## 4. 反方钢人与回应

钢人（替「允许建议令牌门控动作」说到最强）：链是完整的，卡片只是入口，`check_action` 与 `consume` 把关一切，禁止建议是在 UI 层重复策略层已有的保证，白付一层维护成本。

回应：前提「链把关一切」只对**执行完整性**成立。链回答「这份计划是不是他批准的那份」，不回答「他为什么站在批准屏前面」。把用户推到批准屏前面这件事本身就是一种权力，而这种权力今天没有任何闸门管——边界谓词就是给它立的闸门。且由 §3 回声论证，被禁掉的东西本来就没有合法的存在形态，所以「维护成本换零收益」的指控方向恰好反了：**允许**才是为一个空集合维护特例。

---

## 5. 机制形状（v0.2 候选规范，本轮不实现、不进产品 crate）

```rust
enum Suggestability {
    Suggestable(SuggestableAction),          // 4 值闭枚举，卡片值域
    UserInitiatedOnly { reason: &'static str }, // reason 禁止是「未实现」
    TokenGated,
}
fn suggestability(k: ActionKind) -> Suggestability {
    match k { /* 8 臂逐一显式，禁 `_ =>` 通配 */ }
}
```

- **穷尽匹配禁通配**是全部机制里最重的一条：加第 9 个 `ActionKind`，编译失败，直到有人显式登记它的类。与 opus-b R1 §2.1 词表同步纪律同形，两处应引用同一条纪律条款。
- **卡片载荷**：`action: SuggestableAction` 或 `navigate: ScreenId`（`ScreenId` 是闭集，且不含遗忘屏与研究同意屏），二选一。无自由字符串，无批准链工件槽。
- **建议器符号隔离**：建议器模块对 `ActionRequest` / `TokenIssuer` / `CapabilityToken` / `PlanHash` 的引用数恒 0（opus-b R1 §2.4 门 1 的超集）。
- `abstain.not_suggestable`（opus-b R1 §2.3）保留：预测器可以输出 8 值，建议器值域 4 值，两者之差走弃权，不走静默丢弃——差值可数，词表漂移才可测。

**顺带钉住 F6 的通配隐患（本轮新发现）**：`hitl.rs:486-489` 的 scope 映射用 `_ => E1Generate` 兜底。今天无害（`_` 只覆盖 `GenerateWithUserEndpoint` 一个），但 v0.1.1 新增令牌门控动作时有两条坏路径：消费侧不改 → 对文件执行索要 `e1.generate` 令牌，若铸造侧独立正确则 `ScopeMismatch` 拒绝——fail-closed 但理由码是错的，审计上像令牌 bug 而不是映射 bug；若铸造侧照抄同一映射 → 为文件写铸出 `e1.generate` 令牌，「令牌命名能力，不是菜单项」（`hitl.rs:82-84` 自己的注释）失义，实质 fail-open。处方与 suggestability 同一条：**该 match 穷尽化、禁通配**，并加单测断言每个 `needs_capability_token()==true` 的动作都有显式 scope 臂。

**N 轨值域封顶的继承**：R1-SYNTHESIS 里的 N2（待办状态规则）与 N-learn（批准动作转移）全部继承本边界——N2 规则表每行的输出必须 ∈ S ∪ 导航；N-learn 即便日后 G17 解除拿到输入，其输出字母表**由构造**是 4 值 `SuggestableAction`，学习器学不出「建议遗忘」「建议出网」，这是类型保证，不是训练目标。学习候选的任何评测都不必再单测这一条。

---

## 6. 证伪清单（日后实现时逐条转测试）

| # | 测试 | 断言 |
|---|---|---|
| T1 | 性质 | ∀k：`suggestability(k)` 为 `Suggestable` ⇒ `!k.needs_capability_token()` |
| T2 | 编译门 | `suggestability` 与 F6 的 scope 映射均无 `_ =>`；加枚举值必须改这两处才能编译 |
| T3 | 载荷 schema | 卡片序列化 JSON 的键集合不含 `token_id` / `plan_hash` / `preparation_id`（快照 + schema 双查） |
| T4 | 符号隔离 | `rg 'ActionRequest|TokenIssuer|CapabilityToken|PlanHash' <suggester>/src` 命中 0 |
| T5 | 注入回放 | 祈使句文件名 fixture 下，渲染出的全部卡片的动作 ⊆ 4 值 S 集，导航目标 ∉ {遗忘屏, 研究同意屏}（R1 X5 用例族的扩展） |
| T6 | G19 回归钉 | 为三个孤儿 token 补上生产者后，`forget.preview` / `research.preview` 仍为 `UserInitiatedOnly` 且 reason 非「未实现」字样 |
| T7 | 点击路径 | 卡片点击后恰好一次（`scan`+`plan` 打包时两次）`check_action`，全部发生在点击之后；渲染阶段 `check_action` 调用数为 0 |
| T8 | 零铸造 | 任何只含建议器的 headless 回放里 `TokenIssuer::issued_count()` 恒 0——卡片存在本身永不导致令牌铸造 |

---

## 7. 留给 R3 的争点（本轮不裁）

1. **U 类连导航都不给是否过严**：遗忘的可发现性只剩记忆管理界面自身。本轮立场是够用（遗忘是用户想起来才做的事，不是被提醒去做的事），但这是产品判断不是机器判断。
2. **打包建议的 token 形状**：`scan.directory`+`plan.files` 打包后，建议 token 是 `plan.files` 单值还是显式二元组？涉及 opus-b FN1-2 合并规则的同一裁决，两处应一起定。
3. **「让 Soul 试着给建议」开关**（opus-b R1 §6.3 末尾）与本边界的关系：本轮立场是开关只管**要不要弹卡**，永不扩**卡能指什么**——值域封顶不随任何开关移动。若 R3 有人想把开关做成分级放宽值域，先过 §4 的钢人再说。

---

*本文件为 Cycle 2 Round 2 独立产出，写入面恰为本文件；不改锁文档，不写产品 crate，未做任何 git 操作。*
