use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
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
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"resources":c.native_resource_registry(pc)?,"damage_types":c.native_damage_type_registry(pc)?,"ability_types":c.native_ability_type_domain(pc)?,"target_types":c.native_target_domain(pc)?})
        )?
    );
    Ok(())
}
