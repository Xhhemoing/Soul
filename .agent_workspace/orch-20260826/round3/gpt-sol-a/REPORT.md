MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Round 3 / gpt-sol-a：仓库卫生与可复现探针

审计时间：2026-08-26 UTC  
Git 基线：本地 `main` 与 `origin/main` 均为 `a0ec14b723c4ea7a6203b2faf6c4d40e8ad3a181`。当前分支 HEAD 是 `78febb0837c2704839448002642b31d15f01bc95`，因此 `main` 统计全部直接读取 Git tree，没有把当前分支新增报告误算进 `main`。  
约束：未执行 `git checkout`、`git commit` 或 `git push`；除本报告外未修改工作树。

## 结论

1. `main` 跟踪了一整棵 Rust 构建目录：`.agent_workspace/round3/fable-a/t4d-verify/target/`，共 **191 个文件、38,060,396 B（36.297 MiB）**。
2. `main` 的 **12 个零字节文件全部位于该 `target/`**；除此之外没有零字节 tracked file。191 个构建文件中 179 个非空、12 个为空。
3. `main` 的 `.agent_workspace/` 有 **508 个 tracked file、40,360,631 B（38.491 MiB）**，占 `main` 全部 tracked blob 字节的 **98.70%**；上述 `target/` 又占 `.agent_workspace/` 的 **94.30%**。
4. 当前分支在写本报告前的 `.agent_workspace/` 有 **522 个文件、40,619,111 B（38.737 MiB）**；`du -sk` 为 **41,468 KiB**，`du -sh` 显示 **41M**。与 `main` 的差额 258,480 B 来自当前分支已跟踪的 `orch-20260826/round1`、`round2` 报告与探针。
5. `main:docs/` 的 20 个 Markdown 文件共有 **11 个 inline Markdown link**：5 个本地目标存在，**6 个死链**。6 个死链全部在 `docs/scan-rounds/`，都是裸 `bc-...` Cursor run ID 被 Markdown 解释为相对路径；该目录中的 6 个 run 链接坏链率为 **100%**。

## 1. `main` 跟踪的 `target/` 与空文件

### 统计

| 指标 | 数值 |
|---|---:|
| `main` tracked file 总数 | 585 |
| `main` tracked blob 总字节 | 40,890,396 B（38.996 MiB） |
| 路径分量为 `target` 的 tracked file | 191 |
| `target/` tracked blob 字节 | 38,060,396 B（36.297 MiB） |
| `target/` 非空文件 | 179 |
| `target/` 零字节文件 | 12 |
| `target/` 之外的零字节 tracked file | 0 |

191 个文件全部来自一个目录：

```text
.agent_workspace/round3/fable-a/t4d-verify/target/
```

它们由提交 `7b35bdeed54feb5186358579d751ff8c515a666b`（`Verify and freeze v0.1 soul-layer algorithms (#5)`）带入历史。根 `.gitignore` 只有 `/target`，只匹配仓库根的 `target`，不匹配这个嵌套目录；`git check-ignore` 对其中的二进制也无输出。

### 12 个零字节文件

```text
.agent_workspace/round3/fable-a/t4d-verify/target/debug/.cargo-lock
.agent_workspace/round3/fable-a/t4d-verify/target/debug/deps/libt4d_verify-253a23639f535e7f.rmeta
.agent_workspace/round3/fable-a/t4d-verify/target/debug/deps/libt4d_verify-6f83b1a6cbc48d5a.rmeta
.agent_workspace/round3/fable-a/t4d-verify/target/debug/deps/libt4d_verify-e138af828fa0f5d7.rmeta
.agent_workspace/round3/fable-a/t4d-verify/target/debug/incremental/t4d_verify-1c18uljdp3io6/s-hln92swgeh-0nz13s3.lock
.agent_workspace/round3/fable-a/t4d-verify/target/debug/incremental/t4d_verify-1h8k24gxhpnj8/s-hln922yzuw-16n96bi.lock
.agent_workspace/round3/fable-a/t4d-verify/target/debug/incremental/t4d_verify-1mprdj00a9maf/s-hln92w7jhb-0uin8h0.lock
.agent_workspace/round3/fable-a/t4d-verify/target/debug/incremental/t4d_verify-1tjd1gheeysr8/s-hln92w91dn-1ly3s9m.lock
.agent_workspace/round3/fable-a/t4d-verify/target/debug/incremental/t4d_verify-2409jsc31wzgp/s-hln92w7ja2-1hfks6c.lock
.agent_workspace/round3/fable-a/t4d-verify/target/debug/incremental/t4d_verify-2vsnrjcz4cc7m/s-hln92sy2kj-0tmw2s7.lock
.agent_workspace/round3/fable-a/t4d-verify/target/debug/incremental/t4d_verify-33i1o66l5ppbp/s-hln92swg9u-0406vme.lock
.agent_workspace/round3/fable-a/t4d-verify/target/debug/incremental/t4d_verify-3pijt6vlw9kxm/s-hln92sy2vj-1loxukk.lock
```

这些是 Cargo/rustc 构建产物，不是有意保留的占位文件。`target/` 还包含平台相关可执行文件、`.o`、`.rlib`、缓存与 incremental 状态；跟踪它们不能证明可复现构建，反而会把宿主机与工具链状态固化进源码树。

## 2. `.agent_workspace/` 体积

| 基线 | 文件数 | 逻辑字节 | MiB |
|---|---:|---:|---:|
| `main:.agent_workspace/` | 508 | 40,360,631 | 38.491 |
| 当前 HEAD / 写报告前工作树 | 522 | 40,619,111 | 38.737 |
| 当前工作树中的 `target/` | 191 | 38,060,396 | 36.297 |

当前工作树的额外观察：

- 目录数 148，符号链接 0，零字节文件 12。
- `round3/` 为 38,640,659 B（36.851 MiB），占整个 `.agent_workspace/` **95.13%**。
- 单一 `t4d-verify/target/` 占整个 `.agent_workspace/` **93.70%**。
- 磁盘分配量为 41,468 KiB；逻辑大小与磁盘分配量不同，因此报告同时保留两种口径。

## 3. `docs/` 死链

扫描只把 Markdown `[...]()` / `![...]()` 目标当作链接；外部 scheme 不做网络可达性推断，本地目标则相对源文件目录解析，并与 `main` tree 对照。本树没有 reference-style link definition。

| 文件:行 | 当前目标 | Markdown 实际解析出的不存在路径 | 应使用的 Cursor run URL |
|---|---|---|---|
| `docs/scan-rounds/R1-SYNTHESIS.md:4` | `bc-ae1f3731-3cff-57cf-aab3-8e77527156a5` | `docs/scan-rounds/bc-ae1f3731-3cff-57cf-aab3-8e77527156a5` | `https://cursor.com/agents/bc-ae1f3731-3cff-57cf-aab3-8e77527156a5` |
| `docs/scan-rounds/R1-SYNTHESIS.md:5` | `bc-4055f588-dada-5d05-8326-87bf6bf4a1ef` | `docs/scan-rounds/bc-4055f588-dada-5d05-8326-87bf6bf4a1ef` | `https://cursor.com/agents/bc-4055f588-dada-5d05-8326-87bf6bf4a1ef` |
| `docs/scan-rounds/R2-SYNTHESIS.md:4` | `bc-32d82525-2c83-5745-a723-d0af183e7c8a` | `docs/scan-rounds/bc-32d82525-2c83-5745-a723-d0af183e7c8a` | `https://cursor.com/agents/bc-32d82525-2c83-5745-a723-d0af183e7c8a` |
| `docs/scan-rounds/R2-SYNTHESIS.md:5` | `bc-513459d3-2f20-51ca-af15-5acddd931480` | `docs/scan-rounds/bc-513459d3-2f20-51ca-af15-5acddd931480` | `https://cursor.com/agents/bc-513459d3-2f20-51ca-af15-5acddd931480` |
| `docs/scan-rounds/R3-SYNTHESIS.md:4` | `bc-bea41d97-3b1c-5de9-b306-721b3edf466e` | `docs/scan-rounds/bc-bea41d97-3b1c-5de9-b306-721b3edf466e` | `https://cursor.com/agents/bc-bea41d97-3b1c-5de9-b306-721b3edf466e` |
| `docs/scan-rounds/R3-SYNTHESIS.md:5` | `bc-75c9efdf-2a49-5dbb-8708-75daae90d989` | `docs/scan-rounds/bc-75c9efdf-2a49-5dbb-8708-75daae90d989` | `https://cursor.com/agents/bc-75c9efdf-2a49-5dbb-8708-75daae90d989` |

这里能确定的是当前链接语法错误及正确 URL 形状；未把 Cursor 页面是否仍保留、当前读者是否有权限混同为本地死链判定。其余 5 个 inline link 都在 `docs/PLAN_INDEX.md`，目标文件存在。

## 4. 可复现方法

核心 Git tree 探针：

```text
git rev-parse main origin/main
git ls-tree -r -l main
git ls-tree -r -l main -- .agent_workspace
git ls-tree -r -l main -- .agent_workspace/round3/fable-a/t4d-verify/target
git check-ignore -v .agent_workspace/round3/fable-a/t4d-verify/target/debug/t4d-verify
```

工作树体积探针：

```text
du -sb .agent_workspace
du -sk .agent_workspace
du -sh .agent_workspace
```

链接探针读取 `git ls-tree -r --name-only main` 得到目标集合，再读取每个 `main:docs/*.md`，提取 inline Markdown link；无 scheme 的目标用 POSIX 路径规则相对源文档目录归一化后检查是否存在。扫描结果为 20 个 Markdown 文件、11 个链接、5 个有效本地链接、6 个不存在的相对目标。

## 建议（本轮不实施）

1. 从 index 移除整个嵌套 `target/`，并把忽略规则改成可覆盖嵌套 Cargo 项目的 `**/target/`；源文件、`Cargo.toml`、`Cargo.lock` 和文本探针结果可保留。
2. 为 `.agent_workspace/` 定义入库边界：保留必要的 Markdown/文本证据，排除编译器缓存、二进制与 incremental 状态。它目前占 `main` tracked blob 字节的 98.70%。
3. 将 6 个裸 `bc-...` 目标补全为 `https://cursor.com/agents/<run-id>`，并在文档 CI 增加本地 Markdown link checker。
