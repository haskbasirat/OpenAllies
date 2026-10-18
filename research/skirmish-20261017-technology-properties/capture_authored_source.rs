//! Read-only research capture using public existing aa-rules APIs. No runtime
//!definition completion or property effects are claimed by this executable.
use aa_rules::{actors::ActorCatalog,native::TypedValue,sources::{V14Sources,SourceLimits}};
use aa_numeric::Precision;
use serde_json::json;
fn text(node:&aa_rules::actors::ResolvedNode<'_>,name:&str)->String {
    match node.field(name,Precision::Double53).unwrap().unwrap(){TypedValue::String{value,..}=>value,_=>panic!("typed string required")}
}
fn main(){
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:64*1024*1024}).unwrap();
    let actors=ActorCatalog::load_v14(&mut sources).unwrap();let mut rows=Vec::new();
    for actor in &actors.actors {
        if actors.native_category(&actor.id).unwrap().native_kind!=2{continue}
        let node=actors.resolved_actor(&actor.id).unwrap();let Some(structure)=node.child("StructureComponent").unwrap() else{continue};
        let mut requests=Vec::new();let mut lookups=serde_json::Map::new();
        for child in structure.children("TechnologyProperty").unwrap(){
            let request=text(&child,"property_IDS");let property=actors.native_named_definition("Property",&request,Precision::Double53).unwrap().unwrap();
            lookups.insert(request.to_ascii_lowercase(),json!({"id":text(&property,"IDS"),"source":property.sources()}));requests.push(request);
        }
        if !requests.is_empty(){rows.push(json!({"id":actor.id,"source":actor.source,"authored_requests":requests,"canonical_lookups":lookups}));}
    }
    let path=std::env::var_os("AA_CAPTURE_TECHNOLOGY_AUTHORED").unwrap();
    let file=std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap();
    serde_json::to_writer_pretty(file,&json!({"scope":"read-only pure source capture of every nonempty StructureComponent TechnologyProperty list, original authored order","precision":"Double53","rows":rows,"source_identities":actors.sources})).unwrap();
}
