import type { BackdropKind } from "../../stores/types.ts";
import { BACKDROP_LABEL } from "./backdrop.ts";

const BACKDROP_OPTIONS: readonly BackdropKind[] = ["black", "white", "custom"];

/**
 * D-UI-6: the immersive backdrop is a project field, never the app theme. The
 * testid, the `data-backdrop` attribute and the three radio names are load
 * bearing for the theme-backdrop suite — keep them.
 */
export function AssembleBackdrop({ color, kind }: { readonly color: string; readonly kind: BackdropKind }) {
  return (
    <div
      className="assemble__backdrop"
      data-testid="assemble-backdrop"
      data-backdrop={kind}
      style={{ background: color }}
    />
  );
}

export interface BackdropControlsProps {
  readonly kind: BackdropKind;
  readonly color: string;
  readonly onSelect: (kind: BackdropKind) => void;
  readonly onCustomColor: (color: string) => void;
}

export function BackdropControls({ kind, color, onSelect, onCustomColor }: BackdropControlsProps) {
  return (
    <fieldset className="assemble__backdrop-controls">
      <legend>拼装背景（跟随项目，与应用主题无关）</legend>
      {BACKDROP_OPTIONS.map((option) => (
        <label key={option}>
          <input
            type="radio"
            name="backdrop"
            value={option}
            checked={kind === option}
            onChange={() => onSelect(option)}
          />
          {BACKDROP_LABEL[option]}
        </label>
      ))}
      {kind === "custom" && (
        <label>
          背景色
          <input
            type="color"
            value={color}
            onChange={(event) => onCustomColor(event.target.value)}
          />
        </label>
      )}
    </fieldset>
  );
}
