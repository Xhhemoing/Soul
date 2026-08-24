MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Round 2 gpt-sol-b — anti-scope-creep deltas

## Δ1 — G4 导入去重从 P0 降为 DEFER/P1

- `PRODUCT_LOCK` 与 AC-04/05 没有「重复导入必须幂等」要求；Goal 1 `STATUS` 还明确记录「同一文件导入两次会写两遍事件」及其存储边界代价。
- T4D 规定如何评价观测，不保证上游文件无重复。重复观测会影响结果，是数据质量债，但不是「冻结算法未接线」。
- 当前解法会新增持久化身份/事务语义，不能用一个未冻结的摘要公式冒充小修。移出 P0 和工序 4；另开 P1 设计/迁移项。若产品锁以后明确要求幂等，再升格。

## Δ2 — M1 只需停掉第二主干；搬运其“独有项”不是 P0

- P0 到「宣布 PR #2 为唯一 Goal 1 主干、停止/关闭 PR #4」即结束。
- D32 已被 `PRODUCT_LOCK` 的“不抓微信/QQ”覆盖；SOTA 评论文档、e0 crate 钉子、远端分支改名/删除都不影响 PR #2 正确合入。
- 删除 M1 解法中“必须樱桃摘/改名”的门禁语气；这些逐项审后可 DEFER，不能让归档清理继续挡 Goal 1。

## Δ3 — `--no-fail-fast` 是可观测性 P1，不是 CI 红的 P0 根因

- 真正 P0 是 NTFS 大小写折叠夹具；修掉后正常 `cargo test --workspace --all-targets` 会继续跑完。
- `--no-fail-fast` 有诊断价值，建议保留，但不应与 AC-18 夹具并列为“未做就不可合入”，更不必强制拆成两次推送。

## Δ4 — KnownIdentifiers 保持 DEFER，不进入 Goal 1 关键路径

- `STATUS` 已裁定空名单“降低精度不是底线”：默认第三人 turn 整段占位，地址/handle/长数字另有 shape scrub。
- 它不该出现在推荐工序 10 或 Goal 1 关闭清单。只有一个能穿过现有占位边界的 AC-12/13 失败夹具，才足以把它升为修复项。
- “顺便解封图节点姓名”是另一个 UI/解密面，不能绑在标识符清洗上。

## Δ5 — perl/NASM 门禁 IGNORE

- 当前报告命令确实恒成功，但紧随其后的 SQLCipher/桌面 release 编译才是真消费者；缺 perl/NASM 时构建本身会红，不存在假绿。
- 强钉 `OPENSSL_SRC_PERL` / `OPENSSL_RUST_USE_NASM` 会引入 runner 路径耦合，却不增加产品证据。可把报告改成 fail-fast 提示，但不得列为发货或合入阻塞；从 S4 和工序 10 删除。

## Δ6 — `tauri build --no-bundle` IGNORE

- 它最多证明 CLI 解析配置，不能生成或验证 NSIS、安装范围、卸载器、WebView2 策略，也不能替代作者的真实 bundle + install smoke。
- 现有 CI 已构建 frontend、`custom-protocol` release `soul.exe` 并从 PE 读取 asInvoker。再跑 no-bundle 是重复证据，不是 AC-26“打包绿”的闭环。
- 保留真实打包为作者手动证据；不要为消除一句“CLI 未运行”新增 Goal 1 门禁。

## Δ7 — “merge commit，不要 squash”不是正确性的承重墙

- `STATUS.md` 没有引用实现提交 SHA；其中的“哈希”是 plan/audit/schema 哈希。BLOCKERS 所称“96 个 STATUS 引用的提交哈希全部失效”与文件内容不符。
- squash 会丢失逐提交祖先/二分价值，也不会让 PR #1 自动显示 merged；这是历史与 GitHub 记账损失，不会改变合入后的树或算法行为。
- 因 96 个已审提交，仍应**强烈偏好 merge commit**，但把它从 P0 硬门降为集成策略；若被 squash，手动关闭已被内容覆盖的 PR #1 即可。

## Δ8 — 本分析 PR 应先落 `main`

- PR #6 当前以 `main` 为基、可干净合入；它描述的是“如何把 Goal 1 合进 main”，若只放 Goal 1 分支会形成由被集成分支私有控制集成规则的循环。
- Round 2 冻结 `BLOCKERS_FROZEN` 后先合 PR #6 到 `main`，再按 M2 将最新 `main` merge 进 Goal 1；不要另行 cherry-pick 本分析，避免两份历史。
- 因此推荐工序应明确：冻结/合入分析 PR → Goal 1 吸收 main → 修真正 P0 → Goal 1 合 main。
