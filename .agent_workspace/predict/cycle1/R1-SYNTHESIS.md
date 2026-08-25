# Cycle 1 Round 1 结论简报（未等 fable；fable 报告后补折入）

Parent: cursor-grok-4.6-high. 四槽已收：opus-a/b、gpt-sol-a/b。fable-a/b 后台独立。

## 数据面事实（opus-a + gpt-sol-a 交叉确认）

- 前台：**明文**只有 `ts` + 枚举；`app` 与 `duration_ms` **密封**。SQL/研究预览滚桶**打不开 body**，所有 exe 合成一个 `app.foreground` 计数。
- 无本机时区：UTC 小时可从 `ts` 推，**不可把 UTC 小时叫「用户的上午」**。
- 无 `EvidenceKind::AppUsage` 构造点 → 预测落 `SoulInference` 前须加证据种类（日后，非本调研实现）。
- Markov 夹具：`FakeForegroundSource` 序列 → 解封 body → `Counter(zip(seq, seq[1:]))`；同 app 轮询会合并，不自然产生 A→A。

## 经确认可测（并列，不选赢家）

| ID | 算法 | 确认理由 |
|---|---|---|
| P0 | 边际频率 MFU | 一切命中率的对照；文献与 opus 都要求 |
| P1 | 一阶 Markov | 本机转移计数；Parate APPM / 手机 Markov 文献基线 |
| P2 | 二阶 n-gram + backoff | 区分同终点不同路径；稀疏要 backoff |
| P3 | 时段桶最频（UTC hour，诚实标注） | 节律；P1 在 boundary 无上下文时用 |
| P4 | 整数环缓冲 Hawkes（禁不可逆指数累加） | 可测「何时」；须打过 P0/P1 才配做默认 |
| P5 | T4D `last_direct_contact` 沉寂提醒 | 读已有列，不改 T4D；不是人格诊断 |
| P6 | A0 锁门 | 锁轴只挡轴向陈述，不挡 next-app |
| P7 | 确定性仲裁 | 弃权/并列字典序，评测可归因 |
| P8 | 停留时长风险桶 | 打扰门 |
| APPM-lite | 可变阶 PPM | gpt-sol-b：位置特征在文献里增益小，适合窄数据面 |

## 否决（v0.2 默认）

- 联邦 SeqMF / 云协同过滤：要多用户与出网，v0.4 前冲突 E0。
- 完整 ATPP：要位置/POI/多用户预训练。
- 深网/GNN/读窗口标题/读文件正文出网。
- 对锁定轴做人格预测。

## Cycle 1 Round 2 攻坚

收紧：P4 是否降为「对照而非默认」；APPM-lite vs P2 是否合并；预测解释句如何保持可数且非临床；schema 缺口只记录不实现。
