//! Probe for the opponent owner (ignored): the definitions behind the
//! original SAI goal CSV values.
use aa_play::GameData;
use std::path::PathBuf;

#[test]
#[ignore]
fn dump_recruit_inputs() {
    let root = PathBuf::from(std::env::var("AA_GAME_ROOT").unwrap_or_else(|_| r"E:\Games\Axis and Alies".into()));
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
