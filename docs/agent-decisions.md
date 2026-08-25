# Agent decisions (this parent run)

Additive log for `cursor/soul-integration-4a8e`. Does **not** replace `docs/DECISIONS.md` (D1–D60). New product-direction boards still go there.

| ID | Decision | Why |
|---|---|---|
| AD-1 | Parent exclusive branch is `cursor/soul-integration-4a8e`, created from unique Goal 1 trunk `origin/cursor/soul-goal1-7b1c` @ `5309656`. | D49 unique trunk; user 11.10 exclusive branch; cloud branch prefix `cursor/` + suffix `-4a8e`. |
| AD-2 | Do not treat PR #7 as a replacement trunk. Port T4D/A0/G1+/intake-lock/graph-correction from `cursor/goal1-unblock-a073` @ `6133307` onto this branch after a named plan. | Unique trunk is ahead on product honesty/UI; unblock is ahead on frozen algorithms. Wholesale merge would overwrite one or the other. |
| AD-3 | Do not merge PR #4 / `agent/dev-sota`. | D49. |
| AD-4 | This Goal's success is the author's four capabilities, not "twenty empty polish rounds". Round count still follows user 11.13, but each round must have a real scan or fix. Goal 2 prompt stays closed as a *separate* twenty-round polish program until Goal 1 close (D28/D54). | User allowed plan adaptation from the actual tree. PRODUCT_LOCK v0.2 already names shallow prediction; v0.1 already names research preview. |
| AD-5 | Merge to `main` is `BLOCKED` until M2 (plan freeze + algo crates in the app workspace) is done as a dedicated merge, not as a side effect of this PR. Incremental PRs may target the unique trunk when they are FF-safe. | Unique trunk STATUS: PR #2 is CONFLICTING on docs/schema by design. |
| AD-6 | Hosted CI empty runner is `BLOCKED`, not a code defect. Local `just ci` / crate tests are the verification this agent can actually run. | Account billing/spending; workflow auto-push list does not include this branch. |
| AD-7 | Prediction work, when landed, stays inside PRODUCT_LOCK: local derived/aggregate only, third-party rows = 0, no E0, no clinical claims, no scores/percentiles in user-visible copy. Prefer a thin crate that consumes existing events; do not invent a second graph threshold. | D7, D8, D22, red line 11, research/assistant split. |
| AD-8 | Rebase this exclusive branch onto `origin/cursor/first-test-candidate-c441` @ `85aae68` (strict FF of unique trunk). Do not treat c441 as a second product contract; it is the honesty/audit stack on the same line. | R1 scanner #10: 77 commits already on that tip; porting T4D onto pre-c441 would re-collide. |
| AD-9 | v0.1 "simple prediction" is T4D demotion-clock projection (dates/bands/explanations), not a learned model and not a new WP. Lands after the T4D port. Research preview is already the D8 data-collection preview. | R1 scanner #8; D8/D30; COPY_ZH additive first. |
| AD-10 | Do not add this branch to `ci.yml` auto-push while hosted runners are billing-dead. The workflow comment forbids retrigger waste. | R1 scanner #9 vs workflow header. |
