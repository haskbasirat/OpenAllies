//! The command mask `GActor::RebuildActions` 0x4D1350 (Actor +F0/+F4) over the battle.
//!
//! The contributors themselves are the exact, fixture-proven kernels of `aa_actors`
//! (`ability_flags` 0x571640 and 0x57B390, `recruit_actions` 0x582C40, `research_actions`
//! 0x583FC0, `structure_actions` 0x589A60, `denizen_actions` 0x569C70,
//! `construction_actions` 0x565140, `mobile_deployment_actions` 0x575FB0), run in the order
//! of 0x4D1350 by `ability_flags::rebuild`. What is here is what they read: the Actor
//! fields over `World`, and the two Mind contributors (Mind vt+B4) that `aa_actors` does
//! not hold: the regiment's 0x521E10 and the element's 0x530510.
//!
//! **What stands in for a native field.** aa-play keeps no Actor +100/+104: [`state_bits`]
//! derives the states the contributors read from what holds each one. The native mask is
//! a stored word, rebuilt whenever a state changes (0x4D64F0, 0x529D70); this one is
//! rebuilt when asked, so a change that does not itself rebuild natively (a morale that
//! falls while the regiment stays in one state, the bit 7 test at the end of 0x521E10) is
//! seen here at once and there only at the next rebuild. Thing flag 29 is read as no
//! MoverComponent (the Comrades trace: set on the corps and refineries, clear on trucks and
//! soldiers). The inventory attachment of 0x571640 (+14) has no host, so bit 30 stays off
//! as it does natively for an element holding nothing; flares (+C8) have no host, so bit 36
//! stays off; no stock data set has a Settlement or a Wall with a Mind of its own.
use crate::data::DefId;
use crate::morale::RegimentState;
use crate::world::{Activity, ActorId, OrgId, Unit, World};
use aa_actors::ability_flags::{self as flags, ActorFields, AttackGroup, ElementView, Error, Result};
use aa_actors::components::ComponentSlot as Slot;
use aa_actors::{ActorId as Key, construction_actions, denizen_actions, mobile_deployment_actions, recruit_actions, research_actions, structure_actions};
use aa_numeric::Precision;
use std::collections::BTreeMap;

/// Actor state indices (0x477FC0, `aa_actors::state::DESCRIPTORS`).
mod state {
    pub const MIND_ATTACKING: u32 = 0;
    pub const MIND_BOMBARD: u32 = 1;
    pub const MIND_BUILDING: u32 = 2;
    pub const MIND_CHASING: u32 = 3;
    pub const MIND_ENGAGED: u32 = 4;
    pub const MIND_EXHAUSTED: u32 = 5;
    pub const MIND_GUARDING: u32 = 6;
    pub const MIND_JOINING: u32 = 7;
    pub const MIND_MOVING: u32 = 8;
    pub const MIND_REGROUPING: u32 = 10;
    pub const MIND_REPAIRING: u32 = 11;
    pub const MIND_RESTING: u32 = 12;
    pub const MIND_RETREATING: u32 = 13;
    pub const MIND_ROUTING: u32 = 14;
    pub const MIND_REFORMING: u32 = 15;
    pub const MIND_EXPLORING: u32 = 16;
    pub const MIND_RAMPAGING: u32 = 17;
    pub const OUT_OF_SUPPLY: u32 = 19;
    pub const UNDER_SIEGE: u32 = 20;
    pub const CONSTRUCTION: u32 = 21;
    pub const DEAD: u32 = 23;
    pub const STRUCTURE_PACKING: u32 = 25;
    pub const STRUCTURE_UNPACKING: u32 = 26;
    pub const STRUCTURE_NORMAL: u32 = 27;
    pub const STRUCTURE_RESEARCHING: u32 = 28;
    pub const STRUCTURE_RAZING: u32 = 30;
    pub const STRUCTURE_RECRUITING: u32 = 31;
    pub const UNPACKED: u32 = 34;
    pub const MIND_PACKING: u32 = 35;
    pub const MIND_UNPACKING: u32 = 36;
}

/// The Actor +100/+104 a contributor reads, of an organization.
fn org_state_bits(world: &World, o: OrgId) -> u64 {
    let Some(org) = world.orgs.get(o as usize) else { return 0 };
    // 0x4D6F10 starts every thing on structure_normal; nothing moves an organization off it.
    let mut bits = 1u64 << state::STRUCTURE_NORMAL;
    let regiment = crate::morale::regiment_state(world, o);
    // What the current state reports at its vt+20. Unpack (0x51E2C0), Bombard (0x51D9E0),
    // Repair (0x51EC60), Construct (0x51DAE0) and Guard (0x51DC20) walking report their
    // Move state (Mind +74 == 2); a Kill of a move command reports the command's (0x51D8F0).
    let walking = crate::movement::org_move_active(world, o);
    let group4 = match regiment {
        RegimentState::Unpack | RegimentState::Bombard | RegimentState::Repair | RegimentState::Construct | RegimentState::Guard if walking => state::MIND_MOVING,
        RegimentState::Kill if world.combat.org(o).is_some_and(|c| c.kill_is_move) => state::MIND_MOVING,
        RegimentState::Sleep | RegimentState::Exhausted => state::MIND_RESTING,
        RegimentState::Move if crate::orders::retreating(world, o) => state::MIND_RETREATING,
        RegimentState::Move => state::MIND_MOVING,
        RegimentState::Kill => state::MIND_ATTACKING,
        RegimentState::Chasing => state::MIND_CHASING,
        RegimentState::Engage | RegimentState::Siege => state::MIND_ENGAGED,
        RegimentState::Guard => state::MIND_GUARDING,
        RegimentState::Explore => state::MIND_EXPLORING,
        RegimentState::Rout => state::MIND_ROUTING,
        RegimentState::Reform => state::MIND_REFORMING,
        RegimentState::Unpack => state::MIND_UNPACKING,
        RegimentState::Pack => state::MIND_PACKING,
        RegimentState::Bombard => state::MIND_BOMBARD,
        RegimentState::Repair => state::MIND_REPAIRING,
        RegimentState::Construct => state::MIND_BUILDING,
        RegimentState::Join => state::MIND_JOINING,
        RegimentState::Rampage => state::MIND_RAMPAGING,
        RegimentState::Regroup => state::MIND_REGROUPING,
    };
    bits |= 1 << group4;
    if world.combat.org(o).is_some_and(|c| c.exhausted_state) {
        bits |= 1 << state::MIND_EXHAUSTED;
    }
    if world.books.org(o).is_some_and(|b| b.out_of_supply) {
        bits |= 1 << state::OUT_OF_SUPPLY;
    }
    // 0x57CBA0: the leader slot's filling copies state 34 from the leader (0x4D1080).
    let leader = crate::ids::target_actor(world, Unit::Org(o));
    if leader.is_some_and(|l| actor_unpacked(world, l)) && !org.members.is_empty() {
        bits |= 1 << state::UNPACKED;
    }
    bits
}
/// State 34: the thing is what a truck unpacked into, which stands still.
fn actor_unpacked(world: &World, a: ActorId) -> bool {
    world.actors.get(a as usize).is_some_and(|x| world.data.def(x.def).packs_to.is_some())
}
/// The Actor +100/+104 a contributor reads, of an actor.
fn actor_state_bits(world: &World, a: ActorId) -> u64 {
    let Some(x) = world.actors.get(a as usize) else { return 0 };
    let mut bits = 0u64;
    let book = world.books.actor(a);
    if !x.alive {
        bits |= 1 << state::DEAD;
    }
    if book.is_some_and(|b| b.out_of_supply) {
        bits |= 1 << state::OUT_OF_SUPPLY;
    }
    if crate::supply::under_siege(world, a) {
        bits |= 1 << state::UNDER_SIEGE;
    }
    if book.is_some_and(|b| b.constructing) {
        bits |= 1 << state::CONSTRUCTION;
    }
    if actor_unpacked(world, a) {
        bits |= 1 << state::UNPACKED;
    }
    // Group 13 is exclusive: 0x4D6F10 starts every actor on structure_normal.
    let busy = crate::production::busy(world, a);
    let group13 = if busy && x.activity == Activity::Packing {
        state::STRUCTURE_PACKING
    } else if busy {
        state::STRUCTURE_UNPACKING
    } else if book.is_some_and(|b| b.razed) {
        state::STRUCTURE_RAZING
    } else {
        match book.and_then(|b| b.queue.items.first()) {
            Some(item) if item.research.is_some() => state::STRUCTURE_RESEARCHING,
            Some(_) => state::STRUCTURE_RECRUITING,
            None => state::STRUCTURE_NORMAL,
        }
    };
    bits |= 1 << group13;
    if busy {
        bits |= 1 << state::MIND_UNPACKING;
    }
    bits
}
/// Actor +100/+104 as far as the contributors read them (see the module docs).
pub fn state_bits(world: &World, unit: Unit) -> u64 {
    match unit {
        Unit::Org(o) => org_state_bits(world, o),
        Unit::Actor(a) => actor_state_bits(world, a),
    }
}

const ORG: u32 = 0x8000_0000;
fn key(unit: Unit) -> Key {
    match unit {
        Unit::Actor(a) => Key::from_raw(a + 1),
        Unit::Org(o) => Key::from_raw(ORG | o),
    }
}
fn unit(key: Key) -> Unit {
    let raw = key.raw();
    if raw & ORG != 0 { Unit::Org(raw & !ORG) } else { Unit::Actor(raw - 1) }
}

/// One rebuild: the masks asked of the things it touches. The native word of a member
/// is the one stored (0x57A030 reads vt+34); here each is rebuilt once, first time read.
struct Host<'a> {
    world: &'a World,
    masks: BTreeMap<Key, u64>,
    /// Being rebuilt, so a cycle reads what it has so far.
    open: Vec<Key>,
    /// The states of what this rebuild has read: nothing changes them during it.
    states: std::cell::RefCell<BTreeMap<Key, u64>>,
}

impl Host<'_> {
    fn states_of(&self, of: Key) -> u64 {
        *self.states.borrow_mut().entry(of).or_insert_with(|| state_bits(self.world, unit(of)))
    }
    fn mask(&mut self, of: Key) -> u64 {
        if let Some(bits) = self.masks.get(&of) {
            return *bits;
        }
        self.masks.insert(of, 0);
        self.open.push(of);
        // A kernel's own error is a dependency the world does not hold: a native
        // null read, which no mask of a living thing reaches. Its bits so far stand.
        let _ = flags::rebuild(self, of);
        self.open.pop();
        self.masks.get(&of).copied().unwrap_or(0)
    }
    fn present(&self, of: Key, slot: Slot) -> bool {
        match unit(of) {
            Unit::Org(o) => match slot {
                Slot::Organization | Slot::Mind => self.world.orgs.get(o as usize).is_some(),
                _ => false,
            },
            Unit::Actor(a) => self.world.actors.get(a as usize).is_some_and(|x| actor_component(self.world, a, x.def, slot)),
        }
    }
    fn actor_of(&self, of: Key) -> Result<ActorId> {
        match unit(of) {
            Unit::Actor(a) => Ok(a),
            Unit::Org(_) => Err(Error::Missing("an actor's component on an organization")),
        }
    }
    fn org_of(&self, of: Key) -> Result<OrgId> {
        match unit(of) {
            Unit::Org(o) => Ok(o),
            Unit::Actor(_) => Err(Error::Missing("an organization's component on an actor")),
        }
    }
}

/// `assemble_initial`: while a thing unpacks or is under construction it has only Construction,
/// Body, Properties, Wall, Structure, Detection, Element, Economy, MobileDeployment
/// and City; the rest come with the deferred end.
fn actor_component(world: &World, a: ActorId, def: DefId, slot: Slot) -> bool {
    let d = world.data.def(def);
    let late = world.books.actor(a).is_some_and(|b| b.constructing) || crate::production::busy(world, a);
    let name = match slot {
        Slot::Element => "ElementComponent",
        Slot::Wall => "WallComponent",
        Slot::Structure => "StructureComponent",
        Slot::Construction => "ConstructionComponent",
        Slot::MobileDeployment => return d.unpacks_to.is_some() || d.packs_to.is_some() || d.has_component("MobileDeploymentComponent"),
        Slot::Recruit if !late => "RecruitComponent",
        Slot::Research if !late => "ResearchComponent",
        Slot::Denizen if !late => "DenizenComponent",
        // 0x4CF480 case 4: Definition +174 bit 7 the regiment's, bit 23 the aircraft's,
        // bit 8 the element's.
        Slot::Mind => return d.has_component("AircraftComponent") || d.has_component("ElementComponent"),
        Slot::Organization | Slot::Settlement | Slot::Flare => return false,
        _ => return false,
    };
    d.has_component(name)
}

impl flags::World for Host<'_> {
    fn actor_fields(&self, of: Key) -> Result<ActorFields> {
        let u = unit(of);
        let thing_flags = match u {
            Unit::Org(_) => 0,
            Unit::Actor(a) => {
                let x = self.world.actors.get(a as usize).ok_or(Error::Missing("actor"))?;
                let mut f = 0;
                if !x.alive || x.removed {
                    f |= 1;
                }
                if !self.world.data.def(x.def).has_component("MoverComponent") {
                    f |= 1 << 29;
                }
                f
            }
        };
        Ok(ActorFields { thing_flags, state_bits: self.states_of(of), action_bits: self.masks.get(&of).copied().unwrap_or(0) })
    }
    fn set_action_bits(&mut self, of: Key, bits: u64) -> Result<()> {
        self.masks.insert(of, bits);
        Ok(())
    }
    fn has_component(&self, of: Key, slot: Slot) -> Result<bool> {
        Ok(self.present(of, slot))
    }
    fn element(&self, of: Key) -> Result<ElementView> {
        let a = self.actor_of(of)?;
        let org = self.world.actors.get(a as usize).and_then(|x| x.org);
        Ok(ElementView { owner: of, definition_present: false, organization: org.map(|o| key(Unit::Org(o))), ready: false })
    }
    fn organization_member_count(&self, of: Key) -> Result<usize> {
        Ok(self.world.orgs.get(self.org_of(of)? as usize).map_or(0, |o| o.members.len()))
    }
    fn organization_member(&self, of: Key, index: usize) -> Result<Option<Key>> {
        let org = self.world.orgs.get(self.org_of(of)? as usize).ok_or(Error::Missing("organization"))?;
        Ok(org.members.get(index).copied().filter(|&m| self.world.actors.get(m as usize).is_some_and(|x| x.alive && !x.removed)).map(|m| key(Unit::Actor(m))))
    }
    fn organization_member_44(&self, of: Key) -> Result<Option<Key>> {
        Ok(crate::ids::target_actor(self.world, Unit::Org(self.org_of(of)?)).map(|a| key(Unit::Actor(a))))
    }
    fn organization_reference_d0(&self, of: Key) -> Result<Key> {
        let org = self.world.orgs.get(self.org_of(of)? as usize).ok_or(Error::Missing("organization"))?;
        Ok(org.hq.map_or(Key::NULL, |h| key(Unit::Actor(h))))
    }
    fn valid_reference(&self, id: Key) -> Result<bool> {
        Ok(id != Key::NULL)
    }
    fn organization_group_count(&self, of: Key) -> Result<usize> {
        let org = self.world.orgs.get(self.org_of(of)? as usize).ok_or(Error::Missing("organization"))?;
        Ok(self.world.data.def(org.def).slots.len())
    }
    fn organization_attack_group(&self, of: Key, index: usize) -> Result<AttackGroup> {
        let o = self.org_of(of)?;
        let org = self.world.orgs.get(o as usize).ok_or(Error::Missing("organization"))?;
        let def = self.world.data.def(org.def);
        let locked = def.slot_locked.get(index).copied().unwrap_or(false);
        let element = self.world.books.org(o).and_then(|b| b.elements.as_ref()).and_then(|e| e.get(index).copied()).or_else(|| def.slots.get(index).map(|s| s.0));
        let class_flags = element.filter(|e| *e != crate::data::NO_DEF).map_or(Vec::new(), |e| self.world.data.def(e).weapons.iter().filter(|w| !w.defender).map(|w| w.flags).collect());
        Ok(AttackGroup { locked, class_flags })
    }
    fn owner(&mut self, of: Key) -> Result<Option<Key>> {
        // The kingdom stands for itself in the two owner reads below.
        Ok(Some(of))
    }
    /// Kingdom +218, the recruit singles' count (0x57B2E0's refusal 7).
    fn kingdom_word_218(&self, of: Key) -> Result<u32> {
        let kingdom = match unit(of) {
            Unit::Org(o) => self.world.orgs.get(o as usize).map(|x| x.kingdom),
            Unit::Actor(a) => self.world.actors.get(a as usize).and_then(|x| x.kingdom),
        };
        Ok(u32::from(kingdom.is_some()))
    }
    fn state_resupply(&self, state_index: u8) -> Result<bool> {
        let name = aa_actors::state::DESCRIPTORS.get(state_index as usize).ok_or(Error::Missing("state index"))?.name;
        Ok(self.world.data.actor_states.get(name).is_some_and(|s| s.resupply))
    }
    /// 0x4E1430 and 0x4E1E40 on the kingdom's HQ list (+2DC).
    fn owner_can_recruit(&mut self, of: Key, _actor: Key) -> Result<bool> {
        Ok(!crate::orders::hq_cycle_list(self.world, self.org_of(of)?).is_empty())
    }
    fn owner_recruit_count(&mut self, of: Key, _actor: Key) -> Result<i32> {
        let o = self.org_of(of)?;
        let current = self.world.orgs[o as usize].hq;
        Ok(crate::orders::hq_cycle_list(self.world, o).into_iter().filter(|h| Some(*h) != current).count() as i32)
    }
    fn action_available(&mut self, of: Key, index: u32) -> Result<bool> {
        if self.open.contains(&of) {
            return flags::available(self.masks.get(&of).copied().unwrap_or(0), index);
        }
        let bits = self.mask(of);
        flags::available(bits, index)
    }
    fn other_contributor(&mut self, of: Key, slot: Slot) -> Result<()> {
        match slot {
            Slot::Recruit => recruit_actions::contribute(self, of),
            Slot::Research => research_actions::contribute(self, of),
            Slot::Structure => structure_actions::contribute(self, of),
            Slot::Denizen => denizen_actions::contribute(self, of),
            Slot::Construction => construction_actions::contribute(self, of),
            Slot::MobileDeployment => mobile_deployment_actions::contribute(self, of, Precision::Double53),
            _ => Ok(()),
        }
    }
    fn mind_contributor(&mut self, of: Key) -> Result<()> {
        match unit(of) {
            Unit::Org(o) => regiment_mind(self, of, o),
            Unit::Actor(a) => {
                let def = self.world.actors.get(a as usize).map(|x| x.def).ok_or(Error::Missing("actor"))?;
                if self.world.data.def(def).has_component("AircraftComponent") {
                    // 0x518720.
                    for index in [5, 0, 2] {
                        set(self, of, index, true);
                    }
                    Ok(())
                } else {
                    element_mind(self, of, a)
                }
            }
        }
    }
    fn changed(&mut self, _of: Key) -> Result<()> {
        Ok(())
    }
}

fn state_type(world: &World, state_index: u8) -> Result<u32> {
    let name = aa_actors::state::DESCRIPTORS.get(state_index as usize).ok_or(Error::Missing("state index"))?.name;
    // ActorStateData +28: NONE 0, BUSY 1, QUEUE 2.
    Ok(world.data.actor_states.get(name).map_or(0, |s| {
        if s.kind.eq_ignore_ascii_case("busy") {
            1
        } else if s.kind.eq_ignore_ascii_case("queue") {
            2
        } else {
            0
        }
    }))
}

impl recruit_actions::World for Host<'_> {
    type Cursor = usize;
    type Definition = DefId;
    type Layout = ();
    fn recruit_owner(&self, recruit: Key) -> Result<Key> {
        Ok(recruit)
    }
    fn fields(&self, of: Key) -> Result<ActorFields> {
        flags::World::actor_fields(self, of)
    }
    fn set_action_bits(&mut self, of: Key, bits: u64) -> Result<()> {
        flags::World::set_action_bits(self, of, bits)
    }
    fn state_type(&self, state_index: u8) -> Result<u32> {
        state_type(self.world, state_index)
    }
    fn settlement_reference(&self, _: Key) -> Result<Option<Option<Key>>> {
        Ok(None)
    }
    /// 0x581F30: +10 if the list has an entry with ActorData +184, +11 if one without.
    fn recruit_enabled(&self, recruit: Key, location: bool) -> Result<u8> {
        let a = self.actor_of(recruit)?;
        let d = self.world.data.def(self.world.actors[a as usize].def);
        Ok(u8::from(d.recruits.iter().any(|&r| self.world.data.def(r).transport_actor.is_some() == location)))
    }
    fn recruit_list_head(&self, of: Key) -> Result<Option<usize>> {
        let a = self.actor_of(of)?;
        Ok((!self.world.data.def(self.world.actors[a as usize].def).recruits.is_empty()).then_some((a as usize) << 16))
    }
    fn recruit_entry(&self, cursor: &usize) -> Result<recruit_actions::Entry<DefId, ()>> {
        let d = self.world.data.def(self.world.actors[cursor >> 16].def);
        let definition = *d.recruits.get(cursor & 0xffff).ok_or(Error::Missing("recruit list entry"))?;
        Ok(recruit_actions::Entry { definition, location_class: self.world.data.def(definition).transport_actor.is_some(), default_layout: () })
    }
    fn next_recruit_entry(&self, cursor: &usize) -> Result<Option<usize>> {
        let d = self.world.data.def(self.world.actors[cursor >> 16].def);
        Ok(((cursor & 0xffff) + 1 < d.recruits.len()).then_some(cursor + 1))
    }
    fn eligibility(&mut self, recruit: Key, definition: &DefId, _: &()) -> Result<u32> {
        let a = self.actor_of(recruit)?;
        Ok(crate::production::recruit_requirements(self.world, a, *definition).code as u32)
    }
}

impl research_actions::World for Host<'_> {
    fn research_owner(&self, research: Key) -> Result<Key> {
        Ok(research)
    }
    fn state_bits(&self, of: Key) -> Result<u64> {
        Ok(self.states_of(of))
    }
    fn state_type(&self, state_index: u8) -> Result<u32> {
        state_type(self.world, state_index)
    }
    fn research_items_present(&self, of: Key) -> Result<bool> {
        let a = self.actor_of(of)?;
        Ok(!self.world.data.def(self.world.actors[a as usize].def).research.is_empty())
    }
    fn action_bits(&self, of: Key) -> Result<u64> {
        Ok(self.masks.get(&of).copied().unwrap_or(0))
    }
    fn set_action_bits(&mut self, of: Key, bits: u64) -> Result<()> {
        flags::World::set_action_bits(self, of, bits)
    }
}

impl structure_actions::World for Host<'_> {
    fn structure_owner(&self, structure: Key) -> Result<Key> {
        Ok(structure)
    }
    fn structure_reference(&self, of: Key) -> Result<Option<structure_actions::StructureReference>> {
        Ok(Some(structure_actions::StructureReference { owner: of, parent: None }))
    }
    fn settlement_reference(&self, _: Key) -> Result<Option<Option<Key>>> {
        Ok(None)
    }
    fn action_bits(&self, of: Key) -> Result<u64> {
        Ok(self.masks.get(&of).copied().unwrap_or(0))
    }
    fn set_action_bits(&mut self, of: Key, bits: u64) -> Result<()> {
        flags::World::set_action_bits(self, of, bits)
    }
    /// Structure +18: the production queue holds an item.
    fn queue_nonempty(&self, of: Key) -> Result<bool> {
        let a = self.actor_of(of)?;
        Ok(self.world.books.actor(a).is_some_and(|b| !b.queue.items.is_empty()))
    }
    fn rebuild_actions(&mut self, _: Key) -> Result<()> {
        Ok(())
    }
}

impl denizen_actions::World for Host<'_> {
    fn denizen_owner(&self, component: Key) -> Result<Key> {
        Ok(component)
    }
    fn fields(&self, of: Key) -> Result<ActorFields> {
        flags::World::actor_fields(self, of)
    }
    fn set_action_bits(&mut self, of: Key, bits: u64) -> Result<()> {
        flags::World::set_action_bits(self, of, bits)
    }
    fn denizen_byte(&self, component: Key, offset: u32) -> Result<u8> {
        let a = self.actor_of(component)?;
        // A Denizen not yet made (its InitCreate comes with the first update) holds zeros.
        let Some(d) = self.world.books.actor(a).and_then(|b| b.denizen.as_ref()) else { return Ok(0) };
        Ok(match offset {
            0x20 => u8::from(d.sally),
            0x21 => u8::from(d.escort),
            0x38 => d.behaviors as u8,
            0x3c => d.ready as u8,
            _ => return Err(Error::Missing("Denizen byte")),
        })
    }
    fn settlement_present(&self, _: Key) -> Result<bool> {
        Ok(false)
    }
    fn structure_parent(&self, of: Key) -> Result<Option<Option<Key>>> {
        Ok(self.present(of, Slot::Structure).then_some(None))
    }
    fn wall_reference(&self, of: Key) -> Result<Option<Option<Key>>> {
        Ok(self.present(of, Slot::Wall).then_some(None))
    }
}

impl construction_actions::World for Host<'_> {
    fn fields(&self, of: Key) -> Result<ActorFields> {
        flags::World::actor_fields(self, of)
    }
    fn set_action_bits(&mut self, of: Key, bits: u64) -> Result<()> {
        flags::World::set_action_bits(self, of, bits)
    }
    fn definition(&self, of: Key) -> Result<construction_actions::Definition> {
        let a = self.actor_of(of)?;
        let d = self.world.data.def(self.world.actors[a as usize].def);
        Ok(construction_actions::Definition { can_sell: d.can_sell, can_raze: d.can_raze, upgrade_count: 0 })
    }
    fn structure_parent(&self, of: Key) -> Result<Option<Option<Key>>> {
        Ok(self.present(of, Slot::Structure).then_some(None))
    }
    fn wall_reference(&self, of: Key) -> Result<Option<Option<Key>>> {
        Ok(self.present(of, Slot::Wall).then_some(None))
    }
    fn settlement_reference(&self, _: Key) -> Result<Option<Option<Key>>> {
        Ok(None)
    }
    fn properties_flags(&self, _: Key) -> Result<Option<u32>> {
        Ok(Some(0))
    }
    fn allow_sell_under_siege(&mut self) -> Result<bool> {
        // 0x565140: 0x4B2E60(L"AllowSellUnderSiege", 1).
        Ok(self.world.data.rule_ci("AllowSellUnderSiege").unwrap_or(1.0) != 0.0)
    }
    fn settlement_upgrades_ready(&mut self, _: Key) -> Result<bool> {
        Ok(false)
    }
}

impl mobile_deployment_actions::World for Host<'_> {
    fn fields(&self, of: Key) -> Result<ActorFields> {
        flags::World::actor_fields(self, of)
    }
    fn set_action_bits(&mut self, of: Key, bits: u64) -> Result<()> {
        flags::World::set_action_bits(self, of, bits)
    }
    fn has_pack_reference(&self, of: Key) -> Result<bool> {
        let a = self.actor_of(of)?;
        Ok(self.world.data.def(self.world.actors[a as usize].def).packs_to.is_some())
    }
    fn has_unpack_reference(&self, of: Key) -> Result<bool> {
        let a = self.actor_of(of)?;
        Ok(self.world.data.def(self.world.actors[a as usize].def).unpacks_to.is_some())
    }
    fn pack_health_min_bits(&self, of: Key) -> Result<u32> {
        let a = self.actor_of(of)?;
        Ok(self.world.data.def(self.world.actors[a as usize].def).pack_health_min.to_bits())
    }
    fn properties_flags(&self, _: Key) -> Result<Option<u32>> {
        Ok(Some(0))
    }
    fn body_health(&self, of: Key) -> Result<Option<mobile_deployment_actions::BodyHealth>> {
        let a = self.actor_of(of)?;
        let x = &self.world.actors[a as usize];
        if !self.world.data.def(x.def).has_component("BodyComponent") {
            return Ok(None);
        }
        let maximum = self.world.combat.actor(a).filter(|c| c.initialized && c.max_health > 0.0).map_or_else(|| crate::production::max_health(self.world, x.kingdom, x.def), |c| c.max_health);
        Ok(Some(mobile_deployment_actions::BodyHealth { health_bits: x.health.to_bits(), maximum_bits: maximum.to_bits() }))
    }
    fn has_mind(&self, of: Key) -> Result<bool> {
        Ok(self.present(of, Slot::Mind))
    }
}

fn set(host: &mut Host, of: Key, index: u32, value: bool) {
    let bits = host.masks.entry(of).or_default();
    if value {
        *bits |= 1 << index;
    } else {
        *bits &= !(1 << index);
    }
}

/// 0x4D12C0: Thing flag 29, or any of states 34, 26, 36, 25 (0x4D10A0).
fn immobile(host: &Host, of: Key) -> Result<bool> {
    let fields = flags::World::actor_fields(host, of)?;
    Ok(fields.thing_flags & 1 << 29 != 0 || fields.state_bits & (1 << state::UNPACKED | 1 << state::STRUCTURE_UNPACKING | 1 << state::MIND_UNPACKING | 1 << state::STRUCTURE_PACKING) != 0)
}

/// The regiment Mind's vt+B4, 0x521E10.
fn regiment_mind(host: &mut Host, of: Key, o: OrgId) -> Result<()> {
    let world = host.world;
    let org = world.orgs.get(o as usize).ok_or(Error::Missing("organization"))?;
    let regiment = crate::morale::regiment_state(world, o);
    // 0x578140, with what `crate::morale::state` holds of it.
    let status = world.combat.org(o).map_or(2, |c| crate::morale::state(c, org.morale));
    if regiment == RegimentState::Rout || status == 0 {
        for index in 0..13 {
            set(host, of, index, false);
        }
        return Ok(());
    }
    // +1CC is the engage state.
    let not_engage = regiment != RegimentState::Engage;
    let attackers = world.combat.org(o).is_some_and(|c| c.is_engaged());
    let sells = flags::available(host.masks.get(&of).copied().unwrap_or(0), 20)?;
    // Properties +1C bit 11 is off: no stock property sets it. 0x44E340 is the HQ at +D0.
    set(host, of, 10, not_engage && !attackers && !sells && org.hq.is_none());
    let immobile = immobile(host, of)?;
    set(host, of, 7, !immobile);
    set(host, of, 8, !immobile && !not_engage);
    if flags::available(host.masks.get(&of).copied().unwrap_or(0), 7)? && !not_engage && let Some(c) = world.combat.org(o) {
        // [B39AB0]+D8 is MoraleRetreatPenalty; 0x9F32E0 is 1.
        let penalty = world.data.rule_ci("MoraleRetreatPenalty").unwrap_or(0.0);
        if (org.morale as f64) < penalty as f64 + c.breakpoint as f64 + 1.0 {
            set(host, of, 7, false);
        }
    }
    Ok(())
}

fn ability_enabled(w: &crate::data::Weapon) -> bool {
    !w.defender
}

/// The element Mind's vt+B4, 0x530510. The ability tests read the definition's list:
/// 0x57E1C0 repair (0x4BE970), 0x57E280 flag bit 3 (a bombard), 0x57E230 any and 0x57E2F0
/// any without bit 3; a Performer's ability is enabled as its definition has it.
fn element_mind(host: &mut Host, of: Key, a: ActorId) -> Result<()> {
    let world = host.world;
    let x = world.actors.get(a as usize).ok_or(Error::Missing("actor"))?;
    let d = world.data.def(x.def);
    let thing_flags = flags::World::actor_fields(host, of)?.thing_flags;
    let free = thing_flags >> 29 & 1 == 0;
    set(host, of, 5, free);
    let attached = x.org.is_some();
    if attached {
        set(host, of, 2, free);
    }
    // And the rule [B39AB0]+108, on in every film's data.
    set(host, of, 9, free && attached);
    // Builder (Actor +108): the construct list.
    set(host, of, 4, attached && !d.builds.is_empty());
    let abilities = || d.weapons.iter().filter(|w| ability_enabled(w));
    // 0x4BE970: required target bit 7 and an effect of kind 3.
    set(host, of, 3, abilities().any(|w| w.targets_required & 128 != 0 && w.effects.iter().any(|e| e.kind == 3)));
    let indirect = abilities().any(|w| w.flags & 8 != 0);
    set(host, of, 1, indirect);
    let any = abilities().next().is_some();
    let mover = d.has_component("MoverComponent");
    let mut kill = mover && any;
    if kill && indirect {
        kill = abilities().any(|w| w.flags & 8 == 0);
    }
    set(host, of, 0, kill);
    set(host, of, 6, mover || any);
    // Bit 10: no Construction (+9C), not blocked, and an organization's HQ handle (+D0)
    // is not valid.
    let construction = d.has_component("ConstructionComponent");
    let hq = x.org.and_then(|o| world.orgs.get(o as usize)).and_then(|o| o.hq);
    set(host, of, 10, !construction && hq.is_none());
    // 0x575BB0(1) and 0x575C00(1).
    if flags::World::has_component(host, of, Slot::MobileDeployment)? {
        let unpack = mobile_deployment_actions::can_unpack(host, of, true)?;
        set(host, of, 11, unpack);
        let pack = mobile_deployment_actions::can_pack(host, of, true, Precision::Double53)?;
        set(host, of, 12, pack);
    }
    Ok(())
}

/// vt+150(kingdom, command, 1) on `unit`: vt+154 is the owner, then vt+20(1) 0x4CF120 (the
/// Element's parent stands for it; a lone actor with a Mover takes commands only with a
/// MobileDeployment, Recruit or Denizen), then the mask.
pub fn takes(world: &World, kingdom: u32, unit: Unit, index: u32) -> bool {
    let owner = match unit {
        Unit::Org(o) => world.orgs.get(o as usize).filter(|x| x.alive).map(|x| x.kingdom),
        Unit::Actor(a) => world.actors.get(a as usize).filter(|x| x.alive && !x.removed).and_then(|x| x.kingdom),
    };
    if owner != Some(kingdom) {
        return false;
    }
    if let Unit::Actor(a) = unit {
        let x = &world.actors[a as usize];
        let d = world.data.def(x.def);
        if x.org.is_none() && d.has_component("MoverComponent") && !actor_component(world, a, x.def, Slot::MobileDeployment) && !d.has_component("RecruitComponent") && !d.has_component("DenizenComponent") {
            return false;
        }
    }
    action_bits(world, unit) >> index & 1 != 0
}

/// The unit's command mask: 0x4D1350 (see the module docs).
pub fn action_bits(world: &World, unit: Unit) -> u64 {
    let of = key(unit);
    let alive = match unit {
        Unit::Org(o) => world.orgs.get(o as usize).is_some_and(|x| x.alive),
        Unit::Actor(a) => world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed),
    };
    if !alive {
        return 0;
    }
    Host { world, masks: BTreeMap::new(), open: Vec::new(), states: Default::default() }.mask(of)
}
