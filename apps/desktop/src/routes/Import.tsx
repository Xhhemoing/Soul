/**
 * 导入. Point at one exported file, read what is in it, then decide.
 *
 * Two formats and no third way in. There is no OAuth here, no account to sign
 * into and no archive to unpack: the file is chosen by the user in an
 * `<input type="file">`, the WebView reads its text, and the text crosses the
 * IPC to `soulcore`, which does the parsing. The shell has no file-system
 * permission of its own and the core opens no paths, so the only file Soul
 * can read is the one somebody pointed at.
 *
 * What comes back from either half is counts. `ImportPreview` and
 * `ImportReceipt` have no field that could hold a message, a display name or
 * an account handle, so "the preview does not quote the file" is a property of
 * the types rather than a rule this component follows. The file's own text is
 * held here between the two clicks because the core re-parses it on commit
 * rather than keeping somebody's export staged in memory — it is never
 * rendered, and there is nothing on screen it could reach.
 */

import { useEffect, useRef, useState } from "react";

import {
  commitSoulImportV1,
  commitTelegram,
  IMPORT_OVER_BYTE_BUDGET_NOTICE,
  MAX_IMPORT_BYTES,
  previewSoulImportV1,
  previewTelegram,
  type ImportPreview,
  type ImportReceipt,
  type Refusal,
} from "../core";
import { asRefusal, Refused } from "../refusal";

/** The two exports v0.1 will read, and nothing else. */
type Format = "soul-import-v1" | "telegram-desktop";

interface StagedImport {
  readonly format: Format;
  readonly text: string;
  readonly preview: ImportPreview;
}

interface FormatChoice {
  readonly format: Format;
  readonly label: string;
  readonly detail: string;
  readonly accept: string;
}

const FORMATS: readonly FormatChoice[] = [
  {
    format: "soul-import-v1",
    label: "soul-import-v1 JSONL",
    detail:
      "Soul 自己的导入格式：一份 .jsonl 文件，第一行是表头，后面每行一条 JSON，一行一条消息。",
    accept: ".jsonl,.json,.txt",
  },
  {
    format: "telegram-desktop",
    label: "Telegram Desktop 的 result.json",
    detail:
      "Telegram Desktop：Settings → Advanced → Export Telegram data，选 Machine-readable JSON。导出目录里的 result.json 直接选它就行。单聊的 Export chat history 是另一形状，认不得。",
    accept: ".json",
  },
];

function chosen(format: Format): FormatChoice {
  return FORMATS.find((choice) => choice.format === format) ?? (FORMATS[0] as FormatChoice);
}

export function Import(): React.JSX.Element {
  const [format, setFormat] = useState<Format>("soul-import-v1");
  /** Only a successfully previewed file can be committed, with its own format. */
  const [staged, setStaged] = useState<StagedImport | null>(null);
  const [characters, setCharacters] = useState(0);
  const [receipt, setReceipt] = useState<ImportReceipt | null>(null);
  const [refusal, setRefusal] = useState<Refusal | null>(null);
  const [busy, setBusy] = useState(false);
  const generation = useRef(0);
  const fileInput = useRef<HTMLInputElement>(null);

  useEffect(() => () => {
    generation.current += 1;
  }, []);

  const forget = (): void => {
    generation.current += 1;
    if (fileInput.current !== null) fileInput.current.value = "";
    setStaged(null);
    setCharacters(0);
    setReceipt(null);
    setRefusal(null);
    setBusy(false);
  };

  const read = async (file: File | undefined): Promise<void> => {
    forget();
    if (file === undefined) return;
    // The core's own byte budget, checked against the size on disk before
    // anything is read: `file.text()` would hold the whole file in the
    // WebView, and the core would then refuse it with this same sentence.
    // The sentence and the number are the core's constants — this screen
    // composes nothing, it only says the refusal a byte earlier.
    if (file.size > MAX_IMPORT_BYTES) {
      setRefusal({ reason_code: "ROUTINE", explanation: IMPORT_OVER_BYTE_BUDGET_NOTICE });
      return;
    }
    const request = generation.current;
    const selectedFormat = format;
    setBusy(true);
    try {
      const contents = await file.text();
      if (request !== generation.current) return;
      setCharacters(contents.length);
      const preview = await (selectedFormat === "telegram-desktop"
        ? previewTelegram(contents)
        : previewSoulImportV1(contents));
      if (request !== generation.current) return;
      setStaged({ format: selectedFormat, text: contents, preview });
      setBusy(false);
    } catch (error: unknown) {
      if (request !== generation.current) return;
      setRefusal(asRefusal(error));
      setBusy(false);
    }
  };

  const commit = async (): Promise<void> => {
    if (staged === null || busy) return;
    const request = ++generation.current;
    setBusy(true);
    setRefusal(null);
    try {
      const value = await (staged.format === "telegram-desktop"
        ? commitTelegram(staged.text)
        : commitSoulImportV1(staged.text));
      if (request !== generation.current) return;
      setReceipt(value);
      setStaged(null);
      setBusy(false);
    } catch (error: unknown) {
      if (request !== generation.current) return;
      setRefusal(asRefusal(error));
      setBusy(false);
    }
  };

  return (
    <>
      <section className="panel" aria-labelledby="import-heading">
        <h2 id="import-heading">从导出的文件里读往来记录</h2>
        <p className="muted">
          这一版只认下面两种文件，都是你自己从别的软件里导出来的。Soul 不登录任何账号，不联网去取，
          也不去解压压缩包：你挑哪个文件，它就只读那一个文件。
        </p>
        <ul className="facts">
          {FORMATS.map((choice) => (
            <li key={choice.format}>
              <label className="field" htmlFor={`format-${choice.format}`}>
                <input
                  id={`format-${choice.format}`}
                  type="radio"
                  name="import-format"
                  value={choice.format}
                  checked={format === choice.format}
                  disabled={busy}
                  onChange={() => {
                    setFormat(choice.format);
                    forget();
                  }}
                />{" "}
                {choice.label}
              </label>
              <span className="muted">{choice.detail}</span>
            </li>
          ))}
        </ul>

        <label className="field" htmlFor="import-file">
          选择文件
        </label>
        <input
          id="import-file"
          ref={fileInput}
          className="text-input"
          type="file"
          accept={chosen(format).accept}
          disabled={busy}
          onChange={(event) => void read(event.target.files?.[0])}
        />
        {busy ? (
          <p className="muted" role="status">
            正在读取或导入文件，请稍候。
          </p>
        ) : null}
        {characters === 0 ? null : (
          <p className="muted" data-testid="import-file-size">
            读到 {characters} 个字符，正文没有显示在这个页面上。
          </p>
        )}
      </section>

      {refusal === null ? null : (
        <Refused title="这个文件没有读成" refusal={refusal} testId="import-refusal-code" />
      )}

      {staged === null ? null : (
        <Preview preview={staged.preview} busy={busy} onCommit={() => void commit()} onAbandon={forget} />
      )}

      {receipt === null ? null : <Receipt receipt={receipt} />}
    </>
  );
}

interface PreviewProps {
  readonly preview: ImportPreview;
  readonly busy: boolean;
  readonly onCommit: () => void;
  readonly onAbandon: () => void;
}

/**
 * What the file turned out to contain, before it is imported.
 *
 * `writes_anything` is the literal `false` in both the Rust type and the
 * TypeScript one, so this panel cannot be reached by a path that already
 * imported: reading a file and importing it are two commands, and this is the
 * screen between them.
 *
 * That literal is about the *file's* contents and nothing wider. A preview
 * whose export carried injection markers has already appended one
 * `injection.blocked` row to the audit chain by the time this renders, and
 * abandoning the preview leaves it there — so the line below says which rows
 * are not written rather than claiming the database was not touched, and says
 * the audit row out loud on the previews that produced one.
 */
function Preview({ preview, busy, onCommit, onAbandon }: PreviewProps): React.JSX.Element {
  return (
    <section className="panel" aria-labelledby="import-preview-heading">
      <h2 id="import-preview-heading">这个文件里有什么</h2>
      <ul className="facts">
        <li data-testid="preview-source">
          格式：<code>{preview.source}</code>
        </li>
        <li data-testid="preview-counts">
          {preview.participants} 个人，{preview.conversations} 个会话，{preview.messages} 条消息。
        </li>
        <li data-testid="preview-owner">
          {preview.owner_identified
            ? "这份导出里认得出哪一个是你。"
            : "这份导出里认不出哪一个是你，导入会被拒绝。"}
        </li>
        <li data-testid="preview-injection">
          {preview.messages_with_injection_markers === 0
            ? "没有哪一条消息写成了命令的样子。"
            : `有 ${preview.messages_with_injection_markers} 条消息写成了命令的样子。它们会被数出来、照原样入库，Soul 不会照着做。`}
        </li>
        <li data-testid="preview-writes">
          {preview.writes_anything
            ? ""
            : preview.messages_with_injection_markers === 0
              ? "到这一步，这个文件里的人、会话、消息一条都没有写进库里。"
              : "到这一步，这个文件里的人、会话、消息一条都没有写进库里；但上面数出来的那几条已经在审计链上留下了一行「挡下了注入」，你现在换一个文件，那一行也还在。"}
        </li>
        <li data-testid="preview-reimport">
          这一版不记得「这个文件我导过了」：同一份文件再导一次，人不会重复，但里面的往来记录会再写一遍，人脉图上的往来次数和关系强度也会跟着涨。
        </li>
      </ul>
      <div className="switch-row">
        <button
          type="button"
          className="primary"
          onClick={onCommit}
          disabled={busy || !preview.owner_identified}
        >
          确认导入
        </button>
        <button type="button" onClick={onAbandon} disabled={busy}>
          不了，换一个文件
        </button>
      </div>
      <p className="badge" data-testid="preview-notice">
        {preview.notice}
      </p>
    </section>
  );
}

interface ReceiptProps {
  readonly receipt: ImportReceipt;
}

/**
 * What the import wrote, in the same currency the preview quoted: counts.
 *
 * The re-import line is here as well as on the preview because this is the
 * screen a user is on when they wonder whether the last attempt went through.
 * v0.1 keeps no external-id index, so committing the same export twice writes
 * its events twice and the graph counts every duplicate as a real interaction:
 * contacts match by identifier digest and do not clone, tie strength does.
 * Saying so is the whole of it — nothing here deduplicates.
 */
function Receipt({ receipt }: ReceiptProps): React.JSX.Element {
  return (
    <section className="panel" aria-labelledby="import-receipt-heading">
      <h2 id="import-receipt-heading">导入完成</h2>
      <ul className="facts">
        <li data-testid="receipt-source">
          格式：<code>{receipt.source}</code>
        </li>
        <li data-testid="receipt-contacts">
          新建 {receipt.contacts_created} 个人，对上了已经认识的 {receipt.contacts_matched} 个人。
        </li>
        <li data-testid="receipt-events">
          写进 {receipt.events_written} 条往来记录，{receipt.evidence_written} 条证据。
        </li>
        <li data-testid="receipt-ties">
          人脉图重算出 {receipt.ties_rebuilt} 条关系，可以到「人脉图」和「灵魂档案」去看。
        </li>
        <li data-testid="receipt-injection">
          {receipt.messages_with_injection_markers === 0
            ? "没有哪一条消息写成了命令的样子。"
            : `有 ${receipt.messages_with_injection_markers} 条写成了命令的样子，已经当材料存下来，没有被执行。`}
        </li>
        <li data-testid="receipt-reimport">
          这一版不记得哪份文件导过：把同一份文件再导一遍，人会对上不会重复，但上面这些往来记录和证据会再写一遍，人脉图的往来次数和关系强度也会跟着涨。
        </li>
      </ul>
      <p className="badge" data-testid="receipt-notice">
        {receipt.notice}
      </p>
    </section>
  );
}
