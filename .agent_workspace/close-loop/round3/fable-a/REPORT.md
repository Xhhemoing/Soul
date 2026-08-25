claude-fable-5-thinking-xhigh (requested; the serving model self-identifies as Claude Fable 5 with thinking enabled — `cursor-cloud run-info` returns `originalModelName: null`, so the literal variant string is unverifiable from inside the run; no downgrade observed)

# Round 3 · fable-a — final SOTA verdict on closing Goal 1

Slot: fable-a, Round 3 (final) of Goal 1 close-loop. Branch: `cursor/goal1-close-loop-a073`, no
side branch created. Verdict written against HEAD `b210bfd`; nothing outside this directory was
written. No implementation, no Goal 2, no hosted-green claim.

## Verdict: NO — Goal 1 cannot be CLOSED this loop (D54)

D54 defines closed as: every v0.1 row of the FORMAL acceptance matrix passes **and** the thirteen
PRODUCT_LOCK vertical slices hold **simultaneously**. Both gates are half-open today, and both
residues have the same shape: they are not code or docs on this tree. The loop has exhausted the
code side of the close gate; what remains is author hands, hosted minutes, and GitHub merges.
Declaring closure this round would require claiming hosted CI green (forbidden by the round rules,
and false) and real-machine proof that does not exist.

## The honest split: matrix code vs author-manual vs minutes

### Matrix code — exhausted

Every D54-gating item that is code or docs on this tree has landed and is green locally.
Independently re-verified by me at HEAD `b210bfd` (after the last code commit `0f8a073`):

- `cargo test -p soul-graph --offline`: 48 passed, 0 failed (2 + 7 + 12 + 11 + 16;
  `graph_correction` includes Round 3's mixed-store pin, `t4d_product` includes Round 2's four
  named-fixture tests).
- `cargo test -p soul-import --offline`: 45 passed, 0 failed (includes the AC-34 second-sentence
  assertions).
- `cargo run -p xtask -- schema-freeze --check`: `docs/schemas` matches the lock.

gpt-sol-a's Round 3 run extends this to five packages — 190/190 green at `029d8d6`
(`round3/gpt-sol-a/TEST_LOG.txt`) — and the only code commit since is test-only.

Row arithmetic, honestly: the matrix has 33 v0.1 rows (AC-01…AC-26 and AC-28…AC-34; AC-27 is
v0.1.1 and holds no row). 31 rows (AC-02…AC-25, AC-28…AC-34) have their Then provable by tests on
this tree, and those tests are green locally. The two rows that structurally cannot be closed from
this tree are AC-01 (its 谁跑 column says author-manual + install smoke) and AC-26 (hosted five-gate
pipeline plus an author-signed NSIS installer, D56). One asterisk on the 31: AC-28's clause
「解释文案同屏报两个数」 is proven in A2 (`people_summary` renders the graph-written split) but no
`soul-draft` test renders the decisive named fixture itself — `group_heavy_plus_one_direct_each_way`
appears nowhere under `crates/soul-draft` (grep, this round). R2-SYNTHESIS already ruled this
non-blocking, and I concur: the venue-split rendering rule is pinned, just on a different edge.

What code could still be written is below the close bar and was explicitly ruled out of scope:

- AC-32 / AC-33 are covered by combinations of tests across layers (Round 1 AC_MAP), not by one
  integrated single-flow test each. Round 1 judged combination coverage adequate; fable-b's Round 2
  scope control confirmed no new AC rows. Post-merge hardening options, not D54 gates.
- The one optional pin R2 left open (mixed-store `UnscoredEdge`) was closed by opus-a this round
  (`0f8a073`), so even the optional list is now empty.

There is no remaining code task on this tree whose completion would move Goal 1 closer to D54.

### Author-manual — the half no CI can substitute

The thirteen slices are defined 「在干净 Windows 11 x64 上必须同时证明」 — the slice gate is a
real-machine gate by construction, so even fully test-covered slices only count as proven when the
author runs a HEAD build there. STATUS:811 records the seam precisely: soul layer and read-only
agent layer have tests; tray appearance and real-machine collection do not; several rows have moved
from "the product has no path" to "the path exists, nobody has pressed it on a real machine".

Concretely the author must, on a clean Win11 x64 non-admin account
(`scripts/author-manual-checklist.md`, results written back to STATUS per D57):

- install/tray/no-UAC (AC-01; slice 1);
- the two-segment real-machine collection measurement (the real half of AC-09/AC-10; slice 7);
- the OS-level no-traffic half of AC-21/AC-22;
- the AC-13 second-confirmation button and the E1 endpoint round trip on a real machine;
- `keys.dpapi` presence and uninstall behaviour (DPAPI, real machine);
- a real install/uninstall from a HEAD-built, author-signed NSIS installer (second half of AC-26,
  D56; slice 13's install-smoke half).

One-line discrepancy worth recording, not fixing: the checklist's opening paragraph says AC-09 /
AC-10 are marked 「作者手动」 in the matrix, but the matrix's 谁跑 column says CI for both. Harmless —
both documents agree the real-machine half belongs to the author — but a future edit could align
the checklist's sentence.

### Minutes — the hosted form of every CI row

Every row whose 谁跑 column says CI has hosted execution as its acceptance form, and hosted has
never run at any close-loop HEAD: Actions minutes are exhausted, and the last known run on the
trunk lineage is the empty run 32796349061 at `a643eaf`. Local evidence is not hosted evidence
(STATUS discipline), and this report claims none. The code side of this item is already done and
was re-probed twice (Round 2 and Round 3 gpt-sol-b): `ci.yml` has no `pull_request` trigger, and
`cursor/goal1-unblock-a073` is in the push list and all five job gates. When minutes return, one
`workflow_dispatch` at HEAD is the entire remaining action. No empty-commit refresh is permitted.

## What this loop closed (code/docs on this tree, with commits)

| Round | Item | Commits | Evidence |
|---|---|---|---|
| 1 | R-1 legacy regression: `correct_tie`/`release_tie` rebuild a pre-wiring edge before locking; `UnscoredEdge` refusal; schema not loosened | `9fd6870` | my Round 2 five-gate review: PASS on all five |
| 1 | CI-TRUNK: trunk added to push list and all five job `if:` gates; no `pull_request` trigger | (landed pre-loop-R1, verified) | re-probed PASS in R2 and R3 |
| 2 | CODE-2 — AC-28/29/30 strict literal: four product-boundary tests importing the named `soul-algo-tie` fixtures, expectations demanded from the frozen crate; both counterfactual mutations (per-peer `as_of`; T4 venue-blind banding) shown red, then reverted | `e489769`, `6f1b9b2` | `t4d_product` 16/16 green at HEAD |
| 2 | CODE-3 — AC-34 second sentence: an owner group message does not refresh historical speakers' `last_contact_utc`; negative control against the roster fan-out proved the assertion live | `9ccadb6` | `import_to_graph` 6/6 green at HEAD |
| 2 | DOC-D61: D61 appended (D49 byte-unchanged), PLAN_INDEX aligned | `a29f792`, `0418983` | gpt-sol-b R3 probe: PASS |
| 3 | Mixed-store `UnscoredEdge` pin (the one optional item I flagged in R2): a refusal on a forgotten peer leaves the neighbouring locked edge byte-identical; non-vacuity shown by mutation | `0f8a073` | `graph_correction` 12/12 green at HEAD, re-run by me |
| 3 | STATUS cites D61 beneath the 各条线 table, scoped so it cannot be read as a closure claim | `341c921` | read this round |
| 3 | Re-verification: 190/190 five-package green + schema-freeze green; lock hygiene, CI wiring, no-second-threshold, D61 presence, PR #10 base all PASS | `5fc5b3a`, `6665d2c` | slot reports in `round3/` |

## What remains — all human/GitHub, none of it a Round 4 candidate

1. **Merge PR #7** (`cursor/goal1-unblock-a073` → `main`) — the unique merge path (D61). This
   environment's `gh` is read-only; no agent here can do it.
2. **Fold this branch back via PR #10** (`cursor/goal1-close-loop-a073` →
   `cursor/goal1-unblock-a073`) — base verified correct by gpt-sol-b this round. Do not open any
   PR from the `-df52`/`-4799` side forks to `main`.
3. **Close PR #4** (`agent/dev-sota`) — D49/D61, BLOCKERS M1.
4. **PR #6** (`BLOCKERS.md`) after PR #7, resolving the fake-D32 numbering collision noted at
   `DECISIONS.md:74`.
5. **Restore Actions minutes**, then `workflow_dispatch` the five gates at HEAD. Only after that
   run may anyone claim AC-26's hosted half or any CI row's hosted form.
6. **Author Win11 checklist + signed NSIS**, results written back to STATUS (D57).

Items 1–4 are GitHub-side; 5 is billing plus one dispatch; 6 is the author's hands. Nothing on
this list can be advanced by another agent round on this tree — a Round 4 would have nothing
legitimate to do, and the round rules (no empty commits, no new features, no hosted claims) would
forbid the only motions left.

## Bottom line

Goal 1 is code-complete and locally green on this tree. It is not closed, and this loop cannot
close it: D54's two gates each keep one foot off-tree — the matrix in hosted CI and the author's
signature, the slices on a clean Windows 11 machine. The loop's honest deliverable is exactly
that sentence, plus the six-item handoff above.
