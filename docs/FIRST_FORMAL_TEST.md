# 第一次正式测试方案

核于 2026-08-25。这是 Goal 1 v0.1 的**第一次作者 + CI 验收**，不是 Goal 2，不是 LOOP20，不是 AC-27（文件写入仍是 v0.1.1）。

合同：本树 `docs/FORMAL_WORK_PROMPT.md` 的 AC-01..AC-26。`main` 上的 AC-28+ / 算法 crate **不在**这次测试里。

## 打哪一棵

- 分支：`cursor/soul-goal1-7b1c`
- SHA：**`478f19f` 或之后**（origin 尖端曾是 `5309656`）。
- **不要**用 `2e72ddf` 的 `windows-binaries`：那次 NSIS 把程序装进数据目录，卸载会碰到 `keys.dpapi`。
- 若作者明确要测 BUILD audit 的诚实文案 / R3 占位：改打 `cursor/goal1-build-audit-c441` 的**已推送** tip，并在 `STATUS` 写明 SHA。本机未推送的 tip 不能当正式测试对象。

打包只在作者 Win11 上做（`tauri build` 会取 NSIS，CI 不允许）。步骤与勾选见 `scripts/author-manual-checklist.md`。

## 当天怎么走

| 段 | 谁 | 做什么 | 算不算过 |
|---|---|---|---|
| A | 本机 Linux / 本云端 VM | `just ci-full`（或等价：lint / schema / e0 / denylist / workspace test / ui-lint / ui-test） | **本机绿不是 hosted 绿。** 只能证明这台机器。 |
| B | GitHub Actions | Billing & plans 恢复后，对候选 SHA `workflow_dispatch`。workflow 只自动 `push` `main` 与 `cursor/soul-goal1-7b1c`。 | AC-26。空 runner（0 step、空 `runner_name`）**不是**产品回归，是 N4。不要 empty-commit。 |
| C | 作者，Win11 x64 非管理员 | 清机 → 清单第 1–7 节（安装 smoke、托盘、关窗再开、采集、WebView2 观感）。 | AC-01 与 AC-09/10 的真机一半；过不去就**记缺口**，不要让安装器去下载东西。 |
| D | 作者，可选 | 清单第 8–10 节：界面导入、本机端点、人脉图摘要、语气与审计。 | 不是 AC-26 门禁。人脉图**不应**再看到「根据本机统计改写」。 |

## AC-01..AC-26 谁证明

| ID | 谁跑 | 第一次正式测试怎么记 |
|---|---|---|
| AC-01 | 作者手动 + `install-smoke.ps1` | 段 C。UAC 盾牌脚本测不到，人看。 |
| AC-02..AC-08、AC-11..AC-20、AC-23..AC-25 | CI / 本机 `just ci` | 段 A 先跑；段 B 才是 hosted。 |
| AC-09、AC-10 | CI 有一半；真机采集在清单 | 段 C 补切窗口。 |
| AC-21、AC-22 | CI 有源码/套接字一半；OS 流量在清单 | 段 C。 |
| AC-26 | hosted 五门 | 段 B。N4 未解之前不得称正式 CI 绿。 |
| AC-27 | 不在本次 | `/files` 没有执行按钮是正确的。 |

## 已知诚实边界（测的时候不要当意外）

- 空图 + 二次确认豁免 + 名字**不在**「某某说」前面时，未导入姓名仍可能出网。PRODUCT_LOCK 第 76 行仍是无条件承诺。这是已记账的残留，不是这次测试新开的 Goal。
- Graph / 摘要：端点那一句须标明是端点写的、没有证据、本机没核对。
- hosted 空 run 的账本注解是 Billing & plans，不是 Actions 分钟用尽。

## 通过规则

1. 段 C 每一条勾选都要写**看见了什么**，写「过了」不算。
2. 任一条过不去：记进 `docs/STATUS.md` 对应 WP 段落，停止装「下载修复」。
3. 段 B 五门都有 runner 且绿之前，Goal 1 **还不能关**。
4. 不要合 PR #4 / #7，不要启动 Goal 2。
