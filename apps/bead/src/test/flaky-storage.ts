/**
 * A localStorage stand-in whose quota can be exhausted mid-session, which is
 * the condition DATA-1 is about: writes start throwing while the already
 * stored (now stale) copy stays perfectly readable.
 */
export class FlakyStorage implements Storage {
  private readonly entries = new Map<string, string>();
  full = false;

  get length(): number {
    return this.entries.size;
  }

  key(index: number): string | null {
    return [...this.entries.keys()][index] ?? null;
  }

  getItem(key: string): string | null {
    return this.entries.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    if (this.full) throw new DOMException("quota exceeded", "QuotaExceededError");
    this.entries.set(key, value);
  }

  removeItem(key: string): void {
    this.entries.delete(key);
  }

  clear(): void {
    this.entries.clear();
  }
}
