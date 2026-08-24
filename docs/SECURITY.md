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

Linux 上没有 DPAPI。CI 与 headless 测试走 `TestKeyProvider`：密钥来自测试固定值或临时目录，不接触任何平台密钥库，只用于让存储契约在 Linux 上可跑。

WP02 已落 `KeyProvider` 抽象，两个实现在 `crates/soul-store/src/keys.rs`：

- `TestKeyProvider`：`from_seed("…")` 由固定串按域分离派生 DEK 与 KEK（同一 seed 跨进程可重现，崩溃测试的子进程靠这个重开同一个库）；`in_dir(dir)` 把 64 字节种子放在 `dir/soul-test-keys.bin`，首次使用时生成。两条路都不碰平台密钥库，名字里写明只作测试用。
- `DpapiKeyProvider`：**骨架**。类型可构造、跨平台可编译，但两个取密钥入口都返回 `KeyError::Unsupported` 而不是编造密钥——Win32 绑定要引入 `unsafe`，本 crate 现在 `#![forbid(unsafe_code)]`。因此 Windows 真机安装路径此刻是缺口，不是「已实现但没测」；`soulcore` 只从 `open_test_store` 走 `TestKeyProvider`，Linux CI 不受影响。补齐 DPAPI 前不得声称 Windows 上 KEK 已受保护。

### 桌面壳启动时用的是哪把钥匙（WP09 第二段，现状）

桌面壳在 `setup` 里向 `soulcore::commands::store::open_store_for_session(app_local_data_dir)` 要**唯一一个**库句柄。那个函数每次都先问 `DpapiKeyProvider`，两个入口都还返回 `Unsupported`，于是**回退到 `TestKeyProvider::in_dir(数据目录)`**：64 字节种子以明文写在 `%LOCALAPPDATA%\Soul\soul-test-keys.bin`，就在 `soul.db` 旁边。

结论要写实：**这台机器上的 KEK 现在没有受 DPAPI 保护**。库文件和解开它的种子在同一个目录里，任何能读到该目录的进程都能打开库；整库加密此刻挡的只是把 `soul.db` 单独拷走的人，不挡把目录整个拷走的人。

这条事实不是注释，是数据：`open_store_for_session` 返回 `KeyProtection::UnprotectedKeyFile`，壳把它塞进 `ConfigSnapshot { kek_protected: false, key_protection }`，设置页逐字渲染 `KEY_FILE_NOT_PROTECTED_EXPLANATION`（`crates/soulcore/src/commands/shell.rs`）。三道锁看着它：核心侧穷举断言没有任何配置能让 `kek_protected` 变 true；`apps/desktop/src/contract.test.ts` 比对界面渲染的常量与 Rust 常量逐字相等，并扫描整棵 UI 树禁止硬编码任何「已受保护」类字样；`apps/desktop/src-tauri/tests/store_session.rs` 断言壳源码里根本不出现 `KeyProvider` 字样——选哪把钥匙是 `soulcore` 的决定，不是界面的。

补齐 DPAPI（P1）时改的是 `open_store_for_session` 的一条分支与 `KeyProtection` 的一个变体，上面三道锁一条都不用动。在那之前，安装说明与 UI 都只能说「把这台电脑本身当成唯一一道门」。

密钥链在库里的落法：DEK 交给 SQLCipher（`PRAGMA key = "x'<64 hex>'"` 原始密钥形式，不走口令 KDF）；KEK 用 XChaCha20-Poly1305 包裹每把 CK，AAD 为 `content-key|<content_key_id>`，所以一把包好的 CK 不能被搬到另一个 `content_key_id` 下解开。CK 只以包裹态存在 `content_keys` 表，遗忘就是删这一行。

不承诺：抵抗本机管理员、物理取证、内核恶意软件；不承诺 SSD 物理擦除。遗忘的可测语义是「CK 已销毁，正文不可解，派生推断降为 orphaned」，UI 必须照此如实写。

## 审计

独立库/表，哈希链。字段白名单见 `docs/schemas/audit.schema.json`。禁止 body/text/content/quote/summary/prompt。

## HITL

未知动作拒绝。`plan_hash` 变化拒绝。能力令牌一次性、限作用域、限 TTL。v0.1 不消费写文件令牌。

## 注入

导入/粘贴/文件名只进数据通道。红线用例 AC-25。

## 非临床

诊断词 denylist 由 CI 断言。输出声明「工作假设，非临床结论」。
