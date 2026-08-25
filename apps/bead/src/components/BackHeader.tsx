import { Link } from "react-router";

/** Detail routes have no bottom-nav highlight, so they carry their own way back. */
export function BackHeader({ to, label, title }: { to: string; label: string; title: string }) {
  return (
    <header className="section">
      <Link className="button" to={to}>
        ← {label}
      </Link>
      <h1>{title}</h1>
    </header>
  );
}
