//! Native GTIDs beside the aa-play indices (`aa_play::ids`): next-fit Add,
//! Remove bumping the generation, the organization before its members, the
//! end-of-tick sweep and the lookups the parity harness uses.
use aa_play::data::DefClass;
use aa_play::ids::{Gtid, Thing};
use aa_play::{Def, GameData, GameMap, Unit, World};
use glam::Vec3;
use std::sync::Arc;

fn data() -> GameData {
    let soldier = Def { id: "soldier".into(), class: Some(DefClass::Member), health: 100.0, actor_data: true, ..Default::default() };
    let regiment = Def { id: "regiment".into(), class: Some(DefClass::Organization), actor_data: true, slots: vec![(0, [0.0, 0.0]), (0, [2.0, 0.0]), (0, [4.0, 0.0])], leader_slot: Some(1), ..Default::default() };
    GameData { defs: vec![soldier, regiment], ..Default::default() }
}

fn world() -> World {
    World::empty(Arc::new(data()), Arc::new(GameMap::default()), 1)
}

#[test]
fn creations_take_next_fit_ids_and_removals_bump_the_generation() {
    let mut w = world();
    let a = w.spawn_actor(0, Some(0), Vec3::ZERO, 0.0);
    let b = w.spawn_actor(0, Some(0), Vec3::ZERO, 0.0);
    assert_eq!(w.ids.actor(a), Some(Gtid::from_raw(1)));
    assert_eq!(w.ids.actor(b), Some(Gtid::from_raw(2)));
    assert_eq!(w.ids.thing(1), Some(Thing::Actor(a)));
    // aa-play removes the actor: its id goes back at the end of the tick.
    w.actors[a as usize].alive = false;
    w.actors[a as usize].removed = true;
    w.step(vec![]);
    assert_eq!(w.ids.actor(a), None);
    assert_eq!(w.ids.thing(1), None, "stale generation");
    assert_eq!(w.ids.thing_at_slot(1), None);
    // Next-fit: slot 1 is free again but the cursor is at 3.
    let c = w.spawn_actor(0, Some(0), Vec3::ZERO, 0.0);
    assert_eq!(w.ids.actor(c), Some(Gtid::from_raw(3)));
    assert_eq!(w.ids.thing_at_slot(2), Some(Thing::Actor(b)));
    assert_eq!(w.ids.gtid_at_slot(1), None);
    assert_eq!(w.ids.live(), 2);
}

#[test]
fn an_organization_takes_its_id_before_its_members_leader_first() {
    let mut w = world();
    let o = aa_play::production::spawn_organization(&mut w, 1, 0, glam::Vec2::ZERO, 0.0, None);
    assert_eq!(w.ids.org(o), Some(Gtid::from_raw(1)));
    let members = w.orgs[o as usize].members.clone();
    assert_eq!(w.actors[members[0] as usize].slot, Some(1), "the leader first");
    let ids: Vec<u32> = members.iter().map(|&m| w.ids.actor(m).unwrap().raw()).collect();
    assert_eq!(ids, [2, 3, 4]);
    assert_eq!(aa_play::ids::unit(w.ids.thing(1).unwrap()), Some(Unit::Org(o)));
    // The leader is what an order on the regiment aims at.
    assert_eq!(aa_play::ids::target_actor(&w, Unit::Org(o)), Some(members[0]));
    w.actors[members[0] as usize].alive = false;
    assert_eq!(aa_play::ids::target_actor(&w, Unit::Org(o)), Some(members[1]));
    // A dead organization gives its id back.
    w.orgs[o as usize].alive = false;
    w.step(vec![]);
    assert_eq!(w.ids.org(o), None);
    assert_eq!(w.ids.thing(1), None);
}

#[test]
fn saved_map_ids_are_kept_and_the_cursor_starts_at_idmgr() {
    let mut map = GameMap::default();
    map.id_manager = Some(aa_play::map::IdManager { next_slot: 40, highest_slot: 30, generations: vec![2; 30] });
    let placed = |id: &str, raw: u32| aa_play::map::PlacedThing {
        thing_id: id.into(),
        position: [0.0; 3],
        yaw: 0.0,
        owner_slot: None,
        facing: [1.0, 0.0, 0.0],
        scale: 1.0,
        variant: -1,
        serialized_id: raw,
        properties: Vec::new(),
    };
    map.placed = vec![placed("soldier", 2 << 16 | 7), placed("tree", 2 << 16 | 9), placed("tree", 5 << 16 | 11)];
    let w = World::empty(Arc::new(data()), Arc::new(map), 1);
    assert_eq!(w.ids.placed(0), Some(Gtid::from_raw(2 << 16 | 7)));
    assert_eq!(w.ids.thing_at_slot(9), Some(Thing::Placed(1)));
    // The IDMGR generation of slot 11 is 2: a saved id of generation 5 is
    // refused, as 4FAE80's guard refuses it.
    assert_eq!(w.ids.placed(2), None);
    assert_eq!(w.ids.notes.len(), 1, "{:?}", w.ids.notes);
    assert_eq!(w.ids.next_slot(), 40);
}
