//! Read-only source inventory, not live native Property-instance readiness.
use aa_rules::{actors::ActorCatalog,sources::{V14Sources,SourceLimits},native::TypedValue};
use aa_numeric::Precision;
use serde_json::json;
fn main()->Result<(),Box<dyn std::error::Error>> {
    let root=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").ok_or("AA_GAME_ROOT required")?);
    let out=std::path::PathBuf::from(std::env::var_os("AA_TERRAIN_PROPERTY_CENSUS").ok_or("new output required")?);
    let mut sources=V14Sources::open(root.join("Data.rwd"),root.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits {max_member_bytes:64*1024*1024})?;
    let actors=ActorCatalog::load_v14(&mut sources)?;
    let registry=actors.native_data_registry("Terrain").ok_or("Terrain registry required")?;
    let mut rows=vec![];
    for entry in &registry.entries {
        let node=actors.native_data_node(entry)?;
        let mut properties=vec![];
        for child in node.children("Property")? {
            let value=child.field("property_ids",Precision::Double53)?;
            let value=match value {Some(TypedValue::String{value,..})=>json!(value),other=>json!({"required_typed_string":format!("{other:?}")})};
            properties.push(value);
        }
        rows.push(json!({"id":entry.ids,"reloads":entry.reloads.len(),"properties_authored_order":properties,"sources":node.sources()}));
    }
    let positive=rows.iter().filter(|v|!v["properties_authored_order"].as_array().unwrap().is_empty()).count();
    let value=json!({"scope":"read-only native-order Terrain source-view inventory at explicit Double53; no completed source receipt verification inside this standalone tool, no Property link/criteria/instance proof, no reachability or game-readiness claim",
        "source_precision":Precision::Double53,"registrations":rows.len(),"with_authored_properties":positive,"sources":actors.sources,"rows":rows});
    serde_json::to_writer_pretty(std::fs::OpenOptions::new().create_new(true).write(true).open(out)?,&value)?;
    println!("Terrain source registrations={}, positivePropertyviews={positive}",rows.len());
    Ok(())
}
