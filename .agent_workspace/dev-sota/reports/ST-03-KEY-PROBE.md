MODEL: gpt-5.6-sol-xhigh-fast

# ST-03 密钥探针只读审查

审查对象：`crates/soulcore/src/commands/store.rs::open_store_for_session`，并沿调用链核对 `crates/soul-store/src/keys.rs` 与 `crates/soul-store/src/store.rs`。审查版本：`2920d2f7776c46a846c69c9d972d57fe8cf11acf`。未修改代码，未运行会写数据的测试。

## 结论

1. **DPAPI 探针阶段的任意错误都会回退。** `open_store_for_session` 对 `platform.key_encryption_key()` 使用 `Err(_)`（`store.rs:122-130`），没有只匹配 `KeyError::Unsupported`。因此该次调用若返回 `Unavailable`、`Unsupported`、`Io` 或 `Malformed`，都会进入 `TestKeyProvider` 回退，而不是报错终止。
2. **但不能概括为“全流程任意 DPAPI 错误都回退”。** 探针成功后，`open_store(directory, &platform)?` 会让 `SqlCipherStore::open` 再调用 `database_key()` 和 `key_encryption_key()`（`crates/soul-store/src/store.rs:108-109`）；这些后续调用的错误由 `?` 直接向上传播，不再进入回退分支。当前 DPAPI 骨架的两个访问器始终返回 `Unsupported`，所以现版本实际总是在第一次探针处回退。
3. **会在传入的数据目录写明文 seed，但写入是惰性的。** `TestKeyProvider::in_dir(directory)` 本身只记录 `<directory>/soul-test-keys.bin` 路径（`keys.rs:145-149`），并不立即写盘。首次取钥时，`read_or_create_key_file` 生成 64 字节 OS 随机 seed，并用 `std::fs::write(path, &seed)` 原样写入该文件（`keys.rs:177-194`）；没有 DPAPI、加密或包裹。回退路径打开库时必然调用取钥，因此正常执行会触发该写入。

## 安全影响

- `Err(_)` 把“平台能力尚未实现”和“DPAPI blob 损坏、不可读、权限异常”等未来可能出现的故障视为同一种情况。对新目录，这会静默降级为明文 seed；对已有 DPAPI 加密数据库，回退钥匙通常打不开数据库，但明文 `soul-test-keys.bin` 已可能在数据库校验前被创建。
- 64 字节 seed 是 DEK 与 KEK 的共同根材料；代码仅用不同 domain 经 SHA-256 派生两把 32 字节钥匙（`keys.rs:81-91, 160-167`）。能读取 `soul-test-keys.bin` 的主体可重建两把钥匙。
- 写文件时没有显式设置仅限当前用户的权限，也没有原子、排他创建；实际权限依赖进程环境和 umask。
- 探针取得的 KEK 被丢弃，平台路径打开库时又重新读取 DEK/KEK。除了重复操作，这也意味着“探针成功、随后取钥失败”只会报错，不会回退。

## 判定

| 问题 | 判定 |
|---|---|
| DPAPI 任意错误是否都回退 | **探针调用：是。全流程：否。** |
| `TestKeyProvider::in_dir` 是否在数据目录写明文 seed | **最终会。** 构造时不写；首次取钥时将 64 字节原始 seed 写到 `soul-test-keys.bin`。 |

建议后续实现 DPAPI 时仅对明确的“平台不支持/功能未实现”条件允许显式回退；损坏、I/O、权限和解密失败应 fail closed，并在任何回退落盘前取得用户可见的明确授权。
