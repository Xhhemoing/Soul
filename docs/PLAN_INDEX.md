# 计划索引（30 秒定位权威文件）

核于 2026-09-25。本页只导航到当前仓库实际存在的权威文件；历史分支说明不作为当前进度。

## 一、先读四份

| 文件 | 唯一权威范围 |
|---|---|
| [PRODUCT_LOCK.md](PRODUCT_LOCK.md) | 产品边界、v0.1 十三条垂直切片、出网分级、不可协商约束 |
| [DECISIONS.md](DECISIONS.md) | 已拍板的选择；本地门禁为 D61–D63，文档清理为 D64，平台定位为 D65 |
| [algorithms/DECISION.md](algorithms/DECISION.md) | T4D、A0/A1/A2/A3、常量、as_of 纪律与回退链 |
| [ACCEPTANCE.md](ACCEPTANCE.md) | Goal 1 验收矩阵、工作包、红线与关闭条件 |

## 二、按问题查

| 问题 | 去哪 |
|---|---|
| 现在到哪一步、还有哪些缺口 | [STATUS.md](STATUS.md) 顶部当前状态；历史段按当时日期阅读 |
| v0.1 是否做某个功能 | [PRODUCT_LOCK.md](PRODUCT_LOCK.md) 的最小垂直切片与砍/留 |
| 一件事怎样算完成 | [ACCEPTANCE.md](ACCEPTANCE.md)；矩阵与产品十三条切片同时满足 |
| Goal 1 的实现拆分与依赖 | [GOAL1_PLAN.md](GOAL1_PLAN.md) |
| 门禁怎么跑、哪个版本有证据 | [gates/README.md](gates/README.md) 与对应 SHA 的记录 |
| Windows 真机要看到什么 | [作者手动清单](../scripts/author-manual-checklist.md) |
| 数据契约 | [schemas/](schemas/) 正文与 schemas.lock.json |
| 加密、密钥、遗忘、审计 | [SECURITY.md](SECURITY.md) |
| 算法判档与中文话术 | [algorithms/DECISION.md](algorithms/DECISION.md)、[COPY_ZH.md](algorithms/COPY_ZH.md)、[REJECTED.md](algorithms/REJECTED.md) |
| Goal 2 | [GOAL2_PLAN.md](GOAL2_PLAN.md)；前置条件未满足时不启动 |
| 审查发现和修复证据 | [reviews/](reviews/)；局部修复回执不等于完整门禁通过 |

## 三、仓库与验收状态

- 本地主干记录 `8aae8f3` 已合入 M0-2 收敛，仓库包含核心与桌面实现；不再是只有计划和算法 crate。
- M0-3 收尾源码为 `95ff7d6`：继承 `1eb471a` / `e2fdf16`，新增注册路径解码、临时副本卸载及完整目录消失判定；隔离 G-W 与受控回归通过。实际安装首次失败、部分恢复仍有残留，修复后的真实 NSIS 集成待验证，见 [9 月 25 日记录](gates/20260925-95ff7d6-win.md)。旧安装包来源仍为 `e2fdf16`。G-M / G-L 未齐，Goal 1 未关闭。
- 根 Cargo workspace 与 `apps/desktop/src-tauri` 是两个独立 workspace。根目录测试不覆盖桌面壳。
- 计划冻结、代码实现、平台门禁通过、Goal 1 关闭分别记录。旧版本的绿不能替代当前版本的证据。
- 本轮按用户指示暂时跳过 Linux 门禁；这不是豁免 Goal 1 的 G-L 关闭条件。

## 四、历史材料的边界

D64 已移除 FORMAL_WORK_PROMPT、GOAL2_POLISH_PROMPT、PLAN_VERIFY_PROMPT 与扫描流程材料。当前验收和规划使用本页所列文件，不恢复这些旧入口。
历史分支、临时探针、过程草稿可以解释由来，不能替代当前源码与门禁结果。

## 五、改动规则

| 改动 | 前置 |
|---|---|
| 产品方向 | 先改 PRODUCT_LOCK，在 DECISIONS 追记 |
| 算法判档 | 按 algorithms/DECISION 的回退链，DECISIONS 留痕；不另设阈值 |
| 验收矩阵 | 不删已通过项；新增项写清 Given / When / Then 与执行者 |
| 工作包 | 只减不增；WP12 保持删除 |
| schemas | 同批更新 schemas.lock.json，并说明对已落库数据的影响 |
| 合并 | 遵守 D63；本次跳过某平台不等于该平台通过 |
