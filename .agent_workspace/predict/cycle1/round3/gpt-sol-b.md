MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# C1R3：钉死 P3 的 UTC / 本地时语义

## 冻结结论

v0.2 只能保留 **P3-UTC**：桶键中的小时、星期与日期都从事件的 UTC `ts`
推导，输出和评测必须显式标为 UTC。它可以检验“绝对 UTC 时刻是否有区分度”，但不能
据此声称学到了“用户上午九点”“工作日上午”或本地作息。

**P3-local 是以后受 schema 前置条件约束的另一个变体，不是 P3-UTC 的改名。**
在每条会话都能提供事件发生时的本地时偏移之前，P3-local 不可训练、不可回填，也不可
作为候选目录中的已可测项。

代码面的理由是确定的：前台会话开始时刻经
`rfc3339_utc(...)` 写成带 `Z` 的 RFC 3339 `ts`
（`crates/soul-collect/src/collector.rs:241-247`；
`crates/soul-policy/src/clock.rs:31-39`）。当前记录没有事件时的 UTC offset、时区 ID
或本地 civil time。UTC instant 不能唯一反推出它曾对应哪个本地小时。

## ADV-DST 的硬判据

`America/New_York` 只作为隐藏 oracle，不得进入当前生产等价输入。

| 事件 | 已存 UTC | 当前 P3-UTC 桶 | 未来显式纽约语义 |
|---|---|---:|---|
| D1–D3 `alpha` | 2026-10-26/28/30 13:00Z | UTC 13，各 1 次 | 09:00 EDT，offset −04:00 |
| D4 `fold-a` | 2026-11-01 05:30Z | UTC 05 | 第一个 01:30，EDT，offset −04:00 |
| D5 `fold-b` | 2026-11-01 06:30Z | UTC 06 | 第二个 01:30，EST，offset −05:00 |
| D6–D8 `alpha` | 2026-11-02/04/06 14:00Z | UTC 14，各 1 次 | 09:00 EST，offset −05:00 |

因此当前 oracle 必须得到：

- `(alpha, UTC hour 13) = 3`；
- `(alpha, UTC hour 14) = 3`；
- D4 与 D5 分别属于 UTC 05、UTC 06。

当前输入下把六个 `alpha` 合成“本地 09 点 = 6”是 oracle 泄漏。把 UTC 13/14
直接渲染成“本地 13/14 点”同样失败。固定使用 UTC−04 也失败，因为 D6–D8 会被错误
放到本地 10 点。

只有未来记录了真实的事件时偏移后，本地 oracle 才是
`(alpha, local hour 09) = 6`。D4、D5 可同属 local-hour 01 桶，但原始 UTC instant
和 offset 仍须保留；它们是 fold 两侧的两个不同事件，不能折成一个事件。

## P3-local 日后必须存什么

### 不可再少的历史事实

每条会话开始事件至少需要：

```text
started_at_utc
utc_offset_seconds_at_start
```

`utc_offset_seconds_at_start` 必须在采集时从系统有效时区取得，是带符号的数值快照，
不能日后用“机器当前时区”回算。与 UTC instant 合起来，它足以稳定重建该事件当时的
本地日期、星期和小时，也能用 −04:00 / −05:00 区分 ADV-DST 的两个 fold 位置。

仅存当前机器时区、仅存一个用户配置时区，或仅存 `local_hour=9` 都不够：用户可能旅行、
修改时区，规则也可能变化；全局值会重写历史，单独小时则丢失日期、星期和 fold 证据。

### 若要声称“某个命名时区”，还必须有

```text
time_zone_id_at_start
time_zone_id_namespace
time_zone_rules_or_mapping_version
```

数值 offset 冻结“当时墙钟是多少”，但不能说明它是纽约、不能预测下一次 DST 转换。
命名时区 ID（IANA，或明确标注 namespace 的 Windows ID）承担该语义；规则库/映射版本
使重建可复现。稳健方案是 **UTC instant + 当时 offset + 命名时区 provenance** 全存，
而不是在 offset 与 zone ID 之间二选一。若产品只承诺“设备当时的本地小时”而不承诺
命名时区，则前两项是最小契约，命名时区字段不是该窄承诺的必要输入。

这些字段属于前台行为事件的来源事实，应与该事件使用相同同意、密封和遗忘生命周期；
不得从 IP、位置或其他未授权信号猜时区。

## 模型与迁移契约

1. 每份 P3 派生状态必须带不可省略的 `bucket_basis`，至少区分
   `utc_v1` 与 `local_at_event_v1`。两种计数不得合桶。
2. 查询也必须使用同一 basis：P3-UTC 取查询 instant 的 UTC 小时；P3-local 取查询时
   设备实际 offset 对应的本地小时。不能用 UTC 训练、用本地小时查询，反之亦然。
3. schema 升级前的旧事件没有 offset，不能借今天的 zone ID 回填成 local。它们只能
   留在 P3-UTC，或在 P3-local 中作为缺字段而排除并触发既定 backoff/弃权。
4. 从 P3-UTC 切到 P3-local 必须新建并重算模型版本；禁止只改标签继续使用旧计数。
5. prequential 评测在每个预测点只能读取当时已记录的 offset/zone provenance，隐藏的
   ADV-DST 纽约列永远只是评分 oracle。

## 放行测试

- **UTC production parity**：没有 offset/zone 输入时，只产生 13/14（以及 05/06）
  UTC 桶，不出现 local、morning 或一只六计数桶。
- **DST-aware local**：逐事件注入上述 offset 后，六个 `alpha` 全落 local 09；
  D4/D5 同为 local 01，但保持两个 instant 和两个 offset。
- **fixed-offset killer**：固定 −04:00 必须在 D6–D8 上失败。
- **current-zone replay killer**：采集后修改机器时区再重放，历史 local 桶必须不变。
- **migration guard**：无 offset 的旧行不得进入 `local_at_event_v1`，UTC 派生状态也
  不得通过改名冒充 local 状态。

最终候选登记应写成：**P3-UTC 可测、诚实标注但不代表本地作息；P3-local
schema-gated，待逐事件 offset（以及命名时区承诺所需 provenance）落库后再测。**
