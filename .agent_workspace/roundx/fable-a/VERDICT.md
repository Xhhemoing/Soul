MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round X 交叉验证裁定（fable-a 槽位）

## 裁定：**CONFIRM_FREEZE**

`docs/algorithms/DECISION.md`（ALGO_FROZEN）与 `crates/soul-algo-tie`、`crates/soul-algo-trait` 在**冻结实质面上全等**：

- 常量值逐一相符（3 / 10 / 3 / 180 / 360 / Moderate 封顶仅 T4），闭区间 `>=` 语义相符且被测试钉死；
- T4D 三道门全在一对一计数、降档时钟读任一场地、空观测不进时间运算、as_of 全库单值禁 per-peer 禁墙钟——逐条与代码实现一致；
- 保留清单（T4D 默认 / T4 回退 / A0 锁 + intake 补丁 / A2 纯渲染 / A1 待命空转）与代码状态一致；
- 墓碑（T0/T1/T2/T3/T3R/A3）与作废变体（direct-only 时钟、milliscale、群聊附加门）在代码中要么以可执行墓碑存在、要么确实不存在；
- 回退链完好：T4 字节兼容待命，T3R 复活条件连同其对价钉在 `tombstones.rs`，T0/T3 无产品路径可回；
- 已提交测试 228/228 通过，`#![forbid(unsafe_code)]`、零依赖、产品路径无浮点均由测试而非注释保证。

## P0 清单：**空**

未发现任何会改变判档结果、推翻冻结条款、或使 DECISION 与代码在规则/常量上互相矛盾的缺陷。无阻塞项。

## 冻结确认所附的合并门槛（P1——不阻塞冻结，但阻塞第 6 节合并的验收）

以下各项均为 DECISION 第 0/6 节**自行声明**的冻结后合并义务的具体化，本裁定将其列为合并 PR 的硬验收条件，防止"义务"在执行中蒸发。详细依据见同目录 `CONSISTENCY.md`。

1. **常量名归一**（D1）：DECISION 表名（`DEMOTE_ONE_BAND_DAYS`/`FORCE_WEAK_DAYS`/`DORMANT_NOTE_DAYS`）与代码名（`DEMOTE_AFTER_SILENT_DAYS`/`WEAK_AFTER_SILENT_DAYS`/`DORMANT_AFTER_DAYS`）二选一并留痕；合并后全仓库同一常量只许一个名字。
2. **消灭第二个 180**（D2）：合并 crate 后 `DORMANT_AFTER_DAYS` 必须改为从统一常量模块 import，`a2.rs` 的独立字面量删除——这正是 DECISION"不得分裂"条款的落地动作。
3. **COPY_ZH 绑定测试落地**（D4）：按 COPY_ZH §5 七条断言写成测试；档位词汇收敛为单源 `强/中等/弱`（现状三套：`强联系` 系、`中` 系均须让位）；补齐透明句与收尾句；模板改动一律 COPY_ZH 先行。
4. **实现 A2 P2 的 2:1 方向分支并纳入单点定义**（D6）：该阈值目前只存在于 COPY_ZH，代码无对应实现；合并时实现五分支并把 2:1 列入常量纪律。

## P2 备忘（合并时顺手，不设门槛）

- `last_direct_contact` 的纪元第 0 秒哨兵边角（D8，上报字段，永不判档）；
- 自由函数 `explain_zh` 对未知 algorithm_id 的回落路径加固（D8）；
- A1 `TwoKindsAcrossDays` 的"跨日"措辞与实现对齐（D7，v0.1 不可达）;
- DECISION 引用的验证工作区夹具参数与 crate 同名夹具不同构（D5，判决方向全部一致，仅防误读）。

## 立场声明

本裁定未因任何风格问题弱化冻结：D1–D9 无一触及判档语义；凡属实质约束（常量值、区间方向、计数口径、时钟口径、锁语义、渲染器无第二意见）均已验证为文档=代码=测试三方一致。同时本裁定拒绝把"合并义务"当作既成事实——上列 P1 四项在合并 PR 验收前不得视为完成。改判通道维持 DECISION 第 5 节回退链原状，本轮无触发。
