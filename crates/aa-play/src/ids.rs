//! Native thing ids (GTIDs) beside aa-play's own indices. Owner: core.
//!
//! The original names every thing (team, kingdom, map object, actor,
//! organization) by a GTID `generation << 16 | slot` from its GTIDManager
//! (4EFDB0 Add / 4EFEC0 / 4F0190 saved-id registration, 4EFBE0 ascending
//! enumeration). Film orders and selections name things by that slot. aa-play
//! keeps its `ActorId` / `OrgId` / `KingdomId` vector indices as the façade
//! and records the native id of each here, in `aa_actors::IdentityRegistry`:
//! 65,536 slots (slot 0 reserved), next-fit Add wrapping from 65535 to 1,
//! Remove increments the slot's generation, at most 0x8000 live from Add.
//!
//! How ids are given, as the original gives them:
//! - map load (0x4FC600): the registry starts as the map's IDMGR chunk left it
//!   (next-fit cursor, highest slot, generations) and every saved team,
//!   kingdom and placed thing is registered under its saved id (no cap);
//! - custom battle setup: the kingdoms of unused slots are destroyed with
//!   their things (Remove); then 4FDA70 walks the things in ascending slot
//!   order and queues every placeholder of a player kingdom; 4FC4A0 runs the
//!   object requests, then the actor requests, and each (4FBA50) first
//!   destroys the placeholder, then creates the replacement under the
//!   placeholder's own GTID (4FD7F0 / 4FD8C0 request the source id), so it
//!   keeps the saved id and the next-fit cursor stays at IDMGR's;
//! - CreateOrganization 0x4FA150: the organization first, then its leader,
//!   then every other slot (`production::spawn_organization` order);
//! - every later creation (recruits, unpacked trucks...) takes the next-fit
//!   id; a destroyed thing gives its id back when aa-play removes it
//!   (`removed` actors, dead organizations), swept at the end of each tick.
//!
//! aa-play objects the original also has but aa-play does not model (map
//! scenery, unowned placeholders, replacement objects) keep their ids as
//! `Thing::Placed`.
use crate::world::{ActorId, KingdomId, OrgId, Unit, World};
use aa_actors::registry::IdentityRegistry;

/// A native GTID (`generation << 16 | slot`).
pub type Gtid = aa_actors::registry::ActorId;

/// What a GTID names in aa-play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Thing {
    Actor(ActorId),
    Org(OrgId),
    Kingdom(KingdomId),
    /// A map kingdom slot not (yet) bound to an aa-play kingdom.
    MapKingdom(usize),
    /// A TEAMS record (team slot index).
    Team(usize),
    /// A map placed thing aa-play keeps as scenery (or not at all), by
    /// `GameMap::placed` index.
    Placed(usize),
    /// The trailer hitched to this actor (a GActor of its own natively,
    /// created by the truck's InitCreate 0x4D70F2; aa-play draws no trailer).
    Trailer(ActorId),
}

/// The GTID side table (`World::ids`).
#[derive(Debug)]
pub struct NativeIds {
    registry: IdentityRegistry<Thing>,
    actors: Vec<Option<Gtid>>,
    orgs: Vec<Option<Gtid>>,
    trailers: Vec<Option<Gtid>>,
    kingdoms: Vec<Option<Gtid>>,
    placed: Vec<Option<Gtid>>,
    map_kingdoms: Vec<Option<Gtid>>,
    /// Map kingdom slot of each aa-play kingdom.
    kingdom_slots: Vec<Option<usize>>,
    /// The saved id the next creation takes (a kept map thing).
    requested: Option<Gtid>,
    /// Things spawned now are restored from the map (no creation draws), not
    /// created (setup sets it around a kept map thing and its members).
    pub restore_scope: bool,
    /// Registrations and allocations that failed (stale generation, no ids
    /// left), for the load notes and the parity harness.
    pub notes: Vec<String>,
}

impl Default for NativeIds {
    fn default() -> Self {
        Self {
            registry: IdentityRegistry::new(),
            actors: Vec::new(),
            orgs: Vec::new(),
            trailers: Vec::new(),
            kingdoms: Vec::new(),
            placed: Vec::new(),
            map_kingdoms: Vec::new(),
            kingdom_slots: Vec::new(),
            requested: None,
            restore_scope: false,
            notes: Vec::new(),
        }
    }
}

fn set<T: Copy>(v: &mut Vec<Option<T>>, i: usize, x: Option<T>) {
    if v.len() <= i {
        v.resize(i + 1, None);
    }
    v[i] = x;
}

fn get<T: Copy>(v: &[Option<T>], i: usize) -> Option<T> {
    v.get(i).copied().flatten()
}

impl NativeIds {
    /// The registry after map load: the IDMGR state, then every saved team,
    /// kingdom and placed thing under its saved id. A map without IDMGR (a
    /// generated one) starts from the native Reset state with the cursor
    /// after its highest saved slot.
    pub fn load(map: &crate::map::GameMap, data: &crate::data::GameData) -> Self {
        let mut ids = Self::default();
        match &map.id_manager {
            Some(m) => match IdentityRegistry::from_load_metadata(m.next_slot, m.highest_slot, &m.generations) {
                Ok(r) => ids.registry = r,
                Err(e) => ids.notes.push(format!("IDMGR state refused ({e}); ids start from a reset registry")),
            },
            None => {
                let highest = map.placed.iter().map(|p| u32::from(Gtid::from_raw(p.serialized_id).slot())).max().unwrap_or(0);
                if let Ok(r) = IdentityRegistry::from_load_metadata((highest + 1).clamp(1, 65535), 0, &[]) {
                    ids.registry = r;
                }
            }
        }
        for (i, &raw) in map.team_ids.iter().enumerate() {
            ids.register(raw, false, Thing::Team(i));
        }
        for (s, slot) in map.slots.iter().enumerate() {
            if let Some(raw) = slot.serialized_id {
                let id = ids.register(raw, false, Thing::MapKingdom(s));
                set(&mut ids.map_kingdoms, s, id);
            }
        }
        for (i, p) in map.placed.iter().enumerate() {
            let is_actor = data.find(&p.thing_id).is_some_and(|d| data.def(d).actor_data);
            let id = ids.register(p.serialized_id, is_actor, Thing::Placed(i));
            set(&mut ids.placed, i, id);
        }
        ids
    }

    /// 4F0190 saved-id registration (SetGeneration only when the map has no
    /// IDMGR entry for the slot: the generations came from IDMGR).
    fn register(&mut self, raw: u32, is_actor: bool, thing: Thing) -> Option<Gtid> {
        if raw == 0 {
            return None;
        }
        let id = Gtid::from_raw(raw);
        if self.registry.generation_at(id.slot()) != id.generation() && usize::from(id.slot()) as i32 > self.registry.highest_slot() {
            let _ = self.registry.seed_saved_generation(id);
        }
        match self.registry.register_saved(id, is_actor, thing) {
            Ok(()) => Some(id),
            Err(e) => {
                self.notes.push(format!("saved id {raw:#x} of {thing:?} not registered: {e}"));
                None
            }
        }
    }

    fn remove(&mut self, id: Gtid) {
        if let Err(e) = self.registry.remove(id) {
            self.notes.push(format!("remove {:#x}: {e}", id.raw()));
        }
    }

    // ---- setup -----------------------------------------------------------

    /// The aa-play kingdom `kingdom` plays map kingdom slot `slot`: it takes
    /// the slot's saved id, or a new one when the map has none.
    pub fn bind_kingdom(&mut self, kingdom: KingdomId, slot: usize) {
        set(&mut self.kingdom_slots, kingdom as usize, Some(slot));
        let id = match get(&self.map_kingdoms, slot) {
            Some(id) => {
                if let Some(v) = self.registry.value_mut(id) {
                    *v = Thing::Kingdom(kingdom);
                }
                set(&mut self.map_kingdoms, slot, None);
                Some(id)
            }
            None => self.allocate(false, Thing::Kingdom(kingdom)),
        };
        set(&mut self.kingdoms, kingdom as usize, id);
    }

    /// Kingdom destruction (4F53D0 / 4E4130) of an unused map slot: the
    /// kingdom and every thing it owns give their ids back.
    pub fn destroy_map_kingdom(&mut self, map: &crate::map::GameMap, slot: usize) {
        if let Some(id) = get(&self.map_kingdoms, slot) {
            self.remove(id);
            set(&mut self.map_kingdoms, slot, None);
        }
        for (i, p) in map.placed.iter().enumerate() {
            if p.owner_slot == Some(slot as u32) {
                self.destroy_placed(i);
            }
        }
    }

    /// A placed thing is destroyed (a resolved placeholder, a thing of a
    /// destroyed kingdom).
    pub fn destroy_placed(&mut self, index: usize) {
        if let Some(id) = get(&self.placed, index) {
            self.remove(id);
            set(&mut self.placed, index, None);
        }
    }

    /// The next creation keeps placed thing `index`'s saved id (a map thing
    /// aa-play turns into an actor or organization).
    pub fn request_placed(&mut self, index: usize) {
        self.requested = get(&self.placed, index);
        set(&mut self.placed, index, None);
    }

    /// Forget an unused request (the creation did not happen): the id goes
    /// back to the placed thing.
    pub fn clear_request(&mut self, index: usize) {
        if let Some(id) = self.requested.take() {
            set(&mut self.placed, index, Some(id));
        }
    }

    // ---- creation and destruction ------------------------------------------

    fn allocate(&mut self, is_actor: bool, thing: Thing) -> Option<Gtid> {
        match self.registry.allocate(is_actor, thing) {
            Ok(id) => Some(id),
            Err(e) => {
                // 4EFDB0 "no IDs left": the original refuses the creation.
                self.notes.push(format!("{thing:?}: {e}"));
                None
            }
        }
    }

    fn take(&mut self, is_actor: bool, thing: Thing) -> Option<Gtid> {
        match self.requested.take() {
            Some(id) => {
                if let Some(v) = self.registry.value_mut(id) {
                    *v = thing;
                } else if let Err(e) = self.registry.register_saved(id, is_actor, thing) {
                    // 4EE460 requested id: 4F0190 AddID of the freed slot.
                    self.notes.push(format!("requested id {:#x} of {thing:?}: {e}", id.raw()));
                    return None;
                }
                Some(id)
            }
            None => self.allocate(is_actor, thing),
        }
    }

    /// 4FBA50 for placed thing `index`: the placeholder is destroyed (its id
    /// removed, generation + 1) and the next creation registers the very same
    /// id (4EE460 requested GTID: 4EFB70 sets the slot's generation back,
    /// 4F0190 adds it).
    pub fn replace_placed(&mut self, index: usize) {
        let Some(id) = get(&self.placed, index) else { return };
        self.remove(id);
        let _ = self.registry.seed_saved_generation(id);
        set(&mut self.placed, index, None);
        self.requested = Some(id);
    }

    /// Forget an unused request whose thing is gone (a failed replacement).
    pub fn drop_request(&mut self) {
        self.requested = None;
    }

    /// `World::spawn_actor`: the actor's id (4EE460: the requested saved id,
    /// else next-fit). False for a restored map thing (it was not created).
    pub fn created_actor(&mut self, actor: ActorId) -> bool {
        let id = self.take(true, Thing::Actor(actor));
        set(&mut self.actors, actor as usize, id);
        !self.restore_scope
    }

    /// The trailer `actor`'s InitCreate creates (next-fit, after the truck's
    /// own id).
    pub fn created_trailer(&mut self, actor: ActorId) {
        let id = self.allocate(true, Thing::Trailer(actor));
        set(&mut self.trailers, actor as usize, id);
    }

    /// An actor replaced in place (MobileDeployment unpack / pack): the old
    /// actor is destroyed (its id removed, its trailer with it) and the next
    /// creation registers the very same id (the requested GTID of 4EE460).
    pub fn replace_actor(&mut self, actor: ActorId) {
        if let Some(t) = get(&self.trailers, actor as usize) {
            self.remove(t);
            self.trailers[actor as usize] = None;
        }
        let Some(id) = get(&self.actors, actor as usize) else { return };
        self.remove(id);
        let _ = self.registry.seed_saved_generation(id);
        self.actors[actor as usize] = None;
        self.requested = Some(id);
    }

    /// A new organization actor (before its members). False for a restored
    /// map organization.
    pub fn created_org(&mut self, org: OrgId) -> bool {
        let id = self.take(true, Thing::Org(org));
        set(&mut self.orgs, org as usize, id);
        !self.restore_scope
    }

    /// Give back the ids of removed actors and dead organizations.
    pub fn sweep(&mut self, world_actors: &[crate::world::Actor], world_orgs: &[crate::world::Organization]) {
        for (i, a) in world_actors.iter().enumerate() {
            if a.removed
                && let Some(id) = get(&self.actors, i)
            {
                self.remove(id);
                self.actors[i] = None;
            }
            if a.removed
                && let Some(id) = get(&self.trailers, i)
            {
                self.remove(id);
                self.trailers[i] = None;
            }
        }
        for (i, o) in world_orgs.iter().enumerate() {
            if !o.alive
                && let Some(id) = get(&self.orgs, i)
            {
                self.remove(id);
                self.orgs[i] = None;
            }
        }
    }

    // ---- lookups ------------------------------------------------------------

    pub fn actor(&self, actor: ActorId) -> Option<Gtid> {
        get(&self.actors, actor as usize)
    }
    pub fn org(&self, org: OrgId) -> Option<Gtid> {
        get(&self.orgs, org as usize)
    }
    /// The GTID of the trailer hitched to `actor`.
    pub fn trailer(&self, actor: ActorId) -> Option<Gtid> {
        get(&self.trailers, actor as usize)
    }
    pub fn kingdom(&self, kingdom: KingdomId) -> Option<Gtid> {
        get(&self.kingdoms, kingdom as usize)
    }
    pub fn placed(&self, index: usize) -> Option<Gtid> {
        get(&self.placed, index)
    }
    /// The map kingdom slot aa-play kingdom `kingdom` plays.
    pub fn kingdom_slot(&self, kingdom: KingdomId) -> Option<usize> {
        get(&self.kingdom_slots, kingdom as usize)
    }
    /// What the live thing with this full id is (`None` when stale).
    pub fn thing(&self, gtid: u32) -> Option<Thing> {
        self.registry.get(Gtid::from_raw(gtid)).map(|e| e.value)
    }
    /// What the live thing in this 16-bit slot is (orders and selections
    /// carry the slot only).
    pub fn thing_at_slot(&self, slot: u16) -> Option<Thing> {
        let id = Gtid::from_raw(u32::from(self.registry.generation_at(slot)) << 16 | u32::from(slot));
        self.registry.get(id).map(|e| e.value)
    }
    /// The full id now live in `slot`.
    pub fn gtid_at_slot(&self, slot: u16) -> Option<Gtid> {
        let id = Gtid::from_raw(u32::from(self.registry.generation_at(slot)) << 16 | u32::from(slot));
        self.registry.get(id).map(|e| e.id)
    }
    /// Live ids in native enumeration order (4EFBE0, ascending slot).
    pub fn iter(&self) -> impl Iterator<Item = (Gtid, Thing)> + '_ {
        self.registry.iter().map(|e| (e.id, e.value))
    }
    pub fn live(&self) -> usize {
        self.registry.len()
    }
    pub fn next_slot(&self) -> u16 {
        self.registry.next_slot()
    }
    pub fn highest_slot(&self) -> i32 {
        self.registry.highest_slot()
    }
}

/// The unit a thing is ordered as: an organization, or an actor (a member
/// stands for itself; the caller decides whether to order its regiment).
pub fn unit(thing: Thing) -> Option<Unit> {
    match thing {
        Thing::Actor(a) => Some(Unit::Actor(a)),
        Thing::Org(o) => Some(Unit::Org(o)),
        // A trailer is ordered as its truck.
        Thing::Trailer(a) => Some(Unit::Actor(a)),
        _ => None,
    }
}

/// The aa-play kingdom of map kingdom index `index` (KINGDOMS +0x188, the
/// SessionPlayers kingdom index).
pub fn kingdom_by_index(world: &World, index: u32) -> Option<KingdomId> {
    let slot = world.map.slots.iter().position(|s| s.kingdom_index == Some(index))?;
    (0..world.kingdoms.len() as KingdomId).find(|&k| world.ids.kingdom_slot(k) == Some(slot))
}

/// The actor aa-play's `Attack` / `Follow` aim at for `unit`: the actor
/// itself, or the organization's living leader (else its first living
/// member).
pub fn target_actor(world: &World, unit: Unit) -> Option<ActorId> {
    match unit {
        Unit::Actor(a) => world.actor(a).map(|_| a),
        Unit::Org(o) => {
            let org = world.orgs.get(o as usize)?;
            let leader = world.data.def(org.def).leader_slot;
            let living = |m: &&ActorId| world.actors.get(**m as usize).is_some_and(|a| a.alive && !a.removed);
            org.members
                .iter()
                .filter(living)
                .find(|&&m| leader.is_some() && world.actors[m as usize].slot == leader)
                .or_else(|| org.members.iter().find(living))
                .copied()
        }
    }
}

/// End of tick: ids of things aa-play removed this tick go back.
pub fn sweep(world: &mut World) {
    // (scheduler) a destroyed thing's Updatables leave the world queues.
    let actors: Vec<ActorId> = (0..world.actors.len() as ActorId).filter(|&a| world.actors[a as usize].removed && world.ids.actor(a).is_some()).collect();
    let orgs: Vec<OrgId> = (0..world.orgs.len() as OrgId).filter(|&o| !world.orgs[o as usize].alive && world.ids.org(o).is_some()).collect();
    for a in actors {
        crate::scheduler::release_actor(world, a);
    }
    for o in orgs {
        crate::scheduler::release_org(world, o);
    }
    world.ids.sweep(&world.actors, &world.orgs);
}
