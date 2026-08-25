import type { ReactNode } from "react";
import { Link } from "react-router";

// D-UI-8: empty states carry the onboarding, so every one of them owes the
// reader a sentence and at least one way forward.

export interface EmptyStateAction {
  label: string;
  to: string;
}

export function EmptyState({
  message,
  actions = [],
  children,
}: {
  message: string;
  actions?: EmptyStateAction[];
  children?: ReactNode;
}) {
  return (
    <div className="empty-state">
      <p className="empty-state__message">{message}</p>
      {actions.length > 0 && (
        <div className="empty-state__actions">
          {actions.map((action) => (
            <Link key={action.to + action.label} className="button button--primary" to={action.to}>
              {action.label}
            </Link>
          ))}
        </div>
      )}
      {children}
    </div>
  );
}
