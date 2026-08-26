# Soul

灵魂级个人软件。根据你授权的社交档案、电脑操作和日常记录，在 Windows 本机复刻电子版的你——人格、记忆、心理倾向、人脉图——并辅助处理电脑事务、起草回复、分析人与事。本机采集与浅层处理；云端深度分析默认关闭。行为数据可在授权下用于行为预测研究。

## 这个仓库现在有什么

| 位置 | 内容 | 能不能装出来用 |
|---|---|---|
| `main` | 计划权威面 `docs/` + 冻结算法 crate `soul-algo-tie` / `soul-algo-trait` | **不能。** 没有桌面壳 |
| Goal 1 实现主干 `cursor/soul-goal1-7b1c` 与本关闭线 `cursor/goal1-closeout-c49c` | Tauri 桌面壳、加密主库、导入、人脉图、记忆、审计、安装 smoke；本关闭线正在把 T4D/A0/A2 接到产品路径 | 该线上可以编，hosted CI 与作者 Win11 清单尚未全绿 |

一句话：**计划已冻结，算法已冻结，应用在 Goal 1 线上，`main` 还装不出东西。** 进度以 [`docs/STATUS.md`](docs/STATUS.md) 为准。

## 30 秒找到权威文件

先看 [`docs/PLAN_INDEX.md`](docs/PLAN_INDEX.md)。急的话按这个顺序读：

- 产品锁定：[`docs/PRODUCT_LOCK.md`](docs/PRODUCT_LOCK.md)
- 自主拍板：[`docs/DECISIONS.md`](docs/DECISIONS.md)
- 算法冻结：[`docs/algorithms/DECISION.md`](docs/algorithms/DECISION.md)
- 验收矩阵：[`docs/FORMAL_WORK_PROMPT.md`](docs/FORMAL_WORK_PROMPT.md)
- 进度：[`docs/STATUS.md`](docs/STATUS.md)
- 安全规范：[`docs/SECURITY.md`](docs/SECURITY.md)
- 数据契约：[`docs/schemas/`](docs/schemas/)
- Goal 2（Goal 1 关闭后再开）：[`docs/GOAL2_POLISH_PROMPT.md`](docs/GOAL2_POLISH_PROMPT.md)

## 边界

不是通用聊天壳，不是虚拟恋人，不抓取他人社交账号，不默认上传原始记录，不做临床诊断。v0.1 只起草不发送、只预览不写文件、采集默认关。完整清单在 `docs/PRODUCT_LOCK.md`。
