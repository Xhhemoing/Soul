# Soul 仓库规范化实施计划与回执

日期：2026-10-23，America/Los_Angeles。执行：主线程集成，`gpt-6.1-sol/high` 分别整理协作文档、CI，并进行独立只读评审。

本轮依据作者给出的规范化方案，先交付仓库规则、文档导航及 CI 配置，再按明确后续指示推进 N2 runtime/types 与只读 API。当前分支为 `codex/soul-rebuild-20261019`，起点 HEAD 为 `c5be3ed3aaa09b6ef5e2a8861a07d7c7c1fa8b46`，已有 D109 等 dirty 修改保留；该 SHA 不代表本轮未提交候选。

决定见 [ADR-001](../decisions/ADR-001-repository-governance.md)、[ADR-002](../decisions/ADR-002-local-api-runtime.md)，当前完成状态只在 [STATUS](../STATUS.md)。不新增冻结 contract、hash、baseline 或第二套常驻任务账本。

## 落地顺序

| 阶段 | 产物与依赖 | 验收与停止点 |
| --- | --- | --- |
| N1 仓库规则与 CI 候选 | AGENTS、README、四栏 PR 模板；operations；ADR 与 API 架构；STATUS 压缩及旧回执归档；顺序执行的 Windows 合成 workflow。 | 本地文档/配置核验和独立审查；远端实际运行与分支保护另列，不能用本地通过代替。 |
| N2 runtime/types 与只读 API | 一次抽取共享 Session/操作入口与 DTO；同进程受限 API 首接 `/v1/status` 和只读查询，Tauri 转调同一实现。 | 实际 HTTP 与 IPC 读取同一合成状态；默认关闭、开库失败、未授权、非回环请求与重启状态的反例。此阶段不创建无调用者的空 crate。 |
| N3 写入与事件同步 | N2 后逐项迁移导入、偏好、编辑停用、遗忘；预览/确认/一次消费不变，runtime 统一发出操作状态与变化事件。 | 固定合成链“导入→偏好→编辑停用→遗忘→重启核对”，交叉使用 HTTP/IPC；重复确认、断连、旧结果、失败后刷新与审计无正文必须覆盖。 |
| N4 平台与交付 | 经审阅的候选提交及远端实际检查；作者观察剪贴板、原生文件选择、托盘等。 | 核对目标 SHA 和 check run 链接；只有真实观察才填写人工清单，普通 CI 不签发 G-L/G-W/G-M 或 Goal 关闭。 |

N2 的边界和验证断言详见[本地 API 架构](../architecture/local-api.md)。研究实验暂停、G-L 暂缓与只用合成数据保持。

## N3 当前实施切片

2026-10-23：作者明确授权 N3 写入、主体归属、runtime 事件和 UI 刷新及 HTTP＋IPC 合成回归。起点仍为上述 HEAD 加既有 D109/N1/N2 dirty；本轮起点副本、调查、测试与失败记录位于 `E:/SoulSyntheticLab/local-api-phase2-20261023/`。主线程统一集成、维护当前文档并串行调度构建；所有子 agent 使用 `gpt-6.1-sol/high`。

1. **统一操作与主体**：runtime 编排导入 prepare/confirm/discard、范围草稿与偏好 propose/confirm/discard、Memory edit、forget preview/confirm。Tauri 和 HTTP 只转换参数并调用同一方法；导入保留锁内 begin、锁外 execute、锁内 complete。主体来自可信传输，JSON 不可自报授权主体；HTTP 共享 bearer 只代表一个驱动主体，不宣称多驱动隔离。主体元数据绑定核心现有全局单槽；跨主体确认/取消先拒绝。导入有效准备不可替换；新范围草稿和新遗忘预览沿核心原有显式替换，成功替换后归新主体，旧材料失效。提案只能从本主体的草稿产生；原主体仍遵守各槽原有消费、失效和已提交读回规则，普通核心失效同步清除过期归属。
2. **合成输入与审计**：HTTP 导入只选内置合成 fixture ID，由 runtime 持有编译时输入，不开放路径或任意导入正文。其他文本字段仅用于本轮明确的合成库。保留既有事务内业务审计、遗忘 CK 销毁及 cleanup/audit 不确定回执；runtime 主体用于操作授权和无正文事件，不冒称既有持久审计已增加主体字段。不改数据库 schema/迁移；审查发现的导入准备/取消持久状态审计缺口，以两个动作白名单补齐，原因与兼容边界见下段。
3. **事件与刷新**：runtime 产生无正文 started/settled 通知，带进程 epoch、递增序号、操作 ID、可信主体和仅用于显示去重的关联 ID；快照列出当前在途操作。已进入核心的失败也结束通知并使读取失效，不能只通知成功。UI 先订阅再快照，遇到序号缺口、恢复或 focus 时重新核对；自己的 IPC 回声与现有 JS start/finally 去重。外部开始立即隐藏旧正文和预览，结束只重读普通查询，不自动重新准备、确认、遗忘或起草。
4. **测试与交付顺序**：先保存会失败的主体/一次消费/事件用例，再实现 runtime/types 和薄 IPC；接口稳定后并行 HTTP 与前端适配。主线程运行真实回环 HTTP＋生产 IPC handler 链：导入→偏好→编辑停用→遗忘→关闭重开；补跨主体拒绝、重复确认、断连结算、失败刷新、迟到读取和无正文审计。独立只读评审实际候选，修复后定向复审；最后运行涉及的两个 workspace 和前端检查。

本片实际结果见下方 [N3 实施回执](#n3-统一写操作与事件刷新实施回执)。沿用 N2 的 D 盘构建缓存与进程内资源参数（incremental=0、DEV/TEST debug=0、jobs=2），不清理 E 盘缓存，不安装依赖，不自动提交、推送或触发远端 CI。原生交互/人工清单、生产数据和研究暂停边界保持。

独立静态审查指出：`prepare_import` 的操作预留/旧操作失效和 `discard_import` 已有持久写入但没有对应审计；runtime 的跨主体提前拒绝也会绕过原遗忘拒绝审计。本轮依据作者明确授权 N3、原规范“全部写操作进审计”及 ADR-002 第 9 项，修正最初“不改审计 schema”的团队假设：仅增加 `import.prepare`、`import.discard` 两个已有审计字段的白名单值，维护既有 `schemas.lock.json`，不新增 hash、contract 或 gate。相应状态与审计在原事务内提交；跨主体拒绝用现有 `capability.reject` / `TOKEN_SCOPE_MISMATCH` 和 runtime 生成的操作 UUID，不传正文或任意动作。合法遗忘继续原来的提交后审计回执，绝不加“先审计成功才能销毁”的前置条件。旧版程序不能识别新审计动作，故不承诺新链的旧版回放或降级兼容；关闭 API 不等于可以把含新动作的库交回旧二进制。原始审查保存在本轮 `review.log`。

## 对建议的具体取舍

- 保留现有 `soulcore` 和领域 crates，不将其改名/合并成一个 `soul-core`；新增 runtime/types/API 按真实调用迁移，不先进行大规模目录重命名。当前合成 fixture 分布保留，禁止真实数据是输入约束，不依赖文件夹名称。
- pnpm、两个 Cargo workspace 和 Rust 1.83 沿现有锁文件；CI 首版使用已有 `cargo test`。N1 时生成器尚未接入；N2 已实际编译 ts-rs/utoipa，细节见下方。nextest 与 Prettier 未接入。
- CI 直接执行严格 clippy 和既有 deny 配置，不加入“现有告警全部放行”清单。现有 Windows 平台必要 unsafe 不删除；生产 unwrap 规则需在对应实现片区分测试和平台适配，不用全局 lint 制造大批无关改动。
- 审计与诊断只允许安全 ID、操作类别等白名单元数据；请求/响应和错误 details 也不能回显正文或凭据。API 默认关闭、回环绑定、令牌 ACL、Host/Origin 与白名单各自检查。
- 同账户令牌文件不隔离同账户恶意进程。首轮只面对明确隔离的合成环境；生产敏感读写必须先收敛该威胁模型，不能声称“仅当前用户可读”已经解决全部本机授权。
- 数据迁移沿现有编号，每次写明升级、失败、重启和回滚边界；CK 遗忘不承诺通过回滚恢复，不新增降级回滚 API。
- 普通 PR 远端 CI 按新 D110 执行，原正式平台/人工条件仍在。旧本地检查是有效的开发证据，但不等于目标 SHA 已经远端通过或已经发布。

## N1 候选范围与历史保留

本轮只改文档、PR/CI 配置及 `justfile`、`scripts/gate-win.ps1` 的过期 CI 注释；不改业务代码、锁文件、权限协议或数据库 schema。没有暂存、提交、推送、合并、应用安装/卸载或远端配置变更。

完整证据保存在本机持久目录 `E:/SoulSyntheticLab/repository-normalization-20261023/`，未随仓库发布。起点为 `initial-status.txt`、`initial-candidate.patch` 和入口文档副本；各执行者日志为 `attempts.txt`、`docs-worker.log`、`ci-worker.log`、`normalization-review.log`。

- `docs/archive/STATUS_20261023.md` 保存压缩前全文；`docs/archive/2026-10-23-core-framework-v1.3-execution.md` 保存 v1.3 执行全文。相对链接按新目录改写后再反向还原，与原正文逐字符比较相同，结果见 `archive-preservation.json`；原日期、失败、未知费用、dirty 状态与未完成项保留。
- 原 v1.3 路径保留导航及四个现有引用标题；API 当前方案拆为 ADR-002 和 architecture/local-api。归档行为不将 Windows 未完成人工项目改为完成。
- 本次只整理上述入口和相关引用。周整理规则启用于 2026-10-22、首次到期 2026-10-29；没有宣称已核对全部历史主题。

## 已执行核对与后续证据

本节保存 N1 当时的范围与结果；N2 的后续代码和运行结果见下一节，不用新的成功改写旧限制。

| 方法与实际输出 | 结果及限制 |
| --- | --- |
| 起点 `git status --short`、`git branch --show-current`、`git rev-parse HEAD` | 退出 0；dirty 候选保留。首次目录检索包含不存在的 `.github`，`rg` 退出 2、外层退出 1；按实际缺失创建配置目录，不补造旧 CI。 |
| `gh api repos/Xhhemoing/Soul`、`gh api repos/Xhhemoing/Soul/actions/permissions`、`gh api repos/Xhhemoing/Soul/branches/main` | 各退出 0，原始 JSON 留存；仓库 public、Actions `enabled:false`、main `protected:false`，主干 SHA 为 `8aae8f3877bc93529d0a0104c277a85ded9c43aa`。只读查询，不触发工作流。 |
| CI 工具官方资料及标签核对 | checkout/setup-node v7、固定 cargo-deny 0.18.2 Windows 产物、Node 锁文件兼容、Tauri 参数依据留在 worker 日志和来源副本；这是来源核对，不是 hosted runner 执行。 |
| 原始回执归档与链接往返比较 | 退出 0；两份全文保留，位置改写可逆，未清理原始证据。 |
| 独立首轮审查与 PowerShell AST | 六个文档/配置候选未发现阻塞；13 个 run 块语法和首行设置通过。评审提取脚本首轮 regex 误纳后续 YAML 标题而退出 1，修正后退出 0；没有运行块内构建。 |
| YAML 1.2 实际解析与当前 Tauri CLI 帮助 | 复用本机已有 `yaml 2.9.0`，严格解析和事件、权限、runner、采集排除断言退出 0；Tauri 2.11.4 `build --help` 退出 0，支持 no-bundle 和 Cargo 参数透传的帮助语义。没有实际构建；首个 parser 定位命令因 JS 参数转义退出 1，外部脚本修正后退出 0。 |

| 主线程实际本地检查 | `cargo fmt --all -- --check`；桌面同命令另加 `--manifest-path apps/desktop/src-tauri/Cargo.toml`；两 workspace 的 `cargo metadata --format-version 1 --locked --offline --no-deps`；`pnpm --filter @soul/desktop lint`（tsc + ESLint）；`git diff --check` 均退出 0。原始输出分别在 `root-format.txt`、`desktop-format.txt`、`root-metadata.txt`、`desktop-metadata.txt`、`frontend-lint.txt`、`tracked-whitespace.txt`。 |
| 环境与候选保留 | rustc 1.83.0、Node v24.18.0、pnpm 10.15.0 实际版本查询退出 0。32 个既有 D109 源码/测试文件与 `scoped-register-windows-20261023/initial-source/` 对应文件逐字节相同；三份锁文件 `git diff --exit-code -- Cargo.lock apps/desktop/src-tauri/Cargo.lock pnpm-lock.yaml` 退出 0，见 `source-preservation.json`。 |
| 最终独立集成审查 | [评审回执](../reviews/2026-10-23-repository-normalization.md)未发现具体阻塞；独立重算两份全文保留比较相同，STATUS 32 行，14 个入口/快照的本地文件目标缺失 0。检查范围不含所有历史锚点与外链。 |
| 收尾引用与措辞 | 原 v1.3 导航及 archive 的四个既有引用标题均保留（8/8），见 `retained-headings.json`；末次 `git diff --check` 退出 0，既有 PRODUCT_LOCK 的 CRLF 提示不属于本次正文修改。归档目录与 CI 设置依赖三处末次措辞经 reviewer 定向复核无新阻塞。 |

主线程的批量检查复现入口为仓库外 `verify-local.ps1`；按实际命令逐条记退出码。N1 的本地文档/配置候选已实现并完成上述核验、独立审查，不标 GATED/INTEGRATED。当前只完成合成 CI 配置，未在 hosted runner 执行。

工具失败也保留：一次集成 patch 因同路径 Delete/Add 被工具拒绝，未发生写入，改用 Update 后成功；worker 的 parser 探测转义失败意外产生根目录 `unavailable`，按命令与时间确认是本次 246 字节输出后，用核验过的绝对路径移至仓库外 `parser-probe-accidental-output.txt`，未删除失败证据。副产物追查的首个 PowerShell 脚本解析失败亦留在 `ci-worker.log`，修正后的元数据核对退出 0。它们不是产品功能失败，不将其隐藏为全程无错误。

本轮没有运行完整 Rust 测试、clippy、cargo-deny 当前公告库检查、Tauri build 或 GitHub Actions；文档与配置改动未重复运行整套业务验收。actionlint 的 GitHub 专属 schema 检查、实际 runner 工具/时间预算、远端事件 checkout SHA 和分支保护仍待首次远端交付核对。新 API 尚未实现/运行，原生交互未恢复。没有远端 run URL 时保持“未运行”，不填写预期链接或借用历史绿灯。

## N2 共享运行时与只读 API：实施回执

日期：2026-10-23（America/Los_Angeles）。授权：继续 runtime/types 抽取与只读 API 第一期，子 agent 分工实现及独立复审，仅合成数据，发现问题自行修复验证。主线程集成和统一调度检查；`runtime_types_impl` 与 `api_host_impl` 分别持有互不重叠的实现文件，`readonly_api_review` 只读复审，均为 `gpt-6.1-sol/high`。既有 D109/N1 修改保留，未暂存、提交、推送或改变远端设置。

候选新增 `soul-runtime`、`soul-types`、`soul-api`；SessionState 持有同一个 Runtime，Tauri 三个新读命令与 HTTP 的 `/v1/status`、`/v1/people`、`/v1/memories/{id}` 调用同一实现。旧读 DTO 通过 runtime 兼容方法转调，旧写入/后台继续使用可信 shared Session，保持原 begin/execute/complete。不是所有业务已经迁入 runtime；N3 的 HTTP 写入、主体隔离与 UI 事件尚未实现。

产品默认关闭 API；显式 synthetic 模式只在绝对根目录下新建唯一 run/data/profile，继续保留单实例与数据库 lease，拒绝采集、外部模型、导入、任意目录及研究保存入口。令牌在写入前即具 Windows 当前用户 protected DACL，关闭后删除。两个锁文件有明确的新依赖集成，前端锁文件未变；没有 schema/迁移或 CK/遗忘语义变更。关闭 API 即可回到普通启动；不对新建合成目录自动清理或承诺降级恢复。

实现及使用边界见[架构说明](../architecture/local-api.md)，普通生成输出见 [OpenAPI](../architecture/local-api.openapi.json) 和 `apps/desktop/src/generated/runtime.ts`。`ts-rs =10.1.0`、`utoipa =5.3.1` 实际在 Rust 1.83 编译；首次根锁解析选择了要求 Rust 1.85 的 indexmap/hashbrown，已明确将新增 indexmap 锁定为兼容的 2.13.1，并保留网络重试和原始解析输出。不是静默修改锁文件以跳过 `--locked`。

原始证据位于本机持久目录 [E:/SoulSyntheticLab/local-api-phase1-20261023/](E:/SoulSyntheticLab/local-api-phase1-20261023/)，不随仓库发布。`initial-status.txt`、`initial-tracked.patch`、`initial/` 保存起点；`attempts.txt` 记录命令与退出码；`runtime-worker.log`、`api-worker.log`、`readonly-review.log` 保留分工、发现和复查。`verify-n2.ps1` 是实际串行复现入口，重复执行自动使用新日志文件名，保留失败。测试只使用 `tempfile` 新库和仓库合成 fixtures，令牌及密钥不进入日志。

| 已实际执行 | 结果与证据 |
| --- | --- |
| `cargo run -p soul-types --example typescript --locked`；`cargo run -p soul-api --example export_openapi --locked` | 均退出 0，生成输出和 stderr 留存；OpenAPI 加入 default 错误后重新生成并通过普通输出一致性测试。 |
| `cargo test -p soul-types -p soul-runtime -p soul-api --all-targets --locked` | 修复后退出 0，API 单元 4、真实 HTTP 7、OpenAPI 1、runtime 5、types 2，共 19 项；`root-n2-test-fixed.txt`。包括开库失败、严格 Host/Origin/令牌拒绝、限额/慢头、断连结算、共享 owner、分页及精确 ID。 |
| `pnpm --filter @soul/desktop lint`、`test`、`build` | 各退出 0；62 文件、744 测试通过。build 临时设 NODE_ENV=production 并恢复；`frontend-lint.txt`、`frontend-tests.txt`、`frontend-production-build.txt`。 |
| `cargo test -p soulcore --test session_local_preparation --locked` | 退出 0，11 项通过，包括修正版本期待后的完整迁移行为断言；`local-preparation-fixed.txt`。该批与下方扩大回归重叠，不累加。 |
| `cargo test --workspace --all-targets --locked -- --skip windows::runs_on_windows` | 最终退出 0，1640 通过、29 ignored、3 filtered；`root-workspace-synthetic-4.txt`。包含当前 API/runtime/types 和最终服务端依赖负例；不是未过滤全量或 G-W。 |
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked -- --skip windows::runs_on_windows --skip collection_can_be_granted_and_taken_back_over_the_ipc --skip a_restart_finds_collection_off_again` | 最终退出 0，147 通过、2 filtered；`desktop-synthetic-4.txt`。IPC 77、host 4、出网审计 5、壳检查 22 均通过；host 实测 IPC 创建/更新后 HTTP 读取同一 Memory、关停释放、凭据删除和同库重开持久性，profile 测试使用注入 setter，不启动原生 WebView。 |
| 两 workspace 的 `cargo clippy --all-targets --locked -- -D warnings`（根另加 `--workspace`，桌面指定 manifest） | 最终各退出 0；`root-clippy-4.txt`、`desktop-clippy.txt`。没有加入告警允许清单；首次及后续失败保留。 |
| 两 workspace 的 `cargo fmt ... --all -- --check` | 最终各退出 0；`root-format-final-2.txt`、`desktop-format-final-3.txt`，对应最终源码。 |
| `cargo run -q -p xtask --locked -- schema-freeze --check`、`e0-audit`、`denylist-audit` | 各退出 0；`schema-freeze.txt`、`e0-audit.txt`、`denylist-audit.txt`。这是既有 schema/出网/禁词检查，禁词审计不等于 cargo-deny 公告库检查。 |

首轮失败和修复保留：

- 初始 runtime IPC 为“命令不存在/缺少安全错误码”的实际 RED；host RED 是缺模块的编译失败。command_surface 实际 4/6，两项来自前端漏接命令和旧薄层断言；已补生成 DTO wrapper，并只精确允许 runtime 转调。
- 第一轮 N2 为 4 项 API 单元通过、HTTP 5/6；断连测试误用 Axum 状态克隆计数作为 worker 派发信号。以真实 ReadActivity owner guard 同步与退出结算后，原断言通过；失败见 `root-n2-test-1.txt`，不删除或改写。
- 依赖审计独立复审发现 feature 转发及 rename 别名绕过；已在两个 workspace 中收紧 server 特定路径及特性检查，禁止该三项依赖 rename，并补普通、weak、别名反例。没有整体豁免 soul-api 子树，也没有把合法 E1 的 client 特性要求为全局关闭。
- xtask 首次 35/36，因为旧 SBOM 测试认为 Axum 仅用于测试；修正为生产清单包含 Axum/runtime/types/API，继续排除 testkit/xtask，重跑 36/36。最终扩大检查还须覆盖后来增加的 rename 反例；日志分别是 `xtask-self-test.txt` 与 `xtask-self-test-fixed.txt`。
- 桌面首次 `--all-targets` 编译暴露 commands 单元测试仍把 Runtime 字段当作 Arc；改成比较 `state.shared()` 与 `worker.shared()`，保留双向修改可见性和同 store 断言，首次失败见 `desktop-synthetic.txt`。
- 桌面第二次到达 IPC 回归时为 76 通过、1 失败、2 过滤：新增拒绝夹具误写 source 枚举为 `soul_import_v1`，先被参数反序列化拒绝，未到预期合成限制。改为既有合法值 `soul-import-v1`，保留 `SYNTHETIC_MODE_RESTRICTED` 和副作用不发生的断言；失败见 `desktop-synthetic-2.txt`。
- 最后检查 WebView 隔离发现仅窗口配置不能覆盖继承的 `WEBVIEW2_USER_DATA_FOLDER`。根据官方 SDK 和独立复核的 Tauri/wry 调用链，合成分支现在在创建线程/窗口前，统一设置窗口目录和本进程变量。新增注入 setter 回归不读取真实环境/注册表、不启动窗口；源码修复经独立复查。该回归没有另跑“缺 helper”编译 RED，不声称已完成那一步；来源与判断保存在 `webview-profile-source.md`。
- 根扩大检查原配置因 E 盘 `os error 112` 中断，随后 Out-File 写日志也失败；原始 `root-workspace-synthetic.txt` 保留，外层退出 1，native 退出码未知。恢复使用 `verify-with-space.ps1`，顺序共用 `D:/SoulSyntheticBuild/local-api-phase1-20261023/desktop` 临时构建缓存，但两个 workspace 各用自己的 manifest/lockfile；仅进程设置 incremental=0、DEV/TEST debug=0、jobs=2，finally 恢复环境。没有删除、搬动原缓存或降低测试断言；后续结果明确属于此资源配置。
- D 盘第二次根扩大检查仍在编译时执行会话失效；恢复后确认 cargo/rustc/nmake 均不在运行，日志没有测试总结或 native 退出码。`root-workspace-synthetic-2.txt` 按中断/退出码未知保留，不能判通过；复用现有缓存重跑相同命令，使用第三份日志。没有推定中断原因或终止其他进程。
- 第三次根扩大检查在 `session_local_preparation` 为 10 通过、1 失败，旧升级测试仍期望 schema 7，而既有 D109 当前版本为 8。仅将最终版本断言改用 `STORE_SCHEMA_VERSION`，保持默认关闭、不新增模板/偏好、不重排工作及重启断言，产品迁移未改；失败见 `root-workspace-synthetic-3.txt`。
- 桌面第三次中 IPC 77 项通过（2 项真实采集排除）、host 4 项和依赖审计 5 项通过；随后 `shell_is_local_only` 为 21 通过、1 失败。其旧文本检查只识别 `.build(tauri::generate_context!())`，不能识别当前先调整合成 profile 再 `.build(context)` 的链；修复保留 context 来源、configure/注册关闭处理器/build/run 的顺序及托盘行为断言，失败见 `desktop-synthetic-3.txt`。
- 严格 clippy 首次报告 `xtask/egress.rs` 的 nonminimal_bool；按等价德摩根变换修正，无 allow 或安全例外扩张，独立复查确认拒绝条件相同。桌面 fmt 首次只要求 runtime_reads/scoped_register 模块排序，按原输出修正并重跑退出 0；两次格式日志均保留。
- clippy 第二次报告凭据模块的生产/测试缓冲区手写向上取整；两处改为 Rust 1.83 支持的 `div_ceil`，保持 `Vec<usize>` 对齐、Win32 字节长度、零长度拒绝与 DACL/秘密写入顺序。独立复查确认等价，原始 `root-clippy-2.txt` 退出 101 保留。
- clippy 第三次仅报告 HTTP 拒绝测试的多余借用；`contains(&auth)` 改为 `contains(auth)`，仍要求所有拒绝响应不成功且不回显凭据/内部目录，`root-clippy-3.txt` 退出 101 保留。第四次原命令退出 0。
- 工具探测的缺路径、过期 patch 上下文、格式预览非零、早期 `--locked` 需要显式集成依赖，以及网络超时重试均在 `attempts.txt` 和各原始日志保留；不计作业务测试通过。

N2 本地实现及所列合成验证已完成，独立源码与最终原始证据核对见[复审回执](../reviews/2026-10-23-local-api-phase1.md)。最终两套扩大回归的汇总来源是 `summarize-tests.ps1` / `final-test-summary.json`，仅逐条累加各自日志中的 test result，不跨批次累加；执行退出码仍以 `attempts.txt` 和原日志为准。文档局部文件目标检查见 `verify-documents.ps1` / `document-targets-final.json`，不检查全部历史锚点或外链。

最终候选源码保存为 `final-tracked.patch`、`final-scoped-tracked.patch` 和 `final-source/`（47 份，见 `final-source-paths.txt`）。这是 dirty 候选的普通副本，HEAD diff 中保留了重叠路径上的既有修改，结合 `initial/` 与起点 patch 区分，不把 HEAD 本身当成完整交付版本。收尾分支/HEAD 未变，暂存区为空。

本轮结果属于上面注明的 D 盘、debug=0/incremental=0/jobs=2 资源配置；E 盘空间不足尚未处理，原目标目录和缓存保留。没有重新运行原资源配置下的完整测试或生产打包，不能据此签发正式 G-W。`cargo-deny` 工具未安装，本轮没有运行当前安全公告/许可证检查；没有为此自行安装工具。

Windows 原生窗口实际 profile、托盘/剪贴板、安装和远端 CI 未运行；不签发 G-L/G-W/G-M、Goal 关闭、合入或发布。下一片为 N3：在既有授权/一次消费边界下逐项迁移写操作及主体归属，再加入 runtime 事件和 UI 失效处理，最后跑固定 HTTP＋IPC 合成写入链。本轮没有提前开启这些写接口或恢复研究。

## N3 统一写操作与事件刷新：实施回执

2026-10-23（America/Los_Angeles）。本节接续 N2，旧节中的“下一片/未实现”只描述当时版本。主线程集成并运行全部构建；`runtime_types_impl`、`api_host_impl`、`n3_ui_impl` 按互斥文件和接口依赖分片，最多两名实现者同时工作；`readonly_api_review` 独立只读复审，均为 `gpt-6.1-sol/high`。未另开 worktree，沿用用户指定 dirty 工作树，未暂存、提交、推送或改远端设置。

产物是 N3 的统一操作编排、可信主体、共享 DTO/生成物、HTTP 写入口、native relay 与 UI 事件刷新。固定链使用编译时 `fixtures/import/soul-import-v1/valid_basic.jsonl` 和新临时合成库；新库范围起草先通过既有生产 IPC `set_voice` 建立合成档案。导入仍 prepare/confirm，偏好仍 propose/confirm，遗忘仍 preview/confirm；HTTP 不接收任意文件或导入正文。当前 API 的支持范围、错误和重启边界见[架构说明](../architecture/local-api.md)。

新增 `import.prepare` / `import.discard` 只扩展既有审计动作白名单并更新原 schema lock，未改 DB schema 或添加迁移；失败审计与导入状态一起回滚。事件主体不等于持久 actor 审计。新审计链不能保证旧二进制读回，关闭 API 不构成数据库降级方案；CK 遗忘不可恢复。除此以外未新增依赖版本；两个锁文件只更新当前 workspace 内 runtime 的实际依赖。

本机证据目录为 `E:/SoulSyntheticLab/local-api-phase2-20261023/`，`check.ps1` 保留每次命令、独立日志与 native 退出码，`generate.ps1` 保存 TS/OpenAPI 原始输出；全部失败追加保留，不覆盖。环境沿用 `D:/SoulSyntheticBuild/local-api-phase1-20261023/desktop`，仅本次进程 `CARGO_INCREMENTAL=0`、DEV/TEST debug=0、jobs=2，finally 恢复；前端生产构建仅临时设置 NODE_ENV=production。不清缓存、不运行 G-L 或原生窗口，E 盘低空间仍保留。

### 定向运行及失败修复

| 执行范围与原始日志 | 实际结果 |
| --- | --- |
| `cargo test -p soul-runtime --test owned_writes --locked`，`runtime-owned.txt` | 7 项通过，退出 0；跨主体不能消费/取消、原槽替换和过期、原主体确认消费规则、事件元数据及重启只回读不恢复批准。 |
| `cargo test -p soul-store --test import_operations --locked`，`store-import-operations.txt` | 7 项通过，退出 0；新增 hook 失败时状态回滚、正常重试只提交一次。 |
| `cargo test -p soulcore --locked --test session_import_work b1_`，`import-completion-audit.txt` | 2 项通过、14 过滤，退出 0；执行后的失效和审计同事务，包括审计失败保持无 held 批准。后续扩大检查覆盖整文件。 |
| `cargo test -p soul-api --test http_write --locked`，`api-write.txt` | 4 项通过，退出 0；真实 TCP 导入、显式 discard、严格 JSON/正文上限与超时在受理前、断连后已受理操作继续结算再关停。 |
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test local_api_runtime --locked`，`desktop-local-api-2.txt` | 7 项通过，退出 0；生产 IPC＋真实 HTTP 完整“导入→偏好→编辑停用→遗忘→关闭重开”，反向主体拒绝和 native started/settled；native 侧为 MockRuntime，不是 Wry/GUI。 |
| 前端受影响 12 文件，`frontend-targeted-6.txt`；App 恢复原命令，`frontend-app-recovery.txt` | 分别 173 项、34 项通过，均退出 0；后者是后续修复的范围验证，两个批次重叠，不累加。完整前端结果另列。 |

首轮失败、复现与修复路径保留：

- `runtime-red.txt` 是缺少 runtime/context/DTO 的编译失败，不冒充行为 RED；`frontend-red.txt` 实际 4 失败/7 通过，覆盖缺 bridge 和外部刷新。`api-write-red.txt` 旧只读服务的 3 项实际失败，表现为 405/没有受理，不是请求成功。native host 链没有另跑 tests-only 编译 RED，如实保留。
- UI 第一轮 156 通过/1 失败：同一窗口 forget 的 PLAN_HASH_MISMATCH 丢了可继续使用的预览。仅在确认是自己两次通知且无外部交错时恢复原预览，外部变化仍清除。`frontend-import-race-red.txt` 两项失败复现 stale DOM 确认及迟到错误覆盖；同步来源失效、地址/mount/generation 归属修复后通过。
- 独立复审发现仅 correlation 去回声、首个 snapshot 未阻断/刷新、listen 失败无恢复（F1–F3）。新增反例实际失败保存在 `frontend-review-red.txt`；修复主体核验、未知状态阻断、监听重试、首次快照和 ordinary read 的派发/交付检查。`frontend-unknown-read-red.txt` 两项失败保留。初次过度拦截本地 pending 导致 `frontend-targeted-5.txt` 13 项失败，修正只阻断未知/外部在途，保留原本地生命周期；不是删断言换绿。
- 同步失败后 App 曾停在 loading（`frontend-targeted-4.txt` 170 通过/1 失败）；改为安全固定提示。复审再发现恢复先于本地 IPC 回执会卡错误页（F4）：第一份 RED 因 ESM spy harness 错误失败，不计行为复现；第二份 `frontend-app-recovery-red-2.txt` 实际 9 通过/1 失败。App 改订阅原 draft-source，使本地 finally 也能唤醒延期读取，外部通知仍覆盖，原命令 GREEN 34 项。
- B1 的 `import-completion-audit-red.txt` 两项实际失败：Reserved→Invalidated 的执行后分支未审计，审计不可用时仍持久失效、未回滚；调用仍返回来源已变化的拒绝，不是业务成功回执。修复仅接已有事务 hook，原命令两项通过；不更改合法遗忘的提交后审计语义。
- 桌面第一轮 `desktop-local-api.txt` 为 5 通过/2 失败；新库缺语气档案，核心正确以 ROUTINE 拒绝起草。仅通过已有生产 IPC 设置合成语气前提，原主体/一次消费/遗忘/重启断言不变，原命令重跑通过。
- `store-hook-red.txt` 首次 PowerShell 裸 `--` 被脚本参数处理吞掉，cargo usage 失败不是业务测试；改数组 splat 后 `store-hook-red-2.txt` 是缺事务 hook 的编译 RED。读错不存在路径、格式预览、第一轮未格式化候选等均留在 attempts/worker 原始记录，不算产品成功。
- 桌面扩大检查首轮 `desktop-synthetic.txt` 在薄层检查为 5 通过/1 失败：rustfmt 的合法最外层尾逗号使字符串断言误判。只在精确 runtime 转调分支允许这一个可选逗号；方法、参数、可信 context、分号和花括号检查全部保留，生产命令未改。原完整命令重跑 `desktop-synthetic-2.txt` 退出 0，独立复审确认没有放宽为任意逻辑。
- 前端扩大检查 `frontend-tests.txt` 为 64 文件中 61 通过/3 失败，775 项中 772 通过/3 失败。App 用例只拿到初始“读取中”元素便断言最终文字；Memory 用例在真正 A→B 攻击前，初始 native hashchange 尚未完成。分别等待原文字就绪和 mount 前完成初始导航；所有最终状态、旧按钮攻击、旧正文和零写入断言保留。
- 同一扩大检查发现 Graph 的真实延迟聚焦竞态：用户已聚焦 metadata 按钮，导航 rAF 仍强制聚焦关系标题。受控 rAF 反例在 `frontend-focus-red.txt` 实际得到 expected button / received h3。该次 pnpm 参数误带字面 `--`，过滤未生效，实际为 64 文件、775 通过/1 失败，不描述为单例运行。修复记录排队时焦点，仅在未被接管时聚焦，保留原 hash/source/target 检查；补无人接管时标题仍聚焦，`frontend-final-fixes.txt` 5 文件 84 项通过。独立复审闭合，真实 WebView 焦点时序仍未观察。

### 扩大检查与收尾

| 当前候选的扩大检查 | 结果与原始日志 |
| --- | --- |
| `cargo test --workspace --all-targets --locked -- --skip windows::runs_on_windows` | 退出 0；`root-workspace-synthetic.txt`，1654 通过、29 忽略、3 过滤。包含当前共享 DTO/TS、OpenAPI 一致性和完整 `session_import_work`；不是未过滤全量。 |
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked -- --skip windows::runs_on_windows --skip collection_can_be_granted_and_taken_back_over_the_ipc --skip a_restart_finds_collection_off_again` | 最终退出 0；`desktop-synthetic-2.txt`，150 通过、2 过滤。根测试不代替本 workspace；MockRuntime 不代替原生窗口。 |
| 两 workspace 的严格 `cargo clippy --all-targets --locked -- -D warnings`（根另加 `--workspace`，桌面指定 manifest） | 均退出 0；`root-clippy.txt`、`desktop-clippy.txt`。未增加 lint 允许清单。 |
| 两 workspace 的 `cargo fmt ... --all -- --check` | 实施中及最终源码检查均退出 0；最终日志为 `root-format-final.txt`、`desktop-format-final.txt`，初查日志亦保留。 |
| `cargo run -q -p xtask --locked -- schema-freeze --check`、`e0-audit`、`denylist-audit` | 均退出 0；`schema-freeze.txt`、`e0-audit.txt`、`denylist-audit.txt`。既有检查不代替 cargo-deny 当前安全公告扫描。 |
| `pnpm --filter @soul/desktop test` | 最终退出 0；`frontend-tests-2.txt`，64 文件、777 项通过。原先失败和焦点 RED 保留，不与最终批次累加。 |
| `pnpm --filter @soul/desktop build`；`pnpm --filter @soul/desktop lint` | 最终均退出 0；`frontend-production-build.txt`、`frontend-lint-3.txt`。实际执行 tsc、Vite 生产构建及 ESLint；没有运行 Tauri 发布打包/安装。 |

源码独立复审的全部具体发现已经闭合，见[复审回执](../reviews/2026-10-23-local-api-phase2.md)。统计由 `summarize-tests.ps1` 从最终两个 Rust 日志分别逐条求和，见 `final-test-summary.json`；不跨重叠批次累加，native 退出仍以原日志和 `attempts.txt` 为准。工具实际版本在 `tool-versions.txt`：Rust 1.83.0、Node v24.18.0、pnpm 10.15.0。

收尾入口为 `final-capture.ps1` 与 `verify-documents.ps1`：保存当前 branch/HEAD/空暂存区、`final-tracked.patch`、相关 `final-source/` 和路径清单，结合起点 `initial/` 与 `initial-tracked.patch` 区分既有 dirty。普通源文件副本不是新 baseline/hash/gate；本轮没有生成提交 SHA 或远端 run URL。当前文档只核对本地文件目标和 STATUS 行数，不冒充全历史锚点或外链验证。

**N3 本地实现、所列合成运行验证和独立复审已完成；未提交、合入或发布。** 所有业务输入为内置合成 fixture、显式合成文本及临时新库。主链实际使用真实 TCP HTTP 和生产 IPC handler，native 部分使用 MockRuntime；UI 刷新及竞态在 Vitest/jsdom 验证。没有把这种组合称为真实 WebView/Windows 人工交互端到端通过。

当前 `cargo-deny` 可执行文件缺失，本轮未安装、未跑公告/许可证扫描；E 盘低空间没有处理，结果属于本节注明的 D 盘构建资源配置。原生窗口 profile/焦点、剪贴板、文件选择器、托盘、Tauri 发布打包/安装和目标 SHA 远端 CI 未运行；不签发 G-L/G-W/G-M 或 Goal 关闭，不恢复研究或真实数据。后续进入 N4 的提交/远端交付与作者原生观察边界，不能由本地通过自动推定已获发布授权。
