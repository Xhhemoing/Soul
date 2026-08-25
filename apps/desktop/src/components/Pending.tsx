/**
 * An empty route that says who owns it.
 *
 * Deliberately not a mock-up. A screen that looks like the finished feature is
 * worse than an empty one: it invites someone to wire a button to nothing, and
 * in this product some of those buttons are the ones v0.1 has promised not to
 * ship at all.
 */

export interface PendingProps {
  readonly title: string;
  readonly ownedBy: string;
  readonly detail: string;
}

export function Pending({ title, ownedBy, detail }: PendingProps): React.JSX.Element {
  return (
    <section className="panel pending" aria-labelledby="pending-heading">
      <h2 id="pending-heading">{title}</h2>
      <p className="badge" data-testid="pending-owner">
        {ownedBy} 未落地
      </p>
      <p className="muted">{detail}</p>
    </section>
  );
}
