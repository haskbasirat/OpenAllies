//! Concrete trigger lookups and field views over the one authoritative actor
//! store. Name registration is an explicit native publication operation, never
//! synthesized by scanning saved actors. Mind/property effect routing is separate.
use crate::{
    actor_world::{ActorStore, ObjectKey, Source, SpecialSource},
    names,
};
use aa_actors::ActorId;
use aa_body::credit::Object as BodyObject;
use aa_navigation::startup::Object;
use aa_mapworld::special_state::SpecialShell;
use aa_triggers::{
    manager::{Error, Result},
    objectives::{Entry, ObjectiveDef},
    world_program::Presentation,
};
use std::collections::BTreeMap;

fn state(error: impl std::fmt::Display) -> Error {
    Error::State(error.to_string())
}
fn checked(store: &ActorStore, object: Object) -> Result<ObjectKey> {
    store
        .checked_key(object.0)
        .ok_or_else(|| state("unknown canonical object key"))
}
///Raw54DAC0/54DC20 selector. No class filtering and no fallback generation.
pub fn resolve_id(store: &ActorStore, raw: u32) -> Option<Object> {
    store
        .key(ActorId::from_raw(raw))
        .map(|key| Object(key.raw()))
}
///Body APIs currently carry actual32-bit GTIDs. This checked mapping never
///casts the unrelated64-bit arena key into that identity domain.
pub fn body_actor_identity(store: &ActorStore, object: Object) -> Result<BodyObject> {
    let key = checked(store, object)?;
    let id = store
        .id(key)
        .ok_or_else(|| state("object has no registered native identity"))?;
    Ok(BodyObject(id.raw()))
}
pub fn named_actor(
    store: &ActorStore,
    names: &names::Table,
    name: &[u16],
) -> Result<Option<Object>> {
    //Registry actor-class bit comes from the actual load/create classification;
    //it is not inferred from a name or from presence of a Body component.
    Ok(names
        .resolve(name, store.identities(), |entry| entry.is_actor)
        .map_err(state)?
        .map(|entry| Object(entry.value.raw())))
}
pub fn named_kingdom(
    store: &ActorStore,
    names: &names::Table,
    name: &[u16],
) -> Result<Option<Object>> {
    let Some(id) = names.lookup(name).map_err(state)? else {
        return Ok(None);
    };
    let Some(key) = store.key(id) else {
        return Ok(None);
    };
    let entity = store.entity(key).map_err(state)?;
    //Kingdom creation/restoration is a distinct derived constructor. The current
    //store's ordinary actor factory cannot manufacture a Kingdom from an actor ID.
    Ok(
        matches!(&entity.source,Source::Special(s) if matches!(s.source,SpecialSource::Kingdom(_)))
            .then_some(Object(key.raw())),
    )
}
///Mutable view of the canonical packed word containing Kingdom+238..23B.
///The saved provenance record remains immutable.
fn victory_word_mut(store: &mut ActorStore, object: Object) -> Result<&mut u32> {
    let key = checked(store, object)?;
    let Source::Special(s) = &mut store.entity_mut(key).map_err(state)?.source else {
        return Err(state("victory target is not a derived Kingdom"));
    };
    if !matches!(s.source, SpecialSource::Kingdom(_)) {
        return Err(state("victory target is not a Kingdom"));
    }
    s.derived
        .as_mut()
        .ok_or_else(|| state("Kingdom derived constructor unavailable"))?
        .words
        .get_mut(&0x238)
        .ok_or_else(|| state("Kingdom default-victory word is not initialized"))
}
/// The derived Kingdom shell (4E5370 fields) behind a canonical Kingdom handle.
fn kingdom_shell(store: &ActorStore, object: Object) -> Result<&SpecialShell> {
    let key = checked(store, object)?;
    let Source::Special(s) = &store.entity(key).map_err(state)?.source else {
        return Err(state("trigger target is not a derived Kingdom"));
    };
    if !matches!(s.source, SpecialSource::Kingdom(_)) {
        return Err(state("trigger target is not a Kingdom"));
    }
    s.derived
        .as_ref()
        .ok_or_else(|| state("Kingdom derived constructor unavailable"))
}
fn kingdom_shell_mut(store: &mut ActorStore, object: Object) -> Result<&mut SpecialShell> {
    let key = checked(store, object)?;
    let Source::Special(s) = &mut store.entity_mut(key).map_err(state)?.source else {
        return Err(state("trigger target is not a derived Kingdom"));
    };
    if !matches!(s.source, SpecialSource::Kingdom(_)) {
        return Err(state("trigger target is not a Kingdom"));
    }
    s.derived
        .as_mut()
        .ok_or_else(|| state("Kingdom derived constructor unavailable"))
}
/// Kingdom+19C mission result (0 none, 1 won, 2 lost) read by 4E9720.
pub fn shell_mission_result(shell: &SpecialShell) -> Result<u32> {
    shell
        .words
        .get(&0x19c)
        .copied()
        .ok_or_else(|| state("Kingdom+19C mission result is not initialized"))
}
pub fn shell_set_mission_result(shell: &mut SpecialShell, value: u32) -> Result<()> {
    *shell
        .words
        .get_mut(&0x19c)
        .ok_or_else(|| state("Kingdom+19C mission result is not initialized"))? = value;
    Ok(())
}
/// Kingdom+239: byte 1 of the packed +238 word (4E9720 lost-message gate).
pub fn shell_flag_239(shell: &SpecialShell) -> Result<u8> {
    shell
        .words
        .get(&0x238)
        .map(|w| (*w >> 8) as u8)
        .ok_or_else(|| state("Kingdom+238 word is not initialized"))
}
pub fn mission_result(store: &ActorStore, object: Object) -> Result<u32> {
    shell_mission_result(kingdom_shell(store, object)?)
}
pub fn set_mission_result(store: &mut ActorStore, object: Object, value: u32) -> Result<()> {
    shell_set_mission_result(kingdom_shell_mut(store, object)?, value)
}
pub fn kingdom_flag_239(store: &ActorStore, object: Object) -> Result<u8> {
    shell_flag_239(kingdom_shell(store, object)?)
}
/// Kingdom+1EC Team as the saved identity handle (null/stale -> None).
pub fn kingdom_team(store: &ActorStore, object: Object) -> Result<Option<Object>> {
    let shell = kingdom_shell(store, object)?;
    match shell.identity_references.get(&0x1ec) {
        Some(Some(id)) => Ok(store.key(*id).map(|k| Object(k.raw()))),
        Some(None) => Ok(None),
        None if shell.words.get(&0x1ec) == Some(&0) => Ok(None),
        None => Err(state("Kingdom+1EC team reference is unbound")),
    }
}
pub fn default_victory(store: &mut ActorStore, object: Object) -> Result<u8> {
    Ok(*victory_word_mut(store, object)? as u8)
}
///Exact5421A0 single-byte write without losing adjacent initialized flags.
pub fn set_default_victory(store: &mut ActorStore, object: Object, value: u8) -> Result<()> {
    let word = victory_word_mut(store, object)?;
    *word = (*word & 0xffff_ff00) | u32::from(value);
    Ok(())
}
///Exact named branch of4F9510 before its later spatial/publication callbacks.
///Requires the staged Thing placement, so published actors cannot diverge into
///a second flag copy. Clone matches4BEDF0's captured name before the flag write.
pub fn publish_staged_name(
    store: &mut ActorStore,
    names: &mut names::Table,
    object: Object,
) -> Result<bool> {
    let key = checked(store, object)?;
    let id = store
        .id(key)
        .ok_or_else(|| state("name publication precedes identity registration"))?;
    let name = store.object(key).map_err(state)?.thing.name_utf16.clone();
    names
        .publish(
            &name,
            id,
            &mut store.staged_placement_mut(key).map_err(state)?.flags,
        )
        .map_err(state)
}
///4F94D0 aliases retain duplicate names and read the actor's current raw GTID.
pub fn register_alias(
    store: &ActorStore,
    names: &mut names::Table,
    object: Object,
    name: &[u16],
) -> Result<bool> {
    let key = checked(store, object)?;
    let id = store
        .id(key)
        .ok_or_else(|| state("alias actor has no registered identity"))?;
    names.alias(name, id).map_err(state)
}
///Presentation fields are owned independently from renderer controls. No guessed
///initial state is supplied: constructor/Load must provide the actual fields.
pub struct State {
    pub names: names::Table,
    pub presentation: Presentation,
    /// World+250 objective definitions supplied by the map loader.
    pub objective_definitions: Vec<ObjectiveDef>,
    /// Kingdom+300 objective lists. The saved native vector holds process
    /// pointers, so a list is bound only from an EMPTY saved vector; nonempty
    /// saved objective vectors are an explicit unsupported error.
    pub objectives: BTreeMap<u64, Vec<Entry>>,
    /// World+290 current weather data.
    pub weather: Option<Object>,
}
impl State {
    pub fn new(presentation: Presentation) -> Self {
        Self {
            names: names::Table::default(),
            presentation,
            objective_definitions: Vec::new(),
            objectives: BTreeMap::new(),
            weather: None,
        }
    }
    /// Kingdom+300 list for trigger objective methods (4E7050/4E2120/4E2020).
    pub fn objectives_mut(&mut self, store: &ActorStore, kingdom: Object) -> Result<&mut Vec<Entry>> {
        match self.objectives.entry(kingdom.0) {
            std::collections::btree_map::Entry::Occupied(e) => Ok(e.into_mut()),
            std::collections::btree_map::Entry::Vacant(e) => {
                let shell = kingdom_shell(store, kingdom)?;
                let saved = shell
                    .vectors
                    .get(&0x300)
                    .ok_or_else(|| state("Kingdom+300 objective vector is unconstructed"))?;
                if !saved.values.is_empty() {
                    return Err(state(
                        "saved Kingdom+300 objective entries require native pointer rebinding",
                    ));
                }
                Ok(e.insert(Vec::new()))
            }
        }
    }
}

/// Original map load order: MAPSTRINGS (47EA40) registers the map text table,
/// then TriggerManager 54D3D0 reads TRIGGERS in the RMP (external MAPSTRINGS)
/// stream mode, and 49AD70 resolves every localized key through it. Both
/// chunks are the decompressed RMP chunks; `language` is the LanguageIDS game
/// variable. Proven for all 80 original maps by aa-triggers'
/// triggers_native test (research/phase8-triggers/native-triggers.json).
pub fn load_map_triggers(
    triggers: &[u8],
    mapstrings: &[u8],
    language: &str,
    byte_budget: usize,
) -> Result<aa_triggers::decode::SavedManager> {
    use aa_triggers::{
        decode::{Localization, SavedManager},
        mapstrings::MapStrings,
    };
    let strings = MapStrings::parse(mapstrings, language).map_err(state)?;
    let mut saved = SavedManager::decode(triggers, Localization::External, byte_budget).map_err(state)?;
    strings.bind(&mut saved).map_err(state)?;
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_actors::ActorId;

    #[test]
    fn kingdom_constructor_fields_back_mission_result_and_flag() {
        // 4E5370 initial values: +19C = 0, +238 word = 0x01010001.
        let mut shell = SpecialShell::kingdom(ActorId::from_raw(0x10002), 3).unwrap();
        assert_eq!(shell_mission_result(&shell).unwrap(), 0);
        // Little-endian bytes of 0x01010001: +238=1, +239=0, +23A=1, +23B=1.
        assert_eq!(shell_flag_239(&shell).unwrap(), 0);
        shell_set_mission_result(&mut shell, 2).unwrap();
        assert_eq!(shell_mission_result(&shell).unwrap(), 2);
        assert_eq!(shell.words[&0x238], 0x0101_0001);
        assert!(shell.vectors[&0x300].values.is_empty());
        shell.words.remove(&0x19c);
        shell.words.remove(&0x238);
        assert!(shell_mission_result(&shell).is_err());
        assert!(shell_flag_239(&shell).is_err());
    }

    #[test]
    fn map_triggers_load_with_bound_localized_text() {
        // Extracted, SHA256-pinned chunks (research/phase8-triggers/chunks).
        let chunks = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-triggers/chunks");
        let triggers = std::fs::read(chunks.join("Campaign__allies01_el_alamein.RMP.bin")).unwrap();
        let text = std::fs::read(chunks.join("mapstrings/Campaign__allies01_el_alamein.RMP.txt")).unwrap();
        let saved = load_map_triggers(&triggers, &text, "English", triggers.len()).unwrap();
        let mut bound = 0;
        for t in &saved.triggers {
            for e in t.events.iter().chain(&t.conditions).chain(&t.actions) {
                for p in &e.parameters {
                    if let aa_triggers::decode::Value::Localized { inline, .. } = &p.value {
                        assert!(inline.is_some());
                        bound += 1;
                    }
                }
            }
        }
        assert!(bound > 0);
        // Only [Text] nodes are within the proven MAPSTRINGS subset.
        let other: &[u8] = b"[Other]\r\n{\r\n}\r\n";
        assert!(load_map_triggers(&triggers, other, "English", triggers.len()).is_err());
    }
}

