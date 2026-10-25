//! BuilderComponent::BuildActor 0x563160 on the aa-production port
//! (`construction::build_actor`): the structure an engineer places for the
//! construct order (`crate::combat::tasks`). Owner: core.
//!
//! The placement adjustment 0x6939B0 (a tile_locked structure snaps to the
//! half-cell grid, an axis_locked one faces native -Y), the affordability
//! 0x562FD0 (construct_error_affording), the kingdom restrictions 0x4E1910,
//! the creation 0x4F9C10 with the actor under construction (state 21; the
//! Body InitCreate 0x562310 sets its health to 1.0), then the cost quote and
//! the debit of the owner (no second affordability check).
//!
//! Compare against the original: the restriction profiles of missions are
//! not loaded (a skirmish has none); the cost is the definition's quote
//! without the builder's modifier context; a placement the footprint test of
//! `crate::deploy` refuses stands for a failed 0x4F9C10 creation.
use crate::books::{ConstructRefusal, Notice};
use crate::data::DefId;
use crate::world::{ActorId, Event, KingdomId, World};
use aa_production::construction::{BuildActorWorld, BuildParams, BuildRules, Construction, PlacementMessages};
use glam::Vec2;

type R<T> = aa_production::Result<T>;

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}
fn floats(v: &[u32]) -> Vec<f32> {
    v.iter().map(|x| f32::from_bits(*x)).collect()
}

struct Host<'a> {
    world: &'a mut World,
    builder: ActorId,
    created: Option<ActorId>,
}

impl Host<'_> {
    fn kingdom(&self) -> Option<KingdomId> {
        self.world.actors.get(self.builder as usize).and_then(|a| a.kingdom)
    }
    fn refused(&mut self, def: u32, reason: ConstructRefusal) {
        if let Some(kingdom) = self.kingdom() {
            self.world.books.notices.push(Notice::ConstructRefused { kingdom, def, reason });
        }
    }
}

impl Construction for Host<'_> {
    fn construction_kingdom(&mut self) -> R<Option<u32>> {
        Ok(self.kingdom())
    }
    fn construction_cost(&mut self, definition: u32) -> R<Vec<u32>> {
        Ok(bits(&crate::production::quote(self.world, definition, crate::production::Vector::Cost)))
    }
    fn construction_can_afford(&mut self, kingdom: u32, cost: &[u32]) -> R<bool> {
        Ok(crate::production::can_afford(self.world, kingdom, &floats(cost)))
    }
    fn construction_debit(&mut self, kingdom: u32, cost: &[u32]) -> R<()> {
        crate::production::credit(self.world, kingdom, &floats(cost), -1.0);
        Ok(())
    }
    fn construction_credit(&mut self, kingdom: u32, cost: &[u32]) -> R<()> {
        crate::production::credit(self.world, kingdom, &floats(cost), 1.0);
        Ok(())
    }
    fn notify_construct_unaffordable(&mut self, definition: u32, cost: &[u32], _kingdom: u32) -> R<()> {
        self.refused(definition, ConstructRefusal::Unaffordable { cost: floats(cost) });
        Ok(())
    }
    fn notify_upgrade_unaffordable(&mut self, _definition: u32, _cost: &[u32], _kingdom: Option<u32>) -> R<()> {
        Ok(())
    }
    fn has_structure(&mut self) -> R<bool> {
        Ok(false)
    }
    fn state_active(&mut self, _state: u32) -> R<bool> {
        Ok(false)
    }
    fn actor_flags(&mut self) -> R<u64> {
        Ok(0)
    }
    fn definition_flags(&mut self, _definition: u32) -> R<u32> {
        Ok(0)
    }
    /// 0x4E1910: no mission restriction profiles in a skirmish.
    fn kingdom_allows(&mut self, _kingdom: u32, _definition: u32) -> R<bool> {
        Ok(true)
    }
    fn upgrade_list(&mut self) -> R<Vec<u32>> {
        Ok(Vec::new())
    }
    fn construction_begun(&mut self, _kingdom: Option<u32>, _definition: u32) -> R<()> {
        Ok(())
    }
    fn apply_upgrade_modifiers(&mut self, _definition: u32) -> R<()> {
        Ok(())
    }
    fn construction_actor_state(&mut self, _state: u32, _enabled: bool) -> R<()> {
        Ok(())
    }
    fn destroyed(&mut self) -> R<bool> {
        Ok(false)
    }
    fn destroy_actor(&mut self) -> R<()> {
        Ok(())
    }
}

impl PlacementMessages for Host<'_> {
    fn notify_enemies_nearby(&mut self, _kingdom: Option<u32>) -> R<()> {
        Ok(())
    }
    fn notify_creation_failed(&mut self, definition: u32, _kingdom: Option<u32>) -> R<()> {
        self.refused(definition, ConstructRefusal::CreationFailed);
        Ok(())
    }
}

impl BuildActorWorld for Host<'_> {
    /// ActorData +0x1C bit 16 (snap) for a tile_locked footprint, bit 17
    /// (facing (0, -1)) for an axis_locked one.
    fn build_rules(&mut self, definition: u32) -> R<BuildRules> {
        let d = self.world.data.def(definition);
        let flags = u32::from(d.tile_locked) << 16 | u32::from(d.axis_locked) << 17;
        Ok(BuildRules { placement_flags: flags, check_enemies: false, site_class: 0 })
    }
    fn snaps_to_grid(&mut self, definition: u32) -> R<bool> {
        Ok(self.world.data.def(definition).blocking_size.is_some())
    }
    /// 0x46EE00: floor(x / (cell * 0.5) + 0.5) * cell * 0.5.
    fn snap_location(&mut self, _definition: u32, location: [u32; 2]) -> R<[u32; 2]> {
        let half = self.world.map.cell_size * 0.5;
        let snap = |v: f32| ((v / half + 0.5).floor() * half).to_bits();
        Ok([snap(f32::from_bits(location[0])), snap(f32::from_bits(location[1]))])
    }
    fn build_enemies_nearby(&mut self, _definition: u32, _location: [u32; 2], _facing: [u32; 2], _owner: Option<u32>) -> R<bool> {
        Ok(false)
    }
    fn find_site(&mut self, _definition: u32, _location: [u32; 2]) -> R<Option<u32>> {
        Ok(None)
    }
    fn log_missing_site(&mut self, _definition: u32, _location: [u32; 2]) -> R<()> {
        Ok(())
    }
    fn builder_field_28(&mut self) -> R<u32> {
        Ok(0)
    }
    fn create_actor(&mut self, params: &BuildParams) -> R<Option<u32>> {
        let def = params.definition;
        let at = Vec2::new(f32::from_bits(params.location[0]), f32::from_bits(params.location[1]));
        let dir = Vec2::new(f32::from_bits(params.facing[0]), f32::from_bits(params.facing[1]));
        let facing = if dir.length_squared() > 1e-12 { dir.y.atan2(dir.x) } else { 0.0 };
        if crate::deploy::spot_clear(self.world, def, at, None).is_err() {
            return Ok(None);
        }
        let kingdom = params.owner;
        let a = crate::production::spawn_single(self.world, def, kingdom, at, facing);
        // State 21 at creation; the Body InitCreate 0x562310 -> 0x561E50(1.0).
        self.world.books.actor_mut(a).constructing = true;
        self.world.actors[a as usize].health = 1.0;
        if let Some(k) = kingdom {
            self.world.events.push(Event::ConstructionStarted { actor: a, kingdom: k });
        }
        self.created = Some(a);
        Ok(Some(a))
    }
}

/// 0x563160 by `builder` for a structure of `def` at `at` facing `facing`
/// (radians): the structure under construction, or None when refused (the
/// refusal is a `Notice::ConstructRefused`).
pub fn build_actor(world: &mut World, builder: ActorId, def: DefId, at: Vec2, facing: f32) -> Option<ActorId> {
    let location = [at.x.to_bits(), at.y.to_bits()];
    let dir = [facing.cos().to_bits(), facing.sin().to_bits()];
    let mut host = Host { world, builder, created: None };
    match aa_production::construction::build_actor(def, location, dir, 0, &mut host) {
        Ok(Some(_)) => host.created,
        _ => None,
    }
}
