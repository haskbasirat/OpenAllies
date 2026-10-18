use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    native::TypedValue,
    sources::{SourceLimits, V14Sources},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().collect();
    let mut s = V14Sources::open(
        &a[1],
        &a[2],
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )?;
    let c = ActorCatalog::load_v14(&mut s)?;
    let pc = Precision::Double53;
    let mut ids = std::collections::BTreeSet::new();
    let mut actors = Vec::new();
    for id in [
        "usa_infantry",
        "gbr_infantry",
        "ger_infantry",
        "jpn_infantry",
        "rus_infantry",
        "regiment_usa_infantry",
    ] {
        let value = c.native_actor_init_inputs(id, pc)?;
        for p in &value.initial_properties {
            ids.insert(p.id.clone());
        }
        actors.push(value);
    }
    let mut properties = Vec::new();
    for id in ids {
        let node = c.resolved_definition("Property", &id, pc)?;
        let mut modifiers = Vec::new();
        for n in node.children("Modifier")? {
            let mut fields = serde_json::Map::new();
            for field in [
                "type",
                "attribute",
                "amount",
                "repair",
                "modifier_flag",
                "ability_type",
                "damage_type",
            ] {
                fields.insert(field.into(), serde_json::to_value(n.field(field, pc)?)?);
            }
            modifiers.push(fields);
        }
        let flags = node.field("enabled_target_flags", pc)?;
        let TypedValue::String { value: target, .. } = flags.as_ref().unwrap() else {
            panic!()
        };
        properties.push(serde_json::json!({"id":id,"target_flags":target,"modifiers":modifiers,"sources":node.sources()}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"actors":actors,"properties":properties})
        )?
    );
    Ok(())
}
