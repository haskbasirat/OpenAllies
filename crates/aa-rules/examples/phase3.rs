use aa_rules::{
    actors::{ActorCatalog, FieldResolution},
    sources::{SourceLimits, V14Sources},
    tgi::{Document, Limits},
};
use std::collections::BTreeMap;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: phase3 <Data.rwd> <v14runtime> <member budget>".into());
    }
    let mut sources = V14Sources::open(
        &args[1],
        &args[2],
        SourceLimits {
            max_member_bytes: args[3].parse()?,
        },
    )?;
    let mut files = Vec::new();
    let mut failures = Vec::new();
    let mut notices = Vec::new();
    let mut passed = 0;
    for row in sources
        .catalog()
        .into_iter()
        .filter(|r| r.logical_path.to_ascii_lowercase().ends_with(".tgi"))
    {
        let file = sources.read(&row.logical_path)?;
        match Document::parse_native(file.bytes.clone(),Limits::default()) {
   Ok(d)=>{passed+=1;if !d.notices.is_empty(){notices.push(serde_json::json!({"path":file.logical_path,"sha256":file.sha256,"notices":d.notices}));}},
   Err(e)=>failures.push(serde_json::json!({"path":file.logical_path,"sha256":file.sha256,"error":e.to_string()})),
  }
        files.push(file);
    }
    let c = ActorCatalog::from_files(files, Limits::default());
    let mut unsupported = BTreeMap::<String, usize>::new();
    let mut values = 0;
    for a in &c.actors {
        for field in a.fields.values() {
            match field {
                FieldResolution::Unsupported { reason } => {
                    *unsupported.entry(reason.clone()).or_default() += 1
                }
                FieldResolution::Value { .. } => values += 1,
                _ => {}
            }
        }
    }
    let samples: Vec<_> = c
        .actors
        .iter()
        .filter(|a| {
            [
                "usa_infantry",
                "usa_tank_medium",
                "gbr_corps",
                "gbr_depot_oil",
                "start_location",
            ]
            .contains(&a.id.as_str())
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"parsed":passed,"failed":failures.len(),"errors":failures,"notices":notices,"actor_count":c.actors.len(),"resolved_field_values":values,"unsupported":unsupported,"samples":samples,"source_identities":c.sources})
        )?
    );
    Ok(())
}
