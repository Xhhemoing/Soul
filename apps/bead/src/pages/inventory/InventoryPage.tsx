import { useMemo } from "react";

import { useDocumentTitle } from "../../app/useDocumentTitle.ts";
import {
  selectRequirements,
  selectShortages,
  selectSubstituteGroups,
} from "../../stores/inventory.ts";
import { selectInProgress } from "../../stores/projects.ts";
import { useStore } from "../../stores/store.tsx";
import { ShortagePanel } from "./ShortagePanel.tsx";
import { StockForm } from "./StockForm.tsx";
import { StockList } from "./StockList.tsx";
import { SubstitutePanel } from "./SubstitutePanel.tsx";

/**
 * D-INV-13: requirements, shortages, substitutes and the purchase text are all
 * recomputed from `projects` + `inventory` on every render. None of them is
 * written back, so there is nothing to keep in sync and nothing to migrate.
 *
 * D-INV-14: write failures are already covered by the `PersistenceBanner` in
 * AppShell. Nothing on this page adds a second error surface, and CRUD keeps
 * working in memory after a failed write.
 */
export function InventoryPage() {
  useDocumentTitle("资产");
  const { inventory, projects, addInventoryEntry, setInventoryBeads, removeInventoryEntry } =
    useStore();

  const hasInProgress = useMemo(() => selectInProgress(projects).length > 0, [projects]);
  const requirements = useMemo(() => selectRequirements(projects), [projects]);
  const shortages = useMemo(
    () => selectShortages(requirements, inventory),
    [requirements, inventory],
  );
  const groups = useMemo(
    () => selectSubstituteGroups(shortages, requirements, inventory),
    [shortages, requirements, inventory],
  );
  const codes = useMemo(() => inventory.map((entry) => entry.code), [inventory]);

  return (
    <>
      <h1>资产</h1>

      <section className="section" aria-label="色号库存">
        <h2 className="section__title">色号库存</h2>
        <StockForm codes={codes} onAdd={addInventoryEntry} />
        <StockList
          inventory={inventory}
          onSetBeads={setInventoryBeads}
          onRemove={removeInventoryEntry}
        />
      </section>

      <section className="section" aria-label="缺口预警">
        <h2 className="section__title">缺口预警</h2>
        <ShortagePanel
          hasInProgress={hasInProgress}
          requirements={requirements}
          shortages={shortages}
          groups={groups}
        />
      </section>

      <section className="section" aria-label="近似色替代">
        <h2 className="section__title">近似色替代</h2>
        <SubstitutePanel groups={groups} />
      </section>
    </>
  );
}
