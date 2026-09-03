# Goal 1 实现规划

来源：fable planner `claude-fable-5-thinking-xhigh`（bc-95047a81-e209-50af-858a-ebe34b721a8c）。
父代理采纳。工作包只减不增。Goal 2 不要启动。文档 PR `#1` 不夹带应用代码；本文件随 Goal 1 分支落地。

## 钉死

- 目标运行时：Windows 11 x64 Tauri 2。Linux 只做 headless / fake CI。
- `soul-schema` 无 IO。`soul-store` 是唯一落盘业务数据的 crate。`soul-egress` 是唯一含 HTTP client 的 crate。`soul-policy` 是唯一能签发 `EgressPermit` 与能力令牌的 crate。`soulcore` 只编排。`apps/desktop` 不含业务逻辑。
- 加密首选 `rusqlite` + `bundled-sqlcipher-vendored-openssl`。WP01 必须用「开加密库→写→重开验密」冒烟在 ubuntu 与 windows-latest 证明。修不动则回退纯 SQLite + 字段级 AEAD，并在同一 PR 改 `docs/SECURITY.md`。字段级 XChaCha20-Poly1305（AAD=行 id+字段名）两条路都不可省。遗忘=销毁 CK。
- D31：WP11 只读计划预览留在 Goal 1；写执行是 v0.1.1（AC-27）。
- 空实现不能混过：AC-10 / 15 / 18 / 19 / 20 / 25。

## DAG

```
WP01 ──┬──▶ WP02 ──┬──────────────▶ WP07
       ├──▶ WP08 ──┤ ├──▶ WP10
       ├──▶ WP03 ──┼─┤
       ├──▶ WP04 ──┤ └──(WP08)──▶ WP11
       ├──▶ WP05 ──┼──▶ WP10
       ├──▶ WP06 ──┘
       └──▶ WP09(壳) ──▶ WP09(功能视图) ──▶ WP13
```

WP01 独占先行。其后 WP02–WP06、WP08 面向 `soul-store-api` 可并行（每批最多 2 名 opus；数据/权限面与 UI 面分人）。WP07 依赖 WP02+WP08。WP10/WP11 依赖 WP08。WP13 收尾。

## 批次

| 批 | 人 | 包 | 面 |
|---|---|---|---|
| 1 | 1 | WP01 | 基建 |
| 2 | 2 | WP02；WP08 | 数据；权限 |
| 3 | 2 | WP03+WP04；WP05+WP06 | 数据；数据 |
| 4 | 2 | WP07；WP09 壳 | 数据；UI |
| 5 | 2 | WP10；WP11 | 数据/E1；权限 |
| 6 | 2 | WP09 功能视图；WP13 | UI；基建 |

## 仓库布局（目标态）

见 planner 原文。关键 crate：`soul-schema`、`soul-store-api`、`soul-testkit`、`xtask`、`soulcore`（WP01）；`soul-store`（WP02）；`soul-profile` / `soul-memory` / `soul-graph` / `soul-import` / `soul-collect` / `soul-policy` / `soul-egress` / `soul-draft` / `soul-fileplan`；`apps/desktop`（WP09）。

## WP01 完成定义

1. [x] `just ci` 本地绿（AC-26 骨架）。~~GitHub Actions `lint` / `test-linux` / `test-windows` 随本分支推送~~ D50 之后无 hosted CI：Linux 走 `just ci-full`（G-L），Windows 走 `scripts/gate-win.ps1`（G-W）。Windows SQLCipher 在 `2e72ddf` 的 windows-latest 上绿过一次，HEAD 上要等首份 G-W。
2. [x] `schema_wiring` 证明九份 `$ref` 接到 `_defs`；`docs/schemas/schemas.lock.json` 已钉。
3. [x] crash harness demo 绿（子进程真死）；leakage 检查器对 Unicode fixture 全过。
4. [x] `SECURITY.md` 加密落地小节与 CI 实证一致。
5. [x] xtask `self_test` 绿（断言器非空转）。

WP01 已完成，取舍与遗留见 `docs/STATUS.md`。

## WP02 完成定义

1. [x] `crates/soul-store` 的 `SqlCipherStore` 跑通 `soul_store_api::conformance::run_conformance`，与 `FakeStore` 同一套。
2. [x] SQLCipher 整库 + 字段级 XChaCha20-Poly1305（AAD=行 id+字段名），CK 由 KEK 包裹存表；`KeyProvider` 抽象落地，`TestKeyProvider` 在 Linux 可跑，`DpapiKeyProvider` 是骨架（见 STATUS 取舍 3）。
3. [x] AC-15：影响面预览为真查询；执行后关库重开，CK 销毁、正文不可解、推断 orphaned、审计链仍通过；`FORGET_CK_DELETE_MID` 崩溃后重启不留半毁状态。
4. [x] AC-20：研究预览行来自真聚合，第三人候选行查到后排除，`written_to_disk=false`，不写任何文件（运行期与源码级双查）。
5. [x] AC-24 存储侧：`STORE_EVENT_COMMIT_MID` 子进程真死，重开后哈希链通过、最多丢 1 条。
6. [x] AC-04 存储侧：库文件（含 `-wal`/`-shm`）字节里搜不到已知明文。

WP02 已完成，取舍与遗留见 `docs/STATUS.md`。

## WP08 完成定义

1. [x] `soul-policy` 签发 `EgressPermit` / HITL 令牌；未知动作、plan hash 变、令牌重放拒绝；v0.1 不消费写文件令牌。
2. [x] `soul-egress` 是唯一 HTTP client；`send` 必须持有 permit；跨 origin 重定向拒绝。
3. [x] redactor 默认占位；一次性整条豁免不记住；研究路径无豁免。
4. [x] 审计内容无正文；崩溃后链可验证。
5. [x] 外部内容进 `UntrustedText`，三路注入不能变成指令或外连。

WP01 允许：根工装、`crates/{soul-schema,soul-store-api,soul-testkit,xtask,soulcore}`、`fixtures/`、`.github/workflows/ci.yml`、`justfile`、schema 仅 `$ref` 重接与收紧、`schemas.lock.json`、`SECURITY.md` 加密落地段、`STATUS.md`。

WP01 禁止：产品定义文档、`apps/`、业务 crate、HTTP client 进 normal 依赖、实现导入/档案/采集/起草/文件计划。

## AC 映射

权威矩阵在 `docs/FORMAL_WORK_PROMPT.md`。「CI」一律读作本地门禁（D50）。G-L 不能自证的仅 AC-01 托盘目视（G-W 的 `gate-win.ps1` 做 asInvoker 与 `-SkipInstall` smoke 半自证；托盘 author-manual）。AC-09/10 真机行为进手动清单，CI 用 Fake 源走真管道。

## 红线

明文 JSONL 不当库；假采集器不得手工 insert 冒充 AC-10；研究预览不得写常量 0 行；`soul-fileplan` 无写 API；E0 无代码路径；特质轴无数字分数；整理不是卖点；无证据推断不落库；审计无正文；外部内容不进指令位；schema 变更须批准。
