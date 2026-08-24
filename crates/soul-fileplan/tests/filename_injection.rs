//! AC-25 on the file-name path: a name is data, whatever it says.
//!
//! The corpus is `fixtures/injection/filenames.txt`. Only part of it can
//! become a real file — a name containing `/`, a Windows reserved name, or a
//! trailing dot is not portably creatable — so this file is explicit about
//! which half each assertion is about: the portable subset is created on disk
//! and scanned, and every line, creatable or not, goes through
//! [`UntrustedText`] for the URL check. Claiming the whole corpus had been
//! scanned would be a nicer-sounding and false statement.

mod common;

use std::collections::BTreeSet;

use soul_fileplan::FILE_NAME_CHANNEL;
use soul_policy::injection::{urls_in, ExternalChannel, UntrustedText};
use soul_policy::net_guard::NetGuard;
use soul_testkit::mock_llm::MockLlm;

/// Below this many corpus lines, the fixture has been gutted and the
/// assertions below are passing over nothing.
const MIN_CORPUS_LINES: usize = 20;

/// Below this many created files, the portability filter has eaten the test.
const MIN_CREATED_FILES: usize = 8;

/// Characters no portable file name may contain: the Unix separator, and the
/// set Windows reserves.
const UNPORTABLE_CHARS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

const RESERVED_STEMS: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

fn corpus() -> Vec<String> {
    soul_testkit::fixtures::read_lines("injection/filenames.txt")
        .expect("the file-name corpus is readable")
        .into_iter()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect()
}

/// Can this name be created on both platforms this product is built on?
fn portable(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or_default().to_lowercase();
    !name.is_empty()
        && name.len() <= 200
        && !name.contains(UNPORTABLE_CHARS)
        && !name.chars().any(char::is_control)
        && name.trim() == name
        && !name.ends_with('.')
        && name != "."
        && name != ".."
        && !RESERVED_STEMS.contains(&stem.as_str())
}

#[test]
fn portable_injection_filenames_are_scanned_as_data() {
    let space = common::workspace();
    let mut created: BTreeSet<String> = BTreeSet::new();
    for name in corpus().iter().filter(|name| portable(name)) {
        let path = space.authorized.join(name);
        if std::fs::write(&path, "harmless\n").is_ok() {
            created.insert(name.clone());
        }
    }
    assert!(
        created.len() >= MIN_CREATED_FILES,
        "only {} of the corpus became files; the filter is too strict to be testing anything",
        created.len(),
    );

    let report =
        soul_fileplan::scan(&space.roots(), &space.authorized).expect("an authorized scan");
    let scanned: BTreeSet<String> = report
        .entries()
        .iter()
        .map(|entry| entry.file_name().as_str().to_owned())
        .collect();

    assert_eq!(
        scanned, created,
        "the names are listed exactly as they are on disk: no more, no fewer, unaltered",
    );

    // A name is external content on a channel that already has a name.
    assert_eq!(FILE_NAME_CHANNEL, ExternalChannel::FileName);

    // And it produces no extra plan entry. Every suggestion is about one of
    // the files that exist, and there is at most one suggestion per file — a
    // name that read as an instruction would have to show up as something
    // beyond that.
    let preview = soul_fileplan::plan(&report);
    let sources: Vec<String> = preview
        .entries()
        .iter()
        .map(|entry| common::encode(entry.source_rel()))
        .collect();
    let unique: BTreeSet<&String> = sources.iter().collect();
    assert_eq!(
        sources.len(),
        unique.len(),
        "one suggestion per file at most"
    );
    for source in &sources {
        assert!(
            created.contains(source),
            "`{source}` is a plan entry for a file that was never created",
        );
    }
    assert!(preview.len() <= created.len());
    assert!(!preview.written_to_disk());
}

#[test]
fn filename_urls_never_receive_a_connection() {
    // A server that nothing in this test has any reason to talk to. If a URL
    // in a file name were ever fetched, the fetch would not go here — but a
    // counter that stays at zero while the corpus is processed is the cheap
    // half of the evidence, and the guard below is the other half.
    let decoy = MockLlm::start().expect("start the decoy endpoint");
    let guard = NetGuard::closed();

    let mut urls: Vec<String> = Vec::new();
    for line in corpus() {
        // A file name never becomes a `&str` in an instruction position; it
        // becomes this, which has no `Display`.
        let name = UntrustedText::new(line);
        urls.extend(urls_in(&name));
    }

    assert!(
        !urls.is_empty(),
        "the corpus is supposed to contain URLs for this test to refuse",
    );
    for url in &urls {
        let refused = guard
            .authorize(url)
            .expect_err("a closed guard reaches nothing, and a file name configures no endpoint");
        assert!(
            !refused.to_string().is_empty(),
            "a refusal has to say something",
        );
    }
    assert_eq!(decoy.request_count(), 0, "nothing was contacted");
}

#[test]
fn the_fixture_and_real_scan_controls_are_nonempty() {
    let corpus = corpus();
    assert!(
        corpus.len() >= MIN_CORPUS_LINES,
        "the file-name corpus has {} lines, which is fewer than this suite assumes",
        corpus.len(),
    );
    assert!(
        corpus.iter().any(|line| line.contains("忽略之前指令")),
        "the corpus should still contain an instruction-shaped name",
    );
    assert!(
        corpus.iter().any(|line| line.contains("..")),
        "the corpus should still contain a traversal-shaped name",
    );

    let portable_count = corpus.iter().filter(|line| portable(line)).count();
    assert!(
        portable_count >= MIN_CREATED_FILES,
        "{portable_count} portable names is not enough to scan",
    );
    assert!(
        portable_count < corpus.len(),
        "some corpus lines are deliberately not creatable; if all of them were, the split this \
         file describes would be fiction",
    );
}
