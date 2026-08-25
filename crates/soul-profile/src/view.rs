//! The profile as a person sees it, with every claim traced back to evidence.
//!
//! AC-06 says each inference must have evidence that can be dereferenced. A
//! store that holds an `evidence_id` satisfies the letter of that and none of
//! the point, so building a [`ProfileView`] actually resolves every id and
//! fails with [`crate::error::ProfileError::DanglingEvidence`] if one does not
//! come back. That makes the acceptance test a question about the product
//! rather than about the test's own bookkeeping.
//!
//! The rendered summary carries `soul_policy::clinical::WORKING_HYPOTHESIS_NOTICE`
//! and is checked against the diagnostic denylist before it is returned.

use uuid::Uuid;

use soul_policy::clinical::{assert_non_clinical, WORKING_HYPOTHESIS_NOTICE};
use soul_schema::common::EvidenceBand;
use soul_schema::evidence::SoulEvidence;
use soul_schema::inference::SoulInference;
use soul_schema::profile::{AxisPosition, TraitAxis};
use soul_store_api::types::{InferenceState, StoreError};
use soul_store_api::ProfileStore;

use crate::axes;
use crate::error::{ProfileError, ProfileResult};
use crate::service::{ordered_axes, read_profile};
use crate::voice::VoiceProfile;

/// One inference with its evidence already fetched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferenceView {
    pub inference: SoulInference,
    pub state: InferenceState,
    /// Never empty: an inference with no resolvable evidence is an error, not
    /// an empty list.
    pub evidence: Vec<SoulEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisView {
    pub axis_id: Uuid,
    pub label: String,
    pub position: AxisPosition,
    pub evidence_band: EvidenceBand,
    pub locked_by_user: bool,
    /// What the axis itself cites, resolved.
    pub evidence: Vec<SoulEvidence>,
    /// Every stored inference that targets this axis, including ones the lock
    /// stopped from being applied.
    pub inferences: Vec<InferenceView>,
    /// The axis in words, for the UI and for a draft prompt.
    pub reading: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileView {
    pub profile_id: Uuid,
    pub voice: VoiceProfile,
    pub axes: Vec<AxisView>,
    /// The sentence every inference-bearing surface has to carry.
    pub notice: &'static str,
}

/// Build the view, resolving every reference on the way.
pub fn profile_view<S: ProfileStore>(store: &S, profile_id: Uuid) -> ProfileResult<ProfileView> {
    let profile = read_profile(store, profile_id)?;
    let inferences = store.list_inferences()?;

    let mut views = Vec::new();
    for axis in ordered_axes(&profile) {
        views.push(axis_view(store, &axis, &inferences)?);
    }

    Ok(ProfileView {
        profile_id,
        voice: VoiceProfile::from_value(&profile.voice),
        axes: views,
        notice: WORKING_HYPOTHESIS_NOTICE,
    })
}

fn axis_view<S: ProfileStore>(
    store: &S,
    axis: &TraitAxis,
    inferences: &[SoulInference],
) -> ProfileResult<AxisView> {
    let definition = axes::axis_by_id(axis.axis_id);
    let label = axis
        .label
        .clone()
        .or_else(|| definition.map(|d| d.label.to_owned()))
        .unwrap_or_else(|| axis.axis_id.to_string());

    let reading = match definition {
        Some(definition) => definition.describe(axis.position),
        None => format!("{label}：{}", axes::position_key(axis.position)),
    };
    assert_non_clinical(&reading)?;

    let mut axis_evidence = Vec::new();
    for evidence_id in axis.evidence_ids.iter().flatten() {
        axis_evidence.push(resolve(store, None, *evidence_id)?);
    }

    let mut axis_inferences = Vec::new();
    for inference in inferences.iter().filter(|i| targets(i, axis.axis_id)) {
        let mut evidence = Vec::new();
        for evidence_id in &inference.evidence_ids {
            evidence.push(resolve(store, Some(inference.inference_id), *evidence_id)?);
        }
        axis_inferences.push(InferenceView {
            state: store.inference_state(inference.inference_id)?,
            inference: inference.clone(),
            evidence,
        });
    }

    Ok(AxisView {
        axis_id: axis.axis_id,
        label,
        position: axis.position,
        evidence_band: axis.evidence_band,
        locked_by_user: axis.locked_by_user.unwrap_or(false),
        evidence: axis_evidence,
        inferences: axis_inferences,
        reading,
    })
}

/// Does this inference talk about that axis?
fn targets(inference: &SoulInference, axis_id: Uuid) -> bool {
    inference
        .target
        .get("axis_id")
        .and_then(|value| value.as_str())
        .and_then(|text| text.parse::<Uuid>().ok())
        == Some(axis_id)
}

fn resolve<S: ProfileStore>(
    store: &S,
    inference_id: Option<Uuid>,
    evidence_id: Uuid,
) -> ProfileResult<SoulEvidence> {
    match store.get_evidence(evidence_id) {
        Ok(evidence) => Ok(evidence),
        Err(StoreError::NotFound { .. }) => Err(match inference_id {
            Some(inference_id) => ProfileError::DanglingEvidence {
                inference_id,
                evidence_id,
            },
            // An axis citing missing evidence is the same defect; name the
            // axis's own id so the message points somewhere useful.
            None => ProfileError::DanglingEvidence {
                inference_id: evidence_id,
                evidence_id,
            },
        }),
        Err(error) => Err(error.into()),
    }
}

/// The profile in prose, for the UI and for a draft prompt.
///
/// Checked against the diagnostic denylist before it is returned: this is the
/// surface a template could most easily put a forbidden word on.
pub fn render(view: &ProfileView) -> ProfileResult<String> {
    let mut lines = vec![format!("语气：{}", render_voice(&view.voice))];
    for axis in &view.axes {
        let support = match axis.evidence_band {
            EvidenceBand::None => "暂无证据".to_owned(),
            band => format!(
                "证据{}，{}条",
                band_word(band),
                axis.evidence.len().max(axis.inferences.len()),
            ),
        };
        let lock = match axis.locked_by_user {
            true => "（你已锁定）",
            false => "",
        };
        lines.push(format!("{}｜{support}{lock}", axis.reading));
    }
    lines.push(view.notice.to_owned());

    let rendered = lines.join("\n");
    assert_non_clinical(&rendered)?;
    Ok(rendered)
}

fn band_word(band: EvidenceBand) -> &'static str {
    match band {
        EvidenceBand::Weak => "弱",
        EvidenceBand::Moderate => "中",
        EvidenceBand::Strong => "强",
        EvidenceBand::None => "无",
    }
}

/// The voice in words.
///
/// Public because it is a surface the denylist has to be pointed at: an
/// earlier spelling of the "sparing" case read 少量表情, which contains 量表 —
/// the exact scale vocabulary `docs/DECISIONS.md` D22 rules out. Nothing about
/// the meaning was wrong; the check caught the characters, and it can only keep
/// doing that if every combination is reachable from a test.
pub fn render_voice(voice: &VoiceProfile) -> String {
    use crate::voice::{EmojiUse, VoiceDirectness, VoiceRegister, VoiceWarmth};
    let register = match voice.register {
        VoiceRegister::Casual => "随意",
        VoiceRegister::Plain => "平实",
        VoiceRegister::Formal => "正式",
    };
    let directness = match voice.directness {
        VoiceDirectness::Reserved => "含蓄",
        VoiceDirectness::Balanced => "适中",
        VoiceDirectness::Direct => "直接",
    };
    let warmth = match voice.warmth {
        VoiceWarmth::Cool => "克制",
        VoiceWarmth::Even => "平和",
        VoiceWarmth::Warm => "热络",
    };
    let emoji = match voice.emoji_use {
        EmojiUse::Never => "不用表情",
        EmojiUse::Sparing => "偶尔用表情",
        EmojiUse::Frequent => "经常用表情",
    };
    format!("{register}、{directness}、{warmth}、{emoji}")
}
