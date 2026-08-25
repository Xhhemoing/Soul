import { useId, useState, type ChangeEvent } from "react";
import { Link } from "react-router";

import { checkImportFileSize, type ImportErrorCode } from "../../schema/beadproj.ts";
import { hasBeadprojExtension, sniffFile } from "./import.ts";

/**
 * WP-B07's `/create?entry=import-pattern` panel: the bead-pattern *recognition*
 * face (D-IE-2).
 *
 * v0 has no successful parse on this panel, and the copy says so rather than
 * promising one (R-IE-3). `.pat` and `.gamedev` have no public specification
 * and no real sample in the repo, so they are recognised and refused
 * (D-IE-18) — an invented parser would hand back a grid nobody could check.
 * An image is not an error but a wrong door: it is sent to the upload entry,
 * which owns the only byte→pixel path in the app (D-IE-17). Neither branch
 * decodes anything here, and neither writes a byte.
 */

const UPLOAD_ENTRY = "/create?entry=upload";
const PROJECT_ENTRY = "/create?entry=import-project";

type Notice =
  | { readonly kind: "error"; readonly code: ImportErrorCode; readonly message: string }
  | {
      readonly kind: "redirect";
      readonly message: string;
      readonly to: string;
      readonly linkLabel: string;
    };

export function ImportPatternPanel() {
  const fieldId = useId();
  const [notice, setNotice] = useState<Notice | null>(null);
  const [busy, setBusy] = useState(false);

  async function handleFile(event: ChangeEvent<HTMLInputElement>): Promise<void> {
    const file = event.target.files?.[0];
    setNotice(null);
    if (file === undefined) return;

    // D-IE-6 ①: the size question is answered from `File.size`, before any read.
    const tooLarge = checkImportFileSize(file.size);
    if (tooLarge !== null) {
      setNotice({ kind: "error", ...tooLarge });
      return;
    }

    setBusy(true);
    try {
      const kind = await sniffFile(file);
      if (kind === "png" || kind === "jpeg") {
        setNotice({
          kind: "redirect",
          message: "这是一张图片。图片形式的豆图请走上传入口的像素图路径，那里能选识别方式与框定。",
          to: UPLOAD_ENTRY,
          linkLabel: "去上传入口",
        });
        return;
      }
      if (kind === "json" && hasBeadprojExtension(file.name)) {
        setNotice({
          kind: "redirect",
          message: "这看起来是 .beadproj 项目包，请到「导入项目库」入口导入。",
          to: PROJECT_ENTRY,
          linkLabel: "去导入项目库",
        });
        return;
      }
      setNotice({
        kind: "error",
        code: "UNSUPPORTED_FORMAT",
        message: `${file.name}：认得出这是个豆图文件，但本版本暂不支持解析该格式。`,
      });
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="import-panel">
      <p className="upload__field">
        <label htmlFor={`${fieldId}-file`}>选择豆图文件（.pat / .gamedev / 图片）</label>
        <input id={`${fieldId}-file`} type="file" onChange={(event) => void handleFile(event)} />
      </p>

      {busy && <p className="stub-note">正在识别文件…</p>}

      {notice?.kind === "error" && (
        <p className="stock-form__error" role="alert">
          {notice.message}（{notice.code}）
        </p>
      )}

      {notice?.kind === "redirect" && (
        <p className="stub-note" role="status">
          {notice.message}{" "}
          <Link to={notice.to}>{notice.linkLabel}</Link>
        </p>
      )}
    </div>
  );
}
