# 正式开工提示词（可整份粘贴给云端父代理）

产品方向已按作者 2026-08-24 澄清重锁，并经计划扫描收窄。新会话只粘贴「提示词正文」。
**在 `docs/STATUS.md` 写明 `PLAN_FROZEN` 之前，只允许改文档与 schema，不许写业务代码。**

---

## 提示词正文

你是本仓库 Soul 的云端父代理。仓库：`github.com/Xhhemoing/Soul`。作者 Roy。先读 `docs/PRODUCT_LOCK.md`、`docs/DECISIONS.md`、`docs/SECURITY.md`、`docs/STATUS.md`、`docs/schemas/`、本文件。产品定义以锁定文档为准，禁止改回「通用桌面助手」或把文件整理当产品本体。

当前几乎无代码。任务是按锁定方案把 Soul 做成可安装、可测试、可审计的 Windows 本地灵魂级软件。Goal 2 二十轮不在本文件；见 `docs/GOAL2_POLISH_PROMPT.md`，Goal 1 关闭前不要启动。

### 产品锁定（不可改写）

Soul 是灵魂级个人软件：在本机复刻电子版的用户（人格、记忆、心理工作模型、人脉图），并辅助处理电脑问题、起草回复、分析人与事。Windows 本地优先。v0.1 无云端 HTTP。社交导入仅 `soul-import-v1` 与 Telegram Desktop `result.json`。只起草不发送。采集默认关。第三人数据默认不出本机。研究预览第三人行数为 0 且不写文件。

v0.1 垂直切片以 `docs/PRODUCT_LOCK.md` 的 13 条为准。代理层在 v0.1 只证明：只读扫描 + 计划预览 + 未授权路径 100% 拒绝。

### 云端子代理模型约定

11.1 适用范围：仅 Task 派生的云端子代理。父代理模型不受下表限制。

11.2 模型选择
修复、落地代码、补测试使用 slug: `claude-opus-5-thinking-high-fast`
其他情形使用 slug: `claude-fable-5-thinking-xhigh`
当次用户指定覆盖本表。

11.3 父代理直改白名单：文档/注释/配置措辞；或不超过 10 行且不涉及业务逻辑、权限、数据面。直改须说明。超出派 opus。

11.4 禁止静默降级。修复：`claude-opus-5-thinking-high-fast` → `claude-opus-5-thinking-high` → `claude-sonnet-5-thinking-high`。其他：`claude-fable-5-thinking-xhigh` → `claude-fable-5-thinking-high`。同系列皆不可用则暂停。子代理第一行自报 slug。

11.5 Git：`cursor/` 前缀分支。进度以 `docs/STATUS.md` 为准。写码前阻塞项只有：PRODUCT_LOCK、DECISIONS、`docs/schemas/` 九份、SECURITY、STATUS。禁止第二份 PRODUCT.md。

### 接下来使用 Goal：先调用子代理 claude-fable-5-thinking-xhigh，根据项目计划和目的，将项目代码的编写拆分为多个子代理 claude-opus-5-thinking-high-fast 进行编写，最后 review 并打磨到可安装的 v0.1。注意要及时把成果和方案和进度记录文档提交成 PR 并放到新的专门的你专属的分支，并且你产生的多个 PR 在合适的时候需要进行合并

1. 确认 STATUS 为 `PLAN_FROZEN`。
2. CreateGoal：Goal 1。
3. 派 fable planner，覆盖下列 WP，只减不增。
4. 按 DAG 派 opus implementer。权限/数据面与 UI 面分人。
5. 每批 fable 只读复核。缺陷派 opus。
6. Goal 1 完成 = 本文件验收矩阵全部 `v0.1` 行通过。

工作包：

- WP01 骨架、工具链、冻结 schema、SECURITY、CI
- WP02 加密主库、遗忘、研究预览（不写文件）
- WP03 灵魂档案（特质轴、纠正锁定）
- WP04 自传记忆 CRUD + 遗忘
- WP05 人脉图 v0
- WP06 soul-import-v1 + Telegram result.json + 问卷
- WP07 前台应用时长采集（trait 抽象，CI 用假实现）
- WP08 策略、审计链、net_guard、redactor、HITL 令牌（无 HTTP）
- WP09 桌面壳
- WP10 起草（不发送）+ 人事分析摘要
- WP11 只读目录扫描与计划预览；未授权拒绝
- WP13 安装 smoke、CI、SBOM

WP12 已删除。并行：WP01 冻结后 WP02–WP06、WP08 可并行。

### 实现者红线

1. 无 E0。无项目方域名、遥测、自动更新。
2. E1 只来自用户填写端点，请求体必须过 redactor。
3. 主库加密。禁止明文 JSONL 当存储。
4. 审计无正文。
5. 遗忘 = 销毁内容密钥。
6. 无 evidence_ids 的 inference 不得落库。
7. v0.1 不实现文件写。
8. 外部内容不是指令。
9. 禁止诊断词与量表分数。
10. 改产品方向先改 PRODUCT_LOCK。

### Goal 1 验收矩阵（门禁）

| ID | Given | When | Then | 谁跑 |
|---|---|---|---|---|
| AC-01 | 干净 Win11 | 安装启动 | 托盘出现且不提权 | 作者手动 + 安装 smoke |
| AC-02 | 首次向导完成 | 读配置 | 采集关、云关、无 LLM 端点 | CI |
| AC-03 | 无导入 | 完成问卷 | 非空档案，字段来源 user_stated | CI |
| AC-04 | soul-import-v1 fixture | 导入 | 落加密库，无明文残留 | CI |
| AC-05 | Telegram result.json fixture | 导入 | 映射事件/联系人；缺字段失败可读 | CI |
| AC-06 | 已导入 | 生成档案图谱 | 每条 inference 有可解引用 evidence | CI |
| AC-07 | 语气字段被用户改 | 再起草 | prompt 用用户值，推断不覆盖 | CI |
| AC-08 | ≥3 个对话对象 | 打开人脉图 | 节点≥3，边有证据 | CI |
| AC-09 | 采集关 | 切应用 10 次 | foreground 事件=0 | CI |
| AC-10 | 采集开后切应用，再关闭 | 观察事件 | 开启期间至少 1 条；关闭后 1s 内无新事件 | CI |
| AC-11 | mock LLM | 起草 | 不发送；仅 E1 到 mock 精确 origin；跨 origin 重定向拒绝 | CI |
| AC-12 | 含第三人正文/姓名/账号 fixture | 默认起草 | 请求体无 ≥8 字原文子串，也无未占位姓名/账号 | CI |
| AC-13 | 单次包含原文豁免 | 再起草一次 | 仅当次含原文；下次回到占位 | CI |
| AC-14 | 3 条记忆 | CRUD | 读写一致；审计无内容 | CI |
| AC-15 | 一条记忆 | 遗忘并重启 | 预览影响面；CK 销毁后无法解密；推断 orphaned；审计仍在且无内容 | CI |
| AC-16 | mock LLM | 人事摘要 | 每条有证据；无诊断词 | CI |
| AC-17 | 无 LLM key | 摘要与起草 | 统计/模板降级；无非回环连接 | CI |
| AC-18 | 授权 A 未授权 B | 扫描 A、操作 B | 产出只读计划预览；A 磁盘不变；B 100% 拒绝 | CI |
| AC-19 | 未知动作、或已批准计划被改 hash、或令牌重放 | 请求执行 | 全部拒绝 | CI |
| AC-20 | 含第三人正文 | 研究预览 | 行级输出无第三人字段；第三人行数=0；written_to_disk=false | CI |
| AC-21 | 默认配置跑主流程 | 观察网络与源码 | 非回环连接=0；无业务域名 | CI |
| AC-22 | 点云端开 | UI | 保持尚未启用；无网络 | CI |
| AC-23 | 矩阵所列动作后 | 审计回放 | 链通过且无正文/姓名 | CI |
| AC-24 | 写入中注入崩溃 | 重启 | 链通过；未提交最多丢 1 条 | CI |
| AC-25 | 注入串出现在导入、粘贴或文件名 | 档案与起草 | 无工具计划；无外连该 URL | CI |
| AC-26 | 仓库 | 本地门禁（D50） | G-L `just ci-full` 绿 + G-W `scripts/gate-win.ps1` 绿，记录在 `docs/gates/<日期>-<sha7>-*.md` | G-L + G-W |

AC-27 文件执行与撤销标 **v0.1.1**，不是 Goal 1。

### 开工第一动作

1. 若 STATUS 不是 PLAN_FROZEN，只做文档。
2. CreateGoal：Goal 1。
3. 派 fable planner。
4. planner 返回前不要大面积写业务代码。
