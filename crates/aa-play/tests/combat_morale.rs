//! Combat and morale checks on synthetic data with known stats, plus an
//! ignored real-data run (AA_GAME_ROOT, e.g. `<game root>`):
//! `cargo test -p aa-play --test combat_morale -- --ignored --nocapture`.
//! The ticks call `combat::tick`, `morale::tick` and the zone observation (no
//! movement), so units stay where they were placed. A regiment fights what enters
//! its ZOC (zone type 4, `0x514DA0` -> `0x57AB70`), so the scenes stand on a flat
//! map the zone engine can describe, with the native blocked border.
use aa_play::combat::{self, damage_amount, modified_defense, modified_power};
use aa_play::data::{ActorStateDef, DeathClip, Def, DefClass, GameData, Modifier, PropertyDef, Weapon};
use aa_play::map::GameMap;
use aa_play::world::{Activity, Command, Controller, Event, Kingdom, Order, Organization, Unit, World};
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
        // Thing `[Flags] targetable`, actor +8 bit 14: what 0x514DA0 asks of a
        // thing before the ZOC's enemy-entered effects.
        targetable: true,
        // `Size` soft radius of the shipped infantry: the area query 558040 and
        // the attenuation distance of 4C9550 phase 3 take it off.
        soft_radius: 0.5,
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

/// The middle of the map the synthetic scenes stand on.
const C: Vec2 = Vec2::new(400.0, 400.0);

/// A point of a scene, from the middle of the map.
fn o(x: f32, y: f32) -> Vec2 {
    C + Vec2::new(x, y)
}

/// A standing point of a scene, from the middle of the map.
fn spot(x: f32, y: f32) -> Vec3 {
    o(x, y).extend(0.0)
}

fn world_with(data: GameData) -> World {
    let mut w = World::empty(Arc::new(data), Arc::new(open_map(200, 200, 4.0)), 7);
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

/// One update. The fight is on the Minds (`0x524910`, `0x531D30` on queue 4), so it takes
/// the whole of it, not the combat pass alone.
fn step(w: &mut World) -> Vec<Event> {
    w.step(Vec::new());
    // The zones after the things: their tile changes and what the ZOCs ask.
    aa_play::fog::observe_now(w);
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
    let first = w.spawn_actor(0, Some(0), spot(0.0, 0.0), 0.0);
    let second = w.spawn_actor(0, Some(1), spot(8.0, 0.0), 0.0);
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
    let at = w.spawn_actor(2, Some(0), spot(0.0, 0.0), 0.0);
    let rifle = w.spawn_actor(0, Some(0), spot(0.0, 4.0), 0.0);
    let pistol = w.spawn_actor(3, Some(0), spot(0.0, -3.0), 0.0);
    let tank = w.spawn_actor(unarmed, Some(1), spot(5.0, 0.0), 0.0);
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
    let inf = spawn_org(&mut w, 4, 0, o(0.0, 0.0), 0.0);
    let tanks = spawn_org(&mut w, 5, 1, o(18.0, 0.0), 0.0);
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
    let a = spawn_org(&mut w, 8, 0, o(0.0, 0.0), 0.0);
    let b = spawn_org(&mut w, 4, 1, o(8.0, 0.0), 0.0);
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
    let victim = spawn_org(&mut w, 4, 0, o(0.0, 0.0), 0.0);
    let attacker = spawn_org(&mut w, 7, 1, o(9.0, 0.0), 0.0);
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
    assert!(to.distance(enemy) > C.distance(enemy), "flees away from the enemy: {to}");
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
    let vet = spawn_org(&mut w, 4, 0, o(0.0, 0.0), 0.0);
    let foe = spawn_org(&mut w, 4, 1, o(9.0, 0.0), 0.0);
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
    let g = w.spawn_actor(9, Some(0), spot(0.0, 0.0), 0.0);
    let t1 = w.spawn_actor(3, Some(1), spot(8.0, 0.0), 0.0);
    let t2 = w.spawn_actor(3, Some(1), spot(8.0, 1.9), 0.0);
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
    let a = spawn_org(&mut w, 4, 0, o(0.0, 0.0), 0.0);
    let near = spawn_org(&mut w, 4, 1, o(9.0, 0.0), 0.0);
    let far = spawn_org(&mut w, 4, 1, o(0.0, 9.0), 0.0);
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

/// `0x51D1C0` stop: the sleep state replaces the engage state (`0x52A420`), whose
/// exit `0x520FE0` empties the list (`0x51F040`). A thing that stands in the ZOC and
/// does not shoot is not entered again, so the regiment stops for good.
#[test]
fn stop_ends_an_attack_on_a_thing_that_does_not_shoot() {
    let mut d = data();
    d.defs.push(Def { id: "depot".into(), class: Some(DefClass::Building), health: 750.0, destructible: true, targetable: true, damage_factor: vec![1.0; 4], ..Default::default() });
    let depot = d.defs.len() as u32 - 1;
    let mut w = world_with(d);
    let a = spawn_org(&mut w, 4, 0, o(0.0, 0.0), 0.0);
    let t = w.spawn_actor(depot, Some(1), spot(9.0, 0.0), 0.0);
    let fired = |ev: &[Event]| ev.iter().any(|e| matches!(e, Event::Fired { .. }));
    assert!(run_until(&mut w, 5.0, |_, ev| fired(ev)).is_some(), "a regiment shoots an enemy thing in its ZOC, unordered");
    assert_eq!(w.orgs[a as usize].order, Order::Attack { target: t });
    w.apply(Command::Stop { kingdom: 0, units: vec![Unit::Org(a)] });
    assert!(run_until(&mut w, 10.0, |_, ev| fired(ev)).is_none(), "stop ends the shooting");
    assert_eq!(w.orgs[a as usize].order, Order::Idle);
    assert!(combat::org_state(&w, a).unwrap().engage.is_empty());
    assert!(w.orgs[a as usize].members.iter().all(|&m| w.actors[m as usize].combat.target.is_none()));
}

/// `0x51D920` -> `0x52A810(2)`: a new kill order replaces the Engage state, whose exit
/// `0x520FE0` empties the list; the regiment is in the Kill state on the new target.
#[test]
fn a_new_attack_empties_the_engage_list() {
    let mut w = world_with(data());
    let a = spawn_org(&mut w, 4, 0, o(0.0, 0.0), 0.0);
    let near = spawn_org(&mut w, 4, 1, o(9.0, 0.0), 0.0);
    let far = spawn_org(&mut w, 4, 1, o(0.0, 60.0), 0.0);
    assert!(run_until(&mut w, 5.0, |w, _| combat::org_state(w, a).is_some_and(|c| c.engaging())).is_some(), "it engages what is in its ZOC");
    assert!(matches!(aa_play::morale::regiment_state(&w, a), aa_play::morale::RegimentState::Engage));
    let target = w.orgs[far as usize].members[0];
    w.apply(Command::Attack { kingdom: 0, units: vec![Unit::Org(a)], target });
    let c = combat::org_state(&w, a).unwrap();
    assert!(c.engage.is_empty(), "the list went with the state: {:?}", c.engage);
    assert_eq!(c.explicit, Some(combat::TargetRef::Org(far)));
    assert!(matches!(aa_play::morale::regiment_state(&w, a), aa_play::morale::RegimentState::Kill));
    let _ = near;
}

/// `0x521F90` with no Kill state under the Engage state: when the list has emptied, the
/// fight goes through the Reform state (`0x523D50`).
#[test]
fn a_fight_that_ends_reforms() {
    let mut w = world_with(data());
    let a = spawn_org(&mut w, 4, 0, o(0.0, 0.0), 0.0);
    let b = spawn_org(&mut w, 4, 1, o(9.0, 0.0), 0.0);
    assert!(run_until(&mut w, 8.0, |w, _| combat::org_state(w, a).is_some_and(|c| c.engaging())).is_some());
    let reformed = |w: &World, o: OrgId| matches!(aa_play::morale::regiment_state(w, o), aa_play::morale::RegimentState::Reform);
    assert!(run_until(&mut w, 180.0, |w, _| reformed(w, a) || reformed(w, b)).is_some(), "one side reforms when its targets are gone");
}

/// Entrenchment: a resting regiment entrenches after entrenchment_time (30 s);
/// infantry_entrench gives DV +4 and armor_piercing resist 0.4.
#[test]
fn entrenchment_after_rest() {
    let mut w = world_with(data());
    let inf = spawn_org(&mut w, 4, 0, o(0.0, 0.0), 0.0);
    let entrenched_at = run_until(&mut w, 40.0, |w, _| combat::org_state(w, inf).unwrap().entrenched);
    let at = entrenched_at.expect("entrenches");
    eprintln!("entrenched after {at:.2}s of rest");
    assert!((at - 30.0).abs() <= 2.0 * DT);
    // A tank now hits the entrenched infantry for (70 - (1 + 4)) * (0.8 - 0.4) = 26.
    let tank = w.spawn_actor(1, Some(1), spot(15.0, 0.0), 0.0);
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
    // A move order leaves the trenches when the Move dispatches (0x525740 -> 0x579A90).
    // With the tank still shooting, the damage (0x579490) would put the Engage state back
    // over the Move first, and the trench stays: the tank goes.
    w.actors[tank as usize].alive = false;
    w.actors[tank as usize].removed = true;
    w.orgs[inf as usize].order = Order::Move { to: o(-50.0, 0.0), attack_move: false };
    assert!(run_until(&mut w, 2.0, |w, _| !combat::org_state(w, inf).unwrap().entrenched).is_some());
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
    let rifle = w.spawn_actor(0, Some(0), spot(0.0, 0.0), 0.0);
    let city = w.spawn_actor(city_def, Some(1), spot(6.0, 0.0), 0.0);
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
    let org = spawn_org(&mut w, 8, 0, o(0.0, 0.0), 0.0);
    let foe = spawn_org(&mut w, 4, 1, o(200.0, 0.0), 0.0);
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
    // A non-destructible building that cannot be captured stays at 0 health
    // (`5625C0`; the Comrades film's block e81 holds hp 0 for 168 ticks).
    let mut d = data();
    d.defs.push(Def { id: "hq".into(), class: Some(DefClass::Building), health: 500.0, destructible: false, ..Default::default() });
    let hq_def = d.defs.len() as u32 - 1;
    let mut w = world_with(d);
    let hq = w.spawn_actor(hq_def, Some(0), spot(0.0, 0.0), 0.0);
    w.actors[hq as usize].health = 0.0;
    step(&mut w);
    assert!(w.actors[hq as usize].alive);
    assert_eq!(w.actors[hq as usize].health, 0.0);
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
    let a = w.spawn_actor(0, Some(0), spot(0.0, 0.0), 0.0);
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
    let tank = w.spawn_actor(1, Some(0), spot(0.0, 0.0), 0.0);
    let target = w.spawn_actor(3, Some(1), spot(16.0, 0.0), 0.0);
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
    let org = spawn_org(&mut w, 4, 0, o(0.0, 0.0), 0.0);
    let foe = w.spawn_actor(0, Some(1), spot(-2.0, 8.0), 0.0);
    let primary = w.orgs[org as usize].members[0];
    w.orgs[org as usize].anchor = o(0.0, -20.0);
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
            spawn_org(&mut w, 7, side, Vec2::new(x, y) + Vec2::new(100.0, 200.0), 0.0);
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
    let root = std::path::PathBuf::from(std::env::var("AA_GAME_ROOT").unwrap_or_default());
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

// ---------------------------------------------------------------------------
// A real map through `World::step`, with no computer player.

/// `(2v2)Berlin` with two human kingdoms (no SAI) and the spot nearest the
/// map's centre with `clear` metres of passable land around it and no
/// armed thing within 120 m.
fn berlin_scene(clear: f32) -> (World, Vec2) {
    let data = real().expect("original data");
    let root = std::path::PathBuf::from(std::env::var("AA_GAME_ROOT").expect("set AA_GAME_ROOT to the folder of the installed game"));
    let map = Arc::new(GameMap::load_with(&root, "(2v2)Berlin", &data).expect("Berlin"));
    let player = |name: &str, nation: &str, team, start| aa_play::PlayerSetup { name: name.into(), nation: nation.into(), team, color: [0, 76, 255], controller: Controller::Human, start };
    let setup = aa_play::SkirmishSetup {
        map_id: "(2v2)Berlin".into(),
        players: vec![player("A", "rus", 0, 0), player("B", "ger", 1, 2)],
        starting_resources: None,
        seed: 5,
    };
    let w = World::new_skirmish(data, map.clone(), &setup).expect("new_skirmish");
    let tile = map.cell_size;
    let open = |p: Vec2| {
        let r = (clear / tile) as i32;
        let (cx, cy) = ((p.x / tile) as i32, (p.y / tile) as i32);
        (-r..=r).all(|dy| {
            (-r..=r).all(|dx| {
                let (x, y) = (cx + dx, cy + dy);
                x >= 0 && y >= 0 && x < map.width as i32 && y < map.height as i32 && map.flags[(y as u32 * map.width + x as u32) as usize] & (aa_play::map::BLOCKED | aa_play::map::NOT_LAND_PASSABLE) == 0
            })
        })
    };
    let mid = Vec2::new(map.width as f32, map.height as f32) * tile * 0.5;
    let mut spots = Vec::new();
    for y in 0..map.height {
        for x in 0..map.width {
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * tile;
            if w.actors.iter().all(|a| a.removed || w.data.def(a.def).weapons.is_empty() || a.pos.truncate().distance(p) > 120.0) && open(p) {
                spots.push(p);
            }
        }
    }
    spots.sort_by(|a, b| a.distance(mid).total_cmp(&b.distance(mid)));
    let spot = *spots.first().expect("an open spot on Berlin");
    (w, spot)
}

#[derive(Default)]
struct Fight {
    log: Vec<String>,
    first_shot: Option<f32>,
    shots: BTreeMap<String, u32>,
    damage: BTreeMap<String, Vec<f32>>,
    deaths: u32,
}

/// `World::step` for `seconds` with what happens noted, until `stop`.
fn watch(w: &mut World, f: &mut Fight, seconds: f32, mut stop: impl FnMut(&World) -> bool) {
    for _ in 0..(seconds / DT) as u32 {
        w.step(Vec::new());
        let mut last: Option<String> = None;
        for e in w.events.clone() {
            match e {
                Event::Fired { shooter, weapon, .. } => {
                    f.first_shot = f.first_shot.or(Some(w.time));
                    let def = w.data.def(w.actors[shooter as usize].def);
                    let name = format!("k{} {} {}", w.actors[shooter as usize].kingdom.unwrap_or(99), def.id, def.weapons[weapon as usize].name);
                    *f.shots.entry(name.clone()).or_default() += 1;
                    last = Some(name);
                }
                Event::Hit { damage, .. } => {
                    if let Some(n) = &last {
                        f.damage.entry(n.clone()).or_default().push(damage);
                    }
                }
                Event::Died { .. } => f.deaths += 1,
                Event::Routed { org } => f.log.push(format!("{:.1}s routed {org} morale {:.1} -> {:?}", w.time, w.orgs[org as usize].morale, w.orgs[org as usize].order)),
                Event::OrgDestroyed { org } => f.log.push(format!("{:.1}s destroyed {org}", w.time)),
                Event::Promoted { org, level } => f.log.push(format!("{:.1}s promoted {org} to {level}", w.time)),
                Event::Refused { reason, .. } => f.log.push(format!("{:.1}s refused {reason}", w.time)),
                _ => {}
            }
        }
        if stop(w) {
            break;
        }
    }
}

fn show(w: &World, f: &Fight, orgs: &[OrgId]) {
    eprintln!("first shot {:?}, deaths {}", f.first_shot, f.deaths);
    for l in &f.log {
        eprintln!("  {l}");
    }
    for (n, c) in &f.shots {
        let d = f.damage.get(n).cloned().unwrap_or_default();
        let (min, max) = d.iter().fold((f32::MAX, 0.0f32), |(a, b), &x| (a.min(x), b.max(x)));
        eprintln!("  {n}: {c} shots, {} hits {min}..{max}", d.len());
    }
    for &o in orgs {
        let s = combat::org_state(w, o).unwrap();
        let g = &w.orgs[o as usize];
        eprintln!("  org {o} alive {} members {} morale {:.1}/{:.1} xp {:.1} level {} order {:?} pos {} engage {}", g.alive, g.members.len(), g.morale, s.morale_max, g.experience, s.level, g.order, s.position, s.engage.len());
    }
}

/// On a real map, two enemy regiments that come near each other engage and
/// shoot with no attack order; men die.
#[test]
#[ignore]
fn real_map_regiments_fight() {
    let (mut w, c) = berlin_scene(30.0);
    let d = w.data.clone();
    let id = |n: &str| d.find(n).unwrap_or_else(|| panic!("{n}"));
    eprintln!("Berlin spot {c}");
    let a = aa_play::production::spawn_organization(&mut w, id("regiment_rus_infantry"), 0, c - Vec2::new(55.0, 0.0), 0.0, None);
    let b = aa_play::production::spawn_organization(&mut w, id("regiment_ger_infantry"), 1, c + Vec2::new(55.0, 0.0), std::f32::consts::PI, None);
    let mut f = Fight::default();
    watch(&mut w, &mut f, 5.0, |_| false);
    assert!(f.first_shot.is_none(), "too far apart to shoot");
    w.step(vec![aa_play::Command::Move { kingdom: 0, units: vec![aa_play::Unit::Org(a)], to: c + Vec2::new(40.0, 0.0), attack_move: false }]);
    watch(&mut w, &mut f, 240.0, |w| !w.orgs[a as usize].alive || !w.orgs[b as usize].alive);
    eprintln!("--- plain move into the enemy");
    show(&w, &f, &[a, b]);
    assert!(f.first_shot.is_some(), "they shoot with no attack order");
    assert!(f.deaths > 0, "men die");
}

/// A person's `Command::Attack` on a regiment far beyond its control range.
#[test]
#[ignore]
fn real_map_attack_command() {
    let (mut w, c) = berlin_scene(30.0);
    let d = w.data.clone();
    let id = |n: &str| d.find(n).unwrap_or_else(|| panic!("{n}"));
    let a = aa_play::production::spawn_organization(&mut w, id("regiment_rus_infantry"), 0, c - Vec2::new(60.0, 0.0), 0.0, None);
    let b = aa_play::production::spawn_organization(&mut w, id("regiment_ger_infantry"), 1, c + Vec2::new(60.0, 0.0), std::f32::consts::PI, None);
    let mut f = Fight::default();
    watch(&mut w, &mut f, 5.0, |_| false);
    let target = w.orgs[b as usize].members[0];
    w.step(vec![aa_play::Command::Attack { kingdom: 0, units: vec![aa_play::Unit::Org(a)], target }]);
    let start = combat::org_state(&w, a).unwrap().position;
    watch(&mut w, &mut f, 5.0, |_| false);
    let walked = combat::org_state(&w, a).unwrap().position.distance(start);
    eprintln!("--- attack command: walked {walked} m in 5 s, order {:?}", w.orgs[a as usize].order);
    assert!(walked > 2.0, "the attacker walks");
    watch(&mut w, &mut f, 240.0, |w| !w.orgs[a as usize].alive || !w.orgs[b as usize].alive);
    show(&w, &f, &[a, b]);
    assert!(f.first_shot.is_some(), "the attack shoots");
    assert!(f.deaths > 0, "men die");
}

/// Real infantry at grenade range: the grenade (ability 1, area 2 m) splashes,
/// experience promotes (AAInfantryExperienceComponent: 150 xp, HEALTH x1.05),
/// and the promoted regiment hits harder.
#[test]
#[ignore]
fn real_map_grenades_and_promotion() {
    let (mut w, c) = berlin_scene(30.0);
    let d = w.data.clone();
    let id = |n: &str| d.find(n).unwrap_or_else(|| panic!("{n}"));
    let a = aa_play::production::spawn_organization(&mut w, id("regiment_rus_infantry"), 0, c - Vec2::new(8.0, 0.0), 0.0, None);
    let b = aa_play::production::spawn_organization(&mut w, id("regiment_ger_infantry"), 1, c + Vec2::new(8.0, 0.0), std::f32::consts::PI, None);
    w.orgs[a as usize].experience = 149.0;
    w.step(Vec::new());
    let base = w.combat.actor(w.orgs[a as usize].members[0]).unwrap().max_health;
    let (mut grenades, mut splashes, mut promoted) = (0, 0, None);
    let mut rifle_before = Vec::new();
    let mut rifle_after = Vec::new();
    for _ in 0..(60.0 / DT) as u32 {
        w.step(Vec::new());
        let events = w.events.clone();
        for e in &events {
            match *e {
                Event::Promoted { org, level } if org == a => promoted = Some((level, w.time)),
                Event::Fired { shooter, weapon, .. } => {
                    let name = w.data.def(w.actors[shooter as usize].def).id.clone();
                    if weapon == 1 && name.ends_with("infantry") {
                        grenades += 1;
                    }
                }
                _ => {}
            }
        }
        // A tick's hits on one kingdom from rifles of the other.
        for win in events.windows(2) {
            if let [Event::Fired { shooter, weapon: 0, .. }, Event::Hit { target, damage }] = win
                && w.actors[*shooter as usize].kingdom == Some(0)
                && w.actors[*target as usize].kingdom == Some(1)
                && w.data.def(w.actors[*shooter as usize].def).id == "rus_infantry"
            {
                if promoted.is_some() { rifle_after.push(*damage) } else { rifle_before.push(*damage) }
            }
            if let [Event::Hit { .. }, Event::Hit { .. }] = win {
                splashes += 1;
            }
        }
        if !w.orgs[a as usize].alive || !w.orgs[b as usize].alive {
            break;
        }
    }
    eprintln!("grenades {grenades}, double hits {splashes}, promoted {promoted:?}");
    eprintln!("rifle hits before {rifle_before:?}");
    eprintln!("rifle hits after {rifle_after:?}");
    let (level, _) = promoted.expect("the regiment is promoted");
    assert_eq!(level, 1);
    assert!(grenades > 0, "riflemen throw grenades at 9 m");
    if w.orgs[a as usize].alive {
        let now = w.combat.actor(w.orgs[a as usize].members[0]).unwrap().max_health;
        eprintln!("max health {base} -> {now}");
        assert!(now > base, "a promotion raises the maximum health");
    }
}

/// A real regiment breaks: it routs away, the rout ends, it rests and regains
/// morale, stands together again, and obeys and fights again.
#[test]
#[ignore]
fn real_map_rout_and_regroup() {
    let (mut w, c) = berlin_scene(30.0);
    let d = w.data.clone();
    let id = |n: &str| d.find(n).unwrap_or_else(|| panic!("{n}"));
    let a = aa_play::production::spawn_organization(&mut w, id("regiment_rus_infantry"), 0, c - Vec2::new(9.0, 0.0), 0.0, None);
    let b = aa_play::production::spawn_organization(&mut w, id("regiment_ger_infantry"), 1, c + Vec2::new(9.0, 0.0), std::f32::consts::PI, None);
    w.step(Vec::new());
    w.orgs[a as usize].morale = combat::org_state(&w, a).unwrap().breakpoint + 1.0;
    let mut f = Fight::default();
    let mut routed = None;
    for _ in 0..(40.0 / DT) as u32 {
        w.step(Vec::new());
        if w.events.contains(&Event::Routed { org: a }) {
            routed = Some(w.time);
            break;
        }
    }
    let routed = routed.expect("the regiment routs");
    let from = combat::org_state(&w, a).unwrap().position;
    eprintln!("routed at {routed:.1}s from {from}, morale {:.1}, order {:?}", w.orgs[a as usize].morale, w.orgs[a as usize].order);
    assert!(matches!(w.orgs[a as usize].order, Order::Move { .. }), "it runs");
    assert!(w.orgs[a as usize].members.iter().all(|&m| w.actors[m as usize].combat.target.is_none()), "it stops shooting");
    // The enemy goes away with a retreat (mind_retreating is PASSIVE: a plain move
    // would turn on the regiment in its ZOC), so the regiment can rest.
    w.step(vec![aa_play::Command::Order { kingdom: 1, units: vec![aa_play::Unit::Org(b)], order: aa_play::orders::ActorOrder::Retreat { target: None, at: c + Vec2::new(200.0, 0.0) } }]);
    let mut ended = None;
    let mut states = Vec::new();
    for _ in 0..(90.0 / DT) as u32 {
        w.step(Vec::new());
        let st = aa_play::morale::regiment_state(&w, a);
        if states.last().map(|(s, _, _)| *s) != Some(st) {
            states.push((st, w.time, w.orgs[a as usize].morale));
        }
        if ended.is_none() && combat::org_state(&w, a).unwrap().routing.is_none() {
            ended = Some(w.time);
        }
    }
    eprintln!("states {states:?}");
    let ended = ended.expect("the rout ends");
    eprintln!("rout over {:.1}s after it began, ran {} m", ended - routed, combat::org_state(&w, a).unwrap().position.distance(from));
    assert!(ended - routed <= 10.5, "RoutingTime");
    let s = combat::org_state(&w, a).unwrap();
    let g = &w.orgs[a as usize];
    let spread = g.members.iter().map(|&m| w.actors[m as usize].pos.truncate().distance(s.position)).fold(0.0f32, f32::max);
    eprintln!("after 90 s: morale {:.1}/{:.1}, members {}, spread {spread:.1} m, order {:?}, state {:?}", g.morale, s.morale_max, g.members.len(), g.order, aa_play::morale::regiment_state(&w, a));
    assert_eq!(g.order, Order::Idle);
    assert!(g.morale > 40.0, "morale came back: {}", g.morale);
    assert!(spread < 12.0, "together again: {spread}");
    // It obeys and fights again.
    let target = w.orgs[b as usize].members[0];
    let before = combat::org_state(&w, a).unwrap().position;
    w.step(vec![aa_play::Command::Attack { kingdom: 0, units: vec![aa_play::Unit::Org(a)], target }]);
    // The Kill state walks in the combat formation (0x57BFF0, regiment_aggressive SPEED_MAX
    // -0.25), a quarter slower: the 200 m the enemy has retreated take longer than 90 s.
    watch(&mut w, &mut f, 150.0, |w| !w.orgs[a as usize].alive || !w.orgs[b as usize].alive);
    eprintln!("--- attacks again, walked {}", combat::org_state(&w, a).unwrap().position.distance(before));
    show(&w, &f, &[a, b]);
    assert!(f.shots.keys().any(|k| k.starts_with("k0")), "it shoots again");
}

/// Medium tanks ordered onto a rifle regiment: guns kill, rifles scratch, the
/// infantry breaks.
#[test]
#[ignore]
fn real_map_tanks_against_infantry() {
    let (mut w, c) = berlin_scene(30.0);
    let d = w.data.clone();
    let id = |n: &str| d.find(n).unwrap_or_else(|| panic!("{n}"));
    let t = aa_play::production::spawn_organization(&mut w, id("regiment_rus_tank_medium"), 0, c - Vec2::new(30.0, 0.0), 0.0, None);
    let i = aa_play::production::spawn_organization(&mut w, id("regiment_ger_infantry"), 1, c + Vec2::new(30.0, 0.0), std::f32::consts::PI, None);
    let mut f = Fight::default();
    watch(&mut w, &mut f, 2.0, |_| false);
    let target = w.orgs[i as usize].members[0];
    w.step(vec![aa_play::Command::Attack { kingdom: 0, units: vec![aa_play::Unit::Org(t)], target }]);
    watch(&mut w, &mut f, 240.0, |w| !w.orgs[t as usize].alive || !w.orgs[i as usize].alive);
    eprintln!("--- tanks on infantry");
    show(&w, &f, &[t, i]);
    assert!(f.shots.keys().any(|k| k.contains("rus_tank")), "the guns fire");
    assert!(f.log.iter().any(|l| l.contains("routed") || l.contains("destroyed")), "the infantry breaks");
}

/// Column formation (`aggressive = false`): walking through the enemy's control
/// range it does not stop to fight (+8D off); the enemy does engage it.
#[test]
#[ignore]
fn real_map_column_keeps_walking() {
    let (mut w, c) = berlin_scene(30.0);
    let d = w.data.clone();
    let id = |n: &str| d.find(n).unwrap_or_else(|| panic!("{n}"));
    let a = aa_play::production::spawn_organization(&mut w, id("regiment_rus_infantry"), 0, c - Vec2::new(45.0, 0.0), 0.0, None);
    let b = aa_play::production::spawn_organization(&mut w, id("regiment_ger_infantry"), 1, c + Vec2::new(10.0, 12.0), std::f32::consts::PI, None);
    let column = w.data.def(w.orgs[a as usize].def).formations.iter().position(|f| f.id == "regiment_column").expect("column");
    w.step(vec![aa_play::Command::SetFormation { kingdom: 0, orgs: vec![a], formation: column }]);
    w.step(vec![aa_play::Command::Move { kingdom: 0, units: vec![aa_play::Unit::Org(a)], to: c + Vec2::new(45.0, 0.0), attack_move: false }]);
    let mut f = Fight::default();
    watch(&mut w, &mut f, 40.0, |_| false);
    eprintln!("--- column walking past");
    show(&w, &f, &[a, b]);
    assert!(combat::org_state(&w, a).unwrap().engage.is_empty(), "the column does not engage by itself");
    assert!(f.shots.keys().any(|k| k.starts_with("k1")), "the resting regiment shoots at it");
}

/// 150 real regiments fighting through `World::step` on an open map.
#[test]
#[ignore]
fn real_data_performance_150_regiments() {
    let data = real().expect("original data");
    let mut w = World::empty(data.clone(), Arc::new(open_map(256, 256, 4.0)), 3);
    w.kingdoms = vec![kingdom(0, 0), kingdom(1, 1)];
    let (r1, r2) = (data.find("regiment_rus_infantry").unwrap(), data.find("regiment_ger_infantry").unwrap());
    for side in 0..2u32 {
        for r in 0..75 {
            let x = 40.0 + (r % 15) as f32 * 40.0;
            let y = 40.0 + (r / 15) as f32 * 80.0 + if side == 0 { 0.0 } else { 16.0 };
            let (def, facing) = if side == 0 { (r1, std::f32::consts::FRAC_PI_2) } else { (r2, -std::f32::consts::FRAC_PI_2) };
            aa_play::production::spawn_organization(&mut w, def, side, Vec2::new(x, y), facing, None);
        }
    }
    let ticks = 960;
    let t = std::time::Instant::now();
    w.step(Vec::new());
    eprintln!("first tick {:?}", t.elapsed());
    let (mut worst, mut fired) = ((std::time::Duration::ZERO, 0), 0);
    let t0 = std::time::Instant::now();
    for i in 0..ticks {
        let t = std::time::Instant::now();
        w.step(Vec::new());
        if t.elapsed() > worst.0 {
            worst = (t.elapsed(), i);
        }
        fired += w.events.iter().filter(|e| matches!(e, Event::Fired { .. })).count();
    }
    let dead = w.actors.iter().filter(|a| !a.alive).count();
    eprintln!("150 real regiments: avg {:?} per tick, worst (time, tick) {worst:?}, {fired} shots in {ticks} ticks, {dead} dead", t0.elapsed() / ticks);
    assert!(fired > 1000, "they fight");
}

/// A person's selection of three regiments attacking one enemy (the group path
/// `0x4ED360` -> `0x4DF840`): all of them walk and fight.
#[test]
#[ignore]
fn real_map_group_attack() {
    let (mut w, c) = berlin_scene(30.0);
    let d = w.data.clone();
    let id = |n: &str| d.find(n).unwrap_or_else(|| panic!("{n}"));
    let a: Vec<OrgId> = (0..3).map(|i| aa_play::production::spawn_organization(&mut w, id("regiment_rus_infantry"), 0, c + Vec2::new(-60.0, -25.0 + 25.0 * i as f32), 0.0, None)).collect();
    let b = aa_play::production::spawn_organization(&mut w, id("regiment_ger_infantry"), 1, c + Vec2::new(25.0, 0.0), std::f32::consts::PI, None);
    let mut f = Fight::default();
    watch(&mut w, &mut f, 2.0, |_| false);
    let target = w.orgs[b as usize].members[0];
    let units = a.iter().map(|&o| aa_play::Unit::Org(o)).collect();
    w.step(vec![aa_play::Command::Attack { kingdom: 0, units, target }]);
    let start: Vec<Vec2> = a.iter().map(|&o| combat::org_state(&w, o).unwrap().position).collect();
    watch(&mut w, &mut f, 8.0, |_| false);
    for (i, &o) in a.iter().enumerate() {
        let walked = combat::org_state(&w, o).unwrap().position.distance(start[i]);
        eprintln!("regiment {o}: walked {walked} in 8 s, order {:?}", w.orgs[o as usize].order);
        assert!(walked > 5.0, "regiment {o} walks");
    }
    watch(&mut w, &mut f, 120.0, |w| !w.orgs[b as usize].alive);
    let mut all = a.clone();
    all.push(b);
    show(&w, &f, &all);
    assert!(f.deaths > 0);
    assert!(!w.orgs[b as usize].alive || f.log.iter().any(|l| l.contains("routed")), "three on one: the one breaks");
}

/// Attacking a building: the regiment walks to it and shoots it down, and the
/// building's ZOC or guns do what the data says.
#[test]
#[ignore]
fn real_map_attack_a_building() {
    let (mut w, _) = berlin_scene(30.0);
    let d = w.data.clone();
    let id = |n: &str| d.find(n).unwrap_or_else(|| panic!("{n}"));
    let corps = w.actors.iter().find(|a| a.kingdom == Some(1) && !d.def(a.def).weapons.is_empty() && d.def(a.def).class == Some(DefClass::Building)).map(|a| a.id);
    let any = w.actors.iter().find(|a| a.kingdom == Some(1) && d.def(a.def).class == Some(DefClass::Building)).map(|a| a.id).expect("a building of kingdom 1");
    let target = corps.unwrap_or(any);
    let tp = w.actors[target as usize].pos.truncate();
    eprintln!("target {} at {tp} health {}, weapons {}", d.def(w.actors[target as usize].def).id, w.actors[target as usize].health, d.def(w.actors[target as usize].def).weapons.len());
    let a = aa_play::production::spawn_organization(&mut w, id("regiment_rus_tank_medium"), 0, tp + Vec2::new(-70.0, 0.0), 0.0, None);
    let mut f = Fight::default();
    watch(&mut w, &mut f, 2.0, |_| false);
    let h0 = w.actors[target as usize].health;
    w.step(vec![aa_play::Command::Attack { kingdom: 0, units: vec![aa_play::Unit::Org(a)], target }]);
    watch(&mut w, &mut f, 120.0, |w| !w.orgs[a as usize].alive || !w.actors[target as usize].alive);
    show(&w, &f, &[a]);
    eprintln!("building health {h0} -> {}, alive {}, owner {:?}", w.actors[target as usize].health, w.actors[target as usize].alive, w.actors[target as usize].kingdom);
    assert!(f.shots.keys().any(|k| k.starts_with("k0")), "the regiment shoots the building");
    assert!(w.actors[target as usize].health < h0, "the building takes damage");
}

/// Fog on a real map: an enemy regiment beyond sight is hidden; once two regiments
/// fight, each sees the other, and an artillery regiment shelling from beyond the
/// target's sight is shown to it by its fire (reverse RZOD) only as the data says.
#[test]
#[ignore]
fn real_map_fog_in_a_fight() {
    let (mut w, c) = berlin_scene(30.0);
    let d = w.data.clone();
    let id = |n: &str| d.find(n).unwrap_or_else(|| panic!("{n}"));
    aa_play::fog::set_mode(&mut w, aa_play::fog::FogMode::Full);
    let a = aa_play::production::spawn_organization(&mut w, id("regiment_rus_infantry"), 0, c - Vec2::new(60.0, 0.0), 0.0, None);
    let b = aa_play::production::spawn_organization(&mut w, id("regiment_ger_infantry"), 1, c + Vec2::new(60.0, 0.0), std::f32::consts::PI, None);
    for _ in 0..48 {
        w.step(Vec::new());
    }
    let seen = |w: &World, o: OrgId, by: u32| w.orgs[o as usize].members.iter().filter(|&&m| aa_play::fog::actor_visible(w, m, Some(by))).count();
    eprintln!("120 m apart: a sees {} of b, b sees {} of a", seen(&w, b, 0), seen(&w, a, 1));
    assert_eq!(seen(&w, b, 0), 0, "the enemy is hidden");
    let target = w.orgs[b as usize].members[0];
    w.step(vec![aa_play::Command::Attack { kingdom: 0, units: vec![aa_play::Unit::Org(a)], target }]);
    let mut first = None;
    let mut seen_at = None;
    for _ in 0..(120.0 / DT) as u32 {
        w.step(Vec::new());
        if seen_at.is_none() && seen(&w, b, 0) > 0 {
            seen_at = Some((w.time, combat::org_state(&w, a).unwrap().position.distance(combat::org_state(&w, b).unwrap().position)));
        }
        if first.is_none() && w.events.iter().any(|e| matches!(e, Event::Fired { .. })) {
            first = Some((w.time, seen(&w, b, 0), seen(&w, a, 1)));
            break;
        }
    }
    eprintln!("a first sees b at (time, distance) {seen_at:?}; first shot (time, a sees of b, b sees of a) {first:?}");
    let (_, sa, sb) = first.expect("they fight");
    assert!(sa > 0 && sb > 0, "fighters see each other");
}

/// A plain Move that meets a weak enemy stops to fight (`0x524A00` takes over from
/// the move, `0x52A810(3)`) and goes on afterwards.
#[test]
#[ignore]
fn real_map_move_goes_on_after_the_fight() {
    let (mut w, c) = berlin_scene(30.0);
    let d = w.data.clone();
    let id = |n: &str| d.find(n).unwrap_or_else(|| panic!("{n}"));
    let a = aa_play::production::spawn_organization(&mut w, id("regiment_rus_tank_medium"), 0, c - Vec2::new(50.0, 0.0), 0.0, None);
    let b = aa_play::production::spawn_organization(&mut w, id("regiment_ger_engineer"), 1, c, std::f32::consts::PI, None);
    let to = c + Vec2::new(60.0, 0.0);
    let mut f = Fight::default();
    watch(&mut w, &mut f, 1.0, |_| false);
    w.step(vec![aa_play::Command::Move { kingdom: 0, units: vec![aa_play::Unit::Org(a)], to, attack_move: false }]);
    let mut stopped_for_it = false;
    watch(&mut w, &mut f, 200.0, |w| {
        stopped_for_it |= matches!(w.orgs[a as usize].order, Order::Attack { .. });
        w.orgs[a as usize].order == Order::Idle
    });
    eprintln!("--- move through a weak enemy, stopped for it: {stopped_for_it}");
    show(&w, &f, &[a, b]);
    // The order's point is taken on this map, so a walk ends short of it wherever the path
    // ends: the fight's regiment ends where the same regiment does with nobody in the way.
    let at = w.orgs[a as usize].anchor;
    let (mut alone, _) = berlin_scene(30.0);
    let lone = aa_play::production::spawn_organization(&mut alone, id("regiment_rus_tank_medium"), 0, c - Vec2::new(50.0, 0.0), 0.0, None);
    alone.step(vec![aa_play::Command::Move { kingdom: 0, units: vec![aa_play::Unit::Org(lone)], to, attack_move: false }]);
    for _ in 0..(200.0 / DT) as u32 {
        alone.step(Vec::new());
        if alone.orgs[lone as usize].order == Order::Idle {
            break;
        }
    }
    let end = alone.orgs[lone as usize].anchor;
    eprintln!("ended at {at}, the walk alone at {end}, wanted {to}");
    assert!(stopped_for_it, "the regiment turns on what enters its ZOC");
    assert!(w.orgs[a as usize].order == Order::Idle && at.distance(end) < 1.0, "the move goes on afterwards: {at}");
}

/// A plain Move ordered in the middle of a fight leaves the engage state
/// (`0x527E30` -> `0x51F040`); the enemy still in the ZOC comes back on the list as
/// the ZOC moves over it. A retreat (`mind_retreating`, PASSIVE) leaves for good.
#[test]
#[ignore]
fn real_map_leaving_a_fight() {
    for retreat in [false, true] {
        let (mut w, c) = berlin_scene(30.0);
        let d = w.data.clone();
        let id = |n: &str| d.find(n).unwrap_or_else(|| panic!("{n}"));
        let a = aa_play::production::spawn_organization(&mut w, id("regiment_rus_infantry"), 0, c - Vec2::new(8.0, 0.0), 0.0, None);
        let b = aa_play::production::spawn_organization(&mut w, id("regiment_ger_infantry"), 1, c + Vec2::new(8.0, 0.0), std::f32::consts::PI, None);
        let mut f = Fight::default();
        watch(&mut w, &mut f, 5.0, |_| false);
        assert!(f.first_shot.is_some());
        let start = combat::org_state(&w, a).unwrap().position;
        let to = c - Vec2::new(150.0, 0.0);
        let unit = vec![aa_play::Unit::Org(a)];
        let command = if retreat {
            aa_play::Command::Order { kingdom: 0, units: unit, order: aa_play::orders::ActorOrder::Retreat { target: None, at: to } }
        } else {
            aa_play::Command::Move { kingdom: 0, units: unit, to, attack_move: false }
        };
        w.step(vec![command]);
        watch(&mut w, &mut f, 20.0, |_| false);
        let s = combat::org_state(&w, a).unwrap();
        eprintln!("--- {} in a fight: walked {} m in 20 s, order {:?}, state {:?}, engage {:?}", if retreat { "retreat" } else { "plain move" }, s.position.distance(start), w.orgs[a as usize].order, aa_play::morale::regiment_state(&w, a), s.engage);
        show(&w, &f, &[a, b]);
    }
}

/// A person's attack on a city: the regiment walks to it and takes it by presence.
#[test]
#[ignore]
fn real_map_attack_on_a_city() {
    let (mut w, _) = berlin_scene(30.0);
    let d = w.data.clone();
    let id = |n: &str| d.find(n).unwrap_or_else(|| panic!("{n}"));
    let hostile = |w: &World, a: u32| aa_play::orders::hostile_to(w, 0, a);
    let list: Vec<(u32, String, Option<u32>, Vec2, bool)> = w.actors.iter().filter(|a| !a.removed && d.def(a.def).city).map(|a| (a.id, d.def(a.def).id.clone(), a.kingdom, a.pos.truncate(), hostile(&w, a.id))).collect();
    eprintln!("cities {list:?}");
    let target = list.iter().find(|c| c.4).unwrap_or_else(|| panic!("a hostile city")).clone();
    let a = aa_play::production::spawn_organization(&mut w, id("regiment_rus_infantry"), 0, target.3 + Vec2::new(-45.0, -5.0), 0.0, None);
    let mut f = Fight::default();
    watch(&mut w, &mut f, 1.0, |_| false);
    w.step(vec![aa_play::Command::Attack { kingdom: 0, units: vec![aa_play::Unit::Org(a)], target: target.0 }]);
    let mut taken = None;
    for _ in 0..(300.0 / DT) as u32 {
        w.step(Vec::new());
        if taken.is_none() && w.actors[target.0 as usize].kingdom == Some(0) {
            taken = Some(w.time);
            break;
        }
    }
    let s = combat::org_state(&w, a).unwrap();
    eprintln!("--- city {} taken at {taken:?}; regiment at {} ({} m from it), order {:?}, state {:?}, members {}", target.1, s.position, s.position.distance(target.3), w.orgs[a as usize].order, aa_play::morale::regiment_state(&w, a), w.orgs[a as usize].members.len());
    assert!(taken.is_some(), "the city changes hands");
}

/// Artillery's kill is the bombard state (`0x527FB0`): it shells from where it can
/// reach, and the shelled regiment takes the fire.
#[test]
#[ignore]
fn real_map_artillery_shells() {
    let (mut w, c) = berlin_scene(30.0);
    let d = w.data.clone();
    let id = |n: &str| d.find(n).unwrap_or_else(|| panic!("{n}"));
    let a = aa_play::production::spawn_organization(&mut w, id("regiment_rus_tank_artillery"), 0, c - Vec2::new(40.0, 0.0), 0.0, None);
    let b = aa_play::production::spawn_organization(&mut w, id("regiment_ger_infantry"), 1, c + Vec2::new(40.0, 0.0), std::f32::consts::PI, None);
    let indirect = aa_play::orders::guess::bombard_layout(&w, a);
    let mut f = Fight::default();
    watch(&mut w, &mut f, 2.0, |_| false);
    let target = w.orgs[b as usize].members[0];
    let from = combat::org_state(&w, a).unwrap().position;
    w.step(vec![aa_play::Command::Bombard { kingdom: 0, units: vec![aa_play::Unit::Org(a)], target: aa_play::AbilityTarget::Actor(target) }]);
    watch(&mut w, &mut f, 60.0, |w| !w.orgs[a as usize].alive || !w.orgs[b as usize].alive);
    eprintln!("--- artillery: bombard layout {indirect}, walked {}, state {:?}", combat::org_state(&w, a).unwrap().position.distance(from), aa_play::morale::regiment_state(&w, a));
    show(&w, &f, &[a, b]);
    assert!(f.shots.keys().any(|k| k.starts_with("k0")), "the artillery fires");
}

/// `0x51D1C0` (stop: the sleep state) and the kill at a location (`0x520440`) replace
/// the engage state (`0x52A420`), whose exit `0x520FE0` empties the list: the regiment
/// leaves a target that does not move and does not shoot.
#[test]
#[ignore]
fn real_map_stop_leaves_a_harmless_target() {
    let (mut w, c) = berlin_scene(30.0);
    let d = w.data.clone();
    let id = |n: &str| d.find(n).unwrap_or_else(|| panic!("{n}"));
    let building = id("ger_depot_supply");
    eprintln!("target {}", d.def(building).id);
    let t = aa_play::production::spawn_single(&mut w, building, Some(1), c + Vec2::new(25.0, 0.0), 0.0);
    let a = aa_play::production::spawn_organization(&mut w, id("regiment_rus_infantry"), 0, c - Vec2::new(25.0, 0.0), 0.0, None);
    let own = vec![aa_play::Unit::Org(a)];
    let mut f = Fight::default();
    watch(&mut w, &mut f, 1.0, |_| false);
    w.step(vec![aa_play::Command::Attack { kingdom: 0, units: own.clone(), target: t }]);
    watch(&mut w, &mut f, 90.0, |w| w.events.iter().any(|e| matches!(e, Event::Fired { .. })));
    eprintln!("first shot at the building {:?}, order {:?}, health {}", f.first_shot, w.orgs[a as usize].order, w.actors[t as usize].health);
    assert!(f.first_shot.is_some(), "the regiment shoots the building");
    watch(&mut w, &mut f, 3.0, |_| false);
    w.step(vec![aa_play::Command::Stop { kingdom: 0, units: own.clone() }]);
    w.step(Vec::new());
    let mut after = Fight::default();
    watch(&mut w, &mut after, 10.0, |_| false);
    eprintln!("after stop: {:?} order {:?} engage {:?}", after.shots, w.orgs[a as usize].order, combat::org_state(&w, a).unwrap().engage);
    assert!(after.shots.is_empty(), "stop ends the shooting");
    assert_eq!(w.orgs[a as usize].order, Order::Idle);
    // The kill at a location leaves it too, and a new attack from outside the ZOC
    // brings it back in (a thing already in the ZOC is not entered again: only
    // 0x57AB70 and 0x579490 fill the list).
    w.step(vec![aa_play::Command::Attack { kingdom: 0, units: own.clone(), target: t }]);
    let mut again = Fight::default();
    watch(&mut w, &mut again, 2.0, |_| false);
    let from = combat::org_state(&w, a).unwrap().position;
    w.step(vec![aa_play::Command::Move { kingdom: 0, units: own.clone(), to: c - Vec2::new(30.0, 0.0), attack_move: true }]);
    watch(&mut w, &mut again, 30.0, |w| w.orgs[a as usize].order == Order::Idle);
    let walked = combat::org_state(&w, a).unwrap().position.distance(from);
    eprintln!("kill at a location: walked {walked}, order {:?}", w.orgs[a as usize].order);
    w.step(vec![aa_play::Command::Attack { kingdom: 0, units: own, target: t }]);
    let mut third = Fight::default();
    watch(&mut w, &mut third, 40.0, |w| w.events.iter().any(|e| matches!(e, Event::Fired { .. })));
    eprintln!("attacks again from outside: first shot {:?}, order {:?}", third.first_shot, w.orgs[a as usize].order);
    assert!(third.first_shot.is_some(), "it attacks again");
    assert!(walked > 5.0, "the order is obeyed");
}
