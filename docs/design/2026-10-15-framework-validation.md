# 框架参考算法：验证与独立评审

日期：2026-10-15。依据源码 `6e8f3c06b7fd0cbc7920766337572301319b021a`。本次修改是设计文档、离线 HTML、图源和非生产 JS 参考模型；没有修改生产 Rust/React/IPC/schema/依赖。

## 实际执行

环境：Linux；Node `v24.19.0`；源码来自固定提交归档，工作副本没有 `.git`。Bash 诊断不代替仓库要求的 PowerShell 平台门禁。没有运行真实外部模型、个人数据或数据库。

| 检查 | 实际结果 | 能证明什么 |
|---|---|---|
| `node --test docs/design/framework-algorithms.test.mjs` | 最终 37/37，退出 0 | 参考模型的归属、失效、容量、选择与纠正优先 |
| 事件顺序枚举 | 120 种顺序全部满足断言，包含在上述 37 项中的一项 | 单线程线性模型的撤权/取消/来源变更与派发/接收关系；不是 120 个额外测试 |
| 六个定向变异 | 6/6 被原测试检出 | 断言能捕获关键保护被删除；不是生产 mutation coverage |
| 独立只读评审 | 四类问题有反例并已修正；定向 Node 探针退出 0，复审无剩余算法阻塞项 | 候选参考代码和语义边界获得独立检查 |
| HTML 浏览器检查 | 13/13，退出 0 | 四视图、六种任务交错、键盘重置、选材交互、四项下载字节一致、目录、390px 无横向溢出、零脚本错误/自动 HTTP 请求；只验证交付页 |

首稿为 31/31；独立评审发现未覆盖反例后新增 6 项，最终为 37。这里不沿用此前生产前端 338 项、Rust 65 项或旧 HTML 12 项来增加本轮测试数。

HTML 使用本地 Chromium 133 / Playwright，在真实 file:// 打开；1440px 与 390px 布局已检查。测试浏览器注入本地 CJK 字体仅用于截图，交付 HTML 使用系统字体，不含自动字体请求。截图检查发现 SVG 自设字体未采用注入字体，已补明确的 CJK 字体回退并复查；不是 Soul 产品字体变更。

## 评审怎样实际改变了设计

| 反例 | 初稿问题 | 修订与回归 |
|---|---|---|
| capacity=1，准备 A → 撤权 → 重新配置 → 准备 B | 从未派发的失效 A 永久占槽，B 一直 busy | 回收失效/过期的 prepared；仍在运行的 dispatched 等待实际结束。补撤权、过期、来源改变三项 |
| `purposes: 'redraft'`，或字符串 citations | 字符串 includes 误判用途；字符串引用展开成字符 | 严格数组与非空字符串成员校验；补 string/null/mixed 反例 |
| 只命中一条无关材料，库中另有两个地址冲突 | 未召回的记录仍迫使本题澄清 | 冲突限定合格召回并集，仍位于 top-K 截断前；无命中为 abstain |
| origin 是 undefined、空串或 false | 非 null 就被当作已配置 | 模型拒绝非规范 HTTP(S) origin 表示；生产仍必须复用真实 Origin/NetGuard |

独立复审进一步要求的文字修订也已纳入：Accepted 与 UI 实际显示之间还有 IPC 间隙；未来真实提交不得在成功校验后脱离保护；完整请求需要计入固定开销并最终重新计量；复杂度包括队列回收、排除项排序和召回集合。

## 定向变异的复现规则

对 `framework-algorithms.mjs` 分别创建独立临时副本，保持测试文件不变，每次仅应用一行替换，然后运行同一 `node --test`。六个副本退出码均非零，原始参考模型退出 0。临时副本没有写回仓库。

| 故意移除的保护 | 独立替换 | 结果 |
|---|---|---|
| 编辑代次 | `&& ticket.revision === this.#revision` → `&& true` | 检出 |
| 取消标记 | `if (op.cancelled)` → `if (false)` | 检出 |
| 来源版本 | `snapshot.dataVersion === this.#dataVersion` → `true` | 检出 |
| 本次使用许可 | `row.granted !== true \|\|` → `false \|\|` | 检出 |
| 遗忘状态 | `row.active !== true \|\|` → `false \|\|` | 检出 |
| 过期准备回收 | `this.#sweepPrepared(now);` → 注释 | 检出 |

变异检查用于证明确有敏感断言，不作为新的项目 gate 或覆盖率目标。

## 覆盖范围与剩余风险

参考模型假设调用来自可信适配器、方法原子执行、只有一个数据 writer。没有实现锁、线程、实际取消、网络传输、持久化回执、中文分词、向量召回、LLM 语义证据验证、自然语言冲突提取或跨进程同步。

参考选择器接收已经召回的排名和规范化 claim。它不能发现没召回的反证；也不能证明每个引用真的支持正文。模型容量有界，但用于有限实验的 used-ID 集合不是生产回收策略。页面 abandon 不模拟核心 discard IPC，需要接线时串行 drain 或获批的按 ID 丢弃语义。

生产接线仍需测试：React 真实交互、Session/worker 交错、拒绝时零 mock 出站、许可发放后的撤权余量、审计中途变更、IPC 结果到达前失效、SQLCipher 事务/重启/遗忘，以及现有平台验收。未运行全 Rust workspace、独立 Tauri workspace、G-W/G-M；完整 G-L 仍按既有指示暂缓。本轮不签发 GATED/INTEGRATED，不合并 main，不宣布 Goal 1/Goal 2/Wave 1 关闭。

## 构建离线页

`node docs/design/render-framework.mjs /absolute/path/to/marked/lib/marked.esm.js`

renderer 使用调用者提供的 Markdown 渲染依赖，不改变应用依赖或锁文件。HTML 内嵌 SVG、完整研究和参考算法源码；运行时不依赖 Markdown 库、CDN、字体服务器或本地旁路文件。下载 SVG/Mermaid/研究原文/算法源码来自已内嵌的相同字节。
