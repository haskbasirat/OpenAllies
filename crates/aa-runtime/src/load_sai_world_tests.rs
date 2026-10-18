use super::*;
use aa_navigation::startup::{self, Object, World};

struct StopWorld<'a> { collections: Collections<'a>, late: bool }
impl<'a> World<Fields<'a>> for StopWorld<'a> {
    fn clear_list78(&mut self, _: &mut Fields<'a>) -> startup::Result<()> { self.collections.commands.clear(); Ok(()) }
    fn clear_list80(&mut self, _: &mut Fields<'a>) -> startup::Result<()> { self.collections.general_powers.clear(); Ok(()) }
    fn global_revision(&self) -> u32 { 19 }
    fn prepare_topology(&mut self, _: &mut Fields<'a>) -> startup::Result<()> {
        if self.late { Ok(()) } else { Err(startup::Error("injected preparation failure".into())) }
    }
    fn kingdom_count(&self) -> usize { 0 }
    fn kingdom_at(&self, _: usize) -> Option<Object> { unreachable!() }
    fn register_kingdom(&mut self, _: Object, _: &mut Fields<'a>) -> startup::Result<()> { unreachable!() }
    fn ai_slot_count(&self) -> usize { self.collections.players.len() }
    fn ai_slot_at(&self, _: usize) -> Option<Object> { unreachable!() }
    fn start_ai(&mut self, _: Object, _: &mut Fields<'a>) -> startup::Result<()> { unreachable!() }
    fn registry_count(&self) -> i32 { 1 }
    fn next_registered_after(&self, _: u32) -> startup::Result<Option<Object>> { Ok(Some(Object(7))) }
    fn actor_id(&self, _: Object) -> u32 { 7 }
    fn actor_b4(&mut self, _: Object, _: &mut Fields<'a>) -> startup::Result<bool> { Err(startup::Error("injected registration failure".into())) }
    fn actor_flags8(&self, _: Object) -> u32 { unreachable!() }
    fn register_special_actor(&mut self, _: Object, _: &mut Fields<'a>) -> startup::Result<()> { unreachable!() }
    fn actor_b0(&mut self, _: Object, _: &mut Fields<'a>) -> startup::Result<bool> { unreachable!() }
    fn seen_record(&mut self, _: Object, _: Object, _: &mut Fields<'a>) -> startup::Result<Option<Object>> { unreachable!() }
    fn seen_context(&self, _: Object) -> Object { unreachable!() }
    fn context_eligible(&mut self, _: Object, _: &mut Fields<'a>) -> startup::Result<bool> { unreachable!() }
    fn context_words(&self, _: Object) -> [u32; 2] { unreachable!() }
    fn seen_position(&self, _: Object) -> [u32; 2] { unreachable!() }
    fn region_at(&mut self, _: [u32; 2], _: &mut Fields<'a>) -> startup::Result<Option<u32>> { unreachable!() }
    fn region_kingdom(&mut self, _: u32, _: Object, _: &mut Fields<'a>) -> startup::Result<Option<Object>> { unreachable!() }
    fn register_seen(&mut self, _: Object, _: Object, _: &mut Fields<'a>) -> startup::Result<()> { unreachable!() }
}

#[test]
fn same_constructed_manager_keeps_actual_startup_error_prefix() {
    // Explicit failure-injection fixtures; no actual-world readiness assertion.
    for late in [false, true] {
        let mut manager = Sai::new(0x12345678);
        manager.general_powers.push(23);
        manager.word_6c = 123;
        manager.word_70 = 456;
        let rng = manager.rng.state;
        assert!(initialize_manager(&mut manager, |collections| StopWorld {collections, late}).is_err());
        assert_eq!(manager.revision, Some(19));
        assert_eq!(manager.ready, late, "native writes ready before registration");
        assert!(manager.general_powers.is_empty());
        assert_eq!([manager.word_6c, manager.word_70], [123, 456], "final stores were not reached");
        assert_eq!(manager.rng.state, rng);
    }
}

#[test]
#[ignore="requires AA_GAME_ROOT original/v14 sources"]
fn actual_tutorial_map_source_topology_and_preparation_all_precisions() {
    use crate::{battle_terrain, catalog::Catalog, combat_source_completion as completion,
        combat_source_receipt::VerifiedStartupReceipt, settings::Settings, load_spatial::LoadSpatial};
    use aa_rules::{actors::ActorCatalog,resources::ResourceCatalog,sources::{SourceLimits,V14Sources},tgi::Limits};
    use aa_numeric::Precision;
    use serde_json::{json,Value};
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {max_member_bytes:64*1024*1024}).unwrap();
    let actors=ActorCatalog::load_v14(&mut sources).unwrap();
    let settings=Settings::load_v14(&mut sources,Limits::default()).unwrap();
    let resources=std::sync::Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let mut catalog=Catalog::new(&actors);catalog.settings=Some(settings.clone());catalog.native_load_mode=Some(1);
    catalog.bind_resource_catalog(resources).unwrap();
    let receipt=VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut sources).unwrap();
    completion::install(&catalog,&mut sources,receipt.context(),receipt.csv(),receipt.globals()).unwrap();
    completion::compute(&catalog).unwrap();
    let archive=aa_mapworld::MapArchive::from_bytes(std::fs::read(game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP")).unwrap()).unwrap();
    let header=aa_mapworld::header::MapHeader::decode(&archive.decode_chunk("MAP_HEADER",u64::MAX).unwrap()).unwrap();
    let native:Value=serde_json::from_str(include_str!("../../../research/phase7-navigation/native-topology.json")).unwrap();
    let prior=&native["maps"]["Maps/Campaign/tutorial02.RMP"];
    let terrain_source=battle_terrain::Source::bind(&mut sources,&actors,&settings,Precision::Double53).unwrap();
    let depth:Vec<_>=actors.resolved_definitions("SVars").unwrap().into_iter()
        .filter_map(|n|n.field("StaticMapBottleneckDetectionDepth",Precision::Double53).unwrap())
        .map(|v|v.integer().unwrap()).collect();
    let [depth]=depth.as_slice() else {panic!("unique actual depth")};
    let mut rows=Vec::new();
    for pc in [Precision::Single24,Precision::Double53,Precision::Extended64] {
        let terrain=battle_terrain::BattleTerrain::load(&archive,&terrain_source).unwrap();
        let spatial=LoadSpatial::new(&terrain.bound,pc).unwrap();
        let zones=crate::load_zone_services::before_first_record(&terrain.bound,spatial.index(),&header).unwrap();
        let mut map=Map::load(&catalog,&archive,&terrain,spatial.index(),&zones,pc).unwrap();
        if pc==Precision::Single24 {
            let mut rejected=Map::load(&catalog,&archive,&terrain,spatial.index(),&zones,pc).unwrap();
            let name="staticmapbottleneckdetectiondepth";
            let original=catalog.settings.as_ref().unwrap().declarations[name][0].clone();
            catalog.settings.as_mut().unwrap().declarations.get_mut(name).unwrap()[0].raw="987654".into();
            {
                let bank=catalog.definition_bank.borrow();
                let error=rejected.prepare(&catalog,&bank,&zones).unwrap_err();
                assert!(error.contains("Settings raw value/type/span"),"{error}");
                assert!(rejected.lifecycle().dynamic&&rejected.graph().is_some(),"native preparation prefix is retained");
            }
            catalog.settings.as_mut().unwrap().declarations.get_mut(name).unwrap()[0]=original;
        }
        assert_eq!(map.auxiliary_consumed() as u64,prior["consumed"].as_u64().unwrap());
        assert_eq!(map.topology().regions.len(),prior["regions"].as_array().unwrap().len());
        assert!(!map.lifecycle().dynamic && map.lifecycle().route_cache.is_none());
        {
            let bank=catalog.definition_bank.borrow();
            map.prepare(&catalog,&bank,&zones).unwrap();
            assert!(map.prepare(&catalog,&bank,&zones).is_err(),"failed/complete attempt is not reset");
        }
        let graph=map.graph().unwrap();
        assert!(!graph.ready,"map preparation does not authorize engine startup");
        for (i,node) in graph.nodes.iter().enumerate() {
            assert_eq!(json!(node.neighbors),prior["regions"][i]["neighbors"]);
            assert_eq!(json!(map.lifecycle().areas[i]),prior["regions"][i]["area"]);
        }
        let cache=map.lifecycle().route_cache.as_ref().unwrap();
        let cache_json=serde_json::to_value(cache).unwrap();
        let layout=terrain.bound.layout;
        let output=json!({"consumed":map.auxiliary_consumed(),"prepared":map.lifecycle().dynamic,
            "areas":map.lifecycle().areas,"bottlenecks":map.lifecycle().bottlenecks,
            "neighbors":graph.nodes.iter().map(|n|&n.neighbors).collect::<Vec<_>>(),"route_cache":cache_json,
            "primary_island":map.post_load().unwrap().primary_island,"diagonal_bits":map.post_load().unwrap().diagonal_bits});
        rows.push(json!({"precision":pc,"source_precision":"Double53","source_files":actors.sources,
            "map_source":archive.source(),"terrain_source":terrain.terrain.source,
            "input":{"suffix_hex":terrain.terrain.pending_auxiliary_bytes.iter().map(|b|format!("{b:02x}")).collect::<String>(),
                "layout":layout,"flags20":terrain.bound.cells.iter().map(|c|c.flags20).collect::<Vec<_>>(),"depth":depth},"output":output}));
        map.begin_startup().unwrap();
        assert!(map.begin_startup().is_err(),"a later ready prefix cannot bless a repeated world attempt");
    }
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-sai-world-ready/native-actual-map.json");
    if path.exists() {
        let native:Value=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        for (r,n) in rows.iter().zip(native["rows"].as_array().unwrap()) {assert_eq!(r["output"],n["output"]);}
    } else {assert!(std::env::var_os("AA_SAI_MAP_CAPTURE").is_some(),"actual native fixture required");}
    if let Some(path)=std::env::var_os("AA_SAI_MAP_CAPTURE") {
        let file=std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap();
        serde_json::to_writer_pretty(file,&json!({"cases":rows})).unwrap();
    }
}
