MODEL: Cursor Grok 4.6

# ST-V3 文件计划页深化 —— 完成报告

分支 `agent/dev-sota`。glob 内文件；共享文件未碰。

## 1. 结论

`/files` 是只读预览：视图前后磁盘快照（相对路径、字节、内容、mtime）全等；`written_to_disk` 构造级 false；拒绝矩阵含父目录与 Unix 逃出 symlink，message 不把被拒路径列为 roots；注入文件名可上屏、不上审计链。页面建议行内零交互。

## 2. 改动

| 文件 | 改动 |
|---|---|
| `crates/soulcore/tests/fileplan_view.rs` | +4 条：磁盘不变、父目录/symlink 拒绝、注入文件名、written_to_disk 源码级 |
| `apps/desktop/src/routes/Files.tsx` | 建议行 `data-testid=fileplan-row`，供行内零控件断言 |
| `apps/desktop/src/routes/Files.test.tsx` | 新建：空 roots 指路、扫描出表、拒绝上屏、无执行按钮 |

未改 `commands/fileplan.rs`。父目录拒绝的「不回显」不能用裸 substring：授权根 `/tmp/x/a` 包含父路径 `/tmp/x` 作为前缀，改成断言父路径没有作为独立 root 出现在括号列表里。

带 `/` 的注入文件名不能 `join` 成单个文件，本包种了语料里两条合法 Unix 文件名。

## 3. 绿灯

- `cargo test -p soulcore --test fileplan_view` 9/9
- `pnpm --filter @soul/desktop lint` / `test`（含 Files.test.tsx）
