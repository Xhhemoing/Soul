# Round 2 gpt-sol-a probe report

Model: `gpt-5.6-sol-xhigh-fast`

Binding read first: `.agent_workspace/plan-polish/R1-SYNTHESIS.md`.

## Result

One documentation hygiene finding; all structural, hash, and test probes pass.

### Finding: numeric constant leaked into a plan authority document

- `docs/DECISIONS.md:63` (D52) contains literal ``180`` in “禁止再写第三个 `180`”.
- This is still a copied constant literal, despite being used to prohibit another copy. It conflicts with the single-source wording in `docs/PRODUCT_LOCK.md:67` and `docs/FORMAL_WORK_PROMPT.md:45,105`.
- No literal `360` or `MODERATE_MIN` was found in the six requested documents.
- This probe did not modify `docs/`.

### Contextual literal review

- `T0`: two hits, both in `docs/FORMAL_WORK_PROMPT.md:29,72`. Both reject T0 as a Goal 1 closing state; neither presents T0 as current.
- `尚未写应用代码`: one hit at `docs/STATUS.md:24`. It explicitly says not to treat that phrase as a global fact, then states the branch-specific status; it is not a stale current claim.
- `v0.1 实现未开始`: no hits.

## Passed probes

- Schema lock: all 11 schema documents have exact SHA-256 matches; no missing or unlocked schema files.
- Dual source: expected divergence confirmed.
  - `docs/PRODUCT_LOCK.md`: `50bbe63a62fd185857731c10f4eef04b6de14bff3facf4c6d8e9e4ddfbec5ad1`
  - `.agent_workspace/context/plan/PRODUCT_LOCK.md`: `9fa335f00c287e739e37e28d5ac93d077c630196c2ecb1f24fff1bc98cc5e5c7`
- FORMAL: AC-28 through AC-33 each exist exactly once as matrix rows.
- DECISIONS: D32 through D58 all exist. The table contains D1–D58 exactly once each; no duplicate table IDs.
- `cargo test --workspace --offline`: exit 0; 249 passed, 0 failed across unit, integration, and doc tests.

## Artifacts

- `TEST_LOG.txt`: complete probe and Cargo output.
- `probe_docs.py`: read-only hash, literal, AC, and decision-ID probes.
- `run_probes.sh`: reproducible wrapper; directs Cargo build output under this artifact directory.
