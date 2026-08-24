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

## 加密落地（WP01 实测结论）

结论：**走 SQLCipher 路线**，不需要回退。`ASSUMPTION` 中「若打包成本过高则退回 SQLite + 字段级 AEAD」这一条本 WP 未触发。

实测依据（`crates/soul-store-api/tests/sqlcipher_smoke.rs`，ubuntu 与 windows-latest 两个 CI job 都跑）：
`rusqlite` 开 `bundled-sqlcipher-vendored-openssl`，`PRAGMA cipher_version` 返回 SQLCipher 4.5.7；建表写行后关闭，带同一 key 重开可读回；不带 key 打开则第一次读失败；写错 key 同样第一次读失败；库文件字节里搜不到那行明文。

两层加密，两层都不可省：

| 层 | 保护对象 | 构造 | 密钥 |
|---|---|---|---|
| 整库 | `%LOCALAPPDATA%\Soul\soul.db` 的全部页 | SQLCipher | DB DEK |
| 字段 | 记忆标题/摘要、事件正文、联系人显示名等 `sealedText` 指向的正文 | XChaCha20-Poly1305，AAD = `行 id\|字段名` | 每遗忘单元一把 CK |

密钥链：Windows DPAPI 保护 KEK；KEK 包裹 DEK 与全部 CK。整库加密挡的是把 `soul.db` 拷走的人；字段级 AEAD 挡的是「删了行但页面还在」，并且让遗忘有具体的销毁对象——CK 一销毁，即使整库仍能打开，那段正文也解不出来。AAD 绑行 id 与字段名，密文因此不能被搬到另一行或另一列重放（`crates/soul-store-api/tests/field_aead.rs` 正反两向断言）。

Linux 上没有 DPAPI。CI 与 headless 测试走 `TestKeyProvider`：密钥来自测试固定值或临时目录，不接触任何平台密钥库，只用于让存储契约在 Linux 上可跑。`KeyProvider` 抽象与 DPAPI 实现属于 WP02；WP01 只落结论与冒烟证据。

不承诺：抵抗本机管理员、物理取证、内核恶意软件；不承诺 SSD 物理擦除。遗忘的可测语义是「CK 已销毁，正文不可解，派生推断降为 orphaned」，UI 必须照此如实写。

## 审计

独立库/表，哈希链。字段白名单见 `docs/schemas/audit.schema.json`。禁止 body/text/content/quote/summary/prompt。

## HITL

未知动作拒绝。`plan_hash` 变化拒绝。能力令牌一次性、限作用域、限 TTL。v0.1 不消费写文件令牌。

## 注入

导入/粘贴/文件名只进数据通道。红线用例 AC-25。

## 非临床

诊断词 denylist 由 CI 断言。输出声明「工作假设，非临床结论」。
