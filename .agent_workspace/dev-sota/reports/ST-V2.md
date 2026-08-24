MODEL: Cursor Grok 4.6

# ST-V2 起草页深化 —— 完成报告

分支 `agent/dev-sota`。glob 内文件；共享文件未碰。

## 1. 结论

`/draft` 从骨架变成可断言的产品面：「永不发送」钉在类型（`never_sent: true`）、源码（唯一构造写死 true、无 Deserialize、无 setter）和测试三层。页面只渲染核心原话。注入粘贴仍出草稿，链上有 `InjectionBlocked` 且无正文。

## 2. 改动

| 文件 | 改动 |
|---|---|
| `crates/soulcore/tests/draft_view.rs` | +5 条：fixture 泄漏、注入、never_sent 源码级、多段 turns、拒绝不回显粘贴 |
| `apps/desktop/src/routes/Draft.tsx` | 增加「清空」（规范允许的唯一第二种按钮） |
| `apps/desktop/src/routes/Draft.test.tsx` | 新建：生成/空粘贴/无发送控件 + forbidNetwork |

未改 `commands/draft.rs`（构造级承诺已在 V0 落地，本包用源码扫描钉死）。未改 `fixtures/draft/**`（复用既有）。

## 3. 绿灯

- `cargo test -p soulcore --test draft_view` 10/10
- `pnpm --filter @soul/desktop lint` / `test`（含 Draft.test.tsx）
