# Round 2 结论简报 — Goal 1 close-loop

Parent: cursor-grok-4.6-high. Integration branch: `cursor/goal1-close-loop-a073` (agent side-branches cherry-picked here; do not PR those `-df52`/`-4799` forks to `main`).
Injected: `.agent_workspace/close-loop/R1-SYNTHESIS.md`.

## 六个槽位

| 槽位 | 请求 slug | 产出 |
|---|---|---|
| fable-a | claude-fable-5-thinking-xhigh | R-1 五门全过（schema 未松、零值仍序列化、UnscoredEdge 无半写纠正） |
| fable-b | claude-fable-5-thinking-xhigh | CODE-2/3 未开新 AC 行；D61 当时 APPLY，现已落地 |
| opus-a | claude-opus-5-thinking-high-fast | CODE-2：`t4d_product.rs` 16 测；具名夹具产品对拍 |
| opus-b | claude-opus-5-thinking-high-fast | CODE-3：`last_contact_utc` 不被 owner 群消息刷新；D61 + PLAN_INDEX 豁免句 |
| gpt-sol-a | gpt-5.6-sol-xhigh-fast | 并发时测的是 R1 HEAD；父代理在合入后重跑 soul-graph/soul-import **绿** |
| gpt-sol-b | gpt-5.6-sol-xhigh-fast | 工作区 `Cargo.lock` 未脏；ci.yml 无 `pull_request` |

## 演进（相对 Round 1）

R1 关 R-1 写回与 CI 主干触发。R2 关矩阵严格字面：AC-28/29/30 具名夹具 + AC-34 近因句，以及 D61 主干更名追加。

父代理本机：`cargo test -p soul-graph -p soul-import --offline` 绿；`schema-freeze --check` 绿。

## 潜在边界风险

- 子代理开了不合前缀的侧枝（`goal1-r2-code2-df52` 等）。**权威只认** `cursor/goal1-close-loop-a073` → 合回 PR #7。禁止对这些侧枝开对 `main` 的 PR。
- AC-28「解释文案同屏报两个数」仍可能只在 A2 测试而不在 graph UI；不挡合入。
- mixed-store 上 `scored()` 先 rebuild 再拒绝：fable-a 记为可选钉死，非 P0。

## SOTA 验收差距（仍不是代码）

作者 Win11 清单；hosted Actions 分钟；PR #7 合 `main`（本环境不能 merge）；PR #4 关闭（本环境不能关）。Goal 1 **不能**在本轮宣称关闭（D54）。禁止用 Round 3 去开 Goal 2 或补这两件。

## Round 3 只准

交叉核验已落地 diff；可选 mixed-store UnscoredEdge 测试；STATUS/FORMAL 不改写历史吸收句。禁止新功能、禁止 F04c、禁止 empty-commit。
