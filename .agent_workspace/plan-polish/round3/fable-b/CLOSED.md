# Round 3 fable-b — Round 2 遗留逐项判定（closed / open）

核于 2026-08-25，基线 `c3d5960`（Round 2 落地提交）。审计中另一 Round 3 槽位在同树在飞修改了 FORMAL/PLAN_INDEX/STATUS（未提交）；本文件全部判定针对**提交版字节**，探针在 `c3d5960` 干净 worktree 与在飞树各跑一遍、93/93 逐条一致（详见 REPORT.md「同树并发声明」）。
判定依据：只读 `docs/` 提交版字节 + 本目录独立探针（`probe/RESULTS.txt`）+ `git ls-remote` 实测。
**特别说明**：`round2/opus-a/RESULTS.txt` 的「57/57 passed」跑在中间稿上（其记录的 relationship 哈希 `350a7ffc…` ≠ 落地的 `ebf049a9…`，其 schema 副本无 `machine_band`）。本轮全部结论以最终落地字节重跑得出，不采信该份证据。

## 一、R2-SYNTHESIS 表内声称

| 项 | 声称 | 判定 | 证据 |
|---|---|---|---|
| L1 `tie_strength` 类型化 + 三锁定字段 | 关闭 | **关闭（实证）** | schema 82–104 行三字段在 `properties`；探针 `struct/lock_field_*` 与 `accept/rebuilt_unlocked_edge_with_machine_band` 过；unblock 线 `build.rs::tie_strength_of` 逐字段比对（`machine_band: Some(machine_band)`），重建边形态在最终 schema 下 valid |
| L1 lock 重算 | 关闭 | **关闭（实证）** | 独立 `sha256sum`：lock 内 11 条哈希与在盘字节逐一相等，含 `relationship = ebf049a9…`；lock 覆盖目录内全部 schema 文件 |
| L1 四条探针（空对象过 / score 拒 / 半迁移拒 / T4D+machine 过） | 过 | **复现（最终字节）** | `accept/empty_tie_strength_object`、`reject/score_field`、`reject/half_migration_band_only`、`accept/rebuilt_unlocked_edge_with_machine_band` 全过 |
| L2 FORMAL 历史段 / PLAN_VERIFY 横幅 / 唯一开工路径 | 关闭 | **关闭** | FORMAL 66–83 行历史段（标题 + 引用块导言 + 尾水印）；「开工第一动作（本文件唯一可执行的开工路径）」167 行；PLAN_VERIFY 第 3 行横幅 + 83 行尾注；PLAN_INDEX 43–44 行把两者列进「不是权威」；README 对 PLAN_VERIFY 补「历史存档」 |
| L3 SECURITY 指针节；Goal 1 尖端统一 `df5d2dd` | 关闭 | **关闭（实时核对）** | SECURITY 23–39 行指针节，两节实证明确「不能引用为已完成」；STATUS 19 行与 SECURITY 27 行同写 `df5d2dd`；`git ls-remote` 实测 `cursor/soul-goal1-7b1c = df5d2dd6…`，当下仍一致 |
| L4 过程目录留快照 + README 非权威 | 保留 | **符合声称** | `.agent_workspace/context/plan/` 7 份 + README 首行「历史快照，不是权威」；PRODUCT_LOCK 哈希分叉属预期（docs 前进） |
| L5 BLOCKERS 不同树；脚注定 PR #6 处理撞号 | 部分关闭 | **本分支可做部分已关闭** | 本树无 `docs/BLOCKERS.md`（PLAN_INDEX 30 行如实声明）；DECISIONS 72 行脚注写明合入时改写或加括注；远程 `e669b78:docs/BLOCKERS.md` 第 26/56 行仍用「D32」指 e0 禁令——撞号本体按设计留给 PR #6 合入时 |
| L6 产品路径 AC 本分支不可跑，以探针代替 | 仍成立 | **维持原判** | AC-28–34 的产品边界测试在 Goal 1/unblock 线；本轮以 schema/文档探针复核存在性与一致性 |
| D52 `180` → `DEMOTE_ONE_BAND_DAYS` | 关闭 | **关闭** | DECISIONS 63 行；探针 `decisions/D52_constant_name_no_literal` 过；核心八文件（PRODUCT_LOCK/DECISIONS/FORMAL/PLAN_INDEX/SECURITY/STATUS/PLAN_VERIFY/README）`180|360` 零命中；常量名在 `crates/soul-algo-tie/src/constants.rs:57` 真实存在 |
| G1+ 补 AC-34 | 关闭 | **关闭（计划面）** | FORMAL 157 行 AC-34 存在，引 D47/G1+；实现归 Goal 1/PR #7（synthesis 风险 3 原话），不是本分支义务 |
| 追加 D59 | 关闭 | **关闭** | DECISIONS 70 行；D58（69 行）原文经 diff 证实未改动 |

## 二、round2/fable-a/RESIDUAL.md 逐项

### P0

| 项 | 判定 | 证据 |
|---|---|---|
| P0-1 缺三锁定字段致 Goal 1 重建边必红 | **关闭（实证）** | 三字段进 `properties` 且不进 `then.required`（探针 `struct/lock_field_*_not_conditionally_required`）；`dependentRequired.user_band=["locked_by_user"]` 照 RESIDUAL 处方钉上；`additionalProperties:false` 未动；lock 同批重算且独立复算相符。合并红点矩阵实测：主线 `df5d2dd` 八字段边 valid（`accept/mainline_eight_field_edge`）；unblock 线重建边（未锁/锁定/释放三态，按 `build.rs`/`correct.rs` 逐字段构造）全 valid；unblock 线 `t4d_band.rs` 的 legacy 八字段 put 夹具 valid；纠正类测试全部先 `rebuild` 再纠正，不触 legacy 写回。**残余潜在红点见 REGRESSIONS.md R-1（非 P0 复活）** |

### P1

| 项 | 判定 | 证据 |
|---|---|---|
| P1-1 头部增补注范围写错（D41–D56） | **关闭** | 改为「**D41 起**」（RESIDUAL 给的防再犯备选）；`D41–D56` 全文无命中 |
| P1-2 STATUS 阻塞表与同树 schema 自相矛盾 | **关闭** | STATUS 52 行「已关闭：…（D58/D59），含 machine_band / user_band / locked_by_user」，与同树 schema 字节一致（探针 `status/schema_row_closed`） |
| P1-3 收紧无拍板留痕 | **关闭** | D59 记收紧 + 三字段义务 + `xtask schema-freeze` 复核义务；D58 原文未动（拍板只追加，diff 证实）；PLAN_INDEX 24 行改「见 D58/D59」 |
| P1-4 BLOCKERS 合入引爆 D32 撞号 | **关闭（本分支侧）** | DECISIONS 72 行由「樱桃摘时按下一个空闲 ID 重新编号」升级为 PR #6 整份合入检查单（改写或括注 + 禁复用本表号）——正是 RESIDUAL 处方「现在留痕、PR #6 合并时执行」。执行本体留在 PR #6，无法在本分支关死 |
| P1-5 D40 丢「锁定边仍丢掉 filed_band」 | **关闭** | DECISIONS 51 行已补该半句（加法，结论未动）；探针 `decisions/D40_keeps_filed_band_loss` 过 |
| P1-6 同树 Goal 1 尖端三个值 | **关闭（实时核对）** | 同一提交内 STATUS `3161e02→df5d2dd`、SECURITY 新节即写 `df5d2dd`；D57 补「同一棵树内对同一分支的『核于』提交号必须一致」（68 行）；`git ls-remote` 当下实测仍是 `df5d2dd`。**注**：D57 是在原行内增补而非追加新号，且 STATUS 尾部对 D57 的复述未同步该半句——见 REGRESSIONS.md R-3 |

### P2

| 项 | 判定 | 证据 |
|---|---|---|
| P2-1 AC-28 分列计数简写名 | **关闭** | FORMAL 151 行改 `direct_out_count` 等全名（探针 `formal/AC-28_uses_full_count_names`），与 schema 字节、`model.rs` 字段名逐一相同 |
| P2-2 D52 引 `180` 字面量 | **关闭** | 同上表 D52 行 |
| P2-3 「九份」计数 | **关闭** | PLAN_INDEX 24 行「十份正文（九存储 + 一导入）+ `_defs` + lock」定义清楚；FORMAL 11.5 去掉计数改「（含 lock）」；README 去「九份」；D26 作为历史拍板保留「九 schema」——与 RESIDUAL「一处定义清楚，其余不必动」相符 |
| P2-4 历史段原话本体未入引用块 | **关闭（取备选 b：水印行）** | 提交版：节标题自带「不要照着再跑一遍」+ 引用块导言 + 尾行「（历史段到此为止。以下各节仍然有效。）」；作者原话与 1–6 步逐字保留为正文格式，纯扫描式读者理论上仍可能撞见裸编号清单，但前后水印已成对，接受。**后记**：同树在飞改动（未提交）已把原话与清单整体入引用块——待提交后按 a 方案彻底关闭 |
| P2-5 L4 快照漂移成旧版 | **未关闭（拍板保留）** | R2-SYNTHESIS 拍「保留快照 + README 非权威」；RESIDUAL 的 a（删七份正文）与 b（不被引用断言探针）都没做。横幅在（探针 `l4/snapshot_readme_banner`），风险维持 P2 |
| P2-6 STATUS M2 行可以更准 | **未关闭，但无假话** | 实测 `df5d2dd:Cargo.toml` 成员表确无两个 soul-algo crate——M2 行在刷新后的尖端仍为真；「unblock 线已做（`c81c233:Cargo.toml` 20–21 行实测已含）」的注记未补。纯精度遗留。**后记**：同树在飞改动已补该注记（与本轮实测一致），待提交 |
| P2-7 L6 保持原判 | **维持** | 无动作项；本轮未见有人把它误升级为本分支阻塞 |

## 三、R2-SYNTHESIS「留给 Round 3」清单的执行情况

| 要求 | 结果 |
|---|---|
| 只读交叉核验 + 负向探针 | 本目录探针 93/93：OAuth 极性（核心文件仅出现于「无/砍/v0.2」语境）、E0（SECURITY「构建期消除」、红线 1）、文件写（红线 7、PRODUCT_LOCK 147 行「文件写执行」在砍单、SECURITY「不消费写文件令牌」）、`180|360` 常量泄漏（核心八文件零命中）、lock 哈希（11/11 相符）、AC-28–34 存在性（7/7 行在、AC-27 无表行） |
| 不把 BLOCKERS 整份拷进本 PR | 守住：本树无 `docs/BLOCKERS.md` |
| 不改产品方向 / 不加 F04c 第三道门 / 不启动 Goal 2 | 守住：R2 提交未触 `PRODUCT_LOCK.md`；F04c 禁令在 FORMAL 49 行与 PLAN_INDEX 52 行原样；Goal 2 未启动（STATUS 38 行） |
| 新父代理只读 `PLAN_INDEX.md` 能正确开工 | 成立：索引 25 行直接拦「重开 Goal 1 / 重派 planner」，14 行指明开工只认「开工第一动作」，43–44 行把两个历史陷阱点名 |
