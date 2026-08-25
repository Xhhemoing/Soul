import { useEffect, useId, useMemo } from "react";

import { Card } from "../../components/Card.tsx";
import { ColorSwatch } from "../../components/ColorSwatch.tsx";
import { EmptyState } from "../../components/EmptyState.tsx";
import {
  PURCHASE_FILE_NAME,
  buildPurchaseText,
  type RequirementRow,
  type ShortageRow,
  type SubstituteGroup,
} from "../../stores/inventory.ts";

/**
 * D-INV-11: two delivery channels, neither of them a permission prompt. The
 * read-only textarea is there to be selected by hand, the anchor hands over a
 * `Blob`. The Clipboard API is deliberately absent (round2-data §7), and BD15
 * keeps every URL out of the text itself — the object URL is minted at runtime.
 */
function createDownloadHref(text: string): string | null {
  const factory = globalThis.URL?.createObjectURL;
  if (typeof factory !== "function") return null;
  return factory.call(globalThis.URL, new Blob([text], { type: "text/plain;charset=utf-8" }));
}

function PurchaseExport({ text }: { text: string }) {
  const textId = useId();
  const href = useMemo(() => createDownloadHref(text), [text]);

  useEffect(() => {
    if (href === null) return undefined;
    return () => {
      globalThis.URL.revokeObjectURL(href);
    };
  }, [href]);

  return (
    <div className="purchase-export">
      <label htmlFor={textId}>采购清单文本</label>
      <textarea id={textId} className="purchase-export__text" readOnly rows={8} value={text} />
      {href !== null && (
        <a className="button" href={href} download={PURCHASE_FILE_NAME}>
          下载采购清单
        </a>
      )}
    </div>
  );
}

/**
 * D-INV-8: an empty inventory is not an empty state here. Zero stock produces
 * the full shortage list, which is the whole point of a shopping list.
 */
export function ShortagePanel({
  hasInProgress,
  requirements,
  shortages,
  groups,
}: {
  hasInProgress: boolean;
  requirements: readonly RequirementRow[];
  shortages: readonly ShortageRow[];
  groups: readonly SubstituteGroup[];
}) {
  const text = useMemo(() => buildPurchaseText(shortages, groups), [shortages, groups]);

  if (!hasInProgress) {
    return (
      <EmptyState
        message="没有正在拼或待拼的项目，先去挑一张图纸"
        actions={[{ label: "去灵感挑图纸", to: "/explore" }]}
      />
    );
  }
  if (requirements.length === 0) {
    return <EmptyState message="当前项目没有配色清单来源，无法计算缺口" />;
  }
  if (shortages.length === 0) {
    return <EmptyState message="库存足够，当前没有缺口" />;
  }

  return (
    <>
      <ul className="shortage-list">
        {shortages.map((row) => (
          <li key={row.code}>
            <Card className="shortage-row">
              <ColorSwatch code={row.code} hex={row.hex} name={row.name} />
              <span>需求 {row.required} 颗</span>
              <span>库存 {row.inStock} 颗</span>
              <strong>缺 {row.shortage} 颗</strong>
            </Card>
          </li>
        ))}
      </ul>
      <PurchaseExport text={text} />
    </>
  );
}
