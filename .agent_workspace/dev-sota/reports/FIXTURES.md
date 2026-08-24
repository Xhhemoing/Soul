# ST-01 / ST-02 fixture 准备说明

本次只新增 fixture 与说明文件，没有修改 Rust/TypeScript 源码。

## ST-01 起草

- `fixtures/draft/third_party_paste.json`：提供 AC-12 所需的一条第三人粘贴
  消息。正文“明天上午十点在公司门口见”有 12 个中文 Unicode scalar，并带
  姓名“李雷”和账号“@wang_xiao2”。三项取值复用
  `fixtures/leakage/third_party_unicode.json`，便于直接构造
  `LeakageChecker`。
- `fixtures/draft/injection_paste.jsonl`：提供两条第三人粘贴注入行，分别覆盖
  外传 URL 指令和伪造 system role。每行的 `source_ref` 指回
  `fixtures/injection/paste_injection.txt`，避免另起一套注入语料定义。

## ST-02 文件计划

- `fixtures/fileplan/sample_tree.json`：唯一的 harmless 样例树描述，只含小型
  文本内容，不在仓库中落实体测试树。
- `fixtures/fileplan/README.md`：要求测试在 `tempfile` 中创建授权树 `A` 与
  未授权树 `B`；symlink 逃逸也必须现场创建，仓库不提交 symlink。注入文件名
  直接选用 `fixtures/injection/filenames.txt` 的平台可创建子集；含路径分隔符
  或保留名的行只用于纯文本解析断言。

后续测试应验证扫描/预览前后临时树快照不变，并明确区分“真实创建并扫描”的
可移植文件名与“仅作为不可信文本解析”的平台非法样本。
