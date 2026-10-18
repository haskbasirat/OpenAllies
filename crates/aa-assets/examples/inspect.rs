use aa_assets::{AssetStore, HeightMode};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let root = args
        .next()
        .ok_or("usage: inspect <reconstruction-root> [full-map-id]")?;
    let store = AssetStore::open(root)?;
    println!(
        "{} original map identities, {} converted static models",
        store.maps().len(),
        store.model_infos().len()
    );
    let id = args.next().unwrap_or_else(|| "Campaign/tutorial01".into());
    let scene = store.load_scene(&id, HeightMode::NativeConditional)?;
    println!(
        "{}: {}x{} cells; {} placements, {} renderable, {} unique models; {} warnings",
        id,
        scene.terrain.width,
        scene.terrain.height,
        scene.instances.len(),
        scene
            .instances
            .iter()
            .filter(|i| i.model_key.is_some())
            .count(),
        scene.models.len(),
        scene.warnings.len()
    );
    for warning in scene.warnings.iter().filter(|w| {
        w.message.starts_with("invalid")
            || w.message.starts_with("no converted")
            || w.message.starts_with("cannot read")
    }) {
        println!("{}: {}", warning.source, warning.message);
    }
    Ok(())
}
