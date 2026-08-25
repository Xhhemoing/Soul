# Round 1 结论简报 — Goal 1 close-loop

Parent: cursor-grok-4.6-high. Branch: `cursor/goal1-close-loop-a073` @ `b502590`.
Injected seed: `.agent_workspace/close-loop/SHARED_BRIEF.md`. Unique merge-to-main path remains PR #7.

## 六个槽位

| 槽位 | 请求 slug | 产出 |
|---|---|---|
| fable-a | claude-fable-5-thinking-xhigh | 收口检查单：WP 与切片大多 already-done；作者手动 / 分钟 / frozen-wont 明确排除 |
| fable-b | claude-fable-5-thinking-xhigh | 拓扑：追加 D61 不改写 D49；本叠分支合法，**禁止**对 `main` 开第二条合入 PR |
| opus-a | claude-opus-5-thinking-high-fast | **R-1 关闭**：`correct_tie`/`release_tie` 对空 `algorithm_id` 先 rebuild；`UnscoredEdge` 拒绝；schema 未松 |
| opus-b | claude-opus-5-thinking-high-fast | **CI-TRUNK 关闭**：auto-push + 五门 `if:` 加上 `cursor/goal1-unblock-a073`；保留历史 `soul-goal1-7b1c`；无 `pull_request` 触发 |
| gpt-sol-a | gpt-5.6-sol-xhigh-fast | AC-28…34 映射：AC-31 COVERED；其余 PARTIAL。139 测绿；schema-freeze 绿 |
| gpt-sol-b | gpt-5.6-sol-xhigh-fast | R-1 探针（隔离 crate，未改工作区 `Cargo.lock`）；确认本环境不能 merge GitHub PR |

## 已实现 / 已证伪

- 种子 CI-TRUNK、DOC-INDEX：**过时**（本轮或先前吸收已落地）。
- 种子 R1-LEGACY：**本轮关闭**（`9fd6870`）。
- Goal 1 工作包与 T4D/A2/G1+/G3 **不要重做**。
- 本环境 `gh` 只读，**不能**把 PR #7 合进 `main`，也不能关 PR #4。

## 遗留（Round 2 只准这些）

| ID | 谁 | 内容 |
|---|---|---|
| CODE-2 | opus-a | 产品边界收严 AC-28/29/30：具名夹具对拍；决胜边 `direct_active_day_count`；`group_heavy_plus_three_directs`=Moderate 与锚同测；两缘 `as_of_utc` 全等。禁止重推阈值、禁止改 algo crate 常量 |
| CODE-3 | opus-b | AC-34 第二句：owner 群消息不得刷新历史发言人 `last_contact_utc` |
| DOC-D61 | opus-b | 按 fable-b `DOC_PATCHES.md` 补丁 1–4：追加 D61，PLAN_INDEX D 号与豁免句 |
| R1-REVIEW | fable-a | 只读复核 `9fd6870`，不重写 |
| AC-REPROBE | gpt-sol-a | opus 落地后重跑映射与 `cargo test` |
| LOCK-HYGIENE | gpt-sol-b | 确认工作区 `Cargo.lock` 未脏；ci.yml 仍无 `pull_request`；R-1 产品路径绿 |

## 明确不做

Goal 2；F04c 第三道门；GC-6/7；GC-9b；empty-commit 刷 CI；把本分支 PR 打到 `main`（必须叠回 `cursor/goal1-unblock-a073`）；合 PR #1/#2/#4。

## 性能 / SOTA 差距

无性能瓶颈。SOTA 差距 = 矩阵严格字面覆盖（CODE-2/3）+ 作者 Win11 + hosted 分钟，后两件不是代码。
