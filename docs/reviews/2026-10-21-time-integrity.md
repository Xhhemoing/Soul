# 第四批时间完整性独立评审

候选为 `codex/soul-rebuild-20261019 @ 6a56d852d23a051c7c4e9845642791295c541105 + dirty`。本记录由协调者依据独立 `soul_reviewer` 的实际反馈整理；实现与验证见[第四批回执](../design/2026-10-20-research-experiment.md#第四批缺失与非法时间处理2026-10-21)，处理规则唯一决定为 D108。没有提交、合并或平台门禁签发。

评审者先读取用户任务、规则和实际 diff，再核对实现说明；对 build.rs 使用本轮修改前副本区别上批 DisplayInstant，不将旧 dirty 重算为本批变更。评审仅只读，未启动构建、测试或读取真实数据。

| 规则审查发现 | 反例及闭环 |
| --- | --- |
| P2：摘要查找可能短路绕过坏来源 | 第一条合法证据匹配最近联系后，后面的损坏 graph 引用不再被检查。先实际 RED 复现，再改为逐条严格解析全部 resolved；坏引用在前/后的回归覆盖原反例，静态复审确认不再短路 |
| P3：“完整结构”可能被理解为全部字符串语义均已校验 | Timestamp/Sha256Hex 是透明字符串包装。D108 实施说明明确为 required 存在、类型/枚举/UUID 可解码及额外时间校验，保留消费者更严格检查；没有借此新增通用摘要格式规则 |

Telegram 增量及主模块候选的独立静态评审无剩余具体发现：Result 收集不会返回部分观测；未知标签仍忽略；无 owner 也验证显式引用；有 owner 在派生写入前完成解析。摘要、sources、insights 的直接调用者没有吞错后继续使用部分引用的迁移；原授权、来源和预算校验保留。旧导入和当前持久导入测试均调用真实 Session，检查域数据、blob/CK 和审计，后者允许事务外 Rejected 状态但无成功 receipt。

最终原始运行日志已由同一独立评审者核对：`crates-green.log` 为 364 项普通测试和 1 项 doc-test，`core-green.log` 为 69 项，`retention-green.log` 为 7 项，`desktop-green.log` 为 130 项通过、2 项真实采集用例 filtered，四份均 `EXIT=0`；合计 570 项普通测试和 1 项 doc-test。`clippy-final.log` 的五个指定 crate `--all-targets -- -D warnings` 也为 `EXIT=0`。日志位置和完整命令见第四批回执，不累计 RED 或重复运行。

评审者结合此前实际 diff 核对摘要两种顺序、graph 无 owner/时间边界、Telegram 类型矩阵和两个 Session 回滚用例均已通过，未发现新增具体阻塞。P2/P3 已闭环。根/桌面格式检查及 `git diff --check` 的通过依据为协调者实际执行回执，未声称评审者亲自运行或独立读取了这些检查的原始日志。

未验证完整根 workspace、前端、真实采集、安装卸载或 G-L/G-W/G-M。没有改变冻结算法、schema、依赖或许可语义，不签发正式 Goal/Q2、合并或全项目验收。
