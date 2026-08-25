# Round 1 结论简报 — Soul 项目目前情况与下一步

**调度器**：cursor-grok-4.6-high（父）  
**循环**：本目标要求「先单轮」，只执行 Round 1，不进入 Round 2/3 实现冲刺。  
**日期**：2026-08-24  
**六子代理模型（均未静默降级）**：

| ID | 实际 slug | 报告 |
|---|---|---|
| r1-fable-architecture | `claude-fable-5-thinking-xhigh` | `r1-fable-architecture.md` |
| r1-fable-sota-next | `claude-fable-5-thinking-xhigh` | `r1-fable-sota-next.md` |
| r1-opus-core-inventory | `claude-opus-5-thinking-high-fast` | `r1-opus-core-inventory.md` |
| r1-opus-gap-debt | `claude-opus-5-thinking-high-fast` | `r1-opus-gap-debt.md` |
| r1-sol-env-probe | `gpt-5.6-sol-xhigh-fast` | `r1-sol-env-probe.md` |
| r1-sol-test-boundary | `gpt-5.6-sol-xhigh-fast` | `r1-sol-test-boundary.md` |

---

## 一句话

Soul 的产品定义已冻结，Goal 1 的灵魂层与权限底座已在 Draft PR 里写成可证明的 Rust 核心；**用户今天能点的只有向导、概览和「尚未启用」的云开关。** `main` 仍然几乎是空的。下一步应关 Goal 1 的代理层（起草 + 只读文件计划）并把壳接到真库，而不是开 Goal 2。

---

## 仓库现在长什么样

| 引用 | 内容 | 状态 |
|---|---|---|
| `origin/main` | 1 个文件：`README.md` 写着 `# Soul` | 默认主干，无应用代码 |
| [PR #1](https://github.com/Xhhemoing/Soul/pull/1) `cursor/soul-product-lock-7b1c` | 23 文件 / +1111 行：产品锁、schema、三轮扫描 | Draft，无 CI |
| [PR #2](https://github.com/Xhhemoing/Soul/pull/2) `cursor/soul-goal1-7b1c` | 293 文件 / +44523 行：WP01–WP09 壳 | Draft；Linux lint/test 绿；Windows desktop-shell 在本简报落盘时仍在跑 |

拓扑：#2 **严格包含** #1（`merge-base` = product-lock tip `a785317`）。不要按「#2 后再合 #1」的顺序重复合。

本云环境已预装 Rust **1.83.0**（与仓库钉死版本一致）、Node、pnpm；**没有 `just`**；当前 checkout 没有 `Cargo.toml`，在 `main` 上 `cargo check` 会立刻失败——这是预期，不是 Goal 1 编不过。

---

## 已实现功能（树实证，不靠 STATUS 宣称）

产品：本机复刻电子版的你（人格、记忆、心理工作模型、人脉图）；Windows 本地优先；采集默认关；v0.1 零业务出网（E0 无代码路径）。权威：`docs/PRODUCT_LOCK.md`（两远程分支逐字节相同）。

### Rust 核心（13 个 crate，全部有 `tests/`）

| 层 | 已落地 | 入口 |
|---|---|---|
| 数据面 | SQLCipher 主库 + 字段级 AEAD + 密码学遗忘 + 研究预览（第三人行构造级强制为 0） | `soulcore::commands::store` |
| 灵魂层 | 五条方向轴、纠正锁定、自传记忆 CRUD、人脉图（证据推导）、JSONL / Telegram 导入 | `profile` / `memory` / `graph` / `import` |
| 采集 | 仅前台应用时长；未同意事件=0；不采窗口标题（三层测试钉死） | `collect`（soulcore 层无独立测试文件，验收在 `soul-collect/tests/`） |
| 权限面 | EgressPermit 无公开构造、E1 精确 origin、第三人 redactor、HITL 令牌、三路注入隔离、无正文审计链 | `policy` + `soul-egress` |
| 壳 | Tauri 2 + React；向导默认全关；云开关「尚未启用」；托盘代码在 | `apps/desktop`，独立 cargo workspace |

测试资产：约 **58** 个测试文件、**278** 个 crate 集成测试函数、前端 20 项、src-tauri 21 项。红线测试普遍带反例对照（「不武装 fail point 就不死」「改泄漏阈值才抓得到短句」）。

### UI 实际能用的

9 条路由里 **2 条真 UI**（概览、设置）+ 全屏向导。IPC 只注册 3 个命令：`config_snapshot` / `complete_wizard` / `cloud_toggle`。壳握的是内存 `Config::default()`，**从未打开 `SqlCipherStore`**。档案 / 人脉 / 记忆 / 起草 / 文件 / 研究 / 审计 七页都是 `Pending` 占位，测试还故意钉住「起草页没有发送按钮、文件页没有执行按钮」。

**读进度表容易高估可用性**：WP03–WP07 标「完成」指的是 Rust 层，不是用户能点到。

---

## v0.1 十三切片对照

| 切片 | 状态 |
|---|---|
| 1 安装/托盘/向导 | 壳+向导有；安装器从未跑过；托盘/UAC 需真机 |
| 2 问卷+导入 | Rust ✅；UI 无导入页 |
| 3 档案+人脉图 | Rust ✅；UI 占位 |
| 4 纠正锁定 → 起草语气变 | 锁定 ✅；起草 ❌ |
| 5 记忆+遗忘 | Rust ✅；UI 占位 |
| 6 人事分析摘要 | ❌ 无 crate |
| 7 前台采集 | Rust ✅；UI 无同意开关入口 |
| 8 粘贴起草 + E1 占位 | 管道 ✅；`soul-draft` 不存在 |
| 9 只读文件计划 | ❌ `soul-fileplan` 不存在（HITL 拒绝底座已在） |
| 10 研究预览 | Rust ✅；UI 占位 |
| 11 云开关零 E0 | ✅ |
| 12 审计十类动作 | 已有动作覆盖；起草/文件计划无产生者 |
| 13 CI + 安装 smoke | Linux 绿；Windows 根测历史上绿；**打包 = 0** |

约 **16.5 / 26** 条 AC 有 CI 级证据。剩余全部集中在 WP10 / WP11 / WP09 功能视图 / WP13。

---

## 遗留缺陷（父仲裁）

六份报告一致的：

1. **`soul-draft` / `soul-fileplan` 不存在**（含空 crate 都没有）。`soul-memory/src/draft.rs` 是记忆入参，与 WP10 同名陷阱。
2. **壳未接真库**；配置无持久化；`authorized_roots` 恒空，文件计划即使写了也没有「授权 A」。
3. **`DpapiKeyProvider` 两个入口都返回 `Unsupported`**。SECURITY 写明：补齐前不得声称 Windows KEK 已受保护。
4. **桌面是独立 cargo workspace**：根 clippy / cargo-deny / e0 依赖遍历碰不到 `soul.exe` 那棵树；`desktop-check` 从未进 CI。
5. **`tauri build` 从未跑过**；WP09 七条 Windows 手动清单全部仍成立。
6. **GOAL1_PLAN 只给了 WP01/02/08 完成定义**；WP10/11/09 视图/13 没有同等粒度的完成定义。

有分歧、由父仲裁的：

| 议题 | fable-sota | opus-gap | 父结论 |
|---|---|---|---|
| 双问卷合并 | STATUS 方案可行：`soul-profile` 实现 `UserStatedSink`，import 零改 | `RecordedAnswer` **不带答案正文**；两套题型（自由文本 vs 封闭枚举）不同；STATUS 方案站不住 | **采 opus**。合并是 P1，但是一次契约改动，不是「十行适配器」。 |
| DPAPI vs 批 5 | WP10/WP11 可立即并行；接线先用 `TestKeyProvider` 并如实标注 | DPAPI 与壳接库是同一条 Windows 阻塞链，应最先暴露 unsafe | **两者兼容**：批 5 核心 crate 可并行；Windows 发布/AC-01 被 DPAPI 阻塞；UI 不得假装 KEK 已保护。 |
| 导入事件不去重 | 可带债，UI 承认即可 | 会让人脉图强度带翻倍，用户看不见 | **可带债关门，但必须 UI 提示**；否则 AC-08 强度语义在真机上失真。 |

opus 额外发现、STATUS 未写：`soul-profile` 问卷证据既无事件也无内容密钥，落在所有遗忘单元之外——轴问卷答过的内容目前**忘不掉**。

---

## 性能 / 工程瓶颈

本轮是盘点，没有跑基准。已暴露的工程瓶颈：

- SQLCipher vendored OpenSSL：Windows CI 需要 perl/NASM；工具链钉 Rust 1.83，若干依赖被故意降级。
- 嵌套 cargo workspace + 两份 `Cargo.lock`：加 Rust 代码时容易漏桌面树。
- 无 schema 迁移器（`schema_version = 1`）；开发期只能删库。
- 本分析环境未装 `just`，也没有把 Goal 1 检出，故未复跑 `just ci`。

SOTA 对照（相对 Recall / Rewind / Pi / 本地 RAG / computer-use 代理）：Soul 领先在可证明的隐私、密码学遗忘、证据链、第三人占位、无正文审计；落后在采集密度、语义检索、代理能力、可安装成品。前两格是产品锁有意推迟；**起草是 Goal 1 内欠账**；Windows KEK 目前弱于 Recall 的 enclave 方案。

---

## 下轮攻坚重点（若继续实现）

**P0（Goal 1 批 5，可并行）**

1. **WP10 起草 + 人事摘要**：装配已有 redactor / E1 / `read_voice` / mock LLM；不发送；过泄漏与非诊断词。
2. **WP11 只读文件计划**：新建 `soul-fileplan`，源码级禁止写 API；授权根扫描、磁盘字节不变。
3. **壳接真 `SqlCipherStore`**：setup 开一次、单句柄 `manage`；IPC 三条命令改读真快照。无 DPAPI 时用 `TestKeyProvider` 并在 UI/SECURITY 如实写。

**P1（关门）**

4. 补 `DpapiKeyProvider`（unsafe 圈禁仿 WP07 `windows.rs`）；不补完不宣布 AC-01。
5. WP09 功能视图：档案/记忆/人脉/导入/遗忘/研究/审计接到 UI。
6. 合并双问卷（按契约重做，不要照抄 STATUS 的「import 零改」）。
7. WP13：`tauri build` + 安装 smoke + SBOM；作者 7 条真机清单。预先声明 CI 下 WiX/NSIS 是基建流量，不是产品 E0。

**P2 / 明确不要做**

- 不启动 Goal 2。
- 不做文件写执行、E0/云路径、OAuth、窗口标题、自动发送、诊断分数、为赶 SOTA 去上截屏或 RAG。

---

## 本轮未做

- 未 checkout Goal 1 源码，未在本环境编译或跑测试。
- 未合并 PR #1 / #2（两者仍是 Draft；#2 Windows desktop-shell 在落盘时尚未出结论）。
- 未进入 Round 2/3 写码。
