MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 2 fable-b 报告：G2/G3/A2 验收行交叉核验

任务：Round 1 落地后，把 FORMAL 的 AC-08/12/26/28–33 与 D32–D58、BLOCKERS G1–G3、`ALGO_FROZEN`、COPY_ZH、crate 夹具字节逐一对拍。只读 + 只写本目录；未动 `docs/`，未做 git 操作。

## 一句话结论

**六行新 AC 与 D 表、G 项、冻结算法之间没有硬矛盾**；Round 1 主诉（缺 Given/When/Then、双门没写下来）已关闭；真正剩下的洞是 **G1+/D47（owner 群消息扇出归因）在两套关闭门里都没有验收行**，以及 Round 2 引导面修复（历史段注记等）**尚未提交**。

## 核验基准的特别说明

本槽位运行期间父代理正在落 Round 2 改动：核验对象是 `b380f67` + 工作树未提交 diff（FORMAL 历史段注记与红线 11 自查段、AC-29 措辞修订、PLAN_INDEX/PLAN_VERIFY/STATUS 引导面、`relationship.tie_strength` 收紧 + lock 重算）。文中所有「已落地」均指工作树；**提交前一切结论以字节为准需复核 R-1**。

## 三份交付物与其头条

| 文件 | 头条结论 |
|---|---|
| `AC_CONSISTENCY.md` | 硬矛盾 0。逐行绿：AC-28↔D42/D33/G1（夹具 30/10+1/1 已对字节）、AC-29↔锚保全+自愈（`three_directs`=共 3 条 2 发 1 收，Round 1 过程稿「每方向各 3」是错的，落地版对）、AC-30↔D45/§3、AC-31↔D46/D39/G2、AC-32↔D48/D32/D35/G3（P5 原句已核 COPY_ZH:71）、AC-33↔D33/D43/§5.7（Some(0,0) 照渲染 vs 缺席整句不出，两权威咬合不打架）。**落地编号与 Round 1 过程稿存在 28↔29、31↔32 内容换位**，已建映射表；`docs/` 权威面无悬空引用。四个已裁决的漂移列入备忘（§6.1 旧 crate 名→D53；§4.3 夹具名 200 天 vs crate 真名 300 天；字段拼法 `direct_out` vs `direct_out_count`；过程稿参数过时） |
| `ACCEPTANCE_GAP.md` | G/W/T 结构六行全齐；双门语义在 FORMAL×2 + D54 + STATUS 四处一致写死。覆盖缺口按级别：**P1 = G1+/D47 无验收行**（矩阵+切片双门都拦不住扇出 bug 关闭 Goal 1，建议扩 AC-28 的 Given/Then，最省且不动编号）；P2 = AC-28 丢了 BLOCKERS 的「分列精确」等值断言、F04c 反向钉死无 CI 绊线（提议的是钉死**没有**第三道门，非加门）；P3 = as_of「过滤前取 max」半句、`axis_locked_by_user` token 未点名、A1 空转断言、AC-33 缺席臂防误读括注。边界值（179/180/359/360 等）不补：crate 测试 + 红线 11 + 同判义务传递覆盖，接受矩阵瘦身 |
| `COPY_RISK.md` | 历史段：`b380f67` 处**在且未注记**；工作树已被七层注记包死、作者原话逐字保留，L2 处置合格。**头号残余风险是整批未提交（R-1/P1）**。未注记逐字副本仍在 `.agent_workspace/context/plan/` 与 round1 过程稿（=L4，README 非权威横幅已核实，勿删、加负向探针）。另记两个 copy 小疵：「九份 schema」实为 10 份业务 schema（FORMAL/PLAN_INDEX/D26 三处同错）；STATUS「AC-32/33」建议加落地编号括注 |

## 顺手完成的 R1 指定探针

schemas.lock 一致性（R1「Round 2 须确认哈希与文件一致」）：**工作树 11 份 schema sha256 与 lock 全对、JSON 全可解析**，含本轮改过的 `relationship.schema.json`（lock 已同批重算）。

## 纪律自查

- 未新增工作包；未提议 F04c 第三道门（G-3 是反向钉死）；未改 `docs/`；未提交。
- 引用全部落到单点权威（D-id / G 项 / `ALGO_FROZEN` 节号 / crate 文件行号），本报告不复抄任何判档阈值数值。
- 给 Round 3 的最短清单：① 落提交（R-1）；② G1+ 进 AC-28（G-1）+ 分列等值（G-2）；③ F04c 反向钉死进 AC-29（G-3）；④ 三个 P3 括注顺手带走（G-4/5/7、九份计数、AC-32/33 括注）。
