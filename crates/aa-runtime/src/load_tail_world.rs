//! Bounded binding of `4FD3F3..4FD47C` to the authoritative staged objects.
//!
//! This is the camera/local-player part after the completed PostLoad pass, not
//! world publication. It reuses `aa_actors::world_load_tail`; it neither replaces
//! converted records nor starts a simulation. The caller must supply the actual
//! host/session fields and ordered world/Kingdom lists described below. They
//! cannot be inferred from map-header player labels or an actor-owner scan.
//!
//! The `4F1E10` branch requires explicit owned statistics and source bindings.
//! Missing bindings remain errors. Field reads happen only on reached paths.
use crate::actor_world::{
    ActorStore, ObjectKey, RuntimeComponent, RuntimeObject, Source, SpecialSource,
};
use aa_actors::{
    ActorId,
    components::ComponentSlot,
    post_load::{Error, Id, Result},
    world_load_tail::{self, LoadTailWorld},
};
use aa_mapworld::{restore::LoadCompletion, special_state::SpecialShell};
use aa_numeric::Precision;
use serde::Serialize;
use std::collections::BTreeMap;

pub type KingdomId = ActorId;

/// Actual session player `[B36558]+8`, with the player's `+24/+28` fields.
/// A missing player and a player with no Kingdom are distinct native cases.
#[derive(Clone, Debug)]
pub struct LocalPlayer {
    pub name_utf16: Vec<u16>,
    pub kingdom: Option<ActorId>,
}

/// Host application/session state. No default application mode or player is
/// selected by this module. In particular, application state 3 is an explicit
/// native mode, not a fallback for a missing session.
#[derive(Clone, Debug)]
pub struct SessionFacts {
    /// `[B36550]+E4`.
    pub application_state: u32,
    pub local_player: Option<LocalPlayer>,
}

/// Borrow the actual Session/Node owner at every native read. Handles remain
/// scoped to that owner; a copied map label cannot stand in for player+24.
pub trait SessionRead: crate::load_world_globals::SessionPhase {
    type Player;
    fn local_player(&self) -> std::result::Result<Option<Self::Player>, String>;
    fn player_name_24(&self, player: &Self::Player) -> std::result::Result<&[u16], String>;
    fn player_kingdom_28(
        &self,
        player: &Self::Player,
    ) -> std::result::Result<Option<ActorId>, String>;
}

// Existing explicitly supplied bounded fixture view. Concrete application
// startup uses its retained SessionRead owner instead of constructing this.
impl crate::load_world_globals::SessionPhase for SessionFacts {
    fn current_phase(&self) -> std::result::Result<u32, String> {
        Err("bounded tail SessionFacts has no Session+F8 producer".into())
    }
    fn application_state(&self) -> std::result::Result<u32, String> {
        Ok(self.application_state)
    }
}
impl SessionRead for SessionFacts {
    type Player = LocalPlayer;
    fn local_player(&self) -> std::result::Result<Option<Self::Player>, String> {
        Ok(self.local_player.clone())
    }
    fn player_name_24(&self, player: &Self::Player) -> std::result::Result<&[u16], String> {
        // The legacy view returned a cloned value. Select the same retained
        // fixture owner; opaque live handles are validated by their own host.
        self.local_player
            .as_ref()
            .filter(|v| v.name_utf16 == player.name_utf16 && v.kingdom == player.kingdom)
            .map(|v| v.name_utf16.as_slice())
            .ok_or("tail fixture player is not current".into())
    }
    fn player_kingdom_28(
        &self,
        player: &Self::Player,
    ) -> std::result::Result<Option<ActorId>, String> {
        self.local_player
            .as_ref()
            .filter(|v| v.name_utf16 == player.name_utf16 && v.kingdom == player.kingdom)
            .map(|v| v.kingdom)
            .ok_or("tail fixture player is not current".into())
    }
}

/// Inputs with producers outside ActorStore. `None` means the producer has not
/// supplied a value; it never means zero, an empty list, or a false flag.
///
/// `map_settings_mode` is `[GWorld+28]+A8`, NOT MAP_HEADER WorldRules.mode.
/// `kingdom_slots` is GWorld `+148/+14C`, NOT its `+154/+158` major-Kingdom
/// list. `kingdom_actors` holds native head-first Kingdom `+284` membership;
/// do not rebuild it by sorting IDs or scanning owner references.
pub struct WorldFacts<'a> {
    pub map_settings_mode: u32,
    /// Exact result of `59E520(settings)` when the mode gate reaches it.
    pub world_start_flag: Option<bool>,
    /// Saved GWorld `+48`; older saves require the real constructor producer.
    pub world_word_48: Option<u32>,
    /// Terrain `[GWorld+30]+C/+10`; not UI/display dimensions.
    pub map_size_bits: Option<[u32; 2]>,
    pub kingdom_slots: Option<&'a [Option<ActorId>]>,
    pub kingdom_actors: &'a BTreeMap<ActorId, Vec<ActorId>>,
    /// Proven ThingData `+1B4`, keyed by the canonical definition identity.
    /// No source producer is installed by this adapter.
    pub definition_priorities: &'a BTreeMap<String, i32>,
}

/// Exact `404F60` log arguments after the GWorld `+22C/+230` stores. Keeping
/// their binary64 words avoids reformatting source floats for parity checks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LookAtLog {
    pub function: &'static str,
    pub line: u32,
    pub format: &'static str,
    pub x_bits: u64,
    pub y_bits: u64,
}

/// Caller-owned GWorld camera fields and emitted log arguments. `None` records
/// that no camera write has happened; it is not a native zero initializer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraState {
    pub look_at: Option<[u32; 2]>,
    pub messages: Vec<LookAtLog>,
}

/// Authoritative mutable owners and completed source definitions for4F1E10.
/// Major kingdoms at+154 are distinct from the camera's player slots at+148.
pub struct WorldStartFacts<'a> {
    pub economy: &'a mut BTreeMap<u32, crate::restoration::KingdomEconomyLoad>,
    pub started: &'a mut bool,
    pub major_kingdoms: &'a [Option<ActorId>],
    pub source: &'a aa_rules::actors::ActorCatalog,
    pub resources: &'a aa_economy::Catalog,
    pub formulas: &'a aa_economy::statistics_live::DerivedCatalog,
    pub source_precision: Precision,
}

/// Reads actor state directly; no copied pose, flags, owner or component mask
/// is retained. This stage accepts staged (not spatially published) placement.
pub struct World<'a, S: SessionRead + ?Sized = SessionFacts> {
    pub actors: &'a mut ActorStore,
    pub session: &'a S,
    pub facts: WorldFacts<'a>,
    pub camera: &'a mut CameraState,
    pub precision: Precision,
    pub world_start: Option<WorldStartFacts<'a>>,
}

fn error(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}

impl<S: SessionRead + ?Sized> World<'_, S> {
    fn key(&self, id: Id) -> Result<ObjectKey> {
        self.actors
            .key(ActorId::from_raw(id))
            .ok_or_else(|| error(format!("load-tail GTID {id:#010x} is not live")))
    }

    fn object(&self, id: Id) -> Result<&RuntimeObject> {
        self.actors.object(self.key(id)?).map_err(error)
    }

    fn kingdom(&self, id: Id) -> Result<&SpecialShell> {
        let entity = self.actors.entity(self.key(id)?).map_err(error)?;
        match &entity.source {
            Source::Special(s) if matches!(s.source, SpecialSource::Kingdom(_)) => s
                .derived
                .as_ref()
                .ok_or_else(|| error(format!("Kingdom {id:#x} has no derived state"))),
            _ => Err(Error::Undefined(format!("{id:#x} is not a staged Kingdom"))),
        }
    }

    /// Execute only the already-ported load-tail interval. A success proves
    /// camera initialization for the supplied inputs, not a publishable world.
    /// Conversion replacement `4FD47C..4FD781` remains the caller's next stage.
    pub fn after_post_load(&mut self) -> Result<()> {
        for entry in self.actors.identities().iter() {
            let entity = self.actors.entity(entry.value).map_err(error)?;
            let (completion, failure) = match &entity.source {
                Source::Loaded(v) => (v.completion, v.failure.as_ref()),
                Source::Special(v) => (v.completion, v.failure.as_ref()),
                Source::New { .. } | Source::NewObject { .. } => {
                    return Err(Error::Unsupported(
                        "load-tail binding of PostLoad-created fresh objects is unavailable",
                    ));
                }
            };
            if completion != LoadCompletion::PostLoaded || failure.is_some() {
                return Err(error(format!(
                    "load-tail object {:#x} has not completed PostLoad",
                    entry.id.raw()
                )));
            }
        }
        world_load_tail::after_post_load(self)
    }
}

impl<S: SessionRead + ?Sized> LoadTailWorld for World<'_, S> {
    type Player = S::Player;

    fn map_settings_mode(&self) -> Result<u32> {
        Ok(self.facts.map_settings_mode)
    }

    fn world_start_flag(&mut self) -> Result<bool> {
        self.facts.world_start_flag.ok_or(Error::Unsupported(
            "59E520 map-settings world-start flag has no host binding",
        ))
    }

    fn world_start(&mut self) -> Result<()> {
        let start = self.world_start.as_mut().ok_or(Error::Unsupported(
            "4F1E10 requires live world +274, 58F010 statistics tracking and 58F530/45F240 snapshots",
        ))?;
        crate::world_start_world::World {
            actors: self.actors,
            economy: start.economy,
            started: start.started,
            major_kingdoms: start.major_kingdoms,
            source: start.source,
            resources: start.resources,
            formulas: start.formulas,
            source_precision: start.source_precision,
            precision: self.precision,
        }
        .start()
        .map_err(error)
    }

    fn app_state(&self) -> Result<u32> {
        self.session.application_state().map_err(error)
    }

    fn world_word_48(&self) -> Result<u32> {
        self.facts.world_word_48.ok_or(Error::Unsupported(
            "GWorld +48 has no saved or constructor binding",
        ))
    }

    fn local_player(&self) -> Result<Option<Self::Player>> {
        self.session.local_player().map_err(error)
    }

    fn player_name(&self, player: &Self::Player) -> Result<Vec<u16>> {
        Ok(self.session.player_name_24(player).map_err(error)?.to_vec())
    }

    fn player_kingdom(&self, player: &Self::Player) -> Result<Option<Id>> {
        Ok(self
            .session
            .player_kingdom_28(player)
            .map_err(error)?
            .map(ActorId::raw))
    }

    fn world_kingdom_slots(&self) -> Result<Vec<Option<Id>>> {
        self.facts
            .kingdom_slots
            .map(|slots| slots.iter().map(|id| id.map(ActorId::raw)).collect())
            .ok_or(Error::Unsupported(
                "GWorld +148/+14C Kingdom slots have no host binding",
            ))
    }

    fn object_name_18(&self, object: Id) -> Result<Vec<u16>> {
        Ok(self.object(object)?.thing.name_utf16.clone())
    }

    fn map_size_bits(&self) -> Result<[u32; 2]> {
        self.facts.map_size_bits.ok_or(Error::Unsupported(
            "terrain +C/+10 dimensions have no runtime binding",
        ))
    }

    fn kingdom_actors(&self, kingdom: Id) -> Result<Vec<Id>> {
        self.kingdom(kingdom)?;
        self.facts
            .kingdom_actors
            .get(&ActorId::from_raw(kingdom))
            .map(|ids| ids.iter().copied().map(ActorId::raw).collect())
            .ok_or(Error::Unsupported(
                "Kingdom +284 actor membership has no runtime binding",
            ))
    }

    fn kingdom_word_1f8(&self, kingdom: Id) -> Result<Option<Id>> {
        let kingdom = self.kingdom(kingdom)?;
        match kingdom.identity_references.get(&0x1f8) {
            Some(value) => Ok(value.map(ActorId::raw)),
            None if kingdom.words.get(&0x1f8) == Some(&0) => Ok(None),
            _ => Err(Error::Unsupported(
                "Kingdom +1F8 capital identity has no runtime binding",
            )),
        }
    }

    fn object_flags(&self, object: Id) -> Result<u32> {
        Ok(self
            .actors
            .staged_placement(self.key(object)?)
            .map_err(error)?
            .flags)
    }

    fn has_settlement(&self, actor: Id) -> Result<bool> {
        Ok(self
            .object(actor)?
            .components()
            .map_err(error)?
            .present(ComponentSlot::Settlement))
    }

    /// `4CE6C0`: Organization present -> self, otherwise Element+10 if nonnull,
    /// otherwise self. The next native field read resolves the returned object.
    fn organization_of(&self, actor: Id) -> Result<Id> {
        let components = self.object(actor)?.components().map_err(error)?;
        if components.present(ComponentSlot::Organization) {
            return Ok(actor);
        }
        match components.get(ComponentSlot::Element) {
            Some(RuntimeComponent::Element(v)) => Ok(v.organization.map_or(actor, ActorId::raw)),
            None => Ok(actor),
            Some(_) => Err(error("Element slot holds another component")),
        }
    }

    fn definition_word_1b4(&self, object: Id) -> Result<i32> {
        let definition = &self.object(object)?.definition.proof.id;
        self.facts
            .definition_priorities
            .get(definition)
            .copied()
            .ok_or_else(|| {
                error(format!(
                    "ThingData +1B4 has no proven producer for {definition}"
                ))
            })
    }

    fn object_position(&self, object: Id) -> Result<[u32; 2]> {
        let p = self
            .actors
            .staged_placement(self.key(object)?)
            .map_err(error)?;
        Ok([p.position_bits[0], p.position_bits[1]])
    }

    fn set_look_at(&mut self, xy: [u32; 2]) -> Result<()> {
        self.camera.look_at = Some(xy);
        Ok(())
    }

    fn log_look_at(&mut self, xy: [u32; 2]) -> Result<()> {
        self.camera.messages.push(LookAtLog {
            function: world_load_tail::LOOK_AT_FUNCTION,
            line: world_load_tail::LOOK_AT_LINE,
            format: world_load_tail::LOOK_AT_FORMAT,
            x_bits: f64::from(f32::from_bits(xy[0])).to_bits(),
            y_bits: f64::from(f32::from_bits(xy[1])).to_bits(),
        });
        Ok(())
    }

    fn precision(&self) -> Precision {
        self.precision
    }
}
