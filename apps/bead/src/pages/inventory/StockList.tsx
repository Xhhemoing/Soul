import { Card } from "../../components/Card.tsx";
import { ColorSwatch } from "../../components/ColorSwatch.tsx";
import { EmptyState } from "../../components/EmptyState.tsx";
import { MAX_BEADS, paletteCodeKey, selectStock } from "../../stores/inventory.ts";
import type { InventoryEntry, PaletteNamespaceId } from "../../stores/types.ts";

/**
 * D-INV-7: deletion is immediate. Re-entering a row costs one line of the form
 * that is already sitting above the list, so a confirmation layer would buy
 * nothing.
 *
 * §4.2: rows are ordered namespace first, and every callback carries both
 * halves of the identity key — two rows can legitimately show the same code.
 */
export function StockList({
  inventory,
  onSetBeads,
  onRemove,
}: {
  inventory: readonly InventoryEntry[];
  onSetBeads: (paletteId: PaletteNamespaceId, code: string, beads: number) => void;
  onRemove: (paletteId: PaletteNamespaceId, code: string) => void;
}) {
  const stock = selectStock(inventory);
  if (stock.length === 0) {
    return <EmptyState message="还没有库存记录：先在上方录入色号和颗数" />;
  }

  return (
    <ul className="stock-list">
      {stock.map((entry) => (
        <li key={paletteCodeKey(entry)}>
          <Card className="stock-row">
            <ColorSwatch
              code={entry.code}
              hex={entry.hex}
              name={entry.name}
              paletteId={entry.paletteId}
            />
            <span className="stock-row__hex">{entry.hex}</span>
            <span className="stock-row__beads">
              <input
                type="number"
                min={0}
                max={MAX_BEADS}
                step={1}
                value={entry.beads}
                aria-label={`${entry.code} 颗数`}
                onChange={(event) =>
                  onSetBeads(entry.paletteId, entry.code, Number(event.target.value))
                }
              />
              <span>颗</span>
            </span>
            <button
              className="button"
              type="button"
              onClick={() => onRemove(entry.paletteId, entry.code)}
            >
              删除 {entry.code}
            </button>
          </Card>
        </li>
      ))}
    </ul>
  );
}
