//! Data every battle needs, loaded once per run and shared: the original
//! rules (GameData), the computer players' data, the random-map generator's
//! data, the asset store and the audited model fixes. `start` warms them on
//! background threads as soon as the game opens, so by the time the player
//! presses Launch in the menus a custom battle only has to generate its map
//! and place its objects. Owner: lead.
use aa_assets::AssetStore;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

type Shared<T> = OnceLock<std::result::Result<Arc<T>, String>>;

static STORE: Shared<AssetStore> = OnceLock::new();
static FIXES: Shared<crate::model_fixes::ModelFixes> = OnceLock::new();
static GAME: Shared<aa_play::GameData> = OnceLock::new();
static AI: Shared<aa_play::ai::AiData> = OnceLock::new();
static RANDOM: Shared<aa_play::mapgen::RandomMapData> = OnceLock::new();
// appended by the combat owner: the generals and their special operations
static GENERALS: Shared<aa_play::special_ops::GeneralData> = OnceLock::new();

fn get<T>(cell: &'static Shared<T>, init: impl FnOnce() -> Result<T>) -> Result<Arc<T>> {
    cell.get_or_init(|| init().map(Arc::new).map_err(|e| format!("{e:#}"))).clone().map_err(anyhow::Error::msg)
}

/// `reconstruction` is the converted-assets root; its parent is the install.
pub fn store(reconstruction: &Path) -> Result<Arc<AssetStore>> {
    get(&STORE, || Ok(AssetStore::open(reconstruction)?))
}
pub fn fixes(reconstruction: &Path) -> Result<Arc<crate::model_fixes::ModelFixes>> {
    let store = store(reconstruction)?;
    get(&FIXES, || crate::model_fixes::ModelFixes::discover(&store))
}
pub fn game_data(root: &Path) -> Result<Arc<aa_play::GameData>> {
    get(&GAME, || aa_play::GameData::load(root))
}
pub fn ai_data(root: &Path) -> Result<Arc<aa_play::ai::AiData>> {
    get(&AI, || aa_play::ai::AiData::load(root))
}
pub fn random_maps(root: &Path) -> Result<Arc<aa_play::mapgen::RandomMapData>> {
    get(&RANDOM, || aa_play::mapgen::RandomMapData::load(root))
}
/// The generals (Data/Generals) with their special operations and the SAI
/// GeneralPowers (needs the GameData; cached after the first load).
pub fn generals(root: &Path) -> Result<Arc<aa_play::special_ops::GeneralData>> {
    let data = game_data(root)?;
    get(&GENERALS, || aa_play::special_ops::GeneralData::load(root, &data))
}

// appended by the effects and sound owner: the effect tables (FX catalog,
// NIF effect table, texture array, rig bones) and the sound data (audio
// variables, sound file index, every WAV decoded)
static EFFECTS: Shared<crate::effects::Preloaded> = OnceLock::new();
static SOUNDS: Shared<crate::audio::SoundBank> = OnceLock::new();
static SOUNDS_REQUESTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// `root` is the install (the reconstruction folder's parent).
pub fn effects(root: &Path) -> Result<Arc<crate::effects::Preloaded>> {
    let data = game_data(root)?;
    get(&EFFECTS, || {
        let things: Vec<(String, Option<String>)> = data.defs.iter().map(|d| (d.id.clone(), d.model.clone())).collect();
        crate::effects::preload(&things)
    })
}
/// The effect data of a battle on a data folder: read once per folder in a run, and its
/// tables are cached on disk by what the folder holds.
pub fn effects_on(_root: &Path, folder: &Path, things: &[(String, Option<String>)]) -> Result<Arc<crate::effects::Preloaded>> {
    static ON: std::sync::Mutex<Vec<(std::path::PathBuf, Arc<crate::effects::Preloaded>)>> = std::sync::Mutex::new(Vec::new());
    let mut on = ON.lock().map_err(|_| anyhow::anyhow!("effect tables lock"))?;
    if let Some((_, loaded)) = on.iter().find(|(f, _)| f == folder) {
        return Ok(loaded.clone());
    }
    let loaded = Arc::new(crate::effects::preload_on(things, folder)?);
    on.push((folder.to_path_buf(), loaded.clone()));
    Ok(loaded)
}
pub fn sounds(reconstruction: &Path) -> Result<Arc<crate::audio::SoundBank>> {
    get(&SOUNDS, || crate::audio::SoundBank::load(reconstruction))
}
/// The effect data if its job has finished (never blocks).
pub fn effects_ready() -> Option<Arc<crate::effects::Preloaded>> {
    EFFECTS.get().and_then(|r| r.clone().ok())
}
/// The sound data if its job has finished (never blocks).
pub fn sounds_ready() -> Option<Arc<crate::audio::SoundBank>> {
    SOUNDS.get().and_then(|r| r.clone().ok())
}
/// Start the sound job if nothing has (a run that skipped `start`).
pub fn sounds_request(reconstruction: PathBuf) {
    if SOUNDS.get().is_none() && !SOUNDS_REQUESTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        std::thread::spawn(move || {
            let _ = sounds(&reconstruction);
        });
    }
}

// appended by the scene owner: every thing's View shadow type
static SHADOWS: Shared<ShadowTypes> = OnceLock::new();

/// A View's `shadow_type` as KKC_ObjectData::LoadData (0x49DC40) keeps it at
/// ObjectData+F8 (0 NONE, 1 TEXTURE, 2 PROJECTED, 3 STENCIL).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowType {
    None,
    Texture,
    Projected,
    Stencil,
}

/// Thing IDS (lower case) -> its View shadow type, with the template chain
/// resolved (`[template X Template=Y]`, `[Template X Inherit=Y]`, a thing's
/// `Template=`).
pub struct ShadowTypes(std::collections::HashMap<String, ShadowType>);

impl ShadowTypes {
    pub fn get(&self, ids: &str) -> Option<ShadowType> {
        self.0.get(&ids.to_ascii_lowercase()).copied()
    }
}

/// The shadow types of every thing in Data/**/*.tgi (the v14 runtime's
/// replacement files first). 0x49DC40: an unknown name keeps -1 (no
/// shadow); a View without a shadow_texture has none (NONE), and a NONE View
/// with a texture becomes TEXTURE.
pub fn shadow_types(reconstruction: &Path) -> Result<Arc<ShadowTypes>> {
    get(&SHADOWS, || {
        let install = reconstruction.parent().unwrap_or(reconstruction);
        let roots = [install.join("reconstruction/native-game/runtime-expanded-v14/Data"), reconstruction.join("assets/original/Data")];
        // relative path -> file, the first root winning
        let mut files: std::collections::BTreeMap<String, PathBuf> = std::collections::BTreeMap::new();
        for root in &roots {
            let mut stack = vec![root.clone()];
            while let Some(dir) = stack.pop() {
                let Ok(entries) = std::fs::read_dir(&dir) else { continue };
                for e in entries.flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("tgi"))
                        && let Ok(rel) = p.strip_prefix(root)
                    {
                        files.entry(rel.to_string_lossy().to_ascii_lowercase()).or_insert(p);
                    }
                }
            }
        }
        let sources: Vec<Vec<u8>> = files.values().filter_map(|path| std::fs::read(path).ok()).collect();
        Ok(ShadowTypes::parse(sources))
    })
}

impl ShadowTypes {
    /// The table from the TGI sources (one chain resolves across files).
    fn parse(sources: Vec<Vec<u8>>) -> Self {
        use aa_rules::tgi::{Document, Item, Limits, Node, TokenKind};
        // (parent template, shadow_type, shadow_texture) of templates and things
        type Entry = (Option<String>, Option<String>, Option<String>);
        let mut templates: std::collections::HashMap<String, Entry> = std::collections::HashMap::new();
        let mut things: Vec<(String, Entry)> = Vec::new();
        for bytes in sources {
            let Ok(doc) = Document::parse_native(bytes, Limits::default()) else { continue };
            for item in &doc.items {
                let Item::Node(node) = item else { continue };
                let atoms: Vec<&str> = node.header.tokens.iter().filter(|t| t.kind == TokenKind::Atom).filter_map(|t| doc.text(t.span)).collect();
                let Some(kind) = atoms.first() else { continue };
                // a header's parent: `Template=Y` or `Inherit=Y`, past the
                // kind and a template's own name
                let parent_after = |skip: usize| atoms.iter().skip(skip).zip(atoms.iter().skip(skip + 1)).find(|(k, _)| k.eq_ignore_ascii_case("template") || k.eq_ignore_ascii_case("inherit")).map(|(_, v)| v.to_ascii_lowercase());
                let view = |node: &Node| -> (Option<String>, Option<String>) {
                    let mut out = (None, None);
                    for item in &node.items {
                        let Item::Node(child) = item else { continue };
                        if !doc.header_name(&child.header).is_some_and(|h| h.eq_ignore_ascii_case("View")) {
                            continue;
                        }
                        for item in &child.items {
                            let Item::Property(p) = item else { continue };
                            let (Some(name), Some(value)) = (doc.property_name(p), doc.property_value(p)) else { continue };
                            let value = value.split(';').next().unwrap_or("").trim().trim_matches('"').to_owned();
                            if name.eq_ignore_ascii_case("shadow_type") {
                                out.0 = Some(value);
                            } else if name.eq_ignore_ascii_case("shadow_texture") {
                                out.1 = Some(value);
                            }
                        }
                    }
                    out
                };
                if kind.eq_ignore_ascii_case("template") {
                    let Some(name) = atoms.get(1) else { continue };
                    let (t, x) = view(node);
                    templates.insert(name.to_ascii_lowercase(), (parent_after(2), t, x));
                } else {
                    let ids = node.items.iter().find_map(|item| match item {
                        Item::Property(p) if doc.property_name(p).is_some_and(|n| n.eq_ignore_ascii_case("ids")) => doc.property_value(p).map(|v| v.trim().trim_matches('"').to_ascii_lowercase()),
                        _ => None,
                    });
                    if let Some(ids) = ids.filter(|s| !s.is_empty()) {
                        let (t, x) = view(node);
                        things.push((ids, (parent_after(1), t, x)));
                    }
                }
            }
        }
        let mut out = std::collections::HashMap::new();
        for (ids, (mut parent, mut kind, mut texture)) in things {
            let mut depth = 0;
            while (kind.is_none() || texture.is_none()) && depth < 32 {
                let Some(p) = parent.as_ref().and_then(|p| templates.get(p)) else { break };
                kind = kind.or_else(|| p.1.clone());
                texture = texture.or_else(|| p.2.clone());
                parent = p.0.clone();
                depth += 1;
            }
            let mut shadow = match kind.as_deref().map(str::to_ascii_uppercase).as_deref() {
                Some("NONE") => Some(ShadowType::None),
                Some("TEXTURE") => Some(ShadowType::Texture),
                Some("PROJECTED") => Some(ShadowType::Projected),
                Some("STENCIL") => Some(ShadowType::Stencil),
                _ => None,
            };
            let has_texture = texture.as_deref().is_some_and(|t| !t.is_empty() && !t.eq_ignore_ascii_case("null"));
            if !has_texture {
                shadow = shadow.map(|_| ShadowType::None);
            } else if shadow == Some(ShadowType::None) {
                shadow = Some(ShadowType::Texture);
            }
            out.insert(ids, shadow.unwrap_or(ShadowType::None));
        }
        ShadowTypes(out)
    }
}

/// Warm everything in parallel in the background (called once at startup).
pub fn start(reconstruction: PathBuf) {
    let Some(root) = reconstruction.parent().map(Path::to_path_buf) else { return };
    let started = std::time::Instant::now();
    let jobs: Vec<Box<dyn FnOnce() + Send>> = vec![
        {
            let r = reconstruction.clone();
            Box::new(move || {
                let _ = fixes(&r);
            })
        },
        {
            let root = root.clone();
            Box::new(move || {
                let _ = game_data(&root);
                let _ = generals(&root);
            })
        },
        {
            let root = root.clone();
            Box::new(move || {
                let _ = ai_data(&root);
            })
        },
        {
            let root = root.clone();
            Box::new(move || {
                let _ = random_maps(&root);
            })
        },
        // appended by the effects and sound owner
        {
            let root = root.clone();
            Box::new(move || {
                let _ = effects(&root);
            })
        },
        {
            let r = reconstruction.clone();
            Box::new(move || {
                SOUNDS_REQUESTED.store(true, std::sync::atomic::Ordering::SeqCst);
                let _ = sounds(&r);
            })
        },
        // appended by the scene owner
        {
            let r = reconstruction.clone();
            Box::new(move || {
                let _ = shadow_types(&r);
            })
        },
    ];
    std::thread::spawn(move || {
        std::thread::scope(|s| {
            for job in jobs {
                s.spawn(job);
            }
        });
        bevy::log::info!("preload: battle data ready in {:.0} ms", started.elapsed().as_secs_f64() * 1000.0);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(source: &str) -> ShadowTypes {
        ShadowTypes::parse(vec![source.as_bytes().to_vec()])
    }

    /// The field rules of 0x49DC40 over a template chain.
    #[test]
    fn shadow_types_follow_the_chain_and_the_native_rules() {
        let t = table(
            "[Template Object]\n{\n\t[View]\n\t{\n\t\tstring shadow_texture = null\n\t\tstring shadow_type = NONE\n\t}\n}\n\
             [Template Actor Inherit=Object]\n{\n}\n\
             [Template Building Inherit=Actor]\n{\n\t[View]\n\t{\n\t\tshadow_type = STENCIL\n\t\tshadow_texture = /effects/shadows/UnitShadow.dds\n\t}\n}\n\
             [template Hut Template=Building]\n{\n}\n\
             [Thing Template=Hut]\n{\n\tIDS = hut\n}\n\
             [Thing Template=Actor]\n{\n\tIDS = rock\n}\n\
             [Thing Template=Actor]\n{\n\tIDS = grass\n\t[View]\n\t{\n\t\tshadow_type = NONE\n\t}\n}\n\
             [Thing Template=Hut]\n{\n\tIDS = docks\n\t[View]\n\t{\n\t\tshadow_type = NONE\n\t}\n}\n\
             [Thing Template=Actor]\n{\n\tIDS = stone\n\t[View]\n\t{\n\t\tshadow_type = STENCIL\n\t}\n}\n",
        );
        // through Template= and Inherit= to the template that sets them
        assert_eq!(t.get("HUT"), Some(ShadowType::Stencil));
        // nothing set on the way: Object's NONE
        assert_eq!(t.get("rock"), Some(ShadowType::None));
        assert_eq!(t.get("grass"), Some(ShadowType::None));
        // NONE with an inherited texture becomes TEXTURE
        assert_eq!(t.get("docks"), Some(ShadowType::Texture));
        // a shadow without a texture has none
        assert_eq!(t.get("stone"), Some(ShadowType::None));
        assert_eq!(t.get("unknown"), None);
    }

    /// The shipped things, when the original data is present.
    #[test]
    fn shadow_types_of_the_shipped_things() {
        let Some(root) = crate::ui::reconstruction_root() else { return };
        let t = shadow_types(&root).unwrap();
        assert_eq!(t.get("temperate_bridge"), Some(ShadowType::Stencil));
        assert_eq!(t.get("arctic_grasstall"), Some(ShadowType::None));
    }
}
