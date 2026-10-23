//! Opponent decision rules on synthetic worlds (no original files needed).
//! The test ego carries the values of the shipped `ArmorLowIncomeEgo`
//! (Templates/template_sai*.tgi) that these rules read.
use aa_play::ai::data::{
    ActorPreference, ActorPriority, AiData, CvCurve, DefendPriority, Ego, Exploration, Filter, GoalPriority, Profile, Reactive, Recover, RecruitRequest,
    Recruiting, Unpacking, ATTACH, ATTACK_REGION, DEFEND_REGION, EXPLORE, RECOVER, RECRUIT, RESEARCH, UNPACK,
};
use aa_play::ai::{self, goals::cv_curve};
use aa_play::data::{Def, DefClass, GameData, Weapon};
use aa_play::map::GameMap;
use aa_play::world::{Command, Controller, Kingdom, Order, Organization, Unit, World};
use glam::{Vec2, Vec3};
use std::sync::Arc;

fn deep() -> Exploration {
    Exploration {
        num_goals: 1,
        base_priority: 500.0,
        min_priority: 500.0,
        percent_to_count_region_explored: 0.8,
        move_rate_mult: 20.0,
        move_rate_exp: 1.0,
        sight_mult: 5.0,
        sight_exp: 1.0,
        subtract_cv: true,
        cv_mult: 1.0,
        cv_exp: 1.0,
        unit_inertia: 20.0,
        intuition_probability: 1.0,
        intuition_distance: -1,
        intuition_count_settlements: true,
        intuition_count_other_buildings: true,
        num_random_candidates: 10,
        black_fog_priority: 200.0,
        grey_fog_priority: 200.0,
        fog_search_depth: -1,
        intuition_region_bonus: 200.0,
        random_candidate_bonus: 1.0,
        consider_distance_to_kingdom_center: true,
        kingdom_center_distance_cap: -1.0,
        kingdom_center_distance_max: 400.0,
        kingdom_center_distance_exp: 1.0,
        kingdom_center_distance_ceil: 1.0,
        kingdom_center_distance_floor: 0.7,
        consider_distance_to_actor: true,
        actor_distance_cap: 100.0,
        actor_distance_max: 100.0,
        actor_distance_exp: 1.0,
        actor_distance_ceil: 1.0,
        actor_distance_floor: 0.5,
        use_time_since_last_scouted: true,
        time_since_last_scouted_max: 300.0,
        time_since_last_scouted_exp: 1.0,
        fuzzy_factor: 200.0,
        ..Default::default()
    }
}

fn ego(ids: &str, min: f32, max: f32) -> Ego {
    let mut gp = vec![GoalPriority::default(); 23];
    gp[DEFEND_REGION] = GoalPriority { bonus: 500.0, ..Default::default() };
    gp[ATTACK_REGION] = GoalPriority {
        bonus: 750.0,
        preferences: vec![ActorPreference { property: Some("engineer".into()), value: -5000.0, count_value_once_per_company: true, multiplier: 1.0, count_multiplier_once_per_goal: true, count_multiplier_once_per_company: true, ..Default::default() }],
        ..Default::default()
    };
    gp[RECRUIT] = GoalPriority { multiplier: 0.4, ..Default::default() };
    gp[UNPACK] = GoalPriority { multiplier: 5.0, ..Default::default() };
    gp[RECOVER] = GoalPriority { multiplier: 5.0, ..Default::default() };
    gp[RESEARCH] = GoalPriority { multiplier: 1.5, fuzziness: 100.0, ..Default::default() };
    gp[ATTACH] = GoalPriority { bonus: 500.0, ..Default::default() };
    gp[EXPLORE] = GoalPriority {
        preferences: vec![ActorPreference { property: Some("recon".into()), value: 500.0, count_value_once_per_company: true, multiplier: 1.0, count_multiplier_once_per_goal: true, count_multiplier_once_per_company: true, ..Default::default() }],
        ..Default::default()
    };
    let ap = |p: &str, value: f32, repeat: f32| ActorPriority { property: Some(p.into()), count_all_property_actors: true, value, repeat_penalty: repeat, ..Default::default() };
    let req = |p: &str, base: f32, bonus: f32| RecruitRequest { property: Some(p.into()), number_to_request: 1, base_priority: base, economic_value_multiplier: 1.0, hq_type_not_available_bonus: bonus, ..Default::default() };
    Ego {
        ids: ids.into(),
        priority: 2000.0,
        filters: vec![Filter { stat: "money_rate".into(), min, max }],
        cv_influence_exponent: 1.5,
        cv_influence_max_travel_time: 45.0,
        border_influence_range: 300.0,
        think_frequency: 30.0,
        fuzziness: 200.0,
        goal_inertia: 250.0,
        immediate_think_on_build_max_settlements: 2,
        immediate_think_on_build_max_time: 600.0,
        goal_priorities: gp,
        saving_time_limit: 300.0,
        stockpilable_short_need: 50.0,
        stockpilable_enough_need: 25.0,
        stockpilable_plenty_need: 25.0,
        scaling_mult: 0.3,
        enemy_influence_mult: 0.5,
        min_influence_factor: 0.5,
        influence_exponent: 2.0,
        military_cv_mult: 5.0,
        military_value_scaling_mult: 1.0,
        offensive: CvCurve { cv_advantage_target: 1.2, cv_advantage_max: 5.0, cv_exponent_under_target: 3.0, cv_max_mult_over_target: 15.0, min_cv_to_assign: 50.0, ..Default::default() },
        defensive: CvCurve { cv_advantage_target: 1.0, cv_advantage_max: 2.5, cv_exponent_under_target: 1.5, cv_max_mult_over_target: 4.0, min_cv_to_assign: 150.0, count_static_cv: true, ..Default::default() },
        max_travel_time: 300.0,
        travel_time_exponent: 2.0,
        actor_priorities: vec![
            ap("corpshq", 1000.0, 10000.0),
            ap("oil", 250.0, 0.0),
            ap("ammo", 250.0, 0.0),
            ap("infhq", 550.0, 0.0),
            ap("mechhq", 650.0, 0.0),
            ap("tank_company", 700.0, 70.0),
            ap("infantry_company", 500.0, 200.0),
            ap("recon", 600.0, 1000.0),
        ],
        default_max_recruited_per_think: 1,
        build_queue_max_items: 1,
        defend: DefendPriority {
            economic_value_exponent: 0.5,
            economic_value_multiplier: 2.0,
            enemy_influence_exponent: 1.0,
            enemy_influence_multiplier: 0.5,
            friendly_border_influence_exponent: 1.0,
            enemy_border_influence_exponent: 1.0,
            enemy_border_influence_multiplier: 0.2,
            enemy_forces_threat_distance: 80.0,
            enemy_forces_bonus: 3.0,
            capture_risk_ratio: 1.0,
            capture_risk_bonus: 2.0,
            ..Default::default()
        },
        defend_friendly_kingdom_actor_multiplier: 0.2,
        attack_allied_influence_cv_multiplier: 0.5,
        defend_allied_influence_cv_multiplier: 0.2,
        linked_region_distance: 2,
        max_defense_depth: 3,
        attack_commit_distance: 3,
        congruency_min_multiplier: 0.6,
        congruency_power: 3.0,
        penalty_to_builder_cv: 0.01,
        exploration: [Exploration::default(), deep(), Exploration::default()],
        unpacking: Unpacking {
            adjacent_region_depth: 1,
            num_random_regions: 20,
            base_priority: 500.0,
            enemy_static_influence_mult: 3.0,
            enemy_dynamic_influence_mult: 3.0,
            max_friendly_building_bonus: 500.0,
            max_friendly_building_bonus_per_building: 200.0,
            max_distance: 30.0,
            distance_exponent: 1.0,
            travel_distance_mult: 5.0,
            travel_distance_exponent: 1.0,
            fuzzy_factor: 150.0,
            inertia: 300.0,
        },
        pack_out_of_supply_priority: 1000.0,
        recover: Recover { base_priority_hp: 500.0, exponent_hp: 3.0, max_hp: 0.4, base_priority_morale: 500.0, exponent_morale: 3.0, max_morale: 0.5, critical_morale: 0.3 },
        recruiting: Recruiting {
            max_goals_per_think: 3,
            num_military_goals: 1,
            core_cv_multiplier: 3.0,
            support_cv_multiplier: 3.0,
            economic_value_multiplier: 0.2,
            decreased_income_penalty: 20.0,
            insufficient_income_level: 10.0,
            insufficient_income_penalty: 350.0,
            candidate_fuzziness: 50.0,
            base_value: 100.0,
            building_bonus: 50.0,
            location_fuzziness: 100.0,
            replace_company_cv_mult: 0.6,
            requests: vec![req("oil", 0.0, 1100.0), req("ammo", 0.0, 1100.0), req("corpshq", 0.0, 1100.0), req("infhq", -850.0, 900.0), req("mechhq", -850.0, 1000.0)],
            ..Default::default()
        },
        reactive: Reactive {
            think_frequency: 2.0,
            unit_retreat_max_health: 0.3,
            unit_retreat_depth: 1,
            unit_retreat_cv_ratio: 0.9,
            unit_retreat_probability: 1.0,
            unit_retreat_cycles_til_reconsider: 4,
            region_retreat_depth: 2,
            region_retreat_cv_ratio: 0.25,
            region_retreat_probability: 0.75,
            region_retreat_cycles_til_reconsider: 4,
            halt_retreat_cv_ratio: 1.4,
            attack_distance: 2,
            min_attack_advantage: 1.0,
            max_attack_advantage: 1.5,
            threatened_region_influence_ratio: 0.3,
            ..Default::default()
        },
        research_base_priority: 20.0,
        tech_priorities: vec![("tech_special_forces".into(), 50.0)],
        attach_when_engaged: false,
        attach_cv_multiplier: 5.0,
        ..Default::default()
    }
}

fn ai_data() -> AiData {
    AiData {
        source_key: "test".into(),
        egos: vec![ego("low", -9_999_999.0, 40.0), ego("main", 30.0, 120.0), ego("end", 100.0, 1_000_000.0)],
        profiles: vec![Profile {
            ids: "test_sai".into(),
            difficulty_level: "sai_difficulty_level_hard".into(),
            public: true,
            command_max_delay: 0.05,
            command_max_total_delay: 5.0,
            max_ge_commands: 100,
            max_rai_commands: 40,
            ego_fuzziness: 200.0,
            egos: vec![0, 1, 2],
            nations: vec!["nation_rus".into()],
            factions: vec![],
        }],
        handicaps: vec![],
        requirements: Default::default(),
    }
}

struct Defs {
    corps: u32,
    hq_truck: u32,
    hq_regiment: u32,
    hq: u32,
    oil_truck: u32,
    rifle: u32,
    infantry: u32,
    scout: u32,
    recon: u32,
}

fn game_data() -> (GameData, Defs) {
    let mut d = GameData::default();
    let mut push = |def: Def| -> u32 {
        let id = d.defs.len() as u32;
        d.by_id.insert(def.id.to_ascii_lowercase(), id);
        d.defs.push(def);
        id
    };
    let gun = |damage: f32, range: f32| Weapon { name: "gun".into(), damage, range, recharge: 1.0, ..Default::default() };
    let rifle = push(Def { id: "rus_rifleman".into(), class: Some(DefClass::Member), health: 100.0, speed_max: 3.0, sight_range: 40.0, weapons: vec![gun(10.0, 30.0)], properties: vec!["property_infantry_unit".into()], ..Default::default() });
    let scout = push(Def { id: "rus_scout".into(), class: Some(DefClass::Member), health: 60.0, speed_max: 6.0, sight_range: 60.0, weapons: vec![gun(4.0, 30.0)], properties: vec!["recon".into()], ..Default::default() });
    let infantry = push(Def {
        id: "regiment_rus_infantry".into(),
        class: Some(DefClass::Organization),
        cost: [100.0, 0.0, 0.0],
        upkeep: [0.0, 5.0, 0.0],
        unit_limit_consumed: 1.0,
        slots: (0..6).map(|i| (rifle, [i as f32 * 2.0, 0.0])).collect(),
        properties: vec!["org_infantry".into(), "infantry_company".into()],
        supply_used: true,
        ..Default::default()
    });
    let recon = push(Def {
        id: "regiment_rus_infantry_recon".into(),
        class: Some(DefClass::Organization),
        cost: [80.0, 0.0, 0.0],
        upkeep: [0.0, 2.0, 0.0],
        unit_limit_consumed: 1.0,
        slots: (0..3).map(|i| (scout, [i as f32 * 2.0, 0.0])).collect(),
        properties: vec!["infantry_company".into()],
        ..Default::default()
    });
    let hq = push(Def {
        id: "rus_hq_infantry".into(),
        class: Some(DefClass::Building),
        health: 1500.0,
        sight_range: 50.0,
        supply_range: 60.0,
        recruits: vec![infantry, recon],
        production: [12.0, 5.0, 2.0],
        properties: vec!["building".into(), "infhq".into()],
        blocking_size: Some([12.0, 12.0]),
        ..Default::default()
    });
    let hq_truck = push(Def {
        id: "rus_truck_infantry".into(),
        class: Some(DefClass::Truck),
        health: 500.0,
        speed_max: 3.0,
        unpacks_to: Some(hq),
        production: [12.0, 5.0, 2.0],
        properties: vec!["vehicle".into(), "economic_truck".into(), "infhq".into()],
        ..Default::default()
    });
    let hq_regiment = push(Def {
        id: "regiment_rus_hq_infantry".into(),
        class: Some(DefClass::Organization),
        cost: [250.0, 0.0, 0.0],
        slots: vec![(hq_truck, [0.0, 0.0])],
        max_attachments: Some(6),
        properties: vec!["hq".into()],
        ..Default::default()
    });
    let oil_truck = push(Def {
        id: "rus_truck_oil".into(),
        class: Some(DefClass::Truck),
        health: 400.0,
        speed_max: 3.0,
        cost: [150.0, 0.0, 0.0],
        properties: vec!["vehicle".into(), "economic_truck".into(), "oil".into()],
        ..Default::default()
    });
    let corps = push(Def {
        id: "rus_corps".into(),
        class: Some(DefClass::Building),
        health: 3000.0,
        sight_range: 60.0,
        supply_range: 80.0,
        recruits: vec![hq_regiment, oil_truck],
        production: [25.0, 10.0, 10.0],
        unit_limit_provided: 10.0,
        properties: vec!["building".into(), "corpshq".into()],
        blocking_size: Some([16.0, 16.0]),
        weapons: vec![gun(15.0, 40.0)],
        ..Default::default()
    });
    d.defs[oil_truck as usize].unpacks_to = Some(hq);
    (d, Defs { corps, hq_truck, hq_regiment, hq, oil_truck, rifle, infantry, scout, recon })
}

fn flat_map() -> GameMap {
    GameMap { id: "flat".into(), width: 200, height: 200, cell_size: 4.0, flags: vec![0; 200 * 200], ..Default::default() }
}

fn kingdom(id: u32, team: u32, controller: Controller, money: f32) -> Kingdom {
    Kingdom {
        id,
        name: format!("k{id}"),
        nation: "rus".into(),
        team,
        color: [255, 255, 255],
        controller,
        stockpile: [money, 200.0, 200.0],
        income: [25.0, 10.0, 10.0],
        unit_limit: 10.0,
        units_used: 0.0,
        defeated: false,
        independent: false,
    }
}

fn world() -> (World, Defs) {
    let (data, defs) = game_data();
    let mut w = World::empty(Arc::new(data), Arc::new(flat_map()), 7);
    w.kingdoms.push(kingdom(0, 0, Controller::Human, 500.0));
    w.kingdoms.push(kingdom(1, 1, Controller::Computer(1), 500.0));
    ai::attach(&mut w, Arc::new(ai_data()));
    w.ai.logging = true;
    (w, defs)
}

fn org(w: &mut World, def: u32, member: u32, n: usize, kingdom: u32, at: Vec2) -> u32 {
    let id = w.orgs.len() as u32;
    let mut members = Vec::new();
    for i in 0..n {
        let a = w.spawn_actor(member, Some(kingdom), Vec3::new(at.x + i as f32 * 2.0, at.y, 0.0), 0.0);
        w.actors[a as usize].org = Some(id);
        members.push(a);
    }
    w.orgs.push(Organization { id, def, kingdom, members, hq: None, order: Order::Idle, anchor: at, heading: 0.0, morale: 1.0, experience: 0.0, alive: true });
    id
}

fn building(w: &mut World, def: u32, kingdom: Option<u32>, at: Vec2) -> u32 {
    w.spawn_actor(def, kingdom, Vec3::new(at.x, at.y, 0.0), 0.0)
}

/// Run the opponent alone (no other systems) from `from` to `to` seconds.
fn run(w: &mut World, to: f32) -> Vec<Command> {
    let mut all = Vec::new();
    while w.time < to {
        all.extend(ai::think(w));
        w.tick += 1;
        w.time = w.tick as f32 * aa_play::DT;
    }
    all
}

#[test]
fn cv_curve_follows_7acf90() {
    let e = ego("t", 0.0, 0.0);
    // Below target: (cv/target)^exponent.
    let p = cv_curve(&e, 1000.0, 50.0, 100.0, 500.0, true, 0.0);
    assert!((p - 1000.0 * 0.5f32.powi(3)).abs() < 1e-2, "{p}");
    // At target: the base.
    assert_eq!(cv_curve(&e, 1000.0, 100.0, 100.0, 500.0, true, 0.0), 1000.0);
    // Above: linear to cv_max_mult_over_target at the maximum, clamped.
    let p = cv_curve(&e, 1000.0, 300.0, 100.0, 500.0, true, 0.0);
    assert!((p - 1000.0 * (14.0 * 0.5 + 1.0)).abs() < 1e-1, "{p}");
    assert!((cv_curve(&e, 1000.0, 900.0, 100.0, 500.0, true, 0.0) - 15000.0).abs() < 1e-1);
    // Allied influence fills up to the maximum.
    let p = cv_curve(&e, 1000.0, 50.0, 100.0, 500.0, true, 50.0);
    assert_eq!(p, 1000.0);
    // Defensive curve uses its own exponent.
    let p = cv_curve(&e, 800.0, 75.0, 150.0, 375.0, false, 0.0);
    assert!((p - 800.0 * 0.5f32.powf(1.5)).abs() < 1e-2, "{p}");
}

#[test]
fn first_goal_engine_think_recruits_an_hq_and_respects_the_queue() {
    let (mut w, defs) = world();
    building(&mut w, defs.corps, Some(0), Vec2::new(100.0, 100.0));
    let corps = building(&mut w, defs.corps, Some(1), Vec2::new(650.0, 650.0));
    let cmds = run(&mut w, 1.0);
    let recruits: Vec<_> = cmds.iter().filter_map(|c| if let Command::Recruit { kingdom: 1, building, def } = c { Some((*building, *def)) } else { None }).collect();
    // build_queue_max_items = 1 and one corps: exactly one recruit, the HQ
    // regiment (inf HQ request -850 + 900 bonus + 550 ActorPriority + value)
    // ahead of the oil depot (250 + value).
    assert_eq!(recruits, vec![(corps, defs.hq_regiment)], "{:#?}", w.ai.log);
    assert!(w.ai.log.iter().any(|l| l.text.starts_with("ego low")), "{:#?}", w.ai.log);
}

#[test]
fn goal_engine_thinks_on_its_schedule() {
    let (mut w, defs) = world();
    w.kingdoms.push(kingdom(2, 1, Controller::Computer(1), 500.0));
    building(&mut w, defs.corps, Some(0), Vec2::new(100.0, 100.0));
    building(&mut w, defs.corps, Some(1), Vec2::new(650.0, 650.0));
    building(&mut w, defs.corps, Some(2), Vec2::new(650.0, 100.0));
    run(&mut w, 0.1);
    assert_eq!(w.ai.stats.ge_thinks, 1, "only one goal-engine think per GoalEngineThinkFrequencyMinOffset");
    run(&mut w, 2.1);
    assert_eq!(w.ai.stats.ge_thinks, 2);
    // Player 0 next at 30 s, player 1 at 2 + 30 - 1/3*30 = 22 s.
    let p0 = w.ai.players[0].ge_due;
    let p1 = w.ai.players[1].ge_due;
    assert!((p0 - 30.0).abs() < 0.1, "{p0}");
    assert!((p1 - 22.0).abs() < 0.2, "{p1}");
    run(&mut w, 61.0);
    assert_eq!(w.ai.stats.ge_thinks, 6, "0, 2, 22, 30, 52, 60");
    assert!(w.ai.stats.rai_thinks > 25);
}

#[test]
fn idle_truck_unpacks_next_to_the_base() {
    let (mut w, defs) = world();
    building(&mut w, defs.corps, Some(0), Vec2::new(100.0, 100.0));
    let corps_pos = Vec2::new(650.0, 650.0);
    building(&mut w, defs.corps, Some(1), corps_pos);
    let o = org(&mut w, defs.hq_regiment, defs.hq_truck, 1, 1, corps_pos + Vec2::new(30.0, 0.0));
    let truck = w.orgs[o as usize].members[0];
    let cmds = run(&mut w, 1.0);
    let deploy = cmds.iter().find_map(|c| if let Command::Deploy { truck: t, at, .. } = c { Some((*t, *at)) } else { None });
    let (t, at) = deploy.unwrap_or_else(|| panic!("no deploy: {:#?}", w.ai.log));
    assert_eq!(t, truck);
    // The friendly-building bonus keeps the site in the base's region or next to it.
    assert!(at.distance(corps_pos) < 120.0, "{at}");
    assert!(at.distance(corps_pos) >= 8.0 + 6.0, "site overlaps the corps: {at}");
}

#[test]
fn damaged_regiment_recovers_in_supply() {
    let (mut w, defs) = world();
    building(&mut w, defs.corps, Some(0), Vec2::new(100.0, 100.0));
    let corps_pos = Vec2::new(650.0, 650.0);
    building(&mut w, defs.corps, Some(1), corps_pos);
    let o = org(&mut w, defs.infantry, defs.rifle, 6, 1, Vec2::new(400.0, 400.0));
    for &m in &w.orgs[o as usize].members.clone() {
        w.actors[m as usize].health = 25.0;
    }
    let cmds = run(&mut w, 1.0);
    let mv = cmds.iter().find_map(|c| match c {
        Command::Move { units, to, attack_move: false, .. } if units == &vec![Unit::Org(o)] => Some(*to),
        _ => None,
    });
    let to = mv.unwrap_or_else(|| panic!("no recover move: {:#?}", w.ai.log));
    assert!(to.distance(corps_pos) <= 8.0 + 80.0, "{to}");
}

fn attackers_of(cmds: &[Command], target: Vec2, actor: u32) -> Vec<u32> {
    let mut v: Vec<u32> = cmds
        .iter()
        .filter_map(|c| match c {
            Command::Move { units, to, attack_move: true, .. } if to.distance(target) < 60.0 => match units[0] {
                Unit::Org(o) => Some(o),
                _ => None,
            },
            Command::Attack { units, target: t, .. } if *t == actor => match units[0] {
                Unit::Org(o) => Some(o),
                _ => None,
            },
            _ => None,
        })
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

#[test]
fn superior_force_attacks_a_known_nearby_target() {
    let (mut w, defs) = world();
    let enemy_hq = building(&mut w, defs.corps, Some(0), Vec2::new(200.0, 200.0));
    let corps_pos = Vec2::new(500.0, 500.0);
    building(&mut w, defs.corps, Some(1), corps_pos);
    for i in 0..10 {
        org(&mut w, defs.infantry, defs.rifle, 6, 1, corps_pos + Vec2::new(-40.0 + i as f32 * 8.0, 30.0));
    }
    let cmds = run(&mut w, 1.0);
    // ATTACK_REGION (bonus 750, cv_max_mult_over_target 15) outbids an
    // unthreatened DEFEND_REGION (bonus 500, max mult 4).
    assert!(!attackers_of(&cmds, Vec2::new(200.0, 200.0), enemy_hq).is_empty(), "no attack: {:#?}", w.ai.log);
    assert!(w.ai.players[0].memory.attacks.iter().any(|a| a.2 > 0));
}

#[test]
fn far_target_leaves_defenders_at_home() {
    let (mut w, defs) = world();
    let enemy_hq = building(&mut w, defs.corps, Some(0), Vec2::new(60.0, 60.0));
    let corps_pos = Vec2::new(740.0, 740.0);
    building(&mut w, defs.corps, Some(1), corps_pos);
    let mut orgs = Vec::new();
    for i in 0..10 {
        orgs.push(org(&mut w, defs.infantry, defs.rifle, 6, 1, corps_pos + Vec2::new(-40.0 + i as f32 * 8.0, -30.0)));
    }
    let cmds = run(&mut w, 1.0);
    let attackers = attackers_of(&cmds, Vec2::new(60.0, 60.0), enemy_hq);
    // The travel weight (max_travel_time 300, exponent 2) sits at its 0.2
    // floor, so the defend goal keeps CV at home.
    let defenders = w.ai.players[0].memory.assigned.values().filter(|a| matches!(a.key, ai::goals::Key::Defend(_))).count();
    assert!(defenders > 0, "{:#?}", w.ai.log);
    assert!(attackers.len() + defenders <= orgs.len());
}

#[test]
fn threatened_base_is_defended() {
    let (mut w, defs) = world();
    building(&mut w, defs.corps, Some(0), Vec2::new(60.0, 60.0));
    let corps_pos = Vec2::new(600.0, 600.0);
    building(&mut w, defs.corps, Some(1), corps_pos);
    for i in 0..4 {
        org(&mut w, defs.infantry, defs.rifle, 6, 1, corps_pos + Vec2::new(-150.0, -150.0 + i as f32 * 10.0));
    }
    // An enemy regiment next to the corps.
    org(&mut w, defs.infantry, defs.rifle, 6, 0, corps_pos + Vec2::new(30.0, 0.0));
    let cmds = run(&mut w, 1.0);
    // The threat is met: a defend goal or an attack on the intruder's
    // region next to the corps (both draw the regiments home).
    let home = w.ai.players[0].memory.assigned.values().filter(|a| a.combat && a.target.distance(corps_pos) < 80.0).count();
    assert!(home > 0, "{:#?}", w.ai.log);
    assert!(cmds.iter().any(|c| matches!(c, Command::Move { to, attack_move: true, .. } if to.distance(corps_pos) < 80.0)), "{:#?}", w.ai.log);
}

#[test]
fn engaged_weak_regiment_retreats() {
    let (mut w, defs) = world();
    building(&mut w, defs.corps, Some(0), Vec2::new(100.0, 100.0));
    let corps_pos = Vec2::new(650.0, 650.0);
    building(&mut w, defs.corps, Some(1), corps_pos);
    let mine = org(&mut w, defs.infantry, defs.rifle, 6, 1, Vec2::new(400.0, 400.0));
    let theirs = org(&mut w, defs.infantry, defs.rifle, 6, 0, Vec2::new(380.0, 400.0));
    for &m in &w.orgs[mine as usize].members.clone() {
        w.actors[m as usize].health = 20.0;
    }
    // Hostile members fire at mine.
    let target = w.orgs[mine as usize].members[0];
    for &m in &w.orgs[theirs as usize].members.clone() {
        w.actors[m as usize].combat.target = Some(target);
    }
    let first = w.orgs[theirs as usize].members[0];
    for &m in &w.orgs[mine as usize].members.clone() {
        w.actors[m as usize].combat.target = Some(first);
    }
    // No combat state prepared: engagement comes from the member targets;
    // the immediate fight is the two regiments (ratio 0.2 < 0.9).
    let cmds = run(&mut w, 2.5);
    let retreated = cmds.iter().any(|c| matches!(c, Command::Move { units, attack_move: false, .. } if units == &vec![Unit::Org(mine)]));
    assert!(retreated, "{:#?}", w.ai.log);
}

#[test]
fn ego_follows_money_rate_filters() {
    let (mut w, defs) = world();
    building(&mut w, defs.corps, Some(0), Vec2::new(100.0, 100.0));
    building(&mut w, defs.corps, Some(1), Vec2::new(650.0, 650.0));
    w.kingdoms[1].income = [150.0, 10.0, 10.0];
    run(&mut w, 0.5);
    assert_eq!(w.ai.players[0].ego, 2, "money_rate 150 only passes the end-game ego");
    w.kingdoms[1].income = [110.0, 10.0, 10.0];
    run(&mut w, 31.0);
    assert_eq!(w.ai.players[0].ego, 2, "the current ego is kept while its filter holds (100..)");
    w.kingdoms[1].income = [50.0, 10.0, 10.0];
    run(&mut w, 61.0);
    assert_eq!(w.ai.players[0].ego, 1);
}

#[test]
fn recon_regiment_explores_toward_unknown_buildings() {
    let (mut w, defs) = world();
    // FogOfWar NONE in the rules would make everything known; with the
    // default (EXPLORED) the enemy corps placed at the start is known, so
    // add a building after the start that the AI must find.
    building(&mut w, defs.corps, Some(0), Vec2::new(100.0, 100.0));
    let corps_pos = Vec2::new(650.0, 650.0);
    building(&mut w, defs.corps, Some(1), corps_pos);
    for i in 0..3 {
        org(&mut w, defs.infantry, defs.rifle, 6, 1, corps_pos + Vec2::new(-20.0 + i as f32 * 10.0, 30.0));
    }
    let scout = org(&mut w, defs.recon, defs.scout, 3, 1, corps_pos + Vec2::new(0.0, -30.0));
    run(&mut w, 0.2);
    building(&mut w, defs.hq, Some(0), Vec2::new(150.0, 700.0));
    let cmds = run(&mut w, 31.0);
    let explore = cmds.iter().any(|c| matches!(c, Command::Move { units, attack_move: false, .. } if units == &vec![Unit::Org(scout)]));
    assert!(explore, "{:#?}", w.ai.log);
    assert!(w.ai.log.iter().any(|l| l.text.contains("explore with regiment_rus_infantry_recon")), "{:#?}", w.ai.log);
}

#[test]
fn per_tick_cost_is_small() {
    let (mut w, defs) = world();
    building(&mut w, defs.corps, Some(0), Vec2::new(100.0, 100.0));
    for k in 1..4 {
        if k > 1 {
            w.kingdoms.push(kingdom(k, 1, Controller::Computer(2), 500.0));
        }
        let base = Vec2::new(150.0 * k as f32 + 200.0, 600.0);
        building(&mut w, defs.corps, Some(k), base);
        for i in 0..12 {
            org(&mut w, defs.infantry, defs.rifle, 6, k, base + Vec2::new(i as f32 * 6.0, 30.0));
        }
    }
    for i in 0..12 {
        org(&mut w, defs.infantry, defs.rifle, 6, 0, Vec2::new(100.0 + i as f32 * 6.0, 150.0));
    }
    run(&mut w, 120.0);
    let avg = w.ai.stats.total_ns as f64 / w.ai.stats.ticks as f64 / 1e6;
    eprintln!("3 computer players, 120 s: avg {avg:.3} ms/tick, max {:.3} ms, {} GE / {} RAI thinks", w.ai.stats.max_ns as f64 / 1e6, w.ai.stats.ge_thinks, w.ai.stats.rai_thinks);
    assert!(avg < 1.0, "average {avg} ms per tick");
}

#[test]
fn idle_corps_researches_its_technologies() {
    let (mut w, defs) = world();
    {
        let data = Arc::get_mut(&mut w.data).expect("unique data");
        let corps = &mut data.defs[defs.corps as usize];
        corps.recruits.clear();
        corps.research = vec![
            aa_play::data::ResearchItem { property: "tech_improved_machineguns".into(), kind: "GLOBAL".into(), research_time: 60.0, cost: vec![] },
            aa_play::data::ResearchItem { property: "tech_special_forces".into(), kind: "GLOBAL".into(), research_time: 60.0, cost: vec![] },
        ];
    }
    building(&mut w, defs.corps, Some(0), Vec2::new(100.0, 100.0));
    let corps = building(&mut w, defs.corps, Some(1), Vec2::new(650.0, 650.0));
    let cmds = run(&mut w, 1.0);
    let research: Vec<_> = cmds.iter().filter_map(|c| if let Command::Research { building, property, .. } = c { Some((*building, property.clone())) } else { None }).collect();
    // One queue slot (build_queue_max_items 1): the technology the ego's
    // TechPriority list ranks (50) before the base priority (20).
    assert_eq!(research.len(), 1, "{:#?}", w.ai.log);
    assert_eq!(research[0].0, corps);
}

#[test]
fn detached_regiment_attaches_to_a_free_hq() {
    let (mut w, defs) = world();
    building(&mut w, defs.corps, Some(0), Vec2::new(100.0, 100.0));
    let base = Vec2::new(650.0, 650.0);
    building(&mut w, defs.corps, Some(1), base);
    // An unpacked HQ: its building is the member of an HQ organization.
    let hq = building(&mut w, defs.hq, Some(1), base + Vec2::new(40.0, 0.0));
    let hq_org = w.orgs.len() as u32;
    w.actors[hq as usize].org = Some(hq_org);
    w.orgs.push(Organization { id: hq_org, def: defs.hq_regiment, kingdom: 1, members: vec![hq], hq: None, order: Order::Idle, anchor: base, heading: 0.0, morale: 1.0, experience: 0.0, alive: true });
    let o = org(&mut w, defs.infantry, defs.rifle, 6, 1, base + Vec2::new(0.0, 40.0));
    let cmds = run(&mut w, 1.0);
    assert!(cmds.iter().any(|c| matches!(c, Command::Attach { org, hq: h, .. } if *org == o && *h == hq)), "{:#?}", w.ai.log);
}

#[test]
fn city_of_an_enemy_independent_is_taken_by_presence_and_neutral_blocks_are_ignored() {
    let (mut w, defs) = world();
    let (city_def, block_def) = {
        let data = Arc::get_mut(&mut w.data).expect("unique data");
        let mut push = |def: Def| -> u32 {
            let id = data.defs.len() as u32;
            data.by_id.insert(def.id.to_ascii_lowercase(), id);
            data.defs.push(def);
            id
        };
        let city = push(Def {
            id: "western_europe_city_center01_small".into(),
            class: Some(DefClass::Building),
            health: 500.0,
            production: [25.0, 10.0, 10.0],
            properties: vec!["property_city".into(), "economic_building".into()],
            city: true,
            city_zone_radius: 30.0,
            captureable: true,
            blocking_size: Some([10.0, 10.0]),
            ..Default::default()
        });
        let block = push(Def { id: "weurope_cityblock10x10_01".into(), class: Some(DefClass::Building), health: 500.0, properties: vec!["property_city".into()], blocking_size: Some([10.0, 10.0]), ..Default::default() });
        (city, block)
    };
    building(&mut w, defs.corps, Some(0), Vec2::new(60.0, 60.0));
    let base = Vec2::new(650.0, 650.0);
    building(&mut w, defs.corps, Some(1), base);
    // independent_5 (enemy of both teams) owns the city; independent_2
    // (neutral) owns a city block next to the computer.
    let mut enemy = kingdom(2, 0x8000_0005, Controller::Independent, 0.0);
    enemy.independent = true;
    let mut neutral = kingdom(3, 0x8000_0002, Controller::Independent, 0.0);
    neutral.independent = true;
    w.kingdoms.push(enemy);
    w.kingdoms.push(neutral);
    // Relation rows (0 self, 1 friend, 2 neutral, 3 enemy).
    w.books.relations = vec![vec![0, 3, 3, 2], vec![3, 0, 3, 2], vec![3, 3, 0, 2], vec![2, 2, 2, 0]];
    let city_pos = Vec2::new(500.0, 650.0);
    let city = building(&mut w, city_def, Some(2), city_pos);
    let block_pos = Vec2::new(650.0, 520.0);
    building(&mut w, block_def, Some(3), block_pos);
    for i in 0..2 {
        org(&mut w, defs.infantry, defs.rifle, 6, 1, base + Vec2::new(-20.0 + i as f32 * 10.0, 30.0));
    }
    let cmds = run(&mut w, 1.0);
    let moves: Vec<Vec2> = cmds.iter().filter_map(|c| if let Command::Move { to, attack_move: true, .. } = c { Some(*to) } else { None }).collect();
    assert!(moves.iter().any(|to| to.distance(city_pos) <= 30.0), "into the city zone: {moves:?} {:#?}", w.ai.log);
    assert!(moves.iter().all(|to| to.distance(block_pos) > 40.0), "neutral block targeted: {moves:?}");
    assert!(!cmds.iter().any(|c| matches!(c, Command::Attack { target, .. } if *target == city)), "a city is taken by presence, not attacked");
}

#[test]
fn unused_defs_compile() {
    let (_, defs) = game_data();
    let _ = (defs.oil_truck, defs.hq, defs.scout);
}

#[test]
fn truck_unpacks_only_where_it_can_drive() {
    // A water channel (one tile wide, the whole map high) east of the
    // corps: the truck stands beyond it, so the corps side (the best
    // region by the friendly-building bonus) cannot be reached by land.
    let (data, defs) = game_data();
    let mut map = flat_map();
    let channel_x = 168u32; // 672 m
    for y in 0..map.height {
        map.flags[(y * map.width + channel_x) as usize] |= aa_play::map::NOT_LAND_PASSABLE;
    }
    let mut w = World::empty(Arc::new(data), Arc::new(map), 7);
    w.kingdoms.push(kingdom(0, 0, Controller::Human, 500.0));
    w.kingdoms.push(kingdom(1, 1, Controller::Computer(1), 500.0));
    ai::attach(&mut w, Arc::new(ai_data()));
    w.ai.logging = true;
    building(&mut w, defs.corps, Some(0), Vec2::new(100.0, 100.0));
    let corps_pos = Vec2::new(650.0, 650.0);
    building(&mut w, defs.corps, Some(1), corps_pos);
    let o = org(&mut w, defs.hq_regiment, defs.hq_truck, 1, 1, Vec2::new(690.0, 650.0));
    let truck = w.orgs[o as usize].members[0];
    let cmds = run(&mut w, 1.0);
    let (t, at) = cmds.iter().find_map(|c| if let Command::Deploy { truck: t, at, .. } = c { Some((*t, *at)) } else { None }).unwrap_or_else(|| panic!("no deploy: {:#?}", w.ai.log));
    assert_eq!(t, truck);
    assert!(at.x > channel_x as f32 * 4.0 + 4.0, "site across the channel: {at}");
}

#[test]
fn stuck_regiment_is_not_reordered_every_reactive_think() {
    // The opponent runs alone: nothing moves, so every Move it gives leaves
    // the regiment where it stood (an unreachable target). The reactive AI
    // repeats a goal's order at most once per goal-engine period.
    let (mut w, defs) = world();
    let enemy_hq = building(&mut w, defs.corps, Some(0), Vec2::new(200.0, 200.0));
    let corps_pos = Vec2::new(500.0, 500.0);
    building(&mut w, defs.corps, Some(1), corps_pos);
    let mut orgs = Vec::new();
    for i in 0..10 {
        orgs.push(org(&mut w, defs.infantry, defs.rifle, 6, 1, corps_pos + Vec2::new(-40.0 + i as f32 * 8.0, 30.0)));
    }
    let cmds = run(&mut w, 61.0);
    assert!(!attackers_of(&cmds, Vec2::new(200.0, 200.0), enemy_hq).is_empty(), "no attack: {:#?}", w.ai.log);
    for o in orgs {
        let moves = cmds.iter().filter(|c| matches!(c, Command::Move { units, .. } if units == &vec![Unit::Org(o)])).count();
        // Goal-engine thinks at 0, 30 and 60 s, one reactive repeat per period.
        assert!(moves <= 6, "regiment {o} got {moves} Move commands in 61 s");
    }
}

#[test]
fn unpacked_building_is_not_a_construction() {
    // 7B2350 (immediate rethink, new-structure defend flare) runs when a
    // construction finishes; a truck's unpacked HQ is not one.
    let (mut w, defs) = world();
    building(&mut w, defs.corps, Some(0), Vec2::new(100.0, 100.0));
    let corps_pos = Vec2::new(650.0, 650.0);
    building(&mut w, defs.corps, Some(1), corps_pos);
    run(&mut w, 3.0);
    let due = w.ai.players[0].ge_due;
    building(&mut w, defs.hq, Some(1), corps_pos + Vec2::new(40.0, 0.0));
    run(&mut w, 6.0);
    assert_eq!(w.ai.players[0].ge_due, due, "an unpacked building moved the goal-engine think");
    assert!(w.ai.players[0].flares.is_empty(), "{:?}", w.ai.players[0].flares);
}
