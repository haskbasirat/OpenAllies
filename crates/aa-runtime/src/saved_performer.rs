//! Saved and newly created performers share one monotonic Ability identity arena.
//! This module binds the proven constructor to source definitions and the current
//! saved component-construction state. It does not authorize ability gameplay.
use crate::{
    actor_world::{self, PerformerConstruction},
    catalog::Catalog,
    definitions::Bank,
};
use aa_actors::{ActorId, components::Components};
use aa_body::F32;
use aa_combat::{
    ability::Ability,
    constructors::{self, OwnedPerformer},
};
use aa_mapworld::restore::RestoredComponent;
use aa_numeric::{Binary, Precision};
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct AbilityArena {
    entries: BTreeMap<u32, Ability>,
    next: u32,
}
impl Default for AbilityArena {
    fn default() -> Self {
        Self::new()
    }
}
impl AbilityArena {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            next: 1,
        }
    }
    pub fn entries(&self) -> &BTreeMap<u32, Ability> {
        &self.entries
    }
    pub fn next_handle(&self) -> u32 {
        self.next
    }
    pub fn get(&self, handle: u32) -> Result<&Ability, String> {
        self.entries
            .get(&handle)
            .ok_or_else(|| "unknown saved Ability handle".into())
    }
    pub fn get_mut(&mut self, handle: u32) -> Result<&mut Ability, String> {
        self.entries
            .get_mut(&handle)
            .ok_or_else(|| "unknown saved Ability handle".into())
    }
    pub fn allocate(&mut self, value: Ability) -> Result<u32, String> {
        Self::allocate_into(&mut self.entries, &mut self.next, value)
    }
    pub(crate) fn allocate_into(
        entries: &mut BTreeMap<u32, Ability>,
        next: &mut u32,
        value: Ability,
    ) -> Result<u32, String> {
        let id = *next;
        let after = id.checked_add(1).ok_or("Ability identity exhausted")?;
        if id == 0 || entries.contains_key(&id) {
            return Err("Ability allocator identity collision".into());
        }
        *next = after;
        entries.insert(id, value);
        Ok(id)
    }
    pub fn apply_saved(
        &mut self,
        handle: u32,
        saved: &aa_combat::commander::SavedAbility,
    ) -> Result<(), String> {
        aa_combat::commander::apply_ability(self.get_mut(handle)?, saved).map_err(|e| e.to_string())
    }
    /// The Ability destructor of a destroyed owner (4F5490 during the load).
    pub fn release(&mut self, handle: u32) -> Result<(), String> {
        self.entries
            .remove(&handle)
            .map(|_| ())
            .ok_or_else(|| "released Ability handle is not live".into())
    }
    pub fn into_parts(self) -> (BTreeMap<u32, Ability>, u32) {
        (self.entries, self.next)
    }
}
/// No actor/component state is copied. Native57F4D0 allocates actions first and
/// notifies current4D1350 before energy; earlier allocations/writes survive errors.
#[allow(clippy::too_many_arguments)]
pub fn construct(
    catalog: &Catalog<'_>,
    bank: &mut Bank,
    arena: &mut AbilityArena,
    owner: ActorId,
    definition_id: &str,
    components: &Components<RestoredComponent>,
    flags: &mut u32,
    words: &mut BTreeMap<u32, u32>,
    shell: SavedShell,
    pc: Precision,
) -> Result<OwnedPerformer, String> {
    let binding = catalog.actor_bindings(definition_id, pc)?;
    let input =
        actor_world::performer_inputs(&binding, catalog, bank).map_err(|e| e.to_string())?;
    let construction = catalog
        .actors
        .native_construction_flags(&binding.definition.proof.id, pc)
        .map_err(|e| e.to_string())?
        .map(|f| aa_actors::construction_actions::Definition {
            can_sell: f.can_sell,
            can_raze: f.can_raze,
            upgrade_count: f.upgrade_count,
        });
    let allow_sell_under_siege = match &catalog.settings {
        Some(settings) => Some(
            settings
                .allow_sell_under_siege()
                .map_err(|e| e.to_string())?
                .0,
        ),
        None => None,
    };
    let context = actor_world::SavedActionContext {
        construction,
        allow_sell_under_siege,
        object_byte_4c: shell.object_byte_4c,
        thing_word_c: shell.thing_word_c,
    };
    constructors::create(
        owner.raw(),
        &mut Factory {
            input: &input,
            arena,
            owner,
            components,
            flags,
            words,
            context,
        },
        pc,
    )
    .map_err(|e| e.to_string())
}
/// Load-shell words consumed by the saved factory's 4CFCC0 publication.
#[derive(Clone, Copy, Debug)]
pub struct SavedShell {
    pub object_byte_4c: u8,
    pub thing_word_c: u32,
}
struct Factory<'a> {
    input: &'a PerformerConstruction,
    arena: &'a mut AbilityArena,
    owner: ActorId,
    components: &'a Components<RestoredComponent>,
    flags: &'a mut u32,
    words: &'a mut BTreeMap<u32, u32>,
    context: actor_world::SavedActionContext,
}
impl constructors::Factory for Factory<'_> {
    fn local_ability_count(&self, _: u32) -> aa_combat::Result<usize> {
        Ok(self.input.abilities.len())
    }
    fn local_ability(&self, _: u32, index: usize) -> aa_combat::Result<constructors::LocalAbility> {
        self.input
            .abilities
            .get(index)
            .copied()
            .ok_or_else(|| aa_combat::Error::World("source local Ability index".into()))
    }
    fn allocate_ability(&mut self, v: Ability) -> aa_combat::Result<u32> {
        self.arena.allocate(v).map_err(aa_combat::Error::World)
    }
    fn notify_actor_abilities(&mut self, owner: u32) -> aa_combat::Result<()> {
        if owner != self.owner.raw() {
            return Err(aa_combat::Error::World(
                "saved Performer owner changed".into(),
            ));
        }
        actor_world::rebuild_saved_construction_actions(
            self.owner,
            self.components,
            self.flags,
            self.words,
            self.context,
        )
        .map_err(|e| aa_combat::Error::World(e.to_string()))
    }
    fn energy(&self, _: u32) -> aa_combat::Result<F32> {
        Ok(self.input.energy)
    }
    fn turret_count(&self, _: u32) -> aa_combat::Result<usize> {
        Ok(self.input.turret_center_angles.len())
    }
    fn turret_center_angle(&self, _: u32, index: usize) -> aa_combat::Result<Binary> {
        self.input
            .turret_center_angles
            .get(index)
            .cloned()
            .ok_or_else(|| aa_combat::Error::World("source Turret index".into()))
    }
    fn startup_target(&self) -> aa_combat::Result<[F32; 2]> {
        self.input
            .startup_target
            .ok_or_else(|| aa_combat::Error::World("native Turret startup globals missing".into()))
    }
    fn properties_component(&self, _: u32) -> aa_combat::Result<Option<u32>> {
        Err(aa_combat::Error::World(
            "saved factory does not run InitCreate".into(),
        ))
    }
    fn refresh_properties(&mut self, _: u32, _: bool) -> aa_combat::Result<()> {
        Err(aa_combat::Error::World(
            "saved factory does not run InitCreate".into(),
        ))
    }
}
