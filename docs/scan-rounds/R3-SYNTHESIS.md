# R3 综合稿

模型：
- [R3 opus plan verify](bc-bea41d97-3b1c-5de9-b306-721b3edf466e) `PLAN_FROZEN`
- [R3 sol plan verify](bc-75c9efdf-2a49-5dbb-8708-75daae90d989) `PLAN_BLOCKED`

父代理仲裁后补了 schema/验收缺口，结论改为 **`PLAN_FROZEN`**。

## 仲裁

| sol 异议 | 处理 |
|---|---|
| schema 未真正禁止明文/空 privacy | 已收紧 event/audit/export-manifest；补 `soul-import-v1.schema.json` |
| 研究预览可自报 0 行 | 预览必须带 `rows`；research_preview 时 `written_to_disk=false` |
| E0/E1 失败矩阵不全 | AC-11/12/21 补精确 origin、姓名账号、跨 origin |
| 遗忘/审计不可证明 | AC-15 含影响面、CK 销毁、审计仍在 |
| 空采集器可通过 AC-09/10 | AC-10 要求开启期间至少 1 条事件 |
| 删除 WP11/AC-18 | **否决**（D31）。只读预览留在 Goal 1，写执行仍在 v0.1.1 |

Goal 2 已拆出（双方同意）。

## 仍留到 WP01 的 P1

- 用 validator 把九份 schema 的 `$ref` 真正接到 `_defs`
- 补 Unicode/短文本泄漏 fixture
- 崩溃注入点写进测试 harness

这些不阻止冻结：契约与门禁已可执行。
