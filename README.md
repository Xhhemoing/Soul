# Soul

灵魂级个人软件。根据你授权的社交档案、电脑操作和日常记录，在 Windows 本机复刻电子版的你——人格、记忆、心理倾向、人脉图——并辅助处理电脑事务、起草回复、分析人与事。本机采集与浅层处理；云端深度分析默认关闭。行为数据可在授权下用于行为预测研究。

## 这个仓库现在有什么

| 位置 | 内容 | 能不能装出来用 |
|---|---|---|
| `main`（你正在看的这条线） | 计划权威面 `docs/`（产品锁、拍板、验收矩阵、schema、安全规范）+ 已冻结的灵魂层算法 crate：`crates/soul-algo-tie`（人脉关系强度 T4D）、`crates/soul-algo-trait`（特质轴 A0 / A1 / A2 / A3）。都是纯函数：不读时钟、不碰存储、不出网 | **不能。** `main` 上没有桌面壳、没有主库、没有安装包 |
| `cursor/soul-goal1-7b1c` | Goal 1 实现主干：Tauri 桌面壳、加密主库、导入、人脉图、记忆、审计、策略面、安装 smoke。WP01–WP11 与 WP13 已落地 | 该分支上可以，但尚未合回 `main`，hosted CI 与作者 Win11 手动清单也尚未全绿 |

一句话：**计划已冻结，算法已冻结，应用在 Goal 1 分支上，`main` 还装不出东西。** 三件事互相独立，进度以 [`docs/STATUS.md`](docs/STATUS.md) 为准。

## 30 秒找到权威文件

先看 [`docs/PLAN_INDEX.md`](docs/PLAN_INDEX.md)——它是「哪件事以哪份文件为准」的索引。急的话按这个顺序读：

- 产品锁定（唯一产品权威）：[`docs/PRODUCT_LOCK.md`](docs/PRODUCT_LOCK.md)
- 自主拍板：[`docs/DECISIONS.md`](docs/DECISIONS.md)
- 算法冻结（人脉 T4D / 特质 A0）：[`docs/algorithms/DECISION.md`](docs/algorithms/DECISION.md)
- 正式开工提示词与验收矩阵：[`docs/FORMAL_WORK_PROMPT.md`](docs/FORMAL_WORK_PROMPT.md)
- 进度：[`docs/STATUS.md`](docs/STATUS.md)
- 安全规范：[`docs/SECURITY.md`](docs/SECURITY.md)
- 数据契约：[`docs/schemas/`](docs/schemas/)
- Goal 2（Goal 1 关闭后再开）：[`docs/GOAL2_POLISH_PROMPT.md`](docs/GOAL2_POLISH_PROMPT.md)
- 计划验证提示词（**历史存档**，勿当新工单）：[`docs/PLAN_VERIFY_PROMPT.md`](docs/PLAN_VERIFY_PROMPT.md)
- 三轮双模型模板：[`docs/templates/THREE_ROUND_DUAL_SCAN.md`](docs/templates/THREE_ROUND_DUAL_SCAN.md)
- 父代理全流程编排（派单/轮次/Git，不覆盖产品锁）：[`docs/templates/PARENT_ORCHESTRATOR.md`](docs/templates/PARENT_ORCHESTRATOR.md)

## 跑一下现有的算法 crate

```bash
cargo test --workspace          # Rust 1.83，无外网依赖，无 unsafe
cargo run -p soul-algo-tie --example matrix   # 打印 T4/T4D 的判档矩阵
```

这两条只验证冻结算法本身。应用级验收（安装、托盘、导入、遗忘、出网）在 Goal 1 分支上，门禁清单见 `docs/FORMAL_WORK_PROMPT.md`。

## 边界

不是通用聊天壳，不是虚拟恋人，不抓取他人社交账号，不默认上传原始记录，不做临床诊断。v0.1 只起草不发送、只预览不写文件、采集默认关。完整清单在 `docs/PRODUCT_LOCK.md`。
