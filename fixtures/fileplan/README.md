# Fileplan fixture 使用约定

本目录不保存实体测试树或 symlink。`sample_tree.json` 是唯一的样例树描述
JSON，内容全部 harmless；测试应读取描述后，在 `tempfile` 中现场建树，并在
用例结束时随临时目录一起清理。

建议测试布局：

1. 在同一个临时父目录下创建授权树 `A` 和未授权的同级树 `B`，只把 `A`
   传入授权 roots。
2. 把 `sample_tree.json` 的 `relative_path` 逐项拼到 `A` 下，创建普通目录和
   文本小文件；扫描、预览前后比较路径、类型、字节数、内容哈希与 mtime。
3. symlink 逃逸必须在运行时创建：先在 `B` 放目标文件，再令
   `A/outside` 指向 `B`。断言 canonicalize 后拒绝或不跟随，扫描结果不得包含
   `B` 的内容。按平台能力加条件分支；不要把 symlink 提交到 Git。
4. 注入文件名直接复用
   [`fixtures/injection/filenames.txt`](../injection/filenames.txt) 的可移植子集，
   在临时树中创建真实文件并断言名称只作为数据处理。含 `/`、Windows 保留名
   或其他平台非法字符的行只做 `UntrustedText` 解析测试，不要声称它们已完成
   真实文件扫描。

粘贴注入语料复用
[`fixtures/injection/paste_injection.txt`](../injection/paste_injection.txt)；
第三人泄漏边界复用
[`fixtures/leakage/third_party_unicode.json`](../leakage/third_party_unicode.json)。
这些已有 corpus 不在本目录重复维护。
