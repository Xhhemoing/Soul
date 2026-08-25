# R2-SYNTHESIS — BUILD+AUDIT round 2

核于 2026-08-25。分支 `cursor/goal1-build-audit-c441`（基线 `cursor/soul-goal1-7b1c` @ `5309656`）。
模式：BUILD 续跑 + AUDIT。**LOOP20 仍 QUEUED。** 不合 PR #4 / #7 / #10。Goal 1 未关闭。

## 派单与模型

| 代理 | slug | 产出 |
|---|---|---|
| fable-a | claude-fable-5-thinking-xhigh | `round2/fable-a.md` `9dd578f` |
| fable-b | claude-fable-5-thinking-xhigh | `round2/fable-b.md` `3550bcb` |
| opus-a | claude-opus-5-thinking-high-fast | `1f52ca5` Files 空态 |
| opus-b | claude-opus-5-thinking-high-fast | `24ea578` + `d1b6457` E1 文案与 origin 哈希 |
| gpt-sol-a | gpt-5.6-sol-xhigh-fast | `round2/gpt-sol-a.md` `9ec20a8` |
| gpt-sol-b | gpt-5.6-sol-xhigh-fast | `round2/gpt-sol-b.md` `82af9c0` |

无静默降级。未凑第六个假 LOOP3 scope。

## 已闭合（本分支）

| ID | 来源 | 提交 |
|---|---|---|
| P0 Files 空态 | 全员确认 | `1f52ca5` |
| P0 起草确认屏不描述真实请求体 | gpt-sol-a | `24ea578` |
| P1-1 E1 换端点旧计划仍发出 | gpt-sol-b / fable-b | `d1b6457` |

dpapi / collect FFI / HITL 令牌：fable-b 判干净。

## 分歧与采纳

| 议题 | 采纳 | 理由 |
|---|---|---|
| 重复导入幂等 | **不**做索引重写 | D55：P1 设计目标，不进 v0.1 矩阵。只允许收据/文案诚实警告 |
| 合 PR #7 做 T4D | 拒绝 | D49 + 主干 STATUS「不要合 #7」 |
| AC-21 改观察 `soul.exe` | 本 Linux 轮次不落地 | 真机/Windows 产品烟；记 STATUS，不 empty-commit |
| LOOP20 | 继续排队 | Goal 1 未关闭 |

## 下一刀（R2 落笔，路径不重叠）

1. **opus-c** P1-2：无 display_label 的联系人遗忘是空转 — `soul-store/forget.rs` + `soul-import/commit.rs`
2. **opus-d** P1-4：`DirectorySnapshot::of` 无视 `max_entries` — `soul-fileplan/scan.rs`
3. **opus-e** 余下文案 P1：向导 / 导入预览 / 遗忘「不写文件」 / 图摘要 payload — UI + 对应 notice 常量；**不动** `Files.tsx` / `Draft.tsx`
4. **opus-f** gpt-sol-b P1-3：e0-audit `starts_with` 前缀 — `crates/xtask/src/egress.rs`
5. **opus-g** P1-1 收口 + P2-5：换端点清 pending；`forget_memory` 先匹配再 take — 仅 `soulcore` `session.rs` 及测试

P2-1/2/3/4 本轮停车。N4/N5/N6 仍 blocked-on-user。
