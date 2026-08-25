/**
 * D-ASM-7: three labelled buttons in a named cluster, not an anonymous floating
 * blob. 下一步 / 撤销 move the persisted cursor; 锁定当前行 opens a row window
 * inside the current step and is session-only.
 */
export interface ControlOrbProps {
  readonly onNext: () => void;
  readonly onUndo: () => void;
  readonly onToggleRowLock: () => void;
  readonly canAdvance: boolean;
  readonly canUndo: boolean;
  readonly rowLock: boolean;
  /** Row-by-row already is a row at a time, so the window has nothing to add. */
  readonly rowLockUnavailable: boolean;
}

export function ControlOrb({
  onNext,
  onUndo,
  onToggleRowLock,
  canAdvance,
  canUndo,
  rowLock,
  rowLockUnavailable,
}: ControlOrbProps) {
  return (
    <div className="assemble__orb" role="group" aria-label="拼装控制">
      <button type="button" className="assemble__orb-button" onClick={onNext} disabled={!canAdvance}>
        下一步
      </button>
      <button type="button" className="assemble__orb-button" onClick={onUndo} disabled={!canUndo}>
        撤销
      </button>
      <button
        type="button"
        className="assemble__orb-button"
        onClick={onToggleRowLock}
        aria-pressed={rowLock}
        disabled={rowLockUnavailable}
        title={rowLockUnavailable ? "逐行模式本身即按行" : undefined}
      >
        锁定当前行
      </button>
    </div>
  );
}
