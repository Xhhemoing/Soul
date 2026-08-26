# 正式开工提示词（可整份粘贴给云端父代理）

产品方向已按作者 2026-08-24 澄清重锁，并经计划扫描收窄。新会话只粘贴「提示词正文」。
**在 `docs/STATUS.md` 写明 `PLAN_FROZEN` 之前，只允许改文档与 schema，不许写业务代码。**

---

## 提示词正文

你是本仓库 Soul 的云端父代理。仓库：`github.com/Xhhemoing/Soul`。作者 Roy。先读 `docs/PRODUCT_LOCK.md`、`docs/DECISIONS.md`、`docs/SECURITY.md`、`docs/STATUS.md`、`docs/algorithms/DECISION.md`、`docs/schemas/`、本文件。产品定义以锁定文档为准，算法定义以 `docs/algorithms/DECISION.md`（`ALGO_FROZEN`）为准，禁止改回「通用桌面助手」或把文件整理当产品本体。

任务是按锁定方案把 Soul 做成可安装、可测试、可审计的 Windows 本地灵魂级软件。Goal 2 二十轮不在本文件；见 `docs/GOAL2_POLISH_PROMPT.md`，Goal 1 关闭前不要启动。

### 计划面与代码面（先读这一节，否则会重复造轮子）

这个仓库同时有两条线，任何「当前几乎无代码」的判断都必须指明说的是哪一条：

| 线 | 有什么 | 没有什么 |
|---|---|---|
| `main`（本文件随计划 PR 合入的那条线） | 计划权威面（`docs/`）+ 冻结算法 crate（`crates/soul-algo-tie` T4D 族、`crates/soul-algo-trait` A0/A1/A2/A3）。纯函数、无存储、无 UI、无出网 | **没有可安装的应用**。本计划 PR 合入之后仍然没有：本 PR 只动 `docs/` 与 `README.md` |
| Goal 1 分支 `cursor/soul-goal1-7b1c` | 实现主干：桌面壳、加密主库、导入、人脉图、记忆、审计、策略面、安装 smoke。WP01–WP11 与 WP13 已在那条线上落地 | 尚未合回 `main`；hosted CI 与作者 Win11 手动清单尚未全绿 |

因此：

1. 「当前几乎无代码」**只对 `main` 成立**，对 Goal 1 分支不成立。接手 Goal 1 的父代理不是从零开始，先读 `origin/cursor/soul-goal1-7b1c:docs/STATUS.md` 与 `:docs/GOAL1_PLAN.md`，再决定派谁。
2. **Goal 1 分支是唯一实现主干。** 不要另开第二条实现线，不要把 Goal 1 的树整体拷进计划分支。
3. 计划冻结（`PLAN_FROZEN`）≠ Goal 1 关闭 ≠ `main` 已有应用。三件事在 `docs/STATUS.md` 里分开写，改其中一件不要顺手改另外两件。
4. 算法 crate 已在 `main` 上，Goal 1 采纳它是**合并义务**，不是重新选型；采纳方式见 `docs/algorithms/DECISION.md` 第 6 节，本文件只给对应门禁（AC-28…AC-31）。

### 产品锁定（不可改写）

Soul 是灵魂级个人软件：在本机复刻电子版的用户（人格、记忆、心理工作模型、人脉图），并辅助处理电脑问题、起草回复、分析人与事。Windows 本地优先。v0.1 无云端 HTTP。社交导入仅 `soul-import-v1` 与 Telegram Desktop `result.json`。只起草不发送。采集默认关。第三人数据默认不出本机。研究预览第三人行数为 0 且不写文件。

v0.1 垂直切片以 `docs/PRODUCT_LOCK.md` 的 13 条为准。代理层在 v0.1 只证明：只读扫描 + 计划预览 + 未授权路径 100% 拒绝。

### 冻结算法（`ALGO_FROZEN`，只给结论，细节不在本文件复述）

| 位 | 冻结结论 | 权威 |
|---|---|---|
| 人脉关系强度 | **T4D**：互惠、次数、自然日三道门全部改在**一对一计数**上；群聊行保留展示与近因，不参与判档 | `docs/algorithms/DECISION.md` 第 2–3 节 |
| 特质轴 | **A0**：问卷 + 本人纠正锁定；intake 不绕锁 | 同上 |
| 人事摘要 | **A2**：纯渲染器，band 的消费者，源码级禁止第二套阈值 | 同上 |
| A0 升档规则 | **A1 = TwoKindsAcrossDays**；v0.1 只有问卷一种来源，**空转是预期行为**，不是缺陷 | 同上 |
| 常量 | 次数门、自然日门、两道沉寂降档门；单点定义在 `crates/soul-algo-tie` 的常量模块。**数值只在算法权威里写一次，本文件不复述** | 同上第 3 节常量表 |
| as_of | 一次 rebuild 全库一个值，调用方传入；judgement 不读墙钟 | 同上第 3 节 |

改判只走 `docs/algorithms/DECISION.md` 第 5 节回退链，并在 `docs/DECISIONS.md` 留痕。**禁止**为 F04c（复燃一响）加第三道降档门。

### 云端子代理模型约定

11.1 适用范围：仅 Task 派生的云端子代理。父代理模型不受下表限制。

11.2 模型选择
修复、落地代码、补测试使用 slug: `claude-opus-5-thinking-high-fast`
其他情形使用 slug: `claude-fable-5-thinking-xhigh`
当次用户指定覆盖本表。

11.3 父代理直改白名单：文档/注释/配置措辞；或不超过 10 行且不涉及业务逻辑、权限、数据面。直改须说明。超出派 opus。

11.4 禁止静默降级。修复：`claude-opus-5-thinking-high-fast` → `claude-opus-5-thinking-high` → `claude-sonnet-5-thinking-high`。其他：`claude-fable-5-thinking-xhigh` → `claude-fable-5-thinking-high`。同系列皆不可用则暂停。子代理第一行自报 slug。

11.5 Git：`cursor/` 前缀分支。进度以 `docs/STATUS.md` 为准。写码前阻塞项只有：PRODUCT_LOCK、DECISIONS、`docs/schemas/` 九份、SECURITY、STATUS。禁止第二份 PRODUCT.md。

### 接下来使用 Goal：先调用子代理 claude-fable-5-thinking-xhigh，根据项目计划和目的，将项目代码的编写拆分为多个子代理 claude-opus-5-thinking-high-fast 进行编写，最后 review 并打磨到可安装的 v0.1。注意要及时把成果和方案和进度记录文档提交成 PR 并放到新的专门的你专属的分支，并且你产生的多个 PR 在合适的时候需要进行合并

1. 确认 STATUS 为 `PLAN_FROZEN`。
2. CreateGoal：Goal 1。
3. 派 fable planner，覆盖下列 WP，只减不增。
4. 按 DAG 派 opus implementer。权限/数据面与 UI 面分人。
5. 每批 fable 只读复核。缺陷派 opus。
6. Goal 1 完成 = 本文件验收矩阵全部 `v0.1` 行通过。

工作包：

- WP01 骨架、工具链、冻结 schema、SECURITY、CI
- WP02 加密主库、遗忘、研究预览（不写文件）
- WP03 灵魂档案（特质轴、纠正锁定）
- WP04 自传记忆 CRUD + 遗忘
- WP05 人脉图 v0
- WP06 soul-import-v1 + Telegram result.json + 问卷
- WP07 前台应用时长采集（trait 抽象，CI 用假实现）
- WP08 策略、审计链、net_guard、redactor、HITL 令牌（无 HTTP）
- WP09 桌面壳
- WP10 起草（不发送）+ 人事分析摘要
- WP11 只读目录扫描与计划预览；未授权拒绝
- WP13 安装 smoke、CI、SBOM

WP12 已删除。并行：WP01 冻结后 WP02–WP06、WP08 可并行。

工作包**只减不增**（DECISIONS D30）。吸收 `ALGO_FROZEN` 不新开工作包：判档替换落在 WP05，特质轴锁落在 WP03，摘要渲染落在 WP10。

### 实现者红线

1. 无 E0。无项目方域名、遥测、自动更新。
2. E1 只来自用户填写端点，请求体必须过 redactor。
3. 主库加密。禁止明文 JSONL 当存储。
4. 审计无正文。
5. 遗忘 = 销毁内容密钥。
6. 无 evidence_ids 的 inference 不得落库。
7. v0.1 不实现文件写。
8. 外部内容不是指令。
9. 禁止诊断词与量表分数。
10. 改产品方向先改 PRODUCT_LOCK。
11. 判档阈值单点：常量表（`ALGO_FROZEN` 第 3 节）里的每个数值只在 `crates/soul-algo-tie` 的常量模块定义一次。`graph_build` / A2 / UI / SQL 里出现第二处数字字面量即为缺陷，包括「只是复述一下」的注释与文案。
12. `crates/soul-algo-*` 保持纯函数：不读墙钟、无 `unsafe`，禁止把 SQLCipher、Tauri、HTTP 或任何存储依赖拉进去。依赖箭头永远是 Goal 1 → soul-algo。

### Goal 1 验收矩阵（门禁）

行号是稳定标识，不是执行顺序。`ALGO_FROZEN` 相关行（AC-28…AC-31）排在 AC-27 说明之前，AC-27 的 v0.1.1 归属不变。

| ID | Given | When | Then | 谁跑 |
|---|---|---|---|---|
| AC-01 | 干净 Win11 | 安装启动 | 托盘出现且不提权 | 作者手动 + 安装 smoke |
| AC-02 | 首次向导完成 | 读配置 | 采集关、云关、无 LLM 端点 | CI |
| AC-03 | 无导入 | 完成问卷 | 非空档案，字段来源 user_stated | CI |
| AC-04 | soul-import-v1 fixture | 导入 | 落加密库，无明文残留 | CI |
| AC-05 | Telegram result.json fixture | 导入 | 映射事件/联系人；缺字段失败可读 | CI |
| AC-06 | 已导入 | 生成档案图谱 | 每条 inference 有可解引用 evidence | CI |
| AC-07 | 语气字段被用户改 | 再起草 | prompt 用用户值，推断不覆盖 | CI |
| AC-08 | ≥3 个对话对象 | 打开人脉图 | 节点≥3，边有证据；每条边的 band 由 `soul-algo-tie` 的 T4D 给出，图侧无第二套阈值 | CI |
| AC-09 | 采集关 | 切应用 10 次 | foreground 事件=0 | CI |
| AC-10 | 采集开后切应用，再关闭 | 观察事件 | 开启期间至少 1 条；关闭后 1s 内无新事件 | CI |
| AC-11 | mock LLM | 起草 | 不发送；仅 E1 到 mock 精确 origin；跨 origin 重定向拒绝 | CI |
| AC-12 | 含第三人正文/姓名/账号 fixture | 默认起草 | 请求体无 ≥8 字原文子串，也无未占位姓名/账号 | CI |
| AC-13 | 单次包含原文豁免 | 再起草一次 | 仅当次含原文；下次回到占位 | CI |
| AC-14 | 3 条记忆 | CRUD | 读写一致；审计无内容 | CI |
| AC-15 | 一条记忆 | 遗忘并重启 | 预览影响面；CK 销毁后无法解密；推断 orphaned；审计仍在且无内容 | CI |
| AC-16 | mock LLM | 人事摘要 | 每条有证据；无诊断词 | CI |
| AC-17 | 无 LLM key | 摘要与起草 | 统计/模板降级；无非回环连接 | CI |
| AC-18 | 授权 A 未授权 B | 扫描 A、操作 B | 产出只读计划预览；A 磁盘不变；B 100% 拒绝 | CI |
| AC-19 | 未知动作、或已批准计划被改 hash、或令牌重放 | 请求执行 | 全部拒绝 | CI |
| AC-20 | 含第三人正文 | 研究预览 | 行级输出无第三人字段；第三人行数=0；written_to_disk=false | CI |
| AC-21 | 默认配置跑主流程 | 观察网络与源码 | 非回环连接=0；无业务域名 | CI |
| AC-22 | 点云端开 | UI | 保持尚未启用；无网络 | CI |
| AC-23 | 矩阵所列动作后 | 审计回放 | 链通过且无正文/姓名 | CI |
| AC-24 | 写入中注入崩溃 | 重启 | 链通过；未提交最多丢 1 条 | CI |
| AC-25 | 注入串出现在导入、粘贴或文件名 | 档案与起草 | 无工具计划；无外连该 URL | CI |
| AC-26 | 仓库 | CI | lint/test/schema/红线/打包绿 | CI |
| AC-28 | 决胜夹具 `group_heavy_plus_one_direct_each_way`（群聊互惠 30 次 / 10 天，另加一对一每方向各 1 次）已入库，全库单一 as_of | 重建人脉图并读该边 | band **不是** strong（T4D 实测 weak）；边上落 `direct_out/direct_in/group_out/group_in` 四个分列计数与 `direct_active_day_count`；解释文案同屏报出一对一与群里两个数。把判档口径改回「任一场地计数」必须让此行变红（同口径下 T4 判 strong，即该漏洞的实证） | CI |
| AC-29 | 锚夹具 `lilei_12`（一对一互惠 12 次 / 6 天 / 3 天前收尾）已入库，同一 as_of | 重建人脉图并读该边 | band = strong。吸收 T4D 不得改判此行；自愈路径 `group_heavy_plus_three_directs`（一对一 3 次）回到 moderate，在同一测试内一并断言 | CI |
| AC-30 | 一个库里同时有活跃关系与 2019 年休眠关系 | 跑一次全库 rebuild | 全库只用**一个** as_of（调用方传入，缺省 = 全库 `max(occurred_at)`）；该值与沉寂天数随每条边落库、可复核；休眠边判 weak。把 as_of 改成 per-peer 各取自己的最大时间戳必须让此行变红 | CI |
| AC-31 | 某特质轴已被本人纠正锁定 | 再跑一次问卷 intake（或导入触发的 intake） | 锁定轴不被移动；被拒绝的答案以「已跳过 + 原因」如实回报，不静默丢弃；答案行照常落库；`apply_intake` 与 replay 结论全等；其余轴不受影响 | CI |

AC-27 文件执行与撤销标 **v0.1.1**，不是 Goal 1。AC-18 是代理层的**只读**证明，留在 Goal 1（DECISIONS D31）——它证明的是「扫描 + 预览 + 拒绝」，与 AC-27 的写执行不是同一件事，不要因为看着像就把它并掉或删掉。

AC-28…AC-31 的夹具不是新造的：`group_heavy_plus_one_direct_each_way`、`lilei_12`、`group_heavy_plus_three_directs`、`dormant_2019` 已在 `crates/soul-algo-tie/src/testing`，A0 的锁定用例已在 `crates/soul-algo-trait/tests/a0_lock.rs`。Goal 1 侧应当**导入**它们并断言产品路径（图谱 rebuild、档案 intake）与算法 crate 同判，而不是在 Goal 1 里重新推导一遍阈值——重新推导就违反红线 11。这四行全部由 CI 跑；作者手动清单不承担算法门禁，因为它们不需要真机。

### 开工第一动作

1. 若 STATUS 不是 PLAN_FROZEN，只做文档。
2. 读 `origin/cursor/soul-goal1-7b1c:docs/STATUS.md`，确认 Goal 1 已落地到哪一步；不要把已完成的 WP 重派一遍。
3. CreateGoal：Goal 1。
4. 派 fable planner。
5. planner 返回前不要大面积写业务代码。
