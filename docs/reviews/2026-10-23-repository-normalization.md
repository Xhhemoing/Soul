# 仓库规范化 N1 独立只读评审

日期：2026-10-23，America/Los_Angeles。评审者：独立 reviewer agent（`gpt-6.1-sol/high`）。结论：本次认领的文档与 CI 配置候选未发现具体阻塞；这是静态范围的独立审查，不是产品、平台或合并批准。

## 候选与范围

分支 `codex/soul-rebuild-20261019`，HEAD `c5be3ed3aaa09b6ef5e2a8861a07d7c7c1fa8b46` 加既有 D109 与本次规范化 dirty；不能仅用该 HEAD 代指未提交候选。先读项目约束、任务及实际 diff，再看实现日志。起点依据为仓库外 `initial-status.txt`、`initial-candidate.patch`、AGENTS/README 及相关文档副本，既有产品实现不属于本次 N1 审查。

本次覆盖 AGENTS、README、PR 模板、CI workflow、operations 两页、ADR-001/002、local-api 架构、规范化计划、product 导航，以及 STATUS、PLAN_INDEX、DECISIONS 的本轮增量、SECURITY/ARCHITECTURE/gates 入口、v1.3 导航与两份 archive。`justfile` 与 `gate-win.ps1` 只核对过期 CI 注释的实际 diff；没有重审全库或运行产品测试。

原始评审及失败记录保存在本机 `E:/SoulSyntheticLab/repository-normalization-20261023/normalization-review.log`，未随仓库发布；任务及其他执行证据见[规范化计划](../plans/2026-10-23-repository-normalization.md)。

## 发现与证据

| 分类 | 结论及依据 |
| --- | --- |
| 阻塞项 | 未发现可定位的本次引入缺陷。D110 明确替代 hosted CI 禁止与普通合并的逐 SHA 本地记录规则，同时保留正式平台、Goal、研究暂停、合成范围和真实采集排除；未把本地检查或 workflow 文件存在记成远端通过。 |
| 非阻塞项 | 未发现需列为缺陷的具体问题。双 Cargo workspace、Rust 1.83、pnpm/锁文件、原生命令失败传播、只读 PR token、checkout 不留凭据均有明确配置；没有新增宽泛 allowlist、hash 或冻结机制。 |
| 历史保留，实际静态核对 | 两份 archive 去除新增快照说明后，将相对 Markdown 链接规范化到原/新实际目标路径，分别与起点副本逐字符比较相同；不只借用 `archive-preservation.json`。原失败、未知费用、暂停和未完成 Windows 观察保留，没有将历史测试数量用作当前结果。 |
| 入口与状态，实际静态核对 | STATUS 为 32 行。14 个本次入口/快照文件的本地 Markdown 文件目标检查缺失为 0；此项未验证所有锚点或外部 URL。原 v1.3 路径保留已有四个引用标题并导向原文。 |
| API 方案，源码及文本推断 | ADR-002/local-api 明确新 runtime/types/API 尚未实现；记录了同账户令牌局限、合成根、Host/Origin、主体归属、一次消费、在途旧读失效、断连结算和人工平台边界。设计未冒充运行结果。 |
| 本地语法证据 | reviewer 独立解析 workflow 的 13 个 PowerShell run 块，语法和首行 ErrorActionPreference 通过，修正后脚本退出 0；未执行块内命令。首次提取 regex 将后续 YAML 误纳入 PowerShell，退出 1，已保留并修正，不算 workflow 缺陷。 |
| worker 证据核对 | 当前 workflow 与 `ci-candidate-validated.yml` 逐字符相同。原始 worker 记录显示现有 YAML 2.9.0 严格解析退出 0、warnings 0，当前 Tauri 2.11.4 `build --help` 退出 0；首轮 node 参数转义退出 1 已保留。这些仅证明配置解析/帮助语义，未证明 hosted 构建。 |

README 的独立合成目录、WebView profile 与环境变量 `finally` 恢复符合本次文档目的；单实例与退出提示与开库前单实例处理的源码一致。没有读取真实用户数据库、聊天、凭据或正文日志。

## 未验证与范围判断

reviewer 未执行构建、测试、依赖安装、服务启动或远端写操作。固定 cargo-deny 0.18.2 对当前 advisory DB 的兼容、Windows runner 前置工具与时间预算、实际 Tauri 编译及 Cargo 参数执行、fork PR/push 的实际 checkout SHA、远端 Actions 开启/分支保护，以及真实 Windows 窗口、托盘、剪贴板、安装和 API 行为仍未验证。主线程后续实际检查独立记录，不由本评审补签。

后续已获授权的首次 hosted 运行应记录候选 SHA、事件/merge ref、实际 checkout、run 链接、各步骤失败/取消与 filtered/ignored 范围；本阶段没有这些运行记录时保持未运行，不借用旧绿灯。未来 API 验收按 ADR-002/架构执行，不把未来产品扩展作为本次文档候选阻塞。

结论符合 N1 文档/CI 配置范围；不签发 G-L/G-W/G-M、Goal 关闭或合并批准。评审仅获准新增本记录与追加仓库外日志，其余工作树、暂存区、HEAD、分支和配置保持只读。相同候选与覆盖面没有新发现后停止检查。

末次定向核对仅覆盖三处措辞：AGENTS 与 operations 将新归档统一至 `docs/archive/` 且保留既有 history，CI 操作说明将分支保护/required checks 按本次方向、候选审阅同步及实际 hosted 结果推进并明确当前未改远端；未发现与已审范围冲突，无新增阻塞，主线程后续本地检查仍由主线程记录。
