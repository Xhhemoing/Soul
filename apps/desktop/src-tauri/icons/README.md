# 图标

`icon.png` 是 1024×1024 的母版，其余尺寸由它生成：

```
pnpm exec tauri icon src-tauri/icons/icon.png
```

生成器会顺带产出 macOS 与移动端的图标。v0.1 只做 Windows，Linux 只当 CI 宿主，
所以 `icon.icns`、`Square*Logo.png`、`android/`、`ios/` 都删掉了——留着会让人以为
这些平台是支持的。`tauri.conf.json` 里引用的四个文件就是这里的全部。

图标是画出来的，不是下载的：没有第三方素材，也就没有需要跟着走的授权。
