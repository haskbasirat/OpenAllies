//! Read-only follow-up of the exact phase1 unsupported-source list.
use aa_rules::{
    sources::{SourceLimits, V14Sources},
    tgi::Document,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: phase2_followup <Data.rwd> <v14 runtime> <per-input byte budget>".into(),
        );
    }
    let mut sources = V14Sources::open(
        &args[1],
        &args[2],
        SourceLimits {
            max_member_bytes: args[3].parse()?,
        },
    )?;
    let paths: Vec<String> =
        serde_json::from_str(include_str!("../tests/fixtures/unsupported-tgi-paths.json"))?;
    let mut rows = Vec::new();
    for path in paths {
        let file = sources.read(&path)?;
        let result = Document::parse(file.bytes.clone());
        let (passed, error) = match result {
            Ok(d) => {
                assert_eq!(d.original_bytes(), file.bytes);
                (true, None)
            }
            Err(e) => (
                false,
                Some(serde_json::json!({"offset":e.offset,"message":e.message})),
            ),
        };
        rows.push(serde_json::json!({"path":path,"sha256":file.sha256,"bytes":file.bytes.len(),"now_parsed":passed,"error":error}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"scope":"Exact13 phase1 failures;1 narrow native-backed extension,12 explicitly unresolved AST/tolerance cases","sources":rows})
        )?
    );
    Ok(())
}
