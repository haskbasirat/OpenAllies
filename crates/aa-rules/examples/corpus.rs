use aa_rules::{
    sources::{SourceLimits, V14Sources},
    tgi::Document,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: corpus <Data.rwd> <v14 runtime> <per-input byte budget>".into());
    }
    let mut sources = V14Sources::open(
        &args[1],
        &args[2],
        SourceLimits {
            max_member_bytes: args[3].parse()?,
        },
    )?;
    let catalog = sources.catalog();
    let mut passed = 0;
    let mut errors = Vec::new();
    let mut bytes = 0u64;
    for row in catalog
        .iter()
        .filter(|r| r.logical_path.to_ascii_lowercase().ends_with(".tgi"))
    {
        let r = sources.read(&row.logical_path)?;
        bytes += r.bytes.len() as u64;
        match Document::parse(r.bytes.clone()) {
            Ok(d) => {
                assert_eq!(d.original_bytes(), r.bytes);
                passed += 1;
            }
            Err(e) => {
                let lo = e.offset.saturating_sub(60);
                let hi = (e.offset + 120).min(r.bytes.len());
                errors.push(serde_json::json!({"path":row.logical_path,"sha256":r.sha256,"offset":e.offset,"error":e.message,"context":String::from_utf8_lossy(&r.bytes[lo..hi])}));
            }
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"meaning":"syntax coverage only; no native semantics or evaluation","parsed":passed,"failed":errors.len(),"source_bytes":bytes,"errors":errors})
        )?
    );
    Ok(())
}
