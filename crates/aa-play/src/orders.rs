//! Native actor commands that had no battle system of their own, and the Mind
//! states behind them. Owner: orders.
//!
//! The original delivers a command (`KKC_GActorCommand`, decree 86 / 87) through
//! 0x4ED360, 0x4CF430 and 0x4D9B60 to the component of the command definition's
//! slot. A slot 4 command reaches the Mind, whose subtype table (`aa_mind::command`
//! `HANDLER_SLOTS`) gives the regiment Mind (vtable 0xA335A0) these handlers:
//!
//! | command | handler | what it does |
//! |---|---|---|
//! | move 5 | 0x527E30 | to a location: 0x520440; to a hostile actor the kill path; else 0x520320 |
//! | kill 0 | 0x527FB0 | bombard when the layout fires indirect (0x578AC0, 0x57A5E0), else 0x51D920, else 0x520320 |
//! | bombard 1 | 0x528140 | the bombard state |
//! | guard 2 | 0x51F360 | the guard state on an actor (0x51DC80) or a point (0x51DD30) |
//! | construct 4 | 0x51F460 | the construct state |
//! | repair 3 | 0x51F5A0 | the repair state |
//! | stop 6 | 0x51D1C0 | the sleep state |
//! | retreat 7 | 0x51D1E0 | the move handler, then 0x579AB0 if the engage state was current and an enemy is engaged with it |
//! | rout 8 | 0x51D240 | the rout state unless Actor +104 bit 2 |
//! | explore 9 | 0x5249A0 | the explore state, or a new explore point when one runs (scalar 1.0) |
//! | unpack 11 | 0x51E550 | the unpack state |
//! | pack_organization 12 | 0x51E5B0 | the pack state, which orders its leader |
//!
//! The other slots: Organization 0x57C760, Denizen 0x56E730, Structure 0x589D90,
//! Settlement 0x5882B0, Construction 0x5667F0, Recruit 0x583A70,
//! MobileDeployment 0x576650, Research 0x584980, Flare 0x571CF0.
//!
//! **What is a rendition.** The regiment Mind's Move, Kill and fighting states are hosted
//! on the arena (`crate::movement`, `crate::minds`). The states that stay here (`explore`,
//! the retreat's bookkeeping) are the native Think's decisions over aa-play's `Order`, with the
//! native addresses cited: they follow the exact ports (`aa_mind::cai_thinks::explore`) branch
//! by branch but are not fixture-proven. What is exact is said where it is: the command mask
//! `mask` hosts the exact contributors, and the arithmetic of `group` is fixture-proven.
pub mod explore;
pub mod group;
pub mod guard;
pub mod guess;
pub mod mask;

use crate::data::DefId;
use crate::world::{ActorId, Command, KingdomId, Order, OrgId, Unit, World};
use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// An ActorCommand with no battle command of its own, as a film or the HUD
/// gives it (`Command::Order`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ActorOrder {
    /// The next attack is a `move` command on a hostile target (`0x527E30` ->
    /// `0x51D920`, Mind +0x20's kind 5): the Kill state reports mind_moving.
    MoveOnto,
    /// guard (2), 0x51F360: `target` is the guarded thing; without one the
    /// regiment stands at `at`. `aux` is the direction it stands from a
    /// guarded thing (flag bit 3, Guard state +24, read by 0x51EE20).
    Guard { target: Option<Unit>, at: Vec2, aux: Option<Vec2> },
    /// retreat (7), 0x51D1E0: a move with the retreat formation, passive, at a
    /// morale cost if engaged. A hostile target goes through the kill path.
    Retreat { target: Option<ActorId>, at: Vec2 },
    /// rout (8), 0x51D240.
    Rout,
    /// explore (9), 0x5249A0. A scalar of 1.0 on a running explore state only
    /// moves its explore point.
    Explore { at: Vec2, scalar: Option<f32> },
    /// pack_organization (12), 0x51E5B0: the organization's leader packs.
    PackOrganization,
    /// hq_attach (31), TellHQAttachCycle 0x57B0A0: the next HQ. It reads no
    /// field of the command, so a target in the film is ignored.
    HqCycle,
    /// detach (30), 0x578940: drop a member's inventory attachment.
    DetachElement,
    /// attach (29), TellAttach 0x57B830: a member takes an inventory actor.
    InventoryAttach { def: DefId },
    /// recruit_location (17), 0x5836E0 with the point; the ParameterSet as for
    /// `Command::RecruitWith`.
    RecruitAt { def: DefId, at: Vec2, parameters: Option<crate::world::RecruitParameters> },
    /// commission (14), 0x5838B0.
    Commission { def: DefId },
    /// build_structure (13), 0x587FD0.
    BuildStructure { def: DefId, at: Vec2 },
    /// structure_cancel_queued (18), 0x589BD0: the queue item with this id
    /// (the command's scalar through ftol).
    CancelQueued { id: u32 },
    /// construction_cancel (19), 0x565860.
    ConstructionCancel,
    /// upgrade (21), 0x566360.
    Upgrade { def: DefId },
    /// raze (22), 0x564DC0.
    Raze,
    /// sally_forth (23) and recall (24): Denizen +20 (0x56E771, 0x56E784).
    Sally { forth: bool },
    /// escort_on (27) and escort_off (28): Denizen +21 (0x56E80A, 0x56E81D).
    Escort { on: bool },
    /// cancel_flare (36), 0x571CF0: the flare is destroyed.
    CancelFlare,
    /// group_behavior_enable (37) and group_behavior_disable (38), 0x4ED360:
    /// the selection's group +18 and the boolean into +20.
    GroupBehavior { enable: bool, boolean: Option<bool> },
    /// 0x4ED360 with the group behaviour on, for an order that names no selection
    /// here (an unpack, a recruit): the selection becomes a new GGroup (0x4FBE10,
    /// 0x4E0940) before the order goes to each. A command with `units` makes its
    /// group by itself (`group::selection`).
    GroupFormed,
    /// unpack (11) to a group (0x4DF840 -> 0x4DE7F0): the thing's leader unpacks, and
    /// a member without the command takes a Move there. A selection that is not a
    /// group gives `Command::Deploy` itself.
    Unpack { at: Vec2, facing: f32 },
    /// Flag bit 3 of the command that follows: the direction a move ends in (0x527E30:
    /// command +20 into the Move state's +24, which Enter 0x523080 writes to Mind
    /// +2B4). A move reads it through `next_aux`; a guard has its own.
    Facing { aux: Vec2 },
    /// 0x4ED360's lower path over a selection whose things do not all take one command
    /// (each guessed its own, or one holds no such command): 0x4ED280's centre is the
    /// whole selection's, `units`, for the commands up to the `on: false`.
    SelectionSpread { on: bool },
    /// repair (3) with flag bit 9 and +30 set (0x51F5A0 -> 0x51ECF0): the repair state's
    /// +18, which stays on a target that is whole (0x5239A0 step 5, 0x5359D0).
    RepairKept { target: ActorId },
    /// team_give_resources (23 TeamCommand 3), 0x509840: `amount` of the team
    /// resource goes from one kingdom to another.
    GiveResources { to: KingdomId, amount: f32 },
    /// team_give_actor (4), TributeActor 0x509440: the actor changes owner.
    GiveActor { to: KingdomId, actor: Unit },
}

/// Which Mind state an order put an organization in.
#[derive(Clone, Debug)]
pub(crate) enum MindState {
    /// The Guard state is on the regiment Mind's stack (`crate::movement`'s fight).
    Guard,
    /// The move of a retreat (actor state 13 `mind_retreating`, 0x5202DD): the
    /// order it wrote; a different order ended it.
    Retreat { order: Order },
    /// The Explore state is on the regiment Mind's stack (`crate::movement`'s fight).
    Explore,
}

/// Order side state (`World::orders`).
#[derive(Clone, Debug, Default)]
pub struct OrderState {
    pub(crate) orgs: BTreeMap<OrgId, MindState>,
    /// The GGroups and the selection settings (`group`).
    pub groups: group::Groups,
    /// The selection's centre of a command its things take apart (`ActorOrder::SelectionSpread`).
    pub(crate) selection_center: Option<Vec2>,
    /// The things whose repair state has +18 (`ActorOrder::RepairKept`).
    pub(crate) repair_kept: Vec<Unit>,
    /// `ActorOrder::Facing`, for the command being applied.
    pub next_aux: Option<Vec2>,
    /// `ActorOrder::MoveOnto`: the organizations whose next kill order came from a
    /// `move` command (`0x527E30`), until `crate::combat` reads it.
    pub move_onto: Vec<OrgId>,
}

/// A thing's position as the guard state reads it.
pub(crate) fn guard_position(world: &World, unit: Unit) -> Option<Vec2> {
    guard::position(world, unit)
}

/// Whether organization `org` is on a retreat: its formation is the layout's
/// retreat formation (`mind_retreating` formation_required RETREAT) and it
/// fights nothing on the way (behavior_required PASSIVE).
pub fn retreating(world: &World, org: OrgId) -> bool {
    // The Move state's leave ends it (0x520250 reports for the current Move only), before
    // the Sleep that follows in the same update takes its formation.
    matches!(world.orders.orgs.get(&org), Some(MindState::Retreat { order }) if world.orgs.get(org as usize).is_some_and(|o| o.order == *order))
}
/// The organization's order comes from a `move` command on a target
/// ([`ActorOrder::MoveOnto`]); read once.
pub(crate) fn take_move_onto(world: &mut World, org: OrgId) -> bool {
    let n = world.orders.move_onto.len();
    world.orders.move_onto.retain(|&o| o != org);
    world.orders.move_onto.len() != n
}
/// Whether organization `org` is in the guard state (`mind_guarding`).
pub fn guarding(world: &World, org: OrgId) -> bool {
    matches!(world.orders.orgs.get(&org), Some(MindState::Guard))
}
/// Whether organization `org` is in the explore state (`mind_exploring`).
pub fn exploring(world: &World, org: OrgId) -> bool {
    matches!(world.orders.orgs.get(&org), Some(MindState::Explore))
}

/// A new Mind command replaces the state an order put the organization in
/// (0x52A420 leaves the old state). `World::apply` calls this first.
pub(crate) fn superseded(world: &mut World, command: &Command) {
    let clear = |world: &mut World, unit: Unit| {
        crate::combat::tasks::superseded(world, unit);
        world.orders.repair_kept.retain(|u| *u != unit);
        let org = match unit {
            Unit::Org(o) => Some(o),
            Unit::Actor(a) => world.actors.get(a as usize).and_then(|x| x.org),
        };
        if let Some(o) = org {
            world.orders.orgs.remove(&o);
        }
    };
    match command {
        Command::Move { units, .. }
        | Command::Attack { units, .. }
        | Command::Stop { units, .. }
        | Command::Follow { units, .. }
        | Command::Bombard { units, .. }
        | Command::Repair { units, .. }
        | Command::Construct { units, .. }
        | Command::Disband { units, .. } => {
            for &u in units {
                clear(world, u);
            }
        }
        Command::Order { units, order: ActorOrder::PackOrganization | ActorOrder::Unpack { .. } | ActorOrder::RepairKept { .. }, .. } => {
            for &u in units {
                clear(world, u);
            }
        }
        Command::Deploy { truck, .. } => clear(world, Unit::Actor(*truck)),
        Command::Pack { building, .. } => clear(world, Unit::Actor(*building)),
        _ => {}
    }
}

fn own_org(world: &World, kingdom: KingdomId, unit: Unit) -> Option<OrgId> {
    match unit {
        Unit::Org(o) => world.orgs.get(o as usize).filter(|x| x.alive && x.kingdom == kingdom).map(|_| o),
        Unit::Actor(_) => None,
    }
}
/// An actor order goes to the actor; selected on an organization it goes to
/// its leader (0x4D9B60: +7C -> +44 when the slot's component is absent).
fn acting(world: &World, kingdom: KingdomId, unit: Unit) -> Option<ActorId> {
    let a = crate::ids::target_actor(world, unit)?;
    world.actor(a).filter(|x| x.alive && x.kingdom == Some(kingdom)).map(|_| a)
}

/// The target as `kingdom` sees it: whether vt+16C (0x4D1F40 -> vt+140) holds.
pub fn hostile_to(world: &World, kingdom: KingdomId, target: ActorId) -> bool {
    let owner = world.actors.get(target as usize).and_then(|a| a.kingdom);
    crate::setup::relation(world, Some(kingdom), owner) == crate::books::ENEMY
}

/// Apply a `Command::Order`; Err is the refusal reason.
pub fn command(world: &mut World, command: &Command) -> Result<(), String> {
    let Command::Order { kingdom, units, order } = command else {
        return Err(format!("{command:?} is not an actor order"));
    };
    let kingdom = *kingdom;
    match order {
        ActorOrder::Guard { target, at, aux } => {
            // 0x51F360: an unresolvable target refuses; the handler of an
            // element Mind is 0x969B10 (false).
            let mut any = false;
            for &u in units {
                if let Some(o) = own_org(world, kingdom, u) {
                    any |= guard::begin(world, o, *target, *at, *aux);
                }
            }
            if any { Ok(()) } else { Err("nothing selected can guard that".into()) }
        }
        ActorOrder::Retreat { target, at } => retreat(world, kingdom, units, *target, *at),
        ActorOrder::Rout => {
            let mut st = std::mem::take(&mut world.combat);
            st.prepare(world);
            let mut any = false;
            for &u in units {
                if let Some(o) = own_org(world, kingdom, u) {
                    // 52A810(2): the exhausted state (priority 7) outranks it.
                    if !st.orgs[o as usize].exhausted_state {
                        world.orders.orgs.remove(&o);
                        crate::morale::begin_rout(world, &mut st, o);
                        any = true;
                    }
                }
            }
            world.combat = st;
            if any { Ok(()) } else { Err("nothing selected can rout".into()) }
        }
        ActorOrder::Explore { at, scalar } => {
            let mut any = false;
            for &u in units {
                if let Some(o) = own_org(world, kingdom, u) {
                    explore::begin(world, o, *at, *scalar);
                    any = true;
                }
            }
            if any { Ok(()) } else { Err("nothing selected can explore".into()) }
        }
        ActorOrder::PackOrganization => {
            // 0x51E5B0 -> KKC_CAIStatePack 0x527090: the leader's pack action.
            let mut last = Err("nothing selected can pack".to_string());
            for &u in units {
                if let Some(leader) = acting(world, kingdom, u) {
                    last = crate::deploy::command(world, &Command::Pack { kingdom, building: leader });
                }
            }
            last
        }
        ActorOrder::HqCycle => {
            let mut any = false;
            for &u in units {
                if let Some(o) = own_org(world, kingdom, u) {
                    any |= hq_cycle(world, kingdom, o);
                }
            }
            if any { Ok(()) } else { Err("ignoring HQ cycle command with nothing to cycle to".into()) }
        }
        ActorOrder::CancelQueued { id } => {
            let mut any = false;
            for &u in units {
                let Some(building) = acting(world, kingdom, u) else { continue };
                // 0x589BD0 finds the item by its id; -1 refuses.
                let index = world.books.actor(building).and_then(|b| b.queue.items.iter().position(|i| i.id == *id));
                if let Some(index) = index {
                    any |= crate::production::cancel(world, building, index).is_some();
                }
            }
            if any { Ok(()) } else { Err("no such queue entry".into()) }
        }
        ActorOrder::RecruitAt { def, at, parameters } => {
            let mut last = Err("nothing selected recruits".to_string());
            for &u in units {
                if let Some(building) = acting(world, kingdom, u) {
                    last = crate::production::recruit_at(world, kingdom, building, *def, parameters.as_ref(), *at);
                }
            }
            last
        }
        ActorOrder::Sally { forth } => denizen_flag(world, kingdom, units, |d| d.sally = *forth),
        ActorOrder::Escort { on } => denizen_flag(world, kingdom, units, |d| d.escort = *on),
        // 0x4ED360 cases 0x25 / 0x26: selection settings +18, flag bit 9 into +20.
        ActorOrder::GroupBehavior { enable, boolean } => {
            let settings = world.orders.groups.settings.entry(kingdom).or_default();
            settings.on = *enable;
            if let Some(b) = boolean {
                settings.together = *b;
            }
            Ok(())
        }
        ActorOrder::RepairKept { target } => {
            world.orders.repair_kept.extend(units.iter().copied());
            let result = crate::combat::tasks::command(world, &Command::Repair { kingdom, units: units.clone(), target: *target });
            if result.is_err() {
                world.orders.repair_kept.retain(|u| !units.contains(u));
            }
            result
        }
        ActorOrder::SelectionSpread { on } => {
            world.orders.selection_center = on.then(|| group::center_of(world, units)).flatten();
            Ok(())
        }
        ActorOrder::GroupFormed => {
            group::formed(world, kingdom, units);
            Ok(())
        }
        ActorOrder::Facing { aux } => {
            world.orders.next_aux = Some(*aux);
            Ok(())
        }
        ActorOrder::Unpack { at, facing } => {
            // 0x51E550 -> 0x51E2E0: the regiment Mind's unpack state, which orders
            // its leader; a truck takes it itself.
            let mut last = Err("nothing selected can unpack".to_string());
            for &u in units {
                if let Some(truck) = acting(world, kingdom, u) {
                    last = crate::deploy::command(world, &Command::Deploy { kingdom, truck, at: *at, facing: *facing });
                }
            }
            last
        }
        ActorOrder::MoveOnto => {
            for &u in units {
                if let Unit::Org(o) = u {
                    world.orders.move_onto.push(o);
                }
            }
            Ok(())
        }
        ActorOrder::GiveResources { to, amount } => give_resources(world, kingdom, *to, *amount),
        ActorOrder::GiveActor { to, actor } => give_actor(world, *to, *actor),
        // Reached by no film of the corpus and with no host in aa-play yet.
        ActorOrder::Commission { .. } => Err("commission (0x5838B0): no settlement host yet".into()),
        ActorOrder::BuildStructure { .. } => Err("build_structure (0x587FD0): no settlement host yet".into()),
        ActorOrder::ConstructionCancel => construction_cancel(world, kingdom, units),
        ActorOrder::Upgrade { .. } => Err("upgrade (0x566360): no host yet".into()),
        ActorOrder::Raze => raze(world, kingdom, units),
        ActorOrder::DetachElement => Err("detach (0x578940): inventory attachments have no host yet".into()),
        ActorOrder::InventoryAttach { .. } => Err("attach (0x57B830): inventory attachments have no host yet".into()),
        ActorOrder::CancelFlare => Err("cancel_flare (0x571CF0): flares have no host yet".into()),
    }
}

/// construction_cancel (19), `aa_production::construction::cancel_command` 0x565860:
/// under construction 0x5656B0: the paid cost back and the structure gone. A
/// structure from BuildActor 0x563160 stores no paid cost, so nothing comes back.
/// An upgrade in progress cannot exist here (no definition of the data lists one);
/// razed (bit 30) 0x565400: state 0x1B again.
fn construction_cancel(world: &mut World, kingdom: KingdomId, units: &[Unit]) -> Result<(), String> {
    let mut any = false;
    for &u in units {
        let Some(a) = acting(world, kingdom, u) else { continue };
        let Some(book) = world.books.actor(a) else { continue };
        if book.constructing {
            let x = &mut world.actors[a as usize];
            x.alive = false;
            x.removed = true;
            x.order = Order::Idle;
            x.queue.clear();
            any = true;
        } else if book.razed {
            world.books.actor_mut(a).razed = false;
            any = true;
        }
    }
    if any { Ok(()) } else { Err("nothing selected to cancel".into()) }
}

/// raze (22), 0x564DC0: state 0x1E, `structure_razing`, until a cancel (bit 19 of
/// the mask, 0x565140). The data's can_raze (+4E5) and the mask are `guess`'s.
fn raze(world: &mut World, kingdom: KingdomId, units: &[Unit]) -> Result<(), String> {
    let mut any = false;
    for &u in units {
        let Some(a) = acting(world, kingdom, u) else { continue };
        if world.data.def(world.actors[a as usize].def).has_component("ConstructionComponent") {
            world.books.actor_mut(a).razed = true;
            any = true;
        }
    }
    if any { Ok(()) } else { Err("nothing selected can raze".into()) }
}

fn denizen_flag(world: &mut World, kingdom: KingdomId, units: &[Unit], set: impl Fn(&mut crate::books::DenizenBooks)) -> Result<(), String> {
    let mut any = false;
    for &u in units {
        // 0x4D9B60: slot 10; an organization passes it to its leader.
        let Some(a) = acting(world, kingdom, u) else { continue };
        if let Some(d) = world.books.actor_mut(a).denizen.as_mut() {
            set(d);
            any = true;
        }
    }
    if any { Ok(()) } else { Err("no denizen component selected".into()) }
}

/// 0x51D1E0: the engage state is current and 0x4D0D10 finds an enemy engaged
/// with this actor, then the move handler 0x527E30, then 0x579AB0.
fn retreat(world: &mut World, kingdom: KingdomId, units: &[Unit], target: Option<ActorId>, at: Vec2) -> Result<(), String> {
    let orgs: Vec<OrgId> = units.iter().filter_map(|&u| own_org(world, kingdom, u)).collect();
    let mut st = std::mem::take(&mut world.combat);
    st.prepare(world);
    let engaged: Vec<OrgId> = orgs.iter().copied().filter(|&o| st.orgs[o as usize].engaging() && !st.orgs[o as usize].engaged_by.is_empty()).collect();
    world.combat = st;
    let result = match target {
        Some(t) if hostile_to(world, kingdom, t) => crate::movement::command(world, &Command::Attack { kingdom, units: units.to_vec(), target: t }),
        Some(t) => crate::movement::command(world, &Command::Follow { kingdom, units: units.to_vec(), target: t }),
        None => crate::movement::command(world, &Command::Move { kingdom, units: units.to_vec(), to: at, attack_move: false }),
    };
    result?;
    // 0x5202DD: actor state 13 is the move state's under a retreat command; the
    // kill path of a hostile target is the kill state.
    for &o in &orgs {
        let order = world.orgs[o as usize].order.clone();
        if matches!(order, Order::Move { .. }) {
            world.orders.orgs.insert(o, MindState::Retreat { order });
        }
    }
    let mut st = std::mem::take(&mut world.combat);
    for o in engaged {
        crate::morale::retreat_penalty(world, &mut st, o);
    }
    world.combat = st;
    Ok(())
}

/// The HQs organization `o` could attach to, in the kingdom's order (Kingdom
/// +2DC; 0x574A10(org, 0, 0)). The list holds aa-play's HQ organizations by
/// id, the order they were created in.
pub(crate) fn hq_cycle_list(world: &World, o: OrgId) -> Vec<ActorId> {
    let Some(org) = world.orgs.get(o as usize).filter(|x| x.alive) else { return Vec::new() };
    let def = world.data.def(org.def);
    if def.max_attachments.is_some() || !def.supply_used {
        return Vec::new();
    }
    let kingdom = org.kingdom;
    let mut out = Vec::new();
    for hq in world.orgs.iter().filter(|h| h.alive && h.kingdom == kingdom && world.data.def(h.def).max_attachments.is_some()) {
        if let Some(leader) = crate::ids::target_actor(world, Unit::Org(hq.id)) {
            out.push(leader);
        }
    }
    out
}

/// TellHQAttachCycle 0x57B0A0: with a current HQ, the one before it in the
/// kingdom's list, wrapping to the last; without one, the nearest. The cycle
/// ends on the current HQ ("nothing to cycle to"). The attach gates of
/// 0x574A10 are `production::attach`'s.
fn hq_cycle(world: &mut World, kingdom: KingdomId, o: OrgId) -> bool {
    let list = hq_cycle_list(world, o);
    let current = world.orgs[o as usize].hq;
    let org_position = world.combat.org(o).map_or(world.orgs[o as usize].anchor, |c| c.position);
    let mut best: Option<ActorId> = None;
    let mut best_distance = 0.0f32;
    let mut index = 0;
    while index < list.len() {
        let h = list[index];
        match current {
            None => {
                let d = world.actors[h as usize].pos.truncate() - org_position;
                let d2 = d.y * d.y + d.x * d.x;
                if best.is_none() || d2 < best_distance {
                    best = Some(h);
                    best_distance = d2;
                }
            }
            Some(c) if h == c => break,
            Some(_) => best = Some(h),
        }
        index += 1;
    }
    if current.is_some() && best.is_none() {
        // 0x57B16F: on from the current one to the end: the last admits.
        best = list.get(index..).and_then(|rest| rest.last().copied());
    }
    let Some(next) = best else { return false };
    if Some(next) == current {
        return false;
    }
    if current.is_some() {
        // 0x57A460 before 0x578990.
        world.orgs[o as usize].hq = None;
    }
    crate::production::attach(world, kingdom, o, next).is_ok()
}

/// 0x509840 case 3: the giver's stock caps the amount (a defeated receiver,
/// Kingdom +19C == 2, is refused at line 0x1A8). The team resource is the
/// first stockpiled one (0x473520), then 0x4E2080 twice.
fn give_resources(world: &mut World, from: KingdomId, to: KingdomId, amount: f32) -> Result<(), String> {
    if world.kingdoms.get(to as usize).is_none_or(|k| k.defeated) {
        return Err("cannot give resources to a defeated kingdom".into());
    }
    let n = world.data.resources.len();
    let Some(resource) = world.data.resources.iter().position(|r| r.kind == crate::data::ResourceKind::Stockpiled) else {
        return Err("no stockpiled resource".into());
    };
    let have = world.books.kingdom(from).and_then(|k| k.stockpiled.get(resource).copied()).unwrap_or(0.0);
    let amount = if have < amount { have } else { amount };
    let mut values = vec![0.0; n];
    values[resource] = amount;
    crate::production::credit(world, from, &values, -1.0);
    crate::production::credit(world, to, &values, 1.0);
    Ok(())
}

/// TributeActor 0x509440 (a defeated receiver refuses, line 0x1E4): vt+160
/// changes the owner, of an organization's members too.
fn give_actor(world: &mut World, to: KingdomId, actor: Unit) -> Result<(), String> {
    if world.kingdoms.get(to as usize).is_none_or(|k| k.defeated) {
        return Err("cannot tribute actors to a kingdom that has been defeated".into());
    }
    match actor {
        Unit::Org(o) => {
            let Some(org) = world.orgs.get_mut(o as usize).filter(|x| x.alive) else { return Err("no such organization".into()) };
            org.kingdom = to;
            org.hq = None;
            for m in org.members.clone() {
                world.actors[m as usize].kingdom = Some(to);
            }
        }
        Unit::Actor(a) => {
            let Some(x) = world.actor_mut(a) else { return Err("no such actor".into()) };
            x.kingdom = Some(to);
        }
    }
    Ok(())
}

/// The Thinks of the order states (the Mind's queue-4 think; a higher-priority
/// state, engage 4, rout 6, regroup 3, exhausted 7, suspends them).
pub(crate) fn tick(world: &mut World) {
    group::tick(world);
    if world.orders.orgs.is_empty() {
        return;
    }
    let mut st = std::mem::take(&mut world.combat);
    st.prepare(world);
    let ids: Vec<OrgId> = world.orders.orgs.keys().copied().collect();
    for o in ids {
        let i = o as usize;
        if !world.orgs[i].alive || st.orgs[i].destroyed {
            world.orders.orgs.remove(&o);
            continue;
        }
        let Some(state) = world.orders.orgs.get(&o) else { continue };
        match state {
            MindState::Retreat { order } => {
                // A rout or arrival ended it.
                if st.orgs[i].routing.is_some() || world.orgs[i].order != *order {
                    world.orders.orgs.remove(&o);
                }
            }
            MindState::Guard | MindState::Explore => {}
        }
    }
    world.combat = st;
}
