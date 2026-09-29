# Soul 安全与隐私规范

产品定义以 `docs/PRODUCT_LOCK.md` 为准。本文件只写如何做到与如何测。

## 信任边界

资产：灵魂档案、原始语料、内容密钥、审计链。
防御：同机非管理员进程、恶意导入、恶意 LLM 端点、误操作代理。
不承诺：本机管理员、物理取证、内核恶意软件。

## 出网

一切出站必须带 `egress_class ∈ {E0,E1,L}`（schema 与文档一律用 `L`，不用 `loopback`）。E0 在 v0.1 构建期消除：无 cloud feature、无项目方域名、CI 源码断言。E1 仅用户配置的精确 origin；配置变更会使计划哈希失效；跨 origin 重定向拒绝。WebView 只加载本地资源。关闭 Tauri updater。

## 第三人 redactor

默认占位。单次整条豁免需二次确认，审计 `egress.third_party_body_included` 不含正文。研究预览无豁免。泄漏测试：≥8 字 n-gram。

## 密钥与遗忘

DPAPI → KEK → DB DEK → 每单元 CK。正文字段 AEAD，AAD=行 id+字段名。遗忘销毁 CK，预览影响面。不承诺物理擦除。

## 实现与验证依据

核于 2026-09-28：`main` @ `8aae8f3877bc93529d0a0104c277a85ded9c43aa` 已包含应用代码；续写起点 `arena/01a0e375-soul` @ `efcaf8c77af0992a830ff5b9b8922c3a660ab063` 继承并继续修改了相关实现。旧版“main 没有应用代码”及“等待 PR #2 合入”的描述已经过期，不再作为当前结论。

本文件仍是规范。**源码存在不是验收通过，历史测试通过不是当前提交通过。** 当前状态只在 [STATUS](STATUS.md) 维护；验证必须读取对应提交、平台及覆盖范围的 [门禁记录](gates/README.md)。

| 机制 | 源码依据 | 必须验证的结果 |
|---|---|---|
| 加密主库与字段密封 | `crates/soul-store/src/lib.rs`、`store.rs`、`keys.rs` | SQLCipher 后端校验、正确密钥重开、错误/缺失密钥拒绝；正文字段经内容密钥密封 |
| Windows 平台密钥 | `crates/soul-win-dpapi`、`crates/soul-store/src/keys.rs` | 在目标 Windows 账户和真实平台上验证保护/解封；非 Windows 替身不能代替 DPAPI 实测 |
| 内容遗忘 | `crates/soul-store/src/forget.rs` | 提交前失败回滚；内容密钥销毁、墓碑和推断失据；提交后清理/审计问题不得伪装成“什么都没发生” |
| 应用级导入事务 | `crates/soulcore/src/commands/session.rs` 的 `commit_import` | 导入、审计和图谱重建在同一事务边界内；失败不得留下部分导入 |
| 研究预览 | `crates/soul-store/src/research_preview.rs` | 按主体及逐行用途限制过滤；第三人行数为零，预览不写导出文件 |

规矩（沿用 D57 的版本纪律）：

1. 引用跨分支实现或实测时同时注明分支/提交、日期、平台和实际覆盖范围。
2. 机制说明、源码审阅、运行结果和正式验收分别记录；不把历史测试数量复制为新提交的结果。
3. `docs/gates/20260925-ebff0c9-win.md` 等记录只对所标源码版本负责；当前候选需要自己的受影响检查，不恢复 hosted CI。
4. Wave1 的遗忘清理状态、checkpoint 返回检查等待办见既有实施计划；本节列出验证要求，不宣称这些待办已完成。
5. 平台密钥库和任何新增测试都不扩大开头“不承诺”的威胁模型；不承诺 SSD 物理擦除或管理员级攻击防御。

## 审计

独立库/表，哈希链。字段白名单见 `docs/schemas/audit.schema.json`。禁止 body/text/content/quote/summary/prompt。

## HITL

未知动作拒绝。`plan_hash` 变化拒绝。能力令牌一次性、限作用域、限 TTL。v0.1 不消费写文件令牌。

## 注入

导入/粘贴/文件名只进数据通道。红线用例 AC-25。

## 非临床

诊断词 denylist 由 CI 断言。输出声明「工作假设，非临床结论」。
