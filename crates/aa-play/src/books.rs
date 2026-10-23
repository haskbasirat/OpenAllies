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
    /// SupplyComponent zone active (the actor provides supply).
    pub zone_active: bool,
    pub queue: Queue,
    pub deploy: Option<Deployment>,
    /// Battle time this actor was created (construction/unpack start).
    pub created_at: f32,
    /// Properties added to this actor beyond its definition's (from the map:
    /// `PlacedThing::properties`); their modifiers apply to it.
    #[serde(default)]
    pub properties: Vec<String>,
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
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SupplyZone {
    pub provider: ActorId,
    pub kingdom: KingdomId,
    pub centre: Vec2,
    pub radius: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Notice {
    /// Event 3 kingdom_upkeep_shortage (percent of upkeep that cannot be paid).
    UpkeepShortage { kingdom: KingdomId, percent: i32 },
    /// Event 4 kingdom_upkeep_repaired.
    UpkeepRepaired { kingdom: KingdomId },
    /// The head of a production queue paused (native notify_pause).
    ProductionPaused { building: ActorId, reason: PauseReason },
    /// A recruit could not be afforded.
    Unaffordable { kingdom: KingdomId, building: ActorId },
    /// A building finished unpacking or a truck finished packing.
    Packed { truck: ActorId },
    /// A truck or building changed into another actor (old -> new).
    Replaced { old: ActorId, new: ActorId },
    /// A technology finished (kingdom property added, event 5).
    Researched { kingdom: KingdomId, property: String },
    /// An organization was attached to another HQ (0x574C10).
    Attached { org: OrgId, hq: ActorId },
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
