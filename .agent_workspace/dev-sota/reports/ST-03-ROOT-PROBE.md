MODEL: gpt-5.6-sol-xhigh-fast

# ST-03 `authorize_root` 只读审查

审查对象：`crates/soulcore/src/commands/shell.rs` 及
`crates/soulcore/tests/shell_commands.rs`、`apps/desktop/src-tauri/tests/ipc_roundtrip.rs`。
审查版本：`agent/dev-sota`，`2920d2f`。

## 结论

| 项目 | 结论 | 证据 |
|---|---|---|
| `canonicalize` | **有** | `shell.rs:312-317` 在入表和去重前调用 `std::fs::canonicalize`；`shell_commands.rs:333-338` 和 IPC 测试 `ipc_roundtrip.rs:189-215` 都断言读回的是 canonical path |
| `Path::starts_with` | **没有** | `authorize_root` 用 `config.authorized_roots.contains(&canonical)` 做完整 `PathBuf` 相等比较（`shell.rs:319`），相关源码和测试均未调用 `Path::starts_with` |
| 相对路径 | **会接受（只要相对当前进程工作目录存在且是目录）** | trim 后直接 `metadata(Path::new(requested))`，没有 `Path::is_absolute`；随后 `canonicalize` 会把相对路径解析为绝对路径 |
| 不存在 | **拒绝** | `metadata` 的 `NotFound` 映射为 `RootRefusedReason::NotFound`（`shell.rs:291-303`）；核心与 IPC 均有测试 |
| 文件 | **拒绝** | `metadata.is_dir() == false` 映射为 `NotADirectory`（`shell.rs:305-310`）；核心与 IPC 均用真实临时文件测试 |
| 重复 | **拒绝同一 canonical path** | canonicalize 后用 `contains` 精确去重并返回 `AlreadyAuthorized`；测试覆盖尾随 `/`、`/.`、`/../leaf`、首尾空白四种别名，并断言列表仍只有一项 |

## 发现

### [中] 相对路径授权取决于进程 CWD，核心没有落实“完整路径”

拒绝空值时的提示要求用户填写“完整路径”，但核心没有绝对路径检查。输入 `.`、`..` 或
`relative/dir` 时，只要它相对于桌面进程的当前工作目录存在，就会被授权并存成其 canonical
绝对路径。桌面应用的 CWD 可随启动方式变化，因此用户输入的同一字符串可能授权不同目录。

现有测试没有覆盖该行为。重复测试中的 `.../../leaf` 是从临时目录绝对路径拼出的、含 `..`
的绝对路径，不是相对路径测试。

若契约要求只接收完整/绝对路径，应在访问文件系统前检查 `Path::is_absolute`，并补充“存在的
相对目录也必须拒绝”的核心测试及 IPC 测试。若产品明确允许相对路径，则应改文案并把 CWD
解析语义写成测试。`TASK_SPLIT.md` 的 ST-03 明文验收只要求“存在、是目录、canonicalize、
去重”，未明确要求拒绝相对路径；因此这是契约/安全语义缺口，不是该条文字验收的直接失败。

### [低] canonicalize 的符号链接去重承诺缺少直接测试

`shell.rs:273-276` 的注释明确把 symbolic link 列为同目录的第二种拼法，实际实现也会通过
canonicalize 正确解析链接；但相关测试只覆盖词法别名，没有创建符号链接验证。平台允许时应
补一个“真实目录 + 指向它的 symlink，第二次授权返回 `AlreadyAuthorized`”用例。

另一个边界是：去重只会 canonicalize 新请求，不会重新 canonicalize
`Config.authorized_roots` 中既有项。当前会话从空配置开始且写入口统一经过本函数，当前路径
成立；未来若 WP13 直接反序列化非 canonical roots，重复不变量需在加载边界重新建立。

## `Path::starts_with` 判断

这里没有 `Path::starts_with` 本身不是 ST-03 去重缺陷：授权的是“根本身”，同一根应按
canonical path 完整相等；若用 `starts_with` 去重，会把合法的父根/子根误判为重复。

组件级 `canonical_target.starts_with(canonical_root)` 应用于后续“请求目标是否位于授权根内”
的扫描授权判定，而不是本函数的根列表精确去重。`authorize_root` 本身不提供目标包含关系
校验；不能把它当作 AC-18 的路径授权检查器。

## 测试核验

执行：

```text
cargo test -p soulcore --test shell_commands
```

结果：`22 passed; 0 failed`。

除本报告外未修改代码，未 commit。
