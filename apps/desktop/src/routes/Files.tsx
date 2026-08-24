/**
 * The file-plan page: pick an authorised directory, scan, read the preview.
 *
 * Nothing here moves, renames, or deletes a file. The core returns a plan
 * whose `written_to_disk` is false by construction; this file renders that
 * plan and the sentence that says carrying it out is later.
 */

import { useEffect, useState } from "react";

import { authorizedRoots, fileplanView, refusalText, type FilePlanView } from "../core";

export function Files(): React.JSX.Element {
  const [roots, setRoots] = useState<readonly string[]>([]);
  const [target, setTarget] = useState("");
  const [view, setView] = useState<FilePlanView | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    authorizedRoots().then(
      (value) => {
        if (live) setRoots(value);
      },
      (error: unknown) => {
        if (live) setRefusal(refusalText(error));
      },
    );
    return () => {
      live = false;
    };
  }, []);

  const preview = async (): Promise<void> => {
    try {
      const result = await fileplanView(target);
      setView(result);
      setRefusal(null);
    } catch (error) {
      setView(null);
      setRefusal(refusalText(error));
    }
  };

  return (
    <section className="panel" aria-labelledby="files-heading">
      <h2 id="files-heading">授权目录</h2>
      {roots.length === 0 ? (
        <p className="muted" data-testid="files-no-roots">
          还没有授权任何目录。去
          <a href="#/settings">设置</a>
          把要看的目录点头授权之后，再回到这里看建议。
        </p>
      ) : (
        <ul className="facts" data-testid="files-roots">
          {roots.map((root) => (
            <li key={root}>
              <label>
                <input
                  type="radio"
                  name="fileplan-root"
                  value={root}
                  checked={target === root}
                  onChange={() => setTarget(root)}
                />{" "}
                {root}
              </label>
            </li>
          ))}
        </ul>
      )}
      <div className="field-row">
        <button
          type="button"
          className="primary"
          onClick={() => {
            void preview();
          }}
        >
          扫描并预览
        </button>
      </div>
      {refusal === null ? null : (
        <p className="refusal" role="alert">
          {refusal}
        </p>
      )}
      {view === null ? null : (
        <div data-testid="fileplan-result">
          <p className="muted" data-testid="fileplan-counts">
            {view.file_count} 个文件，{view.dir_count} 个目录，{view.entry_count} 条建议
          </p>
          <table className="defaults" data-testid="fileplan-entries">
            <thead>
              <tr>
                <th scope="col">建议</th>
                <th scope="col">从</th>
                <th scope="col">到</th>
              </tr>
            </thead>
            <tbody>
              {view.entries.map((entry) => (
                <tr
                  key={`${entry.action}:${entry.source_rel}`}
                  data-testid="fileplan-row"
                >
                  <td>{entry.action_label}</td>
                  <td>{entry.source_rel}</td>
                  <td>{entry.target_rel ?? ""}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p data-testid="fileplan-notice">{view.notice}</p>
        </div>
      )}
    </section>
  );
}
