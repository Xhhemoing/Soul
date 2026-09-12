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

## 实现实证在哪（指针，不是本树结论）

本文件是**规范**：写清要做到什么、拿什么测。**它不记录任何一条已经跑过的实测结论。** 上面那条密钥链在本树是设计约束，不是已验收事实——`main` 上没有应用代码。

实测依据（`crates/soul-store-api/tests/sqlcipher_smoke.rs`；历史上 ubuntu 与 windows-latest 两个 hosted job 跑过，`2e72ddf` 五门绿；D50 之后在 G-L 与 G-W 两台本地门禁机上跑，见 `docs/gates/`）：
`rusqlite` 开 `bundled-sqlcipher-vendored-openssl`，`PRAGMA cipher_version` 返回 SQLCipher 4.5.7；建表写行后关闭，带同一 key 重开可读回；不带 key 打开则第一次读失败；写错 key 同样第一次读失败；库文件字节里搜不到那行明文。

| 那边多出的节 | 记的是什么 | 本树能不能引用为已完成 |
|---|---|---|
| 加密落地 | SQLCipher 选型的实测结论与「整库 + 字段」两层各挡什么；Linux 无 DPAPI 时测试走哪条密钥路径 | **不能**。是那条分支上的 CI 结果 |
| DPAPI 落地 | `DPAPI → KEK` 从骨架变成真实调用之后的密钥文件格式、作用范围、`unsafe` 隔离在哪个 crate、各测什么 | **不能**。同上 |

规矩（D57）：

1. 跨分支事实必须带分支名与提交号并标「核于」日期，本节即按此写。
2. **不要把那两节提前抄进本文件。** 抄过来就等于宣称 `main` 上已有这些实现，而本文件所在的树里一行应用代码都没有——这正是 STATUS 已经杀掉的那类全局假话。
3. PR #2 合入 `main` 时把两侧合成一份：本文件的规范面（信任边界、出网、redactor、审计、HITL、注入、非临床）保留，实证两节随实现一起进来，不重复规范面已有的句子。
4. 那两节里的任何结论都不改变本文件开头「不承诺」的三条：本机管理员、物理取证、内核恶意软件。平台密钥库不扩大威胁模型的承诺面。

## 审计

独立库/表，哈希链。字段白名单见 `docs/schemas/audit.schema.json`。禁止 body/text/content/quote/summary/prompt。

## HITL

未知动作拒绝。`plan_hash` 变化拒绝。能力令牌一次性、限作用域、限 TTL。v0.1 不消费写文件令牌。

## 注入

导入/粘贴/文件名只进数据通道。红线用例 AC-25。

## 非临床

诊断词 denylist 由 CI 断言。输出声明「工作假设，非临床结论」。
