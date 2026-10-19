//! Optional opening-template inputs in the same transactions as their sources.
use crate::store::{backend, SqlCipherStore};
use rusqlite::{params, Connection, OptionalExtension};
use soul_store_api::assistance::{AssistanceItem, AssistanceSource, AssistanceWorkClaim};
use soul_store_api::preparation::*;
use soul_store_api::types::{StoreError, StoreResult};
use uuid::Uuid;

fn changed() -> StoreError {
    StoreError::ContractViolation("ASSISTANCE_LOCAL_PREPARATION_CHANGED".into())
}
fn parse(value: String) -> StoreResult<Uuid> {
    value.parse().map_err(backend)
}
pub fn local_template_voice(value: &serde_json::Value) -> StoreResult<LocalTemplateVoice> {
    fn field(value: &serde_json::Value, key: &str, allowed: &[&str]) -> StoreResult<String> {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .filter(|v| allowed.contains(v))
            .map(str::to_owned)
            .ok_or_else(|| {
                StoreError::ContractViolation("ASSISTANCE_LOCAL_PROFILE_UNAVAILABLE".into())
            })
    }
    Ok(LocalTemplateVoice {
        register: field(value, "register", &["casual", "plain", "formal"])?,
        directness: field(value, "directness", &["reserved", "balanced", "direct"])?,
        warmth: field(value, "warmth", &["cool", "even", "warm"])?,
        emoji_use: field(value, "emoji_use", &["never", "sparing", "frequent"])?,
    })
}
pub(crate) fn invalidate_item(conn: &Connection, item: &str) -> StoreResult<()> {
    conn.execute("DELETE FROM assistance_local_templates WHERE item_id=?1 AND EXISTS(SELECT 1 FROM assistance_items i WHERE i.item_id=?1 AND (i.revision!=assistance_local_templates.item_revision OR i.handling!='open' OR i.validity!='current'))",[item]).map_err(backend)?;
    conn.execute("DELETE FROM assistance_preparation_preferences WHERE item_id=?1 AND EXISTS(SELECT 1 FROM assistance_items WHERE item_id=?1 AND validity='forgotten')",[item]).map_err(backend)?;
    Ok(())
}
pub(crate) fn invalidate_sources(conn: &Connection) -> StoreResult<()> {
    conn.execute("DELETE FROM assistance_local_templates WHERE item_id IN (SELECT item_id FROM assistance_items WHERE bundle_id IS NOT NULL)",[]).map_err(backend)?;
    Ok(())
}
/// Only four actual enum values affect this pure renderer, never axes or locks.
pub(crate) fn profile_changed(
    conn: &Connection,
    owner: Uuid,
    old: Option<&serde_json::Value>,
    new: &serde_json::Value,
) -> StoreResult<()> {
    let fields = ["register", "directness", "warmth", "emoji_use"];
    if old.is_some_and(|old| fields.iter().all(|key| old.get(*key) == new.get(*key))) {
        return Ok(());
    }
    let mut stmt=conn.prepare("SELECT item_id FROM assistance_preparation_preferences WHERE owner_profile_id=?1 AND enabled=1").map_err(backend)?;
    let items = stmt
        .query_map([owner.to_string()], |r| r.get::<_, String>(0))
        .map_err(backend)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(backend)?;
    drop(stmt);
    for item in items {
        conn.execute(
            "DELETE FROM assistance_local_templates WHERE item_id=?1",
            [&item],
        )
        .map_err(backend)?;
        conn.execute(
            "UPDATE assistance_preparation_preferences SET failure_code=NULL WHERE item_id=?1",
            [&item],
        )
        .map_err(backend)?;
        crate::assistance_work::enqueue(conn, &item)?;
    }
    Ok(())
}
impl SqlCipherStore {
    pub fn local_preparation_preference(
        &self,
        item: Uuid,
    ) -> StoreResult<LocalPreparationPreference> {
        let row:Option<(String,bool,String,Option<String>)>=self.conn.query_row("SELECT revision,enabled,owner_profile_id,failure_code FROM assistance_preparation_preferences WHERE item_id=?1",[item.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(backend)?;
        Ok(match row {
            Some((rev, enabled, owner, failure)) => LocalPreparationPreference {
                item_id: item,
                revision: Some(parse(rev)?),
                enabled,
                owner_profile_id: Some(parse(owner)?),
                failure_code: failure,
            },
            None => LocalPreparationPreference {
                item_id: item,
                revision: None,
                enabled: false,
                owner_profile_id: None,
                failure_code: None,
            },
        })
    }
    pub fn set_local_preparation_preference(
        &mut self,
        item: Uuid,
        revision: Uuid,
        expected: Option<Uuid>,
        enabled: bool,
        owner: Uuid,
    ) -> StoreResult<LocalPreparationPreference> {
        self.transact(|store|{
            let current=store.assistance_item(item)?;
            if current.revision!=revision || !matches!(current.source,AssistanceSource::ManualGoal{..}) || current.validity==soul_store_api::assistance::AssistanceValidity::Forgotten || (enabled && !current.is_current()) || store.local_preparation_preference(item)?.revision!=expected{return Err(changed())}
            store.conn.execute("INSERT INTO assistance_preparation_preferences(item_id,revision,enabled,owner_profile_id) VALUES(?1,?2,?3,?4) ON CONFLICT(item_id) DO UPDATE SET revision=excluded.revision,enabled=excluded.enabled,owner_profile_id=excluded.owner_profile_id,failure_code=NULL",params![item.to_string(),Uuid::now_v7().to_string(),enabled,owner.to_string()]).map_err(backend)?;
            store.conn.execute("DELETE FROM assistance_local_templates WHERE item_id=?1",[item.to_string()]).map_err(backend)?;
            // Never erase another business event's shared pending revision.
            if enabled{crate::assistance_work::enqueue(&store.conn,&item.to_string())?;}
            store.local_preparation_preference(item)
        })
    }
    pub fn local_template(&self, item: Uuid) -> StoreResult<Option<LocalTemplateMetadata>> {
        let row=self.conn.query_row("SELECT template_id,item_revision,preference_revision,source_doc,owner_profile_id,voice_register,voice_directness,voice_warmth,voice_emoji_use,prepared_at FROM assistance_local_templates WHERE item_id=?1 AND kind='opening_v1'",[item.to_string()],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?))).optional().map_err(backend)?;
        row.map(
            |(id, rev, pref, source, owner, register, directness, warmth, emoji_use, at)| {
                Ok(LocalTemplateMetadata {
                    template_id: parse(id)?,
                    item_id: item,
                    item_revision: parse(rev)?,
                    preference_revision: parse(pref)?,
                    source: serde_json::from_str(&source).map_err(backend)?,
                    owner_profile_id: parse(owner)?,
                    voice: LocalTemplateVoice {
                        register,
                        directness,
                        warmth,
                        emoji_use,
                    },
                    prepared_at: at,
                    kind: LocalTemplateKind::OpeningV1,
                })
            },
        )
        .transpose()
    }
    pub fn finish_local_template(
        &self,
        claim: &AssistanceWorkClaim,
        item: &AssistanceItem,
        voice: Option<&LocalTemplateVoice>,
        owner: Uuid,
        at: i64,
        failure: Option<&str>,
    ) -> StoreResult<bool> {
        if self.conn.is_autocommit() {
            return Err(changed());
        }
        let owns:bool=self.conn.query_row("SELECT EXISTS(SELECT 1 FROM assistance_work WHERE item_id=?1 AND claim_id=?2 AND claimed_revision=?3)",params![claim.item_id.to_string(),claim.claim_id.to_string(),claim.revision.to_string()],|r|r.get(0)).map_err(backend)?;
        if !owns {
            return Ok(false);
        }
        let pref = self.local_preparation_preference(item.item_id)?;
        if !pref.enabled
            || pref.revision != claim.preparation_revision
            || pref.owner_profile_id != Some(owner)
            || item.revision != claim.revision
            || !item.is_current()
        {
            return Ok(false);
        }
        let code = failure.map(|code| match code {
            "ASSISTANCE_LOCAL_PROFILE_UNAVAILABLE" => code,
            _ => "ASSISTANCE_LOCAL_TEMPLATE_FAILED",
        });
        self.conn
            .execute(
                "UPDATE assistance_preparation_preferences SET failure_code=?2 WHERE item_id=?1",
                params![item.item_id.to_string(), code],
            )
            .map_err(backend)?;
        if let Some(voice) = voice {
            self.conn.execute("INSERT INTO assistance_local_templates(item_id,template_id,item_revision,preference_revision,source_doc,owner_profile_id,voice_register,voice_directness,voice_warmth,voice_emoji_use,prepared_at,kind) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,'opening_v1') ON CONFLICT(item_id) DO UPDATE SET template_id=excluded.template_id,item_revision=excluded.item_revision,preference_revision=excluded.preference_revision,source_doc=excluded.source_doc,owner_profile_id=excluded.owner_profile_id,voice_register=excluded.voice_register,voice_directness=excluded.voice_directness,voice_warmth=excluded.voice_warmth,voice_emoji_use=excluded.voice_emoji_use,prepared_at=excluded.prepared_at,kind=excluded.kind",params![item.item_id.to_string(),Uuid::now_v7().to_string(),item.revision.to_string(),pref.revision.map(|v|v.to_string()),serde_json::to_string(&item.source).map_err(backend)?,owner.to_string(),voice.register,voice.directness,voice.warmth,voice.emoji_use,at]).map_err(backend)?;
        } else {
            self.conn
                .execute(
                    "DELETE FROM assistance_local_templates WHERE item_id=?1",
                    [item.item_id.to_string()],
                )
                .map_err(backend)?;
        }
        Ok(true)
    }
    pub fn refresh_local_template(&self, item: Uuid) -> StoreResult<()> {
        if self.conn.is_autocommit() {
            return Err(changed());
        }
        self.conn
            .execute(
                "DELETE FROM assistance_local_templates WHERE item_id=?1",
                [item.to_string()],
            )
            .map_err(backend)?;
        crate::assistance_work::enqueue(&self.conn, &item.to_string())
    }
}
