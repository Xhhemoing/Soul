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
- `DpapiKeyProvider`：**已落地**（原为骨架）。见下面「DPAPI 落地」。

密钥链在库里的落法：DEK 交给 SQLCipher（`PRAGMA key = "x'<64 hex>'"` 原始密钥形式，不走口令 KDF）；KEK 用 XChaCha20-Poly1305 包裹每把 CK，AAD 为 `content-key|<content_key_id>`，所以一把包好的 CK 不能被搬到另一个 `content_key_id` 下解开。CK 只以包裹态存在 `content_keys` 表，遗忘就是删这一行。

## DPAPI 落地

`DPAPI → KEK` 这一段现在是真的调用，不是骨架。落在 `%LOCALAPPDATA%\Soul\keys.dpapi`：

| 位置 | 内容 | 谁能解开 |
|---|---|---|
| 文件头 | 8 字节魔数与格式版本 | 任何人（不是秘密） |
| 第一段 | **KEK** 经 `CryptProtectData` 保护后的密文 | 只有当前登录用户在这台机器上 |
| 第二段 | 24 字节 nonce 加 **DEK** 在 KEK 下的 XChaCha20-Poly1305 密文，AAD = `soul/v1/database-dek` | 拿到 KEK 的人 |

于是三段箭头都实在：DPAPI 保护 KEK，KEK 包裹 DEK（`keys.dpapi` 里）与每把 CK（`content_keys` 表里）。DEK 与 KEK 是两把独立的随机密钥，不是一个种子的两次派生——`TestKeyProvider` 才是那种做法，它也只用于测试。

作用范围是**用户级**，不是机器级：不传 `CRYPTPROTECT_LOCAL_MACHINE`，所以同机的另一个账户解不开这个 blob，另一台机器上的同名账户也解不开（除非漫游配置文件与主密钥跟着走）。本机管理员、附在该会话上的调试器、以及任何以该用户身份运行的东西仍然可以——这与本文件开头「不承诺」的那三条一致，DPAPI 不改变它们。`CRYPTPROTECT_UI_FORBIDDEN` 恒开：开库时没有人可以被提示，失败就报失败。

`unsafe` 隔离在 `crates/soul-win-dpapi`：整个 crate 的公开面只有 `protect` / `unprotect` 两个函数，唯一的 `unsafe` 在 `src/sys.rs` 一个函数里（两个 `extern "system"` 声明加一次调用，输出缓冲复制后先擦零再 `LocalFree`），crate 根在 Windows 上是 `deny(unsafe_code)`、其它平台是 `forbid(unsafe_code)`。`soul-store` 因此仍然是 `#![forbid(unsafe_code)]`。这个 crate 在非 Windows 上照常编译并返回 `DpapiError::Unsupported`，所以 `DpapiKeyProvider` 里一个 `#[cfg]` 都没有，Linux 构建照样给 Windows 那条路做类型检查。

`keys.dpapi` 只在首次使用时创建（先写 `.partial` 再 rename），之后**永不重写**。解析不了的 blob 报 `KeyError::Corrupt` 并原样留着：它是这台机器上唯一能打开 `soul.db` 的东西，一个会覆盖它的实现就是一个会把库扔掉的实现。删掉这个文件等于永久失去这个库。

测什么：`crates/soul-win-dpapi/tests/roundtrip.rs`（`cfg(windows)`：往返一致、blob 里搜不到明文密钥、两次保护结果不同、换 entropy 解不开、改一个字节解不开；`cfg(not(windows))`：两个方向都 `Unsupported`）；`crates/soul-store/src/keys.rs` 的单元测试（文件格式往返与九种坏 blob 全拒、DEK 在 KEK 下的包裹与换 KEK/换 AAD 都解不开）；`crates/soul-store/tests/dpapi_key_chain.rs`（开库 → 写一行 → 关库 → 新 provider 重开 → 读回来）。Linux 侧另有 `soulcore/tests/session_commands.rs` 断言这台机器上开库的是种子文件而不是 DPAPI。

不承诺：抵抗本机管理员、物理取证、内核恶意软件；不承诺 SSD 物理擦除。遗忘的可测语义是「CK 已销毁，正文不可解，派生推断降为 orphaned」，UI 必须照此如实写。

## 审计

独立库/表，哈希链。字段白名单见 `docs/schemas/audit.schema.json`。禁止 body/text/content/quote/summary/prompt。

## HITL

未知动作拒绝。`plan_hash` 变化拒绝。能力令牌一次性、限作用域、限 TTL。v0.1 不消费写文件令牌。

## 注入

导入/粘贴/文件名只进数据通道。红线用例 AC-25。

## 非临床

诊断词 denylist 由 CI 断言。输出声明「工作假设，非临床结论」。
