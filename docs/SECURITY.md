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

## 审计

独立库/表，哈希链。字段白名单见 `docs/schemas/audit.schema.json`。禁止 body/text/content/quote/summary/prompt。

## HITL

未知动作拒绝。`plan_hash` 变化拒绝。能力令牌一次性、限作用域、限 TTL。v0.1 不消费写文件令牌。

## 注入

导入/粘贴/文件名只进数据通道。红线用例 AC-25。

## 非临床

诊断词 denylist 由 CI 断言。输出声明「工作假设，非临床结论」。
