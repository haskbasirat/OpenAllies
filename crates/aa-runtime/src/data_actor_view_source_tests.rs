use super::*;
#[test]
fn unbound_foreign_or_destroyed_data_never_becomes_empty_actor_fields(){
    let mut state=State::from_native_constructor();
    let source=Provenance{path:"identity-lab.tgi".into(),sha256:"identity-lab-only".into(),span:aa_rules::tgi::Span{start:0,end:1}};
    let data=state.bind_canonical("Actor",&source,2,&[],Precision::Double53).unwrap();
    let view=ActorView{data};assert!(state.actor_view_is_actor_38(&view).is_err());
    assert!(state.actor_view_turret_count_1fc(&view).is_err());
    assert!(state.actor_view_source_word_23c(&view).is_err());
    assert!(state.actor_view_track_count_260(&view).is_err());
    let foreign=State::from_native_constructor();assert!(foreign.actor_view_is_actor_38(&view).is_err());
    drop(state);assert!(foreign.actor_view_secondary_count_244(&view).is_err());
}

#[test]
#[ignore="requires the actual original/v14 source files and verified completed source Bank"]
fn actual_actor_source_fields_share_canonical_data_and_native_ctor_defaults(){
    use crate::combat_source_receipt::VerifiedStartupReceipt;
    use aa_rules::{actors::ActorCatalog,sources::{V14Sources,SourceLimits},tgi::Limits};
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits{max_member_bytes:u64::MAX}).unwrap();
    let actors=ActorCatalog::load_v14(&mut files).unwrap();let mut catalog=Catalog::new(&actors);
    catalog.settings=Some(crate::settings::Settings::load_v14(&mut files,Limits::default()).unwrap());
    let receipt=VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
    crate::combat_source_completion::install(&catalog,&mut files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    let pc=catalog.definition_bank.borrow().combat_source().unwrap().source_precision();
    let source_nodes=["Thing","Unit","Projectile","VFX"].into_iter().map(|tag|actors.resolved_definitions(tag).unwrap()).flatten().collect::<Vec<_>>();
    let mut selected=[None,None,None];
    for actor in &actors.actors{
        if actors.native_category(&actor.id).unwrap().native_kind!=2{continue}
        let turrets=actors.native_turrets(&actor.id,pc).unwrap();
        //Select through the same resolved source-node API as the producer.
        //native_node_sources has a narrower gameplay-child whitelist and
        //does not admit the Tire list; that refusal is not an empty list.
        let node=source_nodes.iter().find(|node|node.sources().first()==Some(&actor.source)).unwrap();
        let mover=node.child("MoverComponent").unwrap();
        let tire_count=mover.as_ref().map(|node|node.children("Tire").unwrap().len()).unwrap_or(0);
        for(slot,eligible)in [(0,turrets.is_empty()&&mover.is_none()),(1,!turrets.is_empty()),(2,tire_count>0)]{
            if selected[slot].is_none()&&eligible{selected[slot]=Some(catalog.actor_bindings(&actor.id,pc).unwrap());}
        }
        if selected.iter().all(Option::is_some){break}
    }
    assert!(selected.iter().all(Option::is_some),"actual no-Mover, positive Turret and positive Tire source cases required");
    let bank=std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    let mut application=crate::resource_depot::Application::from_sources(files);let mut report=vec![];
    application.with_data_pre_request(|state|{
        for (index,binding)in selected.into_iter().enumerate(){
            let binding=binding.unwrap();let definition=&binding.definition;
            let view=state.actor_view_source(&catalog,&bank,definition)?;
            assert!(state.actor_view_is_actor_38(&view)?);
            let address=std::ptr::from_ref(state.actor_view_fields(&view)?);
            let again=state.actor_view_source(&catalog,&bank,definition)?;
            assert_eq!(address,std::ptr::from_ref(state.actor_view_fields(&again)?));
            let count=state.actor_view_turret_count_1fc(&view)?;
            let tire_count=state.actor_view_secondary_count_244(&view)?;
            let circumference=state.actor_view_source_word_23c(&view)?;
            let tracks=state.actor_view_track_count_260(&view)?;
            if index==0{assert_eq!((count,tire_count,circumference,tracks),(0,0,0,0));}
            if index==1{assert!(count>0);}
            if index==2{assert!(tire_count>0);assert_ne!(circumference,0);}
            let expected=actors.native_turrets(&definition.proof.id,pc).map_err(|e|e.to_string())?;
            let pitch=(0..count as usize).map(|i|state.actor_view_turret_pitch_14(&view,i)).collect::<Result<Vec<_>>>()?;
            assert_eq!(pitch,expected.iter().map(|v|v.pitch_start_bits).collect::<Vec<_>>());
            assert!(state.actor_view_turret_pitch_14(&view,count as usize).is_err());
            assert!(state.actor_view_source(&catalog,&Bank::default(),definition).is_err());
            assert_eq!(address,std::ptr::from_ref(state.actor_view_fields(&view)?));
            report.push(serde_json::json!({"id":definition.proof.id,"source":definition.proof.source,"precision":pc,
                "turrets":count,"tires":tire_count,"tracks":tracks,"circumference":circumference,"pitch":pitch,"same_allocation":true}));
        }
        Ok(())
    }).unwrap();
    let native:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/view-object-init/native-actor-source.json")).unwrap();
    for row in native["rows"].as_array().unwrap(){assert_eq!(row["fields"],serde_json::json!({"0x1fc":0,"0x23c":0,"0x244":0}));assert_eq!(row["actor38"],1);assert_eq!(row["object38"],0);}
    let report=serde_json::json!({"scope":"actual typed source fields and canonical Data owner only; no World or full Loader readiness","rows":report});
    println!("{report}");
    if let Some(path)=std::env::var_os("AA_ACTOR_VIEW_SOURCE_CAPTURE"){
        let path=std::path::PathBuf::from(path);assert!(path.is_absolute());std::fs::write(path,serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}
