#![forbid(unsafe_code)]

use soul_algo_tie::{Band as CanonicalBand, Interaction, TieAlgo};
use t4d_shadow::{fixtures, score, Band};

fn canonical_band(band: CanonicalBand) -> Band {
    match band {
        CanonicalBand::Weak => Band::Weak,
        CanonicalBand::Moderate => Band::Moderate,
        CanonicalBand::Strong => Band::Strong,
    }
}

fn main() {
    let mut disagreements = Vec::new();

    println!("MODEL_SLUG: gpt-5.6-sol-xhigh-fast");
    println!();
    println!("# Round X independent T4D cross-check");
    println!();
    println!(
        "| Fixture | Expected | Shadow | soul_algo_tie | Direct | Group | Direct days | Silent days | Result |"
    );
    println!("|---|---|---|---|---:|---:|---:|---:|---|");

    for fixture in fixtures::all() {
        let canonical_events: Vec<_> = fixture
            .events
            .iter()
            .enumerate()
            .map(|(index, event)| Interaction {
                peer_id: 1,
                outgoing: event.outgoing,
                occurred_at_unix: event.occurred_at_unix,
                venue_direct: event.direct,
                conversation_id: if event.direct { 1 } else { 2 + index as u64 },
            })
            .collect();
        let shadow = score(&fixture.events, fixture.as_of);
        let canonical = TieAlgo::T4D.score(1, &canonical_events, fixture.as_of);
        let canonical_result = canonical_band(canonical.band);
        let agrees = shadow.band == fixture.expected
            && canonical_result == fixture.expected
            && shadow.band == canonical_result
            && shadow.direct_count == canonical.direct_count()
            && shadow.group_count == canonical.group_count()
            && shadow.direct_active_days == canonical.direct_active_day_count
            && shadow.silent_days == canonical.silent_days;
        let result = if agrees { "AGREE" } else { "DISAGREE" };

        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            fixture.id,
            fixture.expected.as_str(),
            shadow.band.as_str(),
            canonical_result.as_str(),
            shadow.direct_count,
            shadow.group_count,
            shadow.direct_active_days,
            shadow.silent_days,
            result,
        );

        if !agrees {
            disagreements.push(format!(
                "| {} | expected {}; shadow {:?} ({}/{}/{}/{}); canonical {:?} ({}/{}/{}/{}) |",
                fixture.id,
                fixture.expected.as_str(),
                shadow.band,
                shadow.direct_count,
                shadow.group_count,
                shadow.direct_active_days,
                shadow.silent_days,
                canonical.band,
                canonical.direct_count(),
                canonical.group_count(),
                canonical.direct_active_day_count,
                canonical.silent_days,
            ));
        }
    }

    println!();
    println!("## DISAGREE rows");
    println!();
    if disagreements.is_empty() {
        println!("None.");
        println!();
        println!("VERDICT CONFIRM_FREEZE");
    } else {
        println!("| Fixture | Difference |");
        println!("|---|---|");
        for row in &disagreements {
            println!("{row}");
        }
        println!();
        println!("VERDICT REOPEN");
        std::process::exit(1);
    }
}
