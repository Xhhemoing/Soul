//! Only deterministic synthetic metadata; no message body or platform key store.

use serde_json::json;
use soul_schema::common::{SchemaVersion, Subject, SupportedBand};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::evidence::{EvidenceKind, EvidenceMethod, SoulEvidence};
use soul_schema::memory::ForgetState;
use soul_schema::relationship::{EgressScope, SoulRelationship};
use soul_store::SqlCipherStore;
use soul_store_api::{GraphStore, ProfileStore};
use soulcore::commands::graph::{
    EvidenceRowView, PeopleGraphView, PersonNodeView, TieEdgeView, WORKING_HYPOTHESIS_NOTICE,
};
use uuid::Uuid;

pub const SEED: &str = "synthetic people view reading";
pub const SLOTS_PER_EDGE: usize = 4;
pub const FIRST: &str = "2026-08-23T09:00:00Z";
pub const LAST: &str = "2026-08-23T10:00:00Z";

#[derive(Debug, Clone, Copy)]
pub enum Shape {
    Unique,
    Shared,
}

impl Shape {
    pub fn name(self) -> &'static str {
        match self {
            Self::Unique => "unique",
            Self::Shared => "shared",
        }
    }

    pub fn distinct(self, peers: usize) -> usize {
        match self {
            Self::Unique => peers * SLOTS_PER_EDGE,
            Self::Shared => 3,
        }
    }
}

pub fn id(n: usize) -> Uuid {
    Uuid::from_u128(0x0192_a1b2_c3d4_7e5f_8a9b_0c1d_0000_0000 + n as u128)
}

pub fn peer(index: usize) -> Uuid {
    id(100 + index)
}

pub fn relationship(index: usize) -> Uuid {
    id(10_000 + index)
}

pub fn evidence_id(index: usize) -> Uuid {
    id(100_000 + index)
}

pub fn evidence(index: usize) -> SoulEvidence {
    let (kind, method, strength) = match index % SLOTS_PER_EDGE {
        0 => (
            EvidenceKind::Message,
            Some(EvidenceMethod::Heuristic),
            SupportedBand::Weak,
        ),
        1 => (
            EvidenceKind::UserCorrection,
            Some(EvidenceMethod::Manual),
            SupportedBand::Strong,
        ),
        2 => (EvidenceKind::Aggregate, None, SupportedBand::Moderate),
        _ => (
            EvidenceKind::UserStatement,
            Some(EvidenceMethod::UserStated),
            SupportedBand::Weak,
        ),
    };
    SoulEvidence {
        schema_version: SchemaVersion,
        evidence_id: evidence_id(index),
        kind,
        subject: Subject::Mixed,
        source_refs: vec![json!({"synthetic_people_slot": index})],
        strength,
        method,
        exportable_to_research: Some(false),
        privacy: None,
    }
}

pub fn refs(index: usize, shape: Shape) -> Vec<Uuid> {
    match shape {
        Shape::Unique => (0..SLOTS_PER_EDGE)
            .map(|slot| evidence_id(index * SLOTS_PER_EDGE + slot))
            .collect(),
        // A repeated id within an edge and reuse across every edge. Output
        // slots must survive both forms of reuse, including their order.
        Shape::Shared => [0, 1, 0, 2].map(evidence_id).to_vec(),
    }
}

pub fn seed(store: &mut SqlCipherStore, peers: usize, shape: Shape) {
    assert!(peers > 0 && peers <= 1000);
    // Both shapes store exactly K evidence rows; only the edge references
    // differ. Unreferenced rows are deliberately present in the shared shape.
    store
        .transact::<_, soul_store_api::types::StoreError, _>(|store| {
            for index in 0..=peers {
                store.put_contact(SoulContact {
                    schema_version: SchemaVersion,
                    contact_id: if index == 0 { id(1) } else { peer(index - 1) },
                    contact_class: if index == 0 {
                        ContactClass::Owner
                    } else {
                        ContactClass::ThirdParty
                    },
                    display_label_ref: None,
                    identifiers: None,
                    forget_state: ForgetState::Active,
                })?;
            }
            for index in 0..peers * SLOTS_PER_EDGE {
                store.put_evidence(evidence(index))?;
            }
            for index in 0..peers {
                let locked = index % 2 == 0;
                store.put_relationship(SoulRelationship {
                    schema_version: SchemaVersion,
                    relationship_id: relationship(index),
                    from_contact_id: id(1),
                    to_contact_id: peer(index),
                    types: Some(vec![json!("direct"), json!("reciprocal")]),
                    // A legitimate legacy stored strength: no invented measured
                    // venue split. User and machine values remain separate.
                    tie_strength: Some(json!({
                        "band": if locked { "strong" } else { "weak" },
                        "interaction_count": 4, "outgoing_count": 3, "incoming_count": 1,
                        "conversation_count": 2, "active_day_count": 1,
                        "first_contact_utc": FIRST, "last_contact_utc": LAST,
                        "locked_by_user": locked,
                        "user_band": if locked { Some("strong") } else { None },
                        "machine_band": "weak"
                    })),
                    evidence_ids: refs(index, shape),
                    egress_scope: Some(EgressScope::LocalOnly),
                })?;
            }
            Ok(())
        })
        .expect("legal synthetic graph fixture");
}

fn expected_evidence(index: usize) -> EvidenceRowView {
    let (kind, method, strength) = match index % SLOTS_PER_EDGE {
        0 => ("message", "heuristic", "weak"),
        1 => ("user_correction", "manual", "strong"),
        2 => ("aggregate", "unstated", "moderate"),
        _ => ("user_statement", "user_stated", "weak"),
    };
    EvidenceRowView {
        evidence_id: evidence_id(index).to_string(),
        kind: kind.into(),
        method: method.into(),
        strength: strength.into(),
    }
}

pub fn expected_view(peers: usize, shape: Shape) -> PeopleGraphView {
    let mut people = vec![PersonNodeView {
        contact_id: id(1).to_string(),
        is_you: true,
        identifier_hint: id(1).to_string().chars().take(8).collect(),
        interaction_count: (peers * SLOTS_PER_EDGE) as u64,
        last_contact_utc: Some(LAST.into()),
        tie_count: peers,
        forgotten: false,
    }];
    let mut ties = Vec::with_capacity(peers);
    for index in 0..peers {
        people.push(PersonNodeView {
            contact_id: peer(index).to_string(),
            is_you: false,
            identifier_hint: peer(index).to_string().chars().take(8).collect(),
            interaction_count: 4,
            last_contact_utc: Some(LAST.into()),
            tie_count: 1,
            forgotten: false,
        });
        let locked = index % 2 == 0;
        let evidence = match shape {
            Shape::Unique => (0..SLOTS_PER_EDGE)
                .map(|slot| expected_evidence(index * SLOTS_PER_EDGE + slot))
                .collect(),
            Shape::Shared => [0, 1, 0, 2].map(expected_evidence).to_vec(),
        };
        ties.push(TieEdgeView {
            relationship_id: relationship(index).to_string(),
            from_contact_id: id(1).to_string(),
            to_contact_id: peer(index).to_string(),
            types: vec!["direct".into(), "reciprocal".into()],
            band: if locked { "strong" } else { "weak" }.into(),
            locked_by_user: locked,
            user_band: locked.then(|| "strong".into()),
            machine_band: Some("weak".into()),
            interaction_count: 4,
            outgoing_count: 3,
            incoming_count: 1,
            conversation_count: 2,
            active_day_count: 1,
            venue_split_measured: false,
            direct_count: 0,
            group_count: 0,
            first_contact_utc: FIRST.into(),
            last_contact_utc: LAST.into(),
            local_only: true,
            evidence,
        });
    }
    PeopleGraphView {
        self_contact_id: Some(id(1).to_string()),
        people,
        ties,
        notice: WORKING_HYPOTHESIS_NOTICE.into(),
        third_party_data_is_local_only: true,
    }
}
