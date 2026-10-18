use aa_rules::{
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: resources <Data.rwd> <v14 runtime> <per-input byte budget>".into());
    }
    let mut sources = V14Sources::open(
        &args[1],
        &args[2],
        SourceLimits {
            max_member_bytes: args[3].parse()?,
        },
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&ResourceCatalog::load_v14(&mut sources)?)?
    );
    Ok(())
}
