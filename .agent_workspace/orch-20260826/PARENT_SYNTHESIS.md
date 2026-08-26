# 父代理终局综合：项目变更与不完善项

调度器模型：cursor-grok-4.6-medium（本父代理）
云端子代理：`claude-fable-5-thinking-xhigh`（environment=cloud，bc-58d7b669-c00e-548a-ba01-89a43414a737）
三轮本地并发：每轮 2×fable（claude-fable-5-thinking-xhigh）+ 2×opus-fast（claude-opus-5-thinking-high-fast）+ 2×gpt-sol（gpt-5.6-sol-xhigh-fast），共 18 路。
核于：2026-08-26；`origin/main` = `a0ec14b`。权威仍以 `docs/` 为准；本文与 `.agent_workspace/**` 均为过程稿。

---

## 1. 项目已经变了什么

相对初始提交 `ea6f62f`，`main` 上实际合入了两段冻结：

| 阶段 | 提交 / PR | 内容 |
|---|---|---|
| 算法冻结 | `7b35bde`（PR #5） | `crates/soul-algo-tie`（T4D）、`crates/soul-algo-trait`（A0/A1/A2/A3）；`docs/algorithms/` |
| 计划冻结 | `095c1f8`（PR #8）+ `a0ec14b` | 产品锁、D1–D60、FORMAL 验收矩阵、SECURITY、typed `tie_strength` schema、STATUS |

三轮复跑 `cargo test --workspace`：**249/249 绿**。`main` 仍**没有**桌面壳、主库、CI、安装包——这一点 README 写对了。

`docs/STATUS.md`（核于 2026-08-25）之后，仓库真实又长出了 STATUS **未登记**的一层：

- Goal 1 主干 `cursor/soul-goal1-7b1c` 前进到约 `6d1058b`，PR #2 **OPEN 且 CONFLICTING**。
- 吸收线 `cursor/goal1-unblock-a073`（PR #7）已把两个算法 crate 加进 workspace，并修了主干上仍开的 G1/G2/G3。
- 测试候选 / 集成栈：`first-test-candidate-c441`、`soul-integration-4a8e`（PR #15，无 D 号改了 PRODUCT_LOCK 的 Telegram 口径）、`beadflow-integration-c441`（同仓兄弟产品 BeadFlow，约 20 个仍开放的 bead PR）。
- 当前约 **34 个 OPEN PR**（BeadFlow 20 / 非 BeadFlow 14，含本审计 PR #57）。

---

## 2. 四门验收（父代理采信 Round 3 fable-a，并吸收 fable-b / opus-b 修正）

| 门 | 判定 | 范围 |
|---|---|---|
| 计划冻结 `PLAN_FROZEN` | **PASS**（附义务） | 仅 `main` 上的 `docs/`。STATUS 时点已过期；BLOCKERS 仍在 PR #6；支线 docs 漂移不管 |
| 算法冻结 `ALGO_FROZEN` | **PASS**（附义务） | 判档规则、常量值、as_of、墓碑。**不管**话术单源、跨 crate 180 物理单源、缺省墙钟护栏 |
| 应用可装 | **FAIL** | `main` 无应用；Goal 1 hosted CI 因私库账单空跑；作者 Win11 清单 0 勾 |
| Goal 1 关闭（D54） | **FAIL** | 矩阵 + 十三片未同时在同一棵已对齐的树上被证明 |

---

## 3. 还不够完善的部分

### P0 — 阻塞合入或关闭

1. **实现主干分裂。** D49 指定 `cursor/soul-goal1-7b1c` 为唯一主干。今天至少还有 unblock / close-loop / soul-integration 自封 exclusive。R3 opus-b：integration 已吸收主干独有代码提交（18/18），但与吸收线 **22 文件冲突**；两条互不相容的 **D61**（主干更替 vs 空图/姓名残差）都未进 `main`（`main` 止于 D60）。没有 owner 做拓扑归一。
2. **Goal 1 主干四缺口仍开：** 图谱 T0 本地阈值、intake 不查轴锁、rebuild 边不可纠正、`Cargo.toml` 无 `soul-algo-*`。修复在 PR #7 线，**未回主干**。今天把 PR #7 合进 `main` 会让 main 比主干**退步**（缺后续诚实文案/IPC 级补证）。
3. **PR #2 与 `main` 冲突**（文档/schema 锁面）。
4. **PR #4**（`agent/dev-sota`）按 D49 应关闭，仍 OPEN 且 CONFLICTING、检查失败。
5. **PR #6** `BLOCKERS.md` 仍未进 `main`（本身 MERGEABLE）；合入时必须用括注处理 BLOCKERS 文中的「D32」撞号，**不能**占用 D61（已被两条支线以不同含义占用）。跳号至少 D64+。
6. **PR #15** 无决策号改产品锁 Telegram 导入语义——这是真违锁，不是 BeadFlow。
7. **hosted CI 账单**与 **作者 76/69/77 项手动清单全 0 勾**——关闭门的外部半边为零证据。
8. **STATUS 失明：** 自称 `main@7b35bde` 与 polish 分支视角，实际 `main@a0ec14b`；对 BeadFlow / 集成栈 / PR #9–#56 零登记。

### P1 — 不推翻两项冻结，但已拍板未完成或会静默坏

1. **话术不单源。** COPY_ZH 收尾句/透明句/`{截止日期}` 在 crate 零命中；A2 把 Moderate 渲染成「中」而非冻结词「中等」，踩 PRODUCT_LOCK「只有强/中等/弱」。可在 main 改字面，但全局模板归属要先走 COPY_ZH 再改代码（DECISION §6.4）。
2. **D52 跨 crate 180 未在 `main` 钉死。** 两 crate 各写一个 180；R2 反证：只改 A2 侧阈值，249 测试仍绿，且可让「半年句」与强档同屏。soul-integration 上 reportedly 已有 `day_constants_agree.rs`——`main` 没有。
3. **D32 棘轮未进 schema。** `locked ⟺ user_band.is_some()` 的禁止组合可通过校验；半锁行在 Goal 1 会被读成未锁，用户纠正静默消失。`relationship.schema.json` 在 main 与 Goal 1 字节相同，这不是「等合入再算」的纸面问题。
4. **墙钟：** 可执行代码当前 0 命中，但缺省 `score_now` 入口可以在全绿下加进去；护栏必须是「剥注释后的源码扫描」，不能裸 `contains("SystemTime")`（会误伤注释）。
5. **`last_contact_evidence_id`：** 渲染器有字段，T4D 生产者不算，schema 也存不下完整 TieScore。
6. **扫描文死链：** `docs/scan-rounds/R{1,2,3}-SYNTHESIS.md` 把 Cursor run id 写成相对路径（6 条）。

### P2 — 卫生

- `main` 跟踪 `.agent_workspace/**` 约 500+ 文件，含 **191** 个 `target/` 构建指纹、**12** 个空文件。清理时须保留 `DECISION.md` 引用的 `round3/fable-a/REPORT.md`。
- 过期开放 PR #1/#3；README/FORMAL 对 Goal 1 覆盖面的无日期复述。

### 明确不算缺陷的

- A1 在 v0.1 永不升档：预期行为。
- 群聊不判档、F04c 一来一回可回强：已定价的已知代价，禁止静默加第三道门。
- BeadFlow **不**违反 PRODUCT_LOCK「禁止第二份 PRODUCT.md」的字面（仓库里没有 `PRODUCT.md`）。问题是**仓库级产品登记缺位**与和 Soul 集成分支的路径撞车。
- `PLAN_FROZEN` ≠ 应用已发布 ≠ Goal 1 可关。

---

## 4. 建议下一步（不启动 Goal 2）

最小止血（不预设哪条是唯一主干）：

1. 按 D49 **关闭 PR #4**。
2. **合 PR #6**（括注撞号，勿占用 D61–D63）。
3. 刷新 `docs/STATUS.md` 尖端与分支拓扑（含 BeadFlow 存在事实，不把它写成 Soul 产品锁）。
4. 作者裁：唯一实现主干现在是 goal1、unblock，还是 integration；**在此之前不要把 PR #7 合进 main**。
5. `main` 上可做的低风险工事（另开 PR）：D52 等值测试、A2「中」→「中等」、墙钟扫描（剥注释）、scan-rounds 死链、停止跟踪 `target/`。

Goal 2 在 Goal 1 关闭前不要打开。
