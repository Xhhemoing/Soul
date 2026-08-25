// a11y rule from the IA review: colour is never the only signal. The code text
// is part of the component so no caller can render a bare chip.

export function ColorSwatch({
  code,
  hex,
  name,
  beads,
}: {
  code: string;
  hex: string;
  name?: string;
  beads?: number;
}) {
  return (
    <span className="swatch">
      <span className="swatch__chip" style={{ background: hex }} aria-hidden="true" />
      <span>
        {code}
        {name ? ` ${name}` : ""}
        {beads === undefined ? "" : ` · ${beads} 颗`}
      </span>
    </span>
  );
}
