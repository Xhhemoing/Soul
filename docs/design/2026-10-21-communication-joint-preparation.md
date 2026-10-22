# 沟通意图与证据共同准备：实施与验证

日期：2026-10-21。起点：`codex/soul-rebuild-20261019` @ `6a56d852d23a051c7c4e9845642791295c541105` + dirty，保留已有修改。本轮用户已明确授权按首批诊断顺序实施，先完成共同准备及独立评审，再推进检索。当前事实仍由 [STATUS](../STATUS.md) 维护。

**目标：** 让绑定人物观察的本人目标，在本次独立明确选择后，与统计证据进入同一份脱敏预览、同一单次批准及来源复验链。

**架构：** 复用已保存 ManualGoal 的 Commitment Memory 身份、分类和精确快照；将可选目标附于现有 Insight context。仅扩展现有准备和派发，不开放任意 memory/insight 合并，不新增存储 schema、hash、冻结规范、门禁或后台出网。

**实现环境：** Rust 1.83，现有 Tauri IPC、React/TypeScript、Vitest、SQLCipher 与 loopback MockLlm。单写者顺序改核心再接桌面；主线程持有唯一构建通道，独立评审只读。

## 已确认的边界与取舍

首批证据见[材料诊断](2026-10-21-communication-material-diagnosis.md)。现有 redactor 根据密封主体分类处理内容，不是语义分类器。任意新 `intent: String` 一律标 Owner 会扩大第三人正文用途，全部标 Mixed 又无法传达意图；固定场景枚举限制表达能力。因此本片复用已有目标的分类和独立 OwnerExcerpt 选择，不新增自由临时输入框。

- 三类场景通过不同目标文本表达，核心不硬编码场景或心理推断。
- 目标选择与匿名计数选择默认 false、相互独立。粘贴原文的二次确认只豁免 paste，不扩展到目标。
- 只接受当前 context ID 与 `include_goal`，不接受前端回传目标正文、任意 memory ID 或主体分类。非 ManualGoal context、不可用或非 Owner 的目标不能通过该选项纳入。
- 使用已有事项编辑入口会修改持久目标，不称为仅本次临时修改。编辑、取消、遗忘及来源变化继续使旧准备失效。
- 目标材料是引用数据；system 指令固定。引用分别说明目标／统计出处，不给整篇生成内容背书。审计只记录既有事件，不带正文。
- 完整实际 JSON 继续受既有 64 KiB 请求预算约束；共同准备超限整次拒绝，不删掉目标后假称成功。
- 仅合成临时库和 loopback。不读取真实数据、不采集、不调用生产模型、不发送消息、不安装、不提交或推送。

## 核心实现步骤

文件：`session_assistance.rs` 负责目标选择；`session_insight.rs` 持有复合上下文；`session_memory.rs` 复用材料构造；`draft.rs` 负责脱敏组合与预算；`session.rs` 负责派发／完成依赖。均位于 `crates/soulcore/src/commands/`。

1. 新增 `tests/session_communication_intent.rs` 及必要模块，先运行失败检查。使用三场景的已保存目标和真实统计 fixture，期望目标／计数独立选择、实际请求等于预览、默认不纳入。新接口尚不存在的编译失败与行为失败分别记录，不冒称已通过运行断言。
2. 在 `AssistanceContextView::Insight` 增加 `goal_preview: Option<MemorySelectionPreview>`；通过绑定的唯一 goal memory ID 调用 `soul_memory::select_for_draft`，query=None、Commitment、Unspecified、既有限额。不得调用会清除其他上下文的公开 `prepare_memory_context` 来拼装。
3. `HeldInsight` 保持目标的精确 `ContextSelection`；`PendingInsight` 仅保存实际纳入的目标依赖。保留旧入口默认 false，增加明确入口：

   ```rust
   prepare_draft_with_insight_and_goal(
       pasted: &str, context_id: &str,
       include_counts: bool, include_original: bool, include_goal: bool,
   ) -> Result<InsightDraftPlan, SessionRefusal>
   ```

4. 复用 memory `material` 的精确 OwnerExcerpt 检查、`redact_memory_context` 与统计脱敏工厂，形成一个实际 `RedactedBody`；返回 `includes_goal`、实际目标预览及既有 `body_preview`。准备失败不保留可批准的旧内容。
5. 内部 Insight 派发结果携带选中的统计和可选目标。保留独立 memory pending 与 insight pending 同时存在的拒绝；仅把已核验绑定目标映射进现有 worker 的 memory 依赖，以复用完成期精确内容复验。不能仅依赖事项 revision。
6. 运行新测试及既有 assistance／memory／insight／policy／egress 相关回归；按实际失败最小修复。

## 桌面接线步骤

核心协议稳定后再修改 `apps/desktop/src-tauri/src/commands.rs`、`apps/desktop/src/core.ts`、`routes/Draft.tsx`、`routes/DraftAssistance.tsx`、`routes/DraftInsights.tsx`，以及对应测试。共享 IPC、授权与生命周期变更串行。

- 既有 `prepare_draft_with_insight` IPC 增加可选 `include_goal`，缺省 false；不新增隐式批准入口。
- `InsightTask.include_goal` 默认 false。仅当前 assistance 的 goal preview 为 Owner 时提供独立许可框，目标／计数均不因重新核对继承许可。
- 最终端点预览使用核心返回的实际材料，不用初始本地目标内容代替实际出网预览。
- 复用已有 `busy/hasQuote/owns`、来源版本和 request generation；旧 DOM、异步迟到结果、目标编辑和路由切换不能恢复已失效许可。
- 前端测试先验证独立许可及失效，再接实现。真实 Tauri command 测试覆盖参数缺省、明确纳入、恶意无目标调用和实际请求；不以 mock wrapper 替代核心行为。

## 合成验收断言

| 检查 | 必须观察到的行为 |
| --- | --- |
| 协调、婉拒、关系跟进 | 三种自由目标及约束明确纳入后位于请求数据段；同一统计依据不吞掉不同意图 |
| 四种选择组合 | goal/count 各自关闭时相应正文／计数不出网，开启只带获准类别 |
| 分类及脱敏 | 已知姓名／账号擦除；Mixed、ThirdParty、Unknown 不得升级 Owner；原文豁免不扩目标 |
| 真实批准链 | 准备零请求；准确批准一次请求，预览与实际一致；旧批准、取消和重放拒绝 |
| 精确来源依赖 | 目标版本、解密内容、CK、分类、固定观察或 bundle 变化在相应阶段拒绝；已发请求先审计，再拒收迟到结果 |
| 大材料 | 统一请求预算超限拒绝且旧准备不可批准；不默默截掉意图 |
| 桌面生命周期 | 新选择默认关闭；目标编辑、导航、准备迟到和旧 DOM 不复活旧准备；物理在途槽不提前释放 |

运行日志与本轮前文件副本放仓库外 `C:\Users\86080\AppData\Local\Temp\soul-joint-preparation-20261021-93c6cf03`，仅用于本轮差异与执行回执，不新增常驻流程系统。主线程记录命令、版本、退出码与未覆盖项；独立评审先看实际差异和任务，再看实现说明，完成修复复验后才开始检索生产变更。

## 检索后续范围

共同准备通过本片验收与独立评审后，沿首批反例比较有界召回方案；具体实现以新增对照结果及现有算法权威为依据。当前不提前更改关键词规则、来源上限或评分，不将共同准备通过当成检索改善。

## 核心阶段运行回执

日志目录为上文仓库外目录。本轮生产改动限于上述核心与桌面准备链；保留开工前的其他 dirty 修改。本节保留首次失败，不用最终通过覆盖失败原因。

| 记录 | 实际结果与解释 |
| --- | --- |
| `intent-20261021-162549.log` | exit 101；新方法及 `goal_preview/includes_goal` 字段不存在的编译 RED，不是运行断言失败 |
| `intent-20261021-162832.log` | 5 passed、3 failed；测试错误地假定 `put_memory` 不推进事项 revision，以及把未登记 sender ID 当已知姓名。合法主体封装、当前 revision 与显示名注册修正后重跑；未因此放宽生产授权 |
| `intent-20261021-163229.log` | 8 passed、2 failed；独立评审的两项反例实际复现：>16 KiB 目标阻断 counts-only；实际纳入目标遗漏注入标记计数审计 |
| `intent-20261021-163344.log` | 10 passed、0 failed，exit 0；修正为本地 excluded preview 与可纳入快照分开，只扫描实际纳入的目标，原反例断言通过 |
| `frontend_intent-20261021-163121.log` | 12 项中 11 failed、1 passed；新增第五参数及目标控件尚未接线的行为 RED。Vitest 已实际执行；pnpm 尾部的 not found 文案不代表未安装或未运行，不据此安装依赖 |
| `ipc_intent-20261021-163438.log` | 1 passed、2 failed、69 filtered，exit 101；真实 handler 尚未转发目标许可，实际预览未纳入目标、伪造无目标请求也未拒绝 |
| `core-20261021-163610.log` | 五个相关 crate 合计 760 passed、0 failed、12 ignored，exit 0；包含新增核心 10 项，不能再将定向通过重复相加 |
| `frontend_intent-20261021-164315.log` | 2 文件、13 passed，exit 0；新增超限目标交互后由首轮 12 项增至 13 项；最终计数以完整前端回归为准 |
| `desktop-20261021-164531.log` | 133 passed、0 failed、2 filtered，exit 0；三个新 IPC 用例均通过，排除两项真实采集 |
| `frontend-20261021-164613.log` | 57 文件、698 passed，exit 0；包含新增 13 项，不重复累计 |
| `lint-20261021-164845.log`、`build-20261021-164915.log` | 前端类型／lint 及 production build 均 exit 0 |
| `clippy-20261021-164930.log` | exit 101；新增目标预览使 `AssistanceContextView` 触发 `large_enum_variant`，需内部间接存储修正，不通过添加 lint 豁免忽略问题 |
| `clippy-20261021-165102.log` | 改为 `Option<Box<MemorySelectionPreview>>` 后 exit 0；serde 对象／null 形状不变，测试 helper 机械解包，断言不变 |
| `core-20261021-165129.log`、`desktop-20261021-165524.log` | Box 后重新执行，分别 760 passed／12 ignored 和 133 passed／2 filtered，均 exit 0；前次运行不重复累计 |
| `desktop_clippy-20261021-165621.log`、`format-20261021-165654.log` | 独立桌面严格 Clippy、根格式均 exit 0 |
| `desktop_format-20261021-165703.log` → `desktop_format-20261021-165736.log` | 新增 IPC 测试两处纯格式差异，首轮 exit 1；仅调整缩进与换行后原命令 exit 0，没有改变断言或功能源码 |

精确内容复验测试通过测试专用 SQL 对临时库替换合法密封摘要，并同时证明新摘要可读、事项 revision 未变；这模拟绕过领域写入的存储变动，不把合法 `put_memory` 误写为不推进版本。非 Owner 测试覆盖 Mixed、ThirdParty、Unknown；三场景各覆盖四种目标／计数选择。目标未选时不增加目标注入标记审计，选中时仅记录计数，不记录正文。

## 执行环境与命令

Windows，PowerShell 7.6.5，Rust/Cargo 1.83.0，VS Developer Shell 17.14.40、MSVC 14.44.35207、Windows SDK 10.0.26100.0。使用既有 OpenSSL 3.6.3 静态缓存；`CARGO_BUILD_JOBS=4`、dev/test debug=0、incremental=0，仅限执行子进程，不改全局配置或 target。根测试为 optimized test profile，桌面为 unoptimized；不是性能对照。前端保留 2 workers。

主线程串行执行以下入口，实际起止／退出码由仓库外 `execution.jsonl` 记录。每条原生命令后立即检查 `$LASTEXITCODE`；失败停止该串行检查，不静默更新锁文件。生产构建临时设 `NODE_ENV=production`，在 finally 恢复。

```pwsh
$ErrorActionPreference = 'Stop'
cargo test -p soulcore -p soul-policy -p soul-egress -p soul-memory -p soul-draft --locked --offline
if ($LASTEXITCODE -ne 0) { throw 'core regression failed' }
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked --offline -- --skip collection_can_be_granted_and_taken_back_over_the_ipc --skip a_restart_finds_collection_off_again
if ($LASTEXITCODE -ne 0) { throw 'synthetic desktop regression failed' }
pnpm --filter @soul/desktop test
if ($LASTEXITCODE -ne 0) { throw 'frontend tests failed' }
pnpm --filter @soul/desktop lint
if ($LASTEXITCODE -ne 0) { throw 'frontend lint failed' }
$hadNodeEnv = Test-Path Env:NODE_ENV
$previousNodeEnv = $env:NODE_ENV
try {
    $env:NODE_ENV = 'production'
    pnpm --filter @soul/desktop build
    if ($LASTEXITCODE -ne 0) { throw 'production build failed' }
} finally {
    if ($hadNodeEnv) { $env:NODE_ENV = $previousNodeEnv }
    else { Remove-Item Env:NODE_ENV -ErrorAction SilentlyContinue }
}
cargo clippy -p soulcore --all-targets --locked --offline -- -D warnings
if ($LASTEXITCODE -ne 0) { throw 'core clippy failed' }
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings
if ($LASTEXITCODE -ne 0) { throw 'desktop clippy failed' }
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { throw 'root format failed' }
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --all -- --check
if ($LASTEXITCODE -ne 0) { throw 'desktop format failed' }
```

## 本片交付边界

本片实现了已保存、绑定观察的本人目标与统计证据的共同准备，目标须本次单独选择，最终材料经过真实脱敏与单次批准链；协调、婉拒、关系跟进各覆盖四种目标／统计组合。核心新增 10 项、桌面新增 3 项、前端新增 13 项均包含在最终相关回归中。工作区仍 dirty，未提交、未合入、未发布。

最终去重范围为相关五 crate 760 项、独立桌面 133 项、前端 698 项；保留 12 ignored／2 项真实采集 filtered。两端严格 Clippy 与格式、前端 lint 和 production build 通过。独立审查的两项功能发现均已修复，完整证据见[评审回执](../reviews/2026-10-21-communication-joint-preparation.md)。

这是材料与授权链的合成验证：未调用真实模型、读取真实资料或测量真人沟通效果；不声称本地固定模板已能根据目标生成定制回复。未做真实 Windows UI 人工观察、完整 root workspace 或正式 G-W/G-L/G-M，不关闭 Goal 1/U7/Goal 2。后续[有界检索对照](2026-10-21-bounded-communication-retrieval.md)另记其新增测试与结论，不把共同准备通过当成检索改善。
