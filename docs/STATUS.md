# STATUS

单一事实来源。每个子代理完工必须更新本文件。

## 当前里程碑

**`PLAN_FROZEN`**。Goal 1 已开工：分支 `cursor/soul-goal1-7b1c`。文档 PR `#1` 不夹带应用代码。Goal 2 在 Goal 1 关闭前不要启动。

## 进度

| 项 | 状态 |
|---|---|
| R1 双模型扫描 | 完成，曾 `PLAN_BLOCKED` |
| R2 规范修复 | 完成并落盘 |
| R3 复核冻结 | 完成。仲裁见 `docs/scan-rounds/R3-SYNTHESIS.md`，结论 `PLAN_FROZEN` |
| Goal 1 规划 | 完成。DAG 见 `docs/GOAL1_PLAN.md` |
| WP01 骨架 | 进行中（opus implementer） |
| v0.1 其余 WP | 未开始 |

## 阻塞

无计划阻塞。WP01 必须把 schema `$ref` 接到 `_defs`，补 Unicode/短文本泄漏 fixture，并把崩溃注入写进测试 harness。

## 下一步

批 1：WP01。完成后派批 2（WP02 数据面 ∥ WP08 权限面）。
