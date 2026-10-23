# Soul 核心框架 v1.3 修订与执行记录入口

2026-10-23 整理：本文件保留既有链接入口。原文、命令、失败、独立评审范围与未完成项完整保存在[归档快照](../archive/2026-10-23-core-framework-v1.3-execution.md)，归档不表示所有验收已完成。当前状态见 [STATUS](../STATUS.md)，后续规范化执行见[计划](../plans/2026-10-23-repository-normalization.md)。

## D109 实施回执（2026-10-22）

[原始 D109 实施回执](../archive/2026-10-23-core-framework-v1.3-execution.md#d109-实施回执2026-10-22)：精确人物＋场合的本人表达偏好、存储/遗忘、Session/IPC/UI 合成检查与各次失败保留；[独立评审](../reviews/2026-10-22-scoped-register-implementation.md)只对所列候选和覆盖范围负责。

## 后续一：核对当前偏好与定位记忆（2026-10-22）

[原始偏好核对与记忆定位回执](../archive/2026-10-23-core-framework-v1.3-execution.md#后续一核对当前偏好与定位记忆2026-10-22)。历史测试数量、dirty 状态及未测范围按原日期解释，不作为新提交验证结果。

## 后续二：Windows 原生窗口合成交互（2026-10-22，进行中）

[原始原生窗口回执](../archive/2026-10-23-core-framework-v1.3-execution.md#后续二windows-原生窗口合成交互2026-10-22进行中)。标题保留以兼容旧链接；实际操作已在作者按 Escape 后停止，剩余项目未验收，不因归档而补签通过。

## 后续三：本地 API 化可行性与建议

[原始只读可行性调查](../archive/2026-10-23-core-framework-v1.3-execution.md#后续三本地-api-化可行性与建议)保留调查依据。当前方案已拆至 [ADR-002](../decisions/ADR-002-local-api-runtime.md)和[本地 API 架构](../architecture/local-api.md)：共享 runtime、类型来源、事件同步与合成验收按新文档执行，未实现部分明确标注。
