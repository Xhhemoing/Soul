# Round X · opus-a 加轮复核

MODEL_SLUG: claude-opus-5-thinking-high-fast
对象：`docs/BLOCKERS.md`（含 R3 修订：S1 已落地 / G1+ 升 P0 / M3 不采纳 `fc96e46` / S2 改依据）× 实时 `origin/cursor/soul-goal1-7b1c` @ `2e72ddf`（17:05:02 UTC，`git ls-remote` 核对，未再前进）。

**FREEZE_OK — 无过期 P0。** 逐条在尖端复验：M1 PR #4 仍 OPEN；M2 `origin/main` 不是尖端祖先、PR #2 仍 CONFLICTING+draft；M3 `unauthorized_paths.rs:128` 仍是无条件 `corpus.len() >= 25`、`:236` / `:278` 仍按 `cfg!(windows)` 选探针（父代理不采纳的那两处原样在树上）；G1 `soul-graph/src/build.rs` 仍 3/10/3 无 180；G1+ `soul-import/src/commit.rs:198` owner 群消息仍扇给该会话全部历史发言人；G2 `soul-profile/src/service.rs` intake 仍直接 `place_axis` 不查轴锁；G3 `build.rs:322` 仍写 `UserVerdict::Unreviewed`；G5 残余成立——`COMMAND_NAMES` 27 个仍无 `import_*`，`collect_enabled` 在 Home/Wizard 只读展示、无开关命令。

一处非 P0 增量（不改冻结，仅供记账）：§4 S3（AC-21，P2）已在 `2e72ddf` 落地——`headless.rs:220` 起 `netwatch::Watch`，`egress_findings` 已对 `non_loopback_connections == 0` 做 `require`。「`headless` 没拿 `observed` 做 require、补一行即可」这句现在过期；S3 可标已落地，剩 Windows 上 `/proc` 不存在时走 `Unsupported` 的观测洞。

不建议为此重开一轮：S3 是 P2，不挡合入也不挡关闭。§5 工序表零改动。
