import { Card } from "../../components/Card.tsx";
import { ColorSwatch } from "../../components/ColorSwatch.tsx";
import { EmptyState } from "../../components/EmptyState.tsx";
import type { SubstituteGroup } from "../../stores/inventory.ts";

/**
 * D-INV-9: a candidate is never presented as a colour alone. Code, name, hex,
 * the ΔE00 distance and the beads left over after that colour serves its own
 * demand are all on the row, so the suggestion survives a monochrome screen and
 * a colour-blind reader alike.
 */
export function SubstitutePanel({ groups }: { groups: readonly SubstituteGroup[] }) {
  if (groups.length === 0) {
    return <EmptyState message="没有缺口时不需要替代建议" />;
  }

  return (
    <ul className="substitute-list">
      {groups.map((group) => (
        <li key={group.wanted.code}>
          <Card>
            <p className="substitute-group__wanted">
              <ColorSwatch
                code={group.wanted.code}
                hex={group.wanted.hex}
                name={group.wanted.name}
              />
              <span>缺 {group.wanted.shortage} 颗</span>
            </p>
            {group.candidates.length === 0 ? (
              <p className="substitute-group__none">{"库存内没有 ΔE00 < 3 的替代"}</p>
            ) : (
              <ul className="substitute-group__candidates">
                {group.candidates.map((candidate) => (
                  <li key={candidate.code}>
                    <ColorSwatch code={candidate.code} hex={candidate.hex} name={candidate.name} />
                    <span>{candidate.hex}</span>
                    <span>ΔE00 {candidate.deltaE.toFixed(2)}</span>
                    <span>余 {candidate.remaining} 颗</span>
                  </li>
                ))}
              </ul>
            )}
          </Card>
        </li>
      ))}
    </ul>
  );
}
