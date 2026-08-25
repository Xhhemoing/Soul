import { useEffect } from "react";

/** Per-route <title>, asserted in the navigation tests. */
export function useDocumentTitle(title: string): void {
  useEffect(() => {
    document.title = `${title} · BeadFlow`;
  }, [title]);
}
