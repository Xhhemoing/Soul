use std::hint::black_box;
use std::time::{Duration, Instant};

use ablate::{
    ablation_fixtures, fixture_scale, score_all_t0, score_all_t3r, score_t3, score_t3r, score_t4,
    FIXTURE_AS_OF_RFC3339, FIXTURE_AS_OF_UNIX,
};

const WARMUP_PASSES: usize = 20;
const TIMED_PASSES: u128 = 200;

fn time_t0(interactions: &[ablate::Interaction], as_of: i64) -> Duration {
    for _ in 0..WARMUP_PASSES {
        black_box(score_all_t0(black_box(interactions), black_box(as_of)));
    }
    let started = Instant::now();
    for _ in 0..TIMED_PASSES {
        let scores = score_all_t0(black_box(interactions), black_box(as_of));
        assert_eq!(black_box(scores.len()), 200);
    }
    started.elapsed()
}

fn time_t3r(interactions: &[ablate::Interaction], as_of: i64) -> Duration {
    for _ in 0..WARMUP_PASSES {
        black_box(score_all_t3r(black_box(interactions), black_box(as_of)));
    }
    let started = Instant::now();
    for _ in 0..TIMED_PASSES {
        let scores = score_all_t3r(black_box(interactions), black_box(as_of));
        assert_eq!(black_box(scores.len()), 200);
    }
    started.elapsed()
}

fn main() {
    println!(
        "AS_OF {FIXTURE_AS_OF_RFC3339} unix={FIXTURE_AS_OF_UNIX}"
    );
    for fixture in ablation_fixtures() {
        let t3 = score_t3(
            &fixture.interactions,
            fixture.primary_peer,
            fixture.as_of,
        );
        let t3r = score_t3r(
            &fixture.interactions,
            fixture.primary_peer,
            fixture.as_of,
        );
        let t4 = score_t4(
            &fixture.interactions,
            fixture.primary_peer,
            fixture.as_of,
        );
        println!(
            "FIXTURE {} interactions={} T3={} T3R={} T4={} event_milli={} day_milli={}",
            fixture.id,
            fixture.interactions.len(),
            t3.band.as_str(),
            t3r.band.as_str(),
            t4.band.as_str(),
            t3r.event_milli.unwrap_or(0),
            t3r.day_milli.unwrap_or(0),
        );
    }

    let scale = fixture_scale();
    let t0_elapsed = time_t0(&scale.interactions, scale.as_of);
    let t3r_elapsed = time_t3r(&scale.interactions, scale.as_of);
    let inputs = TIMED_PASSES * scale.interactions.len() as u128;
    let t0_per_pass = t0_elapsed.as_nanos() / TIMED_PASSES;
    let t3r_per_pass = t3r_elapsed.as_nanos() / TIMED_PASSES;
    let t0_per_interaction = t0_elapsed.as_nanos() / inputs;
    let t3r_per_interaction = t3r_elapsed.as_nanos() / inputs;
    println!(
        "BENCH F_SCALE interactions={} peers=200 passes={} T0_total_ns={} T0_ns_per_pass={} \
         T0_ns_per_interaction={} T3R_total_ns={} T3R_ns_per_pass={} \
         T3R_ns_per_interaction={} T3R_over_T0={:.3}x",
        scale.interactions.len(),
        TIMED_PASSES,
        t0_elapsed.as_nanos(),
        t0_per_pass,
        t0_per_interaction,
        t3r_elapsed.as_nanos(),
        t3r_per_pass,
        t3r_per_interaction,
        t3r_elapsed.as_secs_f64() / t0_elapsed.as_secs_f64(),
    );
}
