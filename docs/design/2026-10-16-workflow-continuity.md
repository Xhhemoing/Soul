# 本地工作流连续性与精确导航

起点为 `413790daaadeab933879406b8d6c01c94c5053b9`，PR #66 草稿。W1–W5 的已实现范围与 720 / 437 项检查见前轮记录；本轮补已核实的使用与验证缺口，不沿用旧绿灯为新源码背书。

## 编码前计划

### 真实 Session 组合轨迹（PLAN REVIEWED）

新增独立 `crates/soulcore/tests/session_workflow_continuity.rs`，复用现有公开 Session、临时 SQLCipher、合成 fixture 和 loopback MockLlm。不可变 prepare/ID-only confirm → 同操作回执幂等及同 namespace 重导去重 → 自动本地候选 → 指定实例 ignore → 用户关系锁使旧候选失效 → 新 UUID 重算 → 手写记忆明确候选及本次用途 → 本机模板/一次批准的端点起草 → 遗忘 → 释放所有持有者并重启核验。

检查实际事件/证据/候选及历史回执，不仅检查函数成功。端点路径在本机用途消费后重新取得 context，核对实际 wire 的第三人占位及明确 Owner 节选、一次批准仅一次请求。遗忘前保留一份准备，遗忘及重启后旧 context/批准不能出站，endpoint 不恢复。导入 peer 的遗忘明确使用既有 store 领域接口，当前没有 contact forget 的 Session/IPC 入口；测试不伪称交付了这个界面。重启后的 revoked namespace 拒重导，旧候选只保历史计数且不能重算，审计无正文，网络观察只允许 loopback。

旧 AC-21 headless 仍保留原用途与步骤；该旧低层主流程不证明本轮 Session 新链路。组合测试不是新 gate，不改变既有算法、schema、API 或 Goal 关闭条件。

### 洞察到精确关系与证据条目（PLAN REVIEWED）

目前洞察只有通用图谱链接，Graph 的关系卡未显示两端已有提示，多个近似计数候选无法明确对应纠正对象。使用已有 contact/evidence UUID 建立有界本地导航目标，图谱对实际加载数据核对目标、显示两端已有 identifier hint、聚焦准确关系并逐条列出证据元数据。导航不自动请求人物摘要、出网或纠正，不解密姓名或来源正文；页面明确区分证据条目与原始材料。

验收包括相同计数的两人仍精确定位、未知/已遗忘/引用改变目标不落到另一条、快速切目标/卸载/来源变化拒绝旧交付，以及键盘聚焦后手工纠正。route 参数须有固定解析边界与长度限制，不能放任意内容或外部跳转。

复审补充具体时序：普通全图初读被来源变更打断后，显示停止状态并允许明确重读，不永久停在加载中；写入未结束时重读禁用。地址已切换而 React 尚未提交新目标时，旧按钮不得触发纠正或人物摘要；回调在 IPC 前核对本次渲染的地址、当前地址及来源代次。来源代次本身须进入 React 状态，每次变更都能刷新准入条件；旧写入在新页面挂载后结束，不能使该页面后续正常点击永久失效。焦点交付须可取消，并核对同一目标、来源和仍连接的标题节点。

目标仅核对当前仍存在的证据元数据，不认证旧洞察陈述或完整快照仍最新。全部指定引用须属于同一条当前关系；额外新增证据不自动使旧 URL 非法。使用最多 64 个唯一引用、8192 字符的严格本地目标，超过容量时整份摘要不生成精确链接并提示限制，不静默裁剪引用。

### 原生 IPC 测试源码修复（PLAN REVIEWED）

沿用实际 `configure` 与现有 mock runtime，限 `apps/desktop/src-tauri/tests/ipc_roundtrip.rs`。将已停用 preview 入口的正向/坏输入/hostile 测试迁移到不可变准备协议；正向 fixture 遇任意 refusal 直接返回不能算通过。旧四入口另测固定拒绝。补本轮新增但没有实际调用的 memory、insight、导入查询/恢复/丢弃命令，核真实 serde 层级和 camelCase 参数。

当前没有 pkg-config、GTK 3、WebKitGTK 4.1、JavaScriptCoreGTK 4.1、libsoup 3 与 AppIndicator 开发依赖。`tauri/test` 不消除生产依赖的 wry/tray；不改 feature/锁或造替代 shell 来冒称 native。源码修复、可执行核心/前端验证和未执行的原生测试分别记录。完整 G-L 继续暂缓，G-W/G-M 未运行，PR 不合并 main。

### 现代导入准备的注入尝试审计（PLAN REVIEWED）

原生测试迁移的独立复审发现：旧 Session 预览曾记录 `injection.blocked`，现代 owned prepare 虽返回标记计数却遗漏该尝试审计。保留原生 hostile 断言，先补可执行 Session 反例取得 RED，再最小恢复生产路径，不能靠放宽断言通过。

在 owned `execute` 的字节预算、解析和 staged 预算均成功后计算预览；同一 store guard 内，对非零标记写与旧路径同形的聚合无正文审计，之后才做来源身份/索引预检与 reserved 持久化。审计失败拒绝准备，不留下可确认的 held 或 reserved。完成时来源已变、接收方放弃或完成对象被丢弃，不能抹去已经实际读取的尝试；未执行直接丢弃、解析失败、超预算或零标记不新增计数。确认提交的既有领域事务与提交审计保持原语义；查询及同操作回执幂等不得补记准备尝试。

验证双格式预览未确认、合法确认后的原有提交审计、来源迟到拒收、身份拒绝、完成对象丢弃、失败审计注入和零标记/无效/预算拒绝。只用合成 fixture 与临时 SQLCipher 库，不新增接口、schema、依赖或冻结规则。

## 不在本轮扩大解释的能力

洞察中的导入统计目前不能直接选入起草上下文；本轮导航和手写记忆组合验证不把这一点说成已经完成。来源正文的受控解引用、统计材料的本次用途与最终引用需要独立类型/快照契约，不能把统计 bundle 冒充手写 memory、自动创建记忆或继承研究许可。

## 实际结果

三个原定模块与审计补漏均已独立源码复审（REVIEWED）。Linux、pwsh 7.4.6、Rust 1.83.0；只用合成 fixture、临时 SQLCipher 与 loopback MockLlm。最终源码版本由包含本文的提交记录，不标 GATED/INTEGRATED。

- 先运行 `cargo test -p soulcore --test session_import_work modern_prepare_audits_injection_attempts_before_confirmation --locked --offline`：编译成功，运行因审计实际 `[]`、预期 `[5]` 而失败，退出 101。补丁未放宽此断言；最终双格式准备、原有确认审计、来源迟到、身份拒绝、完成对象丢弃及审计失败反例均通过。
- 最终 `cargo test -p soulcore -p soul-import --all-targets --locked --offline`：**51 套件 / 360 通过、0 失败、1 忽略**，退出 0。忽略的是既有 Q2-03 五次独立专项测量，要求干净冻结 SHA 和 Windows pwsh，不是本轮新增忽略。新 Session 组合测试在最终运行中通过；此前五套定向 25 项结果不重复累计。父检查点九包 720 项仍只属于当时版本。
- 完整前端 `vitest run --maxWorkers=2`：**39 文件 / 463 测试**通过，TypeScript、ESLint 与临时 `NODE_ENV=production` 的 Vite build 通过。调用仓库已安装的直接 CLI，未更新依赖锁。第一次定向运行暴露焦点交付失败；修复后关联回归再暴露新页面来源代次不刷新，保留既有断言并以 React 来源状态修复；最终完整测试包含两类回归及三个新增异步反例。
- 根及独立桌面 workspace 的 `cargo fmt --all -- --check` 均通过。原 6 条命令源码断言使用实际 52 项 inventory 独立提取、编译和执行，**6/6**；它们不编译 Tauri、不执行原生 IPC。
- `schema-freeze --check`、`e0-audit` 与 `denylist-audit` 通过；E0 实际扫描 **16 shipped crates / 268 源码文件**。冻结 JSON schema、依赖锁、IPC 注册及 T4D/A0–A3 未改。
- 文档静态核对通过：HTML 历史 script/SVG 字节未改，相对链接存在，STATUS / PLAN_INDEX / ARCHITECTURE / HTML 四入口均关联本轮设计；`git diff --check` 通过。没有把静态链接核对称为新一轮浏览器交互验收。

本机未构建或执行原生桌面测试；`ipc_roundtrip.rs` 中 mock runtime 的重启/持有期等断言也仍待实际原生运行，不能由核心重启用例替代。完整 G-L 继续暂缓，G-W/G-M 未运行，没有安装、发布或合并 main。导入统计到起草的类型/来源契约与真实模型效用验证仍在本轮范围外，Goal 1、正式 Goal 2 与 Wave 1 均未关闭。
