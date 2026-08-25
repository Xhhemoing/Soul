# Cycle 2 Round 3 综合（BINDING 于调研轨）

Parent: cursor-grok-4.6-high。已收：opus-a/b、gpt-sol-a/b、fable-b。**fable-a 本轮未到，不等。**
对外入口：`../ESSENCE.md`。

## 冻结下来的结构

1. 文件后继共享形状：**只从今天的 move 集合里减，从不新造 move**（S1-b 提升否决）。
2. 短路：S1（KindDisputed）在 S3（RecentlyModified）之前；吃 as_of 的理由排最后。
3. `as_of_day` **不进** `plan_hash`。gpt-sol-a 给出跨零点判决测试与反向对照。
4. S1 默认关：零读上界造不出（撒谎频率必须读字节）。形状门今天可测，收益线饥饿。
5. S2 默认只注解。opus-b 否决「移入 `重复/`」；该分歧保留给测试日，见 ESSENCE §2.5。
6. n2A 退位（面 A 上 ≡ MRU）。学习下一动作阻塞于 G17（无 forgettable 动作历史）。
7. 进程身份：无新可测算法。
8. gpt-sol-b：三个 `ActionKind` 有 UI 但绕过 `check_action`；`forget.execute` 是跨层不变式缺口。本调研只登记，不修产品。

## 缺口编号（对外以 ESSENCE 为准，避免 R3 两份 opus 撞号）

| ID | 内容 |
|---|---|
| G17 | 无可遗忘动作历史 |
| G18 | 动作事件无毫秒时钟、无法对齐前台会话 |
| G19 | 生产闸门与 UI 词表分叉（改写 R1「无生产者」） |
| G20 | authorized_roots 不在遗忘覆盖内 |
| G21 | 待决状态不可读（只要布尔/计数访问器） |
| G22 | 命令面不是闭集 → 面 B 永不上产品 |
| G23 | 使用剖面无合法标定源（普查只许杀） |
| G24 | 夹具 mtime 是构造时刻 / 写死 NOW_MS 恰在 UTC 零点 |
| G25 | 无「打开文件字节」同意主题（header vs full 两个主题） |
| G26 | 无 READS needle 表 / 快照无 atime |

## 未等的槽位

Cycle 2 Round 3 fable-a：未写入。若日后到达，只许**减**目录或加杀线，不许在未重跑综合的情况下加默认候选。
