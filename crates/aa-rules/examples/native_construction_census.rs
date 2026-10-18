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
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"sources":c.sources.len(),"omissions":c.diagnostics.iter().filter(|d|!d.message.starts_with("native syntax notice ")).collect::<Vec<_>>(),"census":format!("{:?}",c.construction_upgrade_census())})
        )?
    );
    Ok(())
}
