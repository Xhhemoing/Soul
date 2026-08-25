import { useId, useState } from "react";

import type { BomRow } from "../../algo/bom.ts";
import { ColorSwatch } from "../../components/ColorSwatch.tsx";
import { GENERIC_5MM_PALETTE } from "../../stores/types.ts";
import { EDITOR_SWATCHES, type ActiveColor, type Symmetry, type Tool } from "./editor.ts";

/**
 * D-ED-8 / D-ED-9 / D-ED-12 / D-ED-13: everything that is not the board.
 *
 * There is no eraser tool. The active colour is a palette index or「空」, so the
 * brush with「空」 selected erases and the bucket with「空」 selected clears a
 * region — half the tool states disappear and the mental model gets smaller.
 */

/** D-ED-6: the document's palette is always generic-5mm, all 48 codes. */
const SWATCHES = EDITOR_SWATCHES;

const EMPTY_VALUE = "empty";
const EMPTY_LABEL = "空（橡皮）";

const TOOLS: ReadonlyArray<readonly [Tool, string]> = [
  ["brush", "画笔"],
  ["bucket", "油漆桶"],
  ["eyedropper", "拾色器"],
];

const SYMMETRIES: ReadonlyArray<readonly [Symmetry, string]> = [
  ["none", "关"],
  ["x", "左右镜像"],
  ["y", "上下镜像"],
  ["quad", "四向"],
];

export interface EditorToolsProps {
  readonly tool: Tool;
  readonly activeColor: ActiveColor;
  readonly symmetry: Symmetry;
  readonly bom: readonly BomRow[];
  readonly total: number;
  readonly canUndo: boolean;
  readonly canRedo: boolean;
  readonly onTool: (tool: Tool) => void;
  readonly onColor: (color: ActiveColor) => void;
  readonly onSymmetry: (symmetry: Symmetry) => void;
  readonly onUndo: () => void;
  readonly onRedo: () => void;
  readonly onReplace: (from: number, to: ActiveColor) => void;
}

export function EditorTools({
  tool,
  activeColor,
  symmetry,
  bom,
  total,
  canUndo,
  canRedo,
  onTool,
  onColor,
  onSymmetry,
  onUndo,
  onRedo,
  onReplace,
}: EditorToolsProps) {
  const fieldId = useId();
  const [fromValue, setFromValue] = useState<string>("");
  const [toValue, setToValue] = useState<string>("0");

  // The source list is「文档实际用到的码」, so it shrinks as colours leave the
  // board. Resolving the selection against the current list on every render —
  // rather than storing it back — is what keeps a stale code from being offered.
  const source = bom.find((row) => String(row.index) === fromValue) ?? bom[0];
  const target: ActiveColor = toValue === EMPTY_VALUE ? null : Number(toValue);
  const replaceBlocked = source === undefined || source.index === target;

  return (
    <div className="editor__tools">
      <div className="editor__history">
        <button type="button" className="button" disabled={!canUndo} onClick={onUndo}>
          撤销
        </button>
        <button type="button" className="button" disabled={!canRedo} onClick={onRedo}>
          重做
        </button>
      </div>

      <fieldset className="editor__group">
        <legend>工具</legend>
        {TOOLS.map(([value, label]) => (
          <label key={value}>
            <input
              type="radio"
              name={`${fieldId}-tool`}
              checked={tool === value}
              onChange={() => onTool(value)}
            />
            {label}
          </label>
        ))}
      </fieldset>

      <fieldset className="editor__group">
        <legend>对称</legend>
        {SYMMETRIES.map(([value, label]) => (
          <label key={value}>
            <input
              type="radio"
              name={`${fieldId}-symmetry`}
              checked={symmetry === value}
              onChange={() => onSymmetry(value)}
            />
            {label}
          </label>
        ))}
        <span className="stub-note">对称只作用于画笔，油漆桶与替换不受影响</span>
      </fieldset>

      <div className="editor__palette" role="group" aria-label="活动色（通用5mm 48 色）">
        <button
          type="button"
          className="editor__swatch"
          aria-pressed={activeColor === null}
          onClick={() => onColor(null)}
        >
          {EMPTY_LABEL}
        </button>
        {SWATCHES.map((swatch, index) => (
          <button
            key={swatch.code}
            type="button"
            className="editor__swatch"
            aria-pressed={activeColor === index}
            onClick={() => onColor(index)}
          >
            <ColorSwatch code={swatch.code} name={swatch.name} hex={swatch.hex} />
          </button>
        ))}
      </div>

      <section className="editor__panel" aria-label="色号替换">
        <h2 className="section__title">色号替换</h2>
        {bom.length === 0 ? (
          <p className="stub-note">画布还是空的，没有可替换的色号。</p>
        ) : (
          <>
            <p className="editor__field">
              <label htmlFor={`${fieldId}-from`}>要替换的色号</label>
              <select
                id={`${fieldId}-from`}
                value={source === undefined ? "" : String(source.index)}
                onChange={(event) => setFromValue(event.target.value)}
              >
                {bom.map((row) => (
                  <option key={row.code} value={String(row.index)}>
                    {row.code} {row.displayName} · {row.count} 颗
                  </option>
                ))}
              </select>
            </p>
            <p className="editor__field">
              <label htmlFor={`${fieldId}-to`}>替换成</label>
              <select
                id={`${fieldId}-to`}
                value={toValue}
                onChange={(event) => setToValue(event.target.value)}
              >
                <option value={EMPTY_VALUE}>{EMPTY_LABEL}</option>
                {SWATCHES.map((swatch, index) => (
                  <option key={swatch.code} value={String(index)}>
                    {swatch.code} {swatch.name}
                  </option>
                ))}
              </select>
            </p>
            <p className="stub-note">将改写 {source === undefined ? 0 : source.count} 颗</p>
            <button
              type="button"
              className="button"
              disabled={replaceBlocked}
              onClick={() => {
                if (source === undefined) return;
                onReplace(source.index, target);
              }}
            >
              应用替换
            </button>
          </>
        )}
      </section>

      <section className="editor__panel" aria-label="用色统计">
        <h2 className="section__title">用色统计</h2>
        {bom.length === 0 ? (
          <p className="stub-note">画布还是空的，合计 0 颗。</p>
        ) : (
          <>
            {/* D-ED-13: a row is the fastest way back to a colour already on the
                board, so the row itself is the control. */}
            <ul className="editor__stats" aria-label="用色清单">
              {bom.map((row) => (
                <li key={row.code}>
                  <button type="button" className="editor__stat" onClick={() => onColor(row.index)}>
                    <ColorSwatch
                      paletteId={GENERIC_5MM_PALETTE}
                      code={row.code}
                      name={row.displayName}
                      hex={SWATCHES[row.index]?.hex ?? "#808080"}
                      beads={row.count}
                    />
                  </button>
                </li>
              ))}
            </ul>
            <p className="stub-note">合计 {total} 颗</p>
          </>
        )}
      </section>
    </div>
  );
}
