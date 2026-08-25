claude-opus-5-thinking-high-fast

# Round 1 — opus-b — CI-TRUNK

Model slug: requested `claude-opus-5-thinking-high-fast`, ran `claude-opus-5-thinking-high-fast`. No downgrade.
Caveat on the evidence for that line: `cursor-cloud-run-info` for this run
(`bc-25cf65f7-9a30-5eb0-84bc-63d0976ca6ae`) returns `originalModelName: null`, so the slug above is
the dispatched/self-reported one, not an API echo. I am Claude Opus 5 in thinking mode; if the
parent needs an API-side confirmation it has to come from the dispatch record, not from this pod.

Branch: `cursor/goal1-close-loop-a073` (already checked out; did not leave it).
Base for this work: `10da234`.

## What changed

Two files, 13 insertions, 2 deletions.

### `.github/workflows/ci.yml`

1. `on.push.branches` — added `cursor/goal1-unblock-a073` between `main` and
   `cursor/soul-goal1-7b1c`. Kept `main`. Kept `cursor/soul-goal1-7b1c`.
2. All five `if:` conditions (`lint`, `test-linux`, `test-windows`, `sbom`, `package`) — added
   `|| github.ref == 'refs/heads/cursor/goal1-unblock-a073'`. This is required, not cosmetic: the
   `if:` gate is an allowlist of refs, so adding the branch to `on.push` alone would have started
   the workflow and then skipped all five jobs, i.e. a push to the Goal 1 trunk would still have
   produced zero executed jobs.
3. Header comment — three sentences added saying the trunk is now `cursor/goal1-unblock-a073`
   (PR #7) and why `cursor/soul-goal1-7b1c` was kept.

### `docs/STATUS.md`

One sentence appended inside item 2 of 「下一步」 (line 828), immediately after the existing
「workflow 已收窄：只自动 `push` 本分支与 `main`…」 sentence. It states the three auto-push branches,
notes the five `if:` gates were updated in step, and explicitly says this is only the trigger
surface and hosted has not run, let alone gone green. The WP09 paragraph was not touched. Nothing
else in STATUS.md was rewritten.

## Why KEEP `cursor/soul-goal1-7b1c` rather than drop it

Kept, as the brief preferred. Three reasons, in order of weight:

1. It is the ancestor of the current trunk and the head of PR #2, which is still open. Dropping the
   trigger would mean a push to that branch produces no checks at all and no failure either —
   exactly the silent-no-run failure mode the header comment already warns about for the
   `pull_request`-only design.
2. It costs nothing while it is idle. Trigger entries only spend Actions minutes when something is
   actually pushed to that ref, and nobody is pushing to the historical branch. The minutes problem
   is caused by retriggers on active branches, which this does not add.
3. Removing it would be a behaviour change to a branch this slot does not own, and it is not needed
   to make the trunk work. `main` + trunk + historical is additive.

It is not a merge path, and both the workflow comment and STATUS.md already say so (STATUS line 9
and the branch table at line 819). Keeping a CI trigger on it does not promote it.

## Constraints honoured

- No `pull_request` trigger added. `on:` is still exactly `push` + `workflow_dispatch`.
- No empty commit.
- No job `steps:` touched — the diff contains no line inside any `steps:` block. Verified by reading
  `git diff`: every hunk is either the `on:` comment/branches block or an `if:` condition.
- `workflow_dispatch` left as-is, still runnable from any ref.
- No Actions minutes spent. This is a YAML + one-sentence-docs change; nothing was pushed to a
  branch that has an auto-push trigger, and no workflow was dispatched.
- Goal 2 not started. Soul-graph not touched. No other slot's files written.

## Verification (local, no hosted run)

`python3 -c "yaml.safe_load(...)"` on `.github/workflows/ci.yml`:

- triggers parse as `['push', 'workflow_dispatch']` — no `pull_request`;
- `push` parses as
  `{"branches": ["main", "cursor/goal1-unblock-a073", "cursor/soul-goal1-7b1c"], "paths-ignore": ["docs/**", "*.md", "scripts/*.md"]}`;
- all five jobs' `if:` expressions each contain the `main`, `goal1-unblock-a073` and
  `soul-goal1-7b1c` refs plus the `workflow_dispatch` arm.

A ripgrep for the two branch names finds 3 occurrences in `on.push.branches` and 2 per job across
five jobs, with no remaining place that names `cursor/soul-goal1-7b1c` without also naming
`cursor/goal1-unblock-a073`.

This is static validation only. Whether the workflow actually runs green on hosted is unknown and
unchanged by this commit — Actions minutes are still exhausted, and the last known hosted run is
still the empty-runner one recorded in STATUS.

## Not done here (out of this slot's ownership)

- `docs/DECISIONS.md` D49 still names `cursor/soul-goal1-7b1c` as the unique trunk (brief item
  DOC-D49). Not mine to append.
- `paths-ignore` still ignores `docs/**`, so this very commit would not trigger a run even on the
  trunk. Left alone deliberately: narrowing it back would spend minutes on docs pushes, which the
  header comment argues against.
- Note for whoever reads the STATUS sentence later: 「本分支」 in the pre-existing sentence was
  written when the trunk was the old branch. I did not rewrite it; the sentence I added names all
  three branches explicitly, so the ambiguity is resolved by addition rather than by editing
  someone else's text.
