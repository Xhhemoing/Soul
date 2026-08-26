# ROUND 2 · Coverage probe

Probe model: `gpt-5.6-sol-xhigh-fast`.

Baseline: `fa6197ee915f5a4bdcf35c97f4e1a24d8c433f47` on the isolated
`cursor/bead-r2-cov-c441` worktree. This is a test-inventory and gap report only;
it does not change product code.

## 0. Result

- `pnpm --filter @bead/app test`: **22 files, 258 tests, all passed**.
- `cargo test` in `crates/bead-core`: **153 tests, all passed**:
  30 library unit tests + 122 integration tests + 1 doctest.
- Total executable behavior locks inspected: **411**.
- `apps/bead` has no coverage provider, coverage thresholds, or
  statement/branch report configured. The numbers above are discovered test
  cases, not a line-coverage percentage.
- T-ASM-1…19 and T-INV-1…15 are specifications, not locks on this baseline.
  There is no assemble-session test file and no inventory CRUD/shortage test
  file. The existing assemble coverage is shell/guard/backdrop only; the
  existing inventory coverage is one stub empty-state assertion plus repository
  round-trip behavior.

## 1. What is actually locked today

### 1.1 `apps/bead`: 258 Vitest cases

Counts below use Vitest's expanded runtime cases. Parameterized declarations are
shown with their concrete case set.

| File | Count | Test names / concrete cases |
|---|---:|---|
| `src/algo/bom.test.ts` | 7 | `(色号, 显示名, 颗数) 精确`; `Σ颗数 == 非空格总数`; `0×0 与全透明都给出空 BOM`; `单色网格恰 1 行`; `颗数降序，同颗数按色板索引升序`; `越界读取返回空格而不是抛错`; `单元数与尺寸不符时拒绝构造` |
| `src/algo/classify.test.ts` | 15 | `24×24 / 12 色 sprite 放大 8× → PixelArt 且置信度 ≥ 0.8`; `特征值可解释：12 个独特色、格内完全一致`; `256×256 平滑双向渐变 → Photo`; confidence bounded for `1×1` / `单色` / `噪声` / `全透明`; pinned transparent/single-colour/1×1 verdicts; `detectGrid` cases for aligned 8×, 3px phase, continuously changing input, uniform input, and alpha boundary |
| `src/algo/color.test.ts` | 38 | CIEDE2000 `第 1 组`…`第 20 组` plus `第 34 组`; named branch-set presence; symmetry for rows 9 and 13; self-distance for four Lab values; sRGB→Lab anchors for black, white, RGB primaries, and 128 grey; neutral-axis residue; all 256 code-value transfer round-trips; half-up rounding; input/output clamping |
| `src/algo/constraints.test.ts` | 5 | `算法树里不出现 score / 得分 / 分数 / 评分`; `判定器确实以 confidence 命名`; `算法树里没有 http/https 字面量`; `fixture 里没有 $schema 键`; `算法只落在 src/algo，没有 packages/bead-algo` |
| `src/algo/decode.test.ts` | 4 | `请求 colorSpaceConversion: none 与 premultiplyAlpha: none`; `以自然尺寸 1:1 绘制并关掉平滑，解码阶段不重采样`; `解码完释放 bitmap`; `零尺寸解码结果报 InvalidDimensions 而不是继续跑` |
| `src/algo/dither.test.ts` | 18 | direct-path equivalence; exact-swatch equivalence; 2×2 checkerboard; frozen 1×8 ramp and right-only propagation; bounds for 1×1 / 1×9 / 9×1 / 2×3; output closure; 64×64 mean preservation; transparent-hole isolation, alpha 127/128, all-transparent output; half-code rounding discriminator and selected result; runner-up margin and duplicate-colour zero-margin sentinel |
| `src/algo/framing.test.ts` | 30 | 56→28 and 112→56 block identities; aspect targets `100×50→28×14`, `50×100→14×28`, `29×29→28×28`, `3000×1→28×1`, `1×3000→1×28`, minimum-dimension clamp, configurable/default max side; in-bounds/partial/off-image crop and scale+crop size; two `InvalidScale`, two `InvalidCrop`, one `CropOutOfBounds`, zero image, non-integer target; linear-light black/white average, alpha-weighted colour, transparent black; fixed-board square/cover/nearest/box/transparent/error cases; 8× nearest recovery |
| `src/algo/oracle-parity.test.ts` | 21 | 48-colour palette equality; for each of `pixel-art`, `photo-flat`, `photo-dithered`, `with-transparency`: fixed-board metadata, exact code sequence, full forced-Photo `imageToPattern` sequence, BOM equality, and all four step-plan equalities |
| `src/algo/palette.test.ts` | 9 | 48 unique RGB entries; closest pair ΔE00 > 3; every swatch self-maps; lower-index exact tie; zero runner-up margin on tie; 1,000 seeded RGBs return valid IDs; one-entry palette; empty palette; out-of-range entry |
| `src/algo/parity.test.ts` | 14 | fixture path-set presence; for each of `pixel-art-upscaled-4x`, `photo-no-dither`, `photo-dither`, `transparent-hole`: exact pipeline code/BOM result, exact four-mode step summaries, and near-tie margin; fixture-wide margin floor |
| `src/algo/pipeline.test.ts` | 14 | pixel-art collapse, forced no-dither, forced Photo override; truncated terminal cell, 3px leading phase, and phase+truncated tail; 28/56 board output, dither switch, manual crop; BOM/grid invariant and transparent BOM; `EmptyPalette`, default palette, reproducibility |
| `src/algo/steps.test.ts` | 24 | 100 seeded-grid partition property; empty/all-transparent for four modes; determinism for four modes; accent/bulk ordering; single-colour row order; 56×56 tile order; partial and empty tiles; solid outline/fill; two ring/hole cases; diagonal and vertical connectivity; component ordering; two row cases; invalid tile size |
| `src/algo/substitutes.test.ts` | 9 | Sharma and RGB boundary pairs; exact-threshold exclusion; ΔE ordering; inventory-index tie-break; determinism; empty result; exact-colour candidate; Lab/RGB agreement |
| `src/app/empty-states.test.tsx` | 12 | personal-strip empty CTA/counts/current-project link; tag/favourite filter empty state and favourite result; Workspace three empty explanations and status routing; Inventory stub empty state; unknown pattern/creator/project guards |
| `src/app/instantiate.test.tsx` | 3 | `转入工作台` project creation and route; `加入待拼` and count; favourite and count |
| `src/app/navigation.test.tsx` | 11 | four nav entries; `aria-current`; four-route traversal; root redirect; real pattern and creator details; unknown route; document title; normal AppShell; assemble outside AppShell with exit; Escape exit |
| `src/app/persistence-banner.test.tsx` | 2 | healthy storage has no banner; failed storage shows the standing banner in both shells |
| `src/app/theme-backdrop.test.tsx` | 4 | theme token persistence; backdrop reads project not theme; per-project backdrop leaves theme alone; custom backdrop reaches repository |
| `src/pages/create/CreatePage.test.tsx` | 3 | unimplemented entries name their WP; upload explanation says placeholder; usable gallery entry has no placeholder warning |
| `src/stores/ids.test.ts` | 5 | fixed `gal-` / `proj-` / `cr-` prefixes; fixture IDs; unique minted project IDs; source ID retained on instantiation; bare/wrong prefix rejection |
| `src/stores/repository.test.ts` | 7 | project/favourite/inventory localStorage round-trip; malformed JSON/records are dropped; bad/missing backdrop is dropped; write-failure memory continuity; post-failure read-modify-write continuity; one-shot failure signal; no-localStorage session behavior |
| `src/test/isolation.test.ts` | 3 | no Soul/Tauri references in `src/**/*.ts(x)`; no listed egress call or remote URL in that same source set; planted scanner positives and route-URL negatives |

The layer totals are: algorithms **208**, app/pages **35**, stores **12**,
and isolation **3**.

### 1.2 `crates/bead-core`: 153 Cargo cases

#### Library unit tests: 30

| File | Count | Exact test functions |
|---|---:|---|
| `src/bom.rs` | 2 | `inventory_reports_zero_for_an_unknown_code`; `adding_accumulates` |
| `src/color.rs` | 5 | `hex_round_trips`; `hex_rejects_junk`; `alpha_is_a_threshold_not_a_blend`; `the_transfer_function_round_trips_every_code_value`; `hue_angle_is_zero_at_the_origin` |
| `src/detect.rs` | 4 | `gcd_is_the_usual_one`; `a_divisor_is_capped_but_still_divides`; `one_boundary_is_not_a_period`; `a_checkerboard_has_no_equal_neighbours` |
| `src/fit.rs` | 3 | `a_square_source_on_a_square_board_is_not_cropped`; `a_zero_board_count_is_refused`; `rounding_holds_at_one_cell` |
| `src/grid.rs` | 4 | `rejects_a_length_that_does_not_match`; `reads_in_reading_order`; `neighbours_stop_at_the_edge`; `a_zero_sized_grid_is_legal_and_empty` |
| `src/image.rs` | 3 | `rejects_a_short_byte_buffer`; `rejects_a_zero_dimension`; `sampling_outside_is_transparent` |
| `src/palette.rs` | 3 | `the_fixture_is_well_formed`; `duplicate_codes_are_rejected`; `an_exact_swatch_matches_itself_at_zero` |
| `src/parity.rs` | 2 | `strings_are_escaped`; `every_case_has_a_distinct_name` |
| `src/quantize.rs` | 2 | `the_kernel_sums_to_one`; `an_out_of_range_accumulation_saturates_rather_than_wrapping` |
| `src/steps.rs` | 2 | `every_mode_has_a_stable_slug`; `a_hole_is_background_the_edge_cannot_reach` |

#### Integration tests: 122

| File | Count | Exact test functions |
|---|---:|---|
| `tests/bom.rs` | 19 | `the_bill_is_counted_commonest_first`; `empty_cells_are_not_counted`; `an_empty_pattern_has_an_empty_bill`; `equal_counts_break_the_tie_on_the_palette_index`; `an_unknown_id_is_still_counted`; `a_full_shelf_needs_nothing`; `a_partial_shelf_reports_only_the_gap`; `a_colour_that_was_never_stocked_is_still_a_shortage`; `the_substitute_threshold_is_three`; `the_substitute_boundary_is_strictly_below_three`; `the_close_palette_straddles_the_threshold`; `only_the_colour_below_the_threshold_is_offered`; `substitutes_are_sorted_and_deterministic`; `an_identical_colour_under_another_code_leads_the_list`; `a_colour_the_pattern_already_needs_is_not_offered`; `nothing_close_enough_offers_nothing`; `a_widened_threshold_reaches_the_spread_out_fixture`; `inventory_accumulates_and_reports`; `the_bill_agrees_with_every_step_plan` |
| `tests/ciede2000.rs` | 6 | `matches_the_published_table`; `matches_the_near_black_row_the_review_adds`; `handles_the_hue_seam_cases`; `is_symmetric`; `a_colour_is_zero_from_itself`; `a_grey_pair_reduces_to_scaled_lightness` |
| `tests/detect.rs` | 13 | `the_sprite_never_repeats_a_neighbouring_line`; `an_eight_times_export_is_pixel_art_with_high_confidence`; `other_export_factors_are_read_at_their_own_size`; `a_native_resolution_drawing_is_still_pixel_art`; `a_smooth_gradient_is_a_photograph`; `noise_is_a_photograph`; `confidence_is_always_a_number_between_zero_and_one`; `the_degenerate_inputs_are_pinned`; `the_lattice_of_an_aligned_export_is_found`; `a_three_pixel_phase_shift_is_reported_as_a_phase_shift`; `a_continuous_image_has_no_lattice`; `a_non_square_lattice_is_reported_per_axis`; `flat_neighbour_ratio_spans_its_range` |
| `tests/fit.rs` | 19 | `the_standard_boards_are_the_sizes_the_plan_names`; `one_28_board_from_a_square_photo_keeps_all_of_it`; `multiple_boards_multiply_the_cell_count`; `a_shape_mismatch_is_reported_as_a_centre_crop`; `a_block_uniform_source_survives_the_box_filter_intact`; `aspect_fit_rounds_the_way_the_contract_says`; `aspect_fit_keeps_the_whole_picture`; `a_part_filled_board_still_counts_as_a_board`; `scale_one_reproduces_the_fixed_board_framing_on_a_matching_shape`; `a_crop_inside_the_image_samples_only_the_crop`; `a_crop_over_the_edge_is_transparent_outside_the_image`; `a_crop_entirely_off_the_image_is_a_typed_error`; `the_refusals_are_specific`; `the_box_filter_averages_in_linear_light`; `the_mode_enum_routes_to_the_same_answers`; `nearest_sampling_recovers_an_upscaled_sprite_exactly`; `the_two_samplers_answer_differently_on_fine_detail`; `averaging_ignores_the_colour_behind_transparent_pixels`; `rendering_more_cells_than_pixels_stays_in_bounds` |
| `tests/palette.rs` | 11 | `the_fixture_is_forty_eight_uniquely_coded_colours`; `ids_are_positions_and_codes_resolve_back_to_them`; `stored_lab_matches_the_swatch`; `no_two_fixture_colours_are_within_the_substitute_threshold`; `every_swatch_finds_itself_at_zero`; `known_pixels_map_to_known_beads`; `an_exact_tie_goes_to_the_lower_index`; `any_pixel_maps_to_a_real_palette_index`; `within_returns_candidates_nearest_first`; `an_empty_palette_is_refused`; `nearest_lab_and_nearest_rgb_agree` |
| `tests/parity.rs` | 10 | `every_case_matches_its_committed_fixture`; `the_fixture_set_covers_the_paths_the_review_names`; `the_two_photo_cases_share_an_image`; `the_source_is_plain_rgba_bytes`; `no_committed_fixture_rests_on_a_near_tie`; `the_margins_are_not_marginal`; `the_sentinel_catches_a_tie_that_is_really_there`; `an_empty_cell_is_not_a_near_tie`; `the_fixture_format_has_no_floating_point_in_it`; `the_fixture_alpha_values_are_unambiguous` |
| `tests/pipeline.rs` | 8 | `an_exported_sprite_round_trips_to_its_own_colours`; `the_bill_matches_the_grid_it_came_from`; `a_photograph_gets_averaged_and_dithered_by_default`; `the_defaults_can_be_overridden`; `a_bad_framing_is_reported_rather_than_guessed`; `transparency_survives_the_whole_pipeline`; `the_conversion_is_reproducible`; `the_whole_loop_runs_from_a_picture_to_instructions` |
| `tests/quantize.rs` | 11 | `the_kernel_is_the_published_one`; `dithering_an_exact_swatch_changes_nothing`; `the_two_paths_agree_wherever_the_error_is_zero`; `the_weights_and_scan_order_produce_the_derived_checkerboard`; `a_single_row_ramp_is_frozen`; `the_kernel_never_reaches_out_of_bounds`; `an_out_of_range_accumulation_still_lands_on_a_bead`; `the_mixture_preserves_the_mean`; `an_empty_cell_is_isolated_from_the_error`; `dither_none_is_the_default_and_maps_pixels_independently`; `transparency_survives_the_undithered_path` |
| `tests/srgb_lab.rs` | 7 | `white_is_the_d65_white_point`; `black_is_the_origin`; `the_primaries_land_where_srgb_says`; `mid_grey_is_neutral_and_darker_than_half`; `the_transfer_function_has_a_linear_toe`; `the_primaries_map_to_the_expected_beads`; `the_map_reports_how_far_it_had_to_go` |
| `tests/steps.rs` | 18 | `every_mode_partitions_the_pattern`; `the_partition_holds_on_random_grids`; `an_empty_pattern_produces_no_steps`; `planning_twice_gives_the_same_plan`; `colour_by_colour_starts_with_the_rarest_colour`; `equal_counts_always_break_on_the_palette_index`; `a_single_colour_pattern_is_one_step_in_reading_order`; `a_56_square_pattern_is_four_28_boards`; `partial_boards_are_not_padded`; `rows_run_top_to_bottom_and_skip_the_empty_ones`; `a_solid_block_is_its_border_then_its_middle`; `a_ring_has_an_outer_and_an_inner_edge`; `a_one_bead_ring_is_all_outline`; `regions_are_four_connected`; `regions_are_ordered_by_their_first_cell`; `a_region_spans_the_colours_that_touch`; `placements_are_numbered_by_group`; `every_mode_has_a_stable_slug` |

`src/lib.rs` contributes one passing doctest, bringing Cargo's total from 152
unit/integration tests to **153**.

## 2. Highest-value missing tests outside T-ASM / T-INV

These are ordered by failure impact. None duplicates a T-ASM or T-INV row.

### COV-1 · Real reload and delayed-hydration seam

Current evidence stops on either side of the seam:

- `repository.test.ts` round-trips arrays directly.
- UI tests use `createInMemoryRepository` and observe same-mount state.
- No test performs a successful UI mutation, unmounts, creates a fresh
  repository/app over the same `Storage`, and verifies restoration.
- No test delays all three load promises and proves the initial empty reducer
  state is never written over stored data before hydration completes.

Add an integration pair:

1. Create a project, favourite a pattern, and change that project's backdrop;
   wait for persistence, unmount, construct a fresh app/repository over the same
   `Storage`, and assert project/favourite/backdrop restoration. Do the same
   fresh-provider check for the theme's separate storage key.
2. Use a deferred repository; before resolving loads, assert zero save calls;
   resolve stored records and assert they render and are not replaced by
   `EMPTY_STATE`.

This is the main local-first durability boundary today. It does not require or
recommend IndexedDB.

### COV-2 · One Rust-authored detector parity corpus

The shared oracle fixtures are strong but intentionally bypass detection, and
`oracle-parity.test.ts` forces the TS full pipeline down the Photo path. Rust
and TS separately test aligned and phase-shifted lattices, but they do not
consume the same raw bytes for `detect_grid` / `detectGrid`.

Add raw-RGBA detector fixtures authored/emitted by `bead-core` for:

- aligned integer upscale;
- 3px leading phase;
- truncated final cell;
- leading phase plus truncated final cell;
- non-square cell geometry.

Rust should byte-lock the emitted expected geometry; TS should consume it
unchanged and match the oracle. This follows BD18: Rust remains the oracle. It
does **not** propose changing Rust to copy TS behavior. Keep framing modes whose
public parameter models differ out of this corpus until a shared input contract
exists.

### COV-3 · Complete the localStorage trust-boundary table

`parsePersistedState` is tested for malformed JSON, bad ID, and bad/missing
backdrop, but project acceptance is not locked for:

- `status` outside `draft | todo | active | done`;
- missing/non-finite `createdAt`;
- `sourcePatternId` that is neither `null` nor a valid `gal-` ID;
- non-string title/backdrop colour in mixed valid+invalid arrays.

Add one table-driven parser test that retains valid neighbours while dropping
each malformed record, followed by a render test proving a hand-edited blob
cannot strand an invalid project in an unreachable state. Inventory validation
is deliberately omitted here because T-INV-14 already owns it.

### COV-4 · Expand the no-egress and third-party boundary beyond `src/**/*.ts(x)`

The current scanner misses package-owned configuration and manifests, and its
`fetch` pattern deliberately fails to match property calls such as
`window.fetch(...)` and `globalThis.fetch(...)`.

Add tests which:

- scan package TS/TSX/JS/JSON/HTML/CSS configuration and entry files, not only
  `src`;
- prove planted `window.fetch`, `globalThis.fetch`, `EventSource`, remote
  Worker/module URLs, and existing egress forms are rejected;
- lock the direct runtime dependency allowlist to React, React DOM, and React
  Router unless a reviewed change updates the test;
- lock `bead-core` to its current empty dependency/build-dependency surface.

This is more useful than asserting transitive lockfile versions in a unit test:
it protects the local-only design and catches a new network-capable integration
at the review boundary.

### COV-5 · Decode failure cleanup and one real-browser smoke case

The jsdom suite covers successful cleanup and zero dimensions, but not a null
2D context or exceptions from `drawImage` / `getImageData`. Add unit cases that
assert the typed failure and exactly-once `bitmap.close()` on every
post-allocation failure.

At the first real upload surface, add one browser test with tiny committed PNG
and JPEG fixtures to verify natural dimensions and deterministic RGBA handoff.
The current mocked canvas cannot establish that a browser honors the requested
decode options. This browser case should arrive with B03's upload UI, not as a
new third-party decoder.

### Lower-value gaps intentionally not promoted

- A WeakMap cache-hit assertion would lock an implementation detail, not user
  behavior.
- A CI wall-clock assertion for the 512×512 soft budget would be noisy without
  the real upload/Worker surface.
- Snapshotting every Rust public type would create churn without a released
  compatibility target; the behavioral API locks above are currently stronger.

## 3. LOOP20 faces still uncovered

| Face | Surface on this v0 tip | Current lock | Coverage conclusion |
|---|---|---|---|
| Backend | No service, IPC layer, or remote backend by BD7. `bead-core` is an in-process oracle and the store is local. | Algorithm and repository behavior only. | **NO_HIGH_VALUE** — adding a backend test would require inventing a backend. |
| API | Real surfaces exist: Rust public functions, TS algorithm/store contracts, and the shared parity JSON. `.beadproj` is not implemented yet. | Extensive per-side behavior plus fixed-board cross-language palette/code/BOM/steps parity. | Still uncovered: same-byte detector parity (COV-2), successful hydration/reload API seam (COV-1), and future `.beadproj` schema round-trip when B07 creates that surface. |
| Auth | No account, OAuth, role, token, or permission surface by BD7; single-machine profile only. | No auth tests. | **NO_HIGH_VALUE** — correct for v0. No fake auth harness should be introduced. |
| Cache | No product cache layer. Palette preparation's WeakMap is internal memoization; conversion results are not a cache service. | Output determinism is locked; cache hits are not. | **NO_HIGH_VALUE** — cache instrumentation would test an implementation detail. |
| Third-party | Browser platform decode plus React runtime dependencies; `bead-core` currently has zero dependencies. No external vision API or network client. | Source egress scan and algorithm URL constraints; no manifest/direct-dependency gate and no real-browser decode smoke. | COV-4 and COV-5 are the remaining high-value locks. Do not add an image crate or remote vision service to manufacture coverage. |

## 4. Guardrails carried forward

- BD18 remains authoritative: Rust `bead-core` is the oracle. Proposed parity
  additions are Rust-authored fixtures consumed by TS, never a recommendation
  to rewrite Rust to match TS.
- BD19 remains authoritative: keep localStorage for the current project/favourite/
  inventory shell and future step cursor. No IndexedDB recommendation is made
  before a `Grid` is actually persisted.
- T-ASM-1…19 remain B04 implementer work; T-INV-1…15 remain B05 implementer
  work. Their absence explains the current shell-only assemble/inventory
  coverage, but they are not duplicated in this report's gap list.

## 5. Commands recorded

```text
$ pnpm --filter @bead/app test
Test Files  22 passed (22)
Tests       258 passed (258)
```

```text
$ cargo test
30 library unit + 122 integration + 1 doctest = 153 passed
```
