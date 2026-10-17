use soul_draft::draft::{DraftRequest, Drafter, BRIEF_TURN_ID};
use soul_draft::{
    EndpointMemoryUse, MemoryContext, MemoryContextEntry, MemoryContextError, MemoryContextPrivacy,
    ProfileBrief,
};
use soul_policy::redactor::{ExemptionRequest, KnownIdentifiers, Redactor};
use uuid::Uuid;

fn entry(
    privacy: MemoryContextPrivacy,
    endpoint_use: EndpointMemoryUse,
) -> Result<MemoryContextEntry, MemoryContextError> {
    MemoryContextEntry::new(
        Uuid::now_v7(),
        Uuid::now_v7(),
        privacy,
        "本地标题秘密",
        "李雷提到的合成记忆正文。",
        endpoint_use,
    )
}

fn drafter() -> Drafter {
    Drafter::new(
        Redactor::new(KnownIdentifiers::new().with_name("李雷")),
        "synthetic-model",
    )
}

#[test]
fn local_only_memory_prose_and_internal_ids_never_enter_the_endpoint_body() {
    let request = DraftRequest::from_paste(ProfileBrief::neutral(), "粘贴原文秘密");
    let context = MemoryContext::new(
        [
            MemoryContextPrivacy::Owner,
            MemoryContextPrivacy::ThirdParty,
            MemoryContextPrivacy::Mixed,
            MemoryContextPrivacy::Unknown,
        ]
        .into_iter()
        .map(|privacy| entry(privacy, EndpointMemoryUse::LocalReferenceOnly).unwrap())
        .collect(),
    )
    .unwrap();
    let body = drafter()
        .redact_with_memory(&request, &context, None)
        .unwrap();
    assert!(!body.as_str().contains("本地标题秘密"));
    assert!(!body.as_str().contains("合成记忆正文"));
    assert!(!body.as_str().contains("粘贴原文秘密"));
    for entry in context.entries() {
        assert!(!body.as_str().contains(&entry.memory_id().to_string()));
    }
    assert!(!format!("{context:?}").contains("记忆正文"));
    assert_eq!(body.third_party_turns(), 3);
    assert_eq!(body.placeheld_turns(), 3);
}

#[test]
fn owner_excerpt_is_explicit_and_cannot_be_enabled_for_mixed_unknown_or_third_party() {
    for privacy in [
        MemoryContextPrivacy::ThirdParty,
        MemoryContextPrivacy::Mixed,
        MemoryContextPrivacy::Unknown,
    ] {
        assert_eq!(
            entry(privacy, EndpointMemoryUse::OwnerExcerpt),
            Err(MemoryContextError::EndpointOwnerRequired)
        );
    }
    let request = DraftRequest::from_paste(ProfileBrief::neutral(), "paste");
    let context = MemoryContext::new(vec![entry(
        MemoryContextPrivacy::Owner,
        EndpointMemoryUse::OwnerExcerpt,
    )
    .unwrap()])
    .unwrap();
    let body = drafter()
        .redact_with_memory(&request, &context, None)
        .unwrap();
    assert!(body.as_str().contains("合成记忆正文"));
    assert!(!body.as_str().contains("李雷"));
    assert!(!body.carries_exempted_original());
}

#[test]
fn paste_exemption_does_not_expand_to_memory_and_is_not_recreated_when_trimming() {
    let request = DraftRequest::from_paste(ProfileBrief::neutral(), "这是一段粘贴原文。");
    let paste_id = request.third_party_turn_ids()[0];
    let context = MemoryContext::new(vec![
        entry(
            MemoryContextPrivacy::ThirdParty,
            EndpointMemoryUse::LocalReferenceOnly,
        )
        .unwrap(),
        entry(MemoryContextPrivacy::Owner, EndpointMemoryUse::OwnerExcerpt).unwrap(),
    ])
    .unwrap();
    let mut material = drafter()
        .redact_memory_context(
            &request,
            &context,
            ExemptionRequest::for_turn(paste_id).confirm(true),
        )
        .unwrap();
    let body = material.body().unwrap();
    assert!(body.as_str().contains("这是一段粘贴原文"));
    assert_eq!(body.exempted_turn(), Some(paste_id));
    assert_eq!(body.placeheld_turns(), 1);
    assert_eq!(material.endpoint_owner_ids().len(), 1);
    assert!(material.trim_last());
    let trimmed = material.body().unwrap();
    assert_eq!(trimmed.exempted_turn(), Some(paste_id));
    assert!(!trimmed.as_str().contains("合成记忆正文"));
    assert_eq!(material.retained_memory_ids().len(), 1);
    assert!(material.endpoint_owner_ids().is_empty());
    let memory_exemption = ExemptionRequest::for_turn(Uuid::now_v7()).confirm(true);
    assert!(matches!(
        drafter().redact_with_memory(&request, &context, memory_exemption),
        Err(MemoryContextError::ExemptionOutsideConversation)
    ));
}

#[test]
fn trusted_turns_must_not_collide_with_the_paste_or_profile_brief() {
    let request = DraftRequest::from_paste(ProfileBrief::neutral(), "paste");
    for turn_id in [request.third_party_turn_ids()[0], BRIEF_TURN_ID] {
        let material = MemoryContextEntry::new(
            Uuid::now_v7(),
            turn_id,
            MemoryContextPrivacy::Owner,
            "title",
            "summary",
            EndpointMemoryUse::LocalReferenceOnly,
        )
        .unwrap();
        let result = MemoryContext::new(vec![material]);
        if turn_id == BRIEF_TURN_ID {
            assert!(matches!(result, Err(MemoryContextError::DuplicateIdentity)));
        } else {
            assert!(matches!(
                drafter().redact_with_memory(&request, &result.unwrap(), None),
                Err(MemoryContextError::TurnIdentityCollision)
            ));
        }
    }
}

#[test]
fn offline_draft_stays_the_original_deterministic_template() {
    let request = DraftRequest::from_paste(ProfileBrief::neutral(), "paste");
    let draft = drafter().draft_offline(&request).unwrap();
    assert!(!draft.text.contains("本地标题秘密"));
    assert!(draft.text.contains(soul_draft::BODY_SLOT));
    let empty = drafter()
        .redact_with_memory(&request, &MemoryContext::default(), None)
        .unwrap();
    assert_eq!(empty, drafter().redact(&request, None).unwrap());
}
