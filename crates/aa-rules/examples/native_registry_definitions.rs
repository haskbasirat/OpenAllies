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
    let mut rows = Vec::new();
    for n in c.resolved_definitions(&a[3])? {
        let id = n.field("IDS", Precision::Double53)?;
        let mut children = Vec::new();
        if let Some(name) = a.get(4) {
            for child in n.children(name)? {
                let mut fields = serde_json::Map::new();
                for field in &a[5..] {
                    fields.insert(
                        field.clone(),
                        serde_json::to_value(child.field(field, Precision::Double53)?)?,
                    );
                }
                children.push(serde_json::json!({"sources":child.sources(),"fields":fields}));
            }
        }
        rows.push(serde_json::json!({"id":id,"sources":n.sources(),"children":children}));
    }
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}
