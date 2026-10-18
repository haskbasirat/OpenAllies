use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: native_actor_fields <Data.rwd> <accepted-v14-root>".into());
    }
    let mut sources = V14Sources::open(
        &args[1],
        &args[2],
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )?;
    let catalog = ActorCatalog::load_v14(&mut sources)?;
    let mut resolved = Vec::new();
    let mut errors = Vec::new();
    for actor in &catalog.actors {
        match catalog.native_attributes(&actor.id, Precision::Double53) {
            Ok(v) => resolved.push(v),
            Err(e) => errors.push(serde_json::json!({"id":actor.id,"error":e.to_string()})),
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"scope":"Typed bounded Body/Mover definition attributes; no complete actor factory or runtime precision claim","resolved_count":resolved.len(),"unresolved_count":errors.len(),"samples":resolved.iter().filter(|v|["usa_infantry","usa_tank_medium","gbr_corps","gbr_depot_oil","start_location"].contains(&v.id.as_str())).collect::<Vec<_>>(),"errors":errors})
        )?
    );
    Ok(())
}
