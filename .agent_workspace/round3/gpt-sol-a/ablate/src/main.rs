use std::collections::BTreeMap;
use std::hint::black_box;
use std::time::{Duration, Instant};

use ablate::{
    ablation_fixtures, fixture_scale, score_all_t0, score_all_t4d, score_t4, score_t4d,
    Interaction, TieScore, FIXTURE_AS_OF_RFC3339, FIXTURE_AS_OF_UNIX, MODEL_SLUG,
};

const WARMUP_PASSES: usize = 30;
const CHUNKS: usize = 10;
const PASSES_PER_CHUNK: usize = 50;

type Scorer = fn(&[Interaction], i64) -> BTreeMap<String, TieScore>;

fn warm(scorer: Scorer, interactions: &[Interaction], as_of: i64) {
    for _ in 0..WARMUP_PASSES {
        let scores = scorer(black_box(interactions), black_box(as_of));
        assert_eq!(black_box(scores.len()), 200);
    }
}

fn time_chunk(
    scorer: Scorer,
    interactions: &[Interaction],
    as_of: i64,
    passes: usize,
) -> Duration {
    let started = Instant::now();
    for _ in 0..passes {
        let scores = scorer(black_box(interactions), black_box(as_of));
        assert_eq!(black_box(scores.len()), 200);
    }
    started.elapsed()
}

fn benchmark(interactions: &[Interaction], as_of: i64) -> (Duration, Duration) {
    warm(score_all_t0, interactions, as_of);
    warm(score_all_t4d, interactions, as_of);

    let mut t0_elapsed = Duration::ZERO;
    let mut t4d_elapsed = Duration::ZERO;
    for chunk in 0..CHUNKS {
        if chunk % 2 == 0 {
            t0_elapsed += time_chunk(score_all_t0, interactions, as_of, PASSES_PER_CHUNK);
            t4d_elapsed += time_chunk(score_all_t4d, interactions, as_of, PASSES_PER_CHUNK);
        } else {
            t4d_elapsed += time_chunk(score_all_t4d, interactions, as_of, PASSES_PER_CHUNK);
            t0_elapsed += time_chunk(score_all_t0, interactions, as_of, PASSES_PER_CHUNK);
        }
    }
    (t0_elapsed, t4d_elapsed)
}

fn main() {
    println!("MODEL_SLUG: {MODEL_SLUG}");
    println!("AS_OF: {FIXTURE_AS_OF_RFC3339} ({FIXTURE_AS_OF_UNIX})");
    println!("MATRIX");
    println!(
        "| Fixture | Events | All days | Direct events | Direct days | Last age | T4 | T4D |"
    );
    println!("|---|---:|---:|---:|---:|---:|---|---|");
    for fixture in ablation_fixtures() {
        let t4 = score_t4(&fixture.interactions, fixture.primary_peer, fixture.as_of);
        let t4d = score_t4d(&fixture.interactions, fixture.primary_peer, fixture.as_of);
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            fixture.id,
            fixture.interactions.len(),
            t4.active_day_count,
            t4.direct_count,
            t4.direct_day_count,
            t4.last_contact_age_days.unwrap_or(0),
            t4.band.as_str(),
            t4d.band.as_str(),
        );
    }

    let scale = fixture_scale();
    assert_eq!(scale.interactions.len(), 10_000);
    let (t0_elapsed, t4d_elapsed) = benchmark(&scale.interactions, scale.as_of);
    let passes = CHUNKS * PASSES_PER_CHUNK;
    let inputs = passes as u128 * scale.interactions.len() as u128;
    println!(
        "BENCH F_SCALE interactions={} peers=200 passes={} \
         T0_total_ns={} T0_ns_per_pass={} T0_ns_per_interaction={} \
         T4D_total_ns={} T4D_ns_per_pass={} T4D_ns_per_interaction={} \
         T4D_over_T0={:.3}x",
        scale.interactions.len(),
        passes,
        t0_elapsed.as_nanos(),
        t0_elapsed.as_nanos() / passes as u128,
        t0_elapsed.as_nanos() / inputs,
        t4d_elapsed.as_nanos(),
        t4d_elapsed.as_nanos() / passes as u128,
        t4d_elapsed.as_nanos() / inputs,
        t4d_elapsed.as_secs_f64() / t0_elapsed.as_secs_f64(),
    );
}
