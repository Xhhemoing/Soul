# 2026-10-22 GitHub 开发分支同步

日期与时区：2026-10-22，America/Los_Angeles。执行者：主会话 Codex；独立只读范围核验：`sync_review`（`gpt-6.1-sol/high`）。

作者明确要求将最新项目同步到 GitHub。本次同步现有 `codex/soul-rebuild-20261019` 分支，保存源码、项目配置、合成 fixtures、设计和评审记录；不将分支同步解释为合入 main、发布版本或正式平台验收。

## 候选与证据位置

- 开工 HEAD：`6a56d852d23a051c7c4e9845642791295c541105`，dirty，初始暂存区为空。
- `git fetch origin` 成功后，远端同名分支比本地少 25 个提交，无远端独有提交。
- 初始候选共 224 个路径，219 个现存文件及 5 个历史文档旧位置删除；5 个旧文档均有迁入位置。最终候选以本同步提交的文件树为准。
- 远端：`https://github.com/Xhhemoing/Soul.git`。
- 本机持久证据：`E:/SoulSyntheticLab/github-sync-20261022/`。`initial-status.txt`、`initial-paths.txt`、`task.txt`、`fetch.log`、`branch-relation.txt` 保存开工范围与关系；逐项日志、命令退出码、只读评审、提交与推送回读保存在同一目录。本机原始日志未随仓库发布。
- 开工最初一组读取命令未保存完整原始输出，`task.txt` 明确只保留安全元数据摘要；不补造原始日志。

## 本次验证

验证仅使用合成 fixtures、临时测试库和本地 mock。Rust 使用锁文件及离线依赖；Python 只运行测试，不执行研究 runner 或模型请求。根测试排除 `windows::runs_on_windows`；桌面测试须排除两项真实采集用例。默认 ignored 诊断保持未运行。

环境：Windows、PowerShell 7.6.5、Git 2.54.0.windows.1、Rust 1.83.0、pnpm 10.15.0。以下数量来自本次实际输出，历史设计中的通过数量不作为本次证据。

| 实际命令 | 结果 / 退出码 | 本机日志 |
| --- | --- | --- |
| `git fetch origin`；`git rev-list --left-right --count origin/codex/soul-rebuild-20261019...HEAD` | 0；`0 25`，无分叉 | `fetch.log`、`branch-relation.txt` |
| `git diff --check` | 0；保留 PRODUCT_LOCK 的 CRLF 提示 | `diff-check.log` |
| `cargo fmt --all -- --check` | 0 | `root-format-repeat.log` |
| `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --all -- --check` | 0 | `desktop-format-repeat.log` |
| `pnpm --filter @soul/desktop lint` | 0 | `frontend-lint.log` |
| `python -B -m unittest discover -s scripts/tests -p 'test_*.py'` | 21 通过，0 | `python-tests.log` |
| `cargo test --workspace --locked --offline -- --skip windows::runs_on_windows` | **未完成，101**：编译阶段先出现 `memory allocation of 3276816 bytes failed`，随后有 `std` 类型识别及连锁错误；不计为测试通过 | `root-test.log` |
| `cargo test -p soulcore --test open_task_probe --test task_requirements_probe --test task_context --test session_communication_intent --locked --offline -j 1` | 降低并发后的定向复验：56 通过、4 默认 ignored，0；不替代失败的根全套检查 | `root-focused-test.log` |
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked --offline -j 1 -- --skip collection_can_be_granted_and_taken_back_over_the_ipc --skip a_restart_finds_collection_off_again` | 独立桌面 workspace：134 通过、2 真实采集 filtered，0 | `desktop-test.log` |
| `pnpm --filter @soul/desktop test` | **702 通过、1 失败，1**；58 个文件中 1 失败 | `frontend-test.log` |
| `pnpm --filter @soul/desktop test src/routes/Graph.evidence.test.tsx` | 独立复跑：20 通过，0；不覆盖首次失败 | `frontend-graph-retry.log` |

前端首次失败位于 `Graph.evidence.test.tsx:38`，等待名称为“核对此联系人的证据条目（元数据）”的 link 时未找到；当时根 workspace 正在并行编译并随后发生内存分配失败。单文件复跑通过仅表明未持续复现，未确定该前端失败的根因，也没有修改业务代码或放宽超时。

记录保留两次辅助检查失败：编译结束后进程查询因 cargo/rustc/link 均已退出而返回 1；日志定位检查发现首次零输出的格式检查未经空管道创建日志文件而返回 1。首次格式退出码 0 已在 `commands.txt` 保存，随后用显式 UTF-8 `Out-File` 重跑并保留零输出文件；详情见 `diagnostics.txt`。这些不是产品测试失败或新功能修复。

暂存前的路径核对另一次返回 1：并行研究会话新增了 `docs/reviews/2026-10-22-open-task-flow.md`。该次未修改暂存区，新增评审与更新后的研究设计随后交同一评审者定向核对，再纳入同步范围；没有遗漏或还原另一会话的文档。

首次 `git diff --cached --check` 返回 2：此前未跟踪的 `2026-10-21-algorithm-architecture-optimization.md` 末尾有一处多余空行。本次仅删除该终端空行，保留正文；首次输出保存在 `staged-diff-check.log`，修后复验另存 `staged-diff-check-retry.log`。本次没有修改业务源码。

独立只读评审 `review.txt` 对初始 219 个现存文件检查类型、大小、常见凭据字面量及文档迁移；未发现具体上传阻塞。53 份候选 Markdown 的可解析相对文件链接缺失目标为 0，全部锚点、远程页面和仓库外原始材料未全面核对。凭据模式扫描的一处 `task-...` 路径误报已定向排除。评审不替代业务语义或平台验收。

## 结论边界与后续

同步期间，另一会话续写了开放任务研究的运行、失败和未知费用记录，生产源码未变化；该研究结论以[所属设计](2026-10-22-open-task-flow.md)及其独立评审为准。本次只保存其已有源码与材料，没有自行调用模型、重算账本或为其补签研究验收。Goal 1、U7、正式 Goal 2、Wave 1 及 G-L/G-W/G-M 的原状态保持。完整 G-L 继续暂缓，完整 G-W/G-M、安装卸载和外部模型实验不属于本次同步验证。

记录规则首次周整理到期日为 2026-10-29，本次尚未到期；历史存量未全面盘点的限制保留。本次不迁移或删除仓库外原始证据。
