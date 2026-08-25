/**
 * Fixed-seed PRNG for the property tests. A 32-bit xorshift is enough here and
 * has the property that matters: the same seed produces the same stream on
 * every machine and in every run, so a failure is reproducible.
 */
export function seededRandom(seed: number): () => number {
  let state = seed >>> 0;
  if (state === 0) state = 0x9e3779b9;
  return () => {
    state ^= state << 13;
    state >>>= 0;
    state ^= state >>> 17;
    state ^= state << 5;
    state >>>= 0;
    return state / 0x100000000;
  };
}

export function seededInt(next: () => number, maxExclusive: number): number {
  return Math.min(maxExclusive - 1, Math.floor(next() * maxExclusive));
}
