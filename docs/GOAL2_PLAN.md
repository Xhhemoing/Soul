# Goal 2 规划（Goal 1 关闭后启动）

## 前置条件（三条都要能机械检查）

```sh
S=$(git rev-parse --short=7 HEAD)
ls docs/gates/*-$S-linux.md >/dev/null 2>&1 && ls docs/gates/*-$S-win.md >/dev/null 2>&1 || { echo "缺 $S 的 G-L/G-W 记录"; exit 1; }
grep -q 'Goal 1 已关闭' docs/STATUS.md || { echo "STATUS 未记录 Goal 1 关闭"; exit 1; }
grep -q 'ACCEPTANCE 全部 v0.1 行通过' docs/STATUS.md || { echo "STATUS 未记录验收矩阵通过"; exit 1; }
```

三条任一失败即停，不启动 Goal 2。约定：关闭 Goal 1 时，`docs/STATUS.md` 必须逐字写入「Goal 1 已关闭」与「ACCEPTANCE 全部 v0.1 行通过」两句，否则上面的检查永远不会为真。

## 方式

指标驱动：每个方向先写下可测指标与当前值，再动手；没有指标的方向不改。每次改动过本地门禁（`just ci-full`、`scripts/gate-win.ps1`）并落 `docs/gates/`。不放宽 `PRODUCT_LOCK.md`；不把 v0.1 砍掉的项加回来。

## 关注方向

灵魂一致性、图谱证据、采集最小化、研究/助手隔离、提示注入、权限、性能、可靠性、体验、文档、测试缺口、崩溃恢复、安装升级、无障碍。

## 与多端的关系

按作者 2026-09-04 澄清（DECISIONS D65），核心后续部署于 Linux / macOS / Windows，接入端含 Android。多端排期见项目计划，不在本文件。
