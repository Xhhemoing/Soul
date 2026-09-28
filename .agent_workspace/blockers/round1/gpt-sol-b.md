MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Round 1 gpt-sol-b — adversarial anti-false-blocker review

## Verdict rule

- **BLOCK**: must be resolved before the repository can truthfully claim the relevant Goal 1/product behavior.
- **DEFER**: real work or manual evidence, but not a hard blocker for the current autonomous integration step; the narrower claim must remain honest.
- **IGNORE**: rejected, redundant, out of scope, or tombstoned; do not schedule it.

## Actual blockers

| Candidate | Verdict | One-sentence reason | Existing authority |
|---|---|---|---|
| Merge topology | **BLOCK** | PR #2 and PR #4 diverged after `862e858`, independently added conflicting WP10/WP11 implementations, and PR #2's current tip is not an ancestor of PR #4 despite PR #4's stale ancestry claim, so one canonical tree must reconcile both branches and current `main`/PR #5 before any merge-ready claim. | PR #4 body (“stacking relationship”), `FORMAL_WORK_PROMPT.md` §11.5 Git discipline, and the observed Git graph/dirty PR states |
| Windows AC-18 | **BLOCK** | “Unauthorized B is refused 100%” is a target-platform security claim, but the escaping-link test is Unix-only and Windows junction/reparse behavior is not proved, so Linux tests using Windows-looking strings cannot close AC-18 for Windows 11. | `FORMAL_WORK_PROMPT.md` AC-18; `PRODUCT_LOCK.md` platform row and vertical slice item 9; D31 |
| T4D product wiring | **BLOCK** *(when claiming the product uses `ALGO_FROZEN`)* | The Goal 1 `soul-graph/src/build.rs` still computes legacy T0 from all interactions with local 3/10/3 constants and no recency, while the frozen decision explicitly requires replacing `Tally::band()` with T4D and passing a single global `as_of`. | `docs/algorithms/DECISION.md` §6.1–6.5 |

## Soft blockers

| Candidate | Verdict | One-sentence reason | Existing authority |
|---|---|---|---|
| Author-manual AC-01 | **DEFER** | Tray visibility and no-UAC evidence require a person on clean Windows, so they do not block code integration but AC-01 must remain “not yet signed” rather than being reported as passed. | `FORMAL_WORK_PROMPT.md` AC-01; `PRODUCT_LOCK.md` Windows/manual-check assumption; `scripts/author-manual-checklist.md` |
| DPAPI skeleton | **DEFER** | `Unsupported` is acceptable as an honest, fail-closed interim state with UI saying the store is unavailable/unprotected, but it cannot support a claim that the Windows encrypted-store main flow is usable or DPAPI-protected. | `SECURITY.md` “密钥与遗忘/加密落地”; `PRODUCT_LOCK.md` data plane; Goal 1 `STATUS.md` DPAPI disclosure |

## Non-blockers and tombstones

| Candidate | Verdict | One-sentence reason | Existing authority |
|---|---|---|---|
| Reopen frozen product decisions | **IGNORE** | The product lock is the sole authority and already says unmarked choices are decided, so disagreement is not a blocker unless the owner explicitly changes that authority. | `PRODUCT_LOCK.md` status/authority statement; D27/D29/D30 |
| Scope creep / new work packages | **IGNORE** | Post-R2 work packages may only shrink, so adding features cannot be used to postpone Goal 1. | D30; `FORMAL_WORK_PROMPT.md` work-package section |
| Goal 2 | **DEFER** | Goal 2 is a separate polish track that starts only after Goal 1 closes. | D28; `FORMAL_WORK_PROMPT.md` opening and Goal sequence |
| “Innovate” a new demotion gate | **IGNORE** | A new “recent 90-day activity” gate is expressly forbidden as a silent fix; only the documented T4D→T4 or F04c-triggered T3R fallback may change the frozen result. | `docs/algorithms/DECISION.md` §4.2 and §5; `docs/algorithms/REJECTED.md` |
| Merge PR #1 separately | **IGNORE** | PR #1's product-lock commits are already ancestors/content of the Goal 1 lines, so the blocker is reconciling the divergent implementation tips, not separately merging the stacked documentation PR. | PR #2 and PR #4 bodies; observed ancestry of `a785317` |
| Linux desktop | **IGNORE** | Linux is a headless/fake CI host only and no Linux desktop is part of v0.1. | `PRODUCT_LOCK.md` platform row; `GOAL1_PLAN.md` target-runtime lock |
| OAuth | **DEFER** | OAuth was cut from v0.1 and assigned to v0.2, while v0.1 accepts only named file imports. | D3/D17; `PRODUCT_LOCK.md` import row and roadmap |
| WeChat/QQ scrape in Soul | **IGNORE** | Soul must not embed unofficial scraping; any later reader/RPA runs out of process and emits `soul-import-v1`, so in-repo scraping is a tombstone rather than a prerequisite. | D3 and D32; `PRODUCT_LOCK.md` import/non-negotiable rows |
| File-write execution/undo | **DEFER** | Goal 1 deliberately proves only read-only scanning and plan preview; execution/undo is v0.1.1 AC-27. | D10/D19/D31; `FORMAL_WORK_PROMPT.md` AC-27; `PRODUCT_LOCK.md` cut/keep table |
| Clinical scores or diagnostic output | **IGNORE** | The product permits evidence-backed weak/moderate/strong working axes, not clinical diagnoses, scores, scales, or percentiles. | D5/D22; `PRODUCT_LOCK.md` “不是什么” and psychology-model row |
| Add cloud/E0 code | **IGNORE** | v0.1's E0 requirement is absence of the path, so implementing cloud/E0 would violate the gate rather than unblock it; only detected E0 leakage would be a blocker. | D2/D11/D12; `PRODUCT_LOCK.md` E0/E1/L table and roadmap |

## STATUS.md over-claim attack

1. **“阻塞：无” is false at repository scope.** It can only mean “no blocker to continue coding on this branch”; the dirty/divergent PR topology, unproved Windows AC-18, and unwired T4D prevent a merge-ready/product-frozen claim.
2. **“WP05 人脉图完成” is stale after `ALGO_FROZEN`.** Goal 1 still ships T0-style all-venue counting, while `DECISION.md` §6 calls that legacy behavior and makes T4D replacement a post-freeze merge obligation.
3. **“WP11/AC-18 complete” overstates platform evidence.** Windows-shaped strings on Linux test lexical screening, not Windows junction/reparse resolution; the actual escape-link test is `#[cfg(unix)]`.
4. **“WP13 两段都完成” means only the automatable slices are complete.** The same status admits no real `tauri build`, seven manual Windows checks, and an unsupported DPAPI provider, so it must not be paraphrased as Goal 1 or release completion.
5. **PR #2 Goal 1 status and PR #4 P0 status describe different descendants, not one cumulative branch.** PR #4 forked before PR #2's later questionnaire/session/UI/WP13 work, while both branches independently created WP10/WP11; therefore PR #4's statement that current PR #2 is already its ancestor is factually wrong and is the core merge-topology blocker.
6. **Do not invent the opposite over-claim.** `STATUS.md` does not literally say Goal 1 is closed—it lists three remaining items—so the correction is to replace package-level “complete/no blockers” language with acceptance-level states governed by `FORMAL_WORK_PROMPT.md` (D29), not to reopen the frozen scope.

## Bottom line

The hard-blocker set is exactly: reconcile the implementation topology, prove AC-18 on Windows, and wire T4D before claiming `ALGO_FROZEN` is the product behavior; AC-01 and honest-fail-closed DPAPI remain visible soft debt, and every other candidate above must be deferred or ignored rather than allowed to expand Goal 1.
