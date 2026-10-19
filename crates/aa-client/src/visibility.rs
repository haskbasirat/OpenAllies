//! Authored initial visibility for the map preview, not runtime reveal/fog logic.
//! Native source templates give audio emitters an inspectable speaker model and
//! Flags.invisible=true. Preserve their source records, but do not draw/pick them.
use aa_assets::{AssetStore, PreparedScene};
use aa_resources::MemoryPolicy;
use aa_rules::{
    actors::{ActorCatalog, FieldResolution},
    resources::Provenance,
};
use anyhow::{Result, ensure};
use std::{collections::BTreeMap, path::PathBuf, sync::{Arc, OnceLock}};

#[derive(Clone)]
struct AuthoredVisibility {
    invisible: bool,
    source: Provenance,
}

struct Catalog {
    resolved: BTreeMap<String, AuthoredVisibility>,
    unresolved: BTreeMap<String, String>,
}

impl Catalog {
    fn from_actors(actors: ActorCatalog) -> Self {
        let mut catalog = Self {
            resolved: BTreeMap::new(),
            unresolved: BTreeMap::new(),
        };
        for actor in actors.actors {
            let id = actor.id.to_ascii_lowercase();
            match actor.field("Flags.invisible") {
                Some(FieldResolution::Value { raw, source }) => {
                    let value = match raw.as_slice() {
                        b"true" => Some(true),
                        b"false" => Some(false),
                        _ => None,
                    };
                    if let Some(invisible) = value {
                        catalog.resolved.insert(
                            id,
                            AuthoredVisibility {
                                invisible,
                                source: source.clone(),
                            },
                        );
                    } else {
                        catalog
                            .unresolved
                            .insert(id, "non-literal visibility flag".into());
                    }
                }
                Some(FieldResolution::Unsupported { reason }) => {
                    catalog.unresolved.insert(id, reason.clone());
                }
                _ => {
                    catalog
                        .unresolved
                        .insert(id, "missing authored visibility flag".into());
                }
            }
        }
        catalog
    }

    fn apply(&self, scene: &mut PreparedScene) -> Stats {
        let mut stats = Stats::default();
        for instance in &mut scene.instances {
            let id = instance.thing_id.to_ascii_lowercase();
            match self.resolved.get(&id) {
                Some(authored) if authored.invisible => {
                    if instance.model_key.take().is_some() {
                        stats.hidden_instances += 1;
                    }
                    stats
                        .sources
                        .insert(authored.source.path.clone(), authored.source.sha256.clone());
                }
                Some(_) => {}
                None => {
                    stats.unresolved_ids.insert(
                        id,
                        self.unresolved
                            .get(&instance.thing_id.to_ascii_lowercase())
                            .cloned()
                            .unwrap_or_else(|| "Thing definition unavailable".into()),
                    );
                }
            }
        }
        stats
    }
}

#[derive(Default)]
pub struct Stats {
    pub hidden_instances: usize,
    pub unresolved_ids: BTreeMap<String, String>,
    pub sources: BTreeMap<String, String>,
}

pub struct Visibility {
    root: PathBuf,
    catalog: Arc<OnceLock<std::result::Result<Catalog, String>>>,
}

impl Visibility {
    pub fn new(store: &AssetStore) -> Self {
        Self {
            root: store.root().to_owned(),
            catalog: Arc::new(OnceLock::new()),
        }
    }
    /// Start the same one-time catalog resolution `apply` performs (same
    /// root, same policy, same sources) on a background thread, so a later
    /// `apply` finds it done or waits for it instead of loading on its caller.
    pub fn prefetch(&self, policy: MemoryPolicy) {
        if self.catalog.get().is_some() {
            return;
        }
        let (root, catalog) = (self.root.clone(), Arc::clone(&self.catalog));
        let _ = std::thread::Builder::new()
            .name("authored-visibility".into())
            .spawn(move || {
                catalog.get_or_init(|| resolve(&root, policy));
            });
    }

    /// Resolve once per loader from source-hashed original/v14 definitions. A
    /// model-only browser does not incur rule loading or hide its speaker model.
    /// Failed rule loading is returned to the caller, never guessed by filename.
    pub fn apply(
        &self,
        store: &AssetStore,
        policy: MemoryPolicy,
        scene: &mut PreparedScene,
    ) -> Result<Stats> {
        ensure!(
            self.root == store.root(),
            "Visibility catalog belongs to another asset store"
        );
        let catalog = self
            .catalog
            .get_or_init(|| resolve(store.root(), policy))
            .as_ref()
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(catalog.apply(scene))
    }
}
fn resolve(root: &std::path::Path, policy: MemoryPolicy) -> std::result::Result<Catalog, String> {
    crate::authored_rules::load(root, policy)
        .map(Catalog::from_actors)
        .map_err(|e| format!("{e:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_rules::{
        sources::{Origin, ResolvedFile, sha256},
        tgi::Limits,
    };

    fn catalog(text: &str) -> Catalog {
        Catalog::from_actors(ActorCatalog::from_files(
            vec![ResolvedFile {
                logical_path: "visibility-fixture.tgi".into(),
                origin: Origin::V14Loose,
                sha256: sha256(text.as_bytes()),
                bytes: text.as_bytes().to_vec(),
            }],
            Limits::default(),
        ))
    }

    #[test]
    fn exact_source_boolean_inherits_but_explicit_false_overrides_it() {
        let text = "[Template Base]{\n [Flags]\n invisible=false\n}\n[Template Audio Template=Base]{\n [Flags]\n invisible=true\n}\n[Thing Template=Audio]{\n IDS=birds\n}\n[Thing Template=Audio]{\n IDS=visible\n [Flags]\n invisible=false\n}\n[Thing Template=Base]{\n IDS=dynamic\n [Flags]\n invisible=$expression\n}\n";
        let c = catalog(text);
        assert!(c.resolved["birds"].invisible);
        assert!(!c.resolved["visible"].invisible);
        let source = &c.resolved["birds"].source;
        assert_eq!(&text[source.span.start..source.span.end], "invisible=true");
        assert_eq!(source.sha256, sha256(text.as_bytes()));
        assert!(!c.resolved.contains_key("dynamic"));
        assert!(c.unresolved.contains_key("dynamic"));
    }

    #[test]
    fn ambiguous_or_unresolved_actor_definitions_do_not_hide_geometry() {
        let c = catalog(
            "[Thing]{\n IDS=duplicate\n [Flags]\n invisible=true\n}\n[Thing]{\n IDS=duplicate\n [Flags]\n invisible=false\n}\n[Thing Template=Unknown]{\n IDS=missing\n}\n",
        );
        for id in ["duplicate", "missing"] {
            assert!(!c.resolved.contains_key(id));
            assert!(c.unresolved.contains_key(id));
        }
    }

    #[test]
    #[ignore = "requires hash-pinned original/v14 rules and recovered Crete assets"]
    fn original_crete_visibility_preserves_records_models_and_visible_instances() {
        let game = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let store = AssetStore::open(game.join("reconstruction")).unwrap();
        let mut scene = store
            .load_scene(
                "Campaign/axis01_crete",
                aa_assets::HeightMode::NativeConditional,
            )
            .unwrap();
        let before = scene.clone();
        let filter = Visibility::new(&store);
        let started = std::time::Instant::now();
        let stats = filter
            .apply(&store, MemoryPolicy::default(), &mut scene)
            .unwrap();
        eprintln!(
            "Crete visibility: {} hidden, {} unresolved, first load {:.3}s",
            stats.hidden_instances,
            stats.unresolved_ids.len(),
            started.elapsed().as_secs_f64()
        );
        assert_eq!(stats.hidden_instances, 324);
        assert!(
            stats.unresolved_ids.is_empty(),
            "{:?}",
            stats.unresolved_ids
        );
        assert_eq!(scene.map, before.map);
        assert_eq!(scene.models, before.models);
        assert_eq!(scene.terrain, before.terrain);
        let catalog = filter.catalog.get().unwrap().as_ref().unwrap();
        for (now, old) in scene.instances.iter().zip(&before.instances) {
            let authored = &catalog.resolved[&now.thing_id.to_ascii_lowercase()];
            let mut expected = old.clone();
            if authored.invisible {
                expected.model_key = None;
            }
            assert_eq!(*now, expected);
        }
        let second = filter
            .apply(&store, MemoryPolicy::default(), &mut scene)
            .unwrap();
        assert_eq!(second.hidden_instances, 0);
        assert!(catalog.resolved["birds"].invisible);
        assert!(!catalog.resolved["gbr_corps"].invisible);
    }
}
