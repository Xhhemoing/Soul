# Soul 代码检查与优化结果

日期：2026-09-27。对应[实施计划](superpowers/plans/2026-09-25-repository-optimization.md)。

本轮完成仓库级模块检查，并对有实际失败证据的问题做局部修复。它不是“所有执行路径均无缺陷”的证明，也不是 Windows 发布验收。完整安装、升级、签名和发布流程不在本报告的通过结论内。

## 检查范围与方法

- 复核时的 Git 清单为 470 个已跟踪文件，其中 Rust 275 个、TypeScript/TSX 39 个、PowerShell 7 个；覆盖根工作区 18 个 crate，以及桌面前端和构建脚本。清单数字不包含本轮尚未跟踪的新测试和本报告。
- 区分静态发现与动态证据：先用确定性的异步请求或真实 SQLCipher 故障复现，再修改生产代码，最后运行回归测试和独立代码复核。
- 不新增依赖、hash、冻结 contract、baseline 或独立 gate；保留既有安全措施。Windows 脚本仅将已有 lint/test 命令纳入现有发布检查。
- HTTP 非成功响应修复在本轮优化前已由其他任务提交；其余改动先在当前工作区完成，再纳入本地收敛分支。提交、推送和门禁结果以对应 Git SHA 的后续记录为准。

## 已修复的问题与证据

| 问题 | 修复前证据 | 本轮处理 | 代码与回归路径 |
| --- | --- | --- | --- |
| HTTP 非 2xx 响应被当作成功 | 本地真实 HTTP 响应覆盖 302、429、503 | 有界读取响应后返回既有 `HttpStatus`，错误中不回显响应正文 | [网关](../crates/soul-egress/src/lib.rs)、[线级测试](../crates/soul-egress/tests/e1_origin.rs) |
| 导入文件、格式、预览和提交可能错配 | 新增用例在修复前 6 失败、15 通过 | 将 `{format, text, preview}` 一并暂存；代次检查拒绝过期文件读取、预览及提交结果；保留取消和重试 | [Import](../apps/desktop/src/routes/Import.tsx)、[测试](../apps/desktop/src/routes/Import.test.tsx) |
| 停止采集后旧状态响应重新显示“正在采集” | 旧成功、旧失败和重复在途读取共 3 个回归失败 | 写入使旧读取失效；写入期间禁止新读取；合并重复在途状态请求 | [Collect](../apps/desktop/src/routes/Collect.tsx)、[测试](../apps/desktop/src/routes/Collect.test.tsx) |
| 人脉摘要乱序或纠正后旧摘要重新出现 | 旧成功、旧失败和关系纠正共 3 个回归失败 | 新选择、关系纠正和卸载均使旧摘要失效；纠正期间禁用摘要按钮 | [Graph](../apps/desktop/src/routes/Graph.tsx)、[测试](../apps/desktop/src/routes/Graph.test.tsx) |
| 非法数据库 nonce 导致进程 panic | 2 个真实数据库测试均在固定长度转换处 panic：`left: 0, right: 24` | 转换前检查长度；包装 nonce 返回 `Backend`，内容 nonce 返回对应 blob 的 `SealBroken` | [store](../crates/soul-store/src/store.rs)、[测试](../crates/soul-store/tests/malformed_nonce.rs) |
| 审计失败后档案/记忆写入仍部分持久化 | 4 个测试分别发现遗留的内容密钥、档案更改和密文行 | `correct_axis`、`set_voice`、`write_memory`、`edit_memory` 使用现有事务，包含最终回读；没有修改遗忘逻辑 | [Session](../crates/soulcore/src/commands/session.rs)、[测试](../crates/soulcore/tests/session_atomic_edits.rs) |
| URL 审计静默跳过不可解码源码 | UTF-16 PowerShell 样例原先得到空命中、零扫描文件的成功结果 | 读取或 UTF-8 解码失败时返回含文件路径的错误；扫描范围和白名单不变 | [扫描器](../crates/xtask/src/egress.rs)、[测试](../crates/xtask/tests/self_test.rs) |
| 前端验证入口不一致、发布产物携带源码映射 | 默认测试并发设置未集中，现有 Windows 发布检查未运行 UI lint/test | Vitest 固定 2 个 worker；关闭生产 source map；扩展现有 Windows 检查 | [Vite 配置](../apps/desktop/vite.config.ts)、[Windows 脚本](../scripts/gate-win.ps1) |

这里的性能结论限于可验证的变化：重复状态读取被合并、测试并发统一、生产 `.map` 文件移除。没有据此宣称整个应用获得了某个百分比的速度提升。

## 已完成验证

| 检查 | 结果 |
| --- | --- |
| Rust 根工作区完整测试 | 1053/1053 通过，0 失败、0 忽略；含单元、集成和文档测试 |
| Rust Clippy | 整个根工作区、所有目标通过，`-D warnings`；使用 test profile 复用原生依赖编译 |
| 前端完整测试 | 增补同文件重选回归后，14 个文件、208/208 通过（2026-09-27 本机运行）；不等于完整 G-W |
| 前端 lint | TypeScript 与 ESLint 通过 |
| 前端生产构建 | 显式 `NODE_ENV=production`：37 个模块，JS 260.80 kB / gzip 79.60 kB；产物中 0 个 `.map` 文件 |
| nonce 定向回归 | 2/2 通过；每条覆盖长度 0、23、25，恢复原 nonce 后目标密钥与密文仍可解密 |
| 普通写入原子性回归 | 4/4 通过；比较 10 张相关表的完整行值，并验证故障移除后的成功重试及审计链 |
| xtask 自测 | 34/34 通过；定向 Clippy 通过 |
| 独立前端复核 | Import 同文件重选修复经只读复核；Import 24/24、Collect 与 Graph 原有 39/39 局部验证；无本轮新增阻塞发现 |
| 独立后端复核 | nonce 长度校验、四个事务边界和扫描器改动均无阻塞发现 |
| Rust 格式与差异空白 | `cargo fmt --all -- --check`、`git diff --check` 通过 |
| PowerShell 语法 | 7 个已跟踪脚本全部解析通过 |

Rust 根工作区测试和 Clippy 全目标检查已全部通过。独立桌面 Rust 工作区、完整 Windows 发布 gate 和安装冒烟测试尚未执行。根工作区测试中对安装脚本的检查，不等于真正执行安装或卸载；test profile 的静态检查也不等于 release 构建验收。本机完整日志保留在 `target/code-review-workspace-tests.log` 与 `target/code-review-clippy.log`，不纳入 Git。

## 复现命令

从仓库根目录在 `pwsh` 中逐项执行。Rust 与 pnpm 依赖需已安装；`--offline` 不允许临时联网补齐缺失依赖。以下只运行测试、静态检查和前端构建，不执行安装、发布或数据迁移。

```powershell
$ErrorActionPreference = 'Stop'
cargo test -p soul-store --test malformed_nonce --locked --offline -j 2
if ($LASTEXITCODE -ne 0) { throw 'nonce regression failed' }
cargo test -p soulcore --test session_atomic_edits --locked --offline -j 2
if ($LASTEXITCODE -ne 0) { throw 'atomic session regression failed' }
cargo test --workspace --locked --offline -j 2
if ($LASTEXITCODE -ne 0) { throw 'workspace tests failed' }
cargo clippy --workspace --all-targets --profile test --locked --offline -j 2 -- -D warnings
if ($LASTEXITCODE -ne 0) { throw 'workspace Clippy failed' }
$previousNodeEnv = $env:NODE_ENV
try {
    $env:NODE_ENV = 'test'
    pnpm --filter @soul/desktop lint
    if ($LASTEXITCODE -ne 0) { throw 'frontend lint failed' }
    pnpm --filter @soul/desktop test
    if ($LASTEXITCODE -ne 0) { throw 'frontend tests failed' }
    $env:NODE_ENV = 'production'
    pnpm --filter @soul/desktop build
    if ($LASTEXITCODE -ne 0) { throw 'frontend build failed' }
} finally {
    $env:NODE_ENV = $previousNodeEnv
}
```

前端测试与构建若在同一 shell 内执行，不要将手动设置的 `NODE_ENV=test` 带入构建：构建前应清除此覆盖，或显式设置 `NODE_ENV=production`。本报告的产物大小来自重新执行的显式生产模式构建，不将环境切换造成的差异算成代码优化收益。

新增后端测试在临时目录内操作自己的测试数据库，不读取用户的实际数据库。审计故障使用测试专用 SQL 触发器，不向产品加入新故障注入机制。

## 未解决的风险与产品限制

| 项目 | 证据与影响 | 后续处理方向 |
| --- | --- | --- |
| 采集同意撤回与最终写入竞态 | [Collector::record](../crates/soul-collect/src/collector.rs) 在取得 sink 锁前检查同意；检查后、最终写入前仍存在撤回窗口。属于静态发现，本轮未新增并发复现测试。前端状态修复不等于修复此后端问题 | 用确定性并发测试固定竞态，统一同意检查和写入的临界区，并同时验证锁顺序与停止时限 |
| 遗忘提交后的 checkpoint/审计表达 | [checkpoint](../crates/soul-store/src/store.rs) 丢弃查询返回值；[遗忘实现](../crates/soul-store/src/forget.rs) 在销毁事务提交后执行 checkpoint；[记忆服务](../crates/soul-memory/src/service.rs) 又在销毁后写审计。失败返回可能出现在不可逆动作已经发生之后 | 明确“已销毁但后续维护/审计失败”的结果语义，再贯穿存储、命令和 UI；不能直接套用普通写入事务，因为现有要求禁止审计阻止遗忘 |
| 图重建重复线性查询 | [build.rs](../crates/soul-graph/src/build.rs) 对每个 peer 线性查找现有关系和推断，随数据增长可能接近平方级。属于静态复杂度判断，本轮未给出性能测量 | 先固定重复关系、旧数据与用户锁定的选择语义，再预建索引并做同输入前后测量 |
| 内容密钥存在性查询吞掉数据库错误 | [has_content_key](../crates/soul-store/src/store.rs) 将查询错误折叠为 `false`，无法区分不存在与读取失败 | 评估调用方和 trait 的错误表达，补故障用例后统一处理；本轮未改公共接口 |
| 重复导入 | Import UI 和既有测试明确说明 v0.1 再次导入会重复生成记录 | 属于已声明产品限制，不作为本轮缺陷“修复”；没有新增内容 hash 或去重约束 |

测试覆盖也有边界：本轮新增原子性测试直接注入的是审计失败，回读失败和 COMMIT 失败仅做了代码路径复核；新增 UI 用例未穷举所有 StrictMode/卸载调度。后续应优先处理采集撤回和遗忘结果语义，再进行图重建性能优化与完整 Windows 发布验证。

## 外部审阅对照（2026-09-27）

用户提供的《Soul 项目审阅与改进计划》审阅的是 **2026-09-12 的 `8aae8f3877bc93529d0a0104c277a85ded9c43aa`**。本节核对对象是 **`12dcb2b04b3eb3084e7f693e10c61ce3fad9b7b8` 加本报告前文列出的未提交优化**，不是已发布或已完整验收的产品版本。外部报告是待验证意见，不直接替代现有产品决定。

本次对照覆盖 F1–F8 的调用链或文档；仅 F3 的地址转换和 F5 的构建失败处理新增了动态探针。没有在本次对照中再次运行完整测试、G-W、安装或人工清单，也没有新增产品代码、依赖、schema、hash、baseline 或 gate。证据分级：`[V]` 为直接源码或运行证据，`[H]` 为依赖该证据的风险推断，`[T]` 为尚待执行的验证。

### 发现与当前证据

| 项目 | 当前判断 | 证据 → 结论 → 后续路径 |
| --- | --- | --- |
| F1：长任务占用全局会话锁 | **仍成立的结构问题；端到端时限待测** | `[V]` [桌面命令](../apps/desktop/src-tauri/src/commands.rs) 的生成、撤权、清除端点和状态查询均经过 `Mutex<Session>`；[Session 摘要](../crates/soulcore/src/commands/session.rs) 还在调用摘要生成时持有 store 锁；[发送](../crates/soul-egress/src/lib.rs) 是阻塞请求，配置超时为 120 秒。`[H]` 这些控制命令可能等待长请求。`[T]` 用慢 loopback 端点实测“点击撤权 → 最后新增事件”，不能将采集器内部停止测试当成桌面端到端证据。 |
| F2：重复导入 | **仍存在，不能用事务原子性替代幂等性** | `[V]` [commit](../crates/soul-import/src/commit.rs) 明确声明重复导入重复写事件，并为每条事件生成新 UUID；[StagedMessage](../crates/soul-import/src/model.rs) 仍将 `external_id` 标为不持久化。已有[再次导入测试](../crates/soul-import/tests/soul_import_v1.rs) 只验证联系人匹配，不能证明事件去重。修改前需确定账户作用域、来源 ID 的稳定性、历史重复数据与遗忘后重导语义；不能因 UI 已有警告就关闭数据正确性风险。 |
| F3：IPv6 origin 重建 | **已动态复现** | `[V]` [Origin](../crates/soul-policy/src/net_guard.rs) 去掉 IPv6 方括号后，`Display` 没有补回。链接当前已编译的 `soul_policy` 和 `url` 库运行探针：`http://[::1]:11434/v1` 重建为 `http://::1:11434/v1/chat/completions`，URL 解析返回 `EmptyHost`；默认端口 IPv6 同样失败，IPv4 和大小写/default-port 对照成功。当前证明的是功能错误，不是已经证明授权绕过。 |
| F4：出网后才持久化审计 | **调用顺序已确认；崩溃风险尚未注入复现** | `[V]` [PolicySession](../crates/soulcore/src/commands/policy.rs) 先 `soul_egress::send`，再返回 outcome；[Rephraser](../crates/soulcore/src/commands/draft.rs) 仅暂存 audit；Session 在生成结束后追加审计。`[H]` 请求已接收而审计尚未提交之间存在失败窗口。[已有审计崩溃测试](../crates/soul-policy/tests/audit_crash.rs) 检查 audit append 的提交前后，不等价于外部网络副作用与审计的一致性。`[T]` 待按发送前、接收后、终态写入前分别注入故障。 |
| F5：第一个构建失败被第二个成功覆盖 | **旧发现已修复，本轮故障注入符合预期** | `[V]` `git show` 确认旧提交缺少逐条退出码检查；`git blame` 确认 [当前脚本](../scripts/gate-win.ps1) 于 **2026-09-13、`c4870ecb`** 加入两条即时检查，`FailClass` 会抛出终止错误。本轮对实际提取的 Step/FailClass/release 脚本块运行三组替身构建：首个失败只调用一次并失败、第二个失败调用两次并失败、两个成功调用两次并成功。不能再把 F5 列作当前待修缺陷。 |
| F6：安全承诺与实现说明冲突 | **文档冲突仍在** | `[V]` [SECURITY](SECURITY.md) 写“防御同机非管理员进程”，[DPAPI crate](../crates/soul-win-dpapi/src/lib.rs) 则明确说明同用户进程不在其保护边界内；SECURITY 仍有“main 上没有应用代码”等合并前叙述。应区分另一账户与同用户进程，并把历史分支说明与当前实现/验收状态分开；这不等于新加密码学措施即可扩展承诺。本次未实测跨进程 DPAPI 攻击。 |
| F7：导入异步状态与输入规模 | **部分已修，剩余总字节限制** | `[V]` 前文的 [Import 修复](../apps/desktop/src/routes/Import.tsx) 已加入代次检查、格式/正文/预览绑定、busy 状态禁用及成功后清空 staged；不重复实施外部报告里的同一修复。读取仍直接调用 `file.text()`；核对 Session 的两种 preview/commit、JSONL/Telegram 解析入口及 schema，未发现整体输入字节上限。`[T]` 需确定可解释上限并同时验证读取前拒绝、核心独立拒绝和正常边界；尚未做大文件内存测量。 |
| F8：通用模型端点能力 | **限制仍在，属于需要明确支持范围的产品选择** | `[V]` [设置页](../apps/desktop/src/routes/Settings.tsx) 仅收端点地址并声明固定路径；[草稿配置](../crates/soulcore/src/commands/draft.rs) 使用 `unnamed-model`；发送路径没有 API key/Authorization 配置。现有 loopback mock 测试不能证明真实服务的普遍兼容性。近期可先如实限定受支持形态；若扩展模型名、路径和凭据，则另行设计保密存储、配置变更失效与兼容测试，不顺手扩大 origin 权限。 |

### 动态核验的范围与限制

- **F3：4 个输入。** 非默认端口 IPv6 与默认端口 IPv6 都产生无效请求 URL；IPv4 与 `HTTP://LOCALHOST:80/v1` 对照有效。探针使用 `Origin::parse` → `Display` → 固定路径拼接 → `url::Url::parse`，没有发送网络请求。首次直接 rustc 链接缺少 `windows.0.52.0.lib`，补充现有 x64 原生库搜索路径后编译、运行成功；不把首次编译失败计作产品失败。
- **F5：3 组受控构建结果。** 通过 PowerShell AST 提取当前脚本的真实函数和 release block；以调用子 `pwsh` 返回退出码的 `cargo` 替身注入 `(17,0)`、`(0,23)`、`(0,0)`，并显式使用 `$PSNativeCommandUseErrorActionPreference = $false`。核对是否抛错及实际调用次数，结果分别为 `FAIL/1`、`FAIL/2`、`OK/2`。未调用真实 Cargo release 构建，也未执行后续 manifest/安装步骤；这个测试不代表完整 G-W 或产物来源核验通过。
- **未执行：** 慢端点下真实桌面撤权计时、事件级重复导入新增断言、出网时崩溃注入、大文件测量及真实模型服务兼容测试。它们分别属于 F1、F2、F4、F7、F8 后续验证，不能从前文 1053 项 Rust/205 项前端测试推导为通过。

### 修订后的执行建议

1. **M0 缩小范围。** 不重写已修复的 F5；先更正 F6 与历史状态措辞。当前 [STATUS 顶部](STATUS.md) 已有 2026-09-25 的 `ebff0c9` Windows 证据和未完成安装说明；不能说仓库从未跑过 G-W，也不能把旧源码证据平移给本工作区。候选产物对应关系沿用已有发布记录，不默认另加 hash、冻结 contract、baseline 或 gate。
2. **M1 拆成独立工作。** IPv6 是可先做的边界修复，应加 origin 与最终 URL 一致性回归；F7 剩余字节限制也可独立推进。F2 则须先确定来源身份、历史迁移与遗忘语义；文件重复警告只能是过渡措施，不等价于消息级幂等。
3. **联合设计 F1/F4，分步验证。** 先确定短锁准备、锁外执行、短锁提交、撤权和过期结果规则，再设计发送意图/终态及“结果未知”的恢复表达。移动到后台线程本身不能关闭全局锁风险；本地数据库事务也不能回滚已发生的远程请求。没有终态不得自动重发。
4. **F8 先明确边界，再决定是否扩展。** 本轮不把“OpenAI 兼容”解释为对任意服务已有支持，也不未经确认引入 API key 管理。
5. **保留发布与阶段事实。** Goal 1 和正式 Goal 2 没有关闭；[D68 执行顺序调整](GOAL2_PLAN.md) 已允许独立质量预备支线。因此外部报告“Goal 1 关闭前完全不能做 Goal 2 相关准备”的绝对解释不适用；预备工作也不能冒充正式启动或完成验收。

以上为当前会话自审与工具核验，非新增的独立 reviewer 签署。本次没有替用户决定数据迁移、遗忘后重导、取消语义或凭据管理，也没有提交、推送或改动发布状态。
