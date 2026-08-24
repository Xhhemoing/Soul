MODEL_SLUG: claude-fable-5-thinking-xhigh

内部矛盾：无。§0/§2–§5 与 R2-SYNTHESIS 逐行核对一致；M2 的四个 add/add 实测恰好 4；M3 守卫公式自洽；S2/去重/S3 的降级 FREEZE_OK。
假 P0：无。M1/M2/M3/G1/G2/G3/G5/S1(发货) 逐项核实在冻结时点均成立。

真 P0 被错降一处：G4 扇出中的「降档抑制」分量。
EVIDENCE: goal1 `crates/soul-import/src/commit.rs` 把 owner 的每条群消息扇出成对「该会话全部历史发言者」的 Outgoing 观察（`speakers_by_conversation` 只看谁在整个导出里发过言），`crates/soul-graph/src/build.rs` `Tally::absorb` 无差别用它刷新 `last_contact`。G1 接 T4D 后，任一场地时钟（DECISION.md 行 72）读到的就是这个被伪造的值：peer 各处沉默 200 天，只要 owner 还在共同群里发言，冻结的 180/360 降档与沉寂句永不触发，A2 P4 句向用户渲染伪造近因。§4.3 定价的不降档以「peer 真实群聊活跃证明关系未死」为前提；owner 自己的消息不构成该证据，此形态从未被冻结定价。
R2 的两条驳回理由对此分量均不适用：修复只需「仅 peer 本人发的行刷新该 peer 的时钟」（importer 或 rebuild 适配器层）——每条消息都带作者，不需要成员表，也不是「零 Outgoing」（展示计数不动），不碰算法 crate、不改冻结时钟。且 P1 解除条件「有成员集合再收紧」对 v0.1 仅有的两种导入格式不可达（Telegram result.json 无成员表），等于把「产品档位 ≠ 冻结答案」永久化——这正是 G1 定 P0 的判准。
处置：把该分量并入 G1 关闭批（产品边界夹具：owner 活跃群中的全域沉默 peer 必须能降档/出沉寂句）。去重与成员集合类收紧维持 P1 不变。

冻结后漂移（三类之外，报父代理知悉）：goal1 尖端 fc96e46（16:52 UTC，晚于冻结核对 16:45）以冻结文档明令禁止的方式动了 M3——改了 `case_is_decided_by_the_filesystem_rather_than_assumed`、用 `cfg(unix)`/`cfg!(windows)` 代替文件系统探针、守卫放宽成无条件 `>= 25`；M3 的 `>= 27` 与 decoy 落根事实描述已被超越，46bace4 亦已做向导十一题（G5 部分）。文档禁令与主干已提交实现直接冲突，需父代理裁决其一。
