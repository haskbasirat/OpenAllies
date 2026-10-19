//!ActorData+534 over the actual v14 definitions. The fixture is produced by
//!research/skirmish-20261018-actor-deployment-534/native_actor_deployment_534.py,
//!which executes the ORIGINAL464BD0 block,475230/461840/4605B0 and45DD80 on
//!the source values exported here (AA_EXPORT_DEPLOYMENT_INPUT=<new path>).
use super::*;
use crate::catalog::Catalog;
use aa_rules::{actors::ActorCatalog,resources::ResourceCatalog,sources::{SourceLimits,V14Sources},tgi::Limits};
use serde_json::{Value,json};
use std::{path::PathBuf,sync::Arc};

const FIXTURE:&str="../../research/skirmish-20261018-actor-deployment-534/native-actor-deployment-534.json";

fn open_sources()->V14Sources{
    let game=PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap()
}
fn completed_catalog<'a>(actors:&'a ActorCatalog,files:&mut V14Sources)->Catalog<'a>{
    let mut catalog=Catalog::new(actors);
    catalog.settings=Some(crate::settings::Settings::load_v14(files,Limits::default()).unwrap());
    catalog.bind_resource_catalog(Arc::new(ResourceCatalog::load_v14(files).unwrap())).unwrap();
    let receipt=crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog,files).unwrap();
    crate::combat_source_completion::install(&catalog,files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    crate::replacement_flags_source::complete_source_suffix(&catalog).unwrap();
    catalog
}
///Every value the block could read, regardless of branch: the oracle's typed
///reader seams answer only the keys the original code asks for.
fn exported_component(node:&ResolvedNode<'_>,pc:Precision)->Value{
    let mut n=Node{node,pc};
    json!({"unpack_to_actor_ids":n.string("unpack_to_actor_ids").unwrap(),
        "pack_health_min":n.fixed("pack_health_min").unwrap(),"packing_length":n.fixed("packing_length").unwrap(),
        "unpacking_length":n.fixed("unpacking_length").unwrap(),
        "packed_position_offset":n.point("packed_position_offset").unwrap(),"packed_orientation":n.point("packed_orientation").unwrap()})
}

#[test]
#[ignore="requires actual V14 sources (AA_GAME_ROOT); source fields and deferred pass only"]
fn actual_mobile_deployment_534_for_every_registered_actor(){
    let mut files=open_sources();
    let actors=ActorCatalog::load_v14(&mut files).unwrap();
    let catalog=completed_catalog(&actors,&mut files);
    let pc=catalog.definition_bank.borrow().combat_source().unwrap().source_precision();
    let mobile=catalog.native_mobile_deployment_data(pc).unwrap();
    let (mut rows,mut links)=(Vec::new(),std::collections::BTreeMap::new());
    let (mut actor_count,mut forward,mut pack_side)=(0,0,0);
    for entry in &catalog.actors.actors{
        let thing=aa_rules::actors::ThingRef::registered(&entry.id);
        let Ok(category)=catalog.actors.native_category(thing)else{continue};
        if category.native_kind!=2{continue}
        actor_count+=1;
        let definition=catalog.resolve_registered(&entry.id,pc).unwrap();
        let mask=definition.component_mask.unwrap();
        let node=super::super::root(&catalog,&definition).unwrap();
        let scope=node.child(COMPONENT).unwrap();
        let block=scope.as_ref().map(|s|read_block(&mut Node{node:s,pc}).unwrap());
        if let Some(b)=&block{validate_block(b,&entry.id).unwrap();}
        let target=forward_534(mask,&entry.id,block.as_ref(),(mask&MOBILE_DEPLOYMENT_BIT!=0).then_some(&*mobile)).unwrap();
        let Some(scope)=scope else{assert_eq!(mask&MOBILE_DEPLOYMENT_BIT,0);continue};
        if let Some(t)=&target{
            forward+=1;
            let resolved=catalog.resolve_registered(t,pc).unwrap();
            assert_eq!(resolved.proof.category.native_kind,2);
            assert!(resolved.component_mask.unwrap()&MOBILE_DEPLOYMENT_BIT!=0,"{} -> {t}",entry.id);
        }else{pack_side+=1;assert!(mobile.definition(&entry.id).unwrap().pack_to.is_some(),"{} is neither side",entry.id);}
        links.insert(entry.id.clone(),target.clone());
        let block=block.unwrap();
        let alias=block.reference.as_ref().map(|r|catalog.actors.find(r).map(|d|d.id.clone()));
        rows.push(json!({"id":entry.id,"mask":mask,"mask_before":mask&!MOBILE_DEPLOYMENT_BIT,
            "soft_radius":catalog.actors.native_object_definition(thing,pc).unwrap().soft.words[1],
            "component":exported_component(&scope,pc),"reference":block.reference,"find":alias}));
    }
    assert_eq!((actor_count,rows.len(),forward,pack_side),(1028,140,75,65));
    //The definitions that stopped view creation are the pack side: null+534.
    for id in ["rus_corps","ger_corps","usa_corps","gbr_corps","jpn_corps","rus_corps_dynamic","ger_hq_armor","usa_depot_supply","gbr_airfield","jpn_motor_pool"]{
        assert_eq!(links[id],None,"{id}");
    }
    for (truck,building) in [("rus_truck_corps","rus_corps"),("ger_truck_corps","ger_corps"),("rus_truck_corps_dynamic","rus_corps_dynamic"),("usa_truck_airfield","usa_airfield")]{
        assert_eq!(links[truck].as_deref(),Some(building),"{truck}");
    }
    if let Some(path)=std::env::var_os("AA_EXPORT_DEPLOYMENT_INPUT"){
        let text=serde_json::to_string_pretty(&json!({"schema":1,"precision":format!("{pc:?}"),"actors":rows})).unwrap();
        use std::io::Write;
        std::fs::OpenOptions::new().write(true).create_new(true).open(&path).unwrap().write_all(text.as_bytes()).unwrap();
        return;
    }
    //Replay: the original code's +534 for every shipped row.
    let fixture:Value=serde_json::from_str(&std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE)).unwrap()).unwrap();
    let mut checked=0;
    for case in fixture["cases"].as_array().unwrap().iter().filter(|c|c["input"]["shipped"]==true){
        assert_eq!(case["input"]["actors"],Value::Array(rows.clone()),"shipped oracle input differs from current sources");
        let output=&case["output"];assert_eq!(output["error"],Value::Null);
        for (id,word) in output["actors"].as_object().unwrap(){
            assert_eq!(word["534"].as_str(),links[id].as_deref(),"{id}");
            assert_eq!(word["174"].as_u64().unwrap() as u32&MOBILE_DEPLOYMENT_BIT,MOBILE_DEPLOYMENT_BIT,"{id}");
            checked+=1;
        }
    }
    assert!(checked>=140,"shipped oracle rows checked {checked}");
}

struct LabModel;
impl crate::resource_objects::ModelPolicy for LabModel{fn use_hardware_model_73a560(&mut self)->Result<bool>{Ok(false)}}
///Shipped UVars values (DataCompatibilityMode1, DataMissingResourceWarning0),
///as in the resource census laboratory; missing-texture diagnostics recorded.
#[derive(Default)]
struct LabTexture{diagnostics:usize}
impl crate::resource_objects::TexturePolicy for LabTexture{
    fn data_compatibility_mode_current(&mut self)->Result<u8>{Ok(1)}
    fn data_missing_resource_warning_current(&mut self)->Result<u8>{Ok(0)}
    fn missing_texture_diagnostic_404fc0(&mut self,_:&[u16],_:u32)->Result<()>{self.diagnostics+=1;Ok(())}
    fn texture_cache_source_overrides(&mut self)->Result<crate::load_view_variables::NativeOverrides>{Ok(crate::load_view_variables::NativeOverrides::ControlledNone)}
}

///Production pre-World source preparation (the same call the client makes for
///every registered definition) of HQ buildings and their trucks.
#[test]
#[ignore="requires actual V14 sources (AA_GAME_ROOT); pre-World source preparation, no World/GPU readiness"]
fn actual_hq_buildings_and_trucks_prepare_their_534(){
    let mut files=open_sources();
    let actors=ActorCatalog::load_v14(&mut files).unwrap();
    let catalog=completed_catalog(&actors,&mut files);
    let pc=catalog.definition_bank.borrow().combat_source().unwrap().source_precision();
    let mut application=crate::resource_depot::Application::from_sources(files);
    application.install_fresh_cache_variable_scope(&catalog,crate::load_view_variables::NativeOverrides::ControlledNone).unwrap();
    {let bank=catalog.definition_bank.borrow();application.prepare_model_source_context(&catalog,&bank).unwrap();}
    let work=std::env::temp_dir().join(format!("aa-deployment534-{}",std::process::id()));
    std::fs::create_dir_all(&work).unwrap();
    application.configure_portable_work_depot(&work).unwrap();
    let (mut model,mut texture)=(LabModel,LabTexture::default());
    let cases=[("rus_corps",None),("rus_truck_corps",Some("rus_corps")),("ger_corps",None),("ger_truck_corps",Some("ger_corps")),
        ("usa_corps",None),("gbr_hq_armor",None),("jpn_depot_oil",None),("usa_truck_airfield",Some("usa_airfield"))];
    let bank=catalog.definition_bank.borrow();
    let mut prepared=Vec::new();
    for(id,_)in cases{
        let definition=catalog.resolve_registered(id,pc).unwrap();
        let data=application.with_data_pre_request_resources(|state,resources|{
            state.prepare_object_requests(&catalog,&bank,&definition,resources,&mut model,&mut texture)
        }).unwrap_or_else(|e|panic!("{id}: {e}"));
        //Before the post-LoadData deferred pass, +534 is not yet linked.
        assert!(application.with_data_pre_request(|state|state.actor_deployment_534(&data)).is_err(),"{id}");
        prepared.push(data);
    }
    application.with_data_pre_request(|state|state.complete_deferred_links_475230(&catalog,&bank)).unwrap();
    for((id,expected),data)in cases.into_iter().zip(&prepared){
        let got=application.with_data_pre_request(|state|state.actor_deployment_534(data)).unwrap_or_else(|e|panic!("{id}: {e}"));
        assert_eq!(got.map(|d|d.proof.id),expected.map(String::from),"{id}");
    }
    println!("prepared; missing-texture diagnostics {}",texture.diagnostics);
}
