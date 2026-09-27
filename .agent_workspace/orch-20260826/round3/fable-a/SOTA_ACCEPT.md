MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 3 / fable-a — 终局 SOTA 验收判定书

## 0. 验收基准（本轮实锚，非转抄）

- 验收树：`cursor/project-status-audit-c49c` @ `78febb0`；本轮实测 `git diff origin/main HEAD -- docs crates` 为**空**——`docs/` 与 `crates/` 与 `origin/main` @ `a0ec14b` 逐字节相同。判定对象即 `main`。
- 本轮复跑 `cargo test --workspace`：**249 / 249 通过，0 失败，0 忽略**（与 R1 gpt-sol-a、R2 gpt-sol-a 两次独立结果一致，三次核验闭合）。
- 本轮复核关键 PR（gh 实查 2026-08-26）：#2 OPEN CONFLICTING/DIRTY、#4 OPEN CONFLICTING/DIRTY、**#6 OPEN MERGEABLE/CLEAN**、**#7 OPEN MERGEABLE/CLEAN**、#15 OPEN MERGEABLE/CLEAN——与 round2 全部报告记录一致，无新漂移。
- 输入：round1 六份 + round2 六份报告全文。round2 对 round1 的每一处修正（R2 opus-a 对墙钟措辞、R2 opus-b 对 schema 校验器定位与 D32 真值表、R2 fable-a 对 PRODUCT_LOCK 定性与三层栈分层、R2 gpt-sol-b 对 PR 计数）均已采信为终局口径；分歧裁断记录见同目录 `REPORT.md`。

---

## 1. 四门判定总表

| # | 门 | 判定 | 管辖范围（判定只在此范围内有效） |
|---|---|---|---|
| 1 | 计划冻结（`PLAN_FROZEN` / `PLAN_DOCS_FROZEN_FOR_MAIN`） | **PASS**（附三条义务） | `main` @ `a0ec14b` 的 `docs/` 权威面 + `README.md`。**不及于**任何支线的 docs 副本、不及于尚在 PR #6 的 BLOCKERS、不及于 STATUS 的时点性登记职能 |
| 2 | 算法冻结（`ALGO_FROZEN`） | **PASS**（附三条义务） | `main` 的 `crates/soul-algo-tie` v0.3.0 + `crates/soul-algo-trait` v0.3.0 的**判档规则、常量值、as_of 纪律、墓碑** + `docs/algorithms/DECISION.md`。**不及于**渲染话术单源、跨 crate 常量物理单源、缺省墙钟入口护栏（三者均为 DECISION §0 明文的冻结后合并义务） |
| 3 | 应用可装 | **FAIL**（不可证明） | 判定对象不在 `main`（main 无应用代码、无 `.github`）。应用实体在 goal1 主干 `6d1058b` 与 D61 主干（unblock `6133307` ∪ close-loop `d4490b0`）；安装证据链归属 hosted CI（作者账单）+ NSIS 真机 + Win11 清单（作者，D56） |
| 4 | Goal 1 可关闭（D54 门） | **FAIL** | 判定权在 `main` 权威面（FORMAL 矩阵 + PRODUCT_LOCK 十三片 + D54；其权威出处 BLOCKERS 尚不在树）；执行体在 goal1/D61 主干 + 作者 |

### 1.1 计划冻结 — PASS

**成立依据**：冻结标记已随 PR #8 合入（MERGED 2026-08-25T03:29）且 `docs/algorithms/DECISION.md` 在树；PLAN_INDEX 的全部显式权威链接可解析（R1 fable-a 机械校验）；判档阈值无违规双源，数字只在 DECISION §3 常量表，COPY_ZH 第 73 行的 `=180` 有 D60 明文授权（R1 fable-a §2.3）；schema 12 文件与 `schemas.lock.json` 11/11 SHA-256 吻合（R1 opus-b 手算 + R2 opus-b 独立重算双向覆盖，两次闭合）；`docs/BLOCKERS.md`、`docs/PRODUCT.md` 均确认不在树，与 STATUS 自述一致。

**冻结不及于（即 PASS 不背书）以下三件，转为附带义务**：
1. STATUS 的登记职能已失明（终表 P0-4）——冻结的是计划判断与工序框架，不是时点登记；但「单一事实来源」这句自称今天名不副实。
2. 支线对冻结面的漂移：PR #15 无 D 号改 `PRODUCT_LOCK.md`（终表 P0-5）、D61–D63 支线发号（终表 P1-6）。冻结在 main 树内成立，其强制力不达支线——这是治理缺口不是冻结缺口，但验收必须写明边界。
3. BLOCKERS（`BLOCKERS_FROZEN`）尚未上树（终表 P0-3）：合入/关闭阻碍的权威清单目前只能跨分支引用。

### 1.2 算法冻结 — PASS

**成立依据（判档层零缺口，四路独立核验闭合）**：
- band 算术与 DECISION 逐条一致，**零判档偏差**（R1 opus-a：T4D 三道门全在一对一计数、180/360 闭区间、决胜夹具与 §1 裁决表逐格吻合、消融矩阵实跑对表）。
- 四项已知代价全部钉死且多为性质级/穷举级：F04c 复燃一响（`Truth::Exactly` 钉死）、群聊不判档（漏洞实证共存 + 4000 例生成式性质）、as_of 纪律（per-peer 陷阱对照）、A1 空转（346,200 条日志穷举 + 编译期断言）（R1 fable-b §3、R1 opus-b §2.2）。
- A0/A1/A2/A3 + denylist 全部实现且被钉住；intake 不绕锁补丁与 replay 全等有交叉测试（R1 opus-b）。
- 工程纪律：零依赖、`forbid(unsafe_code)` 双设防、无墙钟代码（R1/R2 gpt-sol-a 两轮扫描 0 命中可执行代码）、clippy/fmt 干净、249 测试三轮全绿。
- R2 opus-a 终局裁断：全部已知渲染面问题**不动 band**，且被 DECISION §0「冻结后的合并义务」与 D52「不合，后置」预先定价——不阻塞冻结。

**冻结不及于（PASS 不背书）以下三件，全部列入终表 P1**：
1. **话术单源不成立**（R2 opus-a §4.2 三证）：档位词三套（「中」直接违反 PRODUCT_LOCK:78）；COPY_ZH §0.5/0.6/0.7 三条全局强制项在 `crates/` 0 命中，六类句仅 P1b 逐字一致，D40 承诺的「另记」为空——存在两份都自称冻结的话术权威互相矛盾；COPY_ZH:73「不变式免费成立」的前提（同一个常量）在实现里是假的。
2. **常量物理不单源且漂移不可检测**：R2 opus-a 反证实验——trait 侧 180→150 单侧改后**249 项全绿**，并当场复现 PRODUCT_LOCK 红线三之三禁止的「沉寂句与强档同屏」。D52 拍板的钉相等测试三棵树皆无（R2 gpt-sol-a、R2 fable-b 复核 close-loop 亦无）。
3. **缺省墙钟入口无护栏**：`score_now` 形态可在 135/135 全绿 + clippy 干净下加入公开 API（R2 opus-a §3.4）——DECISION §6.3 明文禁止的形态，唯一可行护栏是源码扫描，尚不存在。

### 1.3 应用可装 — FAIL（不可证明）

「可装」的证据链需要三段缝同时闭合，今天三段都开着：

| 缝 | 现状 | 归属 |
|---|---|---|
| 本机绿 → hosted 绿 | 唯一一次五门全绿是历史 run `32754617268` @ `2e72ddf`（08-24，含 windows/package/install-smoke 15 项）；自 goal1 HEAD `478f19f` 起 hosted 全部空 runner（私库 Billing & plans/spending limit，08-26 00:37 最新 run 仍 0 step；R2 fable-b 再核）；D61 主干（unblock/close-loop）**从未有过任何 hosted run**（账单 + 两线 ci.yml 触发列表不含自己） | 作者（账单）+ 代理（触发配置已在 `d4490b0` 修） |
| hosted 绿 → 工件可装 | 那次绿 run 的 package 工件**早于 NSIS 目录修复**，分支自我标注不能用于卸载/keys.dpapi 验证；NSIS 真机安装是作者签名件（D56） | 作者 |
| 工件可装 → 真机可用 | 作者 Win11 手动清单 **76 项零勾选**（`scripts/author-manual-checklist.md` 实数）；托盘外观、UAC 观感、WebView2 抓包全未做 | 作者 |
| （前置）装的是哪棵树 | 拓扑未归一：goal1 主干、unblock、close-loop、soul-integration 四份载体并存，「候选安装树」本身无裁决 | 裁决 + 代理 |

**管辖说明**：此门 FAIL 不构成对 main 的批评——main 的自我描述（无应用代码）与实况一致，这是分工。FAIL 记在 Goal 1 线与作者侧。

### 1.4 Goal 1 可关闭（D54）— FAIL

D54 门 = 验收矩阵全部 v0.1 行通过 ∧ PRODUCT_LOCK 十三片同时成立（T0 图谱即使 AC-01–26 全绿也不能关）。终局结论采 R2 fable-b：**今天在任何一棵树上都不可关**，且差额已收敛为可枚举四件：

1. **hosted CI 账单**（作者）——AC-26 的 hosted 半边在一切当前尖端上零证据。
2. **作者 Win11 清单 76 项**（作者）——AC-01 与十三片「干净 Win11 上证明」总前提零项。
3. **拓扑归一**（代理，无 owner）——D61 主干缺 goal1 主干侧 17 个代码提交（六连诚实文案修复直接关系切片 1/6/8/10/11 的「UI 如实写」义务 + 第三证明轮把十几条 AC 提到 IPC 缝）；按 PR #10→#7→main 今天合入会**合入即退步**。体量已量化：1 摘 16 待摘 + 恰 5 文件冲突的一次显式合并（merge-tree 实测）。
4. **权威面对齐**（代理）——D61 落 main 的 DECISIONS、两份 STATUS 旧拓扑句改写、BLOCKERS（PR #6）上 main。

**正面认定**：代码功能面不再是阻塞——四个关闭级缺口（G1/G2/G3/M2）+ G1+/M3/GC 命令面/具名夹具对拍在 D61 主干上已齐，R2 fable-b 在 `git archive` 提取树上实跑 211 项相关测试全绿。FAIL 的成分是两项作者动作 + 两项无 owner 的代理工序，不是缺代码。

---

## 2. 不完善项终表

分级定义：**P0** = 阻塞合入（任何权威合并动作）或阻塞 Goal 1 关闭；**P1** = 不阻塞两项冻结，但属已拍板未执行义务、静默失效风险或裁决前置，必须跟踪；**P2** = 卫生。

「修复归属」四值：`main`（main 树内可修，不动 Goal 1 树、不需作者线下动作）、`Goal1`（须在 goal1/unblock/close-loop/D61 主干侧做）、`作者`（只有作者能做：账单、真机、签名、gh 写权限最终归属）、`裁决`（父代理提名、作者拍板后才有修复动作）。

### P0 — 阻塞合入或关闭（8 项）

| ID | 项 | 要点与证据 | 修复归属 |
|---|---|---|---|
| P0-1 | **拓扑归一无 owner** | D61 主干缺 goal1 主干 17 个代码提交；今天按 PR #10→#7→main 合入即退步；1 摘 16 待摘 + 5 文件冲突已量化（R2 fable-b B3/3.1）。这是当前唯一能推进 D54 的代码工序 | Goal1 |
| P0-2 | **权威面三方互斥** | main STATUS（主干=7b1c、#7 待回主干/D50）vs 主干 STATUS（不要合 #7/#10）vs D61（主干更替，仅在 PR #10 链 `a29f792`）；被降级主干在 D61 落笔后仍推进至 17:59（live drift）（R2 fable-b 3.2） | main（D61 收编登记 + main STATUS 改写）+ Goal1（主干 STATUS 改写） |
| P0-3 | **BLOCKERS（PR #6）未上 main** | D54 行的权威出处不在关闭候选树；MERGEABLE/CLEAN 本轮再核。合入时三件套：D32 撞号用**括注方案**（D61 已被支线预定，禁用「下一个空闲 ID」）、§5 工序加核于限定（其世界模型落后现实一代，机械照做步 2 会正面撞 goal1 禁令）、DECISIONS 登记 D61–D63 号段（R2 fable-a 议题3 选项 3A） | main（改动落 main；合并动作需 gh 写权限 = 作者/父代理） |
| P0-4 | **main 权威面失明** | STATUS 自称单一事实来源，但对 PR #9–#56（48 个，R2 gpt-sol-b 逐项核实）、三层集成栈、BeadFlow（41 PR / 763 测试 / 19 条 BD 拍板）、billing 阻塞、PR #2 有意冲突策略零登记；只读 main 的编排者会执行与集成线正面冲突的动作（R1 fable-a P0-1） | main |
| P0-5 | **PR #15 无 D 号改 PRODUCT_LOCK** | Telegram 导入口径语义翻转（main 承诺接收的单聊 Export chat history 被支线宣布拒收），+2/-2 实锚（R2 gpt-sol-b 全量分页核实）；违反 PLAN_INDEX §5「先改锁并追 D 号」。全案唯一「改锁不留痕」 | 裁决（追认补 D 号或回滚；改动实体在集成线） |
| P0-6 | **BeadFlow 治理真空** | 仓库级产品登记无任何权威文件在管；`docs/agent-decisions.md` 两线同路径撞车（先合者赢）；PR #16 挂 first-test-candidate，若该栈被裁为后继主干则第二产品搭车进入 Soul 主干——与主干裁决**耦合**，须先于或同时裁（R1 fable-a P0-2 + R2 fable-a 议题1 修正定性：非 PRODUCT_LOCK 违反，是登记缺位） | 裁决（1A 登记共存 / 1B 迁出 / 1C 冻结 #16 止血）+ main（登记落点） |
| P0-7 | **hosted CI 账单** | 私库 Billing & plans/spending limit；近 100 run 仅 1 次成功；AC-26 hosted 半边在一切当前尖端零证据 | 作者 |
| P0-8 | **作者 Win11 清单 76 项** | AC-01 与十三片总前提零项真机证据；AC-26 NSIS 半边为作者签名件（D56） | 作者 |

### P1 — 不阻塞冻结，必须跟踪（11 项）

| ID | 项 | 要点与证据 | 修复归属 |
|---|---|---|---|
| P1-1 | **D52 钉相等测试缺失**（已拍板未执行） | 两处互不知情的 180；单侧漂移 249 全绿不可检测且可静默破 PRODUCT_LOCK 红线三之三（R2 opus-a §1.4 反证实验；危险方向单向：A2 阈值 < 降档阈值即破锁）。修法：tie 加 `[dev-dependencies] soul-algo-trait` + 一条 `assert_eq!` | main |
| P1-2 | **话术单源不成立** | 三证见 §1.2。分两半：①「中」→「中等」一处三线违反（COPY_ZH §0.1/§5.5 + PRODUCT_LOCK:78），纯字面、无需裁决、需留痕，四份报告一致建议先修；②六类句归属 + §0.5/0.6/0.7 落地 + 两条规范外增补句（一对一日期句、「按现在的规则应是 X」兜底句）须走 DECISION §6.4 程序（先改 COPY_ZH 再改代码）。落地时注意接口坑：COPY_ZH 点名的 `assert_non_clinical` 实为 `assert_publishable_about_peer`（R2 opus-a §2.4） | ① main（留痕）；② 裁决 |
| P1-3 | **墙钟源码级护栏缺失** | 缺省墙钟入口可静默进入公开 API（R2 opus-a P2 探针）；行为测试原理上抓不到该形态；修法一行（`ablation.rs` banned 数组加 `SystemTime`/`Instant`/`std::time`） | main |
| P1-4 | **D32/D48 schema 层缺口**（R2 opus-b 升「高」） | 3×3 真值表 4 格分歧（D06/D07/D08 放行 + D02 反向误拒与自身 `$comment` 冲突）；E01（band 与 user_band 互斥宣称）/E02/E03 放行；`is_locked_by_user()` 合取式把半锁态静默读成未锁——**静默丢用户纠正**，而 `put_relationship` 今天就在用这份 schema 把关（两侧字节相同）；relationship fixture 对 D59 棘轮与 D32 三字段零覆盖。不动字节可立即做：不变式测试 + 合取式裁决 + fixture 补齐；schema 收紧须与 Goal 1 同批（D58） | Goal1 |
| P1-5 | **D49 悬置** | PR #4 未关（三方文件一致要求，本轮再核仍 OPEN/CONFLICTING/三检查失败）；「后继主干宣布程序」缺失（宣布主体/落点/生效前置全空）——三层栈自我授权（soul-integration 自封 exclusive）得以发生且无人能追认或否决的制度根源（R2 fable-a 议题2/治理缺口1） | 作者（关 #4 的 gh 写操作）+ main（程序条文）+ 裁决（2A/2B/2C 选树） |
| P1-6 | **D 号发号权不闭合** | D61–D63 支线预定、BLOCKERS 旧 D32 撞号义务、BeadFlow「D1–D31」过期引用；缺一条明文（支线用线前缀号 / 号段须在 main 登记）（R1 fable-a P1-3 + R2 fable-a 治理缺口2） | main |
| P1-7 | **STATUS/SECURITY 自我指涉与 SHA 过期** | 两处 `7b35bde`、自称 polish 分支、goal1/unblock 核于 SHA 落后一格（`df5d2dd`→`6d1058b`、`c81c233`→`6133307`，均快进合规） | main |
| P1-8 | **schema↔TieScore 映射未成文 + 证据 id 上游缺席** | a2::TieScore 13 字段仅 4 个可原样落库（R2 opus-b F 组）；direct 侧求和口径、`any_direct` 导出、`Band::None ⟺ 省略键`无处成文；`last_contact_evidence_id` 判档侧从不产生，A2「近因句只引最近一条」承诺今天靠 fixture 维持——顺序不能反：先裁判档侧是否携带证据 id，再谈 schema 加字段 | 成文=main（动 schema 字节须与 Goal1 同批）；上游=裁决+Goal1 |
| P1-9 | **BRANCH_MAP 拓扑事实错误** | first-test-candidate 的 `docs/BRANCH_MAP.md` 称 main 与主干无 merge-base；实查共享根 `ea6f62f`（R2 fable-a 事实5）。被测试栈当作处置依据，收编时必修 | Goal1（集成线） |
| P1-10 | **已定价切片残余** | S2（AC-12 嵌中文名 session 缝专用夹具未指认）、G4（重复导入不幂等，D55 不进矩阵）、`soul-egress` 无 Authorization 头→无 key 输入框、AC-28/29/30 具名夹具**字面**导入只在 close-loop（吸收线是行为等价） | Goal1 |
| P1-11 | **`format: date-time` 断言语义待核** | Draft 2020-12 下默认注解不拦（`as_of_utc:"香蕉"` 探针通过）；Goal 1 的 Rust `jsonschema` crate 行为未核——若同为注解，as_of 纪律在存储层无格式兜底（R2 opus-b 6.2） | Goal1（待核项） |

### P2 — 卫生（8 项）

| ID | 项 | 要点 | 修复归属 |
|---|---|---|---|
| P2-1 | `.agent_workspace` 噪音在 main | 510 tracked（含 191 个 `target/`、12 个零字节）、`context/plan` 旧快照分叉 19–146 行、顶层 `PROGRESS.md` 过期自称。清理时**必须保留** `docs/algorithms/DECISION.md:35` 引用的 `.agent_workspace/round3/fable-a/REPORT.md`，否则权威文件出死链 | main |
| P2-2 | 死链 | 权威历史区 6 个（三份 scan-rounds SYNTHESIS 把 Cursor run ID 写成裸相对路径 `bc-…`）；非权威快照另 16 个实例 | main |
| P2-3 | 过期 PR 清场 | #1、#3 已被实质取代仍 OPEN；#11（编排模板）待裁 | 作者/父代理（gh 写权限） |
| P2-4 | 权威文档命名漂移集 | DECISION §4.3 夹具名（`direct_quiet_200_group_yesterday`/200 天 vs 真名 `dormant_direct_group_ping_yesterday`/300 天）、`DORMANT_NOTE_DAYS` 名在代码 0 命中、§6.4 指向 COPY_ZH「第 6 节」实为第 5 节、`assert_non_clinical` 函数名不存在、`DEMOTE_ONE_BAND_DAYS` 在代码中是别名而非主名 | main（动权威 docs 需留痕） |
| P2-5 | crate 内注释过期与就地重复 | `roundx_opus_a.rs` 头注声称存在的 `#[ignore]` `bug_*` 测试已不存在（会误导缺陷盘点）；`SECONDS_PER_DAY` 四处（含 trait crate 内部一处纯重复）；`last_direct_contact_unix = 0` 哨兵重载建议 Option 化（API 形状，随合并义务） | main |
| P2-6 | 跨分支事实无日期复述 | README:10 与 FORMAL:21「WP01–WP11 与 WP13 已落地」不带核于日期，goal1 前进后三处同步漂移 | main |
| P2-7 | 收集脚本丢证据 | 六个 R1 子目录收集时只带 `REPORT.md`，五份探针证据文件丢失（R2 opus-b 已从 `e640b62` 取回交叉核对无分歧）；后续收集须目录级复制或注明证据所在 commit | 编排流程 |
| P2-8 | 工具链 pin 维护 | 1.83.0（2024-11 发布）定期安全/编译器补丁评估无 owner | 作者/维护流程 |

---

## 3. 修复归属汇总

**main 树可修（不动 Goal 1 树、不需作者线下动作）**：P0-3 改动面、P0-4、P0-2 的 main 半边、P1-1、P1-2①、P1-3、P1-6、P1-7、P1-8 成文半边、P2-1、P2-2、P2-4、P2-5、P2-6。其中 P1-1 + P1-3 合计约三行测试代码改动，是全表性价比最高的两项；P0-3 + P0-4 是解除「只读 main 即误导」事故路径的最短动作。

**必须在 Goal1/unblock/D61 主干侧**：P0-1（拓扑归一，17 提交 + 5 文件冲突）、P0-2 的主干 STATUS 半边、P1-4（不变式测试/合取式/fixture 不动字节即可做；schema 收紧须同批）、P1-9、P1-10、P1-11、P1-8 上游执行。

**必须作者侧**：P0-7（账单——解除前 AC-26 与切片 13 在一切当前尖端上物理不可绿）、P0-8（76 项清单 + NSIS 签名件）、P1-5 的关 #4、P2-3、P2-8。前两项应在给作者的汇总里单列为「外部阻塞」，不得继续按普通待办滚动。

**必须裁决（父代理提名、作者拍板）后才有修复动作**：P0-5（PRODUCT_LOCK 私改追认/回滚）、P0-6（BeadFlow 1A/1B/1C）、P1-5 主干归属（2A/2B/2C）、P1-2②（话术归属，DECISION §6.4 程序）、P1-8（证据 id 归属）。

---

## 4. 关闭最短路径（验收人意见，非拍板）

与议题解耦、今天即可安全执行的组合（全部可逆或纯登记）：**合 PR #6 带三件套（P0-3）+ main STATUS/PLAN_INDEX 刷新登记（P0-4）+ 关 PR #4（P1-5 无争议前置，三方文件一致要求）+ 冻结 PR #16 合入直至主干裁决（P0-6 止血）**。随后：裁主干归属（2A 最低成本，但须同条 D 号处置 goal1 侧 17 提交、PR #7 对账、D61–D63 收编、PRODUCT_LOCK 私改四件）→ 执行拓扑归一（P0-1）→ 账单解除后在归一树上 `workflow_dispatch` 五门 → 作者清单。四门中前两门已 PASS，后两门的差额全部可枚举且有量化体量——项目不缺判断，缺的是 owner 与两项作者动作。
