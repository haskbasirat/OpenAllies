//! Film decrees → `aa_play::Command`s.
//!
//! The film names things by their native identity, so a replay needs a
//! table from those identities to aa-play's ids. The core owner provides it
//! by implementing [`NativeIds`] over the native id table being added to
//! aa-play (roadmap F2). What it must answer:
//!
//! | wire field | native meaning | answered by |
//! |---|---|---|
//! | 16-bit thing slot (selection, 87 actor, 65 attack, 70 target) | `thing+0x14` low16, the GTID slot (0xB364C0 table) | [`NativeIds::thing`] |
//! | 32-bit command word (`CommandWire::word`, command +0x10) | full GTID `generation << 16 \| slot` of the target | [`NativeIds::gtid`] |
//! | session player id (298 SetPlayer, SessionPlayers) | SessionPlayer+0x20 | [`NativeIds::kingdom`] |
//! | object reference (`CommandWire::object`) | Thing DataManager `[0xB39F58]` slot, native load order | [`NativeIds::definition`] |
//! | formation reference (`CommandWire::parameter`) | FormationData `[0xB3AB08]` slot | [`NativeIds::formation`] |
//! | property reference (`CommandWire::extra`) | SharedPropertyData `[0xB3AA2C]` slot | [`NativeIds::property`] |
//! | ability reference (70) | AbilityDefinition `[0xB3AB10]` slot | [`NativeIds::general_ability`] |
//! | CString selector (66/67) | the TellActor selector (0x60F760) | [`NativeIds::named`] |
//!
//! The mapper keeps the per-player selection (16/17/18/19/333) and the film
//! ordering player (298), so 86 TellSelectedCommand applies to the current
//! selection like the native 61F210 path. Positions are raw binary32 bits
//! converted with `f32::from_bits`; nothing is rounded.
//!
//! Command semantics of the 0x61EBA0 fields used here come from aa-runtime
//! `element_move_command.rs` (word = target +0x10, position, auxiliary,
//! scalar = speed). Two choices are inferred: the unpack facing (taken
//! from `auxiliary` as a direction) and `attack_move: false` for every
//! move. They are marked where used.
use crate::decree::Decree;
use crate::film::FilmDecree;
use aa_orders::commands::CommandWire;
use aa_play::{ActorId, Command, DefId, KingdomId, OrgId, Unit};
use glam::Vec2;
use std::collections::BTreeMap;

/// The id table aa-play must provide for a replay.
pub trait NativeIds {
    /// Session player id → the kingdom it orders for.
    fn kingdom(&self, player: u32) -> Option<KingdomId>;
    /// Live thing in a 16-bit GTID slot → the aa-play unit (an organization
    /// or a single actor).
    fn thing(&self, slot: u16) -> Option<Unit>;
    /// Full 32-bit GTID → the live unit, `None` when the generation is stale.
    fn gtid(&self, gtid: u32) -> Option<Unit>;
    /// The actor an aa-play `Attack`/`Follow` should aim at for a unit
    /// (for an organization: the actor aa-play targets for it).
    fn target_actor(&self, unit: Unit) -> Option<ActorId>;
    /// Thing DataManager slot → aa-play definition.
    fn definition(&self, object: u32) -> Option<DefId>;
    /// FormationData slot → index into the organization layout's
    /// `Def::formations`.
    fn formation(&self, org: OrgId, formation: u32) -> Option<usize>;
    /// SharedPropertyData slot → the technology property IDS aa-play's
    /// `Command::Research` takes.
    fn property(&self, property: u32) -> Option<String>;
    /// AbilityDefinition slot → index into the kingdom general's abilities.
    fn general_ability(&self, kingdom: KingdomId, ability: u32) -> Option<u32>;
    /// TellActor CString selector → unit.
    fn named(&self, selector: &[u16]) -> Option<Unit>;
}

/// What one decree became.
#[derive(Clone, Debug, PartialEq)]
pub enum Mapped {
    /// Applied as these aa-play commands, in order.
    Commands(Vec<Command>),
    /// Changed the replay state only (ordering player, selection).
    State,
    /// No gameplay effect in a battle simulation (sync, timeline, chat, names).
    NoEffect(&'static str),
    /// A gameplay decree that aa-play has no command for.
    NoEquivalent(String),
    /// The id table could not resolve a reference.
    Unresolved(String),
}

pub use crate::classify::{ACTOR_COMMAND_MAPPING, decree_class};

fn point(bits: [u32; 2]) -> Vec2 {
    Vec2::new(f32::from_bits(bits[0]), f32::from_bits(bits[1]))
}

/// Replay state: the film ordering player and each player's selection.
#[derive(Clone, Debug, Default)]
pub struct Mapper {
    pub player: Option<u32>,
    pub selections: BTreeMap<u32, Vec<u16>>,
}

impl Mapper {
    pub fn new() -> Self {
        Self::default()
    }

    /// Map one decree. Call in film order; the state carries across ticks.
    pub fn map(&mut self, decree: &FilmDecree, ids: &dyn NativeIds) -> Mapped {
        self.player = decree.player;
        match &decree.decree {
            Decree::SetPlayer { player } => {
                self.player = (*player != 0).then_some(*player);
                Mapped::State
            }
            Decree::SelectionSet { slot } => {
                self.selection().clear();
                if *slot != 0 {
                    self.selection().push(*slot);
                }
                Mapped::State
            }
            Decree::SelectionAdd { slot } => {
                let s = self.selection();
                if !s.contains(slot) {
                    s.push(*slot);
                }
                Mapped::State
            }
            Decree::SelectionRemove { slot } => {
                self.selection().retain(|s| s != slot);
                Mapped::State
            }
            Decree::SelectionClear | Decree::SelectionClearNamed { .. } => {
                self.selection().clear();
                Mapped::State
            }
            Decree::SimSyncValue { .. } | Decree::SyncValue { .. } => {
                Mapped::NoEffect("sync checkpoint")
            }
            Decree::SetGameSpeed { .. }
            | Decree::TogglePaused
            | Decree::SetPaused { .. }
            | Decree::SpeedUp
            | Decree::SpeedDown => {
                Mapped::NoEffect("timeline (admin/world interleave, not world state)")
            }
            Decree::UpdateKingdomName { .. } => Mapped::NoEffect("kingdom display name"),
            Decree::TellSelectedCommand(wire) => {
                let slots = self.selection().clone();
                self.command(wire, &slots, ids)
            }
            Decree::TellActorCommand { command, actor } => self.command(command, &[*actor], ids),
            Decree::TellActorAttack { attacker, target } => {
                let Some(kingdom) = self.kingdom(ids) else {
                    return unresolved("ordering player kingdom");
                };
                let Some(unit) = ids.thing(*attacker) else {
                    return unresolved("attacker slot");
                };
                let Some(target) = ids.thing(*target).and_then(|t| ids.target_actor(t)) else {
                    return unresolved("attack target slot");
                };
                Mapped::Commands(vec![Command::Attack {
                    kingdom,
                    units: vec![unit],
                    target,
                }])
            }
            Decree::TellActorMove { selector, words } => {
                let Some(kingdom) = self.kingdom(ids) else {
                    return unresolved("ordering player kingdom");
                };
                let Some(unit) = ids.named(selector) else {
                    return unresolved("TellActorMove selector");
                };
                Mapped::Commands(vec![Command::Move {
                    kingdom,
                    units: vec![unit],
                    to: point([words[0], words[1]]),
                    attack_move: false,
                }])
            }
            Decree::FactionUseAbility {
                ability,
                words,
                target,
            } => {
                let Some(kingdom) = self.kingdom(ids) else {
                    return unresolved("ordering player kingdom");
                };
                let Some(index) = ids.general_ability(kingdom, *ability) else {
                    return unresolved("general ability definition");
                };
                let target = match target {
                    Some(slot) => match ids.thing(*slot).and_then(|t| ids.target_actor(t)) {
                        Some(a) => Some(aa_play::AbilityTarget::Actor(a)),
                        None => return unresolved("ability target slot"),
                    },
                    None => Some(aa_play::AbilityTarget::Point(point(*words))),
                };
                Mapped::Commands(vec![Command::UseAbility {
                    kingdom,
                    ability: index,
                    target,
                }])
            }
            Decree::TellActorAbility { .. } => {
                Mapped::NoEquivalent("TellActorAbility (scripted ability)".into())
            }
            Decree::GamePlayerKingdom { .. } => {
                Mapped::NoEquivalent("GamePlayerKingdom (player binding)".into())
            }
            Decree::Other { .. } => Mapped::NoEquivalent(decree.name.to_owned()),
        }
    }

    fn selection(&mut self) -> &mut Vec<u16> {
        self.selections.entry(self.player.unwrap_or(0)).or_default()
    }
    fn kingdom(&self, ids: &dyn NativeIds) -> Option<KingdomId> {
        ids.kingdom(self.player?)
    }

    /// 0x61EBA0 command on `slots` (86: the selection, 87: one actor).
    fn command(&mut self, wire: &CommandWire, slots: &[u16], ids: &dyn NativeIds) -> Mapped {
        let Some(index) = wire.command else {
            return Mapped::NoEquivalent("command without ActorCommand".into());
        };
        let (name, target) = ACTOR_COMMAND_MAPPING
            .get(index as usize)
            .copied()
            .unwrap_or(("unknown ActorCommand", None));
        if target.is_none() {
            return Mapped::NoEquivalent(format!("ActorCommand {index} {name}"));
        }
        let Some(kingdom) = self.kingdom(ids) else {
            return unresolved("ordering player kingdom");
        };
        let mut units = Vec::new();
        for slot in slots {
            match ids.thing(*slot) {
                Some(u) => units.push(u),
                None => return unresolved(&format!("thing slot {slot}")),
            }
        }
        if units.is_empty() {
            return Mapped::Unresolved(format!("ActorCommand {name} with an empty selection"));
        }
        let has = |bit: u16| wire.flags >> bit & 1 != 0;
        let actors = || -> Option<Vec<ActorId>> {
            units
                .iter()
                .map(|u| {
                    if let Unit::Actor(a) = u {
                        Some(*a)
                    } else {
                        None
                    }
                })
                .collect()
        };
        let orgs = || -> Option<Vec<OrgId>> {
            units
                .iter()
                .map(|u| if let Unit::Org(o) = u { Some(*o) } else { None })
                .collect()
        };
        let target_of = |word: u32| ids.gtid(word).and_then(|u| ids.target_actor(u));
        let out: Vec<Command> = match name {
            "kill" => {
                let Some(target) = has(0).then(|| target_of(wire.word)).flatten() else {
                    return unresolved("kill target GTID");
                };
                vec![Command::Attack {
                    kingdom,
                    units,
                    target,
                }]
            }
            "move" => {
                if has(0) && wire.word != 0 {
                    let Some(target) = target_of(wire.word) else {
                        return unresolved("move target GTID");
                    };
                    vec![Command::Follow {
                        kingdom,
                        units,
                        target,
                    }]
                } else if has(2) {
                    // attack_move: no native flag is decoded for it (inferred false).
                    vec![Command::Move {
                        kingdom,
                        units,
                        to: point(wire.position),
                        attack_move: false,
                    }]
                } else {
                    return Mapped::NoEquivalent("move without position or target".into());
                }
            }
            "stop" => vec![Command::Stop { kingdom, units }],
            "disband" => vec![Command::Disband { kingdom, units }],
            "unpack" => {
                let (Some(trucks), true, true) = (actors(), has(2), has(3)) else {
                    return unresolved("unpack truck actors, position and facing");
                };
                // Facing from the auxiliary direction words (inferred).
                let facing =
                    f32::from_bits(wire.auxiliary[1]).atan2(f32::from_bits(wire.auxiliary[0]));
                let at = point(wire.position);
                trucks
                    .into_iter()
                    .map(|truck| Command::Deploy {
                        kingdom,
                        truck,
                        at,
                        facing,
                    })
                    .collect()
            }
            "recruit" => {
                let (Some(buildings), Some(def)) =
                    (actors(), wire.object.and_then(|o| ids.definition(o)))
                else {
                    return unresolved("recruit buildings and object definition");
                };
                buildings
                    .into_iter()
                    .map(|building| Command::Recruit {
                        kingdom,
                        building,
                        def,
                    })
                    .collect()
            }
            "set_rally_point" => {
                let (Some(buildings), true) = (actors(), has(2)) else {
                    return unresolved("rally buildings and position");
                };
                let at = point(wire.position);
                buildings
                    .into_iter()
                    .map(|building| Command::SetRallyPoint {
                        kingdom,
                        building,
                        at,
                    })
                    .collect()
            }
            "sell" => {
                let Some(buildings) = actors() else {
                    return unresolved("sell buildings");
                };
                buildings
                    .into_iter()
                    .map(|building| Command::Sell { kingdom, building })
                    .collect()
            }
            "pack" => {
                let Some(buildings) = actors() else {
                    return unresolved("pack buildings");
                };
                buildings
                    .into_iter()
                    .map(|building| Command::Pack { kingdom, building })
                    .collect()
            }
            "attach" => {
                let (Some(orgs), Some(Unit::Actor(hq))) =
                    (orgs(), has(0).then(|| ids.gtid(wire.word)).flatten())
                else {
                    return unresolved("attach organizations and HQ GTID");
                };
                orgs.into_iter()
                    .map(|org| Command::Attach { kingdom, org, hq })
                    .collect()
            }
            "hq_detach" => {
                let Some(orgs) = orgs() else {
                    return unresolved("detach organizations");
                };
                orgs.into_iter()
                    .map(|org| Command::Detach { kingdom, org })
                    .collect()
            }
            "set_formation" => {
                let (Some(orgs), Some(formation)) = (orgs(), wire.parameter) else {
                    return unresolved("formation organizations and FormationData");
                };
                let mut out = Vec::new();
                for org in orgs {
                    let Some(index) = ids.formation(org, formation) else {
                        return unresolved("FormationData in the organization layout");
                    };
                    out.push(Command::SetFormation {
                        kingdom,
                        orgs: vec![org],
                        formation: index,
                    });
                }
                out
            }
            "research" => {
                let (Some(buildings), Some(property)) =
                    (actors(), wire.extra.and_then(|p| ids.property(p)))
                else {
                    return unresolved("research buildings and SharedPropertyData");
                };
                buildings
                    .into_iter()
                    .map(|building| Command::Research {
                        kingdom,
                        building,
                        property: property.clone(),
                    })
                    .collect()
            }
            _ => return Mapped::NoEquivalent(format!("ActorCommand {index} {name}")),
        };
        Mapped::Commands(out)
    }
}

fn unresolved(what: &str) -> Mapped {
    Mapped::Unresolved(what.to_owned())
}

/// Map a whole film. `ids` is asked at each decree, so an id table that
/// follows the replayed world (allocations, deaths) stays current when the
/// caller interleaves this with `World::step`. Returns per tick the commands
/// and the non-command outcomes.
pub fn map_tick(
    mapper: &mut Mapper,
    decrees: &[FilmDecree],
    ids: &dyn NativeIds,
) -> (Vec<Command>, Vec<Mapped>) {
    let mut commands = Vec::new();
    let mut other = Vec::new();
    for d in decrees {
        match mapper.map(d, ids) {
            Mapped::Commands(c) => commands.extend(c),
            m => other.push(m),
        }
    }
    (commands, other)
}

/// [`NativeIds`] over aa-play's native id table (`World::ids`,
/// `aa_play::ids`). Players resolve through the film's SessionPlayers rows
/// (Kingdom+0x188 index → `aa_play::ids::kingdom_by_index`), object
/// definitions through the ThingData IDS (`Registries::object_ids`) and
/// `GameData::find`, SharedProperty references through the native slot
/// table (`property_ids`: the wire value indexes the registry's +0x24 entry
/// vector, as for ThingData). Formation, general-ability and selector
/// references are not mapped yet and stay `None` (the mapper then reports
/// `Mapped::Unresolved`).
pub struct PlayIds<'a> {
    pub world: &'a aa_play::World,
    pub players: &'a [crate::film::SessionPlayer],
    pub object_ids: &'a [String],
    /// SharedPropertyData slot -> property IDS (empty: unmapped).
    pub property_ids: &'a [String],
}
impl NativeIds for PlayIds<'_> {
    fn kingdom(&self, player: u32) -> Option<KingdomId> {
        let index = self
            .players
            .iter()
            .find(|p| p.id == player)?
            .kingdom_index?;
        aa_play::ids::kingdom_by_index(self.world, index)
    }
    fn thing(&self, slot: u16) -> Option<Unit> {
        aa_play::ids::unit(self.world.ids.thing_at_slot(slot)?)
    }
    fn gtid(&self, gtid: u32) -> Option<Unit> {
        aa_play::ids::unit(self.world.ids.thing(gtid)?)
    }
    fn target_actor(&self, unit: Unit) -> Option<ActorId> {
        aa_play::ids::target_actor(self.world, unit)
    }
    fn definition(&self, object: u32) -> Option<DefId> {
        self.world.data.find(self.object_ids.get(object as usize)?)
    }
    fn formation(&self, _org: OrgId, _formation: u32) -> Option<usize> {
        None
    }
    fn property(&self, property: u32) -> Option<String> {
        self.property_ids.get(property as usize).cloned()
    }
    fn general_ability(&self, _kingdom: KingdomId, _ability: u32) -> Option<u32> {
        None
    }
    fn named(&self, _selector: &[u16]) -> Option<Unit> {
        None
    }
}
