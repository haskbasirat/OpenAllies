//! Battle state shared by every system. Shared file: change only by agreement
//! (add fields, never repurpose them).
use crate::data::{DefId, GameData, Resources};
use crate::map::GameMap;
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub type KingdomId = u32;
/// Index into `World::actors`; never reused within a battle.
pub type ActorId = u32;
/// Index into `World::orgs`; never reused within a battle.
pub type OrgId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Controller {
    Human,
    /// Computer opponent with the original difficulty index.
    Computer(u8),
    /// A map's independent kingdom (cities, farms): never thinks.
    Independent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Kingdom {
    pub id: KingdomId,
    pub name: String,
    pub nation: String,
    pub team: u32,
    pub color: [u8; 3],
    pub controller: Controller,
    pub stockpile: Resources,
    /// Per-minute rates last computed by the economy (for the HUD).
    pub income: Resources,
    pub unit_limit: f32,
    pub units_used: f32,
    pub defeated: bool,
    /// A map's independent kingdom (header "independent_N", native Kingdom
    /// +0x190 == 1): owns cities and other neutral buildings, is not a
    /// player, never wins or loses, is drawn neutral.
    #[serde(default)]
    pub independent: bool,
}

/// What an organization or a single actor has been told to do.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Order {
    #[default]
    Idle,
    Move { to: Vec2, attack_move: bool },
    Attack { target: ActorId },
    /// Truck driving to a spot to unpack into its building.
    Deploy { at: Vec2, facing: f32 },
}

/// Presentation hint for the renderer (which original clip to play).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Activity {
    #[default]
    Idle,
    Moving,
    Attacking,
    Dying,
    Dead,
    Packing,
    Unpacking,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Motion {
    pub velocity: Vec2,
    pub speed: f32,
    /// Remaining waypoints (native ground points), nearest first.
    pub path: Vec<Vec2>,
    /// Where formation or orders want this actor to stand.
    pub goal: Option<Vec2>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Combat {
    pub target: Option<ActorId>,
    /// Seconds until each weapon (Def::weapons order) can fire again.
    pub recharge: Vec<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Actor {
    pub id: ActorId,
    pub def: DefId,
    pub kingdom: Option<KingdomId>,
    /// Native position: X/Y ground, Z height.
    pub pos: Vec3,
    /// Facing angle in radians about Z (0 = native +X).
    pub facing: f32,
    pub health: f32,
    pub alive: bool,
    /// Removed from the battle (corpse gone, truck unpacked into a building...).
    pub removed: bool,
    pub org: Option<OrgId>,
    pub slot: Option<u16>,
    pub motion: Motion,
    pub combat: Combat,
    pub activity: Activity,
    /// Battle time (s) when `activity` last changed, for animation phase.
    pub activity_since: f32,
    pub order: Order,
    /// Production queue of a building/HQ: (def, seconds left).
    pub queue: Vec<(DefId, f32)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Organization {
    pub id: OrgId,
    pub def: DefId,
    pub kingdom: KingdomId,
    pub members: Vec<ActorId>,
    /// HQ building this regiment is attached to (supply), if any.
    pub hq: Option<ActorId>,
    pub order: Order,
    /// Formation centre and facing the members hold their slots around.
    pub anchor: Vec2,
    pub heading: f32,
    pub morale: f32,
    pub experience: f32,
    pub alive: bool,
}

/// Player input and computer-opponent decisions, applied at the start of a tick.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Command {
    Move { kingdom: KingdomId, units: Vec<Unit>, to: Vec2, attack_move: bool },
    Attack { kingdom: KingdomId, units: Vec<Unit>, target: ActorId },
    Stop { kingdom: KingdomId, units: Vec<Unit> },
    Recruit { kingdom: KingdomId, building: ActorId, def: DefId },
    CancelRecruit { kingdom: KingdomId, building: ActorId, index: usize },
    Deploy { kingdom: KingdomId, truck: ActorId, at: Vec2, facing: f32 },
    Pack { kingdom: KingdomId, building: ActorId },
    /// Research a technology (`production::research`, native 0x584790):
    /// `property` is the technology property IDS the building lists.
    Research { kingdom: KingdomId, building: ActorId, property: String },
    /// Attach a regiment to another HQ (`production::attach`, 0x574C10).
    Attach { kingdom: KingdomId, org: OrgId, hq: ActorId },
    // ---- appended by the UI owner (HUD commands, `crate::player_orders`) ----
    /// The formation buttons (actor command set_formation 0x21): index into
    /// the organization layout's `Def::formations`.
    SetFormation { kingdom: KingdomId, orgs: Vec<OrgId>, formation: usize },
    /// set_rally_point (0x10): the building's recruits move to `at`.
    SetRallyPoint { kingdom: KingdomId, building: ActorId, at: Vec2 },
    /// sell (0x14): remove the structure for SellFactor of its cost.
    Sell { kingdom: KingdomId, building: ActorId },
    /// hq_detach (0x20): the organization leaves its HQ.
    Detach { kingdom: KingdomId, org: OrgId },
    /// disband (10): the selection leaves the battle for good.
    Disband { kingdom: KingdomId, units: Vec<Unit> },
    // ---- appended by the combat owner (special operations) ----
    /// A general's faction ability (KKC_FactionUseAbilityOrder `61E190` ->
    /// `4CDE00`, `crate::special_ops`): `ability` indexes the kingdom's
    /// general's abilities (the HUD's "UseFactionAbility %i").
    UseAbility { kingdom: KingdomId, ability: u32, target: Option<AbilityTarget> },
    // ---- appended by the movement owner ----
    /// The move command aimed at an actor (ActorCommand move 5 with a
    /// target: a right-click on an actor, GuessCommandData `0x4DA570`;
    /// CAIStateMove activation `0x520320`): the units follow the target as
    /// it moves until it is in range, then stop. Their order reads
    /// `Order::Move` toward the target's latest position.
    Follow { kingdom: KingdomId, units: Vec<Unit>, target: ActorId },
}

/// Where a faction ability is aimed: a map point, or a clicked actor.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum AbilityTarget {
    Point(Vec2),
    Actor(ActorId),
}

/// What a player selects and orders: a whole regiment or a single actor
/// (building, truck, HQ).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Unit {
    Org(OrgId),
    Actor(ActorId),
}

/// Things that happened this tick, for the HUD message log, sound and effects.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Recruited { kingdom: KingdomId, org: Option<OrgId>, actor: Option<ActorId> },
    Fired { shooter: ActorId, target: ActorId, weapon: u32 },
    Hit { target: ActorId, damage: f32 },
    Died { actor: ActorId },
    OrgDestroyed { org: OrgId },
    Deployed { building: ActorId },
    Routed { org: OrgId },
    KingdomDefeated { kingdom: KingdomId },
    Victory { team: u32 },
    /// A command was refused, with the original reason when known.
    Refused { kingdom: KingdomId, reason: String },
    /// An organization reached a new experience level (index into its
    /// ExperienceComponent levels; the original's promotion notification).
    Promoted { org: OrgId, level: u32 },
    /// A capturable actor (city) reached zero health and changed owner
    /// (`4D5230`; health set to CaptureHealthFactor of its maximum).
    Captured { actor: ActorId, kingdom: KingdomId },
    // ---- appended by the combat owner (special operations) ----
    /// A general's ability was used (`4C9550` phase 0): its IDS, the
    /// (map-clamped) point and the clicked actor.
    SpecialOperation { kingdom: KingdomId, ability: String, at: Vec2, target: Option<ActorId> },
    /// DESCRIBE_ECONOMY (effect 9): `kingdom` was shown the economy of `of`.
    EconomyDescribed { kingdom: KingdomId, of: KingdomId },
}

pub struct World {
    pub data: Arc<GameData>,
    pub map: Arc<GameMap>,
    pub tick: u64,
    /// Battle time in seconds (tick * DT).
    pub time: f32,
    pub kingdoms: Vec<Kingdom>,
    pub actors: Vec<Actor>,
    pub orgs: Vec<Organization>,
    pub events: Vec<Event>,
    pub winner: Option<u32>,
    /// Core-owner side state (economy books, queues, supply, deployment).
    pub books: crate::books::Books,
    /// Movement-owner side state (move orders, element paths, Mover fields,
    /// path request queue); read it through `crate::movement` functions.
    pub movement: crate::movement::MovementState,
    /// Combat-owner side state (engage lists, morale, routing, experience
    /// levels, corpses); read it through `crate::combat` / `crate::morale`.
    pub combat: crate::combat::CombatState,
    /// Opponent-owner side state (SAI data, players, goals, schedules);
    /// install the original SAI data with `crate::ai::attach`.
    pub ai: crate::ai::AiState,
    // ---- appended by the UI/fog owner ----
    /// Fog of war: per-kingdom tile visibility, detection words and
    /// memory; read it through `crate::fog` functions.
    pub fog: crate::fog::FogState,
    /// HUD player orders (formations, rally points); read it through
    /// `crate::player_orders` functions.
    pub player_orders: crate::player_orders::PlayerOrders,
    // ---- appended by the combat owner ----
    /// Generals' special operations (commanders, experience, enchantments);
    /// install the generals data with `crate::special_ops::attach`.
    pub special_ops: crate::special_ops::SpecialOpsState,
    // ---- appended by the core owner (byte-exact roadmap item 2) ----
    /// The world random stream, GWorld+0xD4: LCG 0x85D840 seeded by
    /// KKC_Sim::CreateGWorld 0x76AA90 with 0x85D880(creator seed). Draw it
    /// through `random` / `random_u32` / `random_below` (each logs the
    /// GetRNG 0x4F02B0 getter entry).
    pub native_rng: aa_simulation::NativeRng,
    /// The world clock fields of 0x4F9970: tick, begin and end time as
    /// `f32(i32 tick * f32(1/16))`. `tick` and `time` mirror it.
    pub clock: aa_simulation::NativeClock,
    /// The simulation S-log emulation and per-tick digests.
    pub trace: crate::trace::Trace,
    // ---- appended by the core owner (byte-exact roadmap item 5) ----
    /// Native GTIDs of the actors, organizations, kingdoms and map things
    /// (`crate::ids`).
    pub ids: crate::ids::NativeIds,
    // ---- appended by the core owner (byte-exact roadmap item 10) ----
    /// The five world queues of 0x4F9970 (`crate::scheduler`).
    pub sched: crate::scheduler::Scheduler,
}

impl World {
    pub fn empty(data: Arc<GameData>, map: Arc<GameMap>, seed: u64) -> Self {
        let ids = crate::ids::NativeIds::load(&map, &data);
        let mut world = Self {
            data,
            map,
            tick: 0,
            time: 0.0,
            kingdoms: Vec::new(),
            actors: Vec::new(),
            orgs: Vec::new(),
            events: Vec::new(),
            winner: None,
            books: Default::default(),
            movement: Default::default(),
            combat: Default::default(),
            ai: Default::default(),
            fog: Default::default(),
            player_orders: Default::default(),
            special_ops: Default::default(),
            native_rng: aa_simulation::NativeRng::default(),
            clock: aa_simulation::NativeClock::stock(),
            trace: Default::default(),
            ids,
            sched: crate::scheduler::Scheduler::new(),
        };
        world.create_rng(seed as u32);
        world
    }
    /// KKC_Sim::CreateGWorld 0x76AA90: the getter logs the default state,
    /// then 0x85D880 seeds the stream with the creator seed. A map carries
    /// the stream as its creation left it (`GameMap::rng_state`: the random
    /// map generator's last state, or a loaded map's saved GWORLD state read
    /// by 0x4FC600), getter entries included, which replaces this.
    pub fn create_rng(&mut self, seed: u32) {
        self.native_rng = aa_simulation::NativeRng::default();
        self.trace.log(crate::trace::SKind::RngGetter, self.native_rng.state);
        self.native_rng = aa_simulation::NativeRng::from_native_seed(seed);
        for _ in 0..crate::trace::GWORLD_INIT_DRAWS {
            self.random_u32();
        }
        if let Some(state) = self.map.rng_state {
            self.trace = crate::trace::Trace::default();
            for &s in &self.map.rng_log {
                self.trace.log(crate::trace::SKind::RngGetter, s);
            }
            self.native_rng = aa_simulation::NativeRng { state };
        }
    }
    /// GetRNG 0x4F02B0 then one 0x85D840 draw: the raw u32.
    pub fn random_u32(&mut self) -> u32 {
        self.trace.log(crate::trace::SKind::RngGetter, self.native_rng.state);
        self.native_rng.next_u32()
    }
    /// 0x40C8A0 after the getter: one draw `% n`, no rejection (0 for n = 0).
    pub fn random_below(&mut self, n: u32) -> u32 {
        let v = self.random_u32();
        if n == 0 { 0 } else { v % n }
    }
    pub fn spawn_actor(&mut self, def: DefId, kingdom: Option<KingdomId>, pos: Vec3, facing: f32) -> ActorId {
        let id = self.actors.len() as ActorId;
        let health = self.data.def(def).health;
        let weapons = self.data.def(def).weapons.len();
        self.actors.push(Actor {
            id,
            def,
            kingdom,
            pos,
            facing,
            health,
            alive: true,
            removed: false,
            org: None,
            slot: None,
            motion: Motion::default(),
            combat: Combat { target: None, recharge: vec![0.0; weapons] },
            activity: Activity::Idle,
            activity_since: self.time,
            order: Order::Idle,
            queue: Vec::new(),
        });
        // (core) its native GTID: a kept map thing's saved id, else next-fit;
        // a created (not restored) thing draws as 0x4F9C10 does.
        if self.ids.created_actor(id) {
            crate::creation::created(self, def, Some(id));
        }
        id
    }
    pub fn actor(&self, id: ActorId) -> Option<&Actor> {
        self.actors.get(id as usize).filter(|a| !a.removed)
    }
    pub fn actor_mut(&mut self, id: ActorId) -> Option<&mut Actor> {
        self.actors.get_mut(id as usize).filter(|a| !a.removed)
    }
    pub fn set_activity(&mut self, id: ActorId, activity: Activity) {
        let time = self.time;
        if let Some(a) = self.actors.get_mut(id as usize) {
            if a.activity != activity {
                a.activity = activity;
                a.activity_since = time;
            }
        }
    }
    /// GetRNG 0x4F02B0 then the unit float 0x45D400 of one 0x85D840 draw
    /// (PC53 product, stored binary32; can equal 1.0).
    pub fn random(&mut self) -> f32 {
        crate::trace::native_unit(self.random_u32())
    }
    /// The battle starts after the world is built: the world sync dump
    /// 0x4F8AE0 -> 0x4F6280 uses one world-RNG draw, then session launch
    /// 0x775D3F resets S (0x77D900 index 0, hash 0), wiping what world
    /// creation logged. The next S entry is tick 1's clock with index 1.
    pub fn begin_battle(&mut self) {
        self.random_u32();
        self.trace.reset(0, 0);
    }
    pub fn ground(&self, p: Vec2) -> Vec3 {
        Vec3::new(p.x, p.y, self.map.height_at(p))
    }
}
