use aa_resources::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let report = assess(
        &query_memory()?,
        MemoryPolicy::default(),
        MemoryEstimate::default(),
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
