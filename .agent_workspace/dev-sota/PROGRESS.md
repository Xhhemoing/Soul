# agent/dev-sota — Goal 1 剩余实现

**分支**：`agent/dev-sota`（基于 `origin/cursor/soul-goal1-7b1c` @ `862e858`）  
**目标**：按 Round 1 简报落地 P0（WP10 起草、WP11 只读文件计划、壳接真库），每步写码 → 优化 → 审查。  
**不做**：Goal 2、文件写执行、E0、OAuth、WP13 完整打包（可列缺口）、全面 WP09 功能视图（本轮最多薄 IPC）。

## 循环

| 轮 | 内容 | 状态 |
|---|---|---|
| Plan | 2 fable 拆任务 + 2 opus 接口勘探 + 2 gpt-sol 工装/测试探针 | 完成 |
| Write 波次1 | ST-00 骨架 + ST-03 壳接库 | 进行中 |
| Write | 多 opus-fast 并行写 crate/测试 | 待 |
| Optimize | 修复 + 补测 | 待 |
| Review | fable 全局 SOTA review | 待 |
| PR | 提交并在可验证后合并 | 待 |

## 约束

- 子代理第一行声明实际模型 slug；禁止静默降级。
- 实现者：`claude-opus-5-thinking-high-fast`。规划/审查：`claude-fable-5-thinking-xhigh`。探针：`gpt-5.6-sol-xhigh-fast`。
- 权限/数据面与 UI 面分人。UI 不含业务逻辑。
- `soul-fileplan` 无写 API。起草永不发送。
- 无 DPAPI 时壳可用 `TestKeyProvider`，UI/SECURITY 必须如实写，不得声称 KEK 已保护。
