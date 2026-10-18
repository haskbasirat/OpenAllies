use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() != 3 {
        return Err("usage: native_object_definitions <Data.rwd> <v14-root>".into());
    }
    let mut source = V14Sources::open(
        &a[1],
        &a[2],
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )?;
    let catalog = ActorCatalog::load_v14(&mut source)?;
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    for id in [
        "usa_infantry",
        "ger_infantry",
        "gbr_infantry",
        "jpn_infantry",
        "rus_infantry",
        "usa_tank_medium",
        "regiment_usa_infantry",
    ] {
        match catalog.native_object_definition(id,Precision::Double53) {
   Ok(definition)=>rows.push(serde_json::json!({"definition":definition,"creation":catalog.native_creation_definition(id,Precision::Double53)?,"attributes":catalog.native_attributes(id,Precision::Double53)?,"ground":catalog.native_ground_definition(id,Precision::Double53)?,"performer":catalog.native_performer_inputs(id,Precision::Double53)?,"mask_if_no_performer_abilities":catalog.native_component_mask(id,Some(0))?,"mask_if_at_least_one_performer_ability":catalog.native_component_mask(id,Some(1))?})),
   Err(error)=>errors.push(serde_json::json!({"id":id,"error":error.to_string()}))
  }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"scope":"Authored original/v14 ground-unit ObjectData shapes and bounded Body/Mover attributes; no full component/ability factory claim","samples":rows,"errors":errors})
        )?
    );
    Ok(())
}
