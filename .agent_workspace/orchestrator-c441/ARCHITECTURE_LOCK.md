# ARCHITECTURE_LOCK — 已冻结架构的对照图（过程稿，不是新权威）

- model: `claude-fable-5-thinking-xhigh`（未降级）
- round: BUILD R3；role: plan（fable-a，只读产品代码）
- 基线: 唯一主干 `cursor/soul-goal1-7b1c`；工作分支 `cursor/goal1-build-audit-c441` @ `2bd56a2`
- 本文件性质: **对照图**。`.agent_workspace/**` 按 `PLAN_INDEX.md` 第四节不是权威面；本页任何一句与左列权威冲突时，以权威为准，改的是本页。

## 一段话：架构已冻结，权威在哪

架构与计划**已经冻结**（`docs/STATUS.md` 里程碑 `PLAN_FROZEN`），本轮只确认与映射，不重开。唯一权威分布是：产品是什么/不是什么与 v0.1 十三片切片在 `docs/PRODUCT_LOCK.md`（D27 禁止第二份 PRODUCT.md）；拍板在 `docs/DECISIONS.md`（本树 D1–D31；`main` 上已扩至 D60）；crate 钉死与 WP DAG 在 `docs/GOAL1_PLAN.md`；验收门禁只有 `docs/FORMAL_WORK_PROMPT.md` 的 Given/When/Then 矩阵（D29 散文不作门禁）；加密/密钥/遗忘/审计规范在 `docs/SECURITY.md`；灵魂层算法在 `origin/main:docs/algorithms/DECISION.md`（`ALGO_FROZEN`，本 Goal 1 树没有这份文件，也没有 `soul-algo-*` crate——这是 `PLAN_INDEX.md` 第三节写明的仓库拓扑，不是缺失）；三十秒定位页是 `origin/main:docs/PLAN_INDEX.md`。本页自己不定义任何新事实。

## 三层（散文层图）

- **灵魂层**——复刻本体：档案（可替换特质轴 + 证据档 + 用户纠正锁定，`soul-profile`）、自传记忆（每单元内容密钥，遗忘=销毁 CK，`soul-memory`）、人脉图（节点=人、边带证据与最近接触，`soul-graph`）、进料（两种导入 + 问卷回退，`soul-import`）。全部落在加密主库（`soul-store`）里，第三人数据默认不出本机。
- **代理层**——在灵魂层约束下、人在回路审批后动手：起草不发送与人事摘要（`soul-draft`）、只读目录扫描与计划预览（`soul-fileplan`，v0.1 无写 API）、前台应用时长采集（`soul-collect`，默认关）。所有权限、同意、审计内容、第三人占位、HITL 令牌集中在 `soul-policy`；唯一能出网的门是 `soul-egress`（E1，用户自带端点）。
- **研究层**——第三条轨道：本机事件库 → 脱敏预览（`soul-store` 的 `research_preview`，边界类型在 `soul-store-api::research`）。v0.1 只预览、`third_party_rows` 恒 0（`zero_third_party_rows` 是唯一构造入口，只接受 0）、不写文件；与助手出网彻底分离（研究路径无豁免）。

编排与呈现横跨三层：`soulcore` 只编排（`Session`、36 条 IPC 命令的真身、config.json 两字段）；`apps/desktop` 是薄壳（WebView + 托盘 + 一行转发）。

## Crate DAG（对照 GOAL1_PLAN「钉死」，逐条核过 Cargo.toml）

依赖箭头自下而上，禁止倒流：

```
soul-schema（纯契约，无 IO）
  └─ soul-store-api（存储边界：trait + conformance + FakeStore；无真库）
       ├─ soul-store（唯一落盘业务数据：SQLCipher + 字段 AEAD + keys.dpapi）
       │    └─ soul-win-dpapi（唯一 unsafe，两个 Win32 调用）
       ├─ soul-policy（唯一签发 EgressPermit / HITL 令牌；无 HTTP）
       │    └─ soul-egress（唯一 HTTP client；无 permit 发不出请求）
       ├─ soul-profile / soul-memory / soul-graph / soul-import / soul-collect（业务，面向边界，不碰真库）
       ├─ soul-draft（无 HTTP 正依赖；ReplyGenerator 只吃 RedactedBody）
       └─ soul-fileplan（只读扫描；无存储依赖，产物在内存）
soulcore（只编排；E1 路径唯一入口；产品配置 config.json）
apps/desktop soul-desktop（独立 workspace 薄壳，转发 soulcore）
工装：soul-testkit（只准走 dev 边）、xtask（门禁本身，不进成品）
```

- **谁可以 HTTP**：只有 `soul-egress`（`reqwest` 正依赖）。强制手段两道：`deny.toml` 第 92 行 `{ name = "reqwest", wrappers = ["soul-egress"] }`；`xtask e0-audit` 只从成品根走 normal/build 边。`soul-testkit` 内的 axum/tokio 与 `soul-draft` 的 `soul-egress` 均为 dev 边（AC-11/12 需要回环真服务器），不构成出网路径。`apps/desktop` 无 `tauri-plugin-http` / `tauri-plugin-updater`，`tests/no_egress_path.rs` 走 Cargo.lock 钉住。
- **谁可以落盘**：业务数据只有 `soul-store`（`soul.db` + `keys.dpapi`）。两个有意的旁侧：`soulcore` 写 `config.json`（仅 `wizard_completed` / `authorized_roots` 两字段，是配置不是业务数据，`session_*.rs` 把文件字节读回来搜词钉住）；`soul-fileplan` **读**磁盘（只读扫描是其宪章），无写 API（`tests/no_write_api.rs` 读回源码断言、`execution_is_refused.rs`）。
- **谁是纯的**：`soul-schema` 正依赖只有 serde/serde_json/jsonschema/thiserror/uuid，src 无任何 fs 调用（仓库级 grep 核过，fs 只出现在其 tests）。`soul-profile` / `soul-memory` / `soul-graph` / `soul-import` / `soul-collect` / `soul-draft` src 均无 fs、无 HTTP，只面向 `soul-store-api`（`soul-collect` 的 `src/windows.rs` 读 OS 前台进程，非磁盘写，且在门后）。
- **UI 无业务逻辑**：`apps/desktop/src-tauri/src/commands.rs` 首注即宪章——每个命令一行转发；进程唯一一个 store 句柄开在 `lib.rs`，`tests/one_store.rs` 读源码钉住不许出现第二次打开。
- **权限单点**：`EgressPermit` 无公开构造函数（WP08 取舍 1），别的 crate 伪造不了；`soul-policy` 自身无 HTTP，依赖方向永远是 `soul-egress → soul-policy`。

## v0.1 垂直切片（指针，不是复述立法）

十三片的唯一权威是 `docs/PRODUCT_LOCK.md`「v0.1 最小垂直切片」，砍/留表与「明确不做」同页。此处只给映射方便派单定位：①安装/托盘/向导（壳 + `soulcore::commands::shell`）②问卷/两种导入（`soul-import`）③档案+图 v0（`soul-profile`/`soul-graph`）④纠正锁定（`soul-profile`）⑤记忆 CRUD+遗忘（`soul-memory`/`soul-store`）⑥人事摘要（`soul-draft`）⑦前台采集（`soul-collect`）⑧起草不发送+占位（`soul-draft`/`soul-policy`/`soul-egress`）⑨只读扫描计划（`soul-fileplan`）⑩研究预览（`soul-store`）⑪云开关尚未启用（零 E0）⑫审计全覆盖（`soul-policy` 内容 + `soul-store` 链）⑬CI+安装 smoke（`xtask`/workflows）。算不算过，只看 FORMAL 矩阵 AC-01–AC-26。

## 明确不归本轮架构裁量的事

- **T4D/A0 吸收**：`ALGO_FROZEN` 的采纳义务（`algorithms/DECISION.md` 第 6 节）归 **PR #7 集成线**，等用户裁决唯一集成线与合并顺序（R2 N6，硬停）。本主干 `crates/soul-graph/src/build.rs:40-44` 的本地 3/10/3 常量是该决议第 6.5 条写明的「过渡期语义/遗留行为」，**禁止在本主干秘密重实现 T4D 接线**。
- **AC-27**（文件整理写执行与撤销）：v0.1.1，不是 Goal 1（D31/D19）。`soul-fileplan` 无写 API 正是架构面。
- **Goal 2**：门在 Goal 1 关闭之后（D28；`GOAL2_POLISH_PROMPT.md`）。LOOP20 保持 QUEUED。
- **hosted CI 账本与作者 Win11 清单**：blocked-on-user，非架构、非代码任务，不许用 empty-commit 或改 workflow「修好它」。

## 漂移核查结论：无

按上面四条钉死逐 crate 核过 16 个 `crates/*/Cargo.toml`、`deny.toml`、`apps/desktop/src-tauri/Cargo.toml` 与 src 级 fs/HTTP 面：**没有任何 crate 违反冻结 DAG**。三个容易误报的点已在上文点名并给出为什么不是漂移：`soul-graph` 本地判档常量（ALGO_FROZEN 过渡期语义，归 PR #7）、`soulcore` 的 config.json（配置非业务数据）、`soul-fileplan` 读盘（宪章内只读）。
