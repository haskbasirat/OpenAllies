//! Combat and morale checks on synthetic data with known stats, plus an
//! ignored real-data run (AA_GAME_ROOT, e.g. `E:\Games\Axis and Alies`):
//! `cargo test -p aa-play --test combat_morale -- --ignored --nocapture`.
//! The ticks call only `combat::tick` and `morale::tick` (no movement), so
//! units stay where they were placed.
use aa_play::combat::{self, damage_amount, modified_defense, modified_power};
use aa_play::data::{ActorStateDef, DeathClip, Def, DefClass, GameData, Modifier, PropertyDef, Weapon};
use aa_play::map::GameMap;
use aa_play::world::{Activity, Controller, Event, Kingdom, Order, Organization, World};
use aa_play::{DT, OrgId};
use glam::{Vec2, Vec3};
use std::collections::BTreeMap;
use std::sync::Arc;

const GUNFIRE: u32 = 0;
const EXPLOSIVE: u32 = 2;
const AP: u32 = 3;

/// A weapon with the AAGunLocal decision weights (template_actorability_aa.tgi).
fn weapon(name: &str, range: f32, recharge: f32, damage: f32, damage_type: u32) -> Weapon {
    Weapon {
        name: name.into(),
        range,
        recharge,
        damage,
        damage_type,
        attenuation_range_min: 1.0,
        attenuation_factor_max: 1.0,
        turret: -1,
        priority: 1.0,
        use_weight: 1.0,
        wait_weight: 0.0,
        move_weight: 1.0,
        use_bonus: 50.0,
        wait_bonus: 50.0,
        move_bonus: 3.0,
        ..Default::default()
    }
}

fn member(id: &str, health: f32, defense: f32, morale_value: f32, factors: [f32; 4], weapons: Vec<Weapon>) -> Def {
    Def {
        id: id.into(),
        class: Some(DefClass::Member),
        health,
        defense,
        speed_max: 2.5,
        weapons,
        morale_value,
        damage_factor: factors.to_vec(),
        rot_time: 0.0,
        sink_time: 2.0,
        properties: vec!["infantry".into()],
        ..Default::default()
    }
}

fn regiment(id: &str, slot: u32, count: usize, breakpoint: f32) -> Def {
    Def {
        id: id.into(),
        class: Some(DefClass::Organization),
        slots: (0..count).map(|i| (slot, [(i as f32 % 3.0) * 4.0 - 4.0, (i / 3) as f32 * -4.0])).collect(),
        morale_breakpoint: breakpoint,
        control_range: 18.0,
        operational_range: 34.0,
        entrenchment_time: 30.0,
        ..Default::default()
    }
}

/// The shipped SVars_AA / SVars values, so no fallback is used.
fn rules() -> BTreeMap<String, f32> {
    let mut r = BTreeMap::new();
    for (k, v) in [
        ("CombatDamageMinimum", 1.0),
        ("KillCreditTimeout", 2.0),
        ("AbilityScoreAttackerBonus", 0.5),
        ("EngageReactionTimeRange", 0.3),
        ("KillTargetIsHighestPriority", 1.0),
        ("ModifierAttributeClampAdd", 1.0),
        ("ModifierAttributeClampMultiply", 0.05),
        ("DefaultRadius", 0.5),
        ("MoraleEngagementLoss", 0.5),
        ("MoraleEngagementLossFactorMax", 3.0),
        ("MoraleDeathFactor", 0.25),
        ("MoraleRetreatPenalty", 0.0),
        ("MoraleSlayerBase", 0.5),
        ("MoraleSlayerFactorMin", 0.5),
        ("MoraleSlayerFactorMax", 3.0),
        ("MoraleBreakerBase", 3.0),
        ("MoraleBreakerFactorMin", 0.5),
        ("MoraleBreakerFactorMax", 2.0),
        ("MoraleReplenish", 2.0),
        ("MoraleSupplyBonus", 1.0),
        ("RoutingTime", 10.0),
        ("ReformTime", 10.0),
        ("RegroupTime", 10.0),
        ("AttackEfficiencyPointsPerSecondClimbing", 0.01),
        ("AttackEfficiencyPointsPerSecondFalling", 0.02),
        ("ExperienceGainMinimum", 0.0),
        ("ExperienceGainMaximum", 10.0),
        ("ExperienceKillMultiplier", 1.0),
        ("ExperienceRoutMultiplier", 1.0),
        ("ExperienceEngageMultiplier", 1.0),
        ("ExperienceDefaultValue", 1.0),
        ("CombatValueRangePower", 0.15),
        ("CombatValueAreaPower", 1.0),
        ("CombatValueOffenseWeight", 0.5),
        ("CombatValueDefenseWeight", 0.5),
        ("CombatValueDefenseWeightRanged", 0.25),
        ("CombatValueRangedMinimum", 12.0),
        ("CombatValueMovementWeight", 0.2),
        ("CombatValueMovementDefault", 1.0),
        ("CombatValueMeanValuesDiscardPercent", 0.2),
        ("RoutRandomCandidates", 20.0),
        ("RoutMaxDistanceDelta", 24.0),
        ("RoutSupplyZoneCandidates", 2.0),
        ("RoutSupplyZoneMaxAngleDelta", 50.0),
        ("RoutBaseValue", 0.0),
        ("RoutFactorEnemyDist", 10.0),
        ("RoutFactorEnemyDistCap", 2000.0),
        ("RoutFactorEnemyAngle", 0.25),
        ("RoutFactorMovedDist", 8.0),
        ("RoutFactorMovedDistCap", 2000.0),
        ("RoutFactorSupplyDist", 1.0),
        ("RoutFactorSupplyDistCap", 2000.0),
        ("RoutBonusSupplyInRange", 50.0),
        ("RoutFuzziness", 20.0),
    ] {
        r.insert(k.to_string(), v);
    }
    // AAInfantryExperienceComponent (template_component_aa.tgi).
    for (n, req, av, dv, hp, morale, rec) in [
        (0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        (1, 150.0, 4.0, 1.0, 0.05, 0.0, 0.05),
        (2, 350.0, 8.0, 2.0, 0.1, 0.5, 0.10),
        (3, 600.0, 16.0, 3.0, 0.2, 1.0, 0.15),
    ] {
        for (f, v) in [("required", req), ("AV", av), ("DV", dv), ("HEALTH", hp), ("MORALE", morale), ("MORALE_RECOVERY_RATE", rec)] {
            r.insert(format!("regiment_rifle.experience{n}.{f}"), v);
        }
    }
    r
}

/// Defs: 0 rifleman, 1 tank, 2 AT soldier, 3 pistol, 4 rifle regiment (6),
/// 5 tank regiment (4), 6 AT regiment (6), 7 rifle regiment of 9, 8 rifle
/// regiment of 2, 9 grenadier.
fn data() -> GameData {
    // Infantry resists armor_piercing 0.2; a tank resists gunfire 0.6 (vehicle 0.4 + tank 0.2).
    let infantry = [1.0, 1.0, 1.0, 0.8];
    let tank = [0.4, 1.0, 0.8, 1.0];
    // AATurretAttackLocal: priority 20, bonuses use 1 / wait 4 / move 0.
    let gun = Weapon { priority: 20.0, use_bonus: 1.0, wait_bonus: 4.0, move_bonus: 0.0, ..weapon("tank gun", 18.0, 6.0, 70.0, AP) };
    let mut tank_def = member("tank", 440.0, 7.0, 20.0, tank, vec![gun]);
    tank_def.properties = vec!["vehicle".into()];
    tank_def.rot_time = 10.0;
    tank_def.sink_time = 5.0;
    // AAGrenadeLocal: bonuses use 75 / wait 1 / move 75.
    let mut grenade = Weapon { use_bonus: 75.0, wait_bonus: 1.0, move_bonus: 75.0, ..weapon("grenade", 9.0, 15.0, 20.0, EXPLOSIVE) };
    grenade.area = 2.0;
    grenade.attenuation_factor_max = 0.5;
    let defs = vec![
        member("rifleman", 200.0, 1.0, 5.0, infantry, vec![weapon("rifle", 10.0, 3.0, 10.0, GUNFIRE)]),
        tank_def,
        member("at_soldier", 200.0, 1.0, 5.0, infantry, vec![weapon("at rifle", 12.0, 4.0, 50.0, AP)]),
        member("pistol", 200.0, 1.0, 5.0, infantry, vec![weapon("pistol", 6.0, 2.0, 5.0, GUNFIRE)]),
        regiment("regiment_rifle", 0, 6, 20.0),
        regiment("regiment_tank", 1, 4, 15.0),
        regiment("regiment_at", 2, 6, 20.0),
        regiment("regiment_rifle9", 0, 9, 20.0),
        regiment("regiment_rifle2", 0, 2, 5.0),
        member("grenadier", 200.0, 1.0, 5.0, infantry, vec![weapon("rifle", 10.0, 3.0, 10.0, GUNFIRE), grenade]),
    ];
    let by_id = defs.iter().enumerate().map(|(i, d)| (d.id.clone(), i as u32)).collect();
    // organization_entrenched -> infantry_entrench / vehicle_entrench
    // (Properties/properties_actor_states_aa.tgi).
    let entrench = |id: &str, required: &str, resist: f32| PropertyDef {
        id: id.into(),
        required_properties: vec![required.into()],
        modifiers: vec![
            Modifier { kind: "ADD".into(), attribute: Some("DV".into()), amount: 4.0, ..Default::default() },
            Modifier { kind: "RESIST_DAMAGE".into(), amount: resist, damage_type: Some(AP), ..Default::default() },
            Modifier { kind: "RESIST_DAMAGE".into(), amount: resist, damage_type: Some(EXPLOSIVE), ..Default::default() },
            Modifier { kind: "MULTIPLY".into(), attribute: Some("MORALE_LOSS_RATE".into()), amount: 0.5, ..Default::default() },
        ],
        ..Default::default()
    };
    let properties = [entrench("infantry_entrench", "infantry", 0.4), entrench("vehicle_entrench", "vehicle", 0.2)]
        .into_iter()
        .map(|p| (p.id.clone(), p))
        .collect();
    let state = ActorStateDef {
        id: "organization_entrenched".into(),
        priority: 6,
        properties: vec!["infantry_entrench".into(), "vehicle_entrench".into()],
        ..Default::default()
    };
    GameData {
        defs,
        by_id,
        damage_types: ["gunfire", "flame", "explosive", "armor_piercing"].map(String::from).to_vec(),
        rules: rules(),
        properties,
        actor_states: [(state.id.clone(), state)].into_iter().collect(),
        ..Default::default()
    }
}

fn kingdom(id: u32, team: u32) -> Kingdom {
    Kingdom {
        id,
        name: format!("k{id}"),
        nation: "rus".into(),
        team,
        color: [255, 0, 0],
        controller: Controller::Human,
        stockpile: [0.0; 3],
        income: [0.0; 3],
        unit_limit: 0.0,
        units_used: 0.0,
        defeated: false,
        independent: false,
    }
}

fn world_with(data: GameData) -> World {
    let mut w = World::empty(Arc::new(data), Arc::new(GameMap::default()), 7);
    w.kingdoms = vec![kingdom(0, 0), kingdom(1, 1)];
    w
}

fn spawn_org(w: &mut World, def: u32, kingdom: u32, center: Vec2, facing: f32) -> OrgId {
    let id = w.orgs.len() as OrgId;
    let slots = w.data.def(def).slots.clone();
    let mut members = Vec::new();
    for (i, (d, offset)) in slots.iter().enumerate() {
        // Null layout slots (no authored element) stay empty, as in
        // production::spawn_organization.
        if *d == u32::MAX {
            continue;
        }
        let p = center + Vec2::new(offset[0], offset[1]);
        let a = w.spawn_actor(*d, Some(kingdom), Vec3::new(p.x, p.y, 0.0), facing);
        w.actors[a as usize].org = Some(id);
        w.actors[a as usize].slot = Some(i as u16);
        members.push(a);
    }
    w.orgs.push(Organization {
        id,
        def,
        kingdom,
        members,
        hq: None,
        order: Order::Idle,
        anchor: center,
        heading: facing,
        morale: 0.0,
        experience: 0.0,
        alive: true,
    });
    id
}

/// One tick of the combat owner's systems.
fn step(w: &mut World) -> Vec<Event> {
    w.events.clear();
    combat::tick(w);
    aa_play::morale::tick(w);
    w.tick += 1;
    w.time = w.tick as f32 * DT;
    std::mem::take(&mut w.events)
}

fn run_until(w: &mut World, seconds: f32, mut stop: impl FnMut(&World, &[Event]) -> bool) -> Option<f32> {
    let ticks = (seconds / DT) as u32;
    for _ in 0..ticks {
        let ev = step(w);
        if stop(w, &ev) {
            return Some(w.time);
        }
    }
    None
}

#[test]
fn damage_formula_known_stats() {
    // Rifle (10 gunfire) vs infantry (defense 1, factor 1): 9.
    assert_eq!(damage_amount(10.0, 1.0, 1.0, 1.0, 1.0, 0, 1.0), 9.0);
    // Rifle vs medium tank (defense 7, gunfire 0.4): (10 - 7) * 0.4 = 1.2.
    assert!((damage_amount(10.0, 1.0, 1.0, 7.0, 0.4, 0, 1.0) - 1.2).abs() < 1e-5);
    // Pistol (5) vs the tank: below the minimum -> CombatDamageMinimum.
    assert_eq!(damage_amount(5.0, 1.0, 1.0, 7.0, 0.4, 0, 1.0), 1.0);
    // AT (50 AP) vs the tank: 43; tank gun (70 AP) vs infantry (0.8): 55.2.
    assert_eq!(damage_amount(50.0, 1.0, 1.0, 7.0, 1.0, 0, 1.0), 43.0);
    assert!((damage_amount(70.0, 1.0, 1.0, 1.0, 0.8, 0, 1.0) - 55.2).abs() < 1e-4);
    // Flag bit0 ignores defense, bit1 forces offense 1.
    assert_eq!(damage_amount(10.0, 1.0, 1.0, 7.0, 1.0, 1, 1.0), 10.0);
    assert_eq!(damage_amount(10.0, 0.5, 1.0, 1.0, 1.0, 2, 1.0), 9.0);
    assert_eq!(damage_amount(10.0, 0.5, 1.0, 1.0, 1.0, 0, 1.0), 4.0);
    // Negative defense counts as 0; area scale multiplies the power.
    assert_eq!(damage_amount(10.0, 1.0, 0.5, -3.0, 1.0, 0, 1.0), 5.0);
    // ModifierState power / defense floors.
    assert_eq!(modified_power(10.0, 0.0, 4.0, 1.0), 14.0);
    assert_eq!(modified_power(10.0, 0.0, -20.0, 1.0), 1.0);
    assert_eq!(modified_defense(1.0, 1.0, 1.0), 2.0);
    assert_eq!(modified_defense(7.0, -10.0, 1.0), 1.0);
}

/// Two lone riflemen at 8 m: shots every 3 s after the reaction delay, 9 per
/// hit, the 23rd hit kills (200 health). The reaction delays are world-RNG
/// draws, so whichever rifleman fires first wins (re-baselined for the native
/// world stream).
#[test]
fn rifle_duel_recharge_and_death() {
    let mut w = world_with(data());
    let first = w.spawn_actor(0, Some(0), Vec3::new(0.0, 0.0, 0.0), 0.0);
    let second = w.spawn_actor(0, Some(1), Vec3::new(8.0, 0.0, 0.0), 0.0);
    let mut shots: BTreeMap<u32, Vec<f32>> = BTreeMap::new();
    let mut hits: BTreeMap<u32, Vec<f32>> = BTreeMap::new();
    let mut died = None;
    for _ in 0..(90.0 / DT) as u32 {
        for e in step(&mut w) {
            match e {
                Event::Fired { shooter, target, .. } if shooter == first || shooter == second => {
                    assert_eq!(target, if shooter == first { second } else { first });
                    shots.entry(shooter).or_default().push(w.time - DT);
                }
                Event::Hit { target, damage } if target == first || target == second => hits.entry(target).or_default().push(damage),
                Event::Died { actor } if actor == first || actor == second => died = Some((actor, w.time - DT)),
                _ => {}
            }
        }
        if died.is_some() {
            break;
        }
    }
    let (b, died) = died.expect("one rifleman dies");
    let a = if b == first { second } else { first };
    let shots_a = shots.remove(&a).unwrap_or_default();
    let hits_b = hits.remove(&b).unwrap_or_default();
    assert!(shots.get(&b).is_none_or(|s| s[0] >= shots_a[0]), "the winner fired first");
    assert!(shots_a[0] <= 0.3 + DT, "first shot after the reaction delay: {}", shots_a[0]);
    for pair in shots_a.windows(2) {
        assert!((pair[1] - pair[0] - 3.0).abs() < DT * 0.5 + 1e-4, "recharge 3 s: {pair:?}");
    }
    assert!(hits_b.iter().all(|&d| d == 9.0));
    assert_eq!(hits_b.len(), 23);
    assert!((died - shots_a[22]).abs() < 1e-4);
    eprintln!("rifle duel: first shot {:.3}s, b died at {died:.3}s after {} hits", shots_a[0], hits_b.len());
    // Dying for the death clip (fallback 1 s), then Dead, removed after rot+sink (2 s).
    assert_eq!(w.actors[b as usize].activity, Activity::Dying);
    run_until(&mut w, 1.1, |_, _| false);
    assert_eq!(w.actors[b as usize].activity, Activity::Dead);
    assert!(!w.actors[b as usize].removed);
    run_until(&mut w, 2.1, |_, _| false);
    assert!(w.actors[b as usize].removed);
}

/// AT soldier vs medium tank: 43 per hit, 11 hits; rifle barely scratches
/// the tank (1.2), a pistol does the minimum (1).
#[test]
fn at_vs_tank_and_scratches() {
    let mut d = data();
    // The tank must not shoot back in this test: an unarmed copy.
    let mut unarmed = d.defs[1].clone();
    unarmed.id = "tank_unarmed".into();
    unarmed.weapons.clear();
    d.defs.push(unarmed);
    let unarmed = d.defs.len() as u32 - 1;
    let mut w = world_with(d);
    let at = w.spawn_actor(2, Some(0), Vec3::new(0.0, 0.0, 0.0), 0.0);
    let rifle = w.spawn_actor(0, Some(0), Vec3::new(0.0, 4.0, 0.0), 0.0);
    let pistol = w.spawn_actor(3, Some(0), Vec3::new(0.0, -3.0, 0.0), 0.0);
    let tank = w.spawn_actor(unarmed, Some(1), Vec3::new(5.0, 0.0, 0.0), 0.0);
    let mut per_shooter: BTreeMap<u32, Vec<f32>> = BTreeMap::new();
    let mut last_shooter = None;
    let died = run_until(&mut w, 60.0, |w, ev| {
        for e in ev {
            match *e {
                Event::Fired { shooter, .. } => last_shooter = Some(shooter),
                Event::Hit { target, damage } if target == tank => {
                    per_shooter.entry(last_shooter.unwrap()).or_default().push(damage);
                }
                _ => {}
            }
        }
        !w.actors[tank as usize].alive
    });
    assert!(died.is_some(), "the tank dies");
    assert!(per_shooter[&at].iter().all(|&d| d == 43.0));
    assert!(per_shooter[&rifle].iter().all(|&d| (d - 1.2).abs() < 1e-5));
    assert!(per_shooter[&pistol].iter().all(|&d| d == 1.0));
    let total: f32 = per_shooter.values().flatten().sum();
    assert!(total >= 440.0);
    eprintln!("AT+rifle+pistol vs tank: dead at {:.2}s; AT hits {}, rifle hits {}, pistol hits {}", died.unwrap(), per_shooter[&at].len(), per_shooter[&rifle].len(), per_shooter[&pistol].len());
}

/// A medium tank regiment (4) against a 6-man rifle regiment at 18 m: the
/// rifles (10 m) cannot answer, the guns (18 m) can; the infantry routs.
#[test]
fn tank_regiment_vs_infantry_regiment() {
    let mut w = world_with(data());
    let inf = spawn_org(&mut w, 4, 0, Vec2::new(0.0, 0.0), 0.0);
    let tanks = spawn_org(&mut w, 5, 1, Vec2::new(18.0, 0.0), 0.0);
    let mut routed = None;
    let mut destroyed = None;
    let mut deaths = Vec::new();
    run_until(&mut w, 300.0, |w, ev| {
        for e in ev {
            match *e {
                Event::Routed { org } if org == inf => routed = routed.or(Some(w.time)),
                Event::OrgDestroyed { org } if org == inf => destroyed = Some(w.time),
                Event::Died { actor } => deaths.push((w.time, actor)),
                _ => {}
            }
        }
        destroyed.is_some()
    });
    eprintln!("tank regiment vs rifle regiment: infantry routed at {routed:?}, destroyed at {destroyed:?}; deaths {deaths:?}");
    let tank_health: Vec<f32> = w.orgs[tanks as usize].members.iter().map(|&m| w.actors[m as usize].health).collect();
    eprintln!("tank health {tank_health:?}");
    assert!(routed.is_some() && destroyed.is_some());
    assert!(routed.unwrap() < destroyed.unwrap());
}

/// Two rifle regiments facing each other: engagement, members choosing
/// targets inside the engaged regiment, death and organization destruction.
#[test]
fn regiments_engage_kill_and_destroy() {
    let mut w = world_with(data());
    let a = spawn_org(&mut w, 8, 0, Vec2::new(0.0, 0.0), 0.0);
    let b = spawn_org(&mut w, 4, 1, Vec2::new(8.0, 0.0), 0.0);
    // Regiment a is too weak to rout before it dies (breakpoint 5, 2 members).
    let mut destroyed = Vec::new();
    let mut routed = Vec::new();
    let mut deaths = 0;
    run_until(&mut w, 200.0, |w, ev| {
        for e in ev {
            match e {
                Event::OrgDestroyed { org } => destroyed.push((*org, w.time)),
                Event::Routed { org } => routed.push(*org),
                Event::Died { .. } => deaths += 1,
                _ => {}
            }
        }
        !destroyed.is_empty()
    });
    let st = combat::org_state(&w, b).unwrap();
    eprintln!("2 vs 6 riflemen: destroyed {destroyed:?}, routed {routed:?}, deaths {deaths}, b morale {} / {}", w.orgs[b as usize].morale, st.morale_max);
    assert_eq!(destroyed.first().map(|d| d.0), Some(a));
    assert!(!w.orgs[a as usize].alive);
    assert!(w.orgs[a as usize].members.is_empty());
    // Engagement of b on its own initiative is over: its order is restored.
    run_until(&mut w, 1.5, |_, _| false);
    assert_eq!(w.orgs[b as usize].order, Order::Idle);
    // b earned kill experience (2 kills of equal value -> 1 each) plus engagement time.
    assert!(w.orgs[b as usize].experience > 2.0);
}

/// Morale: an engaged regiment loses MoraleEngagementLoss per second, routs at
/// the breakpoint (Event::Routed, moves away), and after RoutingTime rests and
/// regains morale.
#[test]
fn morale_break_rout_and_regroup() {
    let mut w = world_with(data());
    let victim = spawn_org(&mut w, 4, 0, Vec2::new(0.0, 0.0), 0.0);
    let attacker = spawn_org(&mut w, 7, 1, Vec2::new(9.0, 0.0), 0.0);
    // Prepare once to initialise morale, then put the victim just above its breakpoint.
    step(&mut w);
    assert_eq!(w.orgs[victim as usize].morale, 30.0, "6 x morale_value 5");
    w.orgs[victim as usize].morale = 21.0;
    let start = w.time;
    let mut routed_at = None;
    run_until(&mut w, 20.0, |w, ev| {
        if ev.contains(&Event::Routed { org: victim }) {
            routed_at = Some(w.time);
            true
        } else {
            false
        }
    });
    let routed_at = routed_at.expect("the regiment routs");
    eprintln!("routed {:.2}s after being set 1 morale above the breakpoint (morale {})", routed_at - start, w.orgs[victim as usize].morale);
    assert!(w.orgs[victim as usize].morale <= 20.0);
    let rout = combat::org_state(&w, victim).unwrap().routing.clone().expect("routing");
    let to = rout.destination.expect("rout destination");
    assert_eq!(w.orgs[victim as usize].order, Order::Move { to, attack_move: false });
    let enemy = w.orgs[attacker as usize].anchor;
    assert!(to.distance(enemy) > Vec2::ZERO.distance(enemy), "flees away from the enemy: {to}");
    assert!(w.orgs[victim as usize].members.iter().all(|&m| w.actors[m as usize].combat.target.is_none()));
    // The attacker gained breaker morale and rout experience.
    assert!(w.orgs[attacker as usize].experience > 0.0);
    // RoutingTime later the regiment rests again (the original timer, 10 s in A&A).
    let morale_before = w.orgs[victim as usize].morale;
    let ended = run_until(&mut w, 12.0, |w, _| combat::org_state(w, victim).unwrap().routing.is_none());
    let ended = ended.expect("rout ends");
    eprintln!("rout ended {:.2}s after it began", ended - routed_at);
    assert!((ended - routed_at - 10.0).abs() <= 2.0 * DT);
    assert_eq!(w.orgs[victim as usize].order, Order::Idle);
    // Move the attacker away so it no longer engages, then the victim regains morale (MoraleReplenish 2/s, no supply).
    for &m in &w.orgs[attacker as usize].members.clone() {
        w.actors[m as usize].pos.x += 200.0;
    }
    w.orgs[attacker as usize].anchor.x += 200.0;
    let m0 = w.orgs[victim as usize].morale;
    run_until(&mut w, 2.0, |_, _| false);
    let m1 = w.orgs[victim as usize].morale;
    eprintln!("morale {morale_before} at rout -> {m0} after it -> {m1} two seconds later");
    assert!(m1 > m0 + 3.0, "replenished at 2/s");
}

/// Experience: engagement time and kills promote the regiment (150 xp ->
/// Regular: AV +4, DV +1, HEALTH x1.05) and the bonus shows in the damage.
#[test]
fn experience_promotion_changes_damage() {
    let mut w = world_with(data());
    let vet = spawn_org(&mut w, 4, 0, Vec2::new(0.0, 0.0), 0.0);
    let foe = spawn_org(&mut w, 4, 1, Vec2::new(9.0, 0.0), 0.0);
    w.orgs[vet as usize].experience = 149.0;
    // The foe must not rout before the check.
    w.orgs[foe as usize].morale = 1000.0;
    let mut promoted = None;
    let mut damages = Vec::new();
    let mut shooter = None;
    run_until(&mut w, 30.0, |w, ev| {
        for e in ev {
            match *e {
                Event::Promoted { org, level } if org == vet => promoted = Some((level, w.time)),
                Event::Fired { shooter: s, .. } => shooter = Some(s),
                Event::Hit { damage, .. } if promoted.is_some() && w.actors[shooter.unwrap() as usize].kingdom == Some(0) => damages.push(damage),
                _ => {}
            }
        }
        damages.len() >= 3
    });
    let (level, at) = promoted.expect("promoted");
    eprintln!("promoted to level {level} at {at:.2}s; hits after promotion {damages:?}");
    assert_eq!(level, 1);
    // Regular: power 10 + 4 against defense 1 (the foe is a recruit): 13.
    assert!(damages.iter().all(|&d| d == 13.0), "{damages:?}");
    let m = w.orgs[vet as usize].members[0];
    assert_eq!(combat::org_state(&w, vet).unwrap().level, 1);
    assert!((w.combat.actor(m).unwrap().max_health - 210.0).abs() < 1e-3);
}

/// Grenade: area 2 m, attenuation to 50% at the edge, shorter range wins the tie.
#[test]
fn grenade_area_damage() {
    let mut w = world_with(data());
    let g = w.spawn_actor(9, Some(0), Vec3::new(0.0, 0.0, 0.0), 0.0);
    let t1 = w.spawn_actor(3, Some(1), Vec3::new(8.0, 0.0, 0.0), 0.0);
    let t2 = w.spawn_actor(3, Some(1), Vec3::new(8.0, 1.9, 0.0), 0.0);
    let mut grenade_hits = Vec::new();
    let mut weapon_fired = None;
    run_until(&mut w, 1.0, |_, ev| {
        for e in ev {
            match *e {
                Event::Fired { shooter, weapon, .. } if shooter == g => weapon_fired = Some(weapon),
                Event::Hit { target, damage } => grenade_hits.push((target, damage)),
                _ => {}
            }
        }
        weapon_fired.is_some()
    });
    assert_eq!(weapon_fired, Some(1), "both ready: the shorter-ranged grenade is chosen");
    let d1 = grenade_hits.iter().find(|h| h.0 == t1).unwrap().1;
    let d2 = grenade_hits.iter().find(|h| h.0 == t2).unwrap().1;
    // Center: (20 - 1) = 19; at 1.9 m - 0.5 radius = 1.4 m: scale 1 - 0.5*0.4 = 0.8 -> 15.
    assert_eq!(d1, 19.0);
    assert!((d2 - 15.0).abs() < 1e-4, "{d2}");
}

/// Explicit attack orders override acquisition and follow the target's organization.
#[test]
fn explicit_attack_targets_the_order() {
    let mut w = world_with(data());
    let a = spawn_org(&mut w, 4, 0, Vec2::new(0.0, 0.0), 0.0);
    let near = spawn_org(&mut w, 4, 1, Vec2::new(9.0, 0.0), 0.0);
    let far = spawn_org(&mut w, 4, 1, Vec2::new(0.0, 9.0), 0.0);
    let target = w.orgs[far as usize].members[2];
    w.orgs[a as usize].order = Order::Attack { target };
    run_until(&mut w, 2.0, |_, _| false);
    for &m in &w.orgs[a as usize].members {
        if let Some(t) = w.actors[m as usize].combat.target {
            assert_eq!(w.actors[t as usize].org, Some(far), "member {m} targets the ordered regiment");
        }
    }
    let _ = near;
}

/// Entrenchment: a resting regiment entrenches after entrenchment_time (30 s);
/// infantry_entrench gives DV +4 and armor_piercing resist 0.4.
#[test]
fn entrenchment_after_rest() {
    let mut w = world_with(data());
    let inf = spawn_org(&mut w, 4, 0, Vec2::new(0.0, 0.0), 0.0);
    let entrenched_at = run_until(&mut w, 40.0, |w, _| combat::org_state(w, inf).unwrap().entrenched);
    let at = entrenched_at.expect("entrenches");
    eprintln!("entrenched after {at:.2}s of rest");
    assert!((at - 30.0).abs() <= 2.0 * DT);
    // A tank now hits the entrenched infantry for (70 - (1 + 4)) * (0.8 - 0.4) = 26.
    let tank = w.spawn_actor(1, Some(1), Vec3::new(15.0, 0.0, 0.0), 0.0);
    let mut hit = None;
    run_until(&mut w, 2.0, |_, ev| {
        for e in ev {
            if let Event::Hit { damage, .. } = *e {
                hit = Some(damage);
            }
        }
        hit.is_some()
    });
    assert_eq!(hit, Some(26.0));
    let _ = tank;
    // A move order leaves the trenches.
    w.orgs[inf as usize].order = Order::Move { to: Vec2::new(-50.0, 0.0), attack_move: false };
    step(&mut w);
    assert!(!combat::org_state(&w, inf).unwrap().entrenched);
}

/// A capturable city at zero health changes owner with CaptureHealthFactor.
#[test]
fn city_capture() {
    let mut d = data();
    d.defs.push(Def {
        id: "city".into(),
        class: Some(DefClass::Building),
        health: 100.0,
        defense: 0.0,
        captureable: true,
        destructible: true,
        // (core) 4D1120: an actor with a CityComponent is never captured by
        // damage (cities change hands by presence, supply.rs City tick), so
        // the damage capture is tested on a captureable non-city building.
        city: false,
        damage_factor: vec![1.0; 4],
        ..Default::default()
    });
    let city_def = d.defs.len() as u32 - 1;
    let mut w = world_with(d);
    let rifle = w.spawn_actor(0, Some(0), Vec3::new(0.0, 0.0, 0.0), 0.0);
    let city = w.spawn_actor(city_def, Some(1), Vec3::new(6.0, 0.0, 0.0), 0.0);
    let mut captured = None;
    run_until(&mut w, 60.0, |w, ev| {
        for e in ev {
            if let Event::Captured { actor, kingdom } = *e {
                captured = Some((actor, kingdom, w.time));
            }
        }
        captured.is_some()
    });
    let (actor, kingdom, at) = captured.expect("captured");
    eprintln!("city captured at {at:.2}s");
    assert_eq!((actor, kingdom), (city, 0));
    assert_eq!(w.actors[city as usize].kingdom, Some(0));
    assert!(w.actors[city as usize].alive);
    assert!((w.actors[city as usize].health - 30.0).abs() < 1e-4);
    // Now friendly: the rifleman stops shooting it.
    step(&mut w);
    step(&mut w);
    assert_eq!(w.actors[rifle as usize].combat.target, None);
}

/// A member brought to 0 health outside combat (upkeep shortfall) dies the
/// same way: Died, Dying -> Dead -> removed, removed from its regiment with
/// the morale loss, no kill credit even when an enemy shot it earlier; the
/// last member's death destroys the regiment without credit.
#[test]
fn shortfall_death_takes_the_death_path() {
    let mut w = world_with(data());
    let org = spawn_org(&mut w, 8, 0, Vec2::new(0.0, 0.0), 0.0);
    let foe = spawn_org(&mut w, 4, 1, Vec2::new(200.0, 0.0), 0.0);
    step(&mut w);
    let [a, b] = [w.orgs[org as usize].members[0], w.orgs[org as usize].members[1]];
    let morale = w.orgs[org as usize].morale;
    // An earlier enemy hit must not turn the shortfall death into a kill.
    let shooter = w.orgs[foe as usize].members[0];
    w.combat.actors[a as usize].last_hit = Some((shooter, w.time));
    w.combat.orgs[org as usize].last_hit = Some((shooter, w.time));
    w.actors[a as usize].health = 0.0;
    let ev = step(&mut w);
    assert!(ev.contains(&Event::Died { actor: a }));
    assert!(!w.actors[a as usize].alive);
    assert_eq!(w.actors[a as usize].activity, Activity::Dying);
    assert_eq!(w.orgs[org as usize].members, vec![b]);
    assert_eq!(w.actors[a as usize].org, None);
    // MoraleDeathFactor 0.25 x morale_value 5, less one tick of replenish (3/s).
    let lost = morale - w.orgs[org as usize].morale;
    assert!((lost - (1.25 - 3.0 * DT)).abs() < 1e-4, "MoraleDeathFactor loss {lost}");
    assert_eq!(w.orgs[foe as usize].experience, 0.0);
    w.actors[b as usize].health = -3.0;
    let ev = step(&mut w);
    assert!(ev.contains(&Event::Died { actor: b }));
    assert!(ev.contains(&Event::OrgDestroyed { org }));
    assert!(!w.orgs[org as usize].alive);
    assert_eq!(w.orgs[foe as usize].experience, 0.0);
    run_until(&mut w, 3.2, |_, _| false);
    assert!(w.actors[a as usize].removed && w.actors[b as usize].removed);
    // A non-destructible building at 0 health is restored to 1.
    let mut d = data();
    d.defs.push(Def { id: "hq".into(), class: Some(DefClass::Building), health: 500.0, destructible: false, ..Default::default() });
    let hq_def = d.defs.len() as u32 - 1;
    let mut w = world_with(d);
    let hq = w.spawn_actor(hq_def, Some(0), Vec3::ZERO, 0.0);
    w.actors[hq as usize].health = 0.0;
    step(&mut w);
    assert!(w.actors[hq as usize].alive);
    assert_eq!(w.actors[hq as usize].health, 1.0);
}

/// Die clip choice (DieAnimation): threshold, damage filter group, 666 for a
/// death without a damage type, weighted pick.
#[test]
fn death_clip_choice() {
    let clip = |id: &str, length: f32, weight: f32, filter: f32, threshold: f32| DeathClip { id: id.into(), length, weight, damage_filter: filter, damage_threshold: threshold };
    let clips = vec![clip("die1", 1.0, 1.0, -1.0, 0.0), clip("die_big", 4.0, 1.0, -1.0, 50.0), clip("die_fire", 3.0, 1.0, 1.0, 0.0), clip("bleed", 2.0, 1.0, 666.0, 0.0)];
    // Gunfire (group 0) for 9: only the generic clip.
    assert_eq!(combat::choose_death_clip(&clips, 9.0, Some(0), 0.99), Some(0));
    // Explosive group 2 for 55: generic or big, by weight.
    assert_eq!(combat::choose_death_clip(&clips, 55.0, Some(2), 0.1), Some(0));
    assert_eq!(combat::choose_death_clip(&clips, 55.0, Some(2), 0.9), Some(1));
    // Flame: generic or fire.
    assert_eq!(combat::choose_death_clip(&clips, 5.0, Some(1), 0.9), Some(2));
    // No damage type (shortfall): generic or bleeding.
    assert_eq!(combat::choose_death_clip(&clips, 0.0, None, 0.9), Some(3));
    // The chosen clip sets the Dying time.
    let mut d = data();
    d.defs[0].death_clips = vec![clip("die_long", 2.5, 1.0, -1.0, 0.0)];
    d.defs[0].death_time = Some(2.5);
    let mut w = world_with(d);
    let a = w.spawn_actor(0, Some(0), Vec3::ZERO, 0.0);
    w.actors[a as usize].health = 0.0;
    step(&mut w);
    assert_eq!(w.combat.actor(a).unwrap().death_clip, Some(0));
    run_until(&mut w, 2.0, |_, _| false);
    assert_eq!(w.actors[a as usize].activity, Activity::Dying);
    run_until(&mut w, 0.6, |_, _| false);
    assert_eq!(w.actors[a as usize].activity, Activity::Dead);
}

/// Projectiles land after distance / projectile_speed.
#[test]
fn projectile_flight_time() {
    let mut d = data();
    d.defs[1].weapons[0].projectile_speed = 40.0;
    let mut w = world_with(d);
    let tank = w.spawn_actor(1, Some(0), Vec3::ZERO, 0.0);
    let target = w.spawn_actor(3, Some(1), Vec3::new(16.0, 0.0, 0.0), 0.0);
    let mut fired = None;
    let mut hit = None;
    run_until(&mut w, 3.0, |w, ev| {
        for e in ev {
            match *e {
                Event::Fired { shooter, .. } if shooter == tank && fired.is_none() => fired = Some(w.time),
                Event::Hit { target: t, .. } if t == target && hit.is_none() => hit = Some(w.time),
                _ => {}
            }
        }
        hit.is_some()
    });
    let flight = hit.unwrap() - fired.unwrap();
    eprintln!("16 m at 40 m/s: hit {flight:.3}s after the shot");
    assert!((flight - 0.4).abs() <= DT + 1e-4, "{flight}");
}

/// ROE: a primary (leader slot) away from its anchor that is under fire and
/// below primary_minimum_health stops attacking for ROEPrimaryEngageReturnTimeout.
#[test]
fn roe_primary_backs_off() {
    let mut d = data();
    d.defs[0].primary_minimum_health = 0.5;
    d.defs[4].leader_slot = Some(0);
    d.rules.insert("ROEPrimaryEngageReturnTimeout".into(), 5.0);
    let mut w = world_with(d);
    let org = spawn_org(&mut w, 4, 0, Vec2::new(0.0, 0.0), 0.0);
    let foe = w.spawn_actor(0, Some(1), Vec3::new(-2.0, 8.0, 0.0), 0.0);
    let primary = w.orgs[org as usize].members[0];
    w.orgs[org as usize].anchor = Vec2::new(0.0, -20.0);
    w.actors[primary as usize].health = 80.0;
    let mut primary_shots = 0;
    run_until(&mut w, 4.0, |_, ev| {
        primary_shots += ev.iter().filter(|e| matches!(e, Event::Fired { shooter, .. } if *shooter == primary)).count();
        false
    });
    let st = w.combat.actor(primary).unwrap();
    eprintln!("primary shots {primary_shots}, retry at {:.2}", st.roe_retry_at);
    assert_eq!(primary_shots, 0);
    assert!(st.roe_retry_at > 0.0);
    let _ = foe;
}

/// 2 x 75 regiments x 9 members fighting: combat + morale tick time.
#[test]
fn performance_150_regiments() {
    let mut w = world_with(data());
    for side in 0..2u32 {
        for r in 0..75 {
            let x = (r % 15) as f32 * 40.0;
            let y = (r / 15) as f32 * 60.0 + if side == 0 { 0.0 } else { 16.0 };
            spawn_org(&mut w, 7, side, Vec2::new(x, y), 0.0);
        }
    }
    assert_eq!(w.actors.len(), 1350);
    for _ in 0..16 {
        step(&mut w);
    }
    let ticks = 1600;
    let mut worst = std::time::Duration::ZERO;
    let mut fired = 0;
    let t0 = std::time::Instant::now();
    for _ in 0..ticks {
        let t = std::time::Instant::now();
        let ev = step(&mut w);
        worst = worst.max(t.elapsed());
        fired += ev.iter().filter(|e| matches!(e, Event::Fired { .. })).count();
    }
    let avg = t0.elapsed() / ticks;
    let dead = w.actors.iter().filter(|a| !a.alive).count();
    eprintln!("150 regiments x 9: avg {avg:?} per tick, worst {worst:?}, {fired} shots in {ticks} ticks, {dead} dead");
    assert!(fired > 1000);
    if !cfg!(debug_assertions) {
        assert!(avg < std::time::Duration::from_millis(3), "{avg:?}");
    }
}

// ---------------------------------------------------------------------------
// Real data

fn real() -> Option<Arc<GameData>> {
    let root = std::path::PathBuf::from(std::env::var("AA_GAME_ROOT").unwrap_or_else(|_| r"E:\Games\Axis and Alies".into()));
    match GameData::load(&root) {
        Ok(d) => Some(Arc::new(d)),
        Err(e) => {
            eprintln!("GameData::load failed: {e}");
            None
        }
    }
}

#[test]
#[ignore]
fn real_data_infantry_and_tank() {
    let data = real().expect("original data");
    let rules = combat::Rules::from_data(&data);
    eprintln!("rules missing from GameData: {:?}", rules.missing);
    let extras = combat::Extras::from_data(&data, &rules);
    eprintln!("extras fallbacks: {:?}; target bits enemy {:?} routing {:?}", extras.fallbacks, extras.bit_enemy, extras.bit_routing);
    let show = |id: &str| {
        let Some(d) = data.find(id) else {
            eprintln!("{id}: missing");
            return None;
        };
        let def = data.def(d);
        eprintln!(
            "{id}: health {} defense {} morale {} armor {} factors {:?} props {:?} cv {:.2}",
            def.health, def.defense, def.morale_value, def.armor_type, def.damage_factor, def.properties, extras.def(d).combat_value
        );
        for w in &def.weapons {
            eprintln!(
                "   {} range {} min {} recharge {} dmg {} type {} area {} att {}/{} flags {:#x} req {:#x} forb {:#x} turret {} proj {:?}",
                w.name, w.range, w.min_range, w.recharge, w.damage,
                data.damage_types.get(w.damage_type as usize).cloned().unwrap_or_default(), w.area, w.attenuation_range_min, w.attenuation_factor_max, w.flags, w.targets_required, w.targets_forbidden, w.turret, w.projectile
            );
        }
        Some(d)
    };
    let inf = show("rus_infantry").expect("rus_infantry");
    let tank = show("rus_tank_medium").expect("rus_tank_medium");
    let ger = show("ger_infantry");
    let at = show("rus_infantry_at");
    for org in ["regiment_rus_infantry", "regiment_ger_infantry", "regiment_rus_tank_medium"] {
        if let Some(o) = data.find(org) {
            let d = data.def(o);
            eprintln!(
                "{org}: slots {} breakpoint {} control {} operational {} entrench {} sight {} detection {}",
                d.slots.len(), d.morale_breakpoint, d.control_range, d.operational_range, d.entrenchment_time, d.sight_range, d.detection_range
            );
        }
    }
    let rifle = &data.def(inf).weapons[0];
    let f = |w: &Weapon, t: u32| data.def(t).damage_factor.get(w.damage_type as usize).copied().unwrap_or(1.0);
    let hit = |w: &Weapon, t: u32| damage_amount(w.damage, 1.0, 1.0, data.def(t).defense, f(w, t), w.flags, rules.damage_minimum);
    eprintln!("rus rifle vs rus infantry: {}", hit(rifle, inf));
    eprintln!("rus rifle vs rus medium tank: {}", hit(rifle, tank));
    let gun = &data.def(tank).weapons[0];
    eprintln!("medium tank gun vs rus infantry: {}", hit(gun, inf));
    if let Some(at) = at {
        let w = &data.def(at).weapons[0];
        eprintln!("rus AT vs medium tank: {}", hit(w, tank));
    }
    // Regiment duel at 9 m (no movement): seconds until one regiment is destroyed or routs.
    let (Some(r1), Some(r2)) = (data.find("regiment_rus_infantry"), data.find("regiment_ger_infantry")) else { return };
    let _ = ger;
    let mut w = World::empty(data.clone(), Arc::new(GameMap::default()), 11);
    w.kingdoms = vec![kingdom(0, 0), kingdom(1, 1)];
    let a = spawn_org(&mut w, r1, 0, Vec2::new(0.0, 0.0), 0.0);
    let b = spawn_org(&mut w, r2, 1, Vec2::new(0.0, 12.0), 0.0);
    let mut log = Vec::new();
    run_until(&mut w, 240.0, |w, ev| {
        for e in ev {
            match e {
                Event::Routed { .. } | Event::OrgDestroyed { .. } | Event::Promoted { .. } => log.push((w.time, e.clone())),
                _ => {}
            }
        }
        !w.orgs[a as usize].alive || !w.orgs[b as usize].alive
    });
    eprintln!("rus vs ger infantry regiments at 12 m: {log:?}");
    eprintln!(
        "end: rus morale {:.1} members {}, ger morale {:.1} members {}",
        w.orgs[a as usize].morale, w.orgs[a as usize].members.len(), w.orgs[b as usize].morale, w.orgs[b as usize].members.len()
    );
}

fn open_map(width: u32, height: u32, cell: f32) -> GameMap {
    let mut flags = vec![0; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                flags[(y * width + x) as usize] = aa_play::map::BLOCKED;
            }
        }
    }
    GameMap { id: "open".into(), width, height, cell_size: cell, flags, speed_limit: vec![f32::MAX; (width * height) as usize], ..Default::default() }
}

/// The whole simulation (`World::step`, with movement) on an open map with
/// the original data: an attack-moving Russian rifle regiment meets an idle
/// German one; then a medium tank regiment attacks the survivor.
#[test]
#[ignore]
fn real_data_battle_with_movement() {
    let data = real().expect("original data");
    let (Some(r1), Some(r2), Some(t1)) = (data.find("regiment_rus_infantry"), data.find("regiment_ger_infantry"), data.find("regiment_rus_tank_medium")) else {
        panic!("regiment definitions")
    };
    let mut w = World::empty(data.clone(), Arc::new(open_map(128, 128, 4.0)), 3);
    w.kingdoms = vec![kingdom(0, 0), kingdom(1, 1)];
    let rus = spawn_org(&mut w, r1, 0, Vec2::new(150.0, 250.0), 0.0);
    let ger = spawn_org(&mut w, r2, 1, Vec2::new(240.0, 250.0), 0.0);
    w.orgs[rus as usize].order = Order::Move { to: Vec2::new(300.0, 250.0), attack_move: true };
    let mut log = Vec::new();
    let mut first_shot = None;
    let mut tanks = None;
    for _ in 0..(240.0 / DT) as u32 {
        w.step(Vec::new());
        for e in w.events.clone() {
            match e {
                Event::Fired { .. } if first_shot.is_none() => first_shot = Some(w.time),
                Event::Routed { org } => {
                    let p = combat::org_state(&w, org).map(|s| s.position);
                    log.push(format!("{:.1}s routed {org} at {p:?} -> {:?}", w.time, w.orgs[org as usize].order));
                }
                Event::OrgDestroyed { org } => log.push(format!("{:.1}s destroyed {org}", w.time)),
                Event::Promoted { org, level } => log.push(format!("{:.1}s promoted {org} to {level}", w.time)),
                _ => {}
            }
        }
        if tanks.is_none() && (!w.orgs[rus as usize].alive || !w.orgs[ger as usize].alive) {
            let (side, survivor) = if w.orgs[rus as usize].alive { (1, rus) } else { (0, ger) };
            let at = combat::org_state(&w, survivor).map(|s| s.position).unwrap_or_default();
            let t = spawn_org(&mut w, t1, side, at + Vec2::new(-80.0, 30.0), 0.0);
            let target = w.orgs[survivor as usize].members[0];
            w.orgs[t as usize].order = Order::Attack { target };
            log.push(format!("{:.1}s tank regiment {t} attacks {survivor}", w.time));
            tanks = Some((t, survivor));
        }
        if let Some((t, s)) = tanks
            && (!w.orgs[t as usize].alive || !w.orgs[s as usize].alive)
        {
            break;
        }
    }
    eprintln!("first shot at {first_shot:?}");
    for l in &log {
        eprintln!("{l}");
    }
    for o in &w.orgs {
        let s = combat::org_state(&w, o.id).unwrap();
        eprintln!(
            "org {} alive {} members {} morale {:.1}/{:.1} xp {:.1} level {} order {:?} pos {:?}",
            o.id, o.alive, o.members.len(), o.morale, s.morale_max, o.experience, s.level, o.order, s.position
        );
    }
}
