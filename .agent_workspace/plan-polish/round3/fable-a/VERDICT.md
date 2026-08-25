# 裁决

MODEL_SLUG: claude-fable-5-thinking-xhigh

## **PLAN_DOCS_FROZEN_FOR_MAIN**

残留 P0：**无。**

核于 2026-08-25。适用对象：HEAD `27b4060` 的树 **加上** opus-a 尚未提交的三文件措辞修复（`docs/FORMAL_WORK_PROMPT.md`、`docs/PLAN_INDEX.md`、`docs/STATUS.md`），确切字节见 `REPORT.md` §0 sha256 表。

依据：SOTA 六维全 PASS（`SOTA_ACCEPT.md`）；13 切片 × AC-01–34 × D1–D59 交叉核验零现存矛盾（`CROSS_CHECK.md`）；独立探针 10 组全绿，含 schema 行为探针 12/12、lock 哈希 11/11、`cargo test --workspace` 249 通过（`REPORT.md` §1）；Round 3 四个并行槽位（opus-a、opus-b、gpt-sol-a、gpt-sol-b）无一报出冻结级缺陷。

## 冻结的确切含义

管**计划权威面可以合 `main`**，只管这一件事。它不是 Goal 1 关闭（D54 两道门都没过）、不是 `main` 有应用（没有）、不解除 `BLOCKERS.md` 任何合入/关闭项。

## 合入前置（本判定的生效条件）

1. 父代理把工作区里 opus-a 的九处未提交修复**原样**提交进本 PR。提交后若字节与 `REPORT.md` §0 哈希不符，本判定对差异部分失效。

## 合入后的既定动作（非本判定新增，只是收拢已有留痕）

1. PR #6（BLOCKERS）随后合入，按 DECISIONS 脚注处理其文中「D32」撞号。
2. COPY_ZH P4 的 `>` 写法修补（opus-b F1 补丁已备好）由父代理分配 D 号后单独落地——main 侧既有措辞债，与本 PR 无关，但不要让它悬着。
3. 远期时间戳污染 as_of（opus-b F4）单独立项为导入层拍板；**禁止**用墙钟上界夹 as_of 的写法（违反 D45）。
4. Goal 1 merge main 后跑 `xtask schema-freeze` 复核 lock（D59 尾句）；届时再议 R-3/R-4/R-5 的 schema 层收紧。
5. Goal 2 在 Goal 1 关闭前不启动。
