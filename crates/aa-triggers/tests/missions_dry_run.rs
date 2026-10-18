//! Integration smoke test (NOT an oracle): every element instance of the 26
//! campaign missions is evaluated through the ported dispatch against an empty
//! recording world (every lookup resolves to null).  It proves the decoded
//! parameters (localized texts bound through MAPSTRINGS) are accepted by the
//! ported selectors without any error.
mod fixture;
use aa_numeric::Precision;
use aa_triggers::{
    decode::{Localization, SavedManager},
    elements,
    manager::Manager,
    mapstrings::MapStrings,
    program::CoreProgram,
};
use fixture::FixtureWorld;
use std::collections::BTreeMap;

#[test]
fn campaign_elements_evaluate_against_an_empty_world() {
    let index: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase8-triggers/chunks/index.json"
    ))
    .unwrap();
    let mut errors: BTreeMap<String, u32> = BTreeMap::new();
    let mut evaluated = 0u32;
    for entry in index.as_array().unwrap() {
        if entry["campaign"] != true {
            continue;
        }
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../research/phase8-triggers/chunks")
            .join(entry["file"].as_str().unwrap());
        let mut saved = SavedManager::decode(&std::fs::read(path).unwrap(), Localization::External, 64 << 20).unwrap();
        let text = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/phase8-triggers/chunks")
                .join(entry["mapstrings_file"].as_str().unwrap()),
        )
        .unwrap();
        MapStrings::parse(&text, "English").unwrap().bind(&mut saved).unwrap();
        let definitions = saved.triggers.clone();
        let mut manager = Manager::from_saved(saved);
        let c = serde_json::json!({"time_bits": 0x4120_0000u32, "map_difficulty": 1});
        let mut world = FixtureWorld::build(&c);
        let mut core = CoreProgram::new(0x4120_0000, 0, Precision::Double53);
        for (ordinal, t) in definitions.iter().enumerate() {
            for e in t.events.iter().chain(&t.conditions).chain(&t.actions) {
                if aa_triggers::world_program::SUPPORTED.contains(&e.opcode) {
                    continue; // proven by phase7 with control/property services
                }
                let result = match core.evaluate_core(&mut manager, ordinal, e) {
                    Ok(Some(v)) => Ok(Some(v)),
                    Ok(None) => elements::evaluate(&mut core, &mut manager, e, &mut world),
                    Err(err) => Err(err),
                };
                evaluated += 1;
                match result {
                    Ok(Some(_)) => {}
                    Ok(None) => panic!("opcode {} not dispatched", e.opcode),
                    Err(err) => {
                        *errors.entry(format!("{} trigger {} {}: {err}", entry["map"], t.id, e.name)).or_default() += 1;
                    }
                }
            }
        }
    }
    println!("evaluated {evaluated} element instances; errors: {errors:#?}");
    assert!(errors.is_empty(), "{errors:#?}");
    assert!(evaluated > 3500);
}
