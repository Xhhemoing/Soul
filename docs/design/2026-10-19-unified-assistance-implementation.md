# 统一辅助实施与验证

本轮按[原计划 U1–U7](2026-10-16-unified-assistance.md)重建。生产起点为 `600f309`，原计划恢复检查点为 `1433951`；此前被工作区维护清理的未同步实现与测试结论不能作为本轮证据。产品、算法和正式验收权威仍由 PLAN_INDEX 指向既有文档。

## U1/U2：统一事项与显式目标

编码前决定见 D97，方案及实际 dirty 源码由独立只读 reviewer 复核。当前实现：

- schema 4 只添加事项元数据；手工目标复用 Commitment Memory 及其内容密钥，创建和修改与事项更新同事务。观察项以真实 namespace/contact 稳定去重，固定 bundle 的替换由用户明确核对版本。
- 截止只接受明确带偏移的绝对 RFC3339 时间；不解析导入文本中的旧“明天”，不补午夜。完成只记自报，取消独立，旧版本动作不能重开终态。
- 可选来源关联不改变手工目标的独立遗忘单位。来源遗忘使其失效，不删除目标正文；目标自身遗忘照旧销毁 CK。遗忘确认比较完整 item/revision 集合，辅助字段损坏不能阻止密钥销毁。
- 事项列表只有元数据，明确详情才读取目标或可信统计。事项跨页交接只携带 ID/revision，core 创建实际绑定事项与来源的 held context，绑定延续到 pending approval、owned worker 和 completion。已发生 E1 请求先记真实审计，再拒收过期回复；实际 worker 结束前不释放槽。
- 前端增加事项与洞察独立入口。地址代次、payload 所属地址/来源、挂载和选择共同拒绝旧结果；未知写入先核对，不自动重放。普通起草及既有本次材料选择保留。

独立复审和实际测试促成的修正：读取/解密故障不能永久改成遗忘；带观察来源的目标也验证自身 CK；重复关联不能把返回旧固定 bundle 的已知结果误标 unknown；新地址 render 不能继续显示或操作旧 state。最后一项用 layout effect 在 DOM commit 后、passive effect 清理前检查，不只验证旧闭包。

### 实际验证（Linux，Rust 1.83.0 / pnpm 10.15.0）

| 检查 | 实际结果与范围 |
| --- | --- |
| `cargo test -p soul-store -p soul-store-api --locked` | 20 个测试输出套件，97 通过、0 失败；含两个无用例 doc-test 套件 |
| `cargo test -p soulcore --test session_assistance --test session_memory_context --test session_insight_context --test session_draft_work --test session_forget_scope --test session_atomic_edits --test session_workflow_continuity --test session_screens --test session_e1 --locked` | 9 套件 / 80 通过；新增 12 项真实 Session/SQLCipher 用例包含其中，不重复累计 |
| `pnpm --filter @soul/desktop test` | 47 文件 / 561 通过；最后仅调整 Assistance effect 的 state 更新位置后，定向 14 项再次通过 |
| `pnpm --filter @soul/desktop lint`、`build` | TypeScript、ESLint、明确 production 构建均退出 0 |
| 现有 `command_surface.rs` 的纯源码断言 | 6 项通过，核对实际 66 个 IPC 名称、注册及薄转发；独立 rustc 适配只替换 crate 常量导入和 include 路径，不执行 Tauri handler |
| 根/桌面格式、schema-freeze、E0、禁词 | 均退出 0；E0 扫描 16 个交付 crate / 289 个文件，禁词扫描 94 项 / 136 个文件 |
| 原生 desktop / IPC / Windows | 未运行；新增真实 handler 场景目前只有源码复核，不能用静态断言代替 |

首次 Rust 编译出现零长度构建缓存对象；限定清理对应 package 缓存后恢复。首次三包全量测试又遇既有测试可执行文件缺执行位，随后重编译出现两个空 ELF 产物；未把这些中断计通过，仅清理已确认的空缓存文件。上表记录后来完整退出 0 的存储检查与受影响核心定向检查，不声明 `soulcore` 全包已通过。前端首轮 9 项失败已逐项修正；没有放宽安全断言或关闭 lint 规则。

## 后续阶段

U3 的持久工作、事件触发、故障恢复与采集交接已完成独立方案复核，尚未编码。U4/U6、U5 自动本地预备及 U7 集成验证继续依赖前项验收，后续结果在本文件追加。

完整 G-L 仍按已有指示暂缓，G-W/G-M 与真实任务效用未执行；Goal 1、正式 Goal 2、Wave 1 没有因本轮局部验证关闭。未自动推送或合并。
