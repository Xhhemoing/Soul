import { EmptyState } from "../../components/EmptyState.tsx";
import { useDocumentTitle } from "../../app/useDocumentTitle.ts";

export function NotFoundPage() {
  useDocumentTitle("页面不存在");
  return (
    <>
      <h1>页面不存在</h1>
      <EmptyState message="这个地址没有对应的页面。" actions={[{ label: "回到灵感", to: "/explore" }]} />
    </>
  );
}
