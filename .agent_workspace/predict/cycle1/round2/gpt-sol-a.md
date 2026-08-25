MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Deterministic `FakeForegroundSource` fixture recipes

These are specifications for later tests, not crate code. Each fixture owns a fresh store, granted collection consent, and independent model state. A sequence boundary is part of the oracle: transitions never cross fixture or trace boundaries. Each persisted session is checked by opening its `body_ref`; executable identity must not be inferred from plaintext event columns.

| Notation | Meaning |
|---|---|
| `switch(app); poll(t)` | Call `switch_to(app)`, then `poll_once(t)` using Unix milliseconds. |
| `poll(t)` | Poll without changing the fake's current foreground application. |
| `clear; poll(t)` | Call `clear()`, then poll; this closes an open session and creates an explicit trace gap. |
| `finish(t)` | Orderly stop; emits the final open session. |
| `record X/d` | `Poll::SessionRecorded`; the opened body is `{app: X, duration_ms: d}`. |
| `start`, `same`, `none`, `error` | `SessionStarted`, `SameApp`, `NothingInForeground`, or the stated collection error. |

## SYN-MARKOV — directed counts, cycles, and a non-MFU answer

Use one trace with `M0 = 1_800_000_000_000`. Every step is 10,000 ms after the previous step.

| Step | Fake/source sequence | Expected collector result | Newly persisted body |
|---:|---|---|---|
| 0 | `switch(alpha.exe); poll(M0)` | `start` | — |
| 1 | `switch(beta.exe); poll(M0 + 10_000)` | `record alpha.exe/10_000` | `alpha.exe` |
| 2 | `switch(alpha.exe); poll(M0 + 20_000)` | `record beta.exe/10_000` | `beta.exe` |
| 3 | `switch(beta.exe); poll(M0 + 30_000)` | `record alpha.exe/10_000` | `alpha.exe` |
| 4 | `switch(alpha.exe); poll(M0 + 40_000)` | `record beta.exe/10_000` | `beta.exe` |
| 5 | `switch(gamma.exe); poll(M0 + 50_000)` | `record alpha.exe/10_000` | `alpha.exe` |
| 6 | `switch(alpha.exe); poll(M0 + 60_000)` | `record gamma.exe/10_000` | `gamma.exe` |
| 7 | `switch(beta.exe); poll(M0 + 70_000)` | `record alpha.exe/10_000` | `alpha.exe` |
| 8 | `switch(alpha.exe); poll(M0 + 80_000)` | `record beta.exe/10_000` | `beta.exe` |
| 9 | `switch(gamma.exe); poll(M0 + 90_000)` | `record alpha.exe/10_000` | `alpha.exe` |
| 10 | `switch(alpha.exe); poll(M0 + 100_000)` | `record gamma.exe/10_000` | `gamma.exe` |
| 11 | `switch(beta.exe); poll(M0 + 110_000)` | `record alpha.exe/10_000` | `alpha.exe` |
| 12 | `switch(alpha.exe); poll(M0 + 120_000)` | `record beta.exe/10_000` | `beta.exe` |
| 13 | `finish(M0 + 130_000)` | final event written | `alpha.exe/10_000` |

| Oracle | Exact expectation |
|---|---|
| Decrypted session sequence | `alpha, beta, alpha, beta, alpha, gamma, alpha, beta, alpha, gamma, alpha, beta, alpha` |
| Directed counts | `alpha→beta = 4`, `alpha→gamma = 2`, `beta→alpha = 4`, `gamma→alpha = 2`; no other edge |
| Row normalization | From `alpha`: `beta = 4/6`, `gamma = 2/6`; from `beta` and `gamma`: `alpha = 1` |
| Marginal counts | `alpha = 7`, `beta = 4`, `gamma = 2` |
| Query after the final `alpha` | First-order prediction is `beta`, supported by 4 of 6 outgoing `alpha` transitions; a marginal-frequency predictor instead ranks `alpha` first |
| Failure conditions | Inventing `alpha→alpha`, treating edges as undirected, dropping repeated edges, crossing another trace boundary, or producing a row whose probabilities do not sum to one |

## SYN-RHYTHM — equal marginals, separable UTC hours

All eight rows write to this fixture's store, but each row is a fresh one-session trace with a fresh fake and collector. This prevents a next-app edge from being invented between morning and evening observations.

| Trace | Fake/source sequence | Expected opened body | UTC bucket |
|---|---|---|---:|
| R1 | `switch(alpha.exe); poll(1_785_744_000_000); finish(1_785_744_300_000)` | `alpha.exe/300_000` | 08 |
| R2 | `switch(beta.exe); poll(1_785_787_200_000); finish(1_785_787_500_000)` | `beta.exe/300_000` | 20 |
| R3 | `switch(alpha.exe); poll(1_785_830_400_000); finish(1_785_830_700_000)` | `alpha.exe/300_000` | 08 |
| R4 | `switch(beta.exe); poll(1_785_873_600_000); finish(1_785_873_900_000)` | `beta.exe/300_000` | 20 |
| R5 | `switch(alpha.exe); poll(1_785_916_800_000); finish(1_785_917_100_000)` | `alpha.exe/300_000` | 08 |
| R6 | `switch(beta.exe); poll(1_785_960_000_000); finish(1_785_960_300_000)` | `beta.exe/300_000` | 20 |
| R7 | `switch(alpha.exe); poll(1_786_003_200_000); finish(1_786_003_500_000)` | `alpha.exe/300_000` | 08 |
| R8 | `switch(beta.exe); poll(1_786_046_400_000); finish(1_786_046_700_000)` | `beta.exe/300_000` | 20 |

| Oracle | Exact expectation |
|---|---|
| UTC histogram | `(alpha, 08) = 4`, `(beta, 20) = 4`; all cross-cells are zero |
| Marginals | `alpha = 4`, `beta = 4`; frequency alone cannot choose |
| UTC-hour queries | At hour 08 rank `alpha`; at hour 20 rank `beta`; support is 4 of 4 in each populated bucket |
| Unseen-hour query | At hour 12 use the separately declared backoff/abstention rule; do not fabricate hour-specific support |
| Transition oracle | Empty: every row is a separate trace |
| Failure conditions | Joining rows into transitions, weighting 300 seconds as 300 observations, random train/test shuffling, or calling these UTC buckets local time |

## SYN-NOISE — coalescing, gaps, source failure, and clock rollback

N1–N4 are separate traces. They may share this fixture's store, but the harness must retain trace and gap markers outside the event list.

| Trace/step | Fake/source sequence | Expected collector result | Persisted consequence |
|---|---|---|---|
| N1.0 | `switch(alpha.exe); poll(1_800_100_000_000)` | `start` | — |
| N1.1 | `poll(1_800_100_001_000)` | `same` | — |
| N1.2 | `switch(ALPHA.EXE); poll(1_800_100_002_000)` | `same` after lowercase normalization | — |
| N1.3 | `switch(beta.exe); poll(1_800_100_005_000)` | `record alpha.exe/5_000` | one `alpha`, not three |
| N1.4 | `finish(1_800_100_009_000)` | final event written | `beta.exe/4_000` |
| N2.0 | `switch(gamma.exe); poll(1_800_200_000_000)` | `start` | — |
| N2.1 | `clear; poll(1_800_200_003_000)` | `record gamma.exe/3_000` | close span before a gap |
| N2.2 | `poll(1_800_200_004_000)` | `none` | no event |
| N2.3 | `switch(delta.exe); poll(1_800_200_005_000)` | `start` | new span |
| N2.4 | `finish(1_800_200_007_000)` | final event written | `delta.exe/2_000` |
| N3.0 | `switch(epsilon.exe); poll(1_800_300_000_000)` | `start` | — |
| N3.1 | `switch(theta.exe); fail_next(Platform("fixture")); poll(1_800_300_001_000)` | `error`; fake now shows `theta`, while collector retains open `epsilon` | no event |
| N3.2 | `poll(1_800_300_002_000)` | `record epsilon.exe/2_000` and start `theta` | duration spans an unobserved switch |
| N3.3 | `finish(1_800_300_003_000)` | final event written | `theta.exe/1_000` |
| N4.0 | `switch(zeta.exe); poll(1_800_400_010_000)` | `start` | — |
| N4.1 | `switch(eta.exe); poll(1_800_400_009_000)` | `record zeta.exe/0` by saturating rollback | `zeta.ts` is later than `eta.ts` |
| N4.2 | `finish(1_800_400_011_000)` | final event written | `eta.exe/2_000` |

| Oracle layer | Exact expectation |
|---|---|
| Collector output by insertion order | N1 `[alpha/5000, beta/4000]`; N2 `[gamma/3000, delta/2000]`; N3 `[epsilon/2000, theta/1000]`; N4 `[zeta/0, eta/2000]` |
| Gap/error-aware transition oracle | Only `alpha→beta = 1` and `zeta→eta = 1`; no `gamma→delta`, no `epsilon→theta`, and no cross-trace edge |
| Coalescing oracle | Repeated and differently cased `alpha` samples remain one session; there is no `alpha→alpha` |
| Ordering oracle | N4 remains `zeta→eta` in insertion/session-end order even though sorting solely by event `ts` reverses it |
| Source-error limitation | N3's stored rows are indistinguishable from a clean adjacent pair. An error-aware live harness can split them; a store-only rebuild cannot honestly claim to do so until error/gap provenance is retained |
| Failure conditions | Flattening all stored bodies into one sequence, bridging `clear` or source error, rejecting legal zero duration, sorting only by `ts`, or counting polls rather than completed sessions |

## ADV-DST — UTC is observable; local civil hour is not

Each row is a fresh one-session trace. `America/New_York` and the local-time columns are hidden oracle metadata, not predictor input. The store receives only UTC `ts`, executable identity, and duration. In 2026 New York leaves daylight time on November 1 at 06:00 UTC.

| Trace | Fake/source sequence | Persisted UTC start | Hidden New York oracle |
|---|---|---|---|
| D1 | `switch(alpha.exe); poll(1_793_019_600_000); finish(1_793_019_900_000)` | 2026-10-26 13:00Z | 09:00 EDT (UTC−04) |
| D2 | `switch(alpha.exe); poll(1_793_192_400_000); finish(1_793_192_700_000)` | 2026-10-28 13:00Z | 09:00 EDT (UTC−04) |
| D3 | `switch(alpha.exe); poll(1_793_365_200_000); finish(1_793_365_500_000)` | 2026-10-30 13:00Z | 09:00 EDT (UTC−04) |
| D4 | `switch(fold-a.exe); poll(1_793_511_000_000); finish(1_793_511_300_000)` | 2026-11-01 05:30Z | first 01:30, EDT (UTC−04) |
| D5 | `switch(fold-b.exe); poll(1_793_514_600_000); finish(1_793_514_900_000)` | 2026-11-01 06:30Z | second 01:30, EST (UTC−05) |
| D6 | `switch(alpha.exe); poll(1_793_628_000_000); finish(1_793_628_300_000)` | 2026-11-02 14:00Z | 09:00 EST (UTC−05) |
| D7 | `switch(alpha.exe); poll(1_793_800_800_000); finish(1_793_801_100_000)` | 2026-11-04 14:00Z | 09:00 EST (UTC−05) |
| D8 | `switch(alpha.exe); poll(1_793_973_600_000); finish(1_793_973_900_000)` | 2026-11-06 14:00Z | 09:00 EST (UTC−05) |

| Evaluation mode | Required oracle |
|---|---|
| Production-parity input: no timezone/offset | Report only `(alpha, UTC hour 13) = 3` and `(alpha, UTC hour 14) = 3`; label both explicitly as UTC. Do not emit “09:00 local,” “morning,” or one six-count local bucket. |
| Hypothetical explicit `America/New_York` input | `(alpha, local hour 09) = 6`; D4 and D5 share civil `01:30` but remain two distinct instants with different offsets/fold positions. |
| Incorrect fixed UTC−04 conversion | Must fail: D6–D8 become 10:00 instead of 09:00. |
| Incorrect UTC-as-local relabeling | Must fail: it reports local hours 13/14 and silently changes the meaning of the stored timestamp. |
| Unidentifiability check | The exact same stored UTC rows also admit other local-hour histories in other zones. The hidden New York column may score an explicitly zone-aware future model, but must never leak into training or be inferred as a stored fact. |
