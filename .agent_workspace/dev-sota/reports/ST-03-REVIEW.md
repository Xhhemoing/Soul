MODEL: claude-fable-5-thinking-xhigh

# ST-03 合入代码只读审查

**审查版本**：`2920d2f7776c46a846c69c9d972d57fe8cf11acf`（feat: register draft/fileplan crates and wire the encrypted store）。
**审查对象**：`crates/soulcore/src/commands/store.rs`（`open_store_for_session`）、`crates/soulcore/src/commands/shell.rs`（`authorize_root`、`KeyProtection`、`ConfigSnapshot`）、`apps/desktop/src-tauri/src/lib.rs`（`install_store`）、`apps/desktop/src-tauri/src/commands.rs`、`apps/desktop/src/routes/Settings.tsx`，以及为它们作保的测试（`store_session.rs`、`command_surface.rs`、`ipc_roundtrip.rs`、`shell_commands.rs`、`contract.test.ts`、`Settings.test.tsx`）。
**方式**：只读。未改代码，未跑测试（ST-03 报告已记录全绿；本文以源码为准）。工作树里 ST-01/ST-02 的未合并改动一概未碰。

结论先行：**没有发现今天就会出错的运行期缺陷**；找到 **两处会在 DPAPI 落地当天变成真问题的潜伏缺陷**（§1 的 `Err(_)`、§3 的文案—错误耦合断裂）、**一处守卫缺口**（§2：`manage` 那一半没有测试钉住）、**一处防线弱于报告声称**（§5：诚实文案对"前端两文件 PR"并不免疫），外加三条顺带发现（§6）。

---

## 1. `Err(_)` 吞掉非 `Unsupported` 错误 —— 缺陷确认（潜伏，今天无症状）

`store.rs:122-131`：

```rust
match platform.key_encryption_key() {
    Ok(_) => Ok(SessionStore { handle: share(open_store(directory, &platform)?), ... }),
    Err(_) => Ok(SessionStore { handle: share(open_store_with_test_file_keys(directory)?), ... }),
}
```

`KeyError` 有四个变体：`Unavailable` / `Unsupported` / `Io` / `Malformed`（`keys.rs:30-48`）。函数文档写的是"It refuses today — both accessors return `KeyError::Unsupported`"，即**回退的正当性建立在错误必然是 Unsupported 之上**，但 match 臂没有把这个前提写进代码。今天 `DpapiKeyProvider::unprotect` 无条件返回 `Unsupported`，所以行为无差别；DPAPI 落地那天，差别立刻出现：

1. **blob 损坏（`Malformed`）、blob 不可读（`Io`）、DPAPI 暂时不可用（`Unavailable`）都会静默降级为明文密钥文件**，而不是 fail closed。对新目录，这是用户从未同意过的安全降级（设置页会诚实地说"明文文件"，但诚实标注不等于征得同意）；对已有 DPAPI 加密库的目录，回退钥匙打不开旧库，`SqlCipherStore::open` 的"did not open under this key"错误会向上传播、应用拒启——但**在失败之前，明文 seed 已经落盘**：`open_store_with_test_file_keys` → `SqlCipherStore::open` → `keys.database_key()` → `read_or_create_key_file`（`keys.rs:187-194`）在 PRAGMA key 校验之前就把 64 字节新 seed 写进了 `soul-test-keys.bin`。这个文件既是残留物，也会在下次启动时掩盖"DPAPI blob 出过事"这个事实。ST-03-KEY-PROBE.md 第 15 行已指出前半段，此处补充：**残留 seed 会让后续每次启动都走进同一个死胡同且不留线索**。
2. **回退策略不对称**：探针（`key_encryption_key`）的任意错误回退，探针之后 `open_store(directory, &platform)?` 内部再次调用 `database_key()` / `key_encryption_key()`（`soul-store/src/store.rs:108-109`）的错误却用 `?` 直接传播。哪些错误回退取决于错误**发生在哪一步**而不是**是什么错误**——这不是一条策略，是两条。
3. **探针结果被丢弃 + 双读 TOCTOU**：`Ok(_)` 拿到真 KEK 又扔掉（zeroize on drop，无泄漏），开库时再读一遍 blob。两读之间 blob 可变。今天无害，DPAPI 落地后是竞态窗口。
4. **不可测**：`open_store_for_session` 在函数体内自行构造 `DpapiKeyProvider`，没有注入缝。即使现在想把 match 改成只对 `Err(KeyError::Unsupported(_))` 回退，也写不出"探针返回 `Io` 时不落明文 seed"的测试——除非给函数加 provider 参数或改造 `DpapiKeyProvider` 可注入失败。这是回退策略至今没被测试钉住的结构性原因。

**建议（P1 修，DPAPI 之前必须修）**：回退只匹配 `Err(KeyError::Unsupported(_))`，其余变体传播；为此给 open 逻辑留注入缝并补"非 Unsupported 探针错误 → 报错且目录里零新文件"的测试。ST-03 报告 §4 的缺口表**没有列这一条**，应补记。

## 2. Session/store 句柄无人消费 —— 事实确认，且"manage 存在"本身没有测试

`lib.rs:66` `app.manage(session.into_handle())` 之后，`Arc<Mutex<SqlCipherStore>>` 在整个代码库里**零读者**：没有任何 `#[tauri::command]` 收 `State<Arc<Mutex<SqlCipherStore>>>`，tray 与前端也不碰它。ST-03 报告 §4 已如实记录"本轮不接触库命令、S-01.2 运行期取证缺失"，这个取舍本身成立。但有四点报告没说：

1. **`manage` 这一半没有守卫**。`store_session.rs` 断言的是"`open_store` 恰好一次、在 lib.rs、`fn install_store` 存在"；`configure_opens_nothing_and_setup_does` 断言 run 体里有 `install_store(app)`。**没有任何测试断言句柄被 `manage` 了**——把 `app.manage(session.into_handle())` 整行删掉（开库、记录 KeyProtection、然后立刻 drop 连接），全部测试照绿。S-01.1 里"`.manage(` 管理的是 `Arc<Mutex<SqlCipherStore>>`"这半句验收条实际上没有落成断言。第一条触库命令落地前，这是一扇没锁的门。
2. **零消费的句柄换来了一个真实代价**：库打不开 → `setup` 报错 → `run()` 的 `.expect("Soul could not start")` panic。v0.1 没有任何功能需要这个库，但磁盘满、目录权限坏、seed 文件被裁剪成错误长度（`Malformed`）都会让用户连设置页和向导都见不到，且没有任何 UI 能显示原因。报告把"库打不开就起不来"记为有意取舍，取舍可以接受，但值得指出它保护的是一个此刻没人用的资源。
3. **裸类型 manage + 返回值丢弃**：`manage` 以类型为键，这里用的是裸 `Arc<Mutex<SqlCipherStore>>` 而不是像 `SessionConfig`、`TrayState` 那样的 newtype；`manage` 返回的 bool（已存在同类型时为 false 且静默不覆盖）被丢弃。今天只有一个调用点，但将来第二个 manage 同类型的人会静默拿不到自己的句柄。
4. **`SessionStore` 被就地拆散**：`key_protection` 进 `Session`、句柄单独 manage，一个事实拆成两份托管状态。等第一条触库命令出现，"句柄"与"这把钥匙怎么来的"就可能被分别取用、分别演化。直接 manage `SessionStore` 本身（或一个包着它的 newtype）会让两者不可分离。`SessionStore::handle()`（clone 入口）目前仅 soulcore 测试使用，生产零调用——不算缺陷，记录在案。

## 3. `kek_protected` 字段 —— 无消费者的第二份真相，且文案与错误语义存在耦合断裂

1. **前端零消费**。`kek_protected` 在 `core.ts:40` 声明、`fakeCore.ts:55` 赋值，之后没有任何组件读它——设置页只渲染 `snapshot.key_protection` 字符串。S-03.1 要的是"快照带布尔 + Rust 测试穷举"，这条满足了（`no_configuration_can_claim_the_key_is_protected` 16×2 穷举 + `the_protection_flag_follows_the_provider_that_answered` 反向）；但落地结果是**线上多了一个专为测试存在的字段**。风险不在今天，在它被启用的那天：一旦某个前端 PR 开始对 `kek_protected` 分支渲染自己的措辞，就绕开了"整句来自核心"的纪律，而 §5 会说明禁词表接不住这种绕开。建议要么给它找一个消费者（例如设置页据此控制一个仅样式的警示态，文字仍用核心句子），要么在注释里明说"此字段只许用于样式分支，禁止据此生成文字"。
2. **布尔语义合并了两种不同状态**：`NoStoreOpened` 和 `UnprotectedKeyFile` 的 `kek_protected()` 都是 false（`shell.rs:101-103`）。"还没有钥匙"和"钥匙明文躺在盘上"对布尔消费者不可区分，只有字符串能分。字段今天无人读所以无害，但这是给未来消费者埋的一颗语义地雷——若要保留布尔，三态更诚实。
3. **文案与错误变体的耦合断裂（接 §1）**：`KEY_FILE_NOT_PROTECTED_EXPLANATION` 的原文是"没有交给 Windows 的 DPAPI —— **那一段还没有实现**"。这句话为真的前提是回退原因必然是 `Unsupported`。因为 §1 的 `Err(_)`，DPAPI 实现之后若因 blob 损坏回退，设置页仍会告诉用户"那一段还没有实现"——一句为诚实而生的文案会开始说谎。修 §1 的 match 臂时，这句常量与 `UnprotectedKeyFile` 变体的对应关系要一并重审。
4. 双字段（bool + String）由 `of_session` 从同一个 `KeyProtection` 派生（`shell.rs:162-163`），当前不可能互相矛盾；穷举测试同时断言了两者。无缺陷，仅记录冗余是受控的。

## 4. IPC wrapper 是否仍 ≤1 语句 —— 是，守卫是行数近似而非语句计数

`commands.rs` 五个 wrapper（`config_snapshot` / `complete_wizard` / `cloud_toggle` / `authorize_root` / `authorized_roots`）的函数体各是**一条转调表达式** `session.0.xxx(...)`，逐一核对属实；没有取锁、没有错误映射、没有分支。S-04 对"新增 store 命令"的要求因本轮没有新增 store 命令而空满足。

守卫本身的两点弱化，记录备查（不构成本轮缺陷）：

- `the_command_layer_stays_thin`（`command_surface.rs:94-113`）数的是首个 `{` 与首个 `}` 之间的**非空非注释行数**，不是语句数。`let g = session.0.lock(); g.xxx()` 挤在一行照样计 1；反过来，一条合法语句被 rustfmt 折行会误红。前者是可钻的缝，后者是误伤，两者都源于同一处近似。
- 首个 `}` 的启发式在函数体含嵌套块（match、闭包）时会提前截断——今天没有这种函数体，但这意味着这条测试对"长出分支的 wrapper"的报错行数可能失真。它仍会红（嵌套块自身多行），只是理由报得不准。

三方名单咬合（`COMMAND_NAMES` / `generate_handler!` / `core.ts COMMANDS`）由 `command_surface.rs` 三条测试 + `contract.test.ts` 的命令名比对双侧钉住，`authorize_root` / `authorized_roots` 两条新命令都在三处，核对无误。

## 5. 诚实文案是否可被前端改掉 —— 生产数据流改不掉；**前端两文件 PR 可以让屏幕不再显示它，且现有三道锁接不住**

先说改不掉的部分：生产链路是 `shell.rs` 常量 → `Session::snapshot()` → IPC → `Settings.tsx:115` 直接渲染 `{snapshot.key_protection}`，中间无变换。WebView 改不了核心送来的**数据**。

能改掉的是**呈现**。逐道锁检验（攻击模型：一个只动 `apps/desktop/src` 的前端 PR）：

| 锁 | 实际锁住的东西 | 能否拦住"屏幕换句好听的" |
|---|---|---|
| Rust 穷举（`shell_commands.rs`） | 核心算出的值 | 拦不住——前端 PR 不碰 Rust，它照绿 |
| `contract.test.ts` 逐字比对 | **fakeCore 常量 ↔ shell.rs 常量** | 拦不住——它比的是测试替身和核心，不是屏幕和核心 |
| `Settings.test.tsx`（"一个字都没改"） | 屏幕 ↔ fakeCore 常量 | **拦得住，但它自己就是前端文件**：同一个 PR 把 `Settings.tsx` 和 `Settings.test.tsx` 一起改，这道锁随手就拆 |
| 禁词扫描（`contract.test.ts:142-155`） | 六个硬编码子串：`DPAPI`、`已受保护`、`已加密保护`、`受到保护`、`已经保护`、`安全保管` | **接不住**——"密钥由系统安全托管"、"密钥已妥善加密存放"、"无需担心密钥安全" 一个都不命中 |

也就是说：**唯一把"屏幕上是那句话"钉死的测试，与被它监督的组件同属一个评审面**。这正是 S-02 注释里"密钥策略写在 UI 面，改 UI 的人就能改密钥策略"要防的结构，只是这次轮到了文案。对比之下，仓库里已有跨语言反向锚的先例——`store_session.rs:176-191` 在 **Rust 测试**里读 `core.ts` 断言命令名——同样手法完全可以用在这里。

**建议**：在 `apps/desktop/src-tauri/tests`（Rust 侧，前端 PR 动它必然触发数据面评审）加一条源码级断言：`Settings.tsx` 的密钥区块必须渲染 `snapshot.key_protection`，且该区块内不得出现其他中文字符串字面量。另外禁词表可以补"托管""妥善""放心""无需担心"一类安抚词，但要认识到黑名单永远追不上措辞，结构性的跨面锚才是对症的。

顺带两点：`contract.test.ts` 只锁了 `KEY_FILE_NOT_PROTECTED_EXPLANATION` 一句；`NO_STORE_OPENED_EXPLANATION`（mock runtime 会真实渲染）没有 TS 侧比对——低危，因为它不作任何保护声称。`rustStringConstant` 的正则若遇到常量改写成 `concat!` 会直接 throw，失败朝向正确（fail closed）。

## 6. 顺带发现（均不在五个指定项内，但属真实问题）

1. **Windows 上 `canonicalize` 的 `\\?\` 前缀会原样出现在用户面前**。`shell.rs:312` 的 `std::fs::canonicalize` 在 Windows 返回 verbatim 路径（`\\?\C:\Users\...`）；`authorized_roots` 按 `display()` 回显（`shell.rs:334-340`），设置页列表和"这个目录已经授权过了：\\?\C:\..."的拒绝语都会带前缀。占位符示例写的是 `C:\Users\你\Documents`，回显却长出 `\\?\`——在唯一的发货平台上是真实的观感缺陷，且 Windows 未真机验证（报告已列手动清单）。去重逻辑本身自洽（比较双方都规范化），只是显示层难看。通常解法是 dunce 一类的去前缀，归 P1。
2. **`authorize_root` 在同步 Tauri 命令里做阻塞文件系统 IO，且持有 Session 锁**。非 async 的 Tauri 命令默认跑在主线程；`metadata` / `canonicalize` 对不可达的网络路径（Windows 上填 `\\dead-server\share` 是用户会真干的事）可能阻塞数十秒，期间整个 UI 冻结，且 `Session` 的 Mutex 被一并占住、所有其他命令排队。改 `#[tauri::command(async)]` 或把文件系统探测挪出锁外即可，v0.1 影响小，第一批 Windows 用户填网络盘时会撞上。
3. **`Settings.tsx` 的授权成功路径有一个小的不一致窗口**：`authorize()` 先 `onSnapshot(await authorizeRoot(draft))` 再 `setRoots(await authorizedRoots())`（`Settings.tsx:55-56`）。若第二个调用失败，计数徽章已 +1、列表却停在旧值、错误栏显示的是列表读取失败的话。概率极低（同一 IPC 通道连续两调），记录在案即可。
4. `store_session.rs` 的一切断言都是文本级：`occurrences("open_store")` 靠子串匹配（实际命中的是 `open_store_for_session` 里的子串），注释里出现同名字样即可满足或触发。这类源码扫描防的是疏忽不是恶意，与仓库既有先例一致，不算缺陷，但评审 lib.rs 改动时不能只信这组测试。

---

## 判定汇总

| 指定审查项 | 判定 | 严重度 |
|---|---|---|
| `Err(_)` 吞掉非 Unsupported | **缺陷成立**（潜伏）：任意探针错误静默降级 + 校验前明文 seed 落盘 + 回退策略前后不对称 + 无注入缝不可测。今天无症状，DPAPI 落地即触发 | P1 前必修 |
| Session/store 句柄无人消费 | 事实成立且已如实报告；**新发现**：`manage` 那一半没有任何测试钉住，删掉整行照绿；裸类型 manage、bool 丢弃、`SessionStore` 被拆散三处结构隐患 | 第一条触库命令前补 |
| `kek_protected` 字段 | 无运行期缺陷；前端零消费、布尔合并两种状态、文案与 `Unsupported` 的耦合未写进类型——三处待第一位消费者出现即成风险 | 记录，随 §1 一并处理 |
| IPC wrapper ≤1 语句 | **满足**，五个 wrapper 均为单表达式转调；守卫是行数近似，单行多语句可钻 | 无需动作 |
| 诚实文案可否被前端改掉 | 数据流改不掉；**呈现可以**：动 `Settings.tsx` + `Settings.test.tsx` 两个前端文件即可换成安抚句而全部剩余测试照绿，禁词表六个子串接不住同义改写。屏幕↔常量的锚应移入 Rust 侧测试 | 建议本批修 |

未改任何代码，未 commit。
