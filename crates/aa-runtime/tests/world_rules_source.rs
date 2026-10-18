use aa_runtime::{load_world_rules::Owner,catalog::Catalog};
use aa_mapworld::WorldInventory;

#[test]
#[ignore="requires actual AA_GAME_ROOT source and sealed startup receipt"]
fn actual_owner_keeps_all_source_defaults_saved_order_and_current_mutations() {
    use aa_rules::sources::{SourceLimits,V14Sources};
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits{max_member_bytes:u64::MAX}).unwrap();
    let actors=aa_rules::actors::ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog=Catalog::new(&actors);
    catalog.settings=Some(aa_runtime::settings::Settings::load_v14(&mut files,aa_rules::tgi::Limits::default()).unwrap());
    let proof=aa_runtime::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
    aa_runtime::combat_source_completion::install(&catalog,&mut files,proof.context(),proof.csv(),proof.globals()).unwrap();
    aa_runtime::combat_source_completion::compute(&catalog).unwrap();
    catalog.native_load_mode=Some(1);
    let native:serde_json::Value=serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-mission-start/native-world-rules.json")).unwrap();
    let mut captures=Vec::new();
    for name in ["Campaign/tutorial02","DeveloperMaps/(3v3)Firefight","(2v2)Berlin"] {
        let map=aa_mapworld::MapArchive::from_bytes(std::fs::read(game.join(format!("reconstruction/assets/original/Data/Maps/{name}.RMP"))).unwrap()).unwrap();
        let inventory=WorldInventory::decode(&map,u64::MAX).unwrap();
        let owner=Owner::load(&catalog,&inventory).unwrap();
        assert_eq!(owner.record("FogOfWar").unwrap().default_value(),1);
        assert_eq!(owner.record("ResourceStockpileDefault").unwrap().default_value(),500f32.to_bits());
        assert_eq!(owner.record("StartingExperience").unwrap().default_value(),200f32.to_bits());
        assert_eq!(owner.record("CityVictoryRatio").unwrap().default_value(),0);
        assert_eq!(owner.record("Sun").unwrap().enum_ids(0).unwrap(),"sun_default");
        owner.validate_in_bank(&catalog,&catalog.definition_bank.borrow()).unwrap();
        let defaults:Vec<_>=owner.records().map(|v|serde_json::json!({"ids":v.ids(),"kind":v.kind(),
            "default":v.default_value(),"editable":u8::from(v.always_editable()),"enum_ids":v.enum_values(),"value":v.value(),"locked":v.locked()})).collect();
        let section=inventory.opaque_initialization.iter().find(|v|v.name=="RULES").unwrap();
        for pc in ["Single24","Double53","Extended64"] {
            let row=native["cases"].as_array().unwrap().iter()
                .find(|r|r["input"]["actual_map"]==name && r["precision"]==pc).unwrap();
            assert_eq!(row["input"]["rules_sha256"],section.source.sha256);
            let projection:Vec<_>=owner.records().map(|r|serde_json::json!({"ids":r.ids(),"locked":r.locked(),"value":r.value()})).collect();
            assert_eq!(serde_json::json!(projection),row["output"]["current"],"{name} {pc}");
            assert_eq!(row["input"]["definitions"],serde_json::json!(defaults),"actual source input changed without refreshing bounded native oracle");
        }
        captures.push(serde_json::json!({"map":name,"rules_sha256":section.source.sha256,"records":defaults}));

    }
    if let Some(path)=std::env::var_os("AA_WORLD_RULES_CAPTURE") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"cases":captures})).unwrap()).unwrap();
    }
}
