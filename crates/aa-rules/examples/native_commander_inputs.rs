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
    let data = ["zhukov_tutorial", "patton_tutorial", "rundstedt"]
        .map(|id| c.native_commander_constructor_inputs(id, Precision::Double53));
    let data: Result<Vec<_>, _> = data.into_iter().collect();
    println!("{}", serde_json::to_string_pretty(&data?)?);
    Ok(())
}
