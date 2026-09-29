/**
 * Page-local admission and result ownership. This does not cancel IPC, revoke
 * permissions or claim that an already-started core operation was rolled back.
 * No domain state is cached here: a successful write still replaces the graph
 * with the core's complete response.
 */
export interface SummaryRequest {
  readonly contactId: string;
  readonly sequence: number;
}

export class GraphRequests {
  private active = false;
  private sequence = 0;
  private write: number | null = null;
  private summary: SummaryRequest | null = null;

  activate(): void {
    this.active = true;
  }

  deactivate(): void {
    this.active = false;
    this.sequence += 1;
    this.write = null;
    this.summary = null;
  }

  /** Claim synchronously, before invoking an IPC factory or setting state. */
  beginWrite(): number | null {
    if (!this.active || this.write !== null) return null;
    this.summary = null;
    this.write = ++this.sequence;
    return this.write;
  }

  finishWrite(request: number): boolean {
    if (!this.active || this.write !== request) return false;
    this.write = null;
    return true;
  }

  /** A newer person may supersede an older request; a duplicate may not. */
  beginSummary(contactId: string): SummaryRequest | null {
    if (!this.active || this.write !== null || this.summary?.contactId === contactId) {
      return null;
    }
    const request = { contactId, sequence: ++this.sequence };
    this.summary = request;
    return request;
  }

  /** Success and failure consume the same ticket, at most once. */
  finishSummary(request: SummaryRequest): boolean {
    if (!this.active || this.write !== null || this.summary !== request) return false;
    this.summary = null;
    return true;
  }
}