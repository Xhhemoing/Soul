# schema/

WP-B07 的家。`beadproj.ts` 是 `.beadproj` v1 的校验器兼形状定义（TypeScript 即 schema），
`beadproj.md` 是同一格式的散文说明，`beadproj.test.ts` 是它的验收矩阵。

这里没有 JSON Schema 文件，也没有校验库依赖：重建式的手写校验器已经 fail closed，
而 schema 文件正是 `$schema` / `$id` 之类外网 URL 的高发地（BD15 / BD17）。

`.pat` / `.gamedev` 在 v0 没有解析路径——识别魔数后一律 `UNSUPPORTED_FORMAT`，
逆向解析等真实样本。嗅探本身在 `pages/create/import.ts`。
