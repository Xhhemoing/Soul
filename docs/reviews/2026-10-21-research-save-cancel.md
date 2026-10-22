# 研究保存取消测量独立评审

候选：`codex/soul-rebuild-20261019 @ 6a56d852d23a051c7c4e9845642791295c541105 + dirty`。本记录由协调者依据独立只读 Agent 的实际反馈整理。范围仅为新增 [research_save_cancel_probe.rs](../../crates/soulcore/tests/research_save_cancel_probe.rs)、必要 Session 保存接口和本次原始日志；不复签另一聊天实施的增长/ZIP/采集停止实验。方法与数字见[第五批取消补充回执](../design/2026-10-20-research-experiment.md#研究保存取消补充实测2026-10-21)。

评审先检查实际测试候选，再核对运行交接。测试使用公开 Session prepare/begin/execute/cancel/complete 接口、合成密封事件和临时 SQLCipher 库，Windows 限定且默认 ignored；无生产代码改动。

| 具体发现 | 修正与复审 |
| --- | --- |
| P2：线程请求公告与查询持锁重叠被称为内部查询竞争 | 公告后线程可能被抢占，在锁释放后才进入方法。字段与汇总统一为 `request_announcement_overlap`、`announcement_to_return_ns`，明确 `internal_method_entry_measured=false`、`actual_store_wait_measured=false`；最终断言只验证公告重叠。定向静态复审通过 |
| P2：发布获胜分支的 Python `output()` 没有等待期限 | 改用临时文件接收输出、120 秒 `try_wait` 截止、超时 kill 后 5 秒有界回收；stdout 检查并限制读取 64 KiB，stderr 不输出正文。定向静态复审通过，本轮没有执行该分支或超时注入 |

实现者同时拆分大 JSON 对象，保留 query/cancel/execute/completion/outcome 各组原始时间点和分母，未提高编译器递归限制。协调者随后实际运行最终候选，评审者独立读取日志并重算：

- `cancel-final.log`：1 项显式 ignored 实验通过，退出 0；9 条 attempt 对应 9 条 sample，100/1000/10000 每档 3 次。
- 9/9 取消、0 已发布；全部完成结果拒绝、目标不存在、ZIP 字节为 0；测试中的事件计数和审计链断言通过。
- 取消与 execute 的公告均 9/9 落在查询持锁观察区间；日志没有测量内部 flag 生效、方法进入或纯锁等待。
- 独立重算 query/cancel/execute 的九组 min/median/max，与协调者汇总一致。10000 的第三次 execute 为 1091.8695 ms、取消为 581.4804 ms，最终仍取消且无文件；不能将差值归因为一个未观测的内部阶段，也不能套用采集 `STOP_BUDGET`。
- 限定该入口的严格 Clippy、单文件 rustfmt 检查日志均退出 0。评审者只读，没有启动构建或复跑测试。

原始日志位于 `C:/Users/86080/AppData/Local/Temp/soul-save-cancel-67828c71dfb645a9844c633e8d95e18c/`：`cancel-final.log`、`clippy-final.log`、`format-final.log`；`summary.json` 是协调者从原始样本重算的派生表，独立评审未依赖它代替原始日志。

两项 P2 已闭环，没有剩余具体发现。本轮未发生发布获胜，因此新增读取器、其超时/终止分支以及失败线程期限只有静态核验，没有运行覆盖；发布成功的跨语言读取仍以第五批增长实验的独立证据解释。未验证真实数据、真实采集、UI 延迟、自然负载分布或完整 ZIP 写盘中取消的精确时刻。不签发正式平台门禁、Goal 关闭或合并批准。
