# Q2 Performance Decomposition Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Add a test-only, reproducible decomposition of Q2 synthetic import time into parsing, encrypted import writes, graph rebuild, and transaction residual.

**Architecture:** Keep the existing Session measurement as end-to-end context. Add a second real-SQLCipher observation in q2_scale.rs that parses outside a transaction and times the import and rebuild command functions inside one SqlCipherStore transaction. Serialize both paths in the existing explicit JSONL report without adding thresholds.

**Tech Stack:** Rust tests, soulcore command modules, SqlCipherStore, tempfile, serde_json, PowerShell validation.

## Global Constraints

- Base commit: b40a5770fe18c3366771db8ee83b53ef306d13e5.
- Product source under test remains ebff0c96325e0f297a1c397de6a3a1421fdd369d; this task changes test and receipt documentation only.
- All shell commands use pwsh and begin with ErrorActionPreference set to Stop.
- Repository text reads and writes use UTF-8.
- Do not modify production code, dependencies, schema, IPC, algorithms, indexes, thread pools, UI, or acceptance criteria.
- Do not add elapsed-time thresholds or claim optimization benefit.
- Use E:\Project\Soul-target-q2-perf-20260925 as the dedicated Cargo target directory.

---

### Task 1: Add the failing decomposition contract

**Files:**
- Modify: crates/soulcore/tests/q2_scale.rs

**Interfaces:**
- Consumes: synthetic_jsonl, timed, import_commands, graph_commands, SqlCipherStore, TestKeyProvider.
- Produces: observe_decomposed(text, messages, peers) returning DecomposedObservation with durations_ns and counts.

- [ ] **Step 1: Write the failing ordinary test**

Add a test named a_hundred_messages_decomposition_reports_real_transaction_stages.
It calls observe_decomposed with the existing 100-message, 10-peer fixture and
asserts the literal duration keys parse, encrypted_import_write, graph_rebuild,
transaction_total, and transaction_overhead_residual. Assert events_written is
100, evidence_written is 100, contacts_created is 11, ties_rebuilt is 10, and:

    transaction_total >= encrypted_import_write + graph_rebuild
    transaction_overhead_residual =
        transaction_total - encrypted_import_write - graph_rebuild

- [ ] **Step 2: Run the test and verify RED**

Run:

    cargo test -p soulcore --test q2_scale a_hundred_messages_decomposition_reports_real_transaction_stages --locked -- --exact

Expected: compilation fails because observe_decomposed and
DecomposedObservation do not exist. A typo or unrelated compiler error does not
count as RED.

- [ ] **Step 3: Implement the minimal real-store helper**

In q2_scale.rs:

- import soulcore::commands::{graph as graph_commands, import as import_commands}
- import soul_store::{SqlCipherStore, TestKeyProvider}
- import soulcore::commands::session::SessionRefusal
- define a fixed measurement audit timestamp after the synthetic event window
- define DecomposedObservation with the two report fields
- open a fresh soul.db under a tempfile with a fixed TestKeyProvider seed
- time read_soul_import_v1
- call SqlCipherStore::transact with Result<_, SessionRefusal>
- time import_commands::commit and graph_commands::rebuild inside the closure
- time the whole transact call and compute the saturating residual
- assert and report literal import and graph counts

- [ ] **Step 4: Run the new test and verify GREEN**

Run the exact test from Step 2. Expected: one test passes, zero failures.

- [ ] **Step 5: Run the existing ordinary q2_scale test**

Run:

    cargo test -p soulcore --test q2_scale a_hundred_messages_across_ten_peers_preserve_counts_privacy_and_audit --locked -- --exact

Expected: one test passes, zero failures.

- [ ] **Step 6: Commit Task 1**

Stage only q2_scale.rs and commit:

    git add crates/soulcore/tests/q2_scale.rs
    git commit -m "test(core): decompose import transaction cost"

### Task 2: Extend the explicit JSONL measurement

**Files:**
- Modify: crates/soulcore/tests/q2_scale.rs

**Interfaces:**
- Consumes: observe and observe_decomposed.
- Produces: soul-q2-scale-v2 JSONL with Session and decomposition sample and summary fields.

- [ ] **Step 1: Extend sample generation**

For every warmup and formal trial, run observe and observe_decomposed on
independent fresh databases. Keep durations_ns and counts unchanged and add
decomposition_durations_ns and decomposition_counts.

- [ ] **Step 2: Extend summaries**

Collect decomposition duration maps for formal trials. For each decomposition
stage, emit hand-defined min, median, and max under decomposition_summary_ns.
Change only the run schema string to soul-q2-scale-v2.

- [ ] **Step 3: Format and run both ordinary tests**

Run:

    cargo fmt --all -- --check
    cargo test -p soulcore --test q2_scale --locked -- --skip measure_four_synthetic_scales_with_five_independent_trials

Expected: formatting succeeds and both ordinary tests pass.

- [ ] **Step 4: Commit Task 2**

Stage only q2_scale.rs and commit:

    git add crates/soulcore/tests/q2_scale.rs
    git commit -m "test(core): report decomposed scale timings"

### Task 3: Execute and validate the clean-source measurement

**Files:**
- Create externally: D:\Soul-q2-performance-evidence-20260925\measurement
- Create externally: D:\Soul-q2-performance-evidence-20260925\validation.json

**Interfaces:**
- Consumes: the clean Task 2 commit and ignored measurement test.
- Produces: immutable JSONL evidence and a structural validation receipt.

- [ ] **Step 1: Verify the source identity**

Require an empty git status, record HEAD, and set SOUL_Q2_SOURCE_SHA to that
exact 40-hex value.

- [ ] **Step 2: Run the ignored measurement**

Run the exact ignored test with --ignored --exact --nocapture and the dedicated
target directory. Do not run G-W, installers, GUI, Linux, or manual gates.

- [ ] **Step 3: Copy the new JSONL report**

Identify the one report created by this run under the dedicated target q2
directory and copy it to the external measurement directory with UTF-8-safe
PowerShell file handling.

- [ ] **Step 4: Validate structure and counts**

Parse every JSONL line and prove:

- one soul-q2-scale-v2 run record with matching source SHA
- four cases and four summary records
- four warmup samples and twenty formal samples
- required Session and decomposition duration keys on every sample
- expected events, evidence, contacts, and ties for each case
- residual arithmetic for every sample
- one complete record with cases=4, warmups=4, trials=20

Write validation.json with exit status, hashes, counts, and limitations.

### Task 4: Record results and review

**Files:**
- Create: docs/reviews/2026-09-25-q2-performance-decomposition.md

**Interfaces:**
- Consumes: measurement JSONL, validation.json, source commit, and agent reviews.
- Produces: a bounded repository receipt that does not claim optimization.

- [ ] **Step 1: Write the receipt**

Record commands, source SHA, report and validation hashes, stage medians, host
and profile limitations, direct-path limitation, and the next evidence-based
decision. Do not add thresholds or generalize to real users or other platforms.

- [ ] **Step 2: Run specification review**

An independent reviewer checks the diff against the approved design and plan.
Resolve every missing or extra requirement before continuing.

- [ ] **Step 3: Run code-quality review**

After specification review passes, an independent reviewer checks correctness,
test quality, measurement bias, naming, and report integrity. Resolve every
critical or important finding.

- [ ] **Step 4: Run final verification**

Run formatting, both ordinary tests, JSONL validation, git diff, and git status.
Confirm no production, dependency, schema, IPC, algorithm, index, UI, or AC
file changed.

- [ ] **Step 5: Commit the receipt**

Stage only the review receipt and commit:

    git add docs/reviews/2026-09-25-q2-performance-decomposition.md
    git commit -m "docs(q2): record performance decomposition"

