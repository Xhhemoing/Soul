MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# C2R3：`as_of_day` 不进入 `plan_hash` 的判决测试

## 结论

`plan_hash` 应绑定用户批准的计划后果，而不是“计算发生在哪一天”。`as_of_day` 是生成计划时
需要钉住并重放的外部输入；它对计划的实质影响已经落在 `moves`、`left_alone` 及其理由里。

因此：

- 跨 UTC 零点但没有文件跨过近因阈值时，计划内容未变，hash 必须不变；
- 有文件在零点跨过阈值时，`moves`/`left_alone` 会变，hash 自然必须变；
- 把 `as_of_day` 本身序列化进 hash preimage，会让第一种情况每天产生一次
  `PLAN_HASH_MISMATCH`。这是时间流逝造成的假拒绝，不是目录或计划变化。

`as_of_day` 不进 `plan_hash` 不等于丢弃它：预览/批准应把它作为与 hash 并列的字段携带，
执行时用批准中的 day 重算，再比较 `plan_hash`。若 day 还参与“计划太旧”等独立门控，
批准载荷必须整体受 session/认证边界保护；不能靠制造每日变化的计划 hash 代替载荷完整性。

## 主判决测试

测试必须对**同一份 scan** build 两次；不要重扫，也不要创建两棵临时目录，否则 snapshot、
root 或 mtime 的差异会污染结论。

```rust
#[test]
fn crossing_utc_midnight_without_a_recency_transition_keeps_the_hash() {
    let scan = scan_once(fixture_with_one_eligible_file_much_older_than_day(200));
    let policy = RecencyPolicy::UtcDay { within_days: 1 };

    let before = build(&scan, policy.at(UtcDay::new(200)));
    let after  = build(&scan, policy.at(UtcDay::new(201)));

    assert_ne!(before.as_of_day(), after.as_of_day()); // 实际跨日
    assert_eq!(before.moves(), after.moves());
    assert_eq!(before.left_alone(), after.left_alone());
    assert_eq!(before.to_json(), after.to_json());     // as_of_day 不在 preimage
    assert_eq!(before.plan_hash(), after.plan_hash()); // 不得假拒绝
}
```

夹具不能是空目录：至少放一个顶层、扩展名可识别、目标未占用且足够老的文件，确保
`recency=on` 的真实候选路径被执行。若实现把 `as_of_day` 以任何键名直接或间接放进 canonical
JSON，`to_json` 与 hash 两条断言都会失败。

## 必需的反向对照

```rust
#[test]
fn crossing_utc_midnight_when_a_file_ages_out_changes_the_hash() {
    // mtime_day=199；within_days=1。
    let scan = scan_once(fixture_with_eligible_file_on_day(199));
    let before = build(&scan, recency(day(200), 1)); // days_since=1：先不移动
    let after  = build(&scan, recency(day(201), 1)); // days_since=2：可以移动

    assert_ne!(before.moves(), after.moves());
    assert_ne!(before.left_alone(), after.left_alone());
    assert_ne!(before.plan_hash(), after.plan_hash());
}
```

这条防止把规则错误实现成“hash 永远忽略时间造成的计划变化”。忽略的只是裸
`as_of_day`；它造成的可见决策变化仍必须进入 hash。

## 批准重放测试

```rust
#[test]
fn approval_replays_its_pinned_day_after_midnight() {
    let approved = preview_at(day(200));
    let approval = approve(approved.plan_hash(), approved.as_of_day());

    let replayed = rebuild_same_snapshot_at(approval.as_of_day());
    assert_eq!(replayed.plan_hash(), approval.plan_hash());

    // 执行发生在 day(201)，但不能偷偷用执行时的“今天”替换批准日。
    execute_at(day(201), approval).expect("same approved plan");
}
```

冻结判据可写成一句性质：

> 固定目录 snapshot 与规则版本时，若两个 `as_of_day` 产生完全相同的计划后果，
> `plan_hash` 必须相同；若后果不同，已有被哈希字段必须使 hash 不同。

本文件只给规格与测试草图；未改产品代码，未进行 git 操作。
