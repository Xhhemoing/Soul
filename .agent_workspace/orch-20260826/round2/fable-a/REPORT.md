MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 2 / fable-a — 治理与主干裁决审计

只读审计。分支 `cursor/project-status-audit-c49c`（未离开、未提交）。核于 2026-08-26 03:00–03:30 UTC，全部结论来自 `git fetch/show/diff/merge-base` 与 `gh pr view` 实查。输入：Round 1 六份报告（重点 fable-a、gpt-sol-b）+ `main` @ `a0ec14b` 权威面 + 四条远程线的树内文件。

---

## 〇、本轮新钉死的事实（R1 报告之外，裁决必须知道）

先给分叉拓扑的精确数字（全部 `git rev-list --count` / `merge-base --is-ancestor` 实测）：

| 线 | 尖端 | 相对位置 |
|---|---|---|
| `cursor/soul-goal1-7b1c`（PR #2，D49 主干） | `6d1058b` | 分叉点 `5309656` + **1 个提交**（纯 docs：billing 归因说明） |
| `cursor/first-test-candidate-c441`（PR #14 → goal1） | `85aae68` | 分叉点 `5309656` + **77 个提交** |
| `cursor/soul-integration-4a8e`（PR #15 → first-test-candidate） | `a27cfd1` | first-test-candidate + **140 个提交** |
| `cursor/beadflow-integration-c441`（PR #16 → first-test-candidate） | `83818fc` | first-test-candidate + **113 个提交** |

PR 可合状态（gh 实查，2026-08-26）：#2 CONFLICTING/DIRTY、#4 CONFLICTING/DIRTY、**#6 MERGEABLE/CLEAN**、**#7 MERGEABLE/CLEAN**。

六个 R1 没有拿到的新事实：

1. **soul-integration 在支线上改了 `PRODUCT_LOCK.md` 的产品语义，且没有 D 号。** 相对其基线（first-test-candidate）改动 2 处：把 v0.1 导入的 Telegram 口径从 main 锁定的「Export chat history → Machine-readable JSON」改成「Settings → Advanced → Export Telegram data 全量导出；**单聊 Export chat history 是另一形状，拒收**」，并重写了对应 ASSUMPTION 行。main 明文承诺接收的格式，支线现在宣布拒收——这是产品承诺的语义翻转。它新增的 D61–D63 都不覆盖这条；PLAN_INDEX §5 明写「改产品方向：先改 PRODUCT_LOCK 并在 DECISIONS 追一条」。这比 D61–D63 撞号更硬：撞号是登记程序问题，这条是**无痕改锁**。
2. **soul-integration 正在单向收敛 main 的合同树。** 其 `docs/DECISIONS.md` = main 版逐字 + D61–D63 三行；`docs/algorithms/*` 整目录从 main 搬入；`relationship.schema.json` 与 `schemas.lock.json` 已与 main **逐字节一致**。「有意合同冲突」正在被支线自己一片一片消化，只剩 PRODUCT_LOCK（含上条私改）与 FORMAL 未收敛。这是裁决时可以利用的收敛信号。
3. **M2 的实质在 soul-integration 上已完成第三份实现。** 其 `Cargo.toml` 成员表 = goal1 的 16 个 + `soul-algo-tie` + `soul-algo-trait`（共 18），crate 目录俱在。至此 T4D/算法 crate 吸收存在三份互不引用的载体：PR #7 分支（CLEAN 无人合）、soul-integration 的 port（AD-2 明说「port，不 merge #7」）、以及 BLOCKERS §5 步 2 设想但从未发生的「main merge 进 goal1」。
4. **两条集成线在 `docs/agent-decisions.md` 同路径撞车。** soul-integration 与 beadflow 各自新增了这个文件，内容不同（diff 43 行），两个 PR（#15、#16）都以 first-test-candidate 为 base——先合者赢，后合者必然冲突。兄弟产品的共存问题已经不是抽象登记缺口，是一颗已埋好的合并地雷。
5. **first-test-candidate 的 `docs/BRANCH_MAP.md` 有一处事实错误。** 它声称「main 与唯一主干没有可用 merge-base（主干根 `d510f1e`，main 家族根 `ea6f62f`）」。实查：两侧共享同一个根 `ea6f62f`，merge-base 存在（= 初始提交，与 R1 gpt-sol-b 一致）。「合并=换合同、冲突面大」的实践结论仍成立，但这份被测试栈当作分支处置依据的文档在最关键的拓扑主张上是错的，收编时必须修。
6. **BeadFlow「不改写 Soul」在文件级有一个登记过的例外。** 194 个改动里 193 个是新增，唯一的 M 是根 `pnpm-lock.yaml`（BD16 自己登记了）。新增文件落在 Soul 的权威命名空间内：`docs/bead/**`、`docs/agent-decisions.md`、`docs/agent-progress.md`、`crates/bead-core`（自带独立 `Cargo.lock`，未进根 workspace，BD3）、`.github/workflows/bead.yml`。其 `docs/agent-decisions.md` 还写着「Soul 拍板仍在 docs/DECISIONS.md（**D1–D31**）」——main 已到 D60，过期两代。

---

## 一、交叉核验（同意 / 修正 R1 哪条）

### 议题 1：BeadFlow 在 Soul 仓的地位 vs PRODUCT_LOCK「唯一产品权威」

**同意** R1 fable-a P0-2 的主体判断（治理真空、main 权威面零登记）与 gpt-sol-b「远程分支膨胀、误选基线风险」（bead-* 分支 45+ 个实存）。

**修正一处定性**：R1 说「唯一产品权威这句话在仓库层面名不副实」。PRODUCT_LOCK 第 4 行的原文语义是「`docs/PRODUCT_LOCK.md` 是 **Soul 产品定义**的唯一权威，禁止再建 `PRODUCT.md`」——防的是 Soul 自己的双源，字面上并没有承诺「本仓库只有一个产品」。BeadFlow 全程没建第二份 PRODUCT.md，没有改写 Soul 锁（BD1 明文自我约束）。所以准确的定性不是「PRODUCT_LOCK 被违反」，而是**「仓库级产品登记」这件事没有任何权威文件在管**：PLAN_INDEX 自称「30 秒定位仓库权威」、拓扑三行自称读代码前先看——两者对同仓 41 个 PR、763 项测试、19 条 BD 拍板的第二产品零字。D41「计划权威只在 docs/」在 BeadFlow 侧成了镜像空洞：它的计划权威活在支线的 `docs/bead/**`，main 的 docs/ 不知情。

**补强 R1 未见的三点**：(a) PR #16 body 首句是「按**用户确认**的拼豆规划」——作者已确认 BeadFlow 存在本身，所以裁决对象不是合法性，是登记与边界；(b) BD15 显示 BeadFlow 已在**单向服从** Soul 的治理工具（`xtask e0-audit` 与 denylist 跨树扫描逼着 bead 代码禁外网 URL 字面量、禁「score/得分/分数/评分」）——共存耦合是双向的、真实存在的，只是没有一份权威文件写下来；(c) 事实 4 的 `docs/agent-decisions.md` 撞车与事实 6 的命名空间占用说明：不裁「docs/ 命名空间规则」，下一次撞车就在权威目录里发生。

**还有一层 R1 未点透的结构风险**：PR #16 的 base 是 first-test-candidate——Soul 的「第一次正式测试候选树」。一旦 #16 被合入，这棵测试候选树将携带第二个产品的 194 个文件；若议题 2 裁定该栈为后继主干，BeadFlow 就顺着搭车进入 Soul 主干。两个议题因此是**耦合的**：BeadFlow 的落点裁决必须在主干裁决之前或同时完成。

### 议题 2：D49「唯一实现主干」是否已被温水违反

**结论：字面已失守，但违反的形态与 R1 的描画需要分层修正。**

**同意**：R1 fable-a P1-2（D49 的「关 PR #4」悬置——本轮复核 #4 仍 OPEN、CONFLICTING、三检查失败，与 gpt-sol-b 一致）；fable-b 第 5 条「合并路径无 owner」——本轮升级为**四线**（goal1 / unblock#7 / first-test-candidate / soul-integration）合流路径无 owner。

**修正 R1 fable-a 对三层栈的一揽子描画**，按程度分层：

- **first-test-candidate（PR #14）：基本不构成违反。** 它与主干的分叉只有主干侧 1 个纯 docs 提交（billing 说明）；「unique trunk fast-forward」的自称核于 2026-08-25 时为真，现在只是过期一格，修复成本是 cherry-pick 一个提交。它的 PR 正对主干（base = goal1），姿态是「请主干收编我」，符合 FORMAL 开工第一动作第 4 步的框架。它的问题不在拓扑，在文档质量（事实 5 的 BRANCH_MAP 错误）。
- **soul-integration（PR #15）：温水违反的主体，且比 R1 记载的更深。** 四件事叠加：(1) 自封「exclusive parent branch」并把 → main 标为 BLOCKED，而 FORMAL 第 4 步允许的「经宣布的后继主干」**没有任何宣布程序发生过**——谁宣布、落哪份文件、程序是什么，权威面全空，自封因此无从被判定合法或非法；(2) 绕开 PR #7 重新实现了 G1/G2/G3/M2 的全部实质（事实 3），使 D50「在 Goal 1 线上一次 merge 完成 M2」的拍板被事实架空；(3) 在权威登记簿上追加 D61–D63（R1 已见）之外，还无 D 号改动了 PRODUCT_LOCK 的产品语义（事实 1，R1 未见）——这条是全案里唯一一处「改锁不留痕」，性质最重；(4) COMMANDS 36→38（`correct_tie`/`release_tie`），产品命令面已领先主干与 PR #7 两代，越晚裁决收编成本越高。
- **但必须如实记录它的自我约束**：AD-1 引 D49 自证从主干长出；AD-3 拒合 #4；AD-2 明文「不把 PR #7 当替代主干」；AD-5 明文「→ main BLOCKED 直到 M2 作为专门 merge 完成」；BRANCH_MAP N6 明文「只有作者能裁是否吸收 PR #7」。这不是 dev-sota 式的第二条野线，是一条**在等待裁决、且正在主动向 main 收敛（事实 2）的延长线**。「温水」的确切含义是：主干自身停止生长（分叉后仅 1 个 docs 提交）+ 后继宣布程序缺失，让 exclusive 自封成了无人能反驳也无人能追认的悬置状态。
- **beadflow-integration：不构成对 D49 的违反**（它不做 Soul 实现，BD13 明文不碰 Goal 1/PR #4/#7/#10），但它把自己挂在候选栈上，使 D49 裁决被动携带议题 1（见上）。

**一句话给父代理**：D49 的失守不是有人另立山头，而是「唯一主干」三个字的所有权没人行使——主干沉默、PR #4 没人关、后继程序没人走。裁决的重心应放在补程序与选后继，而不是定罪。

### 议题 3：PR #6（BLOCKERS）合入顺序 vs 集成线「不要合 #7/#4」

**先纠正一个容易读错的地方（R1 fable-a P0-1 的表述会引人误判）**：集成线的禁令（「Do not merge PR #7 or #4」「不要用合 main 修冲突」）与「合 PR #6 进 main」**零冲突**——#6 只往 main 加一份 `docs/BLOCKERS.md`，不碰 goal1 线，不碰集成栈，gh 实查 MERGEABLE/CLEAN。「两套下一步在打架」是真的，但打架的不是第 0 步（合 #6），而是 #6 合入之后 BLOCKERS §5 的阶梯。

**冲突的准确位置**（逐条对表）：

| BLOCKERS §5（`BLOCKERS_FROZEN`，核对主干 @`2e72ddf` 时代） | 分支侧现实（2026-08-26） | 判定 |
|---|---|---|
| 步 0：#6 合 main；停 dev-sota | #6 CLEAN 可合；#4 仍 OPEN | 无冲突，纯悬置 |
| 步 2：M2 = main merge 进 goal1；PR #2 mark ready | goal1 STATUS 明文「不要用合 main 修好冲突」；PR #2 CONFLICTING/DIRTY 系有意为之 | **正面顶撞** |
| 步 3–5：G1/G1+/G2/G3 在 Goal 1 线上做 | 实质已在 soul-integration 用另一套实现关闭（port，非 #7 合并） | **载体被事实超越** |
| 步 9：PR #2 merge 进 main；关 #3 #4 | soul-integration AD-5：→ main BLOCKED 直至 M2 专门 merge；自称 exclusive | **顺序模型已过期** |

即：BLOCKERS 的 P0 清单与「不许重开」判断依然有效，但 §5 工序的世界模型（只有主干 + unblock 两条线）落后现实（四条线）一整代。**合 #6 会把一份过期工序以权威身份放上 main**——这不是不合的理由，是合入时必须加限定的理由。

**撞号问题升级（加深 R1 fable-a P1-3）**：DECISIONS.md 脚注要求 #6 合入时把 BLOCKERS 正文的「D32」改写为「届时下一个空闲 ID」**或**加括注。若机械执行前者：main 上下一个空闲 ID 是 D61，而 soul-integration 已用 D61–D63——同一个号将出现第三种含义。本轮明确建议：**执行括注方案**（「dev-sota 编号，非 docs/DECISIONS.md 之 D32」），或跳号至 D64+；两种都应同时在 DECISIONS.md 登记「D61–D63 已被 soul-integration 线预定，合流时按内容审后收编或重编号」。

---

## 二、治理缺口（按根因归并，五条）

1. **后继主干宣布程序缺失。** FORMAL 开工第一动作第 4 步写了「或经宣布的后继主干」，但没写宣布主体（作者？父代理？）、落点（DECISIONS 追 D 号？STATUS？）、生效前置（PR #7 对账？测试门？）。这是三层栈自我授权得以发生、且无人能追认或否决的制度根源。议题 2 的一切裁决都要先过这里。
2. **D 号发号权在树间不闭合。** 「新增拍板只在本文件追加」没有回答「哪棵树上的本文件」。两条线并行追加必然撞号（已发生：BLOCKERS 的旧 D32、soul-integration 的 D61–D63、beadflow 的 BD 系列与「D1–D31」过期引用）。缺一条明文：支线拍板用线前缀号（如 AD-/BD-），D 号只在 main 上发；或支线可预定号段但须在 main 登记。
3. **仓库级产品/树登记缺位。** PRODUCT_LOCK 管 Soul 产品定义、PLAN_INDEX 管 Soul 权威定位、STATUS 管计划线——没有任何文件管「本仓库现有哪些产品、哪些活跃树、各自权威面在哪」。BeadFlow 的合法性只活在 PR body 的「用户确认」四个字里；`docs/` 命名空间无规则（`docs/bead/**`、两份撞车的 `docs/agent-decisions.md` 都是后果）。
4. **权威文件在支线被改动无告警、无对账义务。** soul-integration 无 D 号改 PRODUCT_LOCK（事实 1）今天只有靠人工 diff 才能发现。schema 侧有 `schemas.lock.json` 这样的锁-匙机制（虽然本树无校验器，R1 opus-b 5.3），产品锁与登记簿没有任何等价物。
5. **冻结标记不区分「冻结的判断」与「时点性的工序」。** `BLOCKERS_FROZEN` 把 P0 清单（判断，仍有效）与 §5 工序（时点建议，已过期）盖在同一个戳下。同病：goal1 STATUS 的「不要合 PR #7/#10」没有写明是永久裁决还是「在 M2 专门 merge 之前」的临时禁令，读者无从判断它何时失效。

---

## 三、给父代理的裁决选项（每题 2–3 个可执行选项；本报告不替作者拍板）

### 议题 1：BeadFlow 的仓库级地位

- **选项 1A（登记共存，推荐给作者优先评估）**：在 main 追加一条 D 号 + PLAN_INDEX 拓扑第四行，明文五件事：① BeadFlow 是同仓兄弟产品，权威面在 `cursor/beadflow-integration-c441` 的 `docs/bead/**`，不属 Soul 权威；② BD 号与 D 号两套发号互不占用；③ 共存边界（不进根 Cargo workspace、不进根 pnpm filter、Soul 的 e0/denylist 跨树扫描对 bead 的适用性照 BD15 收编成文）；④ `docs/` 命名空间规则（bead 文档只准落 `docs/bead/`；通用名如 `agent-decisions.md` 必须带线名或裁定归属，解除事实 4 的地雷）；⑤ BeadFlow 与 Soul 的合流关系（永不合进 Soul 主干 / 或指定专门收编树）。工作量：一条 D 号 + PLAN_INDEX 四~五行，全在 main docs。
- **选项 1B（迁出独立仓库)**：BeadFlow 全线迁走，本仓关闭 PR #16–#56，main 留一条 D 号记录迁出与理由。彻底解耦，但与作者「同仓」的既成确认相逆、涉及 45+ 分支迁移，必须作者本人点头。
- **选项 1C（最小止血，若作者短期不裁）**：只做两个动作：在 STATUS 登记「BeadFlow 存在于支线、权威面另在、归属待裁」；**冻结 PR #16 的合入**（不许合进 first-test-candidate），直到议题 2 裁决落地——防止第二产品搭车进入 Soul 的测试候选树/未来主干。

### 议题 2：D49 主干归属

三个选项共享一个**无争议前置**，建议无论选哪个都立即执行：**关闭 PR #4**（D49 明令 + BLOCKERS §5 步 0 + soul-integration AD-3，三方文件一致要求，无任何一方反对；gh 侧需作者或父代理有写权限的一方执行）。

- **选项 2A（追认后继）**：走一次正式「宣布后继主干」：在 main 的 DECISIONS.md 追加 D 号，宣布 soul-integration（或退一层宣布 first-test-candidate）为 D49 主干的后继；同一条 D 号里必须一并处置四件事——① goal1 侧那 1 个 docs 提交 cherry-pick 进后继；② PR #7 的终局（与 soul-integration 的 port 做一轮语义对账 diff 审计后合并或墓碑化——AD-2 的 port 是否等价于 unblock 线，目前**没有人验证过**，这是本选项的前置工作量）；③ D61–D63 收编或重编号；④ PRODUCT_LOCK 的 Telegram 私改（事实 1）补 D 号追认或回滚。优点：顺着 140 个提交的既成事实与收敛趋势（事实 2）走，成本最低；风险：等于承认「先斩后奏可以被追认」，必须在同一条 D 号里写明下不为例的程序（见缺口 1）。
- **选项 2B（回归主干）**：维持 goal1 为唯一主干，要求集成栈以 PR 逐级回灌：PR #14 先合回 goal1（近 FF，冲突面 ≈1 个 docs 提交），soul-integration 冻结 Round 17 后 rebase/merge 回灌并逐项审查（140 提交），PRODUCT_LOCK 私改在回灌审查中裁决。优点：D49 字面完整、审查最严；代价：审查量最大，且要先叫停一条正在跑的 20 轮线（其 PR body 自述 Round 17 in flight）。
- **选项 2C（程序先行，暂不裁人）**：先只补制度：在 FORMAL/DECISIONS 落一条「后继主干宣布程序」（宣布主体=作者或其明文授权的父代理；落点=DECISIONS 追 D 号；生效前置=PR #7 对账完成 + BRANCH_MAP 事实错误修正），同时要求两条 exclusive 线停用「exclusive」自称、冻结 → base 的合入直至程序走完。优点：不逼作者立刻选树；代价：四线并行状态延长，PR #7（CLEAN）与 soul-integration 的实现差每多一轮就多一分。

### 议题 3：PR #6 合入

- **选项 3A（立即合，带三件套，与议题 2 解耦）**：按 STATUS 下一步第 1 条合入，同 PR 内做三件事：① D32 撞号按**括注方案**处理（明确不用「下一个空闲 ID」方案，理由见交叉核验议题 3——D61 已被支线预定）；② 在 BLOCKERS §5 顶部加一行核于限定：「本工序核于主干 @`2e72ddf` 时代的两线拓扑；步 2–9 的载体与顺序需按四线现状（见 STATUS）重裁，清单判断不受影响」；③ DECISIONS.md 追加号段登记（D61–D63 被 soul-integration 预定待审）。优点：阻碍项权威终于进树、撞号一次除净、过期工序不会误导只读 main 的编排者；这是三个议题里唯一**今天就能安全执行**的动作。
- **选项 3B（原样合，只处理 D32 撞号）**：最小改动合入，§5 的过期问题留给议题 2 的裁决输出一并修。风险明确：合入后 main 上出现权威署名的过期工序，只读 main 的编排者照 §5 步 2 把 main merge 进 goal1，会正面撞上 goal1 STATUS 的禁令——这正是 R1 fable-a P0-1 警告的事故路径。
- **选项 3C（押后到议题 2 之后）**：先裁主干，再按裁决改写 §5 后合入。代价：STATUS「下一步第 1 条」自 2026-08-25 起的悬置继续延长，BLOCKERS 权威继续只能跨分支引用；#6 今天 CLEAN，押后越久与 main docs 的冲突概率越大。**不推荐**，除非议题 2 能在极短周期内裁掉。

### 三题的耦合顺序（给父代理的执行序建议，非拍板）

议题 3 选 3A 可先行（与其余解耦）；议题 1 与议题 2 耦合（PR #16 挂在候选栈上），若作者带宽有限，最小组合是 **3A + 1C + 2 的无争议前置（关 #4）**：三个动作都不预设主干归属，全部可逆或纯登记，把「失明、撞号、搭车」三个正在恶化的面先止住，再等作者裁 2A/2B。

---

## 附：本轮证据索引（可复核命令）

- 拓扑：`git merge-base --is-ancestor` / `git rev-list --count 5309656..{6d1058b,85aae68}`、`85aae68..{a27cfd1,83818fc}`
- 事实 1：`git diff 85aae68:docs/PRODUCT_LOCK.md a27cfd1:docs/PRODUCT_LOCK.md`
- 事实 2：`git diff origin/main:docs/DECISIONS.md origin/cursor/soul-integration-4a8e:docs/DECISIONS.md`（仅 +3 行）；schema/lock 同法 diff 为空
- 事实 3：`git show origin/cursor/soul-integration-4a8e:Cargo.toml`（18 members）
- 事实 4：`git diff origin/cursor/soul-integration-4a8e:docs/agent-decisions.md origin/cursor/beadflow-integration-c441:docs/agent-decisions.md --stat`
- 事实 5：`git merge-base origin/main origin/cursor/soul-goal1-7b1c` → `ea6f62f`；对照 `git show 85aae68:docs/BRANCH_MAP.md`
- 事实 6：`git diff --name-status 85aae68 83818fc`（194 A + 1 M）
- PR 状态：`gh pr view {2,4,6,7,14,15,16} --json state,mergeable,mergeStateStatus,body`
