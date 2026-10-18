//! Completed no-spatial/empty-reference branch of ordinary GActor::Load.
//! The guards are native field/component conditions, not source-ID exclusions.
use crate::{
    actor_payload::ActorTail,
    restore::{RestoreDefinitions, RestoredComponent},
};
use aa_actors::{components::ComponentSlot, loading::LoadShell};
use aa_numeric::Precision;

pub fn required_by_ground_slot(
    slot: ComponentSlot,
) -> std::result::Result<Option<ComponentSlot>, String> {
    Ok(match slot {
        ComponentSlot::Construction => Some(ComponentSlot::Body),
        ComponentSlot::Experience => Some(ComponentSlot::Properties),
        // Defender vtable A3BBD4 +20 = 6B2450 (`mov eax,5`): Performer.
        ComponentSlot::Defender => Some(ComponentSlot::Performer),
        // Supply A400C0, Recruit A3E6D0, MobileDeployment A3CD00 and
        // Research A3EB58 +20 = 580F70 (`or eax,-1`): no requirement.
        // Lair A3CC6C and City A3B6F8 +20 are 580F70 as well.
        // Transport A40100 +20 = 7C7840 (`mov eax,3`): Mover.
        ComponentSlot::Transport => Some(aa_actors::transport_state::REQUIRED_COMPONENT),
        // Builder A3B2A8 +20 = 7C53E0 (`mov eax,4`): Mind.
        ComponentSlot::Builder => Some(aa_actors::builder_state::REQUIRED_COMPONENT),
        ComponentSlot::Supply
        | ComponentSlot::Recruit
        | ComponentSlot::MobileDeployment
        | ComponentSlot::Research
        | ComponentSlot::Lair
        | ComponentSlot::City => None,
        ComponentSlot::Body
        | ComponentSlot::Mover
        | ComponentSlot::Character
        | ComponentSlot::Performer
        | ComponentSlot::Structure
        | ComponentSlot::Properties
        | ComponentSlot::Organization
        | ComponentSlot::Element
        | ComponentSlot::Economy
        | ComponentSlot::Denizen
        | ComponentSlot::Detection => None,
        _ => {
            return Err(format!(
                "Actor requirement virtual for slot{slot:?} is unavailable"
            ));
        }
    })
}

pub fn validate_supported_requirements(
    actor: &aa_actors::loading::ActorShell<RestoredComponent>,
) -> std::result::Result<(), String> {
    for (slot, value) in actor.components.iter() {
        if slot != value.slot() {
            return Err("Actor component is stored in the wrong native slot".into());
        }
        let required = match value {
            RestoredComponent::Mind(mind) => Some(mind.required),
            _ => required_by_ground_slot(slot)?,
        };
        if let Some(required) = required
            && !actor.components.present(required)
        {
            // 4CFB00 false -> 858AD0 "Component requirements not met for
            // actor: %s" -> 41DFB0 (TGC_LoadWarningException, caught by 4FAE80).
            return Err(crate::restore::load_warning_exception(format!(
                "Fatal File Error: Component requirements not met for actor ({slot:?} needs {required:?})"
            )));
        }
    }
    Ok(())
}

/// Restore-wide inputs of the GActor::Load name tail (4D971B..4D99C2).
pub struct NameTail<'a> {
    pub id: aa_actors::ActorId,
    /// Saved actor version (signed compare with 0xE).
    pub actor_version: u32,
    /// 4E4980 increments the owner Kingdom's +35C table, which Kingdom Load
    /// restored from the save (4E910C..4E9171).
    pub registry: &'a mut aa_actors::IdentityRegistry<crate::restore::RestoredRecord>,
    pub messages: &'a mut Vec<crate::restore::NativeMessage>,
    pub source_ordinal: usize,
    /// GWorld+E0 (the saved world time) read by 4D67F0.
    pub world_time_bits: u32,
}
struct NameWorld<'a, 'b, R: RestoreDefinitions + ?Sized> {
    tail: &'a mut NameTail<'b>,
    definitions: &'a R,
    definition_id: &'a str,
    owner_e8: u32,
    pc: Precision,
}
fn name_error(e: String) -> aa_actors::organization_state::LoadError {
    aa_actors::organization_state::LoadError::World(e)
}
impl<R: RestoreDefinitions + ?Sized> aa_actors::actor_names::NameWorld for NameWorld<'_, '_, R> {
    fn gtid(&self) -> aa_actors::organization_state::Result<u32> {
        Ok(self.tail.id.raw())
    }
    fn definition_name(&self) -> aa_actors::organization_state::Result<String> {
        Ok(self.definition_id.to_owned())
    }
    fn display_name(&self) -> aa_actors::organization_state::Result<Vec<u16>> {
        self.definitions
            .actor_display_name(self.definition_id, self.pc)
            .map_err(name_error)
    }
    fn numbered(&self) -> aa_actors::organization_state::Result<bool> {
        let id: Vec<u16> = self.definition_id.encode_utf16().collect();
        self.definitions.actor_numbered(&id, self.pc).map_err(name_error)
    }
    fn gtid_live(&self, id: u32) -> aa_actors::organization_state::Result<bool> {
        Ok(self.tail.registry.get(aa_actors::ActorId::from_raw(id)).is_some())
    }
    fn localize(&mut self, key: &str) -> aa_actors::organization_state::Result<Option<Vec<u16>>> {
        let key: Vec<u16> = key.encode_utf16().collect();
        self.definitions.localize_optional(&key).map_err(name_error)
    }
    fn strings_from_stream(&mut self) -> aa_actors::organization_state::Result<bool> {
        // 59E520 on the RMP load context: MAPSTRINGS is an external chunk
        // (the TriggerManager reads in the same External mode).
        Ok(false)
    }
    fn map_string_exists(&mut self, key: &[u16]) -> aa_actors::organization_state::Result<bool> {
        Ok(self
            .definitions
            .localize_optional(key)
            .map_err(name_error)?
            .is_some())
    }
    fn warn(&mut self, message: aa_actors::organization_state::Message) -> aa_actors::organization_state::Result<()> {
        self.tail.messages.push(crate::restore::NativeMessage {
            source_ordinal: self.tail.source_ordinal,
            function: 0x5a1040,
            line: 0,
            text: message.to_string(),
        });
        Ok(())
    }
    fn owner_kingdom(&mut self) -> aa_actors::organization_state::Result<Option<u32>> {
        // GActor virtual +108 (4D1E80) returns +E8.
        Ok((self.owner_e8 != 0).then_some(self.owner_e8))
    }
    fn owner_word_e8(&self) -> aa_actors::organization_state::Result<u32> {
        Ok(self.owner_e8)
    }
    fn assign_number(&mut self, owner_e8: u32) -> aa_actors::organization_state::Result<u32> {
        let id: Vec<u16> = self.definition_id.encode_utf16().collect();
        let token = self
            .definitions
            .actor_definition_token(&id, self.pc)
            .map_err(name_error)?;
        let Some(crate::restore::RestoredRecord::Kingdom(kingdom)) = self
            .tail
            .registry
            .value_mut(aa_actors::ActorId::from_raw(owner_e8))
        else {
            return Err(name_error(format!(
                "4E4980 on owner {owner_e8:#010x}, which is not a restored Kingdom"
            )));
        };
        let shell = kingdom
            .runtime
            .as_mut()
            .and_then(|r| r.derived.as_mut())
            .ok_or_else(|| name_error("owner Kingdom has no derived state".into()))?;
        shell
            .definition_counts_mut()
            .assign(token)
            .map_err(|e| name_error(e.to_string()))
    }
}
pub fn finish_unlinked_actor<D>(
    base: &mut LoadShell<D, RestoredComponent>,
    tail: &ActorTail,
    definition_id: &[u16],
    canonical_definition_id: &str,
    definitions: &impl RestoreDefinitions,
    pc: Precision,
    names: &mut NameTail<'_>,
) -> std::result::Result<Option<ComponentSlot>, String> {
    let actor = base
        .actor
        .as_mut()
        .ok_or("ordinary Actor load shell disappeared")?;
    // Ground virtual+20 methods are580F70(-1),4C9FA0(Body) and581370(Properties).
    validate_supported_requirements(actor)?;
    // 4D9271: +E4 = actor+60[index] unless the saved index is -1; an empty
    // slot stores null. Indices outside the 31-entry table read other actor
    // words natively: refused.
    let active = if tail.current_component == -1 {
        None
    } else {
        let slot = usize::try_from(tail.current_component)
            .ok()
            .and_then(ComponentSlot::from_index)
            .ok_or_else(|| {
                format!(
                    "saved current component {} reads outside actor+60[31]",
                    tail.current_component
                )
            })?;
        actor.components.present(slot).then_some(slot)
    };
    if let Some(v) = tail.word_158 {
        actor.words.insert(0x158, v);
    }
    if tail.legacy_word_15c.is_some() {
        actor.words.insert(0x15c, u32::MAX);
    }
    if tail
        .word_160_source
        .as_ref()
        .is_some_and(|v| v.presence != 0)
    {
        return Err("Actor owner property-name lookup is unavailable".into());
    }
    if let Some(v) = tail.word_164.as_ref().and_then(|v| v.value) {
        actor.words.insert(0x164, v);
    }
    if let Some(key) = &tail.localized_name_key_utf16 {
        actor.string_dc_utf16 = key.clone();
        actor.string_e0_utf16 = definitions.localize(key)?;
    }
    for (o, v) in [
        (0x168, tail.word_168),
        (0x16c, tail.word_16c),
        (0x170, tail.word_170),
    ] {
        if let Some(v) = v {
            actor.words.insert(o, v);
        }
    }
    if tail.lists.as_ref().is_some_and(|v| {
        !v.list_118.values.is_empty()
            || !v.list_120.values.is_empty()
            || !v.list_128.values.is_empty()
            || !v.list_130.values.is_empty()
    }) {
        return Err("Actor owned linked-list restoration is unavailable".into());
    }
    if let Some(v) = tail.word_174 {
        actor.words.insert(0x174, v);
    }
    // 4D67F0 (aa_spatial::overlaps::refresh_overlaps): Thing+8 is read
    // once; bit 29 set, a false 4D10A0 (state bits 25/26 in +100 or 34/36 in
    // +104) or bit 10 clear return without touching the +110 list. Only the
    // remaining actors query the spatial index, which during Load holds the
    // records already initialized by 4FAE80's per-record InitExisting.
    let flags = base.thing.flags;
    let eligible = actor.components.state.low & 0x0600_0000 == 0
        && actor.components.state.high & 0x14 == 0;
    if flags & (1 << 29) == 0 && eligible && flags & (1 << 10) != 0 {
        // The query only prepends expiry +0.0 entries to the +110 list, which
        // is not saved (the tail restores +118..+130 only). Every surviving
        // record then runs InitExisting (4FAE80 -> virtual E8 = 4D7230, or
        // the deferred bit-7 path) whose own 4D67F0 first drops each entry
        // whose expiry is not after GWorld+E0, then queries again. With a
        // world time >= +0.0 the Load-time entries can never be read, so the
        // query is not repeated here. A negative or NaN time would keep them.
        let time = names.world_time_bits;
        let nonnegative = time & 0x8000_0000 == 0 && (time & 0x7fff_ffff) <= 0x7f80_0000;
        if !nonnegative {
            return Err(
                "Actor Load overlap refresh 4D67F0 with a negative world time keeps its entries (restore spatial index needed)"
                    .into(),
            );
        }
    }
    // 4D971B..4D99C2: +158 required object, saved MapString key/text and
    // numbered names (saved-components agent's actor_names port).
    let _ = definition_id;
    let mut number = actor.words.get(&0x170).copied().unwrap_or(0);
    let owner_e8 = actor.words.get(&0xe8).copied().unwrap_or(0);
    let required_158 = actor.words.get(&0x158).copied().unwrap_or(0);
    let actor_version = names.actor_version;
    let result = aa_actors::actor_names::finish(
        aa_actors::actor_names::SavedName {
            required_158,
            key: &mut actor.string_dc_utf16,
            text: &mut actor.string_e0_utf16,
            number: &mut number,
        },
        actor_version,
        &mut NameWorld {
            tail: names,
            definitions,
            definition_id: canonical_definition_id,
            owner_e8,
            pc,
        },
    );
    actor.words.insert(0x170, number);
    result.map_err(|e| e.to_string())?;
    Ok(active)
}
