# Round 1 baseline inventory

Probe time: 2026-08-25 UTC  
Worktree: `cursor/polish-project-plan-5280` at `c65a0b0243a19fd09e6a55153cdf76bb886f10e9`  
Raw evidence: [`TEST_LOG.txt`](TEST_LOG.txt)  
Reproducer: [`run_probes.sh`](run_probes.sh), [`probe.py`](probe.py)

## 1. `docs/` inventory

| Label | Ref | Commit | Files | Difference from current |
|---|---|---:|---:|---|
| current | `HEAD` | `c65a0b0243a1` | 30 | — |
| product lock | `origin/cursor/soul-product-lock-7b1c` | `a785317aa5a4` | 22 | Current adds exactly the 8 `docs/algorithms/*` files |
| Goal 1 | `origin/cursor/soul-goal1-7b1c` | `3161e02ca16c` | 24 | Goal 1 alone has `docs/GOAL1_PLAN.md` and `docs/schemas/schemas.lock.json`; current alone has the 8 algorithm files |
| main | `origin/main` | `7b35bdeed54f` | 8 | Main has exactly the 8 algorithm files; current adds the 22 product-lock files |

The complete path lists are in `TEST_LOG.txt`. The current branch is the set union of main's 8 algorithm docs and the product-lock branch's 22 docs.

## 2. Dual-source copies

`docs/PRODUCT_LOCK.md` and `.agent_workspace/context/plan/PRODUCT_LOCK.md` are byte-for-byte identical:

`sha256 = 9fa335f00c287e739e37e28d5ac93d077c630196c2ecb1f24fff1bc98cc5e5c7` (9,065 bytes, 152 lines).

Only those two `PRODUCT_LOCK.md` copies exist in the worktree outside `.git`.

| Context copy | `docs/` counterpart | SHA-256 | Exact |
|---|---|---|---|
| `DECISIONS.md` | `docs/DECISIONS.md` | `7f972db0f92148a37088c55fafa0c9fd9bbef6de617ad40eff6367867248f8a4` | Yes |
| `FORMAL_WORK_PROMPT.md` | `docs/FORMAL_WORK_PROMPT.md` | `ca1b114412cc344e3b9d48c2ff61d7922b66d2b59f8a1f21cfc86f7797c20dab` | Yes |
| `PRODUCT_LOCK.md` | `docs/PRODUCT_LOCK.md` | `9fa335f00c287e739e37e28d5ac93d077c630196c2ecb1f24fff1bc98cc5e5c7` | Yes |
| `inference.schema.json` | `docs/schemas/inference.schema.json` | `2140d97e21bc9c0104b5261af7443a29772a4e43e983e00c2c30ab1848949f35` | Yes |
| `profile.schema.json` | `docs/schemas/profile.schema.json` | `fc239712f848491b484517939a49b1cf3ac3754a64801717ea5c82463d247935` | Yes |
| `relationship.schema.json` | `docs/schemas/relationship.schema.json` | `87bfb87e54ea35e3a3647288fbf6cf8fa5260349828f6cb976f61795d0127cf5` | Yes |

The six `.agent_workspace/context/plan/` files are therefore duplicate sources even though none has drifted yet.

## 3. Nine schemas and shared `_defs`

“Shared `_defs`” below means an external `$ref` whose target is `_defs.schema.json`; a local `#/$defs/...` reference does not count.

| Schema | Current branch | Goal 1 |
|---|---:|---:|
| `event` | No; 2 refs to its own embedded `$defs` | Yes (5 refs) |
| `evidence` | No refs | Yes (3 refs) |
| `inference` | No refs | Yes (3 refs) |
| `profile` | No refs | Yes (4 refs) |
| `memory` | No refs | Yes (6 refs) |
| `contact` | No refs | Yes (3 refs) |
| `relationship` | No refs | Yes (4 refs) |
| `audit` | No refs | Yes (7 refs) |
| `export-manifest` | No refs | Yes (2 refs) |
| **Total** | **0/9 use shared `_defs`** | **9/9 use shared `_defs` (37 refs)** |

Thus the current `STATUS.md` WP01 note is accurate: the nine plan schemas have not been connected to the shared `_defs.schema.json`. Goal 1 has already done that and additionally carries `schemas.lock.json`.

## 4. `STATUS.md` falsehood/staleness scan

| Ref | `未开始` | `PLAN_FROZEN` | `ALGO_FROZEN` | `尚未写应用` | Finding |
|---|---:|---:|---:|---:|---|
| current | 1 | 2 | 0 | 1 | Says Goal 1 may begin, “尚未写应用代码”, and v0.1 is “未开始” |
| product lock | 1 | 2 | 0 | 1 | Same matching lines as current |
| Goal 1 | 0 | 2 | 0 | 0 | Says Goal 1 is in progress and WP01–WP11/WP13 are implemented; also explicitly says HEAD hosted CI has not run successfully |
| main | — | — | — | — | `docs/STATUS.md` does not exist |

The current wording is true only if scoped narrowly to this plan branch, which contains no application crates. As project-wide status it is stale: the read-only Goal 1 branch reports substantial implementation. It also omits the `ALGO_FROZEN` state that is present on current/main under `docs/algorithms/`.

## 5. Cargo workspace

`cargo metadata --no-deps --format-version 1` exited 0.

| Workspace member | Version | Manifest | Rust version |
|---|---:|---|---:|
| `soul-algo-tie` | 0.3.0 | `crates/soul-algo-tie/Cargo.toml` | 1.83 |
| `soul-algo-trait` | 0.3.0 | `crates/soul-algo-trait/Cargo.toml` | 1.83 |

Result: the current workspace has exactly two members, both algorithm crates; there are no application crates on this branch.

## 6. JSON parsing

All 11 files matching `docs/schemas/*.json` parse with Python's JSON parser:

| Result | Count |
|---|---:|
| Pass | 11 |
| Fail | 0 |

This probe establishes JSON syntax only; it does not claim JSON Schema semantic or fixture validation.
