import { SPLIT_MODES, type SplitMode } from "../../algo/steps.ts";
import { MODE_LABEL, formatDuration } from "./session.ts";

/**
 * D-ASM-10, spelled out rather than hidden in a tooltip: a browser converts
 * `mm` at a nominal 96dpi, so on a scaled or high-density display the printed
 * size is off by a double-digit percentage and the only honest fix is a ruler.
 */
export const PHYSICAL_DISCLAIMER =
  "1:1 按 CSS 毫米近似：显示器未校准，实际大小可能有偏差。28 格应约 140mm，可用尺核对。";

export const MODE_SWITCH_HINT = "切换模式将从第 1 步开始（用时不清零）";

/** Isolated so the once-a-second tick has the smallest possible blast radius. */
export function SessionClock({ elapsedMs }: { readonly elapsedMs: number }) {
  return <output className="assemble__clock">{formatDuration(elapsedMs)}</output>;
}

export interface SessionHUDProps {
  readonly elapsedMs: number;
  readonly beadsPerMinute: number | null;
  readonly stepNumber: number;
  readonly stepTotal: number;
  readonly percent: number;
  readonly stepDescription: string;
  readonly mode: SplitMode;
  readonly onModeChange: (mode: SplitMode) => void;
  readonly physical: boolean;
  readonly onTogglePhysical: () => void;
  readonly announcement: string;
}

export function SessionHUD({
  elapsedMs,
  beadsPerMinute,
  stepNumber,
  stepTotal,
  percent,
  stepDescription,
  mode,
  onModeChange,
  physical,
  onTogglePhysical,
  announcement,
}: SessionHUDProps) {
  return (
    <div className="assemble__hud">
      <dl className="assemble__readout">
        <div>
          <dt>用时</dt>
          <dd>
            <SessionClock elapsedMs={elapsedMs} />
          </dd>
        </div>
        <div>
          <dt>速度</dt>
          <dd>{beadsPerMinute === null ? "—" : `${beadsPerMinute} 颗/分`}</dd>
        </div>
        <div>
          <dt>进度</dt>
          <dd>
            第 {stepNumber}/{stepTotal} 步 · {percent}%
          </dd>
        </div>
        <div>
          <dt>当前</dt>
          <dd>{stepDescription}</dd>
        </div>
      </dl>

      <fieldset className="assemble__modes">
        <legend>步骤模式</legend>
        {SPLIT_MODES.map((candidate) => (
          <label key={candidate}>
            <input
              type="radio"
              name="split-mode"
              value={candidate}
              checked={mode === candidate}
              onChange={() => onModeChange(candidate)}
            />
            {MODE_LABEL[candidate]}
          </label>
        ))}
        <p className="assemble__hint">{MODE_SWITCH_HINT}</p>
      </fieldset>

      <div className="assemble__scale">
        <label>
          <input type="checkbox" checked={physical} onChange={onTogglePhysical} />
          1:1 透光模式
        </label>
        {physical && <p className="assemble__hint">{PHYSICAL_DISCLAIMER}</p>}
      </div>

      <p className="assemble__status" role="status">
        {announcement}
      </p>
    </div>
  );
}
