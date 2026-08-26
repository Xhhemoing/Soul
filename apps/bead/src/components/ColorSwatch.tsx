import { PALETTE_NAMESPACE_LABEL, type PaletteNamespaceId } from "../stores/types.ts";

// a11y rule from the IA review: colour is never the only signal. The code text
// is part of the component so no caller can render a bare chip.
//
// BD20 adds a second thing a code alone cannot carry: which palette it belongs
// to. Gallery `G07` and generic-5mm `G07` are different beads, so wherever a
// namespace is known it is rendered in its own span ahead of the code.

export function ColorSwatch({
  code,
  hex,
  name,
  beads,
  paletteId,
}: {
  code: string;
  hex: string;
  name?: string;
  beads?: number;
  paletteId?: PaletteNamespaceId;
}) {
  return (
    <span className="swatch">
      <span className="swatch__chip" style={{ background: hex }} aria-hidden="true" />
      {paletteId !== undefined && (
        <span className="swatch__namespace">【{PALETTE_NAMESPACE_LABEL[paletteId]}】</span>
      )}
      <span>
        {code}
        {name ? ` ${name}` : ""}
        {beads === undefined ? "" : ` · ${beads} 颗`}
      </span>
    </span>
  );
}
