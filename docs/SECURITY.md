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

「销毁 CK」不止是 `DELETE` 掉 `content_keys` 那一行——被删的字节还在两处，而这两处都是 DEK 打得开的文件：页内被释放的空间（行只是进了空闲页），以及删除之前写下的 WAL 帧（它们带着 CK 还在页上时的那一页）。所以现在多做两件事：

- 开库时 `PRAGMA secure_delete = ON`，释放的页内字节被清零而不是留着。SQLCipher 在 codec 挂上时本来就会把它打开，但那是依赖内部的行为，本仓库任何地方都没写过；现在是显式设置并**读回校验**，`SqlCipherStore::open` 读到的不是 1 就直接报错退出。
- `execute_forget` 提交之后立刻 `wal_checkpoint(TRUNCATE)`，把日志折回主库并截到 0 字节。这一步此前没有，日志会一直留着旧页直到应用下次碰巧 flush。
- 被销毁的 CK id 记进 `destroyed_content_keys`（只有裸 UUID，没有第二列；`SECURITY.md` 本来就允许被遗忘对象的 UUID 活在审计链里）。删掉那一行只是把 id 空了出来，而 `ensure_content_key` 对任何自己没有的 id 都会新铸一把——于是下一次以同一个 id 密封会成功，那些已经标成 `forgotten` 的墓碑行背后又有了活密钥。有了这张表，这种密封直接返回 `ContentKeyDestroyed`。`meta.schema_version` 因此升到 2：旧库下次开库自动建表，但**它记不住升级之前发生过的遗忘**，那之前销毁的 id 仍可被重铸。

于是可测语义变成：CK 那一行没了，包裹字节在主库页与 WAL 里都不再是可读的形态，那个 id 也不会再有第二把密钥，正文解不出来。`crates/soul-store/tests/forget.rs` 断言 pragma 读回 1（关库重开后仍是 1，它是连接属性不是文件属性）、`execute_forget` 之后 `-wal` 是 0 字节（去掉 checkpoint 这条测试就红）、以及遗忘之后再拿同一个 CK id 密封被拒且重开库仍然被拒（去掉台账这条测试也红）。**没有**做解密后的逐页扫描：SQLite 唯一能做这件事的接口是 `sqlite_dbpage`，`libsqlite3-sys` 的 bundled 构建只开了 `SQLITE_ENABLE_DBSTAT_VTAB`，而且那张虚表可写，开它等于给任何拿到 DEK 的东西一条改原始页的路；测试里以「这个构建确实没有 `sqlite_dbpage`」的断言记着这件事，哪天有了就该把真扫描补上。同一个测试用明文库做对照，证明这个构建里关掉 pragma 删除的字节确实还在文件里、打开就没了。

仍然不承诺 SSD 物理擦除：盘上更早的物理块可能仍带着旧密文，wear leveling 与 TRIM 不在本文件的承诺范围内。

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

`keys.dpapi` 只在首次使用时创建，之后**永不重写**。「首次使用」这四个字对**同时发生的两次首次运行**也成立：创建走 `OpenOptions::create_new` 直接落最终文件名，只有一个进程建得了这个名字，其余进程收到 `AlreadyExists`，读回赢家的字节、用赢家的 DEK 开库。旧写法是先写 `.partial` 再 rename，两个进程各造一把 DEK，后 rename 的那个把前一个刚建好的库永久锁死——`TestKeyProvider` 的种子文件当时也是同一个形状的洞。被建出来却一个字节都没写进去的空文件是「首次运行中途断电」的唯一残留，它里面没有任何密钥材料，也就不可能有库是用它开的，所以下一次启动把它**就地填上**：拿到这个文件的 OS 独占锁（`fs4`，Windows 是 `LockFileEx`，Unix 是 `flock`），持锁看长度，为零就写、不为零就顺着这把锁读回别人刚写的字节。Soul 在任何情况下都不会删 `keys.dpapi`——先删再 `create_new` 重铸看着更直接，但那是把原来的竞态原样搬了回来：两个恢复者都删、都建，后建的那份才是磁盘上的文件，先建的那个带着一把谁也没有的密钥去开库；更糟的是这一删可能落在第三个进程已经把文件填好之后，删掉的就是某个库正在用的密钥。改读也走持锁的那个句柄，不再另开一次路径：Windows 锁的是字节区间，不持锁去读别人锁住的区间会直接失败，而 Unix 的 `flock` 只是劝告锁，另开一次读有可能读到写了一半的文件。解析不了的 blob 报 `KeyError::Corrupt` 并原样留着：它是这台机器上唯一能打开 `soul.db` 的东西，一个会覆盖它的实现就是一个会把库扔掉的实现。删掉这个文件等于永久失去这个库。

测什么：`crates/soul-win-dpapi/tests/roundtrip.rs`（`cfg(windows)`：往返一致、blob 里搜不到明文密钥、两次保护结果不同、换 entropy 解不开、改一个字节解不开；`cfg(not(windows))`：两个方向都 `Unsupported`）；`crates/soul-store/src/keys.rs` 的单元测试（文件格式往返与九种坏 blob 全拒、DEK 在 KEK 下的包裹与换 KEK/换 AAD 都解不开；八个「首次运行」抢一个不存在的名字，以及八个恢复者同时抢一个被遗弃的空文件——四轮，每轮所有人拿到的都必须是磁盘上那一份，目录里也只许剩这一个文件）；`crates/soul-store/tests/dpapi_key_chain.rs`（开库 → 写一行 → 关库 → 新 provider 重开 → 读回来）；`crates/soul-store/tests/key_blob_race.rs`（六次「首次运行」同时开库、各写一行，四轮；散场后只用幸存的密钥文件重开，六行都要读得回来）。抢文件名那一步没有平台，所以 `keys.rs` 里的抢锁单元测试与这个竞态测试在 Linux CI 上照跑，`windows-latest` 上跑的是同一份代码经由真 DPAPI 的那一路。Linux 侧另有 `soulcore/tests/session_commands.rs` 断言这台机器上开库的是种子文件而不是 DPAPI。

不承诺：抵抗本机管理员、物理取证、内核恶意软件；不承诺 SSD 物理擦除。遗忘的可测语义是「CK 已销毁——那一行没了，包裹字节在主库页与 WAL 里也不再可读（见「密钥与遗忘」）——正文不可解，派生推断降为 orphaned」，UI 必须照此如实写。

## 审计

独立库/表，哈希链。字段白名单见 `docs/schemas/audit.schema.json`。禁止 body/text/content/quote/summary/prompt。

## HITL

未知动作拒绝。`plan_hash` 变化拒绝。能力令牌一次性、限作用域、限 TTL。v0.1 不消费写文件令牌。

## 注入

导入/粘贴/文件名只进数据通道。红线用例 AC-25。

## 非临床

诊断词 denylist 由 CI 断言。输出声明「工作假设，非临床结论」。
