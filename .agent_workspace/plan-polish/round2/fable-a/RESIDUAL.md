# Round 2 fable-a — 残余缺陷清单（P0/P1/P2）

核于 2026-08-25 02:50 UTC，在飞基线（`b380f67` + 未提交 Round 2 改动）。每条附最小修法；全部不改产品方向、不加第三道降档门、不新增工作包。

## P0

### P0-1 类型化 `tie_strength` 缺 D32/D48 锁定字段，会让 Goal 1 每条重建边验证失败

**事实链（全部实测）**：

1. 在飞 `relationship.schema.json` 的 `tie_strength` 枚举 15 个属性并 `additionalProperties: false`。枚举表里**没有** `machine_band`、`user_band`、`locked_by_user`。
2. D32 拍板「未锁边也写 `machine_band`，重建边常为 `Some`」；D48/AC-32 拍板「生效档 = 用户锁定档，机器档另存且继续更新」。
3. unblock 线（PR #7，Goal 1 吸收 T4D 的那条线）已照 D32 实现：`crates/soul-graph/src/model.rs` 的 `TieStrength` 含这三个字段，`build.rs:161` 每次重建写 `machine_band: Some(...)`（`skip_serializing_if = Option::is_none` 救不了恒 `Some` 的字段）。
4. Goal 1 的 `soul-schema` 用 `include_str!("../../../docs/schemas/…")` 把 schema 字节编译进 crate，并用 `jsonschema`（Draft 2020-12）验证实例。schema 字节合过去的那一刻，**每条重建边、每个 roundtrip 测试**都会被 `additionalProperties:false` 拒绝——CI 立刻变红，且红的原因是计划文件自己违反了计划自己的 D32。

**最小修法**（本分支、合并前）：在 `tie_strength.properties` 追加三个字段——`machine_band`（同 band 的枚举约束）、`user_band`（同上，或 null）、`locked_by_user`（boolean）；三者**不进** `if/then` 的 required（锁定是可选状态）；可在 `dependentRequired` 里钉 `user_band: ["locked_by_user"]` 与 D32 的 `locked ⟺ user_band.is_some()` 对齐。然后**同批**重算 `schemas.lock.json`（本分支手工/脚本均可，Goal 1 侧合并时由 `xtask schema-freeze --write` 复算复核）。改完把「附加字段即缺陷」的执行点保住：`additionalProperties:false` 不动。

**为什么是 P0**：这是唯一一条「合并即红、且红在别人家 CI」的缺陷；其余所有残余都只是文字账。

## P1

### P1-1 DECISIONS 头部增补注范围写错

头部写「**D41–D56** 为本计划打磨轮次…」，表实到 **D58**（D57 STATUS 纪律、D58 schema 义务在范围外）。防撞装置自己记错账，后来者会以为 D57/D58 另有出处。**修法**：改成 D41–D58（或改成「D41 起」以免再犯）。

### P1-2 STATUS 阻塞表与同树 schema 字节自相矛盾（在飞新引入）

阻塞表行「schema 倒退 | Round 1 已改用 Goal 1 接线版；`tie_strength` 仍为裸 object（D58）」——在飞树上后半句已假。「Round 2 · 其余」行自己声明「状态以那一路落盘为准」，但那一路落盘后没回填。**修法**：schema 槽位落定（含 P0-1 修复）后同一提交更新该行为「已类型化（D58 + 新拍板号），待 Goal 1 同批重冻」。

### P1-3 D58 停在收紧前时态，收紧本身无拍板留痕

D58 原文承诺「进一步收紧须与 Goal 1 `schemas.lock.json` 同批重算」——收紧已发生、lock 已在本分支重算，但 DECISIONS 无一行记录：收紧的字段清单已进 schema 字节、采用 `dependentRequired`+条件 required 的兼容策略、Goal 1 合并侧须 `xtask schema-freeze` 复核、以及 P0-1 的三个锁定字段。**修法**：追加 **D59**（下一个空闲号）记录本次收紧与同批义务；D58 保持原文不改（拍板只追加）。PLAN_INDEX「收紧见 D58」相应改「见 D58/D59」。

### P1-4 BLOCKERS 合入将引爆 D32 撞号

`origin/cursor/blockers-analysis-a073:docs/BLOCKERS.md` 正文两处用「D32」指 dev-sota 线的 e0 发送 crate 禁令；本树 D32 = machine_band。DECISIONS 脚注已声明「不是本文件的 D32、樱桃摘时重新编号」，但 PR #6 是整份文件合入，合入当天 `docs/` 里就会同时存在两种 D32 语义。**修法**（现在留痕、PR #6 合并时执行）：给 PR #6 的合并检查单加一条——合入时把 BLOCKERS 里的「D32」改写为重新编号后的 ID（届时的下一个空闲号），或加「（dev-sota 编号，非 docs/DECISIONS.md 之 D32）」括注。本分支现在可在 DECISIONS 脚注补一句「PR #6 合入时按此处理」。

### P1-5 D32–D40 与 unblock 线同号同义但措辞漂移

逐条比对：D34（对方多「R4/R5」「记 STATUS 遗留」）、D36/D38（对方多半句理由）、D39（对方写「WP03 遗留 8」）、D40（对方多「锁定边仍丢掉 filed_band」一个语义点）。同号同义成立，但 D40 那半句是**语义**不是措辞——本表丢了它。**修法**：把「锁定边仍丢掉 filed_band」补进本表 D40（加法，不改结论）；其余措辞差留给合并时取长版。

### P1-6 同树「核于同日」的 Goal 1 尖端三个值

STATUS 写 `3161e02`，在飞 SECURITY 写 `6c91d39`，远程实际 `52431d6`（今日已又推三个提交：IPC 面测试与 hosted AC-26 注记）。各自符合 D57，但同树两说让「核于」失去公信。**修法**：合并前做一次统一刷新（一个提交里把两处指到同一尖端同一核于时刻）；给 D57 加半句「同一棵树内对同一分支的『核于』提交号必须一致」。

## P2

### P2-1 FORMAL AC-28 分列计数用简写名

AC-28 写「`direct_out/direct_in/group_out/group_in` 四个分列计数」；schema 字节与 unblock 实现都是 `*_count` 全名（`direct_out_count`…）。门禁行与契约字节对不上名，实现者要多猜一次。**修法**：AC-28 改全名（门禁只增不删原则下这是澄清不是改判）。

### P2-2 DECISIONS D52 引 `180` 字面量

「产品 crate 禁止再写第三个 `180`」——用数值指认常量，恰是红线 11 禁的「只是复述一下」。**修法**：改写为「禁止再写第三处 `DEMOTE_ONE_BAND_DAYS` 字面量」。

### P2-3 「九份」计数与目录不符

`docs/schemas/` 现有 10 份正文（九份存储契约 + `soul-import-v1` 导入契约）+ `_defs` + lock。PLAN_INDEX/D26/FORMAL 11.5 三处「九份」沿用 PR #1 口径。**修法**：一处定义清楚（如 PLAN_INDEX 写「十份正文（九存储 + 一导入）+ `_defs` + lock」），其余两处不必动——D26 是历史拍板，FORMAL 11.5 改一次即可。

### P2-4 FORMAL 历史段的原话本体未入引用块

引导块是引用格式，但作者原话与其后的 1–6 步清单是正文格式；纯扫描式读者仍可能把编号清单当活步骤。**修法**：把原话与清单整体缩进为引用块或加「存档」水印行；作者原话逐字保留的约束不受影响。

### P2-5 L4 快照已从「双源」漂移成「旧版」

`context/plan/` 七份副本与 docs 全部 differs（docs 前进所致）。横幅在，但没有任何机制阻止有人 diff 出旧版内容并当真。**修法**（择一）：a) 删除七份正文只留 README 横幅（快照价值已由 git 历史承担）；b) 落 R1 说的探针：CI/脚本断言该目录不被任何 docs 文件引用。倾向 a——最便宜的单源。

### P2-6 STATUS 的 M2 行可以更准

「未做：Goal 1 `Cargo.toml` 成员表里没有两个算法 crate」对 `cursor/soul-goal1-7b1c` 为真；但 unblock 线已在合并提交 `b3cb6d4` 完成 M2（成员表含两个 soul-algo crate，实测）。下次刷新 STATUS 时补「unblock 线已做，待回主干」。

### P2-7 L6 保持原判

AC-28–33 在本分支仍只有夹具名存在性可验；产品路径在 Goal 1/unblock 线（已在做，`t4d_adapt.rs` 实测存在）。计划义务定性正确，无需动作，仅防有人误把它升级成本分支阻塞。

## 修复顺序建议

P0-1 →（同一提交）P1-2 + P1-3 → P1-1 / P1-5 / P1-6 →  P2 按顺手。P1-4 留到 PR #6 合并检查单。
