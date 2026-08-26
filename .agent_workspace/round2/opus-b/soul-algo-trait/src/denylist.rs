//! What this crate is not allowed to say.
//!
//! Three separate screens, because they guard three different promises:
//!
//! 1. [`DIAGNOSTIC_DENYLIST`] — PRODUCT_LOCK「不是临床医学产品，不给出精神疾病
//!    诊断」and D5. No clinical word may appear in anything a user reads. The
//!    ten entries are the list the Round 2 brief froze, verbatim.
//! 2. [`NUMERIC_RATING_MARKERS`] — D22. An axis carries a direction and a band,
//!    never a score, a percentage or a place on a scale. Plain counts ("往来 12
//!    次") are not ratings and stay allowed; that is the whole difference
//!    between "we counted this" and "we graded you".
//! 3. [`PEER_CLAIM_DENYLIST`] — the personnel summary (A2) describes the record
//!    of contact and nothing else. It may not claim friendship, intimacy or a
//!    personality for a third party, who never consented to being profiled.
//!    「强关系」 is on it: a `Strong` band is a statement about how much
//!    evidence there is, and rendering it as a statement about the relationship
//!    is the exact slip A2 exists to avoid.
//!
//! The screens run in tests over every user-facing string this crate can
//! produce, and [`assert_publishable`] is available to callers that want the
//! same check at a write boundary.

/// Words that would turn a working hypothesis into a medical claim.
///
/// `人格障碍` is listed even though `障碍` already catches it, and `量表` is
/// here as well as among the rating markers: the denylist is read by people
/// deciding what to add, not only by the matcher.
pub const DIAGNOSTIC_DENYLIST: [&str; 10] = [
    "抑郁",
    "焦虑",
    "障碍",
    "诊断",
    "人格障碍",
    "病",
    "score",
    "percentile",
    "百分位",
    "量表",
];

/// Markers of a rating, a percentage or a place on a scale.
pub const NUMERIC_RATING_MARKERS: [&str; 10] = [
    "%",
    "％",
    "score",
    "percentile",
    "百分位",
    "评分",
    "打分",
    "满分",
    "分位",
    "量表",
];

/// Claims A2 may not make about another person.
pub const PEER_CLAIM_DENYLIST: [&str; 14] = [
    "朋友",
    "好友",
    "闺蜜",
    "知己",
    "亲密",
    "关系好",
    "强关系",
    "弱关系",
    "感情",
    "性格",
    "人格",
    "内向",
    "外向",
    "信任",
];

/// A string that failed one of the screens, and which word failed it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForbiddenWord {
    /// Which screen rejected the string.
    pub screen: Screen,
    /// The word that tripped it.
    pub word: &'static str,
}

/// Which promise a rejected string would have broken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    /// A clinical word.
    Diagnostic,
    /// A score, a percentage or a place on a scale.
    NumericRating,
    /// A claim about a third party this product may not make.
    PeerClaim,
}

/// The first clinical word in `text`, if any.
///
/// ASCII words match case-insensitively so that `Score` and `SCORE` cannot
/// slip through; the Chinese words have no case to fold.
pub fn diagnostic_hit(text: &str) -> Option<ForbiddenWord> {
    first_hit(text, &DIAGNOSTIC_DENYLIST, Screen::Diagnostic)
}

/// The first rating marker in `text`, if any.
pub fn numeric_rating_hit(text: &str) -> Option<ForbiddenWord> {
    first_hit(text, &NUMERIC_RATING_MARKERS, Screen::NumericRating)
}

/// The first forbidden third-party claim in `text`, if any.
pub fn peer_claim_hit(text: &str) -> Option<ForbiddenWord> {
    first_hit(text, &PEER_CLAIM_DENYLIST, Screen::PeerClaim)
}

/// Both screens every user-facing string in this crate must pass.
pub fn assert_publishable(text: &str) -> Result<(), ForbiddenWord> {
    match diagnostic_hit(text).or_else(|| numeric_rating_hit(text)) {
        Some(hit) => Err(hit),
        None => Ok(()),
    }
}

/// [`assert_publishable`] plus the third-party screen, for A2 output.
pub fn assert_publishable_about_peer(text: &str) -> Result<(), ForbiddenWord> {
    assert_publishable(text)?;
    match peer_claim_hit(text) {
        Some(hit) => Err(hit),
        None => Ok(()),
    }
}

fn first_hit(text: &str, words: &[&'static str], screen: Screen) -> Option<ForbiddenWord> {
    let folded = text.to_lowercase();
    words
        .iter()
        .find(|word| folded.contains(&word.to_lowercase()))
        .map(|word| ForbiddenWord { screen, word })
}
