# 自主拍板记录

作者 2026-08-24 声明：除已给出的产品方向外，其余问题可自主决定。本文件记录拍板，避免后续子代理重开方向战。

2026-08-25 增补：

- **D32–D40** 与 Goal 1 吸收线（`cursor/goal1-unblock-a073`）同号同义，禁止本文件另写一套。
- **D41 起** 为本计划打磨轮次把 `ALGO_FROZEN` / `BLOCKERS_FROZEN` 已拍过、但尚未进本表的板收进来。**不改产品方向、不新增工作包。**

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
| D28 | Goal 2 | 拆到 GOAL2_POLISH_PROMPT.md | Goal 1 必须有终点 |
| D29 | 验收权威 | FORMAL 中 Given/When/Then 矩阵 | 散文不作门禁 |
| D30 | 工作包增减 | R2 后只减不增 | 防回弹 |
| D31 | 只读整理预览是否留在 Goal 1 | 留。执行仍在 v0.1.1。这是代理层只读证明，不是产品卖点 | 否决 R3-sol 删除 AC-18 的建议 |
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

`agent/dev-sota` 线上另有一条「e0 发送 crate 禁令」曾被 BLOCKERS 称作 D32。**不是**本文件的 D32。PR #6 整份合入时，必须把 BLOCKERS 正文里的那个「D32」改写为届时下一个空闲 ID，或加括注「dev-sota 编号，非 docs/DECISIONS.md 之 D32」。樱桃摘时禁止复用本表号。

新增拍板只在本文件追加，禁止在 `.agent_workspace/` 或其他文档另立第二份（D27 / D41）。
