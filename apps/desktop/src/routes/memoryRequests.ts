/** Page ownership only. No prose, database decisions, or core cancellation. */
export class MemoryRequests {
  private live = false;
  private serial = 0;
  private active: number | null = null;
  private preview: object | null = null;
  private listSerial = 0;

  activate(): void {
    this.live = true;
  }

  dispose(): void {
    this.live = false;
    this.active = null;
    this.preview = null;
    this.serial += 1;
    this.listSerial += 1;
  }

  begin(quoted?: object): number | null {
    if (!this.live || this.active !== null) return null;
    if (quoted !== undefined && quoted !== this.preview) return null;
    if (quoted === undefined) this.preview = null;
    this.listSerial += 1;
    this.active = ++this.serial;
    return this.active;
  }

  owns(ticket: number): boolean {
    return this.live && this.active === ticket;
  }

  finish(ticket: number): boolean {
    if (!this.owns(ticket)) return false;
    this.active = null;
    return true;
  }

  remember(ticket: number, preview: object | null): void {
    if (this.owns(ticket)) this.preview = preview;
  }

  abandon(): boolean {
    if (!this.live || this.active !== null) return false;
    this.preview = null;
    return true;
  }

  nextList(): number {
    return ++this.listSerial;
  }

  ownsList(ticket: number): boolean {
    return this.live && this.active === null && this.listSerial === ticket;
  }
}
