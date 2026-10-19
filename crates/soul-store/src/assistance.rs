//! Assistance metadata and conditional writes in the encrypted main database.
use crate::store::{backend, enum_text, SqlCipherStore};
use rusqlite::{params, Connection, OptionalExtension, Row};
use soul_store_api::assistance::{AssistanceItem, AssistanceObservationSource, AssistanceSource};
use soul_store_api::types::{StoreError, StoreResult};
use uuid::Uuid;

const COLUMNS: &str = "item_id,revision,kind,memory_id,observation_item_id,observation_revision,namespace_id,contact_id,bundle_id,validity,handling,deadline_at,created_at,updated_at";

struct Raw {
    id: String,
    revision: String,
    kind: String,
    memory: Option<String>,
    observation: Option<String>,
    observation_revision: Option<String>,
    namespace: Option<String>,
    contact: Option<String>,
    bundle: Option<String>,
    validity: String,
    handling: String,
    deadline: Option<String>,
    created: i64,
    updated: i64,
}
impl Raw {
    fn read(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get(0)?,
            revision: r.get(1)?,
            kind: r.get(2)?,
            memory: r.get(3)?,
            observation: r.get(4)?,
            observation_revision: r.get(5)?,
            namespace: r.get(6)?,
            contact: r.get(7)?,
            bundle: r.get(8)?,
            validity: r.get(9)?,
            handling: r.get(10)?,
            deadline: r.get(11)?,
            created: r.get(12)?,
            updated: r.get(13)?,
        })
    }
    fn decode(self) -> StoreResult<AssistanceItem> {
        let required = |v: Option<String>| {
            v.ok_or_else(|| StoreError::ContractViolation("invalid assistance source".into()))
        };
        let id = |v: String| Uuid::parse_str(&v).map_err(backend);
        let source = match self.kind.as_str() {
            "manual_goal" => AssistanceSource::ManualGoal {
                memory_id: id(required(self.memory)?)?,
                observation: match self.observation {
                    Some(item) => Some(AssistanceObservationSource {
                        item_id: id(item)?,
                        revision: id(required(self.observation_revision)?)?,
                        namespace_id: id(required(self.namespace)?)?,
                        contact_id: id(required(self.contact)?)?,
                        bundle_id: id(required(self.bundle)?)?,
                    }),
                    None => None,
                },
            },
            "observation" => AssistanceSource::Observation {
                namespace_id: id(required(self.namespace)?)?,
                contact_id: id(required(self.contact)?)?,
                bundle_id: id(required(self.bundle)?)?,
            },
            _ => {
                return Err(StoreError::ContractViolation(
                    "invalid assistance kind".into(),
                ))
            }
        };
        Ok(AssistanceItem {
            item_id: id(self.id)?,
            revision: id(self.revision)?,
            source,
            validity: serde_json::from_value(serde_json::Value::String(self.validity))
                .map_err(backend)?,
            handling: serde_json::from_value(serde_json::Value::String(self.handling))
                .map_err(backend)?,
            deadline_at: self.deadline,
            created_at: self.created,
            updated_at: self.updated,
        })
    }
}

fn write(conn: &Connection, item: &AssistanceItem, expected: Option<Uuid>) -> StoreResult<()> {
    let (kind, memory, observation, observation_revision, namespace, contact, bundle) =
        match &item.source {
            AssistanceSource::ManualGoal {
                memory_id,
                observation,
            } => (
                "manual_goal",
                Some(memory_id.to_string()),
                observation.as_ref().map(|s| s.item_id.to_string()),
                observation.as_ref().map(|s| s.revision.to_string()),
                observation.as_ref().map(|s| s.namespace_id.to_string()),
                observation.as_ref().map(|s| s.contact_id.to_string()),
                observation.as_ref().map(|s| s.bundle_id.to_string()),
            ),
            AssistanceSource::Observation {
                namespace_id,
                contact_id,
                bundle_id,
            } => (
                "observation",
                None,
                None,
                None,
                Some(namespace_id.to_string()),
                Some(contact_id.to_string()),
                Some(bundle_id.to_string()),
            ),
        };
    let sql = if expected.is_some() {
        "UPDATE assistance_items SET revision=?2,kind=?3,memory_id=?4,observation_item_id=?5,observation_revision=?6,namespace_id=?7,contact_id=?8,bundle_id=?9,validity=?10,handling=?11,deadline_at=?12,created_at=?13,updated_at=?14 WHERE item_id=?1 AND revision=?15"
    } else {
        "INSERT INTO assistance_items(item_id,revision,kind,memory_id,observation_item_id,observation_revision,namespace_id,contact_id,bundle_id,validity,handling,deadline_at,created_at,updated_at) SELECT ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14 WHERE ?15 IS NULL"
    };
    let changed = conn
        .execute(
            sql,
            params![
                item.item_id.to_string(),
                item.revision.to_string(),
                kind,
                memory,
                observation,
                observation_revision,
                namespace,
                contact,
                bundle,
                enum_text(&item.validity)?,
                enum_text(&item.handling)?,
                item.deadline_at,
                item.created_at,
                item.updated_at,
                expected.map(|r| r.to_string())
            ],
        )
        .map_err(backend)?;
    if changed != 1 {
        return Err(StoreError::ContractViolation(
            "assistance revision changed".into(),
        ));
    }
    Ok(())
}

impl SqlCipherStore {
    pub fn assistance_item(&self, id: Uuid) -> StoreResult<AssistanceItem> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM assistance_items WHERE item_id=?1"),
                [id.to_string()],
                Raw::read,
            )
            .optional()
            .map_err(backend)?
            .ok_or_else(|| StoreError::not_found("assistance", id))?
            .decode()
    }
    /// No domain rebuilding or blob access is permitted in this metadata list.
    pub fn assistance_items(&self) -> StoreResult<Vec<AssistanceItem>> {
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM assistance_items ORDER BY created_at,item_id"
            ))
            .map_err(backend)?;
        let rows = statement.query_map([], Raw::read).map_err(backend)?;
        rows.map(|row| row.map_err(backend)?.decode()).collect()
    }
    pub fn assistance_observation(
        &self,
        namespace: Uuid,
        contact: Uuid,
    ) -> StoreResult<Option<AssistanceItem>> {
        self.conn.query_row(&format!("SELECT {COLUMNS} FROM assistance_items WHERE kind='observation' AND namespace_id=?1 AND contact_id=?2"),params![namespace.to_string(),contact.to_string()],Raw::read)
            .optional().map_err(backend)?.map(Raw::decode).transpose()
    }
    pub fn insert_assistance_item(&mut self, item: &AssistanceItem) -> StoreResult<()> {
        write(&self.conn, item, None)
    }
    /// Caller keeps domain changes and this CAS inside its existing transaction.
    pub fn replace_assistance_item(
        &mut self,
        item: &AssistanceItem,
        expected: Uuid,
    ) -> StoreResult<()> {
        let tx = self.conn.savepoint().map_err(backend)?;
        write(&tx, item, Some(expected))?;
        // A source's new revision never silently follows into another goal.
        tx.execute("UPDATE assistance_items SET validity='stale',revision=?2,updated_at=?3 WHERE observation_item_id=?1 AND validity!='forgotten'",
            params![item.item_id.to_string(),Uuid::now_v7().to_string(),item.updated_at]).map_err(backend)?;
        tx.commit().map_err(backend)
    }
}
