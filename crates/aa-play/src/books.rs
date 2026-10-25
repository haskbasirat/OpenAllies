//! Core-owner side state that the shared types do not carry: the kingdom
//! resource books (the native Kingdom arrays), production queue timers,
//! supply state, deployment timers and regeneration progress. Owner: core.
//!
//! Indexed like the shared vectors: `kingdoms[KingdomId]`, `actors[ActorId]`,
//! `orgs[OrgId]`; the actor and organization tables grow on demand
//! (`Books::actor_mut`), so any system may spawn actors without touching them.
use crate::data::DefId;
use crate::world::{ActorId, KingdomId, OrgId};
use glam::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Books {
    pub kingdoms: Vec<KingdomBooks>,
    pub actors: Vec<ActorBooks>,
    pub orgs: Vec<OrgBooks>,
    /// Indices into `map.placed` that stayed scenery (not turned into actors
    /// and not removed with an unused player slot). The renderer draws these.
    pub scenery: Vec<usize>,
    /// Economy/production messages of the current tick (HUD message log).
    pub notices: Vec<Notice>,
    /// Production queue item ids (native recruit item ids).
    pub next_item: u32,
    /// Supply zones active this tick: (provider, kingdom, centre, radius).
    pub zones: Vec<SupplyZone>,
    /// Kingdom relations `relations[a][b]` (how kingdom a regards kingdom b),
    /// native values: 0 self, 1 friend, 2 neutral, 3 enemy. Built by setup;
    /// empty means the team rule (same team friend, else enemy).
    pub relations: Vec<Vec<u8>>,
    /// The tiles each zone of `zones` covers (same order; sorted tile
    /// indices of the ZoneSolver flood, `supply::flood_tiles`).
    #[serde(skip)]
    pub zone_tiles: Vec<std::sync::Arc<Vec<u32>>>,
    /// Flood results by (start tile, radius bits): the map does not change
    /// during a battle, so a zone is flooded again only when it moves.
    #[serde(skip)]
    pub zone_floods: std::collections::HashMap<(u32, u32), std::sync::Arc<Vec<u32>>>,
    /// The live denizen groups (0x568180 objects; the aircraft formation
    /// groups of AircraftComponent +0xA8/+0xAC), by id (`crate::denizen`).
    #[serde(default)]
    pub groups: std::collections::BTreeMap<u32, Mission>,
    /// The next group id (ids start at 1; not native, the groups are
    /// pointers there).
    #[serde(default)]
    pub next_group: u32,
    /// The hosted native Minds (`crate::minds`).
    #[serde(default)]
    pub minds: crate::minds::Minds,
}

/// Native relation values (4E52D0).
pub const SELF: u8 = 0;
pub const FRIEND: u8 = 1;
pub const NEUTRAL: u8 = 2;
pub const ENEMY: u8 = 3;

/// The native Kingdom economy arrays (kingdom +1C4 stock and the provider /
/// consumer vectors), in `GameData::resources` order, per minute.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KingdomBooks {
    /// Sum of the eligible actors' production.
    pub raw_provided: Vec<f32>,
    /// `raw_provided` with limited providers capped at EconomyLimitedResourceMax.
    pub provided: Vec<f32>,
    /// Upkeep, queue reservations and the shortage purchase charges
    /// (an upkeep deficit bought with money at the resource PurchaseCost).
    pub consumed: Vec<f32>,
    /// Upkeep before shortage charges.
    pub upkeep: Vec<f32>,
    pub stockpiled: Vec<f32>,
    /// Kingdom +1F0: 1 - provided/consumed while money ran out, else 0.
    pub shortage_ratio: f32,
    /// Kingdom +1F4: ratio last announced.
    pub announced_ratio: f32,
    /// Technologies researched (kingdom property records +0x318, GLOBAL
    /// research items), Property IDS in completion order.
    pub technologies: Vec<String>,
    /// The kingdom's difficulty handicap property (Game/handicaps.tgi, e.g.
    /// property_handicap_easy), applied to every kingdom actor like a
    /// technology.
    pub handicap: Option<String>,
    /// Kingdom +0x324: kingdom-level research in progress (0x4E2310).
    #[serde(default)]
    pub research: aa_production::research::KingdomResearch,
    /// The general WorldParameters 0x50E8B0 picked at random for this kingdom
    /// (Faction IDS), used by `special_ops::set_general(.., None)`.
    #[serde(default)]
    pub general: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ActorBooks {
    /// `map.placed` index this actor was created from.
    pub placed: Option<usize>,
    /// Actor state 19 (out_of_supply).
    pub out_of_supply: bool,
    /// SupplyComponent +18 relay stockpile (seconds of supply left when cut off).
    pub supply_stock: f32,
    /// SupplyComponent +1C relay this actor draws from.
    pub supply_relay: Option<ActorId>,
    /// SupplyComponent zone active (the actor provides supply), as its last
    /// 0x58AE10 decided.
    pub zone_active: bool,
    /// Its InitExisting 0x58B0E0 ran (the first 0x58AE10).
    #[serde(default)]
    pub supply_started: bool,
    /// Actor state 21 (actor +0x100 bit 21): placed by BuildActor 0x563160
    /// and under construction until a repair brings its body whole
    /// (0x561E50 -> 0x566AC0 -> OnConstructionFinished 0x565B00).
    #[serde(default)]
    pub constructing: bool,
    /// CityComponent +0x18: the candidates admitted by 0x564300 on entering
    /// the city zone (head first), pruned by the City Tick 0x564C10.
    #[serde(default)]
    pub city_candidates: Vec<CityCandidateRef>,
    pub queue: Queue,
    pub deploy: Option<Deployment>,
    /// Battle time this actor was created (construction/unpack start).
    pub created_at: f32,
    /// Properties added to this actor beyond its definition's (from the map:
    /// `PlacedThing::properties`); their modifiers apply to it.
    #[serde(default)]
    pub properties: Vec<String>,
    /// Its Economy component's applied contributions (570020).
    #[serde(default)]
    pub economy: EconomyCache,
    /// ResearchComponent +0x10: the research records it completed, newest
    /// first (0x584120).
    #[serde(default)]
    pub research_completed: Vec<u32>,
    /// Its DenizenComponent (airfields, carriers; `crate::denizen`).
    #[serde(default)]
    pub denizen: Option<DenizenBooks>,
    /// A denizen aircraft's link to its base (CreateDenizen 0x56CC10).
    #[serde(default)]
    pub aircraft: Option<AircraftBooks>,
}

/// A DenizenComponent (Denizen.cpp, component slot 10; constructor
/// 0x56B720, InitCreate 0x568750, InitExisting 0x5687E0).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DenizenBooks {
    /// +0x18 inventory slots, head first.
    pub slots: Vec<DenizenSlot>,
    /// +0x20 sally_forth / recall and +0x21 escort_on / escort_off (0x56E730).
    pub sally: bool,
    pub escort: bool,
    /// +0x2C runway queue: aircraft waiting in the hangar, head first.
    pub runway: Vec<ActorId>,
    /// +0x34 time of the last takeoff release (0x568750: -delay).
    pub last_release: f32,
    /// +0x38 behaviours of the definition (0x5687E0) and +0x3C the ready
    /// ones (0x5696B0).
    pub behaviors: u32,
    pub ready: u32,
    /// +0x10 / +0x14 the occupants' current and full value (0x569E40).
    pub value: f32,
    pub value_max: f32,
    /// +0x40 missions (`Books::groups` ids), head first; each node holds a
    /// reference.
    pub missions: Vec<u32>,
}

/// One inventory slot (0x56B720 node: +0 occupant, +4 definition,
/// +8 health, +C / +D the 0x568880 flags for masks 2 and 0x2C).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DenizenSlot {
    pub def: DefId,
    /// None while housed.
    pub occupant: Option<ActorId>,
    pub health: f32,
    pub worker: bool,
    pub defensive: bool,
}

/// A deployed denizen group (0x568180, vtable 0xA3BC14).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mission {
    pub id: u32,
    /// +0x24 the base's GTID, as its actor.
    pub base: ActorId,
    /// +4 the reference count (0x568180 starts it at 0; the base's list,
    /// each member Mind +0x30 and each flight +0xA8 / +0xAC hold one).
    pub refs: u32,
    /// +0xC members in creation order.
    pub members: Vec<ActorId>,
    /// +8 the DenizenGroup (index into `DenizenDef::groups`).
    pub group: usize,
    /// +0x14 the mean of the group's FormationPositions over the members
    /// (0x568180: NaN for a group without positions, as 0 * (1/0)).
    pub offset: [f32; 3],
    /// +0x20 the mode of its use (0x568480).
    pub mode: u32,
    /// The ability context of the use (0x4CA330 / 0x4CA760).
    pub order: AirOrder,
    /// +0x28 its escort mission (0x56E390).
    pub escort: Option<u32>,
}

/// The command a mission's use hands each member (0x4D9B60).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum AirOrder {
    /// 0x4CA330(target): attack (command 0) the actor.
    AttackActor(ActorId),
    /// 0x4CA330(0) with the point (flag 4): attack the ground point.
    AttackPoint(Vec2),
    /// 0x4CA760: move (command 5) to the point.
    Recon(Vec2),
    /// 0x56E390's escort context of the mission.
    Escort(u32),
}

/// An actor with an AircraftComponent. Its Mind (home +0x28, mode +0x24,
/// groups +0x30 / +0x1D4) is hosted in `Books::minds` (`crate::combat::air`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AircraftBooks {
    /// The AircraftComponent itself (actor+0xBC, constructor 0x55E8E0): the
    /// movement owner's MoveTick state, read and written by the aircraft
    /// Mind through its word accessors. Stored once.
    pub flight: aa_movement::aircraft::AircraftState,
    /// Actor flag +8 bit 30 (0x4D04B0): waiting in the hangar for the
    /// runway release (0x5695B0).
    pub hidden: bool,
    /// Actor flag +8 bit 2: a deployed denizen (CreateDenizen 0x56CC10).
    #[serde(default)]
    pub denizen: bool,
    /// Actor +0x3C, the model scale: the AircraftComponent taxi_scale at
    /// CreateDenizen; AircraftComponent MoveTick 0x5612F0 rewrites it every
    /// move (the movement owner's flight).
    #[serde(default = "unit_scale")]
    pub scale: f32,
    /// The Performer's mana (actor+0x74 Performer; PerformerComponent
    /// mana_max at creation): each ability use pays its mana_cost (57E030).
    #[serde(default)]
    pub mana: f32,
}

fn unit_scale() -> f32 {
    1.0
}

/// An Economy component's state 570020 keeps (EconomyState +0/+1 vectors
/// and the +10/+11 applied flags): what this thing has added to its owner
/// kingdom's provided and consumed arrays, so a change removes exactly that
/// and adds the new values.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EconomyCache {
    /// Resources[0]: provided (production) vector, applied while `provides`.
    pub provided: Vec<f32>,
    /// Resources[1]: upkeep vector, applied while `pays`.
    pub upkeep: Vec<f32>,
    /// +10 / +11 flags.
    pub provides: bool,
    pub pays: bool,
    /// The kingdom the vectors were added to.
    pub kingdom: Option<KingdomId>,
    /// The organization the actor was a member of at the last refresh.
    pub parent: Option<OrgId>,
    /// Fingerprint of the inputs of the last refresh (not native).
    pub key: u64,
}

/// Native production queue (aa-production `Queue`), seconds of battle time.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Queue {
    pub items: Vec<QueueItem>,
    /// Completion time of the head item.
    pub deadline: f32,
    pub duration: f32,
    /// Time the head item was paused (None while running).
    pub paused_at: Option<f32>,
    pub pause_reason: Option<PauseReason>,
    /// Registered on the periodic updatable queue 0 (0x50AC00); a new
    /// registration draws the world RNG for its bucket (0x50B2C0).
    #[serde(default)]
    pub scheduled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PauseReason {
    /// Native 0: the LIMITED_CONSUMED upkeep (unit limit) cannot be reserved.
    LimitedResource,
    /// Native 1: the building requires supply and is out of supply.
    Supply,
    /// Native 2: the building is disabled (packing/unpacking).
    Disabled,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: u32,
    pub def: DefId,
    /// Cost paid at the request (refunded on cancel or failed creation).
    pub paid: Vec<f32>,
    /// LIMITED_CONSUMED upkeep the item needs (unit limit).
    pub requested_upkeep: Vec<f32>,
    /// The part currently reserved in the kingdom consumption.
    pub current_upkeep: Vec<f32>,
    /// The item reserves an HQ attachment (organization recruited by an HQ).
    pub attaches: bool,
    /// A research item (native queue kind 1): the technology Property IDS.
    /// Its `def` is the researching building's own definition.
    pub research: Option<String>,
    /// Item +0x4C: the HQ attachment slot released at removal.
    #[serde(default)]
    pub organization_slot: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeployKind {
    /// Building appearing from its truck.
    Unpacking,
    /// Building folding back into its truck.
    Packing,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Deployment {
    pub kind: DeployKind,
    /// What the actor becomes when done.
    pub into: DefId,
    pub started: f32,
    pub done_at: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OrgBooks {
    /// Organization +50[slot]: health rebuilt so far for an empty slot.
    pub progress: Vec<f32>,
    /// The organization actor's supply state (state 19).
    pub out_of_supply: bool,
    /// HQ attachment slots reserved by queued recruits (HQ organizations).
    pub reserved_attachments: u32,
    /// The organization actor's Economy component (570020).
    #[serde(default)]
    pub economy: EconomyCache,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SupplyZone {
    pub provider: ActorId,
    pub kingdom: KingdomId,
    pub centre: Vec2,
    pub radius: f32,
}

/// A city candidate (a lone actor or an organization, 0x5640A0).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CityCandidateRef {
    Actor(ActorId),
    Org(OrgId),
}

/// Why BuildActor 0x563160 refused a structure (`Notice::ConstructRefused`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ConstructRefusal {
    /// construct_error_affording|ss (0x562FD0) with the quoted price.
    Unaffordable { cost: Vec<f32> },
    /// construct_error_creating|s (0x4F9C10 failed).
    CreationFailed,
}

/// The refused purchase of `Notice::Unaffordable`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Purchase {
    /// A recruit of this definition.
    Recruit(DefId),
    /// Research of this property id.
    Research(String),
    /// Attaching this organization to the HQ.
    Attach(OrgId),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Notice {
    /// Event 3 kingdom_upkeep_shortage (percent of upkeep that cannot be paid).
    UpkeepShortage { kingdom: KingdomId, percent: i32 },
    /// Event 4 kingdom_upkeep_repaired.
    UpkeepRepaired { kingdom: KingdomId },
    /// The head of a production queue paused (native notify_pause).
    ProductionPaused { building: ActorId, reason: PauseReason },
    /// A purchase could not be afforded (0x582950 recruit_error_affording,
    /// research_error_affording, hq_error_attach_affording): what was
    /// refused at `building` (the HQ for an attach) and the quoted price, in
    /// `GameData::resources` order.
    Unaffordable { kingdom: KingdomId, building: ActorId, what: Purchase, cost: Vec<f32> },
    /// A building finished unpacking or a truck finished packing.
    Packed { truck: ActorId },
    /// A truck or building changed into another actor (old -> new).
    Replaced { old: ActorId, new: ActorId },
    /// A technology finished (kingdom property added, event 5).
    Researched { kingdom: KingdomId, property: String },
    /// An organization was attached to another HQ (0x574C10).
    Attached { org: OrgId, hq: ActorId },
    /// A construct order's BuildActor (0x563160) refused the structure.
    ConstructRefused { kingdom: KingdomId, def: DefId, reason: ConstructRefusal },
    /// denizen_error_affording|ss (0x56D5A0 / 0x56D7C0 / 0x56DA00): the air
    /// attack or air recon of `building` could not be afforded. `what` is
    /// the subject string id (denizen_air_attack / denizen_air_recon).
    DenizenUnaffordable { kingdom: KingdomId, building: ActorId, what: String, cost: Vec<f32> },
}

impl Books {
    pub fn actor(&self, id: ActorId) -> Option<&ActorBooks> {
        self.actors.get(id as usize)
    }
    pub fn actor_mut(&mut self, id: ActorId) -> &mut ActorBooks {
        let i = id as usize;
        if self.actors.len() <= i {
            self.actors.resize_with(i + 1, ActorBooks::default);
        }
        &mut self.actors[i]
    }
    pub fn org(&self, id: OrgId) -> Option<&OrgBooks> {
        self.orgs.get(id as usize)
    }
    pub fn org_mut(&mut self, id: OrgId) -> &mut OrgBooks {
        let i = id as usize;
        if self.orgs.len() <= i {
            self.orgs.resize_with(i + 1, OrgBooks::default);
        }
        &mut self.orgs[i]
    }
    pub fn kingdom(&self, id: KingdomId) -> Option<&KingdomBooks> {
        self.kingdoms.get(id as usize)
    }
    pub fn kingdom_mut(&mut self, id: KingdomId, resources: usize) -> &mut KingdomBooks {
        let i = id as usize;
        if self.kingdoms.len() <= i {
            self.kingdoms.resize_with(i + 1, KingdomBooks::default);
        }
        let k = &mut self.kingdoms[i];
        for v in [&mut k.raw_provided, &mut k.provided, &mut k.consumed, &mut k.upkeep, &mut k.stockpiled] {
            if v.len() < resources {
                v.resize(resources, 0.0);
            }
        }
        k
    }
    /// How kingdom `a` regards kingdom `b` (native 4E52D0). An unowned side
    /// is neutral (the native global default relation).
    pub fn relation(&self, kingdoms: &[crate::world::Kingdom], a: Option<KingdomId>, b: Option<KingdomId>) -> u8 {
        let (Some(a), Some(b)) = (a, b) else { return NEUTRAL };
        if a == b {
            return SELF;
        }
        if let Some(v) = self.relations.get(a as usize).and_then(|row| row.get(b as usize)) {
            return *v;
        }
        match (kingdoms.get(a as usize), kingdoms.get(b as usize)) {
            (Some(x), Some(y)) if x.team == y.team => FRIEND,
            (Some(_), Some(_)) => ENEMY,
            _ => NEUTRAL,
        }
    }
    pub fn out_of_supply(&self, id: ActorId) -> bool {
        self.actor(id).is_some_and(|a| a.out_of_supply)
    }
    /// Seconds left on the head of `building`'s queue (frozen while paused).
    pub fn head_seconds_left(&self, building: ActorId, now: f32) -> Option<f32> {
        let q = &self.actor(building)?.queue;
        q.items.first()?;
        Some((q.deadline - q.paused_at.unwrap_or(now)).max(0.0))
    }
}
