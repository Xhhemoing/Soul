# 沟通意图与证据共同准备：独立评审回执

日期：2026-10-21。候选：`codex/soul-rebuild-20261019` @ `6a56d852d23a051c7c4e9845642791295c541105` + dirty。详细边界、命令及原始日志入口见[实施与验证](../design/2026-10-21-communication-joint-preparation.md)。

主线程记录实际评审结果；核心实现者为 `/root/communication_probe_impl`，桌面实现者为 `/root/communication_ui_impl`，两者串行修改共享授权链。独立只读评审者 `/root/optimization_challenge` 先看任务和实际差异，再看实现说明；用开工前仓库外副本区分本片与已有 dirty 修改。评审者未修改实现、未运行构建，所有实际执行由主线程安排。

## 发现及修复

- **超限目标阻断纯统计准备，已关闭。** 首版要求目标选择必须恰好返回一条，导致超过既有 16 KiB 选择预算的目标连 counts-only 准备也无法使用。修正为保留本地 excluded preview，仅对可用目标持有可纳入快照；目标不可用时拒绝 `include_goal=true`，统计许可仍独立。
- **纳入目标时漏记既有注入标记计数，已关闭。** 首版组合经过脱敏但遗漏 memory 路径已有的标记计数审计。修正为仅扫描实际纳入的目标，审计仍只携带计数；未选择的目标不触发该计数。审计写入失败丢弃准备，不残留旧批准。

两项均在 `intent-20261021-163229.log` 实际复现为 8 passed、2 failed；修复差异经独立复审，`intent-20261021-163344.log` 十项全部通过。评审者核对了具名反例测试及 EXIT=0，没有用静态推测替代复现。

## 覆盖面

核心审查覆盖绑定目标唯一身份、OwnerExcerpt 分类与原文豁免隔离、目标／统计独立许可、固定 system 与引用数据段、64 KiB 完整请求预算、失败时旧准备清除、独立 memory/insight pending 互斥，以及准备／派发／完成三个阶段的来源复验。内部组合复用原 worker 的 memory 依赖，不开放任意前端 memory ID 或正文。

桌面审查覆盖 Tauri 可选布尔值缺省 false 的薄转发、精确单条 Owner 才显示可选目标、重新核对重置许可、当前事项／来源／路由／粘贴版本与旧 DOM 防护，以及最终预览采用 core 实际返回的材料。持久编辑入口明确说明会修改已保存目标。未发现新的具体缺陷。

## 已独立核对的运行证据

- `core-20261021-165129.log`：Box 修正后五个相关 crate，760 passed、0 failed、12 ignored、0 filtered、EXIT=0；包含新增十项，不是整个根 workspace。旧材料诊断、规模和人工入口等仍 ignored，不算本轮执行。
- `frontend_intent-20261021-164315.log`：两个指定文件，13 passed、EXIT=0；不与后续完整前端重复累计。
- `desktop-20261021-165524.log`：Box 修正后独立桌面 workspace 的 `--all-targets`，133 passed、0 failed、0 ignored、2 filtered、EXIT=0。明确排除两项真实采集测试；新增三项 `communication_goal` IPC 测试均具名通过，也实际核对 Box 的 JSON 兼容性。
- `frontend-20261021-164613.log`：完整前端 57 个文件、698 passed、EXIT=0；已包含定向十三项。

- `lint-20261021-164845.log`、`build-20261021-164915.log`：前端类型／lint 和实际 production build 均 EXIT=0；此后前端源码未变。
- `clippy-20261021-165102.log`、`desktop_clippy-20261021-165621.log`：两个 workspace 的限定严格 Clippy 均 EXIT=0。首轮 enum 体积问题通过新增预览字段装箱修正，未加 lint 豁免；此最小差异也经独立复审。
- `format-20261021-165654.log`：根格式 EXIT=0。桌面首轮 `desktop_format-20261021-165703.log` 因新增 IPC 测试两处排版差异 EXIT=1，主线程仅修缩进与换行，复查 `desktop_format-20261021-165736.log` EXIT=0。

本回执不签发正式 G-W/G-L/G-M，不关闭 Goal 1/U7/Goal 2。真实模型效果、真人效用、Windows UI 人工观察及未过滤采集仍未验证。首次失败和修复均保留于实施回执，不把旧成功迁移给 Box 修正后的源码，也不把 repeated／targeted 测试重复累加。

最终独立评审已收口，所有上列最终日志均经核验，当前没有未关闭的具体功能缺陷或检查阻塞；之后才将检索实验草稿应用到仓库并运行。
