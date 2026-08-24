# NOTES — Round 2 / fable-b 过程稿

## 证据核对记录（全部行号对过源码，防止规格建立在转述上）

读法：`git fetch origin cursor/soul-goal1-7b1c` 后 `git show origin/cursor/soul-goal1-7b1c:<path>`，未拷仓。

- 扇出：`crates/soul-import/src/commit.rs` L198-207（peers 分支）、L397-416（`speakers_by_conversation`，
  全文件汇总、排除 owner、无时间约束）。扇出观察的 `event_id` 都指回同一条事件——这保证了去重与
  未来重归因的可追溯性，是「保留扇出」方案成立的前提。
- 双写自认：`commit.rs` 模块文档 L22-24「Committing the same file twice writes the events twice…The caller decides.」
  ——v0.1 的「caller decides」实际是没人 decide，Telegram 累积导出使其成为主路径事故。
- external_id 存在但被丢：`model.rs` L135-137（「Not stored」）；Telegram 侧 `telegram.rs` L177
  `format!("{chat_id}:{message_id}")`；JSONL 侧 `soul_import_v1.rs` L174 `external_id: message.id`，
  schema 强制非空字符串但不强制唯一。
- clobber：`crates/soul-graph/src/build.rs`（= context/impl 的 graph_build.rs）L234-252 整行覆写边，
  L322 `user_verdict: Some(UserVerdict::Unreviewed)`。`UserVerdict::Corrected` 枚举在
  `soul-schema/src/inference.rs` L24-29 已存在，纯粹无人写入。
- 自由 object：`relationship.schema.json` L14 `"tie_strength": { "type": "object" }`——锁字段零契约变更的依据。
- 轴侧对称模板：`soul-profile/src/service.rs::correct_axis` L222-271（证据→place_axis(locked=true)→审计）。
- WP10 纯消费者核实：`soul-draft/src/analysis.rs` L244 `let band = strength.band;`，全文件无阈值常量。
  R1-SYNTHESIS P1-8 的「WP10 可能自带第二套阈值」在 goal1 主干不成立，成立的是 opus-b 参考实现的
  `sample_band`（opus-b REPORT F-2 自认，DA-6）。ACCEPTANCE_GAP 已按此改写风险位置。

## 决策取舍（被否掉的方案，防止 Round 3 重开战场)

1. **扇出限窗（B 案）**：窗宽是新超参且用户不可复核；否。删扇出（A 案）制造 group-only 隐形，否。
2. **算法内去重**：违反接口契约（算法要看 evidence id 才能去重），把管道债搬进判定函数；否。
3. **(peer, timestamp) 启发式去重**：会把同秒两条真实消息误合；只认 source|external_id；否启发式。
4. **纠正 API 允许凭空造边**：无证据边违反「无证据不落库」，且 UI 无处解释它的计数全为 0；
   v0.2 user-stated 节点再议；否。
5. **band 恒机器、user_band 另读**：每个消费者都要记得查锁，漏一处=装饰性锁 2.0；否。
   代价是 `band` 字段语义变成「生效档」，已在规格里用黑体钉死并配 GC-9 测试。
6. **纠正证据 subject 用 ThirdParty**：纠正行内容只有 relationship_id + band 词，是 owner 对自己
   关系图的裁决，无第三人姓名/正文；取 Owner，与 `correct_axis` 完全对称。若评审认为
   「关于某条第三人边」应记 Mixed，改一个枚举值即可，不影响结构。

## 开放问题（需要父代理或 Round 3 回答）

- **审计枚举**：`GraphCorrect` 与 `DuplicateImport` 两个 ReasonCode/Action 是否值得动冻结 audit schema？
  两份规格都设计成不依赖它（复用 `ProfileCorrect` / 计数进回执），增补只是锦上添花。
- **R3 的未锁定无 tally 边**：GRAPH_CORRECTION 只裁决了锁定边（存活、纠正行为唯一证据）；
  未锁定的陈旧边删除 vs 墓碑归 P2-6，Round 3 连同「遗忘→rebuild」测试一起定。
- **within-file 重复 id 对 soul-import-v1 是否升级为拒收 defect**：规格给了 SHOULD；
  若父代理想要 MUST，注意这是我们自己格式的契约收紧，要同步 schema 文档措辞。
- **fable-a 的 T3/T3R 规范是否接受「Strong 只消费 Direct 计数」的改写**：这改动 CANDIDATE_SPEC
  的冻结文本（其 §T3 用的是 any_direct 布尔）。按其自己的流程「改定义先改本文件并留痕」，
  裁决夹具跑出结果后由 fable-a 落笔，我不越权改。

## 自查

- 只写入 `.agent_workspace/round2/fable-b/`（5 个文件）；未 commit / push / 开 PR。
- 未把 goal1 整仓拷进本分支（只在 /tmp 留了阅读用抽取件，不在仓库路径下）。
- 三份规格相互引用处均用文件名+节号，无悬空引用。
