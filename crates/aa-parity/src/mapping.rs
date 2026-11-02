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
//!
//! A command names the thing selected, not the thing that acts on it. The
//! delivery 0x4D9B60 (`KKC_GActorCommand::Process`) gives it to the selected
//! actor's component of the command definition's slot; an actor without that
//! component passes it on through vtable +0x158 / +0x15C or to its
//! organization's leader (+0x7C -> +0x44), and so on. So a recruit selected
//! on an HQ regiment reaches the structure its leader unpacked into
//! (0x4D9BE8 -> 0x4CE6BA -> 0x583BF8 in the film's native chain), and an
//! unpack selected on it reaches the regiment Mind (`KKC_CAIStateUnpack`
//! 0x526E60), which orders its leader. [`Mapper::dropped`] counts the order
//! fields that aa-play's `Command` cannot carry yet, so a replay reports them
//! rather than losing them silently.
//!
//! A default order (flag bit 6 clear, the player's plain click) carries no
//! ActorCommand: each selected unit guesses its own (0x4ED360 -> 0x4DA570,
//! [`NativeIds::guess`]), with `non_group_excluded` when the selection has
//! distinct head and tail; a unit that guesses nothing is skipped. The units are
//! then grouped by what they guessed and mapped like explicit commands.
//!
//! The Mind handlers shape two of them: `kill` and a `move` at a hostile actor go
//! through the bombard state when the layout fires indirect (0x527FB0, 0x527E30:
//! 0x578AC0 and 0x57A5E0) and fall back to a move at the actor when the kill
//! state refuses a target that is not hostile (0x51D920, 0x57A500, 0x520320).
//!
//! Delivery checks the thing's command mask twice (0x4ED360 vt+150, 0x4D9B60 vt+38), so
//! a regiment Mind command the thing does not hold is not given: a retreat from an
//! engaged regiment too low in morale, anything to a routing one. Those are counted in
//! [`Mapper::dropped`] as `<command>: not in the command mask`. They are decided by
//! `aa_play::orders::guess::action_bits`, a rendition of 0x4D1350.
use crate::decree::Decree;
use crate::film::FilmDecree;
use aa_orders::commands::CommandWire;
use aa_orders::payload::Field;
use aa_play::orders::ActorOrder;
use aa_play::{AbilityTarget, ActorId, Command, DefId, KingdomId, OrgId, Unit};
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
    /// OrganizationLayout slot → its IDS (`Registries::layout_ids`).
    fn layout(&self, _layout: u32) -> Option<String> {
        None
    }
    /// 0x4DA570 for a default order: the ActorCommand index this unit takes.
    fn guess(&self, _unit: Unit, _flags: u16, _word: u32, _non_group_excluded: bool) -> Option<u32> {
        None
    }
    /// vt+150 (0x4ED360) / vt+38 (0x4D9B60): the ActorCommand is in this unit's
    /// command mask now (Actor +F0, rebuilt by 0x4D1350).
    fn accepts(&self, _unit: Unit, _index: u32) -> bool {
        true
    }
    /// 0x578AC0 and 0x57A5E0: a kill of this unit is a bombard.
    fn bombards(&self, _unit: Unit) -> bool {
        false
    }
    /// Target vt+16C (0x4D1F40 -> vt+140): the target is hostile to `kingdom`.
    fn hostile(&self, _kingdom: KingdomId, _target: ActorId) -> bool {
        true
    }
    /// A 16-bit slot naming a kingdom (68, and the 23 team commands).
    fn thing_kingdom(&self, _slot: u16) -> Option<KingdomId> {
        None
    }
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
    /// Order fields aa-play's `Command` cannot carry: how many, by name.
    pub dropped: BTreeMap<&'static str, u32>,
    /// 68 GamePlayerKingdom: the kingdom a session player orders for now.
    pub kingdoms: BTreeMap<u32, KingdomId>,
    /// 0x4ED360 cases 0x25 / 0x26: VWorldState +24's +18, each player's group
    /// behaviour of a selection of more than one thing (0x4ECF00 makes the
    /// settings null for a single one).
    pub group_on: BTreeMap<u32, bool>,
    /// The command being mapped goes to a group, which runs each member's Mind
    /// handler itself.
    grouped: bool,
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
                self.selected(wire, &slots, ids)
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
                // 0x6109F0: the words are always the point; a target the reader
                // resolves to null (0x76A5A0) leaves the point.
                let at = aa_play::AbilityTarget::Point(point(*words));
                let target = match target {
                    Some(slot) => match ids.thing(*slot).and_then(|t| ids.target_actor(t)) {
                        Some(a) => Some(aa_play::AbilityTarget::Actor(a)),
                        None => Some(at),
                    },
                    None => Some(at),
                };
                Mapped::Commands(vec![Command::UseAbility {
                    kingdom,
                    ability: index,
                    target,
                }])
            }
            // 0x61E100 -> 0x4D0E60: both names resolve through 0x4F8E90, which
            // the id table does not answer; no corpus film has this decree.
            Decree::TellActorAbility { .. } => unresolved("TellActorAbility selector"),
            // 0x75E...: the session player orders for this kingdom from now on.
            Decree::GamePlayerKingdom { kingdom } => {
                match (self.player, ids.thing_kingdom(*kingdom)) {
                    (Some(player), Some(k)) => {
                        self.kingdoms.insert(player, k);
                        Mapped::State
                    }
                    _ => unresolved("GamePlayerKingdom kingdom slot"),
                }
            }
            Decree::Other { fields } => self.other(decree, fields, ids),
        }
    }

    fn selection(&mut self) -> &mut Vec<u16> {
        self.selections.entry(self.player.unwrap_or(0)).or_default()
    }
    fn kingdom(&self, ids: &dyn NativeIds) -> Option<KingdomId> {
        let player = self.player?;
        self.kingdoms.get(&player).copied().or_else(|| ids.kingdom(player))
    }

    /// The decrees without a typed form.
    fn other(&mut self, decree: &FilmDecree, fields: &[Field], ids: &dyn NativeIds) -> Mapped {
        use crate::decree::id;
        match decree.descriptor as usize {
            id::TEAM_COMMAND => self.team_command(fields, ids),
            // 0x60EE60: an observer joins; the world does not change. 0x60EF40
            // 0x61F530 and 0x76E9B0 (the AIs taking over mid-game) change who
            // gives the film's next orders, which the film itself names.
            69 => Mapped::NoEffect("game player observer"),
            72 => Mapped::NoEffect("vote to resign (the session votes; the kingdom's end is its own decree)"),
            329 => Mapped::NoEffect("midgame AIs and session players"),
            _ => Mapped::NoEquivalent(decree.name.to_owned()),
        }
    }

    /// KKC_TeamCommand 0x509090 / Process 0x509840: word 0 the definition, the
    /// giver thing, optionally the receiver, then by the definition's flags a
    /// GTID (bit 0), a point (bit 1) and an amount (bit 3).
    fn team_command(&mut self, fields: &[Field], ids: &dyn NativeIds) -> Mapped {
        let mut it = fields.iter();
        let Some(Field::Uint { value: index, .. }) = it.next() else {
            return unresolved("team command definition");
        };
        let Some(Field::Thing { slot: from, .. }) = it.next() else {
            return unresolved("team command kingdom");
        };
        let Some(Field::Uint { value: has_to, .. }) = it.next() else {
            return unresolved("team command receiver bit");
        };
        let to = if *has_to == 1 {
            match it.next() {
                Some(Field::Thing { slot, .. }) => Some(*slot),
                _ => return unresolved("team command receiver"),
            }
        } else {
            None
        };
        let Some((name, flags)) = crate::registries::TEAM_COMMANDS.get(*index as usize).copied() else {
            return unresolved("team command index");
        };
        let word = if flags & 1 != 0 {
            match it.next() {
                Some(Field::Uint { value, .. }) => Some(*value),
                _ => return unresolved("team command GTID"),
            }
        } else {
            None
        };
        if flags >> 1 & 1 != 0 {
            let _ = (it.next(), it.next());
        }
        let amount = if flags >> 3 & 1 != 0 {
            match it.next() {
                Some(Field::Uint { value, .. }) => Some(f32::from_bits(*value)),
                _ => return unresolved("team command amount"),
            }
        } else {
            None
        };
        let Some(kingdom) = ids.thing_kingdom(*from) else {
            return unresolved("team command kingdom slot");
        };
        match name {
            "team_give_resources" => {
                let (Some(to), Some(amount)) = (to.and_then(|t| ids.thing_kingdom(t)), amount) else {
                    return unresolved("team_give_resources receiver and amount");
                };
                Mapped::Commands(vec![Command::Order {
                    kingdom,
                    units: Vec::new(),
                    order: ActorOrder::GiveResources { to, amount },
                }])
            }
            "team_give_actor" => {
                let (Some(to), Some(actor)) = (to.and_then(|t| ids.thing_kingdom(t)), word.and_then(|w| ids.gtid(w))) else {
                    return unresolved("team_give_actor receiver and actor");
                };
                Mapped::Commands(vec![Command::Order {
                    kingdom,
                    units: Vec::new(),
                    order: ActorOrder::GiveActor { to, actor },
                }])
            }
            // 0x509840 cases 5, 6, 8 and 9: messages for the players only.
            "team_request_resources" | "team_request_actor" | "team_chat" | "team_report" => {
                Mapped::NoEffect("team message")
            }
            // Cases 0, 1, 2, 7: a flare actor (0x572E50). Flares have no host in
            // aa-play and no film of the corpus has one.
            other => Mapped::NoEffect(match other {
                "team_attack" => "team_attack flare (no host)",
                "team_defend" => "team_defend flare (no host)",
                "team_explore" => "team_explore flare (no host)",
                _ => "team_build flare (no host)",
            }),
        }
    }

    /// 0x4ED360, the selection's path of 0x61F210. With the group behaviour on and
    /// more than one thing selected the selection becomes a new group (not for
    /// set_formation): `aa_play::orders::group::selection` does that with the
    /// command. A default order takes the lowest type its members guess (0x4DE740),
    /// and an order that names no selection here is preceded by the group thing.
    fn selected(&mut self, wire: &CommandWire, slots: &[u16], ids: &dyn NativeIds) -> Mapped {
        let player = self.player.unwrap_or(0);
        let multi = slots.first() != slots.last();
        let index = wire.command;
        if multi {
            match index {
                Some(37) => {
                    self.group_on.insert(player, true);
                }
                Some(38) => {
                    self.group_on.insert(player, false);
                }
                _ => {}
            }
        } else if matches!(index, Some(37 | 38)) {
            // The settings are null for one thing (0x4ECF00): the command goes on
            // to the thing, which has no such component.
            return Mapped::Commands(Vec::new());
        }
        let grouped = multi
            && index != Some(33)
            && index != Some(37)
            && index != Some(38)
            && self.group_on.get(&player).copied().unwrap_or(false);
        if !grouped {
            return self.command(wire, slots, ids);
        }
        let Some(kingdom) = self.kingdom(ids) else {
            return unresolved("ordering player kingdom");
        };
        let units: Vec<Unit> = slots.iter().filter_map(|s| ids.thing(*s)).collect();
        self.grouped = true;
        let mapped = match index {
            Some(_) => self.command(wire, slots, ids),
            None => {
                let least = units.iter().filter_map(|u| ids.guess(*u, wire.flags, wire.word, false)).min();
                match least {
                    Some(index) if !units.is_empty() => self.explicit(index, wire, units.clone(), kingdom, ids),
                    _ => Mapped::Commands(Vec::new()),
                }
            }
        };
        self.grouped = false;
        match mapped {
            Mapped::Commands(mut c) => {
                let selection = c.iter().any(|c| aa_play::orders::group::Request::of(c).is_some());
                if !selection && !c.is_empty() {
                    c.insert(0, Command::Order { kingdom, units, order: ActorOrder::GroupFormed });
                }
                Mapped::Commands(c)
            }
            other => other,
        }
    }

    /// 0x61EBA0 command on `slots` (86: the selection, 87: one actor). The
    /// selection's thing that is gone is skipped, as 0x4ED360's vt+B4 skips it.
    fn command(&mut self, wire: &CommandWire, slots: &[u16], ids: &dyn NativeIds) -> Mapped {
        let Some(kingdom) = self.kingdom(ids) else {
            return unresolved("ordering player kingdom");
        };
        let mut units = Vec::new();
        for slot in slots {
            match ids.thing(*slot) {
                Some(u) => units.push(u),
                None => *self.dropped.entry("selected thing gone").or_default() += 1,
            }
        }
        if units.is_empty() {
            let name = match wire.command {
                Some(index) => ACTOR_COMMAND_MAPPING.get(index as usize).map_or("unknown ActorCommand", |c| c.0),
                None => "default order",
            };
            return Mapped::Unresolved(format!("ActorCommand {name} with an empty selection"));
        }
        let Some(index) = wire.command else {
            // 0x4ED360 / 0x4CF430: each unit guesses its own command.
            let excluded = units.len() > 1;
            let mut groups: Vec<(u32, Vec<Unit>)> = Vec::new();
            for u in units {
                let Some(index) = ids.guess(u, wire.flags, wire.word, excluded) else {
                    *self.dropped.entry("default order: unit guessed nothing").or_default() += 1;
                    continue;
                };
                match groups.iter_mut().find(|g| g.0 == index) {
                    Some(g) => g.1.push(u),
                    None => groups.push((index, vec![u])),
                }
            }
            let mut out = Vec::new();
            for (index, units) in groups {
                match self.explicit(index, wire, units, kingdom, ids) {
                    Mapped::Commands(c) => out.extend(c),
                    other => return other,
                }
            }
            return Mapped::Commands(out);
        };
        self.explicit(index, wire, units, kingdom, ids)
    }

    /// The ActorCommand `index` on `units`.
    fn explicit(
        &mut self,
        index: u32,
        wire: &CommandWire,
        units: Vec<Unit>,
        kingdom: KingdomId,
        ids: &dyn NativeIds,
    ) -> Mapped {
        let (name, target) = ACTOR_COMMAND_MAPPING
            .get(index as usize)
            .copied()
            .unwrap_or(("unknown ActorCommand", None));
        if target.is_none() {
            return Mapped::NoEquivalent(format!("ActorCommand {index} {name}"));
        }
        // 0x4ED360: vt+150 then 0x4D9B60: vt+38. A thing without the command in its
        // mask does not get it: an engaged regiment too low in morale to retreat
        // (0x521E10), a routing one. A group asks each member itself (0x4DE7F0). Only
        // the slot 4 commands: the mask of the other slots is not rendered.
        let units = if !self.grouped && (index as usize) < MASKED.len() {
            let before = units.len();
            let units: Vec<Unit> = units.into_iter().filter(|u| ids.accepts(*u, index)).collect();
            if units.len() != before {
                *self.dropped.entry(MASKED[index as usize]).or_default() += (before - units.len()) as u32;
            }
            if units.is_empty() {
                return Mapped::Commands(Vec::new());
            }
            units
        } else {
            units
        };
        let has = |bit: u16| wire.flags >> bit & 1 != 0;
        // A lone actor order and an order on an organization that lacks the
        // slot's component: 0x4D9B60 hands it to the leader (+7C -> +44).
        let acting = |ids: &dyn NativeIds| -> Vec<ActorId> {
            units
                .iter()
                .filter_map(|u| match u {
                    Unit::Actor(a) => Some(*a),
                    Unit::Org(_) => ids.target_actor(*u),
                })
                .collect()
        };
        // 0x4ED360's vt+150: the units of the selection that hold the command;
        // here, of a slot 7 command, the organizations.
        let orgs = || -> Vec<OrgId> {
            units
                .iter()
                .filter_map(|u| if let Unit::Org(o) = u { Some(*o) } else { None })
                .collect()
        };
        let target_of = |word: u32| ids.gtid(word).and_then(|u| ids.target_actor(u));
        let order = |units: Vec<Unit>, order: ActorOrder| Command::Order { kingdom, units, order };
        let aux = has(3).then(|| point(wire.auxiliary));
        // The direction of a move (0x527E30, 0x527FB0): an order before it.
        let facing = aux.filter(|a| *a != Vec2::ZERO && matches!(name, "kill" | "move" | "retreat")).map(|aux| Command::Order {
            kingdom,
            units: units.clone(),
            order: ActorOrder::Facing { aux },
        });
        let out: Vec<Command> = match name {
            "kill" | "move" if has(0) && wire.word != 0 => {
                let Some(target) = target_of(wire.word) else {
                    return unresolved(if name == "kill" { "kill target GTID" } else { "move target GTID" });
                };
                self.count_scalar(wire);
                let hostile = ids.hostile(kingdom, target);
                if self.grouped {
                    // 0x4DF840: the same command to each member.
                    let onto = (name == "move" && hostile).then(|| order(units.clone(), ActorOrder::MoveOnto));
                    return Mapped::Commands(onto.into_iter().chain(facing).chain([Command::Attack { kingdom, units, target }]).collect());
                }
                let mut bombard = Vec::new();
                let mut attack = Vec::new();
                for &u in &units {
                    if hostile && ids.bombards(u) {
                        bombard.push(u);
                    } else {
                        attack.push(u);
                    }
                }
                let mut out: Vec<Command> = facing.into_iter().collect();
                if !bombard.is_empty() {
                    out.push(Command::Bombard {
                        kingdom,
                        units: bombard,
                        target: AbilityTarget::Actor(target),
                    });
                }
                if name == "move" && hostile && !attack.is_empty() {
                    out.insert(0, order(attack.clone(), ActorOrder::MoveOnto));
                }
                if !attack.is_empty() {
                    out.push(if hostile {
                        Command::Attack { kingdom, units: attack, target }
                    } else {
                        Command::Follow { kingdom, units: attack, target }
                    });
                }
                out
            }
            "kill" => {
                if !has(2) {
                    return unresolved("kill target GTID");
                }
                self.count_scalar(wire);
                let to = point(wire.position);
                if self.grouped {
                    return Mapped::Commands(facing.into_iter().chain([Command::Move { kingdom, units, to, attack_move: true }]).collect());
                }
                let (bombard, moving): (Vec<Unit>, Vec<Unit>) =
                    units.iter().partition(|u| ids.bombards(**u));
                let mut out: Vec<Command> = facing.into_iter().collect();
                if !bombard.is_empty() {
                    out.push(Command::Bombard { kingdom, units: bombard, target: AbilityTarget::Point(to) });
                }
                if !moving.is_empty() {
                    out.push(Command::Move { kingdom, units: moving, to, attack_move: true });
                }
                out
            }
            "move" => {
                if !has(2) {
                    return Mapped::NoEquivalent("move without position or target".into());
                }
                self.count_scalar(wire);
                // attack_move: no native flag is decoded for it (inferred false).
                facing.into_iter().chain([Command::Move { kingdom, units, to: point(wire.position), attack_move: false }]).collect()
            }
            "bombard" => {
                let target = if has(2) {
                    AbilityTarget::Point(point(wire.position))
                } else {
                    match target_of(wire.word) {
                        Some(t) => AbilityTarget::Actor(t),
                        None => return unresolved("bombard target GTID"),
                    }
                };
                vec![Command::Bombard { kingdom, units, target }]
            }
            "guard" => {
                let target = if has(0) {
                    match ids.gtid(wire.word) {
                        Some(t) => Some(t),
                        None => return unresolved("guard target GTID"),
                    }
                } else {
                    None
                };
                self.count_scalar(wire);
                vec![order(units, ActorOrder::Guard { target, at: point(wire.position), aux })]
            }
            "repair" => {
                let Some(target) = target_of(wire.word) else {
                    return unresolved("repair target GTID");
                };
                if has(9) {
                    // The 0x51ECF0 argument: how long the repair state stays.
                    *self.dropped.entry("repair boolean").or_default() += 1;
                }
                vec![Command::Repair { kingdom, units, target }]
            }
            "construct" => {
                let Some(def) = wire.object.and_then(|o| ids.definition(o)) else {
                    return unresolved("construct object definition");
                };
                let facing = f32::from_bits(wire.auxiliary[1]).atan2(f32::from_bits(wire.auxiliary[0]));
                vec![Command::Construct { kingdom, units, def, at: point(wire.position), facing }]
            }
            "stop" => vec![Command::Stop { kingdom, units }],
            "retreat" => {
                let target = if has(0) {
                    match target_of(wire.word) {
                        Some(t) => Some(t),
                        None => return unresolved("retreat target GTID"),
                    }
                } else {
                    None
                };
                self.count_scalar(wire);
                facing.into_iter().chain([order(units, ActorOrder::Retreat { target, at: point(wire.position) })]).collect()
            }
            "rout" => vec![order(units, ActorOrder::Rout)],
            "explore" => {
                let scalar = has(4).then(|| f32::from_bits(wire.scalar));
                vec![order(units, ActorOrder::Explore { at: point(wire.position), scalar })]
            }
            "disband" => vec![Command::Disband { kingdom, units }],
            // 0x4DF840 -> 0x4DE7F0: a member without the command takes a Move there.
            "unpack" if self.grouped && has(2) => {
                let facing = f32::from_bits(wire.auxiliary[1]).atan2(f32::from_bits(wire.auxiliary[0]));
                vec![order(units, ActorOrder::Unpack { at: point(wire.position), facing })]
            }
            "unpack" => {
                let trucks = acting(ids);
                if trucks.is_empty() || !has(2) {
                    return unresolved("unpack truck actors and position");
                }
                // Facing from the auxiliary direction words (inferred). The
                // Berlin film's unpacks carry none (flags 0x44): 0x575C80 then
                // hands the zero words to the creation record, and every
                // building it unpacks is axis_locked (0x4F3640; the sampler
                // shows direction (0, -1)), which `deploy::placement`
                // overrides. `atan2(0, 0)` = 0 is a stand-in for a zero
                // direction: a building that is not axis_locked would need the
                // zero-direction rule of 0x4F3640 / 0x4F9C10, not measured.
                let facing =
                    f32::from_bits(wire.auxiliary[1]).atan2(f32::from_bits(wire.auxiliary[0]));
                let at = point(wire.position);
                trucks
                    .into_iter()
                    .map(|truck| Command::Deploy { kingdom, truck, at, facing })
                    .collect()
            }
            "pack_organization" => vec![order(units, ActorOrder::PackOrganization)],
            "build_structure" => {
                let Some(def) = wire.object.and_then(|o| ids.definition(o)) else {
                    return unresolved("build_structure object definition");
                };
                vec![order(units, ActorOrder::BuildStructure { def, at: point(wire.position) })]
            }
            "commission" => {
                let Some(def) = wire.object.and_then(|o| ids.definition(o)) else {
                    return unresolved("commission object definition");
                };
                vec![order(units, ActorOrder::Commission { def })]
            }
            "recruit" | "recruit_location" => {
                let buildings = acting(ids);
                let Some(def) = wire.object.and_then(|o| ids.definition(o)) else {
                    return unresolved("recruit buildings and object definition");
                };
                if buildings.is_empty() {
                    return unresolved("recruit buildings and object definition");
                }
                // ParameterSet (flag 5): the organization layout and the
                // element of each layout entry the order chose.
                let parameters = match &wire.parameters {
                    Some(p) => {
                        let mut elements = Vec::with_capacity(p.objects.len());
                        for object in &p.objects {
                            match object {
                                Some(o) => match ids.definition(*o) {
                                    Some(d) => elements.push(d),
                                    None => return unresolved("recruit ParameterSet element"),
                                },
                                None => elements.push(aa_play::data::NO_DEF),
                            }
                        }
                        let layout = p.definition.and_then(|l| ids.layout(l));
                        Some(aa_play::RecruitParameters { layout, elements })
                    }
                    None => None,
                };
                if name == "recruit_location" {
                    let at = point(wire.position);
                    buildings
                        .into_iter()
                        .map(|b| order(vec![Unit::Actor(b)], ActorOrder::RecruitAt { def, at, parameters: parameters.clone() }))
                        .collect()
                } else {
                    buildings
                        .into_iter()
                        .map(|building| match &parameters {
                            Some(parameters) => Command::RecruitWith { kingdom, building, def, parameters: parameters.clone() },
                            None => Command::Recruit { kingdom, building, def },
                        })
                        .collect()
                }
            }
            "set_rally_point" => {
                let buildings = acting(ids);
                if buildings.is_empty() || !has(2) {
                    return unresolved("rally buildings and position");
                }
                let at = point(wire.position);
                buildings
                    .into_iter()
                    .map(|building| Command::SetRallyPoint { kingdom, building, at })
                    .collect()
            }
            "structure_cancel_queued" => {
                // 0x589DC3: fld scalar, ftol: the id of the queue item.
                let id = f32::from_bits(wire.scalar) as i32 as u32;
                acting(ids)
                    .into_iter()
                    .map(|b| order(vec![Unit::Actor(b)], ActorOrder::CancelQueued { id }))
                    .collect()
            }
            "construction_cancel" => vec![order(units, ActorOrder::ConstructionCancel)],
            "sell" => {
                let buildings = acting(ids);
                if buildings.is_empty() {
                    return unresolved("sell buildings");
                }
                buildings
                    .into_iter()
                    .map(|building| Command::Sell { kingdom, building })
                    .collect()
            }
            "upgrade" => {
                let Some(def) = wire.object.and_then(|o| ids.definition(o)) else {
                    return unresolved("upgrade object definition");
                };
                vec![order(units, ActorOrder::Upgrade { def })]
            }
            "raze" => vec![order(units, ActorOrder::Raze)],
            "sally_forth" => vec![order(units, ActorOrder::Sally { forth: true })],
            "recall" => vec![order(units, ActorOrder::Sally { forth: false })],
            "air_attack" | "air_recon" => {
                // 0x56E797: the actor when flag bit 0 resolves it, else the point.
                let target = match has(0).then(|| target_of(wire.word)).flatten() {
                    Some(t) => AbilityTarget::Actor(t),
                    None if has(2) => AbilityTarget::Point(point(wire.position)),
                    None => return unresolved("air order target"),
                };
                acting(ids)
                    .into_iter()
                    .map(|building| {
                        if name == "air_attack" {
                            Command::AirAttack { kingdom, building, target }
                        } else {
                            Command::AirRecon { kingdom, building, target }
                        }
                    })
                    .collect()
            }
            "escort_on" => vec![order(units, ActorOrder::Escort { on: true })],
            "escort_off" => vec![order(units, ActorOrder::Escort { on: false })],
            "attach" => {
                let Some(def) = wire.object.and_then(|o| ids.definition(o)) else {
                    return unresolved("attach object definition");
                };
                vec![order(units, ActorOrder::InventoryAttach { def })]
            }
            "detach" => vec![order(units, ActorOrder::DetachElement)],
            // This port's own Attach, which names its HQ (the marker of
            // `aa_play::player_orders::native`; no real film has this flag).
            "hq_attach" if has(0) && has(4) => {
                let Some(Unit::Actor(hq)) = ids.gtid(wire.word) else {
                    return unresolved("attach HQ GTID");
                };
                orgs().into_iter().map(|org| Command::Attach { kingdom, org, hq }).collect()
            }
            "hq_attach" => {
                if has(0) {
                    // 0x57B0A0 reads no field of the command.
                    *self.dropped.entry("hq_attach target (ignored by the cycle)").or_default() += 1;
                }
                vec![order(units, ActorOrder::HqCycle)]
            }
            "hq_detach" => orgs()
                .into_iter()
                .map(|org| Command::Detach { kingdom, org })
                .collect(),
            "set_formation" => {
                let Some(formation) = wire.parameter else {
                    return unresolved("formation organizations and FormationData");
                };
                let mut out = Vec::new();
                for org in orgs() {
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
            "pack" => acting(ids)
                .into_iter()
                .map(|building| Command::Pack { kingdom, building })
                .collect(),
            "research" => {
                let buildings = acting(ids);
                let Some(property) = wire.extra.and_then(|p| ids.property(p)) else {
                    return unresolved("research buildings and SharedPropertyData");
                };
                if buildings.is_empty() {
                    return unresolved("research buildings and SharedPropertyData");
                }
                buildings
                    .into_iter()
                    .map(|building| Command::Research {
                        kingdom,
                        building,
                        property: property.clone(),
                    })
                    .collect()
            }
            "cancel_flare" => vec![order(units, ActorOrder::CancelFlare)],
            "group_behavior_enable" => {
                let boolean = has(9).then_some(wire.boolean);
                vec![order(units, ActorOrder::GroupBehavior { enable: true, boolean })]
            }
            "group_behavior_disable" => {
                vec![order(units, ActorOrder::GroupBehavior { enable: false, boolean: None })]
            }
            _ => return Mapped::NoEquivalent(format!("ActorCommand {index} {name}")),
        };
        Mapped::Commands(out)
    }

    /// Flag bit 4 on a move: the state's +2C speed argument (0x520440), which
    /// `Command::Move` does not carry. 0.0 is its default (0x9F32DC), and every
    /// such order of the corpus has it.
    fn count_scalar(&mut self, wire: &CommandWire) {
        if wire.flags >> 4 & 1 != 0 && f32::from_bits(wire.scalar) != 0.0 {
            *self.dropped.entry("move speed scalar").or_default() += 1;
        }
    }
}

/// [`Mapper::dropped`]'s names of the commands the mask gate takes away, by index:
/// the regiment Mind's commands 0..=12 (0x521E10).
const MASKED: [&str; 13] = [
    "kill: not in the command mask",
    "bombard: not in the command mask",
    "guard: not in the command mask",
    "repair: not in the command mask",
    "construct: not in the command mask",
    "move: not in the command mask",
    "stop: not in the command mask",
    "retreat: not in the command mask",
    "rout: not in the command mask",
    "explore: not in the command mask",
    "disband: not in the command mask",
    "unpack: not in the command mask",
    "pack_organization: not in the command mask",
];

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
/// vector, as for ThingData), FormationData references through the catalog's
/// registration order (`formation_ids`, `Registries::formation_ids`: the +C
/// ordinal of 0x481C70, count 5 measured live, the slot order itself inferred
/// from that registration). AbilityDefinition references go through
/// [`set_ability_ids`] to the general's list by IDS. Selector references are not
/// mapped and stay `None` (the mapper then reports `Mapped::Unresolved`).
pub struct PlayIds<'a> {
    pub world: &'a aa_play::World,
    pub players: &'a [crate::film::SessionPlayer],
    pub object_ids: &'a [String],
    /// SharedPropertyData slot -> property IDS (empty: unmapped).
    pub property_ids: &'a [String],
    /// FormationData slot -> formation IDS (empty: unmapped).
    pub formation_ids: &'a [String],
    /// AbilityDefinition slot -> IDS of this data set (empty: [`set_ability_ids`]).
    pub ability_ids: &'a [String],
    /// OrganizationLayout slot -> IDS of this data set (empty: [`set_layout_ids`]).
    pub layout_ids: &'a [String],
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
    fn layout(&self, layout: u32) -> Option<String> {
        let table = if self.layout_ids.is_empty() {
            LAYOUT_IDS.get()?.as_slice()
        } else {
            self.layout_ids
        };
        table.get(layout as usize).cloned()
    }
    fn formation(&self, org: OrgId, formation: u32) -> Option<usize> {
        let id = self.formation_ids.get(formation as usize)?;
        let org = self.world.orgs.get(org as usize)?;
        self.world
            .data
            .def(org.def)
            .formations
            .iter()
            .position(|f| f.id.eq_ignore_ascii_case(id))
    }
    fn property(&self, property: u32) -> Option<String> {
        self.property_ids.get(property as usize).cloned()
    }
    fn general_ability(&self, kingdom: KingdomId, ability: u32) -> Option<u32> {
        let table = if self.ability_ids.is_empty() {
            ABILITY_IDS.get()?.as_slice()
        } else {
            self.ability_ids
        };
        let id = table.get(ability as usize)?;
        let general = aa_play::special_ops::general(self.world, kingdom)?;
        general
            .abilities
            .iter()
            .position(|a| !id.is_empty() && a.id.eq_ignore_ascii_case(id))
            .map(|i| i as u32)
    }
    fn named(&self, _selector: &[u16]) -> Option<Unit> {
        None
    }
    fn guess(&self, unit: Unit, flags: u16, word: u32, non_group_excluded: bool) -> Option<u32> {
        let fields = aa_play::orders::guess::Fields { flags: u32::from(flags), target: word };
        aa_play::orders::guess::guess(self.world, unit, fields, non_group_excluded)
    }
    fn accepts(&self, unit: Unit, index: u32) -> bool {
        aa_play::orders::guess::action_bits(self.world, unit) >> index & 1 != 0
    }
    fn bombards(&self, unit: Unit) -> bool {
        match unit {
            Unit::Org(o) => aa_play::orders::guess::bombard_layout(self.world, o),
            Unit::Actor(_) => false,
        }
    }
    fn hostile(&self, kingdom: KingdomId, target: ActorId) -> bool {
        aa_play::orders::hostile_to(self.world, kingdom, target)
    }
    fn thing_kingdom(&self, slot: u16) -> Option<KingdomId> {
        match self.world.ids.thing_at_slot(slot)? {
            aa_play::ids::Thing::Kingdom(k) => Some(k),
            _ => None,
        }
    }
}

/// AbilityDefinition slot -> IDS for every replay of this process
/// (`Registries::ability_ids`), set once by the caller like the formation and
/// property tables. Unset: 70 FactionUseAbility stays unresolved.
pub fn set_ability_ids(ids: Vec<String>) {
    let _ = ABILITY_IDS.set(ids);
}
static ABILITY_IDS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

/// OrganizationLayout slot -> IDS (`Registries::layout_ids`), set once by the
/// caller like the formation table. Unset: a recruit's ParameterSet is taken
/// to name the definition's own layout, as every film of the corpus does.
pub fn set_layout_ids(ids: Vec<String>) {
    let _ = LAYOUT_IDS.set(ids);
}
static LAYOUT_IDS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

#[cfg(test)]
mod tests {
    use super::*;
    use aa_orders::commands::ParameterSet;

    struct Ids;
    impl NativeIds for Ids {
        fn kingdom(&self, player: u32) -> Option<KingdomId> {
            (player == 2).then_some(1)
        }
        fn thing(&self, slot: u16) -> Option<Unit> {
            match slot {
                13399 => Some(Unit::Org(7)),
                13410 => Some(Unit::Actor(30)),
                _ => None,
            }
        }
        fn gtid(&self, gtid: u32) -> Option<Unit> {
            self.thing(gtid as u16)
        }
        fn target_actor(&self, unit: Unit) -> Option<ActorId> {
            match unit {
                Unit::Actor(a) => Some(a),
                Unit::Org(7) => Some(41),
                Unit::Org(_) => None,
            }
        }
        fn definition(&self, object: u32) -> Option<DefId> {
            (object == 954).then_some(5)
        }
        fn formation(&self, _org: OrgId, _formation: u32) -> Option<usize> {
            None
        }
        fn property(&self, _property: u32) -> Option<String> {
            None
        }
        fn general_ability(&self, _kingdom: KingdomId, _ability: u32) -> Option<u32> {
            None
        }
        fn named(&self, _selector: &[u16]) -> Option<Unit> {
            None
        }
    }

    fn decree(decree: Decree) -> FilmDecree {
        FilmDecree {
            descriptor: 86,
            name: "KKC_TellSelectedCommandOrderDecree",
            player: Some(2),
            payload_bit: 0,
            decree,
            fields: Vec::new(),
        }
    }

    /// 0x4ED360 vt+150: a Mind command a thing does not hold is not given (0x521E10
    /// takes retreat from an engaged regiment too low in morale); other slots pass.
    #[test]
    fn a_command_out_of_the_mask_is_not_given() {
        struct Masked;
        impl NativeIds for Masked {
            fn kingdom(&self, player: u32) -> Option<KingdomId> {
                Ids.kingdom(player)
            }
            fn thing(&self, slot: u16) -> Option<Unit> {
                Ids.thing(slot)
            }
            fn gtid(&self, gtid: u32) -> Option<Unit> {
                Ids.gtid(gtid)
            }
            fn target_actor(&self, unit: Unit) -> Option<ActorId> {
                Ids.target_actor(unit)
            }
            fn definition(&self, object: u32) -> Option<DefId> {
                Ids.definition(object)
            }
            fn formation(&self, _org: OrgId, _formation: u32) -> Option<usize> {
                None
            }
            fn property(&self, _property: u32) -> Option<String> {
                None
            }
            fn general_ability(&self, _kingdom: KingdomId, _ability: u32) -> Option<u32> {
                None
            }
            fn named(&self, _selector: &[u16]) -> Option<Unit> {
                None
            }
            fn accepts(&self, _unit: Unit, _index: u32) -> bool {
                false
            }
        }
        let mut m = Mapper::new();
        m.map(&decree(Decree::SelectionSet { slot: 13399 }), &Masked);
        let retreat = CommandWire {
            flags: 0x44,
            command: Some(7),
            position: [160.0f32.to_bits(), 366.0f32.to_bits()],
            ..Default::default()
        };
        assert_eq!(m.map(&decree(Decree::TellSelectedCommand(retreat)), &Masked), Mapped::Commands(Vec::new()));
        assert_eq!(m.dropped.get("retreat: not in the command mask"), Some(&1));
    }

    /// Film flags 0x44: an unpack with a position and no auxiliary words,
    /// selected on the HQ regiment, goes to its leader (0x4D9B60).
    #[test]
    fn unpack_on_a_regiment_goes_to_its_leader_without_facing_words() {
        let mut m = Mapper::new();
        m.map(&decree(Decree::SelectionSet { slot: 13399 }), &Ids);
        let wire = CommandWire {
            flags: 0x44,
            command: Some(11),
            position: [160.0f32.to_bits(), 366.0f32.to_bits()],
            ..Default::default()
        };
        let out = m.map(&decree(Decree::TellSelectedCommand(wire)), &Ids);
        let Mapped::Commands(c) = out else {
            panic!("{out:?}")
        };
        assert!(matches!(
            c[0],
            Command::Deploy {
                kingdom: 1,
                truck: 41,
                ..
            }
        ));
    }

    /// A recruit selected on an HQ regiment reaches its leader's structure,
    /// and the order's ParameterSet goes with it.
    #[test]
    fn recruit_on_a_regiment_carries_its_parameter_set() {
        let mut m = Mapper::new();
        m.map(&decree(Decree::SelectionSet { slot: 13399 }), &Ids);
        let wire = CommandWire {
            flags: 0x62,
            command: Some(15),
            object: Some(954),
            parameters: Some(ParameterSet {
                definition: Some(5),
                objects: vec![Some(954), None],
                negative_count: None,
            }),
            ..Default::default()
        };
        let out = m.map(&decree(Decree::TellSelectedCommand(wire)), &Ids);
        assert_eq!(
            out,
            Mapped::Commands(vec![Command::RecruitWith {
                kingdom: 1,
                building: 41,
                def: 5,
                parameters: aa_play::RecruitParameters { layout: None, elements: vec![5, aa_play::data::NO_DEF] },
            }])
        );
        assert!(m.dropped.is_empty());
    }
}
