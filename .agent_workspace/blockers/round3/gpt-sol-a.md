MODEL_SLUG: gpt-5.6-sol-xhigh-fast
G1: FREEZE_OK — T4D、全库单一 `as_of`、分列计数与 A2 消费符合 `DECISION.md` §6。
G2: FREEZE_OK — intake 不绕本人锁、保留证据且拒绝状态跃迁符合 §2 的 A0 冻结。
G4 dedup: FREEZE_OK at P1 — 导入幂等不是 `DECISION.md` 的算法冻结前置项。
G4 fan-out: P1_DEMOTION_UNACCEPTABLE。
EVIDENCE: §4.3 只允许真实群聊活动刷新任一场地 `last_contact`；当前导入器却在没有同期成员集时把一条 owner 消息扇出给所有历史发言者，能伪造近因并让 200 天未私聊的关系免于降档。
REJECTED: CLOSED；不重开。
