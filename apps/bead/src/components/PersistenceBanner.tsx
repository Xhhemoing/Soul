import { useStore } from "../stores/store.tsx";

/**
 * DATA-1: when the repository stops persisting, every later change in the
 * session is lost too — so this is a standing banner and not a toast. Colour is
 * never the only signal (D-UI-4): the sentence says what happened.
 */
export function PersistenceBanner() {
  const { persistenceFailed } = useStore();
  if (!persistenceFailed) return null;
  return (
    <p className="persistence-banner" role="alert">
      本地存储不可用，本次更改不会保存。
    </p>
  );
}
