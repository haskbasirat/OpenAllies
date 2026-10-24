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
