# Q2 Performance Decomposition Design

## Purpose

Q2-03 established descriptive end-to-end timings for synthetic imports, but its
commit_including_rebuild stage cannot distinguish parsing, encrypted writes,
graph rebuild work, and transaction overhead. This change adds diagnostic
measurement only. It does not optimize the product or create a performance
gate.

## Scope

The implementation changes only crates/soulcore/tests/q2_scale.rs. It uses the
existing public parser, command-layer import and graph functions, the real
SqlCipherStore transaction, and independent temporary databases. It adds no
production instrumentation, dependency, schema, IPC, index, thread pool,
algorithm, UI, or acceptance criterion.

## Measurement Paths

The existing Session observation remains the end-to-end context and continues
to report commit_including_rebuild. A second observation uses a fresh database
and separates:

1. parse: read_soul_import_v1 produces a StagedImport without writing.
2. encrypted_import_write: import_commands::commit writes contacts, encrypted
   bodies, events, interaction evidence, and its audit entries inside the real
   transaction.
3. graph_rebuild: graph_commands::rebuild derives graph rows and appends its
   audit entries in the same transaction.
4. transaction_total: wall time around SqlCipherStore::transact, including
   BEGIN IMMEDIATE, both measured operations, and COMMIT.
5. transaction_overhead_residual: transaction_total minus encrypted import
   write and graph rebuild durations. This is a residual that also includes
   timer and closure gaps; it must not be labelled commit or fsync time.

The direct path intentionally excludes Session::sync_identifiers because that
method is private and runs after the transaction. The report therefore keeps
the Session timing and direct decomposition separate and never subtracts one
path from the other.

## Cases and Sampling

The ignored explicit measurement keeps the existing four cases:

- 1,000 messages and 10 peers
- 1,000 messages and 100 peers
- 10,000 messages and 10 peers
- 10,000 messages and 100 peers

Each case uses one discarded warmup and five formal trials. Every Session and
decomposition observation gets its own temporary SQLCipher database. The
report keeps nanoseconds, fixture SHA-256, source SHA, machine and toolchain
metadata, counts, one summary record per case, and a final completeness record.
There is no elapsed-time pass/fail threshold and no p95 claim.

## Correctness Contract

The ordinary non-ignored test exercises the real parser, store transaction,
encrypted writes, graph rebuild, and audit append path at 100 messages and 10
peers. It checks literal stage names, event and evidence counts, contact count,
edge count, and transaction arithmetic. A realistic regression such as
skipping the parser, import command, graph rebuild, or transaction must fail
this test.

The existing Session assertions continue to protect message, peer, privacy,
research, graph, and audit invariants. The explicit large measurement reuses
both real paths and serializes their independent counts.

## Report Contract

The run record advances to schema soul-q2-scale-v2. Each sample keeps the
existing durations_ns and counts fields for the Session path and adds
decomposition_durations_ns and decomposition_counts. Each case summary keeps
summary_ns and adds decomposition_summary_ns. The complete record still proves
four cases, four warmups, and twenty formal trials.

## Limitations

The work runs on one Windows host under the test profile. OS page cache,
background scheduling, filesystem state, and thermal state remain uncontrolled.
The profile and memories remain empty. The direct path is diagnostic and is not
the product IPC or UI path. Results may identify a dominant stage, but they do
not prove an optimization, hardware floor, production p95, Linux behavior, or
real-user performance.

## Acceptance

- The pre-change targeted q2_scale test passes.
- The new ordinary decomposition test is observed failing before its helper is
  implemented, then passes with real SQLCipher work.
- Formatting and the targeted q2_scale test pass.
- The measurement code is committed before timed execution and refuses a dirty
  worktree or mismatched source SHA.
- The JSONL validator confirms the run record, four summaries, four warmups,
  twenty formal samples, required stage keys, literal counts, and final
  completeness record.
- Specification review and code-quality review have no unresolved findings.

