use super::*;
use crate::{collision,index::{ActorKey,Member},index_world::CollisionPolicy};
use serde_json::{Value,json};

struct Policy{fail:bool}
fn unreachable_child<T>()->collision::Result<T>{panic!("empty native rectangle descriptor reached an unrelated child")}
impl CollisionPolicy for Policy{
    fn definition_flags(&self,_:u32)->collision::Result<u32>{unreachable_child()}
    fn definition_flag_1a8(&self,_:u32)->collision::Result<bool>{
        //Fixture's actual allocated Data+1A8 byte remains zero.
        if self.fail{Err(collision::Error::World("required current source field".into()))}else{Ok(false)}
    }
    fn custom_filter(&mut self,_:&mut SpatialIndex,_:ActorKey,_:&mut Descriptor)->collision::Result<bool>{unreachable_child()}
    fn source_excludes(&mut self,_:&mut SpatialIndex,_:ActorKey,_:ActorKey,_:&mut Descriptor)->collision::Result<bool>{unreachable_child()}
    fn allegiance(&mut self,_:&mut SpatialIndex,_:ActorKey,_:&mut Descriptor)->collision::Result<Option<u32>>{unreachable_child()}
    fn hostility_dispatch(&self,_:u32)->collision::Result<u32>{unreachable_child()}
    fn hostile(&mut self,_:&mut SpatialIndex,_:u32,_:Option<u32>,_:Option<u32>,_:&mut Descriptor)->collision::Result<bool>{unreachable_child()}
    fn moving_exclusion(&mut self,_:&mut SpatialIndex,_:ActorKey,_:&mut Descriptor)->collision::Result<bool>{unreachable_child()}
    fn outer_actor(&mut self,_:&mut SpatialIndex,_:ActorKey,_:&mut Descriptor)->collision::Result<Option<u32>>{unreachable_child()}
}
impl QueryPolicy for Policy{fn definition_has_object_data(&self,_:u32)->collision::Result<bool>{unreachable_child()}}
fn fixture()->Value{serde_json::from_str(include_str!("../../../research/development10/organization-spawn/native-rectangle-actors.json")).unwrap()}
fn setup(c:&Value)->(SpatialIndex,QueryGeometry){
    let input=&c["input"];let mut index=SpatialIndex::new(serde_json::from_value(input["grid"].clone()).unwrap(),
        serde_json::from_value(input["flags"].clone()).unwrap(),serde_json::from_value(c["precision"].clone()).unwrap()).unwrap();
    for member in input["members"].as_array().unwrap(){let m:Member=serde_json::from_value(member.clone()).unwrap();let key=m.key;index.register(m).unwrap();index.insert(key).unwrap();}
    for key in input["premarked"].as_array().unwrap(){let key=key.as_u64().unwrap()as u32;index.set_query_link(key,Some(key)).unwrap();}
    let geometry=QueryGeometry::from_native_extents(&index,serde_json::from_value(input["extent_bits"].clone()).unwrap(),index.precision()).unwrap();
    (index,geometry)
}
#[test]
fn actual_original_v14_mode0_rectangle_actor_order_and_cleanup(){
    let doc=fixture();let rows=doc["rows"].as_array().unwrap();assert_eq!(rows.len(),144);
    for c in rows{let input=&c["input"];let(mut index,geometry)=setup(c);
        let addresses:Vec<_>=index.members().map(|m|(m.key,m as *const Member)).collect();
        let found=collect(&mut index,&geometry,serde_json::from_value(input["position"].clone()).unwrap(),input["radius"].as_u64().unwrap()as u32,
            serde_json::from_value(input["source_facing"].clone()).unwrap(),&mut Policy{fail:false}).unwrap();
        assert_eq!(json!(found),c["native"]["references"],"{c}");
        for(key,address)in addresses{assert_eq!(index.member(key).unwrap()as *const Member,address);assert_eq!(json!(index.query_link(key).unwrap()),c["native"]["links"][key.to_string()],"{c}");}
    }
}
#[test]
fn source_failure_releases_own_marks_and_keeps_preexisting_marks(){
    let doc=fixture();let c=doc["rows"].as_array().unwrap().iter().find(|c|c["scenario"]==10).unwrap();
    let(mut index,geometry)=setup(c);let input=&c["input"];
    let before:Vec<_>=index.members().map(|m|(m.key,index.query_link(m.key).unwrap())).collect();
    let result=collect(&mut index,&geometry,serde_json::from_value(input["position"].clone()).unwrap(),input["radius"].as_u64().unwrap()as u32,
        serde_json::from_value(input["source_facing"].clone()).unwrap(),&mut Policy{fail:true});
    assert!(result.is_err());for(key,mark)in before{assert_eq!(index.query_link(key).unwrap(),mark);}
}
#[test]
fn actual_native_cursor_interleaving_marks_and_end_cleanup(){
    let doc:Value=serde_json::from_str(include_str!("../../../research/development10/organization-spawn/native-rectangle-actors-v2.json")).unwrap();
    for c in doc["rows"].as_array().unwrap(){let input=&c["input"];let(mut index,geometry)=setup(c);
        let mut cursor=Cursor::begin(&mut index,&geometry,serde_json::from_value(input["position"].clone()).unwrap(),
            input["radius"].as_u64().unwrap()as u32,serde_json::from_value(input["source_facing"].clone()).unwrap()).unwrap();
        for step in c["native"]["steps"].as_array().unwrap(){
            assert_eq!(json!(cursor.next(&mut index,&mut Policy{fail:false}).unwrap()),step["reference"],"{c}");
            for m in index.members(){assert_eq!(json!(index.query_link(m.key).unwrap()),step["links"][m.key.to_string()],"{c}");}
        }
        cursor.finish(&mut index).unwrap();
    }
}
