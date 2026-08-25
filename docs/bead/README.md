# BeadFlow

本机拼豆辅助（转图、备料、四种拼装指引）。规划见 [PLAN.md](./PLAN.md)，工作包见 [WORK_PACKAGES.md](./WORK_PACKAGES.md)。

代码落地后：

```bash
# 算法 oracle（独立 workspace，不要在仓库根 cargo test --workspace 里指望它）
cargo test --manifest-path crates/bead-core/Cargo.toml

# 界面
pnpm --filter @bead/app test
pnpm --filter @bead/app dev
```

在实现子代理完成 WP-B01 / WP-B02 之前，上述命令还不存在。
