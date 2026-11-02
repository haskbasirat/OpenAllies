//! Player orders of the original game interface that the battle systems do
//! not carry themselves: the formation buttons, rally points, selling a
//! structure and detaching from an HQ. Owner: UI (HUD commands).
//!
//! Behaviour-faithful reading of the original actor commands (ids of
//! `0x4DA2F0`):
//! * set_formation (0x21, the formation buttons' `FormationChange <id>`,
//!   `0x629AB0`): the organization takes one of its layout's formations;
//!   its members hold that formation's slots from the next formation turn
//!   (`0x57BEF0` reads the current formation), at once when it stands.
//! * set_rally_point (0x10): every organization recruited from the
//!   building then moves to the point ("Commands all units recruited from
//!   this building to move to target area").
//! * sell (0x14, availability `0x565140`): a structure with a
//!   ConstructionComponent (can_sell) is removed at once and the kingdom
//!   gets SellFactor (SVars 0.5) of its cost, each resource rounded
//!   (`0x565500`); refused while under siege when AllowSellUnderSiege is
//!   false (SVars_AA).
//! * hq_detach (0x20): the organization leaves its HQ.
//! * disband (10): own organizations and single actors leave the battle at
//!   once ("Permanently disbands selection"), without a refund.
use crate::data::{DefClass, DefId};
use crate::world::{ActorId, Command, KingdomId, Order, OrgId, Unit, World};
use glam::Vec2;
use std::collections::BTreeMap;

/// Player order side state (`World::player_orders`).
#[derive(Debug, Default)]
pub struct PlayerOrders {
    /// Chosen formation per organization (index into `Def::formations`).
    formations: BTreeMap<OrgId, usize>,
    /// Rally point per recruiting building.
    rally: BTreeMap<ActorId, Vec2>,
}

/// The organization's current formation (index into `Def::formations`):
/// the one chosen with the formation buttons, else the layout default.
pub fn formation(world: &World, org: OrgId) -> Option<usize> {
    let o = world.orgs.get(org as usize)?;
    let def = world.data.def(o.def);
    world.player_orders.formations.get(&org).copied().filter(|&i| i < def.formations.len()).or(def.default_formation)
}

/// The rally point of a building, if one is set.
pub fn rally_point(world: &World, building: ActorId) -> Option<Vec2> {
    world.player_orders.rally.get(&building).copied()
}

/// A structure that can be sold (ConstructionComponent, not a city).
pub fn sellable(world: &World, building: ActorId) -> bool {
    let Some(a) = world.actor(building) else { return false };
    let d = world.data.def(a.def);
    d.class == Some(DefClass::Building) && d.has_component("ConstructionComponent") && !d.city && !d.captureable
}

/// Under siege: an enemy attacks the structure now, or it is a city held
/// under siege (the under_siege actor state; 0x4D6660 reads City +0x20).
pub fn under_siege(world: &World, building: ActorId) -> bool {
    world.combat.actor(building).is_some_and(|c| c.targeted) || world.books.actor(building).is_some_and(|b| b.city_siege)
}

/// The sell value per resource (`0x565500`): floor(SellFactor * cost + 0.5).
pub fn sell_value(world: &World, def: DefId) -> Vec<f32> {
    let factor = world.data.rule_ci("SellFactor").unwrap_or(0.5);
    let d = world.data.def(def);
    let n = world.data.resources.len();
    let mut cost = d.cost_all.clone();
    if cost.len() < n {
        cost.resize(n, 0.0);
        for (i, v) in d.cost.iter().enumerate() {
            if cost[i] == 0.0 {
                cost[i] = *v;
            }
        }
    }
    cost.iter().map(|c| (factor * c + 0.5).floor()).collect()
}

/// Whether selling is refused now (the original disables the button).
pub fn sell_refused(world: &World, building: ActorId) -> Option<&'static str> {
    if !sellable(world, building) {
        return Some("this structure cannot be sold");
    }
    if crate::production::busy(world, building) {
        return Some("the structure is busy");
    }
    let allow = world.data.rule_ci("AllowSellUnderSiege").unwrap_or(1.0) != 0.0;
    if !allow && under_siege(world, building) {
        return Some("cannot sell a structure under siege");
    }
    None
}

/// Apply one of this module's commands; Err is the refusal reason.
pub fn command(world: &mut World, command: &Command) -> Result<(), String> {
    match command {
        Command::SetFormation { kingdom, orgs, formation } => {
            let mut any = false;
            for &o in orgs {
                let Some(org) = world.orgs.get(o as usize).filter(|x| x.alive && x.kingdom == *kingdom) else { continue };
                let def = world.data.def(org.def);
                if *formation >= def.formations.len() {
                    continue;
                }
                world.player_orders.formations.insert(o, *formation);
                any = true;
                // a standing regiment turns into the new formation at once
                if org.order == Order::Idle {
                    let to = org.anchor;
                    let _ = crate::movement::command(world, &Command::Move { kingdom: *kingdom, units: vec![Unit::Org(o)], to, attack_move: false });
                }
            }
            if any { Ok(()) } else { Err("no organization takes this formation".into()) }
        }
        Command::SetRallyPoint { kingdom, building, at } => {
            let a = world.actor(*building).filter(|a| a.alive && a.kingdom == Some(*kingdom)).ok_or("not your building")?;
            if world.data.def(a.def).recruits.is_empty() {
                return Err("this building does not recruit".into());
            }
            world.player_orders.rally.insert(*building, *at);
            Ok(())
        }
        Command::Sell { kingdom, building } => sell(world, *kingdom, *building),
        Command::Detach { kingdom, org } => {
            let o = world.orgs.get(*org as usize).filter(|o| o.alive && o.kingdom == *kingdom).ok_or("not your organization")?;
            if o.hq.is_none() {
                return Err("not attached to an HQ".into());
            }
            world.orgs[*org as usize].hq = None;
            Ok(())
        }
        Command::Disband { kingdom, units } => {
            let mut gone = Vec::new();
            for u in units {
                match *u {
                    Unit::Org(o) => {
                        let Some(org) = world.orgs.get(o as usize).filter(|x| x.alive && x.kingdom == *kingdom) else { continue };
                        gone.extend(org.members.iter().copied());
                        world.orgs[o as usize].alive = false;
                    }
                    Unit::Actor(a) => {
                        if world.actor(a).is_some_and(|x| x.alive && x.kingdom == Some(*kingdom) && x.org.is_none()) {
                            gone.push(a);
                        }
                    }
                }
            }
            if gone.is_empty() {
                return Err("nothing of this kingdom to disband".into());
            }
            for id in gone {
                let x = &mut world.actors[id as usize];
                x.alive = false;
                x.removed = true;
                x.order = Order::Idle;
            }
            Ok(())
        }
        other => Err(format!("{other:?} is not a player order")),
    }
}

fn sell(world: &mut World, kingdom: KingdomId, building: ActorId) -> Result<(), String> {
    let a = world.actor(building).filter(|a| a.alive && a.kingdom == Some(kingdom)).ok_or("not your building")?;
    if let Some(r) = sell_refused(world, building) {
        return Err(r.into());
    }
    let def = a.def;
    let value = sell_value(world, def);
    let n = world.data.resources.len();
    let max = world.data.rule_ci("EconomyStockpileResourceMax").unwrap_or(f32::MAX);
    {
        let k = world.books.kingdom_mut(kingdom, n);
        for (s, v) in k.stockpiled.iter_mut().zip(&value) {
            *s = (*s + v).clamp(0.0, max);
        }
    }
    crate::economy::publish(world, kingdom);
    // the structure (and an HQ's organization) is removed at once
    let org = world.actors[building as usize].org;
    let mut gone = vec![building];
    if let Some(o) = org {
        if world.data.def(world.orgs[o as usize].def).max_attachments.is_some() {
            gone.extend(world.orgs[o as usize].members.iter().copied());
            world.orgs[o as usize].alive = false;
        }
    }
    for id in gone {
        let x = &mut world.actors[id as usize];
        x.alive = false;
        x.removed = true;
        x.order = Order::Idle;
        x.queue.clear();
    }
    for r in world.orgs.iter_mut() {
        if r.hq == Some(building) {
            r.hq = None;
        }
    }
    world.player_orders.rally.remove(&building);
    Ok(())
}

/// After the battle tick. The rally points are applied where the recruit
/// completes (production, `0x4D3D20`: the new organization gets its move
/// to the building's rally point there), so nothing is left to do here;
/// matching this tick's recruits again would give them a second order.
pub fn tick(_world: &mut World) {}

/// The player orders of the original on the wire: a player's commands as the
/// order requests a client sends (aa-orders payload fields) and the decrees
/// every node of a session executes, back to the commands of the battle.
///
/// Native carriers (aa-orders schemas, the generic command 0x61EBA0 /
/// writer 0x61E6B0):
/// * the selection orders SelectionSet 332/333, SelectionAdd 12/16,
///   SelectionRemove 13/17, SelectionClear 14/18 (UIWorldState 0x640640 /
///   0x438E10 emit them as the player selects; the decree processes
///   0x5F4450 / 0x5F4490 / 0x5F44C0 / 0x5F44F0 keep each player's
///   selection), then KKC_TellSelectedCommandOrder 84/86 (process 0x61F210 ->
///   VWorldState delivery 0x4ED360) on that selection;
/// * KKC_TellActorCommandOrder 85/87 (process 0x61E220 -> 0x4D9B60) for an
///   order of one structure (its HUD action buttons);
/// * KKC_FactionUseAbilityOrder 51/70 (process 0x61E190 -> 0x4CDE00).
///
/// The generic command carries an explicit ActorCommand (mask bit 6) here.
/// The original's right click (0x620760) sends 84 without a definition and
/// every node guesses the command per actor (GuessCommandData 0x4DA570); the
/// HUD of this port decides the command itself, so it sends the definition
/// the guess would have produced (a valid order of the original's protocol;
/// the request bits of a right click differ).
///
/// One order has no exact native form: `hq_attach` (31) carries no target in
/// the original, its handler cycles to the next HQ (TellHQAttachCycle
/// 0x57B0A0), while [`Command::Attach`] names the HQ (0x574C10). Here it is
/// `hq_attach` with the HQ in the command word, which only this port's
/// decoder reads. A session with the original would attach to another HQ.
pub mod native {
    use super::*;
    use crate::world::AbilityTarget;
    use aa_orders::payload::{Field, Registry, ThingCheck};

    /// Definition registries (global pointer addresses) the payloads use.
    pub const ACTOR_COMMANDS: Registry = Registry(0xb3aa58);
    pub const OBJECT_DEFINITIONS: Registry = Registry(0xb39f58);
    pub const FORMATIONS: Registry = Registry(0xb3ab08);
    pub const ORGANIZATION_LAYOUTS: Registry = Registry(0xb3ab00);
    pub const SHARED_PROPERTIES: Registry = Registry(0xb3aa2c);
    pub const ABILITIES: Registry = Registry(0xb3ab10);

    /// Order descriptors (aa-orders native_registry.rs).
    pub const SELECTION_ADD_REQUEST: usize = 12;
    pub const SELECTION_CLEAR_REQUEST: usize = 14;
    pub const SELECTION_ADD_DECREE: usize = 16;
    pub const SELECTION_REMOVE_DECREE: usize = 17;
    pub const SELECTION_CLEAR_DECREE: usize = 18;
    pub const SELECTION_CLEAR_NAMED_DECREE: usize = 19;
    pub const FACTION_USE_ABILITY_REQUEST: usize = 51;
    pub const TELL_ACTOR_ATTACK_DECREE: usize = 65;
    pub const TELL_ACTOR_MOVE_DECREE: usize = 66;
    pub const TELL_ACTOR_ABILITY_DECREE: usize = 67;
    pub const FACTION_USE_ABILITY_DECREE: usize = 70;
    pub const TELL_SELECTED_REQUEST: usize = 84;
    pub const TELL_ACTOR_REQUEST: usize = 85;
    pub const TELL_SELECTED_DECREE: usize = 86;
    pub const TELL_ACTOR_DECREE: usize = 87;
    pub const SELECTION_SET_REQUEST: usize = 332;
    pub const SELECTION_SET_DECREE: usize = 333;

    /// ActorCommand definitions in registry order (the 39 predefined by
    /// 0x475870, actor command ids of 0x4DA2F0).
    pub mod command {
        pub const KILL: u32 = 0;
        pub const BOMBARD: u32 = 1;
        pub const REPAIR: u32 = 3;
        pub const CONSTRUCT: u32 = 4;
        pub const MOVE: u32 = 5;
        pub const STOP: u32 = 6;
        pub const DISBAND: u32 = 10;
        pub const UNPACK: u32 = 11;
        pub const RECRUIT: u32 = 15;
        pub const SET_RALLY_POINT: u32 = 16;
        pub const STRUCTURE_CANCEL_QUEUED: u32 = 18;
        pub const SELL: u32 = 20;
        pub const AIR_ATTACK: u32 = 25;
        pub const AIR_RECON: u32 = 26;
        pub const HQ_ATTACH: u32 = 31;
        pub const HQ_DETACH: u32 = 32;
        pub const SET_FORMATION: u32 = 33;
        pub const PACK: u32 = 34;
        pub const RESEARCH: u32 = 35;
    }

    /// Mask bits of the generic command (0x61EBA0 field order).
    const WORD: u32 = 1 << 0;
    const OBJECT: u32 = 1 << 1;
    const POSITION: u32 = 1 << 2;
    const AUXILIARY: u32 = 1 << 3;
    const SCALAR: u32 = 1 << 4;
    const PARAMETERS: u32 = 1 << 5;
    const DEFINITION: u32 = 1 << 6;
    const PARAMETER: u32 = 1 << 7;
    const EXTRA: u32 = 1 << 8;
    const BOOLEAN: u32 = 1 << 9;

    /// Slot order (wire index -> IDS) and counts of the registries.
    #[derive(Clone, Debug, Default)]
    pub struct Tables {
        /// Thing DataManager [0xB39F58] registration slots.
        pub objects: Vec<String>,
        /// FormationData [0xB3AB08].
        pub formations: Vec<String>,
        /// SharedProperty [0xB3AA2C] and AbilityDefinition [0xB3AB10] slots
        /// with their live counts (the slot lists may be partial).
        pub properties: Vec<String>,
        pub property_count: usize,
        pub abilities: Vec<String>,
        pub ability_count: usize,
        pub organization_layout_count: usize,
    }
    fn index_of(list: &[String], id: &str) -> Option<u32> {
        list.iter().position(|s| s.eq_ignore_ascii_case(id)).map(|i| i as u32)
    }
    impl Tables {
        fn count(&self, registry: Registry) -> usize {
            match registry {
                OBJECT_DEFINITIONS => self.objects.len(),
                FORMATIONS => self.formations.len(),
                SHARED_PROPERTIES => self.property_count,
                ABILITIES => self.ability_count,
                ORGANIZATION_LAYOUTS => self.organization_layout_count,
                _ => 39,
            }
        }
    }

    /// The generic command 0x61EBA0 with its optional fields.
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct Wire {
        pub flags: u32,
        pub command: Option<u32>,
        pub word: u32,
        pub object: Option<u32>,
        pub position: [u32; 2],
        pub auxiliary: [u32; 2],
        pub scalar: u32,
        pub parameter: Option<u32>,
        pub extra: Option<u32>,
        pub boolean: bool,
        pub trailing: bool,
    }
    impl Wire {
        fn new(command: u32) -> Self {
            Self { flags: DEFINITION, command: Some(command), ..Self::default() }
        }
        fn target(mut self, gtid: u32) -> Self {
            self.flags |= WORD;
            self.word = gtid;
            self
        }
        fn at(mut self, p: Vec2) -> Self {
            self.flags |= POSITION;
            self.position = [p.x.to_bits(), p.y.to_bits()];
            self
        }
        /// The facing as a direction (auxiliary words; the unpack / construct
        /// placement drag 0x6210B0 sends flag 8 with the facing).
        fn facing(mut self, facing: f32) -> Self {
            self.flags |= AUXILIARY;
            self.auxiliary = [facing.cos().to_bits(), facing.sin().to_bits()];
            self
        }
        fn object(mut self, index: u32) -> Self {
            self.flags |= OBJECT;
            self.object = Some(index);
            self
        }
        /// The payload fields in the schema order of aa-orders `complex::command`.
        pub fn fields(&self, tables: &Tables) -> Vec<Field> {
            let has = |bit: u32| self.flags & bit != 0;
            let def = |registry: Registry, v: Option<u32>| Field::Def { registry, index: v.unwrap_or(tables.count(registry) as u32) };
            let mut f = vec![Field::Bounded { n: 0x800, value: self.flags }];
            if has(DEFINITION) {
                f.push(def(ACTOR_COMMANDS, self.command));
            }
            if has(WORD) {
                f.push(Field::Uint { width: 32, value: self.word });
            }
            if has(OBJECT) {
                f.push(def(OBJECT_DEFINITIONS, self.object));
            }
            if has(POSITION) {
                f.extend(self.position.map(|v| Field::Uint { width: 32, value: v }));
            }
            if has(AUXILIARY) {
                f.extend(self.auxiliary.map(|v| Field::Uint { width: 32, value: v }));
            }
            if has(SCALAR) {
                f.push(Field::Uint { width: 32, value: self.scalar });
            }
            if has(PARAMETER) {
                f.push(def(FORMATIONS, self.parameter));
            }
            if has(PARAMETERS) {
                // the null parameter set (1 = null)
                f.push(Field::Uint { width: 1, value: 1 });
            }
            if has(EXTRA) {
                f.push(def(SHARED_PROPERTIES, self.extra));
            }
            if has(BOOLEAN) {
                f.push(Field::Uint { width: 1, value: u32::from(self.boolean) });
            }
            f.push(Field::Uint { width: 1, value: u32::from(self.trailing) });
            f
        }
        /// Read the decoded fields back (returns the wire and the fields after it).
        pub fn read<'f>(fields: &'f [Field], tables: &Tables) -> Result<(Self, &'f [Field]), String> {
            let mut it = fields.iter();
            let mut w = Wire::default();
            let bad = |what: &str| format!("generic command field {what}");
            let uint = |it: &mut std::slice::Iter<'f, Field>, what: &str| match it.next() {
                Some(Field::Uint { value, .. }) | Some(Field::Bounded { value, .. }) => Ok(*value),
                _ => Err(bad(what)),
            };
            let def = |it: &mut std::slice::Iter<'f, Field>, registry: Registry| match it.next() {
                Some(Field::Def { index, .. }) => Ok((*index as usize != tables.count(registry)).then_some(*index)),
                _ => Err(bad("definition")),
            };
            w.flags = uint(&mut it, "mask")?;
            let has = |bit: u32| w.flags & bit != 0;
            if has(DEFINITION) {
                w.command = def(&mut it, ACTOR_COMMANDS)?;
            }
            if has(WORD) {
                w.word = uint(&mut it, "word")?;
            }
            if has(OBJECT) {
                w.object = def(&mut it, OBJECT_DEFINITIONS)?;
            }
            if has(POSITION) {
                w.position = [uint(&mut it, "x")?, uint(&mut it, "y")?];
            }
            if has(AUXILIARY) {
                w.auxiliary = [uint(&mut it, "aux x")?, uint(&mut it, "aux y")?];
            }
            if has(SCALAR) {
                w.scalar = uint(&mut it, "scalar")?;
            }
            if has(PARAMETER) {
                w.parameter = def(&mut it, FORMATIONS)?;
            }
            if has(PARAMETERS) && uint(&mut it, "parameter set null bit")? == 0 {
                def(&mut it, ORGANIZATION_LAYOUTS)?;
                let n = uint(&mut it, "parameter count")? as i32;
                for _ in 0..n.max(0) {
                    def(&mut it, OBJECT_DEFINITIONS)?;
                }
            }
            if has(EXTRA) {
                w.extra = def(&mut it, SHARED_PROPERTIES)?;
            }
            if has(BOOLEAN) {
                w.boolean = uint(&mut it, "boolean")? == 1;
            }
            w.trailing = uint(&mut it, "trailing")? == 1;
            Ok((w, it.as_slice()))
        }
    }

    /// One order request: descriptor and payload fields.
    pub type Request = (usize, Vec<Field>);

    /// What the local player last requested as its selection.
    #[derive(Clone, Debug, Default)]
    pub struct Outbox {
        pub selection: Vec<u16>,
    }

    /// The thing slot of a unit (thing+0x14 low16, the GTID slot).
    pub fn slot_of(world: &World, unit: Unit) -> Option<u16> {
        match unit {
            Unit::Org(o) => world.ids.org(o).map(|g| g.slot()),
            Unit::Actor(a) => world.ids.actor(a).map(|g| g.slot()),
        }
    }
    fn gtid_of(world: &World, actor: ActorId) -> Option<u32> {
        world.ids.actor(actor).map(|g| g.raw())
    }

    /// The requests of one battle command of the local player: the selection
    /// orders that make the replicated selection the command's units, then
    /// the command itself.
    pub fn requests(world: &World, tables: &Tables, outbox: &mut Outbox, command: &Command) -> Result<Vec<Request>, String> {
        let mut out = Vec::new();
        let selected = |out: &mut Vec<Request>, outbox: &mut Outbox, units: &[Unit], wire: Wire| -> Result<(), String> {
            let slots: Vec<u16> = units.iter().filter_map(|u| slot_of(world, *u)).collect();
            if slots.is_empty() {
                return Err("none of the units has a thing id".into());
            }
            if slots != outbox.selection {
                out.push((SELECTION_SET_REQUEST, vec![Field::Thing { check: ThingCheck::B0, slot: slots[0] }]));
                for s in &slots[1..] {
                    out.push((SELECTION_ADD_REQUEST, vec![Field::Thing { check: ThingCheck::B0, slot: *s }]));
                }
                outbox.selection = slots;
            }
            out.push((TELL_SELECTED_REQUEST, wire.fields(tables)));
            Ok(())
        };
        let actor = |out: &mut Vec<Request>, unit: Unit, wire: Wire| -> Result<(), String> {
            let slot = slot_of(world, unit).ok_or("the structure has no thing id")?;
            let mut f = wire.fields(tables);
            f.push(Field::Thing { check: ThingCheck::B4, slot });
            out.push((TELL_ACTOR_REQUEST, f));
            Ok(())
        };
        let target = |t: ActorId| gtid_of(world, t).ok_or_else(|| format!("target {t} has no thing id"));
        match command {
            Command::Move { units, to, attack_move, .. } => {
                // kill at a location (actor_commands.tgi kill) engages on the way
                let c = if *attack_move { command::KILL } else { command::MOVE };
                selected(&mut out, outbox, units, Wire::new(c).at(*to))?;
            }
            Command::Attack { units, target: t, .. } => selected(&mut out, outbox, units, Wire::new(command::KILL).target(target(*t)?))?,
            Command::Follow { units, target: t, .. } => selected(&mut out, outbox, units, Wire::new(command::MOVE).target(target(*t)?))?,
            Command::Stop { units, .. } => selected(&mut out, outbox, units, Wire::new(command::STOP))?,
            Command::Disband { units, .. } => selected(&mut out, outbox, units, Wire::new(command::DISBAND))?,
            Command::Repair { units, target: t, .. } => selected(&mut out, outbox, units, Wire::new(command::REPAIR).target(target(*t)?))?,
            Command::Bombard { units, target: t, .. } => {
                let w = match t {
                    AbilityTarget::Actor(a) => Wire::new(command::BOMBARD).target(target(*a)?),
                    AbilityTarget::Point(p) => Wire::new(command::BOMBARD).at(*p),
                };
                selected(&mut out, outbox, units, w)?;
            }
            Command::Construct { units, def, at, facing, .. } => {
                let index = index_of(&tables.objects, &world.data.def(*def).id).ok_or("structure is not in the Thing registry")?;
                selected(&mut out, outbox, units, Wire::new(command::CONSTRUCT).object(index).at(*at).facing(*facing))?;
            }
            Command::Attach { org, hq, .. } => actor(&mut out, Unit::Org(*org), Wire::new(command::HQ_ATTACH).target(target(*hq)?))?,
            Command::SetFormation { orgs, formation, .. } => {
                let units: Vec<Unit> = orgs.iter().map(|o| Unit::Org(*o)).collect();
                let def = orgs.first().and_then(|o| world.orgs.get(*o as usize)).map(|o| world.data.def(o.def)).ok_or("no organization")?;
                let id = def.formations.get(*formation).map(|f| f.id.clone()).ok_or("no such formation")?;
                let index = index_of(&tables.formations, &id).ok_or("formation is not in the FormationData registry")?;
                let mut w = Wire::new(command::SET_FORMATION);
                w.flags |= PARAMETER;
                w.parameter = Some(index);
                selected(&mut out, outbox, &units, w)?;
            }
            Command::Recruit { building, def, .. } => {
                let index = index_of(&tables.objects, &world.data.def(*def).id).ok_or("recruit is not in the Thing registry")?;
                actor(&mut out, Unit::Actor(*building), Wire::new(command::RECRUIT).object(index))?;
            }
            Command::CancelRecruit { building, index, .. } => {
                let mut w = Wire::new(command::STRUCTURE_CANCEL_QUEUED);
                w.flags |= SCALAR;
                w.scalar = *index as u32;
                actor(&mut out, Unit::Actor(*building), w)?;
            }
            Command::Deploy { truck, at, facing, .. } => actor(&mut out, Unit::Actor(*truck), Wire::new(command::UNPACK).at(*at).facing(*facing))?,
            Command::Pack { building, .. } => actor(&mut out, Unit::Actor(*building), Wire::new(command::PACK))?,
            Command::Sell { building, .. } => actor(&mut out, Unit::Actor(*building), Wire::new(command::SELL))?,
            Command::SetRallyPoint { building, at, .. } => actor(&mut out, Unit::Actor(*building), Wire::new(command::SET_RALLY_POINT).at(*at))?,
            Command::Detach { org, .. } => actor(&mut out, Unit::Org(*org), Wire::new(command::HQ_DETACH))?,
            Command::Research { building, property, .. } => {
                let index = index_of(&tables.properties, property).ok_or("property is not in the SharedProperty registry")?;
                let mut w = Wire::new(command::RESEARCH);
                w.flags |= EXTRA;
                w.extra = Some(index);
                actor(&mut out, Unit::Actor(*building), w)?;
            }
            Command::AirAttack { building, target: t, .. } | Command::AirRecon { building, target: t, .. } => {
                let c = if matches!(command, Command::AirAttack { .. }) { command::AIR_ATTACK } else { command::AIR_RECON };
                let w = match t {
                    AbilityTarget::Actor(a) => Wire::new(c).target(target(*a)?),
                    AbilityTarget::Point(p) => Wire::new(c).at(*p),
                };
                actor(&mut out, Unit::Actor(*building), w)?;
            }
            Command::UseAbility { kingdom, ability, target: t } => {
                // 0x6109F0: definition, two words (the point), presence bit, actor.
                let general = crate::special_ops::general(world, *kingdom).ok_or("no general")?;
                let id = general.abilities.get(*ability as usize).map(|a| a.id.clone()).ok_or("no such ability")?;
                let index = index_of(&tables.abilities, &id).ok_or("ability is not in the AbilityDefinition registry")?;
                let mut f = vec![Field::Def { registry: ABILITIES, index }];
                let (p, actor) = match t {
                    Some(AbilityTarget::Point(p)) => (*p, None),
                    Some(AbilityTarget::Actor(a)) => (world.actor(*a).map(|x| x.pos.truncate()).unwrap_or_default(), slot_of(world, Unit::Actor(*a))),
                    None => (Vec2::ZERO, None),
                };
                f.push(Field::Uint { width: 32, value: p.x.to_bits() });
                f.push(Field::Uint { width: 32, value: p.y.to_bits() });
                match actor {
                    Some(slot) => {
                        f.push(Field::Uint { width: 1, value: 1 });
                        f.push(Field::Thing { check: ThingCheck::B4, slot });
                    }
                    None => f.push(Field::Uint { width: 1, value: 0 }),
                }
                out.push((FACTION_USE_ABILITY_REQUEST, f));
            }
        }
        Ok(out)
    }

    /// Decree replay state: every session player's selection.
    #[derive(Clone, Debug, Default)]
    pub struct Replay {
        pub selections: BTreeMap<u32, Vec<u16>>,
    }

    fn point(bits: [u32; 2]) -> Vec2 {
        Vec2::new(f32::from_bits(bits[0]), f32::from_bits(bits[1]))
    }
    fn unit_at(world: &World, slot: u16) -> Option<Unit> {
        crate::ids::unit(world.ids.thing_at_slot(slot)?)
    }
    fn actor_at_gtid(world: &World, gtid: u32) -> Option<ActorId> {
        let unit = crate::ids::unit(world.ids.thing(gtid)?)?;
        crate::ids::target_actor(world, unit)
    }

    /// The battle commands of one executed decree of `player` (who orders
    /// for `kingdom`). Selection decrees only change the replay state.
    pub fn commands(
        world: &World,
        tables: &Tables,
        replay: &mut Replay,
        kingdom: Option<KingdomId>,
        player: u32,
        descriptor: usize,
        fields: &[Field],
    ) -> Result<Vec<Command>, String> {
        let thing = |f: Option<&Field>| match f {
            Some(Field::Thing { slot, .. }) => Ok(*slot),
            other => Err(format!("thing field {other:?}")),
        };
        match descriptor {
            SELECTION_SET_DECREE => {
                let slot = thing(fields.first())?;
                let s = replay.selections.entry(player).or_default();
                s.clear();
                if slot != 0 {
                    s.push(slot);
                }
                return Ok(Vec::new());
            }
            SELECTION_ADD_DECREE => {
                let slot = thing(fields.first())?;
                let s = replay.selections.entry(player).or_default();
                if slot != 0 && !s.contains(&slot) {
                    s.push(slot);
                }
                return Ok(Vec::new());
            }
            SELECTION_REMOVE_DECREE => {
                let slot = thing(fields.first())?;
                replay.selections.entry(player).or_default().retain(|s| *s != slot);
                return Ok(Vec::new());
            }
            SELECTION_CLEAR_DECREE | SELECTION_CLEAR_NAMED_DECREE => {
                replay.selections.entry(player).or_default().clear();
                return Ok(Vec::new());
            }
            _ => {}
        }
        let Some(kingdom) = kingdom else { return Err(format!("player {player} orders no kingdom")) };
        match descriptor {
            TELL_SELECTED_DECREE | TELL_ACTOR_DECREE => {
                let (wire, rest) = Wire::read(fields, tables)?;
                let slots = if descriptor == TELL_ACTOR_DECREE {
                    vec![thing(rest.first())?]
                } else {
                    replay.selections.get(&player).cloned().unwrap_or_default()
                };
                // References the reader resolved to null are skipped (0x76A5A0).
                let units: Vec<Unit> = slots.iter().filter_map(|s| unit_at(world, *s)).collect();
                if units.is_empty() {
                    return Ok(Vec::new());
                }
                decode_command(world, tables, kingdom, &wire, units)
            }
            FACTION_USE_ABILITY_DECREE => {
                let index = match fields.first() {
                    Some(Field::Def { index, .. }) => *index,
                    other => return Err(format!("ability field {other:?}")),
                };
                let words = match (fields.get(1), fields.get(2)) {
                    (Some(Field::Uint { value: x, .. }), Some(Field::Uint { value: y, .. })) => [*x, *y],
                    _ => return Err("ability point".into()),
                };
                let target_slot = match (fields.get(3), fields.get(4)) {
                    (Some(Field::Uint { value: 1, .. }), Some(Field::Thing { slot, .. })) => Some(*slot),
                    _ => None,
                };
                let id = tables.abilities.get(index as usize).ok_or("ability slot")?;
                let general = crate::special_ops::general(world, kingdom).ok_or("no general")?;
                let ability = general.abilities.iter().position(|a| a.id.eq_ignore_ascii_case(id)).ok_or("the general has no such ability")? as u32;
                let target = match target_slot {
                    Some(slot) => match unit_at(world, slot).and_then(|u| crate::ids::target_actor(world, u)) {
                        Some(a) => Some(AbilityTarget::Actor(a)),
                        None => return Ok(Vec::new()),
                    },
                    None => Some(AbilityTarget::Point(point(words))),
                };
                Ok(vec![Command::UseAbility { kingdom, ability, target }])
            }
            TELL_ACTOR_ATTACK_DECREE => {
                let (Some(unit), Some(target)) = (unit_at(world, thing(fields.first())?), unit_at(world, thing(fields.get(1))?).and_then(|u| crate::ids::target_actor(world, u))) else {
                    return Ok(Vec::new());
                };
                Ok(vec![Command::Attack { kingdom, units: vec![unit], target }])
            }
            // These two name their actor by text (0x410B50, process 0x61E830 /
            // 0x61E100, no battle command of this port stands for them).
            TELL_ACTOR_MOVE_DECREE | TELL_ACTOR_ABILITY_DECREE => Err(format!("decree {descriptor} names its actor by text: not decoded")),
            _ => Ok(Vec::new()),
        }
    }

    fn decode_command(world: &World, tables: &Tables, kingdom: KingdomId, w: &Wire, units: Vec<Unit>) -> Result<Vec<Command>, String> {
        let has = |bit: u32| w.flags & bit != 0;
        let Some(c) = w.command else { return Err("a command without ActorCommand (guessed commands are not decoded)".into()) };
        let actors = || -> Vec<ActorId> { units.iter().filter_map(|u| if let Unit::Actor(a) = u { Some(*a) } else { None }).collect() };
        let orgs = || -> Vec<OrgId> { units.iter().filter_map(|u| if let Unit::Org(o) = u { Some(*o) } else { None }).collect() };
        let target = || has(WORD).then(|| actor_at_gtid(world, w.word)).flatten();
        let facing = f32::from_bits(w.auxiliary[1]).atan2(f32::from_bits(w.auxiliary[0]));
        let def = |index: Option<u32>| index.and_then(|i| tables.objects.get(i as usize)).and_then(|id| world.data.find(id));
        let out = match c {
            command::KILL if has(WORD) => match target() {
                Some(t) => vec![Command::Attack { kingdom, units, target: t }],
                None => Vec::new(),
            },
            command::KILL | command::MOVE if has(POSITION) && !has(WORD) => {
                vec![Command::Move { kingdom, units, to: point(w.position), attack_move: c == command::KILL }]
            }
            command::MOVE if has(WORD) => match target() {
                Some(t) => vec![Command::Follow { kingdom, units, target: t }],
                None => Vec::new(),
            },
            command::STOP => vec![Command::Stop { kingdom, units }],
            command::DISBAND => vec![Command::Disband { kingdom, units }],
            command::REPAIR => match target() {
                Some(t) => vec![Command::Repair { kingdom, units, target: t }],
                None => Vec::new(),
            },
            command::BOMBARD => {
                let t = if has(WORD) { target().map(AbilityTarget::Actor) } else { Some(AbilityTarget::Point(point(w.position))) };
                t.map(|target| vec![Command::Bombard { kingdom, units, target }]).unwrap_or_default()
            }
            command::CONSTRUCT => match def(w.object) {
                Some(def) => vec![Command::Construct { kingdom, units, def, at: point(w.position), facing }],
                None => Vec::new(),
            },
            command::HQ_ATTACH => match target() {
                Some(hq) => orgs().into_iter().map(|org| Command::Attach { kingdom, org, hq }).collect(),
                None => Vec::new(),
            },
            command::SET_FORMATION => {
                let id = w.parameter.and_then(|i| tables.formations.get(i as usize)).ok_or("formation slot")?;
                orgs()
                    .into_iter()
                    .filter_map(|o| {
                        let d = world.data.def(world.orgs.get(o as usize)?.def);
                        let formation = d.formations.iter().position(|f| f.id.eq_ignore_ascii_case(id))?;
                        Some(Command::SetFormation { kingdom, orgs: vec![o], formation })
                    })
                    .collect()
            }
            command::RECRUIT => match def(w.object) {
                Some(def) => actors().into_iter().map(|building| Command::Recruit { kingdom, building, def }).collect(),
                None => Vec::new(),
            },
            command::STRUCTURE_CANCEL_QUEUED => {
                actors().into_iter().map(|building| Command::CancelRecruit { kingdom, building, index: w.scalar as usize }).collect()
            }
            command::UNPACK => actors().into_iter().map(|truck| Command::Deploy { kingdom, truck, at: point(w.position), facing }).collect(),
            command::PACK => actors().into_iter().map(|building| Command::Pack { kingdom, building }).collect(),
            command::SELL => actors().into_iter().map(|building| Command::Sell { kingdom, building }).collect(),
            command::SET_RALLY_POINT => actors().into_iter().map(|building| Command::SetRallyPoint { kingdom, building, at: point(w.position) }).collect(),
            command::HQ_DETACH => orgs().into_iter().map(|org| Command::Detach { kingdom, org }).collect(),
            command::RESEARCH => {
                let property = w.extra.and_then(|i| tables.properties.get(i as usize)).cloned().ok_or("SharedProperty slot")?;
                actors().into_iter().map(|building| Command::Research { kingdom, building, property: property.clone() }).collect()
            }
            command::AIR_ATTACK | command::AIR_RECON => {
                let t = if has(WORD) { target().map(AbilityTarget::Actor) } else { Some(AbilityTarget::Point(point(w.position))) };
                match t {
                    Some(target) => actors()
                        .into_iter()
                        .map(|building| if c == command::AIR_ATTACK { Command::AirAttack { kingdom, building, target } } else { Command::AirRecon { kingdom, building, target } })
                        .collect(),
                    None => Vec::new(),
                }
            }
            other => return Err(format!("ActorCommand {other} has no battle command")),
        };
        Ok(out)
    }
}
