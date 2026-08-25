# Round 2 fable-a — 合并风险分析（本 PR → main，随后 Goal 1 并入 main）

核于 2026-08-25 02:50 UTC。**结论不是推演，是实测**：在临时 worktree 把「main + 本 PR（含在飞改动）」试合进 Goal 1 尖端 `52431d6`，记录冲突后 abort。方法：`git stash create` 拿到工作树快照提交 → `git worktree add --detach` 检出 Goal 1 尖端 → `git merge --no-commit` → `git status --porcelain` 取冲突 → `git merge --abort` → 删 worktree。真实工作树全程未动。

前置拓扑事实（unshallow 后实测；shallow 克隆下这些全测不出来）：

- `origin/main` 尖端 = `7b35bde` = 本分支基底。**main 自本分支创建以来零漂移。**
- `merge-base(Goal 1, main)` = `ea6f62f`（Initial commit，只有一个 README.md）。Goal 1 的全部 153 个提交都在这个几乎空白的基底之上——所以两边共有的每个文件对 git 都是 **add/add**（双方新增），没有三方基底可用。
- `cursor/goal1-unblock-a073`（PR #7）拓扑特殊：它在 Goal 1 提交 `80c9011` 之上**已经合并过 main**（合并提交 `b3cb6d4`，成员表已含两个 soul-algo crate——M2 在那条线上已完成）。

## 场景 A：本 PR 合入 main

**零风险。** main 零漂移，无论 merge commit 还是 squash，结果树 = 本分支树。合入后 main 获得 `docs/` 计划权威面 + README，算法 crate 与 `docs/algorithms/` 不动。`cargo test --workspace` 本分支实测 249 通过。

## 场景 B：Goal 1（`52431d6`）随后并入 main（或 main 并入 Goal 1，冲突集对称）

实测冲突 **13 个文件**；其余全部干净（详见下）。

| 文件 | 类型 | 冲突块 | 解法 |
|---|---|---|---|
| `docs/schemas/relationship.schema.json` | AA | 1（`tie_strength` 整块） | **取 main 侧（类型化版）**，但必须先修 RESIDUAL P0-1（补 machine_band/user_band/locked_by_user），否则合入当天 Goal 1 每条重建边验证失败 |
| `docs/schemas/schemas.lock.json` | AA | 1（relationship 哈希行） | 与上一行**同批**：合并后在 Goal 1 侧跑 `cargo run -p xtask -- schema-freeze --write` 复算，`--check` 在 CI 复核。注意 Goal 1 的 `soul-schema` 是 `include_str!` 编译期内嵌 schema 字节——schema 冲突解错一个字节，红的是**编译产物**不是文档 |
| `docs/DECISIONS.md` | AA | 2 | 取 main 侧（D1–D58 超集；D1–D31 前缀两边逐字节同，自动并干净——冲突只在头部增补注与 D32 之后的尾巴）。按 RESIDUAL P1-5 把 D40 的「filed_band」半句补上 |
| `docs/STATUS.md` | AA | 2（合并态文件 891 行——Goal 1 侧 STATUS 极长） | 不要择一。STATUS 自己的纪律第 4 条就是为这一天写的：「PR #2 合入时把两侧合成一份」。计划三态/各线表取 main 侧骨架，Goal 1 的 WP 明细并入；同时消掉 RESIDUAL P1-6 的尖端两说 |
| `docs/SECURITY.md` | AA | 1 | 按 main 侧「实现实证在哪」节的规矩 3 执行：规范面保留，Goal 1 的加密落地/DPAPI 两节随实现进来，不重复规范句 |
| `docs/FORMAL_WORK_PROMPT.md` | AA | ~2 | 取 main 侧（历史段标注、AC-28–33、红线 11/12、双门语义都只在 main 侧）。Goal 1 侧若有本地增补先 diff 确认无独有语义 |
| `docs/PRODUCT_LOCK.md` | AA | 1 | 取 main 侧（「灵魂层算法（v0.1）」节只在 main 侧；产品锁以计划 PR 为权威） |
| `docs/PLAN_VERIFY_PROMPT.md` | AA | 1 | 取 main 侧（历史存档横幅） |
| `README.md` | **UU**（基底 = Initial commit 的 README，两边都改过） | 1 | 取 main 侧（仓库拓扑版），把 Goal 1 侧的构建/运行增量并进「跑一下」节 |
| `Cargo.toml` | AA | 1 | **union 成员表**（Goal 1 的 16 个成员 + `soul-algo-tie`/`soul-algo-trait`）——这正是 M2，unblock 线的 `b3cb6d4` 已给出现成解。注意暗礁：`workspace.package.license` 两边字符串不同（main `LicenseRef-All-Rights-Reserved` vs Goal 1 `LicenseRef-Soul-Proprietary`），得拍一个，别让 union 时随手带过 |
| `Cargo.lock` | AA | — | 不手解，成员表定稿后 `cargo update --workspace` 或直接重新生成 |
| `rust-toolchain.toml` | AA | 1 | 实质相同（1.83）。取 Goal 1 侧（多 rustfmt/clippy components 与 minimal profile），channel 统一写 `1.83.0` 与 main 对齐 |
| `.gitignore` | AA | 1 | union（Goal 1 侧是超集，含 target/node_modules/db 等） |

**干净合并（无需人手）**：`docs/schemas/` 其余 10 份（与 Goal 1 逐字节同一 → add/add 相同内容自动解决）、`docs/GOAL2_POLISH_PROMPT.md`（字节同一）、`docs/scan-rounds/`、`docs/templates/`、`docs/PLAN_INDEX.md`（只在 main 侧，净新增）、`docs/algorithms/`（只在 main 侧）、`crates/soul-algo-*`（只在 main 侧）、Goal 1 的 `apps/`、16 个 crate、`fixtures/`、`.github/`、pnpm 三件套（只在 Goal 1 侧）、`docs/GOAL1_PLAN.md`（只在 Goal 1 侧）。

这验证了 Round 1「schema 正文逐字节采纳 Goal 1 版」策略的价值：**add/add 在字节同一时零成本**。本 PR 与 Goal 1 每一处有意的字节分歧（现在只有 relationship + lock 两处）都精确对应一个计划内的冲突，冲突解法已预先写在文档里（D58/STATUS 纪律/SECURITY 规矩）。

## 场景 C：实际更可能的路径——经 unblock 线回主干

unblock 线已含 main@`7b35bde` 的历史，所以它将来并入「main + 本 PR」时基底是 `7b35bde` 而非 Initial commit：

- `Cargo.toml`、`Cargo.lock`、`rust-toolchain.toml`、`.gitignore`：main 侧自基底未动 → 三方合并自动取 unblock 侧已解好的版本。**工具链类冲突整组消失。**
- `docs/` 计划文件与 README：基底上不存在/是旧版，冲突集与场景 B 相同（unblock 继承的是 Goal 1 的 docs 版本）。
- 结论:**无论走哪条路径，真正要人手解的都是同一批 docs 文件**，且解法同表。经 unblock 回主干可少解 4 个工具链文件，还白拿一个已完成的 M2。

## 场景之外的三个时间性风险

1. **PR #6（BLOCKERS）在本 PR 之后合**（STATUS 已定此顺序）：BLOCKERS 是 main 上的净新增文件，不冲突；但它正文里的「D32」指 dev-sota 的 e0 禁令，与 docs/DECISIONS.md 的 D32（machine_band）撞号。合并检查单加 RESIDUAL P1-4 的改写动作，否则单源索引里出现两个 D32 语义。
2. **Goal 1 尖端在持续移动**：本轮期间实测从 `6c91d39` 推进到 `52431d6`（新增 IPC 面测试与 hosted AC-26 注记）。STATUS/SECURITY 里的「核于」提交号在合并当天必须统一刷新一次（RESIDUAL P1-6），否则合并者对着三个不同提交号做决定。
3. **schema 收紧与 P0-1 的竞态**：若本 PR 带着未修的类型化 schema 先合 main，而 unblock 线随后带着 machine_band 序列化并 main——两个各自全绿的 PR 合流后 CI 必红，且两边都会觉得是对方的错。**P0-1 必须在本 PR 内修掉**，这是本文件唯一的行动级结论。

## 合并检查单（给执行合并的那个代理）

1. 合并前:确认 RESIDUAL P0-1 已修、lock 已随之重算（`sha256sum docs/schemas/relationship.schema.json` 与 lock 行一致）。
2. 场景 B/C 逐表解冲突；docs 类一律 main（计划）侧为骨架，Goal 1 侧只并入实现实证与 WP 明细。
3. 解完 schema 后在 Goal 1 工作区跑 `cargo run -p xtask -- schema-freeze --check` + `cargo test --workspace`。
4. 同一提交刷新 STATUS（三态、尖端提交号、阻塞表 schema 行）。
5. PR #6 合入时执行 D32 改写。
