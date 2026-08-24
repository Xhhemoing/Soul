# Blockers Round 2 synthesis (BINDING)

MODEL_SLUG: cursor-grok-4.6-high (parent)
Date: 2026-08-24
Frozen user doc: `docs/BLOCKERS.md` status `BLOCKERS_FROZEN`

## Calls

| Round 2 claim | Parent call |
|---|---|
| G4 dedup is not matrix P0 (fable-b, gpt-sol-b) | **Agree → P1.** STATUS already discloses double-import. |
| Fan-out last_contact is band-changing P0 (gpt-sol-a, fable-b Δ3) | **P1 / known limit.** Telegram lacks member lists; "zero outgoing" over-corrects. T4D count gates already stop Strong-from-groups. Document after wiring. |
| S2 KnownIdentifiers is AC-12 P0 (fable-b) | **P1 until a failing fixture exists** (gpt-sol-b). |
| §3 header "矩阵" is wrong (opus-a) | **Agree.** Closing needs lock slices AND matrix. Retitled. |
| G5 under-scoped (opus-a) | **Agree in prose.** Close-gate remains wizard + /profile; other unwired slices listed, not new Goal 2. |
| A2 cannot alias DEMOTE (fable-b, opus-a/b) | **Agree.** Workspace equality test; no third 180 in product crates; no crate merge this phase. |
| `points_for` has no second 3/10/3 (opus-a) | **Agree.** A2 swap is COPY_ZH, not constant-dedup. |
| conversation_id mapping (fable-b, gpt-sol-a) | **Taken into G1.** |
| GC-9 vs COPY_ZH (fable-b) | **Taken.** DECISIONS additive template first. |
| Probe corpus guard (opus-b) | **Taken.** Exact 25+2*probe+3*unix. |
| DPAPI in soul-winkeys not soul-store (opus-b) | **Taken.** Round 1 in-crate sketch withdrawn. |
| `open_with_keys` so "green each step" is possible (opus-b) | **Taken.** |
| `--no-fail-fast` not separate P0 (gpt-sol-b) | **Taken.** Same push as fixture. |
| perl/NASM and tauri --no-bundle IGNORE (gpt-sol-b) | **Taken.** |
| Cherry-pick / rename not P0 (gpt-sol-b) | **Taken.** M1 P0 is stop+close #4. |
| merge-commit not correctness P0 (gpt-sol-b) | **Taken.** Strong preference. |
| This PR lands on main first (gpt-sol-b) | **Taken.** |
| PR #2 is draft; absorb path unstated (fable-a) | **Taken.** |
| S3 P2 (opus-a, opus-b) | **Taken.** |
| DPAPI does not fail any AC row (opus-a) | **Taken as matrix fact.** Still P0-ship; document the wording hole like S5. |
| A2 u64 / generic ID (gpt-sol-a) | Adapter first; interface widen allowed. |

Rejected: promoting DPAPI before T4D; cfg!(windows) expected lists; relaxing corpus to `>= 25`; F04c third gate; Goal 2.
