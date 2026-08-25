# fable-closeout — BUILD/AUDIT R2 收口复核（READONLY）

- model: `claude-fable-5-thinking-xhigh`（未降级）
- round: BUILD/AUDIT R2 close-out
- role: review（独立复核；本文件是唯一落笔）
- 基线: 唯一主干 `cursor/soul-goal1-7b1c` @ `5309656`；复核对象分支 `cursor/goal1-build-audit-c441`，尖端 **`e9cf29b`**
- 已读: `R2-SYNTHESIS.md`、`round2/fable-a.md`、`round2/fable-b.md`、`round2/gpt-sol-a.md`、`round2/gpt-sol-b.md`、`PROGRESS.md`、分支全量 diff `5309656..e9cf29b`
- 方法: 每一条「已闭合」都在 HEAD 的代码里逐行核过，并在本机重跑钉住它的测试；不凭提交信息记账

## 一句话

R2 报出的**代码缺陷全部在 HEAD 闭合，无一误报**；本机针对性测试全绿（xtask 33、soulcore 62、store/import/fileplan 18、vitest 166、ipc_roundtrip 51）。残留的只有两句向导文案（gpt-sol-a 余项，STATUS 已如实记账）、一处测试基建视差（FakeStore `keys_of`），以及三件从来不是代码任务的 blocked-on-user P0。**本主干上没有任何未闭合的代码 P0。**

## 逐条判定

| # | 缺陷 | 修复提交 | 判定 | 核到的证据 |
|---|---|---|---|---|
| 1 | P0 Files 空态谎称「读不到任何文件」 | `1f52ca5` | **闭合** | `Files.tsx:129` 现只承诺目录扫描器，点名两个例外（导入页自选文件、自身配置与库）；`Files.test.tsx:55-62` 钉新句并断言旧句不在；旧句在产品源码里已绝迹（仅余测试负断言与文档引述） |
| 2 | P0 起草确认屏不描述真实请求体 | `24ea578` | **闭合** | `draft.rs:91-96` `E1_PLAN_NOTICE` 点名真正进 JSON 的四样（模型名、固定系统指令、档案摘要含口吻/来源/要点/非临床句、占位或单次豁免后的粘贴），并写明段数/哈希/准备号**不**发出；`Draft.tsx:235` 渲染 `plan.notice`（常量在 Rust 侧，屏幕软化不了）；`draft_commands` 18 绿 |
| 3 | P1-1 换端点旧计划仍发出（origin 绑定 + pending 丢弃） | `d1b6457` + `b2d2915` | **闭合**（fable-b 要求的两半都在） | `policy.rs:245-253` `e1_plan` 把 `target` origin 打进哈希（地址本身不上屏不进审计）；`draft.rs:288-299` `generate` 用**当前**配置重导哈希、在铸 token 之前拒绝，新旧两个地址都听不到；`session.rs:874,890` `set_user_endpoint`/`clear_user_endpoint` 都 `draft.discard()`，解析失败则保留（正确：手误不该罚掉在读的计划）。钉住它的测试正是 gpt-sol-b 开的方子：`an_endpoint_saved_after_the_plan_voids_the_approval_and_neither_address_hears_it`（A/B mock 请求数均 0）、`clearing…voids…too`、`saving_another_endpoint_drops_the_prepared_body…`、`re_pointing_the_endpoint_voids_a_token…`、IPC 层 `an_endpoint_saved_after_the_plan_refuses_the_stale_approval_over_the_ipc` — 全绿 |
| 4 | P1-2 无 display_label 联系人遗忘空转 | `e2f7368` + `3ccf9d6` | **闭合**（真库；fake 视差另记） | `commit.rs:416-433` 无标签时写 `content_key_anchor` 锚 blob（`row_id`=联系人行）；`forget.rs:83-91` `content_keys_of(Contact)` UNION 标签键与 `sealed_blobs WHERE row_id=联系人`；`forget.rs:115-124` 反向路径让该行落成墓碑；`sql.rs` 补 `sealed_blobs_by_row` 索引。重复导入每次铸的新键各带各的锚，遗忘一次全收。`forget_after_import` 端到端证明：预览=收据、关库重开后 `ContentKeyDestroyed`、别人的正文原样可读、行成墓碑；`3ccf9d6` 再钉边上推断失据。全绿 |
| 5 | P1-4 快照无视 `max_entries` | `8c635b2` | **闭合** | `scan.rs:184-197` 快照走查两处检查 `max_entries`；截断进哈希（`:246-248`，半个目录永远哈希不等于整个）；`scan()` 把 before/after 截断并进 `DirectoryScan::truncated`（`:493`）；`scan_is_bounded` 6 绿 |
| 6 | gpt-sol-b P1-3 e0-audit `starts_with` 前缀放行 | `1a4cc22` | **闭合** | `egress.rs` `is_allowed_url` 解析 scheme/authority、拒绝 `@` 凭据形、整 host 大小写不敏感相等、端口自由、`soul.local/schemas/` 按路径子树；负例测试盖 `localhost.attacker.invalid`、`127.0.0.1.attacker.invalid`、**`127.0.0.10`**、`localhostess.example`、`localhost@evil.invalid`、`soul.local/collect`；xtask 33 绿含全库 e0 复审 |
| 7a | 向导「已授权目录」两句主语过宽 | `d32a786` | **闭合** | `Wizard.tsx:79-93` 收窄到目录扫描器，点名两个例外与「读写自己的配置与数据库」 |
| 7b | 导入预览「什么都没有写进库里」 | `6412049` | **闭合** | `Import.tsx:231-237` 改说「这个文件里的人、会话、消息一条都没写」；带注入标记时明说审计链上已留一行、换文件也还在 |
| 7c | 重复导入无声翻倍 | `c8cebce` | **闭合**（按 D55 的诚实警告路线，不做索引） | 预览 `preview-reimport` 与收据 `receipt-reimport` 都写明：人不重复、往来记录会再写一遍、图上次数与强度会涨 |
| 7d | 遗忘说明「不写任何文件」 | `7cb9d24` | **闭合** | `memory.rs` `FORGET_NOTICE` 改为「不动数据目录以外的任何文件，但 Soul 自己的加密库要写（删键行密文行、落墓碑失据与一条审计、库文件与日志都变）」；SSD 半句原样保住；作者清单同步改判据 |
| 7e | 图摘要 payload 只说「往来次数」 | `71eaaac` | **闭合** | `Graph.tsx:139` 点名整组统计（次数、天数与会话数、双方各发多少、有无一对一、最近日期、关系档位、固定改写要求），仍写明不带姓名不带原话、中间不再问 |
| 8 | P2-5 错的确认吃掉 pending 预览 | `7a29ca7` | **闭合** | `session.rs:1153-1156` `take_if` 先匹配再取；`held_forget` 文档改为「refused 时原样留下」，代码与合同一致；`a_forget_refused_for_the_wrong_id_leaves_the_preview_the_user_read_standing` 绿 |

**误报：0。** 每条在主干 `5309656` 上都真实存在（多名审计者独立命中 1/3/6），闭合都是真闭合。

fable-b 判干净的三件（dpapi unsafe、collect FFI、HITL 令牌）本轮无人改动，维持干净。

## 本机测试（HEAD `e9cf29b` 树上重跑，非转述）

`xtask self_test` 33；`soul-store forget` 4 + `conformance_real` 4；`soul-store-api fake_conformance` 3；`soul-import forget_after_import` 1；`soul-fileplan scan_is_bounded` 6；`soulcore draft_commands` 18 / `policy_commands` 7 / `session_e1` 27 / `session_screens` 10；桌面 vitest 14 文件 166；`ipc_roundtrip` 51。**全绿。本机绿不是 hosted 绿（AC-26 仍空 runner）。**

## 剩余 P0

**代码 P0：无。** 审计意义上的收口条件满足。

非代码、blocked-on-user 的三件维持 open（与 fable-a 的 N4/N5/N6 一字不差，不复述修法）：

1. **N4** hosted 五门在 HEAD 真跑（AC-26，GitHub Billing → dispatch）；
2. **N5** 作者 Win11 手动清单（AC-01、AC-21/22 肉眼半）；
3. **N6** 集成线裁决（归 PR #7；本主干禁止重实现 T4D/A0）。

任何一件没回来，Goal 1 不能关、LOOP20 保持 QUEUED。

## 残留 P1 / P2

| 级 | 项 | 状态 |
|---|---|---|
| P1（文案） | 向导欢迎段「只有你以后自己填写的模型端点例外，发出去的内容会先占位」（`Wizard.tsx:168`）——把第三人默认过度概括成全部 E1 内容：档案摘要与图统计按原样走，豁免那一条也按原文走 | **残留**。gpt-sol-a 余项，opus-e 批未动；STATUS `aaf1ff2` 已如实记账。修法：一句话收窄主语（第三人正文才占位）+ Wizard.test 钉住。半个 opus-fast 的量 |
| P1（流程，非本主干代码） | gpt-sol-b P1-2：AC-21 自动化观察的是 `soul-headless` 不是装出来的 `soul.exe` | **残留（有意）**。R2 裁决：Linux 轮次不落地，归真机/Windows 产品烟（N5 属地），已记 STATUS，不 empty-commit |
| P2（文案） | 向导「要用生成能力，得你自己填一个兼容 OpenAI 的地址」（`Wizard.tsx:76`）——本机确定性起草不用端点 | 残留，停车 |
| P2 | fable-b P2-1 单 mutex + 120s egress 超时挡撤回；P2-2 目录换符号链接 TOCTOU；P2-3 NTFS $UpCase 大小写折叠；P2-4 首启键 blob 竞态（`keys.rs` 本轮零改动，核实仍开）；gpt-sol-b P2-1 E0 审按客户端名枚举、P2-2 IPv6 origin 序列化丢括号（fail-closed）、P2-3 netwatch 采样窗 | 全部残留，按 R2 综合停车决定维持；本轮无人碰这些路径，判定不变 |

## FakeStore `keys_of` 视差：值不值一单 opus

**值，作为一单 P2 测试基建 opus-fast；不挡 Goal 1。**

- **事实**：`soul-store-api/src/fake.rs:96-122` `keys_of(Contact)` 仍只收 display-label 键，`SealedBlob`（`:31-35`）连 `row_id` 都不存，锚路径在 fake 里**无法表达**——fake 至今建模的是 P1-2 修掉之前的空转语义。`contacts_under`（`:138-149`）同病。合同缝在 `conformance.rs`：`run_conformance` 只有 `ForgetUnit::Memory` 的遗忘检查，没有 `Contact` 行，所以两个后端今天**合法地**各说各话。
- **为什么不挡关闭**：fake 不在任何产品路径上——`soul-headless` 用的是 `SqlCipherStore`（`headless.rs:50`），桌面壳也是；真库的行为已被 `forget_after_import` 钉死。
- **为什么仍值得做**：下一个针对联系人遗忘、写在 fake 上的测试（soul-memory / soul-profile / soulcore 的大半测试都跑 fake）会安静地继承旧语义；一个断言「收据为零」的测试在 fake 上绿、在真库上是谎。遗忘是本产品最信任敏感的一条链，它的联系人语义应该进 conformance，与 memory 语义同级。
- **单子的形状**（路径最小，全在测试基建，不碰产品文件与 schema）：①`fake.rs` `SealedBlob` 记 `row_id`；②`keys_of(Contact)`/`contacts_under` 走锚路径；③`conformance.rs` 加一条「遗忘无标签联系人销毁行锚键、正文打不开、行成墓碑」，`fake_conformance.rs` 与 `conformance_real.rs` 两个既有跑器自动双向钉死。一单 opus-fast 的量。

## 输出合同

- **round**: BUILD/AUDIT R2 close-out
- **role**: review（fable-closeout，只读产品文件）
- **scope**: 核 `5309656..e9cf29b` 全部「已闭合」声明
- **done**: 上表 12 条判定（11 闭合、0 误报、判定间发现 1 条新 P2 视差）；本机 9 个 Rust 套件 + vitest + ipc 全绿
- **open**: N4/N5/N6（blocked-on-user）；向导两句（1 P1 文案 + 1 P2 文案）；FakeStore 视差（P2 基建）；R2 停车的 P2 群
- **tests**: 只跑未改；未放宽任何断言
- **assumptions**: hosted 未跑，本机绿只作本机绿记账；PR #7 线上的 AC-28+ 不作本主干事实
- **do_not_touch**: 未动任何产品文件；未合 PR #4/#7/#10；未启 Goal 2；无 empty-commit；本文件是唯一落笔
- **next**: ①请求用户办 N4/N5/N6；②可选一单 opus-fast 收向导「先占位」句（P1 文案）；③可选一单 opus-fast 做 FakeStore/conformance 视差（P2）；④其余 P2 维持停车
