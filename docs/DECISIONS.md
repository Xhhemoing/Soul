# 自主拍板记录

作者 2026-08-24 声明：除已给出的产品方向外，其余问题可自主决定。本文件记录拍板，避免后续重开方向战。

| ID | 问题 | 决定 | 理由 |
|---|---|---|---|
| D1 | Soul 是助手还是数字复刻 | 数字复刻为灵魂层，助手为代理层，复刻是本体 | 作者定义是电子版的你 |
| D2 | 要不要默认云端 | 默认全本地；v0.1 无云端 HTTP | 本地优先 |
| D3 | 社交数据怎么进来 | 官方导出或手动导入；v0.1 无 OAuth；禁止爬虫 | 合法可审计 |
| D4 | 能否自动回消息 | v0.1 只起草不发送 | 代发不可逆 |
| D5 | 心理模型是否临床 | 否。可替换特质轴 + 证据档 + 用户纠正 | 避免医疗声称 |
| D6 | 手机何时做 | Android 在 v0.3 | Windows 先闭环 |
| D7 | 研究数据如何用 | v0.1 只预览不写文件；与助手出网分离 | 防偷渡 |
| D8 | v0.1 是否要真预测模型 | 只要 schema 与离线入口 | 先有数据契约 |
| D9 | 技术栈 | Tauri2 + Rust + React | 常驻与打包 |
| D10 | 文件整理是否产品本体 | 否。v0.1 只做只读计划预览；执行在 v0.1.1 | 代理层证明不靠真写文件 |
| D11 | 出网怎么分级 | E0/E1/L，全部经 net_guard | 否则「零出网」不可测 |
| D12 | v0.1 的 E0 | 无实现、无域名、无 HTTP client | 关开关不够 |
| D13 | 第三人正文进 E1 | 默认占位；单次整条豁免；研究无豁免 | 第三人未同意 |
| D14 | 主库 | 加密 SQLite；JSONL 非存储 | R1 S3 |
| D15 | 遗忘 | 销毁内容密钥；不承诺物理擦除 | 可测 |
| D16 | 遗忘 vs 审计 | 审计无内容，可留孤立 UUID | 两承诺共存 |
| D17 | v0.1 导入 | soul-import-v1 + Telegram result.json | 具名才可测 |
| D18 | 研究导出形态 | 预览 only | 脱敏未红队前不落盘 |
| D19 | WP11 写执行 | Goal 1 不做 | 防误读成产品本体 |
| D20 | 记忆是否门禁 | 是 | 作者原意第 6 条 |
| D21 | 人事分析是否门禁 | 是；无 key 则统计降级 | 作者原意第 5 条 |
| D22 | 大五怎么写 | 方向轴 + 弱中强，禁 score/percentile | 非量表 |
| D23 | 采集面 | 仅前台应用时长 | 同意闭环一项即可 |
| D24 | HITL | 未知拒绝；plan_hash 变拒绝；令牌一次性 | 可测 |
| D25 | 注入 | 外部内容永不进指令位；红线进门禁 | R1 S14 |
| D26 | 写码前文档 | 锁、拍板、九 schema、SECURITY、STATUS | 14 份会空转 |
| D27 | 第二份 PRODUCT.md | 禁止 | 防双源 |
| D28 | Goal 2 | 单独规划，Goal 1 关闭后再排（2026-09-04 改指 `docs/GOAL2_PLAN.md`，原文件已移除，见 D64） | Goal 1 必须有终点 |
| D29 | 验收权威 | `docs/ACCEPTANCE.md` 中 Given/When/Then 矩阵（2026-09-04 自开工合同迁出，AC 文本未改，见 D64） | 散文不作门禁 |
| D30 | 工作包增减 | 计划冻结后只减不增 | 防回弹 |
| D31 | 只读整理预览是否留在 Goal 1 | 留。执行仍在 v0.1.1。这是代理层只读证明，不是产品卖点 | 否决评审中删除 AC-18 的提案 |
| D32 | 未锁边是否写 `machine_band` | 写。`locked ⟺ user_band.is_some()`；重建边 `machine_band` 常为 `Some` | 缺席无法区分「未锁」与「换血前的行」；Goal 1 吸收线已钉测试 |
| D33 | G1+ 后 `{群聊次数}` 口径 | 渲染持久化 `group_out_count + group_in_count`（可含 0）；不从证据重算；不改 COPY_ZH 措辞 | owner 群消息不再归因后 `group_out_count` 结构为 0；改模板须先改 COPY_ZH |
| D34 | 遗忘 vs 锁（GC-6/7） | Goal 1 不实现。rebuild 仍只遍历有观测的 peer；时间戳保持非 Option | 墓碑/Option 化是迁移，不是默认 |
| D35 | GC-9b「由你本人指定」 | 禁止。COPY_ZH 未冻结该 key 之前产品非测试源码与 fixture 不得出现该句 | 测试里的反向断言字面量除外 |
| D36 | v1 按发言人数判 Direct/Group | 维持。单活跃发言人的群可被判 Direct | 改判据要动冻结契约 |
| D37 | Telegram `date_unixtime` 范围 | 对应 RFC3339 年必须在 1970–9999（四位年）；否则该消息 defect，不入库 | 否则 fail-hard rebuild 永久失败；导入非事务 |
| D38 | v1 / Telegram 民事日 | `is_civil_datetime` 按月天数（含闰年）校验；`2026-02-31` 为缺陷 | schema 层与本地校验双钉 |
| D39 | `IntakeReceipt.ignored` | 加法字段（question_id + reason token）；`answered` 不计被锁轴拒答 | WP03 遗留；链上不加新审计枚举 |
| D40 | Goal 1 人事摘要走哪套话术 | `soul-draft` 调用冻结 `soul_algo_trait::a2_render`；不改 a2.rs 模板；锁定边仍丢掉 `filed_band` | A2 是渲染器；COPY_ZH 与 crate 文案漂移另记 |
| D41 | 计划权威放在哪 | 只在 `docs/`。`.agent_workspace/**` 与各轮过程稿一律只是过程稿，冲突时以 `docs/` 为准 | 与 D27 同一条防双源纪律 |
| D42 | 人脉档位用哪套 | T4D，唯一权威。三道门全在一对一计数上；群聊行只供展示与近因，不判档 | `DECISION.md` §1–§3，`ALGO_FROZEN` |
| D43 | 特质轴与人事摘要 | 特质轴 A0（问卷 + 纠正锁）；A2 是档位的纯渲染器，源码级禁止第二套阈值；A1 空转是预期行为 | `DECISION.md` §2 |
| D44 | F04c（久别之后一来一回回强） | 不加第三道降档门。不满意只走 `DECISION.md` 第 5 节回退链，并在本文件留痕 | `DECISION.md` §4.2 / §5 |
| D45 | `as_of` 口径 | 一次重建全库只有一个 `as_of`（缺省 = 全库 `max(occurred_at)`）。禁 per-peer，禁墙钟 | `DECISION.md` §3 |
| D46 | intake 与轴锁的关系 | 锁定的轴不 `place_axis`；答案仍落证据，`ignored` 理由 `axis_locked_by_user` | D5 + `BLOCKERS.md` G2；收据字段见 D39 |
| D47 | owner 群消息如何归因 | 不对该会话的历史发言人写 Outgoing；incoming 仍记实际发送者 | `BLOCKERS.md` G1+ |
| D48 | 人脉图纠正 | 生效档 = 用户锁定时的档，机器档另存。GC-9 话术须先加性增补 `COPY_ZH.md` | `BLOCKERS.md` G3；与 D32、D35 配套 |
| D49 | 唯一实现主干 | `cursor/soul-goal1-7b1c`。停 `agent/dev-sota`、关 PR #4；分支一律 `cursor/` 前缀 | `BLOCKERS.md` M1 |
| D50 | 算法 crate 何时进应用工作区 | 按 M2，在 Goal 1 线上一次 merge 时把两个 crate 加进成员表。**不在计划 PR 里做** | 计划 PR 不夹带应用代码 |
| D51 | 算法 crate 的依赖方向 | 箭头恒为应用 → 算法 crate。crate 保持纯函数、`forbid(unsafe_code)`、不读墙钟 | `DECISION.md` §6.2 |
| D52 | 两个算法 crate 现在合并吗 | 不合，后置。两侧天数常量用工作区测试钉相等；产品 crate 禁止再写第三处 `DEMOTE_ONE_BAND_DAYS` 字面量 | `DECISION.md` §0 |
| D53 | 冻结文稿名 vs 仓库真名 | 实现跟真名：`soul-algo-tie` / `soul-algo-trait`、`crates/soul-graph/src/build.rs` | `BLOCKERS.md` §0 |
| D54 | Goal 1 怎么算关闭 | FORMAL 验收矩阵与 PRODUCT_LOCK 十三片切片**同时**过 | `BLOCKERS.md` 开篇：两套门互补不互斥 |
| D55 | 重复导入是否必须幂等 | 是 P1 设计目标，不进 v0.1 切片、不进验收矩阵 | D30 只减不增 |
| D56 | AC-26 的「打包绿」怎么读 | 读作「package job 绿 + 作者签名安装包」。CI 里不下 NSIS、不跑完整 `tauri build` | `BLOCKERS.md` S5 |
| D57 | STATUS 怎么写 | 只写本文件所在树可验证的事实；跨分支必须带分支名与提交号并标「核于」日期；同一棵树内对同一分支的「核于」提交号必须一致；「阻塞：无」只覆盖本树 | 旧 STATUS 的「阻塞：无」曾被误读成可以合 `main` |
| D58 | `tie_strength` schema | 计划义务：字段清单（band、分列计数、last_contact、silent_days、as_of、algorithm_id）写入规范。本计划 PR 先采纳 Goal 1 已接线的 schema 正文，避免把裸 string 版推回 `main`。进一步收紧 `tie_strength` 须与 Goal 1 `schemas.lock.json` 同批重算 | Round 1 探针：两条线 `tie_strength` 仍是裸 object；9 份 schema 正文已分叉 |
| D59 | `tie_strength` 收紧已执行 | 本 PR 将 `tie_strength` 从裸 object 改为 T4D 可复核面；`algorithm_id` 触发整包必填；空对象仍合法。必须同时允许 `machine_band` / `user_band` / `locked_by_user`（D32/D48），否则 Goal 1 重建边会被 `additionalProperties:false` 打红。Goal 1 合并后跑 `xtask schema-freeze` 复核 lock | Round 2 P0 |
| D60 | COPY_ZH P4 触发比较符 | 近因句触发写成闭区间 `>= DEMOTE_ONE_BAND_DAYS`，与降档常量同一点。用户可见句子「你们最近半年没有往来」不动 | Round 3：原稿「超过 180 天」是已作废的 `>` 写法；crate 实现与测试已是 `>=` |
| D61 | 门禁在哪跑 | **不用 hosted CI**。GitHub Actions 已在仓库设置里关闭（`actions/permissions.enabled=false`），`.github/workflows/ci.yml` 删除。GitHub 只做代码存储与同步。门禁 = G-L（Linux 开发机 `just ci-full`）+ G-W（作者 Win11 `scripts/gate-win.ps1`）+ G-M（`scripts/author-manual-checklist.md`），证据落 `docs/gates/<yyyymmdd>-<sha7>-<linux|win>.md` | 作者 2026-09-03 明令；私库账本不可控（8/25 起五门 0 step）；hosted 本来就证不了托盘 / UAC / DPAPI / WebView2 真机行为。AC-26 的「CI」自此读作本地门禁 |
| D62 | Linux 门禁机 | 本项目指定的 Linux 门禁机（4 vCPU / 3.9 GB）。首份 G-L：`docs/gates/20260903-e2b4e48-linux.md`。这台机没有 webview 开发库也没有 sudo，桌面壳 mock-runtime 测试（`just desktop-shell-test`）不在 G-L 内，由 G-W 的 `desktop-test --all-targets` 覆盖 | 冷编 35 分钟可接受；装 GUI 栈要 root |
| D63 | 没有门禁文件的提交能不能合 | 不能。合到 `main` 或 Goal 分支的每个 sha 必须在同一提交序列里带对应 G-L 记录；改到 `apps/desktop`、`soul-collect/windows.rs`、`soul-win-dpapi`、NSIS 的还要 G-W | 本地门禁唯一弱点是「谁跑的、跑了没」，用落盘记录堵上；D29 同一条纪律 |
| D64 | 开发流程文档 | 作者 2026-09-04 指示：仓库只保留代码与规划。移除 `FORMAL_WORK_PROMPT.md`、`GOAL2_POLISH_PROMPT.md`、`PLAN_VERIFY_PROMPT.md`、`templates/THREE_ROUND_DUAL_SCAN.md`、`scan-rounds/`。验收矩阵、工作包、红线迁至 `docs/ACCEPTANCE.md`（AC 文本未改）；Goal 2 方向迁至 `docs/GOAL2_PLAN.md`。**合并规则**（为落实上述指示而拟，非作者原话，待作者确认）：这些文件在任何后续合并中一律保留删除；来源分支对验收矩阵的实质变更（如 `main` 的 AC-28–34）移植进 `ACCEPTANCE.md`，不恢复原文件。评审轮次溯源标签（D14/D25/D31）已归一化为「2026-08 计划评审」。同理，`.agent_workspace/`、`docs/agent-*.md`、`.github/workflows/` 在任何后续合并中一律不带入。历史见 `3eb0d8f`（清理前的 `cursor/soul-goal1-7b1c` HEAD，远端亦有；本机另有 tag `backup/pre-cleanup-20260904` 指向同一提交） | 流程配置不是产品；写下规则防止合并时静默复活或静默丢失 |
| D65 | 平台定位 | 作者 2026-09-04 原话：「本项目不是 windows only，只是前期现在 Windows 系统上进行开发和测试，后期支持核心部署到 linux mac windows 多端，同时可以接入 mac linux windows Android 多端进行数据获取和反馈呈现」。归纳：Windows 是 v0.1 的开发与测试平台，不是唯一目标。核心后续部署 Linux / macOS / Windows；接入端（数据采集与结果呈现）含 macOS / Linux / Windows / Android。v0.1 边界表不变。**不变量**：非测试代码中 `cfg(windows)` 目前只在 6 个文件（`soulcore/src/netwatch.rs`、`soul-win-dpapi/src/lib.rs`、`soul-store/src/keys.rs`、`soul-collect/src/lib.rs`、`soul-collect/src/source.rs`、`apps/desktop/src-tauri/src/instance.rs`）；此外新增 Windows 绑定须先记一条决策（此不变量为落实作者澄清而拟，非作者原话，待作者确认） | 多端是产品方向；移植面现在有界，不让它扩大 |
| D66 | 长期目标是否三选一 | 不三选一。个人数字分身、面向其他人的商业产品、人格与行为建模研究共同发展。作者 2026-09-25 明确“都有啊”，并要求自主分析、编制可由多 agent 共同推进的具体计划。长期路线落在 ROADMAP.md；产品边界仍只由 PRODUCT_LOCK 定义，研究与助手用途许可继续分离 | 记录本次明确方向，不把研究增益或商业诉求当作扩大采集/出网的授权 |
| D67 | 后续计划如何协作推进 | 采用 ROADMAP 的阶段与工作包、GOAL2_PLAN 的近期执行细节；协调者 + 最多两条隔离实现线 + 独立审查，文件单 owner，共享接口先冻结。新 Q2 / 后续编号不扩充冻结 Goal 1 WP；Goal 1 未关不启动 Goal 2。本次自主选定的样本量、容量和商业假设均标明待实测，不冒称作者逐项指定 | 在原两条实现线约束内支持多 agent；继续保持 D64 已删除材料的删除状态，不另建工作流系统 |
| D68 | 受阻安装验收期间能否推进其他部分 | 作者 2026-09-25 明确“记录并跳过这里，继续完成其他部分”：真实 NSIS 临时副本与完整安装闭环本轮记为用户要求跳过 / NOT RUN，不再反复索取该操作许可。先在隔离源码基线推进 GOAL2_PLAN 的 01–04 独立合成质量验证与已复现 v0.1 缺陷最小修复，完成独立规格及质量审查；这是执行顺序调整，非 Q2-00 正式启动或 Goal 1 关闭 | 只调整该项阻塞对其他工作的顺序；Linux 仍暂缓、人工不代勾，算法/schema/IPC/产品边界不变，不推送或合并 |
| D69 | 如何将使用框架接入首个生产用例 | 按作者 2026-10-16「按照这个框架进行项目的调整」指示，先接起草页与起草慢任务。撤回线性点为 Session 内不再发新派发许可，并拒收旧代次回复；已发许可可能继续出网，不承诺首字节撤回。资料写入前使准备/结果失效，失败写入也保守失效；同址设置也失效。物理 worker 结束前不释放唯一槽。准备/回复接收采用单调时钟两分钟窗口（复用既有能力令牌 120 秒默认值），过期重新核对；这是本轮实现选择，不是作者逐项指定 | 不新增出网用途、自动记忆选择、迁移或重导语义；不扩大 D55。仅起草 HTTP 移出会话锁，非所有长任务独立控制。GitHub 同步沿用既有授权，草稿 PR 不合并 |
| D70 | 后续如何推广执行框架和导入结果呈现 | 按作者继续依据最新框架、逐板块先规划验证再编码的指示，先完成导入交付状态与人物摘要。摘要采用 begin/锁外 execute/complete，与起草共用唯一 E1 物理 worker 槽；本地 counts 不占网络槽。沿用 D69 的来源失效和已发许可余量边界，不增加摘要新期限或第二批准面板。导入页在当前 WebView 运行期共享 pending/unknown/最近计数回执；未知结果暂停继续提交，不能称回滚，重启不证明未提交 | 是独立规划复核后的实现选择；IPC 形状、持久化 schema、算法、用途许可、D55 及遗忘重导不变。导入交付不构成持久幂等；摘要/起草之外的扫描和导入长锁、原生平台验收仍待补齐。见 [计划与验证](design/2026-10-16-import-summary-continuation.md) |

> 注（2026-09-11 M0-2）：合入 `goal1-closeout-c49c` 后保留 `main`/c49c 的 D32–D60；原 cleanup 门禁三条（曾编号 D50–D52，与 main 算法 D50–D52 撞号）重号为 D61–D63；D64/D65 保留。凡引用「D50 门禁/hosted CI」处，现应读作 D61。

`agent/dev-sota` 线上另有一条「e0 发送 crate 禁令」曾被 BLOCKERS 称作 D32。**不是**本文件的 D32。PR #6 整份合入时，必须把 BLOCKERS 正文里的那个「D32」改写为届时下一个空闲 ID，或加括注「dev-sota 编号，非 docs/DECISIONS.md 之 D32」。樱桃摘时禁止复用本表号。

新增拍板只在本文件追加，禁止在 `.agent_workspace/` 或其他文档另立第二份（D27 / D41）。

## 2026-10-16 当前框架续写决定

作者要求按计划完成剩余能力，并允许工程选择自主推进。以下是编码前经独立规划复核的保守选择，不表示平台门禁或 Goal 已关闭。

| 编号 | 问题 | 决定 | 边界 |
| --- | --- | --- | --- |
| D71 | 长工作与撤回 | 扫描沿用 owned 授权快照并移出 Session 锁；导入长存储工作排斥其它物理慢任务及活跃采集。存储占用时所有 Session 存储入口立即拒绝，不持 Session 等 store。清除端点、丢弃批准、撤回采集仍可达；撤回审计若暂不能记录，明确显示待记录并在完成后重试 | 不称物理取消，不撤回已发送字节，不新增文件执行；直接丢弃非生产 owned completion 只能恢复容量，不能声称审计完成 |
| D72 | 本次记忆用途 | 只读明确候选，候选内关键词/类型筛选；缺事实时间则澄清。本次选择仅授权本次本地起草引用。Owner excerpt 必须另选且纳入最终 E1 计划；third/mixed/unknown 保持占位。引用指向手写记忆及实际密封 blob，不能伪造外部事实证据 | 用途与研究许可不继承；不增加语义事实推断、向量检索或人格规则；按真实最终 JSON 字节预算裁剪并重算预览/出处 |
| D73 | 导入身份与冲突 | 核心持有不可变准备，确认只收操作/预览 UUID；namespace 由用户明确选择，不从文件名、时间或正文猜。外部自然键密封保存，按有界批次比较；同键同内容跳过，内容/发送者/时间/上下文变化整体拒绝，缺失消息不删除旧消息 | 不增加正文 hash；不同来源账号不自动混同；本人身份冲突拒绝 |
| D74 | 旧库、迁移与遗忘重导 | 新增表与 schema stamp 同事务升级，失败回滚，较新版本拒开。历史无 external ID 不补造；默认拒绝可能重叠历史，用户可明确确认仅本次及以后建立索引、历史可能重叠。遗忘同事务删除 CK 关联索引并保留拒绝重导所需无正文墓碑；不重生旧密钥 | 旧数据仍可读；不称全历史去重；只用合成库验证迁移，不打开真实用户库 |
| D75 | 操作恢复与本地反馈 | 持久操作行只存 UUID、固定状态、计数及错误代码；领域数据、原导入审计与成功回执同事务。独占进程 lease 证明旧 worker 不存活后才能标 interrupted。重启不恢复原文或旧批准、不自动重放 E1。导入后只用现有算法生成本地统计候选；忽略只抑制明确候选，纠正接已有用户锁，来源变化后旧洞察失效 | COMMIT 不确定则查回执或报告 unknown，不假称回滚；无端点自动调用，无自动训练或研究授权 |
| D76 | 导入洞察如何用于本次起草 | 按作者持续依项目计划开发、自动审查及阶段同步的指示，先接一个有效 bundle 的一个联系人的完整可信 A2 统计。独立 typed context 绑定完整来源快照，不能冒充手写 memory；默认只本地核对，另选 includeCounts 才将纯派生统计纳入既有 E1 最终预览与单次批准。沿用可信 summary_body 聚合适配，不把原事件或证据重新分类为 Owner，不接受 UI 统计正文；不带标识或原文。单项按既有 64 KiB 实际 JSON serializer 完整容纳或拒绝，不裁剪统计或引用后假称完整 | 本机模板不变，false 时端点未获计数；引用仅为统计材料出处，不认证生成草稿逐句有据。准备、派发及回复接收复验，来源失效后先审计真实请求再拒收；Session ignore/recompute 保守失效，直接新建 bundle 不自动改旧行。无 raw resolver、新研究用途、算法/schema/锁变更或新 Goal 1 关闭条件；见[方案](design/2026-10-16-insight-draft-context.md) |
| D77 | 怎样核对一条导入消息来源 | 按作者持续开发指示，先具体规划并独立复核，新增两步纯本地读取：精确关系/证据元数据，再明确选择一条事件正文。每次重验当前 Graph 用途、关系和实际 event 唯一索引归属，不套用洞察 namespace；旧数据缺精确索引即不可查看。身份和正文首次解封前检查真实密封行归属及长度；identity 本地查看上限沿既有65536字符的四倍字节，超大合法导入仍保持原导入/去重语义。owner 群消息拒绝；本人直接消息还须核对同关系/真实来源/会话的有效 peer 发言，整次有限读取，不能只凭可伪造关系引用补出对方 | 只显示单条纯文本，不继承 Draft/E1/Memory/研究或 paste 权限，不认证旧 A2 快照或真实收件人。无新 hash/schema/锁或读取伪审计；来源/地址/关闭/卸载使页面结果失效，清状态不宣称内存安全擦除。见[具体方案](design/2026-10-16-local-evidence-view.md) |

## 2026-10-19 统一辅助重建决定

工作区维护清理后，本轮从可取得的生产提交 `600f309b8c5f81fe8596d3575f4539e5e769ec89` 和原计划提交 `64faa56989239f26d76f618b49dcb4dda6788292` 重建。此前未同步的实现和 D78–D96 原文不可取得；保留其编号空缺，不根据摘要伪造历史原文或移用历史验证结论。以下决定是本轮编码前的新决定。

D99 的工程调度预算为每秒检查，连续间断超过 5 秒、墙钟与单调时钟差值偏离超过 5 秒或真实系统恢复事件均使过期提醒仅留收件箱。时间保留明确偏移；提醒的亚毫秒精度向上取到毫秒，不得截断导致提前。完整 W5 资格核对使用 owned 慢工作，并与 item/schedule/category 三个版本及唯一 attempt 的创建同事务；该 claim 是本次 OS 派发许可，后续取消只能另行撤回，不能承诺 SQL 与 OS 原子。OS 调用前再检查当前元数据与撤回意图，以减少不必要的过期投递。

| 编号 | 问题 | 决定 | 边界 |
| --- | --- | --- | --- |
| D97 | U1/U2 的事项、目标、来源与版本 | 事项稳定 UUID，revision 为仅作相等比较的不透明 UUID；列表只返回元数据，正文经明确详情读取。手工目标复用 Commitment Memory 及其 CK，创建同事务，普通 Memory 正文编辑同 savepoint 更新事项 revision，绑定期间拒绝改出 Commitment。观察项按真实 namespace/contact 唯一，创建冲突不换来源、不重开终态；重绑固定 bundle 必须显式 CAS。手工目标可明确绑定一项当前观察的 item/revision 和真实来源快照，来源失效使目标 stale，但不取消独立承诺或销毁其 CK；显式重绑/清除才能恢复有效。全量编辑和完成/取消均核对 revision；时间只接受明确带偏移的 RFC3339，null 表示无截止，不解析旧语料中的相对时间 | 新增加法 schema，不新增正文副本、事实推断、算法阈值、研究许可或自动 E1。自身遗忘仍销毁 CK；遗忘影响确认比较精确事项/revision 集合并在原遗忘事务内处理，坏辅助记录不能阻断密钥销毁。事项转交在 core 实际 held context、批准、派发与回复接收全程绑定版本和来源；实际已发请求先如实审计再拒收过期结果，物理 worker 结束前不释放槽。Windows 真机及正式 G-L/G-W/G-M 仍沿原暂缓边界，不以此决定声明验收通过 |
| D98 | U3 的提交、恢复、失败与采集交接 | 同库每事项一行工作，pending revision 与实际 claim UUID/claimed revision 分开；领域变更和待办、结果和本 claim 完成分别同事务。保存 exact completed claim 回执，运行中的新 pending 不被旧 completion 清除。未知提交仅核对原 claim，不重放；物理 worker 真结束后才可协调遗弃 claim。失败保存固定 code，用户按当前事项 CAS 显式重试，不定时热循环。恢复复用既有规范路径 OS lease 的新持有期一次事务，旧批准和采集同意不恢复。新导入事务创建缺失稳定观察项，既有项不隐式换 bundle。Graph 变化在事务尾核对实际固定来源全快照；直接 store 变更仅合并需核对待办，定时器不全量重算 W5。原 W5 读取错误先严格传播，不能把数据库错误伪造成事实消失并永久失效 | 后台占既有真实物理慢槽，held/pending 前台准备优先，不调用 invalidate_drafts 清走前台材料，不持有端点或 E1 许可。导入 owned worker 在长 store 锁外真实 stop/flush/join collector；每次显式 grant/revoke 改授权代次，只有原代次仍有效、审计已协调且有可重建 source factory 才恢复，错误和丢 completion 不假称已恢复。仅应用存活期间运行；driver 锁内再次检查 stop，锁外 execute，已开始工作仍 complete；线程失败有固定可读状态。平台真机与正式门禁边界不变 |
| D99 | U4/U6 的提醒、通知与明确反馈 | 提醒默认关闭，单独 schedule revision 绑定当前事项 revision；开启、稍后和改期要求用户明确的未来 RFC3339 时间，不从截止/正文推断。任何事项业务 revision 变化、来源失效、完成取消或遗忘，与旧提醒失效/撤回意图同事务。每 schedule 最多一个持久 attempt；先 claim 后 OS 提交，提交成功、展示、打开、撤回分别记录，未知不重投，已知失败也须用户重新安排。新持有期恢复、系统恢复或调度/时钟异常期间的过期提醒只留收件箱，不补发。类别偏好仅 manual_goal/observation 的明确静默与恢复，独立 CAS、可撤销，恢复不补积压；helpful/not_relevant/clear 是独立版本反馈，不改事实、人格、语气、批准或完成。语气仍由已有明确 profile 偏好入口修改 | 为完成原计划新增隔离 Windows Shell_NotifyIconW 绑定，核心不依赖平台。自有隐藏顶层 HWND 和线程，固定通用文字，仅 attempt UUID 跨边界，点击由核心解析稳定事项并重读；不承诺通知中心驻留或应用退出后的提醒。v4 回调 u16 在同 HWND 持有期不复用，有界队列和实际图标容量分离；超时只报 unknown，保留真实 owner，实际 join 后才报停止。资源清退不冒称用户未见/撤回成功。默认实际图标容量 1、队列 32、资源保留 5 分钟为工程预算，非推断算法或提醒时刻；过去回执和 u16 墓碑保留至 transport 退出。桌面调度最多每分钟一次新 OS 提交，超额只留收件箱；此频率为首版避免打扰的产品默认，不表示效果已获实测。原生 Windows、安装和人工效果验证仍必须单独执行 |
| D100 | U5 的自动本地准备与明确预览 | 默认关闭，仅当前 manual_goal 由用户按事项和独立偏好 revision 明确启用；后台复用 U3 持久待办与物理槽。按真实 OWNER_PROFILE_ID 的四项语气枚举，调用既有 `soul_draft::render_template(ProfileBrief, TemplateContext::opening())`，保留 BODY_SLOT；不把目标 summary 冒作收到的消息。只持久化模板 UUID、事项/偏好 revision、完整固定来源、owner、四项语气、准备时间和 opening_v1 类型，瞬时正文丢弃。显式预览精确模板 ID 时，同一受保护读取核验当前事项、CK、来源、偏好和四项语气后重新渲染；失配先同事务排入刷新，提交后在事务外返回拒绝；操作读取错误不永久失效。真实 profile 保存仅四项语气变化时同原 savepoint 合并已启用事项待办，不因人格轴或 user_set 标记变化而改模板，也不改事项/提醒 revision | opt-out 同事务撤销可见模板并更新偏好；保留共用 U3 pending/claim，防止取消模板时丢失新业务版本的基础核对。worker 按精确偏好 revision 拒收旧模板，仍完成基础 U3 工作；重新启用新 revision，旧结果不能接收。语气四枚举 A→B→A 的纯函数结果可复用，但不恢复任何批准。缺失/读取失败的 owner profile 明确不可用，不静默降级 neutral。模板及偏好纳入精确遗忘范围，来源遗忘不删除独立目标。无端点、E1 或自动采用；准备回复仍走既有明确上下文、预览和单次批准。后台只减少本地准备等待，不声称任务完成或实际效用已验证 |

## 2026-10-19 核心预备工作顺序

| 编号 | 问题 | 决定 | 边界 |
| --- | --- | --- | --- |
| D101 | 系统验收后置时如何继续核心计划 | 作者在核心优先建议后要求按计划逐部分分析可行性、编码、独立 review 并提交，完成本轮计划再停止。允许将 VAL-01 的 V1 记录/汇总、V2 brief/T1、V3 T2/T3 与 Session 桥接提前为测试专用合成质量预备；U7 先查具体差集，无新缺口不重复扩测试；DATA 继续原设备 L1 设计收敛 | 仅调整该合成适配的实施顺序，不启动正式 Q2-00/05、真人试验或 DATA/MODEL 生产实现，不关闭 Goal 1/U7；G-L 暂缓、系统 G-M 与安装后置。只用合成 fixtures、临时库与 loopback，不增产品 UI/IPC/schema/权限、依赖或 gate。分阶段本地提交已获授权，不因此推送、合并或发布；实际结果写入既有设计和 STATUS |
