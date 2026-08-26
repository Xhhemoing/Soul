MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 1 fable-b：验收矩阵与工作包审计报告

只读审计。未改 `docs/`、未改 `crates/`、未 commit/push。产出四份专项文件在本目录。

## 一句话结论

计划的产品边界是紧的（无 v0.1 膨胀、无 Goal 2 泄漏），但验收矩阵对冻结算法是**零覆盖**——一个 T0 图谱实现能通过全部 26 行门禁（BLOCKERS G1 已实证 Goal 1 现行 `Tally::band` 正是 T0），且 PR #1 种子文档的「未开工」时态对任何读者都已是假话。修复面收敛为三件事：补六行算法门禁、把关闭语义改成双门、把 STATUS/FORMAL/README 改成三态诚实。

## 交付物索引

| 文件 | 内容 |
|---|---|
| `AC_MATRIX_GAP.md` | AC-01–AC-27 逐行 × 13 条切片 × T4D/A0 义务；判定 keep 23 / amend 3（AC-08、AC-12、AC-26）+ 关闭语义与 schema 锁两条 amend 级计划义务 / add 6（AC-28–33）/ not-a-gate 8 项（AC-27 维持既有 not-a-gate 标注） |
| `SOTA_ACCEPT.md` | 六道缺失门的完整 Given/When/Then：T4D 直连判档（含 F04c 钉死为冻结代价）、群扇出不产档 + G1+ 归因、as_of 单钟、人脉边纠正（含 GC-9 话术门）、intake 不绕轴锁（含 A1 空转钉死）、A2 纯渲染器 |
| `WP_DAG.md` | WP 拓扑计划级仍真（WP12 保持删除，不发明新包）；G1/G1+/G2/G3 按表落进 WP03/05/06/09/10 范围；PR #1 九条陈旧声明逐条引用与处置 |
| `KEEP_OR_KILL.md` | 膨胀排查（锁本体无膨胀）；Goal 2 隔离完好；真正的 KILL 区是陈旧引导句（重派 planner、重跑三轮扫描的诱导源）与已履历化的 `PLAN_VERIFY_PROMPT.md` |

## 关键发现（按危害排序）

1. **矩阵对算法零覆盖（P3 最大洞）**。T4D/A0 只存在于 `docs/algorithms/`，PRODUCT_LOCK/DECISIONS/FORMAL 三份计划权威一次都没有出现。切片 3「人脉图 v0」与自主拍板「边=互动强度」是钩子，但没有任何 AC 行消费它。补法见 `SOTA_ACCEPT.md`（AC-28–33，全部 CI 可跑，夹具复用 main 上已有的 `soul_algo_tie::testing`）。合法性：D30 只减不增约束的是工作包不是验收行，SHARED_BRIEF 第 5 条明确授权补行。
2. **关闭语义单门 vs 双门**。FORMAL「Goal 1 完成 = 矩阵全过」低于 BLOCKERS_FROZEN 的「矩阵与切片两套门互不否决」。不改，T0 图谱可借 26 行全绿关闭 Goal 1——这是隐性砍灵魂层。
3. **PR #1 时态假话**。「v0.1 实现未开始」「尚未写应用代码」「下一步派 planner」九条陈旧声明逐条列在 `WP_DAG.md` 第三节；诚实三态 = 计划已冻结 / Goal 1 已在 PR #2 线落地待合入（hosted 未绿、作者清单未过）/ main 无应用。
4. **`relationship.tie_strength` 仍是裸 `object`**（本分支与 Goal 1 线两版皆然）。字段清单（band、分列计数、last_contact、silent_days、as_of_utc、algorithm_id）应作为计划义务写进文档；**不要在本分支改 schema 字节**——Goal 1 线 `schemas.lock.json` 钉了 sha256，字节变更须与锁重生成在 Goal 1 线同一获批变更完成（细节见 `AC_MATRIX_GAP.md` 第四节）。
5. **WP 拓扑不用动**。清单、并行声明、D31 分界三处全部仍真；T4D 吸收以既有 WP 范围内批注落位，WP12 不复活。
6. **两处 amend 级措辞**：AC-12 的断言缝要从 crate 边界挪到 session/产品缝（BLOCKERS S2，否则嵌中文名夹具永远红不了）；AC-26「打包绿」按 S5 改「package job 绿 + 作者签名安装包」。
7. **膨胀与重复排查为阴性**：锁本体、砍/留表、后期路线、Goal 2 隔离全部咬合；唯一的范围风险来自陈旧引导句诱导重做（`KEEP_OR_KILL.md` C 区）。

## 对评价维度的自查

| 维 | 本报告贡献 |
|---|---|
| P1 单源 | DAG 不复制进 FORMAL；GC-9 话术先改 COPY_ZH；「九 schema」计数精确化；单源风险登记两处功能性重复 |
| P2 可测 | 六行新门全部有 Given/When/Then + 谁跑（CI）+ 夹具名 |
| P3 冻结吸收 | AC-28–33 把 DECISION §2/§3/§4/§6 与 BLOCKERS G1–G3 验收化；常量、闭区间、as_of 纪律、模板绑定全部进门禁 |
| P4 诚实 | 九条陈旧声明逐条处置；三态措辞给定 |
| P5 不膨胀 | not-a-gate 八项封死（预测准确率、去重、F04c 第三道门、WP12 等）；F04c 钉死方向是「钉冻结代价」不是「加门」 |
| P6 SOTA | 全部建议可由父代理直接落笔；无需再读 Goal 1 整树 |

## 建议 Round 2 落笔顺序（供父代理，非本槽位执行）

1. FORMAL：插入 AC-28–33；改关闭语义为双门；AC-12/AC-26 措辞；删「开工第一动作」换 BLOCKERS 工序指针。
2. STATUS：三态重写 + 补 `ALGO_FROZEN` 行。
3. PRODUCT_LOCK：自主拍板「人脉图」行加一句「边强度=T4D band（`docs/algorithms/DECISION.md` 唯一权威）」；不动 13 条切片。
4. DECISIONS：新增 D 条目记录 tie_strength 字段义务与 schema 锁协议、GC-9 话术增补通道。
5. README：三态一句话 + 补 `docs/algorithms/` 链接；`PLAN_VERIFY_PROMPT.md` 履历化。
