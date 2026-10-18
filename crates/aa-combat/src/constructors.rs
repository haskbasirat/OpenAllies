//! Concrete native Performer constructor57F4D0 and InitCreate57DB90.
use crate::{Error, Result, ability::Ability, performer::Performer};
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct LocalAbility {
    pub definition: u32,
    pub local_definition: u32,
    pub main_power: F32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Turret {
    pub current_angle: F32,
    pub target_angle: F32,
    pub time: F32,
    pub pending: bool,
    pub target: [F32; 3],
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedPerformer {
    pub state: Performer,
    pub maximum_mana: F32,
    pub turrets: Vec<Turret>,
    pub turret_update_pending: bool,
    pub word60: u32,
}
/// Definition objects remain valid across construction. Action handles are
/// arena identities; allocation layout/capacity are not game-state semantics.
/// Errors after published action allocation do not promise a rollback.
pub trait Factory {
    fn local_ability_count(&self, owner: u32) -> Result<usize>;
    fn local_ability(&self, owner: u32, index: usize) -> Result<LocalAbility>;
    fn allocate_ability(&mut self, ability: Ability) -> Result<u32>;
    /// Original4D1350, after action instances but before the energy copy.
    fn notify_actor_abilities(&mut self, owner: u32) -> Result<()>;
    fn energy(&self, owner: u32) -> Result<F32>;
    fn turret_count(&self, owner: u32) -> Result<usize>;
    /// Native45DC30 result; its subtraction remains extended before comparison.
    fn turret_center_angle(&self, owner: u32, index: usize) -> Result<Binary>;
    /// Source startup globals B279C0/B279C4, supplied explicitly.
    fn startup_target(&self) -> Result<[F32; 2]>;
    fn properties_component(&self, owner: u32) -> Result<Option<u32>>;
    /// Native581420(properties,0), not a fabricated successful initialization.
    fn refresh_properties(&mut self, properties: u32, immediate: bool) -> Result<()>;
}
fn n(f: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(f.0)?)
}

pub fn create(
    owner: u32,
    world: &mut impl Factory,
    precision: Precision,
) -> Result<OwnedPerformer> {
    let mut abilities = Vec::new();
    let mut index = 0;
    while index < world.local_ability_count(owner)? {
        let input = world.local_ability(owner, index)?;
        let handle = world.allocate_ability(Ability::new(
            input.definition,
            Some(input.local_definition),
            input.main_power,
        ))?;
        abilities
            .try_reserve(1)
            .map_err(|e| Error::World(format!("ability list allocation failed: {e}")))?;
        abilities.push(handle);
        index += 1;
    }
    world.notify_actor_abilities(owner)?;
    let energy = world.energy(owner)?;
    let mut turrets = Vec::new();
    let mut index = 0;
    while index < world.turret_count(owner)? {
        let angle = world
            .turret_center_angle(owner, index)?
            .sub(&n(F32(0x3fc9_0fdb))?, precision)?;
        let mut stored = F32(angle.store_f32()?);
        if angle.compare(&Binary::from_i32(0)) == Ordering::Less {
            stored = F32(n(stored)?
                .add(&n(F32(0x40c9_0fdb))?, precision)?
                .store_f32()?);
        }
        let xy = world.startup_target()?;
        turrets
            .try_reserve(1)
            .map_err(|e| Error::World(format!("turret list allocation failed: {e}")))?;
        turrets.push(Turret {
            current_angle: stored,
            target_angle: stored,
            time: F32::ZERO,
            pending: false,
            target: [xy[0], xy[1], F32::ZERO],
        });
        index += 1;
    }
    Ok(OwnedPerformer {
        state: Performer {
            owner,
            stage: 3,
            abilities,
            mana: energy,
            target_id: 0,
            target_position: [F32::ZERO; 2],
            active_ability: None,
            active_index: -1,
            started_at: F32::ZERO,
            animation_length: F32::ZERO,
            event_kind: 0,
            animation: None,
            turret_index: -1,
        },
        maximum_mana: energy,
        turrets,
        turret_update_pending: false,
        word60: 0,
    })
}
/// Called in the actor's ordinary ascending component InitCreate phase.
pub fn init_create(owner: u32, world: &mut impl Factory) -> Result<bool> {
    if let Some(properties) = world.properties_component(owner)? {
        world.refresh_properties(properties, false)?;
    }
    Ok(true)
}
