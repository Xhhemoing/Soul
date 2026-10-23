# N2 共享运行时与只读 API：独立复审

日期：2026-10-23（America/Los_Angeles）。独立评审者：`readonly_api_review`，`soul_reviewer` / `gpt-6.1-sol/high`；主线程按其原始报告整理本回执。评审者只写仓库外 `E:/SoulSyntheticLab/local-api-phase1-20261023/readonly-review.log`，没有改实现、运行构建或自签平台门禁。

起点为 `codex/soul-rebuild-20261019` / `c5be3ed3aaa09b6ef5e2a8861a07d7c7c1fa8b46`，包含既有 D109/N1 与本片 dirty 修改。评审先读任务、权威规则、起点副本和实际 diff，再检查源码、测试和原始日志，最后读取实现者说明。范围为新 runtime/types/API、Tauri 接线/合成 host、凭据与生命周期、生成文件及原出网审计的服务端适配。

## 发现、修复与定向复查

| 发现及反例 | 修复与复核结论 |
| --- | --- |
| Rust 命令表新增三项 runtime 读命令，core.ts 缺相应名称与 wrapper；command_surface 实际 4/6 | 补生成 DTO import、三个命令与 typed wrapper。薄层检查精确允许六个 runtime 读转调，并保留敏感同步命令的合成拒绝前缀；未广泛放行任意 runtime 代码。 |
| server 特定 Hyper 例外未检查 API 自身 feature 转发；普通 `hyper/client` 与 weak feature 可以绕过 | 两 workspace 同时检查 package.features；只允许既定 server 特性，不放行整个 API 依赖子树。 |
| 初次修补仍漏 `h = {package="hyper"}` 与 `default=["h/client"]` 的别名 | 三项 server 依赖明确禁止 rename；普通、weak、别名的负例都覆盖。当前无别名需求，较小修复已封住该具体反例。 |
| 首轮 HTTP 断连用例错误地将 Axum 的 Arc 克隆计数当作真实 worker 派发信号 | ReadActivity 在 spawn_blocking 前同步取得 owner guard，guard/许可随真实 worker 存活；连接结算后等 owner 清空，再销毁执行器。原“持有 Session 锁时 shutdown 必须等待”断言保留，7 项 HTTP 测试重跑通过。 |
| 桌面 all-targets 单元测试把包装后的 Runtime 当作 Arc 比较，E0308 | 改为 `Arc::ptr_eq(&state.shared(), &worker.shared())`，保留真实 Session 指针同一性、双向修改可见与同 store 断言；独立定向复查无新阻塞。 |
| 合成启动只设置窗口 data_directory，继承的 WEBVIEW2_USER_DATA_FOLDER 仍可使 SDK 使用旧 profile | 结合独立读取的 Tauri/wry 调用链与主线程保存的 Microsoft SDK 规则确认；合成分支在线程/窗口创建前设置所有窗口配置和同一进程变量。注入 setter 回归已通过，无全局环境/注册表写入；原生 WebView 观察仍未执行。 |
| 根扩大回归的旧迁移测试仍期待 schema 7，既有 D109 起点已是 8 | 仅使用当前 STORE_SCHEMA_VERSION 作为最终 stamp 期待，默认关闭、不新增模板、不重排队及重开库断言保留；独立复核了起点 diff、实际迁移和失败日志，没有修改产品迁移。 |
| 桌面壳源码检查只匹配内联 generate_context，不能识别为 profile 隔离而提前生成的 context | 检查改为生成 context、configure、注册关闭处理器、build(context)、run 的顺序；原关闭/托盘状态与单实例断言完整。独立检查实际 diff 和启动链，无新增缺陷。 |

最终源码定向复审**未发现仍未修复的具体阻塞缺陷**。Windows protected DACL 在写秘密前创建；OpenAPI 与 DTO 一致；Memory 精确 ID、不可用库、拒绝不回显内部信息、同库 lease、派发后断连结算均在本片测试范围内。

## 已核对的证据与限制

评审者读取了 `attempts.txt` 和真实原始日志：N2 三 crate 的 19 项通过；前端 62 文件/744 项通过，lint 与 production build 退出 0；旧 xtask 一次 35/36 后修复 SBOM 归属为 36/36。后者早于最终 rename 反例，不能单凭那次日志宣布最终审计候选已通过。

首轮命令/HTTP 失败、桌面 E0308 与根扩大检查磁盘满均保留。根扩大检查原配置因 E 盘 `os error 112` 且 Out-File 写日志失败中断，外层退出 1，native 退出码未知。资源恢复脚本改用 D 盘临时 target、仅进程设置 incremental=0、DEV/TEST debug=0、jobs=2，并在 finally 恢复；没有删除或搬动原 cache。原定两个临时 target 后调整为顺序共用已构建的临时依赖缓存，两个 workspace 仍分别选各自 manifest/lockfile 检查。后续结果属于该资源配置，不冒充原 profile 完整验收。

主线程已完成最终根/桌面过滤回归、clippy、fmt 与审计；命令及资源配置见[实施回执](../plans/2026-10-23-repository-normalization.md#n2-共享运行时与只读-api实施回执)。独立评审者读取实际日志和退出码并重新核算结果，未独立重跑构建；本报告不把主线程执行改写成评审者执行。

历史 `desktop-synthetic-3.txt` 已通过 IPC 77 项（2 项真实采集排除）、host 4 项、出网审计 5 项，但随后在旧壳源码检查失败，不能将该条命令计为通过。`root-workspace-synthetic-3.txt` 在上述 schema 期待失败；这些失败与两次根检查中断均保留，最终成功来自后续第四轮原命令。

最终独立核对结果：

- `root-workspace-synthetic-4.txt` 的 204 条 test result 合计 1640 通过、0 失败、29 ignored、3 filtered；`desktop-synthetic-4.txt` 的 8 条合计 147 通过、0 失败、2 filtered。与主线程汇总 JSON 一致，未跨批次累加。
- 实际日志确认 Windows ACL、断连结算、14 个合成能力拒绝、profile 注入、IPC→HTTP→同库重开、schema 6 升级和两 workspace 服务端依赖负例通过；desktop shell 22 项通过。这些是 Windows 上的 Rust/真实回环 HTTP/生产 IPC handler 测试，Tauri 使用 mock runtime，不是原生交互验收。
- `root-clippy-4.txt`、`desktop-clippy.txt`、`root-format-final-2.txt`、`desktop-format-final-3.txt`、`schema-freeze.txt`、`e0-audit.txt`、`denylist-audit.txt` 的退出 0 均已对照 `attempts.txt` 与原输出。
- 后续 clippy 的等价布尔简化、对齐缓冲区 `div_ceil`、HTTP 测试多余借用，以及 IPC 模块排序均经定向复核；拒绝条件、对齐、秘密写入顺序及安全断言保持。原失败和中断日志仍非空保留，不以最后成功覆盖。

最终已审源码与所列运行证据范围内，无未解决的具体阻塞缺陷。主线程按评审建议在原证据目录保存 `final-tracked.patch`、`final-scoped-tracked.patch` 与 `final-source/` 的 47 份源码/manifest/生成物/文档副本，清单为 `final-source-paths.txt`；重叠路径的旧修改仍由起点记录区分，不新增 hash、冻结 contract 或 gate。本机没有 cargo-deny，当前公告库检查未执行，禁词检查不能替代它。

非阻塞范围限制：people 分页只限制响应，核心仍完整读取图；核心锁等待可能延长退出；当前用户 ACL 不隔离同账户恶意进程。HTTP 写入、主体隔离和 UI 事件在 N3，已有 UI 刷新缺口没有被重分类为本片新增缺陷。没有原生 WebView/profile、托盘、剪贴板、安装或真实采集观察；不签发 G-L/G-W/G-M、Goal 关闭、合并或发布批准。
