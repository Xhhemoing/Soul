export function ProgressBar({ value, label }: { value: number; label: string }) {
  const clamped = Math.min(100, Math.max(0, Math.round(value)));
  return (
    <div
      className="progress-bar"
      role="progressbar"
      aria-label={label}
      aria-valuenow={clamped}
      aria-valuemin={0}
      aria-valuemax={100}
    >
      <div className="progress-bar__fill" style={{ width: `${clamped}%` }} />
    </div>
  );
}
