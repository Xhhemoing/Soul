# Round 3 fable-a — SOTA 冻结核验报告（计划权威面）

MODEL_SLUG: claude-fable-5-thinking-xhigh（按派单，无静默降级）

核于 2026-08-25。BINDING：`R2-SYNTHESIS.md`、`R1-SYNTHESIS.md`、`SHARED_BRIEF.md` 全读。
本槽位职责：对 Soul **计划**（不是 Goal 1 应用）出 SOTA 冻结判定。只读核验 + 独立探针；未改任何 `docs/` 文件（唯一发现的可改项都已被并行槽位处理或按「宁报告不改」留给父代理）。

## 0. 被核验的树到底是哪一棵（先钉这个，因为它在我核验期间动过）

- HEAD：`27b4060`（父代理在本轮中途合入了 opus-b 的 report-only 产物；`c3d5960` 是 Round 2 尖端）。
- 工作区另有 **opus-a 的九处原子措辞修复尚未提交**（`docs/FORMAL_WORK_PROMPT.md`、`docs/PLAN_INDEX.md`、`docs/STATUS.md`，+17/−17 行）。我先读到了改前版本，中途文件变更，已重读 diff 并把**改后**字节纳入核验面。
- 本判定覆盖的确切字节（sha256）：

| 文件 | sha256 |
|---|---|
| `docs/PRODUCT_LOCK.md` | `50bbe63a62fd185857731c10f4eef04b6de14bff3facf4c6d8e9e4ddfbec5ad1` |
| `docs/DECISIONS.md` | `afa83f0a79356c395384e94ea3760fdf0e01e4aa88bf66547ef477371a5c2fd1` |
| `docs/FORMAL_WORK_PROMPT.md`（含未提交修复） | `4a8e403938530ca06163916f1943abf3f0918fecb1ec1139ffbb52a316dd81d1` |
| `docs/STATUS.md`（含未提交修复） | `7575de71863fe7a2d33ddd0fb9a4d62c6eae12f5ed2cb608cb407ef5995ff54d` |
| `docs/PLAN_INDEX.md`（含未提交修复） | `ca2ade44399b53f56c06011da9de028198ee57b187ba5902739eca2c1cc4b280` |
| `docs/SECURITY.md` | `7ba94bebd56b258efafa53af212825cf3954f5221afd1a71d7bb9479cf61d6cd` |
| `README.md` | `a20f0669d3d36330a2be444b06129cabaa4824300369793b494351bdace32f34` |
| `docs/schemas/relationship.schema.json` | `ebf049a990366ab74bc837c7b24957d8d8ecda49dac44d26053db00b32ee4be7` |

**给父代理的硬要求**：合 PR 前必须把这三个文件的未提交修复原样提交。若提交前又有改动，本判定对改动部分失效，须复核。

## 1. 独立探针（不采信并行槽位日志，全部自跑）

| # | 探针 | 结果 |
|---|---|---|
| 1 | 常量泄漏：`180/360` 扫 PRODUCT_LOCK / DECISIONS / FORMAL / STATUS / PLAN_INDEX / SECURITY / README / schemas | **零命中**（唯一疑似是 `schemas.lock.json` 里某条 sha256 的十六进制子串 `a180`，非泄漏）。D52 已用常量名 `DEMOTE_ONE_BAND_DAYS`。FORMAL 里的 30/10、12/6/3 等均为夹具身份数，红线 11 自查段已声明 |
| 2 | 禁止项负向扫描：OAuth / E0 / 文件写 | 全部命中均为否定式、砍项或 v0.2/v0.1.1 去向声明，无一句写反。第二份 `PRODUCT.md` 不存在 |
| 3 | `schemas.lock.json` | **11/11 sha256 与磁盘字节一致**，本 PR 未动 schema 而不重算 lock 的情况不存在 |
| 4 | `relationship.tie_strength` 行为探针（本目录 `schema_probe.py`，jsonschema 4.26 实跑） | **12/12 符合预期**：空对象过（Goal 1 现状宽松度保留）、`score` 拒（D22 执行点）、半迁移拒（if/then 整包）、T4D+machine_band 过（D32/D59）、锁定边三字段过（D48）、`user_band` 无 `locked_by_user` 拒、分列计数无 `algorithm_id` 拒、`T5` 拒（第三套规则进不来）、`band:"none"` 拒、`last_direct_contact_utc:null` 过、`tie_strength` 可整体缺席 |
| 5 | AC / D 编号完整性 | 矩阵行 AC-01–26、AC-28–34 齐全各一行；AC-27 只作 v0.1.1 注记不占行（两处声明）。D1–D59 连续无缺号；BLOCKERS「D32」撞号检查单在 DECISIONS 脚注 |
| 6 | FORMAL 声称的夹具/常量真实存在 | `group_heavy_plus_one_direct_each_way`、`lilei_12`、`group_heavy_plus_three_directs`、`dormant_2019` 在 `crates/soul-algo-tie`；`a0_lock.rs`、`examples/matrix.rs` 存在；五个常量名与 `crates/soul-algo-tie` 常量模块对得上 |
| 7 | `cargo test --workspace --offline` | **249 passed / 0 failed**（Rust 1.83） |
| 8 | 双源哈希 | `.agent_workspace/context/plan/` 首行即「历史快照，不是权威」，与 `docs/` 哈希分叉属预期（R2 L4 拍板保留快照） |
| 9 | 本 PR 对冻结面的扰动 | `git diff 7b35bde..HEAD -- docs/algorithms/` 仅 2 行，即 R1 明报的夹具叙述校准（100/50 → 代码夹具 30/10；`group_heavy_plus_three_directs` 名字与常量名指代），**冻结结论未动** |
| 10 | 跨分支「核于」一致性（D57） | 同树对 `cursor/soul-goal1-7b1c` 的提交号：STATUS 与 SECURITY 均为 `df5d2dd`，一致。STATUS M2 新引入 `cursor/goal1-unblock-a073 @ c81c233`（opus-a 已 `git show` 核实该提交成员表含两 crate） |

## 2. 对并行槽位发现的复核

- **opus-a 九处修复**：逐条对过 diff，全部是收窄/纠错，无一处改判或扩权。其中两处实质性：①矩阵前言与夹具段把 AC-34 从「crate 已有夹具」的假话里摘出来（R2 边界风险 3 点名的事，改后 Goal 1 实现者不会去 crate 里找不存在的群聊导出样本）；②AC-33 括注封死「适配器援引本行少填分列」的空子，与 schema `if(algorithm_id)` 对齐——这原本是我方交叉核验单上的唯一候选矛盾，已被该修复关闭。
- **opus-b F1**（COPY_ZH P4「超过 180 天」是被 DECISION.md 明文作废的 `>` 写法）：**属实，但不阻塞本判定**。三个理由：它在 `docs/algorithms/COPY_ZH.md`，该文件本 PR 一字未动、且已随 PR #5 在 `main` 上——这是 main 侧既有措辞债，不是本 PR 引入或应由本 PR 冻结的面；方向安全（只会少说一句沉寂句，不会在未降档时说出）；补丁与决议措辞 opus-b 已备好，只差父代理分配 D 号。
- **opus-b F4**（远期时间戳污染全库 as_of）：属实的未命名风险，处置是**导入层新拍板**，且绝不能用「as_of 夹墙钟上界」糊——那违反 D45。单独立项，不进本 PR。
- **gpt-sol-b 覆盖度**：11 切片全覆盖 + 2 切片部分覆盖（切片 3 缺「编辑偏好/边界并验证」的显式行、切片 12 的 AC-23 未逐项枚举十类审计动作）。均为**粒度**缺口而非孤儿映射，且 D54 双门语义下切片自身仍是关闭门禁，矩阵行不足不构成放水通道。不建议本轮加行（D30 只减不增的姿态延伸到矩阵纪律：加行是父代理裁量，不是冻结前置）。

## 3. 残留清单（全部非 P0，逐条给出为什么不阻塞）

| # | 残留 | 级 | 为什么不阻塞 / 去向 |
|---|---|---|---|
| R-1 | COPY_ZH P4 `>` 写法（opus-b F1） | P1（main 侧） | 本 PR 未触碰该文件；方向安全；补丁待父代理 D 号，建议本 PR 后立即单独落 |
| R-2 | 远期时间戳污染 as_of（opus-b F4） | P2 | 导入层新拍板，单独立项；禁止墙钟夹逼方案 |
| R-3 | schema 只单向执行 `user_band → locked_by_user`，`locked_by_user:true` 而无 `user_band` 仍合法 | P2 | D32 双条件（locked ⟺ user_band.is_some()）的另一半在 Goal 1 测试执行；schema 层再收紧须与 `schemas.lock.json` 同批重算（D58 尾句），属 Goal 1 合并后事项 |
| R-4 | `machine_band` 不在 if/then 必填清单 | P3 | D32 措辞是「常为 Some」不是「恒为」；D59 明写收紧不得打红 Goal 1 重建边，此宽松是拍过的板 |
| R-5 | `relationship.types` 仍是裸 array | P3 | PR #1 原状，不在 D58 义务清单内；收紧同样须同批重算 lock |
| R-6 | 「≥8 字」n-gram 出现在 PRODUCT_LOCK / SECURITY / AC-12 三处 | P3 | 三处取值一致；它是泄漏测试参数，不是判档阈值，不在红线 11 辖区。纯单源洁癖项，改动收益低于合并冲突面 |
| R-7 | 切片 3 / 12 覆盖粒度（gpt-sol-b） | P3 | D54 双门下切片自身仍守门；加行属父代理裁量 |
| R-8 | `DECISION.md` §6 沿用旧 crate 名 | 已缓解 | FORMAL 第 4 条括注 + D53 已把「实现跟真名」摆到读者眼前 |

## 4. 结论

四份交付各就位：本文件、`SOTA_ACCEPT.md`（P1–P6 全 PASS）、`VERDICT.md`（**PLAN_DOCS_FROZEN_FOR_MAIN**）、`CROSS_CHECK.md`（13 切片 × AC-01–34 × D1–D59，零现存矛盾）。

按 R2 留给本轮的边界重申：本判定冻结的是**计划权威面可以合 `main`**，不等于 Goal 1 关闭（D54 双门都没过）、不等于 `main` 有应用。合入顺序仍按 STATUS：本 PR → PR #6（处理 D32 撞号）→ Goal 1 线剩余项。不重开产品方向、无 F04c 第三道门、不启动 Goal 2——本轮核验确认这三条禁令在全部权威文件里无一处被写反。
