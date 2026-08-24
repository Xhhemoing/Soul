/**
 * The draft page: paste, generate, read. Nothing here sends anything.
 *
 * Empty paste, redaction, and which route the draft took are questions the
 * core answers. This file renders the answer, including the sentence that
 * says there is no code path that would send it.
 */

import { useState } from "react";

import { draftView, refusalText, type DraftView } from "../core";

export function Draft(): React.JSX.Element {
  const [paste, setPaste] = useState("");
  const [view, setView] = useState<DraftView | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);

  const generate = async (): Promise<void> => {
    try {
      const result = await draftView([paste]);
      setView(result);
      setRefusal(null);
    } catch (error) {
      setView(null);
      setRefusal(refusalText(error));
    }
  };

  return (
    <section className="panel" aria-labelledby="draft-paste-heading">
      <h2 id="draft-paste-heading">要回复的那段话</h2>
      <p className="muted">贴进来，生成本机草稿。没有发出去的按钮，这一版也没有发出去的代码。</p>
      <div className="field-row">
        <label htmlFor="draft-paste">粘贴</label>
        <textarea
          id="draft-paste"
          value={paste}
          rows={8}
          onChange={(event) => setPaste(event.target.value)}
        />
        <button
          type="button"
          className="primary"
          onClick={() => {
            void generate();
          }}
        >
          生成草稿
        </button>
      </div>
      {refusal === null ? null : (
        <p className="refusal" role="alert">
          {refusal}
        </p>
      )}
      {view === null ? null : (
        <div data-testid="draft-result">
          <p className="badge" data-testid="draft-route">
            {view.route_label}
          </p>
          <p data-testid="draft-text">{view.text}</p>
          <p className="muted" data-testid="draft-placeheld">
            占位 {view.placeheld_turns} 段
          </p>
          <p data-testid="draft-notice">{view.notice}</p>
        </div>
      )}
    </section>
  );
}
