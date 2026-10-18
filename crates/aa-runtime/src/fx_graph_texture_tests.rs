use super::*;
use aa_mapworld::restore::RestoreDefinitions;
#[test]
#[ignore="requires installed V14 sources/completed Bank; resource/source preparation only"]
fn actual_radar_graph_retains_same_resource_and_settings_before_any_request(){
    use aa_rules::{actors::ActorCatalog,sources::{V14Sources,SourceLimits},tgi::Limits};
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
    let actors=ActorCatalog::load_v14(&mut files).unwrap();
    let source_graphs;
    let mut catalog=Catalog::new(&actors);
    catalog.settings=Some(crate::settings::Settings::load_v14(&mut files,Limits::default()).unwrap());
    let receipt=crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
    crate::combat_source_completion::install(&catalog,&mut files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    let pc=catalog.definition_bank.borrow().combat_source().unwrap().source_precision();
    let definition=catalog.resolve(&"thing_objective_primary01".encode_utf16().collect::<Vec<_>>(),pc).unwrap();
    let bank=std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    let mut application=crate::resource_depot::Application::from_sources(files);
    struct FoundOnly;
    impl TexturePolicy for FoundOnly{} //Every reached missing/current-warning callback remains required.
    source_graphs=application.source_graphs();
    let mut resources=application.resources();
    let graph=Graph::compile_complete_resources(&catalog,&bank,&definition,ListKind::Created,&mut resources,&mut FoundOnly).unwrap();
    assert_eq!(graph.nodes().len(),1);assert_eq!(graph.node_kind(graph.node_id(0).unwrap()),Some(FxcKind::CreateRfx));
    let handle=graph.nodes()[0].texture(0x3c).unwrap().as_ref().unwrap().clone();
    assert_eq!(String::from_utf16(handle.logical_ids()).unwrap().to_ascii_lowercase(),"minimap/icons/iconradar01.png");
    assert_eq!(handle.kind(),crate::resource_objects::Kind::Texture);
    assert!(handle.source_file().is_some());
    assert_eq!(resources.flags_08(&handle).unwrap(),0x100000);
    assert_eq!(resources.requests_04(&handle).unwrap(),0);
    resources.with_entry(&handle,|entry|{
        let texture=entry.texture.as_ref().unwrap();
        assert_eq!(texture.render_count60,1);
        assert_eq!(texture.render_slots[0],[1,0,255,65280,16711680,4278190080,32,0x20820881,0xc4100]);
        assert!(texture.source3c.is_none());Ok(())
    }).unwrap();
    //A second real source compile performs a second Find and pushes onto the
    //SAME mutable texture; it never copies the Resource allocation into Graph.
    let references=resources.references_14(&handle).unwrap();
    let next=Graph::compile_complete_resources(&catalog,&bank,&definition,ListKind::Created,&mut resources,&mut FoundOnly).unwrap();
    assert!(next.nodes()[0].texture(0x3c).unwrap().as_ref().unwrap().same_identity(&handle));
    assert_eq!(resources.references_14(&handle).unwrap(),references.wrapping_add(1));
    resources.with_entry(&handle,|entry|{let t=entry.texture.as_ref().unwrap();assert_eq!(t.render_count60,2);assert_eq!(t.render_slots.len(),4);Ok(())}).unwrap();
    graph.require_runtime_fields().unwrap(); //Full fields, not effect execution/readiness.
    let request_only=Graph::compile_request_resources(&catalog,&bank,&definition,ListKind::Created,&mut resources,&mut FoundOnly).unwrap();
    assert!(request_only.require_runtime_fields().is_err(),"legacy request API cannot promote its graph");
    let failed=Graph::compile_request_resources(&catalog,&Bank::default(),&definition,ListKind::Created,&mut resources,&mut FoundOnly).unwrap_err();
    assert!(failed.partial.nodes().is_empty());
    assert_eq!(resources.references_14(&handle).unwrap(),references.wrapping_add(2));
    drop(resources);
    struct ModelLab;
    impl crate::resource_objects::ModelPolicy for ModelLab{fn use_hardware_model_73a560(&mut self)->Result<bool>{Ok(false)}}
    let prepared=application.with_data_pre_request_resources(|state,resources|{
        let data=state.prepare_object_requests(&catalog,&bank,&definition,resources,&mut ModelLab,&mut FoundOnly)?;
        let graph=state.prepared_view_graph(&catalog,&bank,&definition,ListKind::Created)?.ok_or("actual prepared source arena missing")?;
        let references=resources.references_14(&handle)?;
        let again=state.prepare_object_requests(&catalog,&bank,&definition,resources,&mut ModelLab,&mut FoundOnly)?;
        assert!(data.same_allocation(&again));assert_eq!(resources.references_14(&handle)?,references);
        assert!(std::sync::Arc::ptr_eq(&graph,&state.prepared_view_graph(&catalog,&bank,&definition,ListKind::Created)?.unwrap()));
        Ok(graph)
    }).unwrap();
    let retained=source_graphs.prepared_view_graph(&catalog,&bank,&definition,ListKind::Created).unwrap().unwrap();
    assert!(std::sync::Arc::ptr_eq(&retained,&prepared));
    catalog.bind_source_graphs(&source_graphs).unwrap();
    assert!(std::sync::Arc::ptr_eq(&retained,&catalog.prepared_view_graph(&bank,&definition,ListKind::Created).unwrap().unwrap()));
    assert_eq!(application.resources().requests_04(&handle).unwrap(),0,"read cap permits a disjoint real Resource loan");
    let foreign_files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
    let mut foreign=crate::resource_depot::Application::from_sources(foreign_files);
    let mismatch=application.with_data_pre_request(|state|state.prepare_object_requests(&catalog,&bank,&definition,&mut foreign.resources(),&mut ModelLab,&mut FoundOnly)).unwrap_err();
    assert!(mismatch.contains("DataFXSession Resource Application differs"));
    assert!(std::sync::Arc::ptr_eq(&retained,&source_graphs.prepared_view_graph(&catalog,&bank,&definition,ListKind::Created).unwrap().unwrap()));
    assert!(source_graphs.prepared_view_graph(&catalog,&Bank::default(),&definition,ListKind::Created).is_err());
    //The read capability keeps no Application alive, even though it aliases
    //the same source State allocation. Retained immutable Graph is separate.
    drop(application);
    assert!(source_graphs.prepared_view_graph(&catalog,&bank,&definition,ListKind::Created).unwrap_err().contains("retired"));
    println!("actual radar {} same source-PC {:?}, retained Texture and two native format pushes",definition.proof.id,pc);
}
