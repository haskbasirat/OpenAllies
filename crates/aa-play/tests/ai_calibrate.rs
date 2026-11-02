//! Probe for the opponent owner (ignored): the definitions behind the
//! original SAI goal CSV values.
use aa_play::GameData;
use std::path::PathBuf;

#[test]
#[ignore]
fn dump_recruit_inputs() {
    let root = PathBuf::from(std::env::var("AA_GAME_ROOT").unwrap_or_else(|_| r"<game root>".into()));
    let data = GameData::load(&root).unwrap();
    let rules = aa_play::combat::Rules::from_data(&data);
    let cv = aa_play::combat::combat_values(&data, &rules);
    let res: Vec<String> = data.resources.iter().map(|r| r.id.clone()).collect();
    eprintln!("resources {res:?}");
    for id in ["rus_truck_oil", "rus_depot_oil", "rus_truck_ammo", "rus_depot_ammo", "rus_truck_supply", "rus_depot_supply", "rus_truck_infantry", "rus_hq_infantry", "regiment_rus_hq_infantry", "rus_truck_mechanized", "rus_hq_mechanized", "regiment_rus_hq_mechanized", "rus_truck_artillery", "rus_brigade_artillery", "rus_truck_motor", "rus_motor_pool", "rus_truck_engineer", "rus_brigade_engineer", "regiment_rus_infantry", "rus_infantry", "regiment_rus_infantry_mortar", "rus_corps", "western_europe_city_center01_small"] {
        let Some(d) = data.find(id) else { eprintln!("{id}: missing"); continue };
        let def = data.def(d);
        eprintln!(
            "{id}: class {:?} cost {:?} upkeep {:?} prod {:?} ul+ {} ul- {} cv {:.1} health {} slots {:?} unpacks {:?} props {:?}",
            def.class, def.cost_all, def.upkeep_all, def.production_all, def.unit_limit_provided, def.unit_limit_consumed, cv[d as usize], def.health,
            def.slots.iter().filter(|(s, _)| *s != u32::MAX).map(|(s, _)| data.def(*s).id.clone()).collect::<Vec<_>>(),
            def.unpacks_to.map(|u| data.def(u).id.clone()),
            def.properties
        );
    }
}

/// The ego ActorValue ComponentValue lists, the Bombardement and AirAttacks
/// settings, and the 7BB5E0 target flags of the city buildings.
#[test]
#[ignore]
fn dump_ego_component_values() {
    let root = PathBuf::from(std::env::var("AA_GAME_ROOT").unwrap_or_else(|_| r"<game root>".into()));
    let sai = aa_play::ai::AiData::load(&root).unwrap();
    for e in sai.egos.iter().filter(|e| e.ids.starts_with("chuikov") || e.ids.starts_with("rommel")) {
        let named: Vec<(String, f32, bool)> = e
            .component_values
            .iter()
            .enumerate()
            .filter(|(_, (v, _))| *v != 0.0)
            .map(|(i, (v, n))| (aa_play::ai::data::COMPONENT_TYPES[i].to_string(), *v, *n))
            .collect();
        eprintln!("{}: components {named:?} bombard {} / {} air {:?} flares {:?}", e.ids, e.bombard_unengaged_target_multiplier, e.bombard_balancing_constant, e.air, e.flares);
    }
    let data = GameData::load(&root).unwrap();
    for id in ["rus_halftrack_artillery", "rus_halftrack", "rus_infantry_mortar", "rus_mortar", "rus_artillery", "rus_howitzer"] {
        let Some(d) = data.find(id) else { eprintln!("{id}: missing"); continue };
        let def = data.def(d);
        let w: Vec<(String, u32, f32, f32)> = def.weapons.iter().map(|w| (w.name.clone(), w.flags, w.range, w.min_range)).collect();
        eprintln!("{id}: weapons {w:?}");
    }
    for def in data.defs.iter().filter(|d| d.city || d.id.contains("bank") || d.id.contains("refinery") || d.id.contains("farm") || d.id.contains("city")) {
        eprintln!("{}: city {} zone {} invulnerable {} targetable {} destructible {} components {:#x}", def.id, def.city, def.city_zone_radius, def.invulnerable, def.targetable, def.destructible, def.components);
    }
}
