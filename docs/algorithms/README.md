# Soul v0.1 算法

权威：`DECISION.md`（`ALGO_FROZEN`）。

保留：

1. **T4D** — `crates/soul-algo-tie`（默认 `TieAlgo::T4D`）
2. **A0** — `crates/soul-algo-trait`

过程稿在 `.agent_workspace/`。Goal 1 采纳方式：用本 crate 的 `score` 替换 `soul-graph` 的 `Tally::band`，不要把 SQLCipher 拉进算法 crate。

落地前挡住项目的项见 `docs/BLOCKERS.md`（`BLOCKERS_FROZEN`）。
