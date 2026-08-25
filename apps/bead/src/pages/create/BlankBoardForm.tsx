import { useId, useState } from "react";
import { useNavigate } from "react-router";

import { BOARD_28, BOARD_56 } from "../../algo/framing.ts";
import { createGrid } from "../../algo/grid.ts";
import { mintProjectId } from "../../stores/ids.ts";
import { createPatternDoc } from "../../stores/patterns.ts";
import { createProjectFromBlank } from "../../stores/projects.ts";
import { useStore } from "../../stores/store.tsx";

/**
 * D-ED-4: the `/create?entry=blank` panel, now a mint form.
 *
 * It stays in the entry panel slot (D-UI-1) — the entry being chosen is already
 * in the URL, and the thing that needs its own route is the editor the form
 * hands off to (D-ED-3), not the form.
 *
 * The confirm sequence is D-UP-10's, reused verbatim: the empty document is
 * awaited onto disk first, and only then does a project point at it. A project
 * whose grid is not stored is a project that opens to nothing, so a failed
 * write mints nothing and says so inline.
 */

const UNTITLED = "未命名豆图";
const MAX_TITLE_LENGTH = 64;
export const BLANK_SAVE_ERROR = "空白豆图没有保存，项目也没有创建";

/** D-ED-5: two square presets, 56 per axis being the v0 ceiling (D-UP-7). */
const SIZES = [BOARD_28, BOARD_56] as const;

export function BlankBoardForm() {
  const fieldId = useId();
  const navigate = useNavigate();
  const { addProject, savePatternDoc } = useStore();

  const [title, setTitle] = useState("");
  const [size, setSize] = useState<(typeof SIZES)[number]>(BOARD_28);
  const [error, setError] = useState<string | null>(null);
  const [minting, setMinting] = useState(false);

  async function mint(): Promise<void> {
    if (minting) return;
    setMinting(true);
    setError(null);

    const projectId = mintProjectId();
    const trimmed = title.trim().slice(0, MAX_TITLE_LENGTH) || UNTITLED;
    // All -1, no provenance: nothing converted this board, a person is about to
    // draw it. `paletteId` is generic-5mm like every v0 document (D-ED-6).
    const doc = createPatternDoc(projectId, createGrid(size, size));

    try {
      await savePatternDoc(doc);
    } catch {
      setError(BLANK_SAVE_ERROR);
      setMinting(false);
      return;
    }

    addProject(createProjectFromBlank(projectId, trimmed));
    setMinting(false);
    void navigate(`/edit/${projectId}`);
  }

  return (
    <div className="upload">
      <p className="upload__field">
        <label htmlFor={`${fieldId}-title`}>项目名称</label>
        <input
          id={`${fieldId}-title`}
          value={title}
          maxLength={MAX_TITLE_LENGTH}
          placeholder={UNTITLED}
          onChange={(event) => setTitle(event.target.value)}
        />
      </p>

      <fieldset className="upload__group">
        <legend>板型</legend>
        {SIZES.map((value) => (
          <label key={value}>
            <input
              type="radio"
              name={`${fieldId}-size`}
              checked={size === value}
              onChange={() => setSize(value)}
            />
            {value}×{value}
          </label>
        ))}
      </fieldset>

      <button
        type="button"
        className="button button--primary"
        disabled={minting}
        onClick={() => void mint()}
      >
        新建并开始编辑
      </button>

      {error !== null && (
        <p className="stock-form__error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
