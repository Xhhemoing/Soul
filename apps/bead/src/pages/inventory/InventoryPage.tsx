import { Card } from "../../components/Card.tsx";
import { ColorSwatch } from "../../components/ColorSwatch.tsx";
import { EmptyState } from "../../components/EmptyState.tsx";
import { useDocumentTitle } from "../../app/useDocumentTitle.ts";
import { selectStock } from "../../stores/inventory.ts";
import { useStore } from "../../stores/store.tsx";

export function InventoryPage() {
  useDocumentTitle("资产");
  const { inventory } = useStore();
  const stock = selectStock(inventory);

  return (
    <>
      <h1>资产</h1>

      <section className="section" aria-label="色号库存">
        <h2 className="section__title">色号库存</h2>
        {stock.length > 0 ? (
          <ul className="stock-list">
            {stock.map((entry) => (
              <li key={entry.code}>
                <Card>
                  <ColorSwatch code={entry.code} hex={entry.hex} name={entry.name} beads={entry.beads} />
                </Card>
              </li>
            ))}
          </ul>
        ) : (
          <EmptyState message="录入库存后才能做缺口预警" actions={[{ label: "录入库存", to: "/create" }]}>
            <p className="stub-note">库存录入与编辑归 WP-B05，这里先给入口占位。</p>
          </EmptyState>
        )}
      </section>

      <section className="section" aria-label="缺口预警">
        <h2 className="section__title">缺口预警</h2>
        <EmptyState message="缺口预警要拿项目 BOM 和库存对比，归 WP-B05。" />
      </section>

      <section className="section" aria-label="近似色替代">
        <h2 className="section__title">近似色替代</h2>
        <EmptyState message="ΔE00 小于 3 的替代建议归 WP-B05，色差算法本身归 WP-B01。" />
      </section>
    </>
  );
}
