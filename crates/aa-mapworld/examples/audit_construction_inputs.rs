//! Resolved source census; does not stand in for the native upgrade linker.
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
use std::path::PathBuf;
fn main() {
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut source = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 << 20,
        },
    )
    .unwrap();
    let catalog = ActorCatalog::load_v14(&mut source).unwrap();
    let mut construction = vec![];
    let mut errors = vec![];
    for actor in &catalog.actors {
        let result = (|| {
            let node = catalog.resolved_actor(&actor.id)?;
            if let Some(construction) = node.child("ConstructionComponent")? {
                let upgrades = construction.children("Upgrade")?;
                return Ok::<_, aa_rules::native::Error>(Some((
                    construction.sources(),
                    upgrades
                        .into_iter()
                        .map(|v| v.sources())
                        .collect::<Vec<_>>(),
                )));
            }
            Ok(None)
        })();
        match result {
            Ok(Some((sources, upgrades))) => construction
                .push(serde_json::json!({"id":actor.id,"sources":sources,"upgrades":upgrades})),
            Ok(None) => {}
            Err(error) => errors.push(serde_json::json!({"id":actor.id,"error":error.to_string()})),
        }
    }
    let costs:Vec<_>=["rus_corps","weurope_farm6x6_01"].into_iter().map(|id|{let input=catalog.native_economy_inputs(id,Precision::Double53);serde_json::json!({"id":id,"values":input.as_ref().ok(),"error":input.err().map(|e|e.to_string())})}).collect();
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"scope":"Source census of resolved Construction.Upgrade lists plus two actual null-modifier cost inputs; native linker proof is a separate dependency","actor_count":catalog.actors.len(),"sources":catalog.sources,"construction":construction,"errors":errors,"selected_costs":costs})).unwrap());
}
