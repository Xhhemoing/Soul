import { useId, useState, type ChangeEvent } from "react";

import {
  BEADPROJ_ARCHIVE_FILE_NAME,
  checkImportFileSize,
  parseBeadprojText,
  serializeBeadproj,
  type ImportFailure,
} from "../../schema/beadproj.ts";
import { useStore } from "../../stores/store.tsx";
import { collectBeadprojArchive, downloadBeadproj } from "./export.ts";
import {
  describeImportResult,
  importBeadproj,
  type BeadprojImportResult,
} from "./import.ts";

/**
 * WP-B07's `/create?entry=import-project` panel: the project library's
 * two-way backup channel (D-IE-2, D-IE-15 ①). Import on the left of the same
 * panel, 「导出全部」 on the right — one file format, one route, no new page.
 *
 * Every write goes through the actions that already existed: the document
 * first via `savePatternDoc`, then `addProject`, then the cursor (D-IE-10).
 * The panel itself stores nothing but its own report.
 */

const EXPORT_UNAVAILABLE = "这个环境不支持文件下载";

export function ImportProjectPanel() {
  const fieldId = useId();
  const {
    projects,
    inventory,
    progress,
    addProject,
    addInventoryEntry,
    upsertProgress,
    loadPatternDoc,
    savePatternDoc,
  } = useStore();

  const [failure, setFailure] = useState<ImportFailure | null>(null);
  const [result, setResult] = useState<BeadprojImportResult | null>(null);
  const [importing, setImporting] = useState(false);
  const [exportNote, setExportNote] = useState<string | null>(null);
  const [exportError, setExportError] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);

  async function handleFile(event: ChangeEvent<HTMLInputElement>): Promise<void> {
    const file = event.target.files?.[0];
    setFailure(null);
    setResult(null);
    if (file === undefined) return;

    const tooLarge = checkImportFileSize(file.size);
    if (tooLarge !== null) {
      setFailure(tooLarge);
      return;
    }

    setImporting(true);
    try {
      const parsed = parseBeadprojText(await file.text());
      if (!parsed.ok) {
        setFailure(parsed.failure);
        return;
      }
      setResult(
        await importBeadproj(parsed.document, {
          savePatternDoc,
          addProject,
          upsertProgress,
          addInventoryEntry,
          inventory,
        }),
      );
    } finally {
      setImporting(false);
    }
  }

  async function handleExportAll(): Promise<void> {
    setExportNote(null);
    setExportError(null);
    setExporting(true);
    try {
      const { file, skippedProjects } = await collectBeadprojArchive({
        projects,
        inventory,
        progress,
        loadPatternDoc,
      });
      // D-IE-16: the blob URL is minted here, inside the click, and never in a
      // render path (B05 review MED-1).
      if (!downloadBeadproj(BEADPROJ_ARCHIVE_FILE_NAME, serializeBeadproj(file))) {
        setExportError(EXPORT_UNAVAILABLE);
        return;
      }
      setExportNote(
        skippedProjects === 0
          ? `已导出 ${file.projects.length} 个项目到 ${BEADPROJ_ARCHIVE_FILE_NAME}`
          : `已导出 ${file.projects.length} 个项目到 ${BEADPROJ_ARCHIVE_FILE_NAME}；` +
              `另有 ${skippedProjects} 个项目读不到豆图文档，未纳入归档`,
      );
    } finally {
      setExporting(false);
    }
  }

  const hasSomethingToExport = projects.length > 0 || inventory.length > 0;

  return (
    <div className="import-panel">
      <p className="upload__field">
        <label htmlFor={`${fieldId}-file`}>选择 .beadproj 项目包</label>
        <input
          id={`${fieldId}-file`}
          type="file"
          accept=".beadproj,application/json"
          onChange={(event) => void handleFile(event)}
        />
      </p>

      {importing && <p className="stub-note">正在导入…</p>}

      {failure !== null && (
        <p className="stock-form__error" role="alert">
          {failure.message}（{failure.code}）
        </p>
      )}

      {result !== null && (
        <>
          <p className="stub-note" role="status">
            {describeImportResult(result)}
          </p>
          {result.entryErrors.length > 0 && (
            <ul className="import-panel__errors" aria-label="被拒绝的条目">
              {result.entryErrors.map((error) => (
                <li key={error.index} className="stock-form__error">
                  第 {error.index} 条：{error.message}（{error.code}）
                </li>
              ))}
            </ul>
          )}
        </>
      )}

      {/* D-IE-15 ①: nothing to back up, no control — the D-INV-11 mirror. */}
      {hasSomethingToExport && (
        <div className="import-panel__export">
          <button
            className="button"
            type="button"
            disabled={exporting}
            onClick={() => void handleExportAll()}
          >
            导出全部
          </button>
          {exportError !== null && (
            <p className="stock-form__error" role="alert">
              {exportError}
            </p>
          )}
          {exportNote !== null && (
            <p className="stub-note" role="status">
              {exportNote}
            </p>
          )}
        </div>
      )}
    </div>
  );
}
