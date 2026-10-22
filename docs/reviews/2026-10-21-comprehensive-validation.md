# 综合测试与最小修复独立核验

日期：2026-10-21。候选：`codex/soul-rebuild-20261019 @ 6a56d852d23a051c7c4e9845642791295c541105 + dirty`。范围与实际命令见[综合测试分析](../design/2026-10-21-comprehensive-validation.md)，当前项目状态只维护在 [STATUS](../STATUS.md)。

独立核验：`program_test_lead`、`algorithm_test_lead`、`architecture_test_lead`，均为 `gpt-6.1-sol/high`；单文件修复由另一名 `desktop_lint_fix`（相同模型/推理级别）实施。主会话统一执行构建/测试并整理本回执。各核验者没有修改候选或自行重跑完整检查。

## 核验范围与结论

程序负责人先读取实际 diff 和仓库外修改前快照，再核对实现说明。`desktop-lib-before-lint.rs` 到最终 `apps/desktop/src-tauri/src/lib.rs` 仅移除 `window.eval(&format!(…))` 的多余 `&` 并调整该表达式换行；`desktop-lib-before-format.rs` 到最终仅为换行。对应非空白比较均一致，路由字符串、UUID 类型、窗口选择、调用顺序、错误处理不变。已有研究保存命令注册三行在修改前已存在，没有被归入本轮修复。

本地 Tauri 2.11.5 的 `eval(&self, js: impl Into<String>)` 可直接接收格式化字符串。通知回调仍限定 `AssistanceNotificationId = uuid::Uuid`，未放宽到任意源文本或 URL，未新增 allow、依赖或 lint 降级。独立静态评审无剩余具体发现。

三条测试线分别核对根程序、算法/VAL、桌面/前端/读取器原始结果。程序负责人另独立比对构建包装实验的实际 AST 函数与六条子进程日志，并对最终失败/修复日志及综合结论作定向复核。未重复全库扫描或以历史通过替代本轮结果。

## 最终原始证据

原始目录：`C:/Users/86080/AppData/Local/Temp/soul-comprehensive-8fba6660c53343d9ba4c0f06ad43743b/`。

| 文件与行 | 核验结果 |
| --- | --- |
| `root-tests.log:2602`、`root-doc-tests.log:119` | 均 exit 0；179 个顶层 binary 的 1490 个通过与逐 test 行计数一致，doc 仅 1 个实际用例 |
| `desktop-clippy.log:301` | 首次 exit 101，实际 `needless_borrows_for_generic_args`；不能归类为环境失败 |
| `desktop-clippy-recheck.log:5` | 同一完整严格命令修后 exit 0 |
| `desktop-tests-recheck.log:177` | 7 个 binary 合计 130 passed / 0 failed / 0 ignored / 2 filtered；exit 0 |
| `core-production-check.log:16`、`root-format.log:3` | 普通核心库编译、根格式均 exit 0 |
| `desktop-format.log:15` | 首次 exit 1，仅要求同一表达式换行；与最终修正相符 |
| `desktop-format-recheck.log:3`、`desktop-clippy-final.log:5` | 最终格式源码的格式/严格 Clippy 均 exit 0 |
| `execution.jsonl` | 19 条回执、17 成功、2 首次失败；逐条日志末尾 EXIT 与回执一致，无缺失 |

常规入口总计 2313 = 1490 + 1 + 130 + 685 + 7；12 ignored / 5 filtered。初次与修后桌面 130 没有重复累计。最终仅换行后未再运行桌面 130；根 1490 未因桌面修复重跑，综合分析与 STATUS 对这些时间及范围关系记载准确。

独立核验用 PowerShell 文本读取、日志解析和差异比较，核验脚本成功退出。两次 `git diff --no-index` 的退出码 1 表示预期文件差异，不是测试失败。上述检查均为只读，未自行启动构建或测试。

## 保留边界

综合报告未把规则/管线正确性写成真人效用，未把 dispatch 前零出站扩大到已颁发许可的在途请求，未把历史 T0 oracle 写成 T4D oracle，也未把前端精简总计伪造为逐 suite 日志。

`just ui-build` 环境风险仍为未复现的静态发现；完整 G-W/G-L/G-M、真实 WebView/通知、安装、真实采集、物理断电、生产恢复 N3、完整 ZIP 发布与采集停止并发、长期增长及真人效用均未转记为通过。本次结论为所列 Windows 合成检查和最小修复核验完成，不签发正式 GATED/INTEGRATED 或 Goal 关闭。
