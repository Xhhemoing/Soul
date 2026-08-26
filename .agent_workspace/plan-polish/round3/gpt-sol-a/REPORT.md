# Round 3 acceptance report

Model: `gpt-5.6-sol-xhigh-fast`  
Binding: `R2-SYNTHESIS.md`  
Result: **PASS**

| Gate | Result | Evidence |
|---|---|---|
| Schema lock SHA-256 | PASS | Lock file set equals all 11 `*.schema.json` files; every digest matches |
| JSON Schema cases | PASS | 3 expected accepts and 3 expected rejects, including `dependentRequired` |
| AC-28–AC-34 | PASS | Each acceptance-matrix row ID occurs exactly once in FORMAL |
| D32–D59 | PASS | All 28 decision-table row IDs occur exactly once |
| Standalone `180` / `360` leakage | PASS | Zero matches across the seven named core plan files |
| `cargo test --workspace --offline` | PASS | Exit 0; 249 passed, 0 failed, 0 ignored |

No acceptance blocker found. No `docs/` file was edited. Detailed evidence is in `TEST_LOG.txt`; machine-readable results are in `GATES.json`.
