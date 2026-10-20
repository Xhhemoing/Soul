//! Test-only registration types from ROADMAP 7.2. No prompt/output text is stored.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MissingReason {
    NotStarted,
    ConsentWithdrawnBeforeGeneration,
    #[allow(dead_code)] // Registered missing reason; V2 fills failed brief construction.
    BriefBuildFailed,
    NotGenerated,
    NotRated,
    NotCompleted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Measured<T> {
    Value(T),
    Missing(MissingReason),
    NotApplicable,
}

impl<T> Measured<T> {
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Value(value) => Some(value),
            Self::Missing(_) | Self::NotApplicable => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reported<T> {
    Known(T),
    Unknown,
    NotExposed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParticipantScope {
    Author,
    VolunteerAnonymous,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedbackPurpose {
    ProductEvaluation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsentState {
    Granted,
    Withdrawn,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Variant {
    A,
    B,
    C,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskType {
    T1,
    T2,
    T3,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SemanticStratum {
    NotApplicable,
    Coordination,
    #[allow(dead_code)] // ROADMAP's other predeclared T3 task-card layers.
    BoundaryRefusal,
    #[allow(dead_code)]
    RelationshipFollowup,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Contrast {
    AB,
    AC,
    BC,
}
impl Contrast {
    pub fn variants(self) -> (Variant, Variant) {
        match self {
            Self::AB => (Variant::A, Variant::B),
            Self::AC => (Variant::A, Variant::C),
            Self::BC => (Variant::B, Variant::C),
        }
    }
    pub fn target(self) -> Variant {
        match self {
            Self::AB | Self::BC => Variant::B,
            Self::AC => Variant::C,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Comparability {
    Comparable,
    NotComparable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairCompletion {
    Completed,
    Incomplete,
    Aborted,
    SystemFailure,
    #[allow(dead_code)] // Retain registered safety outcomes, outside P1–P8 arithmetic.
    SafetyStop,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preference {
    Left,
    Right,
    Tie,
    Unrated,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlindStatus {
    Intact,
    Broken,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Path {
    LocalTemplate,
    EndpointPlaceholder,
    EndpointOriginal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RedactionCondition {
    LocalOnly,
    Placeholder,
    SingleUseOriginal,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Conditions {
    pub path: Path,
    pub model_id: Reported<String>,
    pub model_version: Reported<String>,
    pub service_parameters: Reported<String>,
    pub context_limit: Reported<u32>,
    pub redaction_condition: RedactionCondition,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActualBrief {
    Neutral,
    FromView,
    ManualVoice,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BriefBuildStatus {
    Built,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DraftSource {
    ToneTemplate,
    UserEndpoint,
    None,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DegradedReason {
    None,
    #[allow(dead_code)] // Actual reply degradation distinctions are filled by V3.
    ReplyUnreadable,
    ReplyEmpty,
    #[allow(dead_code)]
    ReplyClinical,
    NotGenerated,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateCompletion {
    Completed,
    Incomplete,
    SystemFailure,
    #[allow(dead_code)] // Keep safety termination distinct from a system failure.
    SafetyStop,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FailureCode {
    None,
    NotStarted,
    ConsentWithdrawnBeforeGeneration,
    BriefBuildFailed,
    TransportFailure,
    #[allow(dead_code)] // V3 records actual refusal/response/safety outcomes here.
    ApprovalRejected,
    #[allow(dead_code)]
    ResponseRejected,
    #[allow(dead_code)]
    SafetyStop,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusalCode {
    #[allow(dead_code)] // Refusal metadata is registered but absent in P1–P8.
    Approval,
    #[allow(dead_code)]
    Origin,
    #[allow(dead_code)]
    Safety,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SanitizedNote {
    SyntheticFeedback,
    TemplateSlotFilled,
    FactCorrectedBeforeCompletion,
}

#[derive(Clone, Debug)]
pub struct Run {
    pub evaluation_run: String,
    /// Synthetic monotonic seconds; real adapters must use one time basis.
    pub run_started_at: u64,
    pub run_deadline_at: u64,
    pub participant_scope: ParticipantScope,
    pub feedback_purpose: FeedbackPurpose,
    pub consent_state: ConsentState,
}

impl Run {
    /// Admission for a new operation; never used to invalidate historical rows.
    pub fn can_start(&self, now: u64) -> bool {
        self.consent_state == ConsentState::Granted
            && self.feedback_purpose == FeedbackPurpose::ProductEvaluation
            && now >= self.run_started_at
            && now < self.run_deadline_at
    }
}

#[derive(Clone, Debug)]
pub struct Pair {
    pub evaluation_run: String,
    pub task_slot: u32,
    pub pair_id: String,
    pub task_type: TaskType,
    pub semantic_stratum: SemanticStratum,
    pub contrast: Contrast,
    pub left_variant: Variant,
    pub right_variant: Variant,
    pub presentation_seed: u64,
    pub comparability_status: Comparability,
    pub pair_started_at: Option<u64>,
    pub pair_completed_at: Option<u64>,
    pub pair_completion_status: PairCompletion,
    pub preference: Preference,
    pub blind_status: BlindStatus,
    pub conditions: Conditions,
}

#[derive(Clone, Debug)]
pub struct Candidate {
    pub pair_id: String,
    pub candidate_id: String,
    pub requested_variant: Variant,
    pub actual_brief_state: Measured<ActualBrief>,
    pub brief_build_status: Measured<BriefBuildStatus>,
    pub generated: bool,
    pub draft_source: DraftSource,
    pub degraded_reason: DegradedReason,
    pub candidate_completion_status: CandidateCompletion,
    pub first_pass_accepted: Measured<bool>,
    pub edit_seconds: Measured<u64>,
    pub pause_seconds: Measured<u64>,
    pub help_count: Measured<u32>,
    pub unsupported_claim: Measured<bool>,
    pub required_fact_error_count: Measured<u32>,
    pub privacy_or_safety_stop: Measured<bool>,
    pub failure_code: FailureCode,
    pub placeheld_turns: Measured<u32>,
    pub carries_exempted_original: Measured<bool>,
    pub refusal_code: Measured<RefusalCode>,
    pub missing_reason: Option<MissingReason>,
    pub sanitized_note: Measured<SanitizedNote>,
    pub delivery: bool,
}

impl Candidate {
    /// Allocate before brief construction; adapters fill fields as stages occur.
    pub fn planned(pair_id: &str, candidate_id: &str, variant: Variant, task: TaskType) -> Self {
        let missing = MissingReason::NotStarted;
        Self {
            pair_id: pair_id.into(),
            candidate_id: candidate_id.into(),
            requested_variant: variant,
            actual_brief_state: Measured::Missing(missing),
            brief_build_status: Measured::Missing(missing),
            generated: false,
            draft_source: DraftSource::None,
            degraded_reason: DegradedReason::NotGenerated,
            candidate_completion_status: CandidateCompletion::Incomplete,
            first_pass_accepted: Measured::NotApplicable,
            edit_seconds: Measured::Missing(missing),
            pause_seconds: Measured::Missing(missing),
            help_count: Measured::Missing(missing),
            unsupported_claim: if task == TaskType::T1 {
                Measured::NotApplicable
            } else {
                Measured::Missing(missing)
            },
            required_fact_error_count: if task == TaskType::T3 {
                Measured::Missing(missing)
            } else {
                Measured::NotApplicable
            },
            privacy_or_safety_stop: Measured::Missing(missing),
            failure_code: FailureCode::NotStarted,
            placeheld_turns: Measured::Missing(missing),
            carries_exempted_original: Measured::Missing(missing),
            refusal_code: Measured::NotApplicable,
            missing_reason: Some(missing),
            sanitized_note: Measured::Missing(missing),
            delivery: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredSlot {
    pub evaluation_run: String,
    pub task_slot: u32,
    pub pair_id: String,
    pub task_type: TaskType,
    pub semantic_stratum: SemanticStratum,
    pub contrast: Contrast,
    pub left_variant: Variant,
    pub right_variant: Variant,
    pub presentation_seed: u64,
    pub conditions: Conditions,
}

impl RegisteredSlot {
    /// Capture assignment before execution; it contains no outcome or feedback.
    pub fn from_pair(pair: &Pair) -> Self {
        Self {
            evaluation_run: pair.evaluation_run.clone(),
            task_slot: pair.task_slot,
            pair_id: pair.pair_id.clone(),
            task_type: pair.task_type,
            semantic_stratum: pair.semantic_stratum,
            contrast: pair.contrast,
            left_variant: pair.left_variant,
            right_variant: pair.right_variant,
            presentation_seed: pair.presentation_seed,
            conditions: pair.conditions.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Evaluation {
    pub runs: Vec<Run>,
    pub registered_slots: Vec<RegisteredSlot>,
    pub pairs: Vec<Pair>,
    pub candidates: Vec<Candidate>,
}
