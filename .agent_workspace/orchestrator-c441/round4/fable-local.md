MODEL_SLUG: claude-fable-5-thinking-xhigh
（自报：系统提示词标识 Claude Fable 5，thinking 开启；「xhigh」档位无法从会话内部证实，照派单记。）

# fable-local — Round 4 本地兜底（云端 fable-a / fable-b 均 unauthenticated）

分支：`cursor/goal1-build-audit-c441`（未离开）。到达时父代理已在 `17634fe` 写下
`docs/BRANCH_MAP.md` 与 `docs/FIRST_FORMAL_TEST.md`，所以本轮按派单改为**复核 + 改进**，不重写。

## 复核结论：`17634fe` 的事实全部核实

- 主干 `origin/cursor/soul-goal1-7b1c` tip `5309656`，共 30 提交，根 `d510f1e`；`478f19f` 是其祖先（主干尖端往回 4 个提交）。
- `origin/cursor/goal1-build-audit-c441` `9b4bcd4`：+41 / −0，对主干可快进。
- `origin/cursor/goal1-closeout-c441-2d70` `9eeef94`：+4 / −1，叉于 `a9a7490`；`git merge-tree` 里 changed-in-both 的确是 `docs/STATUS.md`（closeout 侧另改 `Files.tsx` / `Files.test.tsx`，主干侧只改 STATUS）。
- `main`（`a0ec14b`，8 提交，根 `ea6f62f`）与主干**无 merge-base**——两棵独立历史，印证「故意冲突、禁止合入」。
- `.github/workflows/ci.yml`：push 只挂 `main` 与 `cursor/soul-goal1-7b1c`，`paths-ignore` 掉 `docs/**` / `*.md`，`workflow_dispatch` 可从任意 ref 跑；五个 job 是 `lint` / `test-linux` / `test-windows` / `sbom` / `package`。
- `justfile`：`ci-full: ci deny`，`ci` = lint schema e0 denylist fixtures-verify test smoke-lint sbom ui-lint ui-test。
- `docs/PRODUCT_LOCK.md` 第 76 行确是「进入 E1 的请求体必须经 redactor……姓名/账号同样占位」的无条件承诺。
- `scripts/author-manual-checklist.md` 第 0 节已点名 `478f19f`+ 与 `docs/FIRST_FORMAL_TEST.md`，且排除 `2e72ddf` 安装包——与测试方案无冲突。

## 我改了什么

`docs/BRANCH_MAP.md`：

1. 新增「全量清单」表：origin 上 19 条 ref 逐条列 tip / 末次提交日 / 对主干 ahead-behind（或无 merge-base）/ 家族归属，全部用 `git rev-list --count` 与 `git merge-base` 核过。
2. 「两棵合同树」那段补上根 SHA（主干 `d510f1e`、main `ea6f62f`），并点名两条连 `main` 也对不上的独根过程枝：`blockers-analysis-a073`（单提交 `e669b78`）、`predict-algo-survey-a073`（根 `b3cb6d4`，30 提交）。
3. 本机超前 origin 的措辞由「约 20 个提交」改为「20+（R3 姓名占位与 R4 文档），推送仍 HTTP 401」。

`docs/FIRST_FORMAL_TEST.md`：

1. 段 A 的门列表改为与 `justfile` 逐字一致（补 fixtures-verify / smoke-lint / sbom / cargo-deny）。
2. 段 B 补两条已核实的机制：dispatch 可从任意 ref 跑；`docs/**` / `*.md` 推送不触发（docs-only 推不出 run 不算新故障）。
3. AC-26 那行点名五门 job 名。

三条红线未破：**没有**建议合 `agent/dev-sota` / `cursor/goal1-unblock-a073` / `main` 进主干；方案仍是主干 `478f19f`+、作者 Win11 清单、Billing 恢复后 hosted AC-26；不是 Goal 2、不是 AC-27。

## 没做的

- 未推送：`git fetch origin` 已报 Invalid username or token（HTTP 401），按派单不推。
- 未动 Rust / TS，未执行任何合并。
