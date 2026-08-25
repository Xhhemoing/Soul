# Soul

桌面端本地优先的个人关系图谱。v0.1 的产品锁、算法冻结、工作包合同与计划入口在 [`docs/PLAN_INDEX.md`](docs/PLAN_INDEX.md)。

本仓库 **同时** 承载：

- 计划面：`docs/GOAL1_PLAN.md`、`docs/GOAL2_PLAN.md`、`docs/algorithms/` 冻结、D1–D60。
- 代码面：Tauri 2 桌面壳、SQLCipher 本地仓、图构建（T4D / A2 / G1+ / G3）、导入与会话。

主集成路径是 [PR #7](https://github.com/Xhhemoing/Soul/pull/7)（`cursor/goal1-unblock-a073`）。合入后 `main` 不再是「只有计划文档、没有可安装应用」。

## 现在能做什么 / 还不能做什么

**能：**

- 在作者 Win11 上按 `docs/STATUS.md` 的安装清单编译并打开桌面壳（需本机 SQLCipher / WebView2 / 证书；本仓库不代装系统依赖）。
- 跑 Goal 1 工作包对应的离线单测（`soul-graph` / `soul-draft` / `soul-profile` / `soul-schema` 等）。

**不能（产品锁，不是缺实现）：**

- 没有云同步、没有账号、没有联网模型、没有自动发消息。
- 没有「你的早晨」本地墙钟、没有窗口标题、没有预取、没有从应用猜性格。

**诚实限制：**

- 作者 Win11 全量安装清单 **尚未** 全部打勾（见 `docs/STATUS.md`）。
- GitHub hosted Actions 已耗尽免费分钟，空跑 runner 不能当证据。
- 本 Linux 云代理环境 **不能** 编译 `soul-app`（缺 webkit gtk）——那不是产品缺口。

## 文档从哪读

| 问题 | 文档 |
|------|------|
| 产品锁 / 算法冻结 | [`docs/PRODUCT_LOCK.md`](docs/PRODUCT_LOCK.md)、[`docs/algorithms/DECISION.md`](docs/algorithms/DECISION.md) |
| 计划入口 / 仓库拓扑 | [`docs/PLAN_INDEX.md`](docs/PLAN_INDEX.md) |
| Goal 1 / Goal 2 工作包 | [`docs/GOAL1_PLAN.md`](docs/GOAL1_PLAN.md)、[`docs/GOAL2_PLAN.md`](docs/GOAL2_PLAN.md) |
| 当前实现状态 | [`docs/STATUS.md`](docs/STATUS.md) |
| 形式化开工 | [`docs/FORMAL_WORK_PROMPT.md`](docs/FORMAL_WORK_PROMPT.md) |
| 安全边界 | [`docs/SECURITY.md`](docs/SECURITY.md) |

## 构建（开发者）

```bash
cargo test --workspace --all-targets --offline   # 在能编 soul-app 的机器上
cargo run -p xtask -- schema-freeze --check
```

Linux CI / 无 webkit 的机器请跳过 `soul-app`，只跑算法与 schema crate。
