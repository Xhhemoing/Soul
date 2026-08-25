# agent/dev-sota — Goal 1 剩余实现

**分支**：`agent/dev-sota`  
**目标**：P0（WP10/WP11/壳接库）已落地。作者指示：聊天数据读取先不做；继续把已有核心接到 UI。

## 循环

| 轮 | 内容 | 状态 |
|---|---|---|
| Plan | 拆 P0 | 完成 |
| Write / Optimize / Review P0 | WP10 + WP11 + 壳接库 + SOTA 补钉 | 完成（`9705fb4`） |
| 拍板 D32 | QQ/微信读取推迟：开源 QQ / wechat-rpa，进程外导出再导入 | 完成（`215a310`） |
| Plan 波次 UI | fable 拆 WP09 起草页 + 文件页 | 完成 |
| Write | ST-V0～V3：host DTO/IPC、起草 UI、文件 UI | 完成（`a3c8f92`） |
| Review | fable SOTA 两页 | 完成（ship，无 must-fix） |
| Merge | 合入 `main` 的算法冻结 #5；denylist 豁免 algo crate | 完成（`08c9efa`） |
| Docs | ST-V4 写回 STATUS / GOAL1_PLAN | 完成 |
| T4D | `soul-graph` rebuild 走 `assess_tie`，替换 T0 | 完成 |
| PR | 更新 #4；hosted CI 因 Actions 分钟用尽秒失败，未合并 | 进行中 |

## 作者约束（2026-08-24）

- **数据读取先不考虑。** QQ：既有开源项目。微信：wechat-rpa。
- Soul 本仓库不嵌 RPA/抓取；日后只适配它们的导出为 `soul-import-v1`。
- 采集页、OAuth、微信/QQ 客户端通道均不在本波次。

## 本波次已做

把已落地的 `soul-draft` / `soul-fileplan` 接到桌面：粘贴起草（无发送按钮）、授权目录只读计划（无执行按钮）。UI 无业务逻辑；IPC wrapper ≤1 句。
