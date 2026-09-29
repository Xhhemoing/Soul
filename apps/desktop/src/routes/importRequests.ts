/** The existing two import formats; no new parser or IPC contract. */
export type ImportFormat = "soul-import-v1" | "telegram-desktop";

export interface ImportRead {
  readonly format: ImportFormat;
  readonly sequence: number;
}

/**
 * Page-local ownership of one read, its preview, and its confirmation.
 * Admission happens before an asynchronous function is invoked, not when React
 * eventually renders a disabled button. Tickets contain no file text.
 * This is neither message deduplication nor core cancellation: unmounting only
 * makes this page ignore results, and a committed import is not rolled back.
 */
export class ImportRequests {
  private active = false;
  private sequence = 0;
  private reading: ImportRead | null = null;
  private ready: ImportRead | null = null;
  private writing: number | null = null;

  activate(): void {
    this.active = true;
  }

  deactivate(): void {
    this.active = false;
    this.invalidate();
  }

  /** Abandon a preview, not an in-flight read or an already-started commit. */
  reset(): boolean {
    if (!this.active || this.reading !== null || this.writing !== null) return false;
    this.invalidate();
    return true;
  }

  beginRead(format: ImportFormat): ImportRead | null {
    if (!this.active || this.reading !== null || this.writing !== null) return null;
    this.ready = null;
    const request = { format, sequence: ++this.sequence };
    this.reading = request;
    return request;
  }

  isReading(request: ImportRead): boolean {
    return this.active && this.reading === request;
  }

  /** Only a successful preview may be used by a later confirmation. */
  finishRead(request: ImportRead, succeeded: boolean): boolean {
    if (!this.isReading(request)) return false;
    this.reading = null;
    this.ready = succeeded ? request : null;
    return true;
  }

  beginCommit(preview: ImportRead): number | null {
    if (!this.active || this.writing !== null || this.ready !== preview) return null;
    this.writing = ++this.sequence;
    return this.writing;
  }

  /** Preserve an unsuccessful preview for the existing explicit retry path. */
  finishCommit(request: number, succeeded: boolean): boolean {
    if (!this.active || this.writing !== request) return false;
    this.writing = null;
    if (succeeded) this.ready = null;
    return true;
  }

  private invalidate(): void {
    this.sequence += 1;
    this.reading = null;
    this.ready = null;
    this.writing = null;
  }
}
