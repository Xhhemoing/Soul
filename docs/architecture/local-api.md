# 本地 API 与共享运行时

当前候选已接入 N3 的导入、范围偏好、Memory 编辑/遗忘及 runtime 事件；实际验证进度以[实施计划与回执](../plans/2026-10-23-repository-normalization.md#n3-当前实施切片)为准。本文描述当前接口和限制，不代替运行证据或原生平台验收。决定见 [ADR-002](../decisions/ADR-002-local-api-runtime.md)。范围仅限用户授权的合成数据。

## 当前基础

- `soulcore` 是 Rust library，`Session::open(directory)` 可无 Tauri 运行；它返回的对象可能 store 不可用，健康状态必须检查 `status().store_opened`。
- `apps/desktop/src-tauri/src/commands.rs` 的 SessionState 持有 `soul_runtime::Runtime`；Runtime 是 `Arc<Mutex<Session>>` 的构造 owner，两个传输与既有后台使用同一对象。
- 导入/生成等长任务具有锁内 begin、锁外 execute、锁内 complete 模式，worker 持有真实操作并负责结算。不能把 HTTP future 丢弃当作业务取消。
- `core.ts` 保留本地调用的 start/finally 通知，同时接收 native runtime 事件和快照；按可信 WebView 主体与关联 ID 去除自己的回声，外部变更失效旧读取并触发重读。
- 测试树存在 VAL01 HTTP operator，产品 `soul-headless` 没有通用 API serve 子命令。测试操作器不是产品服务。

抽取前的静态调查见历史 [API 可行性核对](../archive/2026-10-23-core-framework-v1.3-execution.md#后续三本地-api-化可行性与建议)。当前候选沿 `c5be3ed` 加 D109/N1 dirty 工作树实现；不是该提交本身已带 API。

## 目标依赖

```text
React UI → Tauri adapter ─┐
                         ├→ soul-runtime → soulcore + existing domain crates
Script → soul-api ────────┘         │
                                   └→ body-free changes → UI invalidation/refresh
          soul-types ← shared boundary DTO → generated TypeScript / OpenAPI
```

| 模块 | 职责与边界 |
| --- | --- |
| `soulcore` / 现有领域 crate | 算法、事务、存储、授权、审计、预览与一次消费；保留现有 crate 名称 |
| `soul-runtime` | Session owner、安全投影及旧 IPC DTO 兼容；统一编排本片写操作、调用主体和无正文事件；无 Tauri 类型 |
| `soul-types` | 共享读写 DTO、状态、事件和拒绝码；不依赖 Session、Tauri 或 HTTP handler |
| `soul-api` | 版本化路由、鉴权、Host/Origin、有界输入输出、HTTP 状态映射及派发操作的退出结算；无领域规则 |
| `src-tauri` | 构造/持有 runtime，commands 参数转换与调用，原生窗口/托盘和事件桥接 |

避免依赖环：types 不依赖 core；core 如需共享外部 DTO，只单向依赖 types；runtime 依赖 core/types；两个传输都依赖 runtime/types。只有需要独立部署时才加 headless host，不为目录完整先创建空 crate。

## 接口分片

以下路由已在当前候选实现，只在显式合成启动时监听：

| 切片 | API | 行为 |
| --- | --- | --- |
| 第一期 | `GET /v1/status`、`GET /v1/people`、`GET /v1/memories/{id}` | 固定当前 Session owner，分页/限量，精确 ID，鉴权后读取；健康状态不泄漏内部路径/配置 |
| 第二期 | `POST /v1/scoped/draft`、`/propose`、`/confirm`；`POST /v1/scoped/{proposal_id}/discard`；`GET /v1/scoped/status` | 沿已有状态/来源回执和一次消费；本地生成不调用模型 |
| 第二期 | `POST /v1/imports/prepare`、`POST /v1/imports/{operation_id}/confirm`、`/discard`；`GET /v1/imports/{operation_id}` | prepare 只接受内置合成 fixture `basic`；保留提交状态、原主体幂等确认及显式结果回读 |
| 第二期 | `PATCH /v1/memories/{memory_id}`；`POST /v1/memories/{memory_id}/forget-preview`、`/forget-confirm`；`GET /v1/memories/{memory_id}/state` | Memory 编辑停用偏好；遗忘保留预览、确认、CK 销毁、清理及审计不确定状态；state 仅回报存在/墓碑状态 |
| 第二期 | `GET /v1/events/snapshot` | 返回进程 epoch、序号和在途操作元数据；HTTP 未开放 SSE/WebSocket，桌面通过 native 事件接收即时通知 |

JSON 错误外形采用 `{code, message, details}`，details 为按码定义的安全结构，不放原始异常或请求内容。业务代码不依赖 HTTP status。未知字段/枚举拒绝；缺失状态不补零。路由请求不含可切换 data-dir 的参数。

新库导入后，范围起草仍要求先在桌面档案页设置语气；导入联系人不自动创建用户档案。本片没有 HTTP 档案设置端点，不因测试便利跳过核心前提。

DTO 以 Rust 类型为源，使用 `ts-rs 10.1.0` 生成 `apps/desktop/src/generated/runtime.ts`；`utoipa 5.3.1` 从相同 DTO 导出 [OpenAPI](local-api.openapi.json)。普通测试比较输出，不新增冻结 contract。为保留 IPC 兼容，既有命令仍可返回原核心 DTO，新安全 HTTP 投影和事件使用共享类型；不宣称全部旧前端类型已迁完。

HTTP 的 `people` 默认 limit=50，允许 1–100，offset 0–1,000,000；排序后只返回节点元数据。底层仍读取完整图及证据，分页只限制输出，不承诺图计算成本已分页。Memory 要求精确的连字符 UUID；不存在或不可读统一 `read_failed`/503，不根据通用核心拒绝伪造 404。正常 JSON 读错误只含安全字段，协议解析前的畸形 HTTP 可能返回空 400/431 或关闭连接。

输入头限制 32 项/8192 字节、头读取 2 秒，响应序列化上限 1 MiB、开始写响应后 2 秒；连接和派发操作各最多 8 个。写请求须为 JSON，提供 Content-Length，最多 64 KiB、正文接收 2 秒，拒绝 chunked、未知字段及多余 query；正文验证完成才进入 runtime。无参数 POST 仍发送 `{}`。关闭先停监听，再等连接及已派发操作实际结算；客户端断连不提前释放许可或 Session 所有权。已受理的核心工作没有强制超时取消，慢工作可能延长退出。

## 合成启动与调用

普通启动不创建 API 文件或监听端口。当前只支持 `SOUL_LOCAL_API=synthetic` 和现存绝对目录 `SOUL_API_SYNTHETIC_ROOT`；每次启动在其中创建全新的 `run-…/data`、`webview` 与 `credentials.json`。不接受既有库参数、不回落平台目录；重复启动继续遵守桌面单实例，已有实例时拒绝，不自动终止或唤醒它。该模式关闭采集、目录授权/预览、任意正文或文件导入、外部模型、研究保存和自动 assistance/notification owners；导入仅可选编译进 runtime 的 `basic` 合成夹具（`fixtures/import/soul-import-v1/valid_basic.jsonl`）。

合成启动在创建线程和窗口前，将所有窗口的 `data_directory` 及本进程 `WEBVIEW2_USER_DATA_FOLDER` 指向同一新建 profile。该环境变量会覆盖 SDK 的目录参数，因此只设置 Tauri 窗口配置不足以隔离继承的旧 profile；依据见 [Microsoft WebView2 SDK](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/webview2-idl#createcorewebview2environmentwithoptions)。普通启动不改该变量，合成分支不写用户/系统环境或注册表。注入 setter 的合成测试检查配置与进程设置一致；真实 WebView 实际选用的目录仍需原生验收。

从仓库根目录可按以下方式启动（需要先由作者退出已有 Soul；本轮未执行原生窗口验收）：

```pwsh
$ErrorActionPreference = 'Stop'
if (Get-Process | Where-Object { $_.ProcessName -eq 'soul' }) {
    throw 'Exit the existing Soul instance first.'
}
$previousMode = $env:SOUL_LOCAL_API
$previousRoot = $env:SOUL_API_SYNTHETIC_ROOT
try {
    $env:SOUL_LOCAL_API = 'synthetic'
    $env:SOUL_API_SYNTHETIC_ROOT = 'E:/SoulSyntheticLab/local-api-manual'
    New-Item -ItemType Directory -Path $env:SOUL_API_SYNTHETIC_ROOT -Force | Out-Null
    pnpm dev
    if ($LASTEXITCODE -ne 0) { throw 'Synthetic local API startup failed' }
} finally {
    $env:SOUL_LOCAL_API = $previousMode
    $env:SOUL_API_SYNTHETIC_ROOT = $previousRoot
}
```

脚本只读取本次 `run-…/credentials.json` 的 `endpoint` 与 `token`，令牌放入 `Authorization: Bearer …` 请求头；不要打印凭据或放在 URL。文件在 Windows 创建时即采用受保护的当前用户专属 DACL，正常退出删除；异常退出可能遗留已失效文件，不自动清理其他运行目录。ACL 不隔离同账户恶意进程。`GET /v1/status` 的 `store_opened` 才是库健康判据，HTTP 200 不单独证明可用。每次启动用新库；关闭后同一库重开的持久性由合成集成测试覆盖，尚无产品“恢复旧 API 会话”入口。

生成入口为 `cargo run -p soul-types --example typescript --locked`、`cargo run -p soul-api --example export_openapi --locked`；输出须审核后更新生成文件。接口详情看 OpenAPI，权限与生命周期说明以本文为准。

## 授权与数据隔离

入口默认关闭；启动前要求明确的绝对合成根目录，规范化后检查范围，缺参直接拒绝，不回落真实平台目录。目录独立还不够：请求能力同时排除真实采集、任意目录读取、外部模型和研究明文保存；HTTP 导入不接收路径或任意正文。

仅绑定数字 `127.0.0.1`，严格 Host 与 Origin，不使用 wildcard CORS；无 Origin 的脚本仍须鉴权。凭据按运行期生成，在请求头传递，日志与 URL 不携带；令牌文件按 ADR-002 的有限保证实施，不把同用户 ACL 描述成同账户隔离。拒绝请求不得触发 Session 业务副作用。

主体由可信传输注入：Tauri 从实际 WebView label 构造，HTTP 从已认证入口构造；JSON 不能自报 caller，关联 ID 不授予权限。一个 bearer 对应一个 HTTP 驱动主体，不隔离多个持有者。WebView label 是当前应用生命周期内的身份，未承诺任意销毁并同名重建窗口的独立会话隔离。

核心全局提案槽保持原语义：导入已有有效准备不可被替换；新范围草稿和新遗忘预览可显式替换原槽，成功后归新主体。跨主体确认/取消先拒绝且不消费原槽；本主体仍受原过期、错误确认和一次消费规则约束。导入提交后仅在当前 runtime 保留的原主体归属下支持原幂等确认；重启后可读回提交结果，不恢复批准权。范围提案只能来自本主体当前草稿。错误统一为安全结构，`caller_mismatch`、`confirmation_unavailable` 等不回显内容。

合成驱动在明确脚本范围内操作，真实数据敏感动作仍需独立用户确认。API 鉴权不授予永久 capability；不开放任意 command/invoke/SQL/文件读写。

## 一致性与生命周期

本片导入、范围偏好、Memory 编辑/遗忘仅在 runtime 编排一次；未接入的旧操作/后台仍通过可信 Session handle，不能把本片描述为所有业务迁移完成。开始时先登记在途操作再等待 Session 锁，结算在释放锁后通知；已进入核心的失败也结算并使旧读取失效。未提交失败、已提交但清理/审计异常、取消请求、取消实际生效分别保留，`refused` 事件不证明完全没有持久变化。

通知只含 epoch、序号、操作 ID、操作类别、阶段/结果、可信主体与可选关联 ID，无正文和密钥。runtime 使用有界非阻塞订阅；native relay 转为 `soul-runtime-change`，UI 挂载时先订阅后快照，并在 focus/恢复及定时核对快照。IPC `runtime_event_snapshot` 与 HTTP 快照共享来源。序号缺口、重复、迟到和 epoch 改变均按普通状态核对处理，不新增持久 hash/baseline。

UI 仅当事件的桌面主体等于当前 WebView 且关联 ID 属于本地调用时去除自己的回声，保留既有 JS start/finally 回执。外部开始、监听未就绪或同步失败时隐藏旧正文/预览，阻止交付旧读取；结算后重读普通查询，不自动重新导入、起草、准备或确认。监听失败保持不可读并重试，明确显示同步失败。Memory、Import、Graph、范围草稿和 App 各保留地址/来源/操作归属检查，异步旧响应不能覆盖新状态。

导入准备、旧操作失效和取消分别补入 `import.prepare` / `import.discard` 审计白名单，与相应持久状态在原事务内提交；跨主体拒绝使用既有 `capability.reject`。事件有主体元数据，持久审计未增加 actor 字段。合法遗忘仍按原 CK 销毁及提交后审计回执，不增加“先审计成功才遗忘”的条件。未改数据库 schema/迁移；旧二进制不认识新审计动作，不承诺新审计链可降级回放，也不承诺遗忘可回滚。

同一个数据库只允许一个 runtime host。遵守 Session 的 canonical-path OS lease；不删除 lock 文件、不绕过单实例。原始 store open 不强制 lease，故其只维护合作式所有权，不能当作恶意进程防护。

请求/响应大小、分页、连接和队列数量、超时有限；导入沿现有业务上限，避免为所有消息硬套一个不合适的小上限。HTTP 断连不终止已派发 worker，完成后仍结算；查询已存在的 operation 结果，不无条件重试确认。

## 验收与边界

首片必须实际运行真实 loopback HTTP + 合成 Session，验证鉴权拒绝、状态不可用、精确查询、边界输入、同库占用和关闭释放。第二片验证 UI/API 交叉的提案替换、断连结算、旧读投递以及“导入→偏好→编辑停用→遗忘→退出重启”。两片均独立评审后才按运行结果记录。

剪贴板、原生文件选择器、托盘和安装由作者手动验收，API 测试不能替代；当前既有未完成项保留。远程/多用户、生产真实数据、外部模型及被暂停的研究不随架构重构开启。
