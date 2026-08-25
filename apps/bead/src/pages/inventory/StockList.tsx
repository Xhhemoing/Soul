import { Card } from "../../components/Card.tsx";
import { ColorSwatch } from "../../components/ColorSwatch.tsx";
import { EmptyState } from "../../components/EmptyState.tsx";
import { MAX_BEADS, selectStock } from "../../stores/inventory.ts";
import type { InventoryEntry } from "../../stores/types.ts";

/**
 * D-INV-7: deletion is immediate. Re-entering a row costs one line of the form
 * that is already sitting above the list, so a confirmation layer would buy
 * nothing.
 */
export function StockList({
  inventory,
  onSetBeads,
  onRemove,
}: {
  inventory: readonly InventoryEntry[];
  onSetBeads: (code: string, beads: number) => void;
  onRemove: (code: string) => void;
}) {
  const stock = selectStock(inventory);
  if (stock.length === 0) {
    return <EmptyState message="还没有库存记录：先在上方录入色号和颗数" />;
  }

  return (
    <ul className="stock-list">
      {stock.map((entry) => (
        <li key={entry.code}>
          <Card className="stock-row">
            <ColorSwatch code={entry.code} hex={entry.hex} name={entry.name} />
            <span className="stock-row__hex">{entry.hex}</span>
            <label className="stock-row__beads">
              <input
                type="number"
                min={0}
                max={MAX_BEADS}
                step={1}
                value={entry.beads}
                aria-label={`${entry.code} 颗数`}
                onChange={(event) => onSetBeads(entry.code, Number(event.target.value))}
              />
              <span>颗</span>
            </label>
            <button className="button" type="button" onClick={() => onRemove(entry.code)}>
              删除 {entry.code}
            </button>
          </Card>
        </li>
      ))}
    </ul>
  );
}
