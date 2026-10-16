import type { E1DraftPlan } from "../core";

export type DraftRequestKind = "local" | "prepare" | "original" | "generate" | "discard";

interface Slot {
  readonly kind: DraftRequestKind;
}

interface LaneState {
  readonly busy: boolean;
  readonly uncertain: boolean;
}

/**
 * discard_draft has no preparation ID. Its physical completion must precede
 * the next preparation, including when the route is unmounted and reopened.
 * An unknown IPC result is not completion evidence: endpoint work then stays
 * blocked until the application session is restarted. Local templates remain
 * available. This is page coordination, not core cancellation or authorization.
 */
export class DraftLane {
  private active: Slot | null = null;
  private state: LaneState = { busy: false, uncertain: false };
  private readonly listeners = new Set<() => void>();

  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => { this.listeners.delete(listener); };
  };

  readonly snapshot = (): LaneState => this.state;

  isPreparing(): boolean {
    return this.active?.kind === "prepare" || this.active?.kind === "original";
  }

  begin(kind: DraftRequestKind): Slot | null {
    if (this.active !== null || (this.state.uncertain && kind !== "local")) return null;
    const slot = { kind };
    this.active = slot;
    this.publish(true, this.state.uncertain);
    return slot;
  }

  owns(slot: Slot): boolean {
    return this.active === slot;
  }

  finish(slot: Slot, unknown: boolean): boolean {
    if (!this.owns(slot)) return false;
    this.active = null;
    this.publish(false, this.state.uncertain || (unknown && slot.kind !== "local"));
    return true;
  }

  private publish(busy: boolean, uncertain: boolean): void {
    this.state = { busy, uncertain };
    this.listeners.forEach((listener) => listener());
  }
}

/** Shared by route instances; never reset just because a page was unmounted. */
export const draftLane = new DraftLane();

export interface DraftQuote {
  readonly plan: E1DraftPlan;
  readonly revision: number;
  readonly text: string;
}

export interface DraftTicket {
  readonly slot: Slot;
  readonly lifetime: number;
  readonly revision: number;
  readonly text: string;
}

/** Synchronous admission and exact input/confirmation ownership for one page. */
export class DraftRequests {
  private live = false;
  private lifetime = 0;
  private revision = 0;
  private text = "";
  private quote: DraftQuote | null = null;

  constructor(private readonly lane = draftLane) {}

  activate(): void {
    this.live = true;
  }

  dispose(): void {
    this.live = false;
    this.lifetime += 1;
    this.quote = null;
    // Do not release the physical slot: the IPC may still be running.
  }

  edit(text: string): number {
    this.text = text;
    return this.invalidate();
  }

  invalidate(): number {
    this.revision += 1;
    this.quote = null;
    return this.revision;
  }

  hasQuote(): boolean { return this.quote !== null; }

  begin(kind: DraftRequestKind, revision: number, quote?: DraftQuote): DraftTicket | null {
    if (!this.live || revision !== this.revision) return null;
    if (kind === "generate" || kind === "original" || kind === "discard") {
      if (quote === undefined || quote !== this.quote || quote.revision !== this.revision) return null;
      if (kind === "original" && quote.plan.carries_exempted_original) return null;
    }
    if (kind !== "discard" && this.text.trim() === "") return null;
    const snapshot = { lifetime: this.lifetime, revision: this.revision, text: this.text };
    const slot = this.lane.begin(kind);
    if (slot === null) return null;
    this.quote = null;
    return { slot, ...snapshot };
  }

  owns(ticket: DraftTicket): boolean {
    return this.current(ticket) && this.lane.owns(ticket.slot);
  }

  remember(ticket: DraftTicket, plan: E1DraftPlan): DraftQuote | null {
    if (!this.owns(ticket)) return null;
    this.quote = { plan, revision: ticket.revision, text: ticket.text };
    return this.quote;
  }

  finish(ticket: DraftTicket, unknown = false): boolean {
    const owned = this.owns(ticket);
    // Release even a stale page ticket once its actual IPC has settled.
    return this.lane.finish(ticket.slot, unknown) && owned && this.current(ticket);
  }

  private current(ticket: DraftTicket): boolean {
    return this.live && ticket.lifetime === this.lifetime && ticket.revision === this.revision;
  }
}

export function isDraftRefusal(error: unknown): boolean {
  if (error === null || typeof error !== "object") return false;
  const shaped = error as Record<string, unknown>;
  return typeof shaped.reason_code === "string" && typeof shaped.explanation === "string";
}
