//! Bounded source facts consumed by Recruit582C40. ActorData464BD0 records
//! RecruitActor references at467C89..467DB6;473AD0 prepends deferred links,
//! and475230 ->461810 ->460310 visits them backwards and prepends+4EC.
//! Thus successful final targets retain authored order, including duplicates.
//! Backreference updates and full ActorData loading are not implemented here.
use crate::{
    actors::{ActorCatalog, ResolvedNode},
    native::{Error, Result, TypedValue},
    resources::Provenance,
};
use aa_numeric::Precision;
use serde::Serialize;

fn actor<'a>(catalog: &'a ActorCatalog, id: &str) -> Result<ResolvedNode<'a>> {
    let found = catalog
        .find(id)
        .ok_or_else(|| Error::Unsupported(format!("missing canonical ActorData {id}")))?;
    if !found.id.eq_ignore_ascii_case(id) {
        return Err(Error::Unsupported(format!(
            "canonical ActorData {id} is shadowed by translation to {}",
            found.id
        )));
    }
    if catalog.native_category(id)?.native_kind != 2 {
        return Err(Error::Unsupported(format!(
            "{id} is not standard ActorData"
        )));
    }
    catalog.resolved_actor(id)
}
fn text(node: &ResolvedNode<'_>, name: &str) -> Result<String> {
    match node.plain_string_field(name)? {
        Some(TypedValue::String { value, .. }) => Ok(value),
        _ => Err(Error::Unsupported(format!(
            "required string {name} is missing"
        ))),
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct RecruitTargets {
    /// Canonical ActorData IDs in native completed+4EC list order.
    pub ids: Vec<String>,
    pub sources: Vec<Provenance>,
}

/// Success-only projection of Recruit targets, with real typed source refs.
/// Missing/invalid targets fail; they never turn a nonempty list into absence.
pub fn actor_recruit_targets(catalog: &ActorCatalog, id: &str) -> Result<RecruitTargets> {
    let node = actor(catalog, id)?;
    let Some(recruit) = node.child("RecruitComponent")? else {
        return Ok(RecruitTargets {
            ids: vec![],
            sources: vec![],
        });
    };
    let entries = recruit.children("RecruitActor")?;
    if !entries.is_empty() && node.child("StructureComponent")?.is_none() {
        return Err(Error::Unsupported(format!(
            "Recruit actor {id} has targets without StructureComponent (467D26 throws)"
        )));
    }
    // Deferred callbacks run last-created first. Validate in that order,
    // then restore the final list order after successful front insertions.
    let mut ids = Vec::with_capacity(entries.len());
    for entry in entries.iter().rev() {
        let lookup = text(entry, "actor_ids")?;
        let target = catalog.find(&lookup).ok_or_else(|| {
            Error::Unsupported(format!("Unknown recruit target {lookup:?} for {id}"))
        })?;
        if catalog.native_category(&lookup)?.native_kind != 2 {
            return Err(Error::Unsupported(format!(
                "recruit target {lookup:?} is not ActorData"
            )));
        }
        ids.push(target.id.clone());
    }
    ids.reverse();
    Ok(RecruitTargets {
        ids,
        sources: recruit.sources(),
    })
}

/// Completed transport_actor deferral4608C0, observed by582C40 as+184!=0.
pub fn actor_transport_class(catalog: &ActorCatalog, id: &str) -> Result<bool> {
    let node = actor(catalog, id)?;
    let lookup = text(&node, "transport_actor")?;
    if lookup.is_empty() {
        return Ok(false);
    }
    if catalog.find(&lookup).is_none() || catalog.native_category(&lookup)?.native_kind != 2 {
        return Err(Error::Unsupported(format!(
            "Unknown transport ActorData {lookup:?} for {id}"
        )));
    }
    if catalog
        .native_node_sources(&lookup, &["TransportComponent"])?
        .is_empty()
    {
        return Err(Error::Unsupported(format!(
            "transport actor {lookup:?} has no TransportComponent (4608C0 throws)"
        )));
    }
    Ok(true)
}

/// Loaded ActorState+28;48A630 registers NONE0,BUSY1,QUEUE2. The compiled
/// state's canonical name must not be passed through a second translation.
pub fn actor_state_type(catalog: &ActorCatalog, id: &str, pc: Precision) -> Result<u32> {
    if let Some(entry) = catalog.native_data_find("ActorState", id)
        && !entry.ids.eq_ignore_ascii_case(id)
    {
        return Err(Error::Unsupported(format!(
            "canonical ActorState {id} is shadowed by translation to {}",
            entry.ids
        )));
    }
    let value = catalog.native_actor_state(id, pc)?.state_type;
    let TypedValue::String { value, .. } = value else {
        return Err(Error::Unsupported("ActorState.type is not a string".into()));
    };
    ["NONE", "BUSY", "QUEUE"]
        .iter()
        .position(|s| s.eq_ignore_ascii_case(&value))
        .map(|v| v as u32)
        .ok_or_else(|| Error::Unsupported(format!("unknown native ActorState type {value:?}")))
}
