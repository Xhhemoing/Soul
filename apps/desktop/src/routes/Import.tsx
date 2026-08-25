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

import { useState } from "react";

import {
  commitSoulImportV1,
  commitTelegram,
  previewSoulImportV1,
  previewTelegram,
  type ImportPreview,
  type ImportReceipt,
  type Refusal,
} from "../core";
import { asRefusal, Refused } from "../refusal";

/** The two exports v0.1 will read, and nothing else. */
type Format = "soul-import-v1" | "telegram-desktop";

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
      "Telegram Desktop 桌面版里 Export chat history → Machine-readable JSON 导出的那份 result.json，直接选它就行。",
    accept: ".json",
  },
];

function chosen(format: Format): FormatChoice {
  return FORMATS.find((choice) => choice.format === format) ?? (FORMATS[0] as FormatChoice);
}

export function Import(): React.JSX.Element {
  const [format, setFormat] = useState<Format>("soul-import-v1");
  /** The picked file's text. Sent to the core twice, rendered never. */
  const [text, setText] = useState<string | null>(null);
  const [characters, setCharacters] = useState(0);
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [receipt, setReceipt] = useState<ImportReceipt | null>(null);
  const [refusal, setRefusal] = useState<Refusal | null>(null);
  const [busy, setBusy] = useState(false);

  const forget = (): void => {
    setText(null);
    setCharacters(0);
    setPreview(null);
    setReceipt(null);
    setRefusal(null);
  };

  const read = (file: File | undefined): void => {
    forget();
    if (file === undefined) return;
    setBusy(true);
    file.text().then(
      (contents) => {
        setText(contents);
        setCharacters(contents.length);
        const reading =
          format === "telegram-desktop" ? previewTelegram(contents) : previewSoulImportV1(contents);
        reading.then(
          (value) => {
            setPreview(value);
            setBusy(false);
          },
          (error: unknown) => {
            setRefusal(asRefusal(error));
            setBusy(false);
          },
        );
      },
      (error: unknown) => {
        setRefusal(asRefusal(error));
        setBusy(false);
      },
    );
  };

  const commit = (): void => {
    if (text === null) return;
    setBusy(true);
    setRefusal(null);
    const writing =
      format === "telegram-desktop" ? commitTelegram(text) : commitSoulImportV1(text);
    writing.then(
      (value) => {
        setReceipt(value);
        setPreview(null);
        setBusy(false);
      },
      (error: unknown) => {
        setRefusal(asRefusal(error));
        setBusy(false);
      },
    );
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
          className="text-input"
          type="file"
          accept={chosen(format).accept}
          disabled={busy}
          onChange={(event) => read(event.target.files?.[0])}
        />
        {characters === 0 ? null : (
          <p className="muted" data-testid="import-file-size">
            读到 {characters} 个字符，正文没有显示在这个页面上。
          </p>
        )}
      </section>

      {refusal === null ? null : (
        <Refused title="这个文件没有读成" refusal={refusal} testId="import-refusal-code" />
      )}

      {preview === null ? null : (
        <Preview preview={preview} busy={busy} onCommit={commit} onAbandon={forget} />
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

/** What the import wrote, in the same currency the preview quoted: counts. */
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
      </ul>
      <p className="badge" data-testid="receipt-notice">
        {receipt.notice}
      </p>
    </section>
  );
}
