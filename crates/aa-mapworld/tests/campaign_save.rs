//! Every saved World War II campaign of the original on this computer, read whole
//! (read-only): the original's own profile (`<personal folder>/A&A/data/` + AVar
//! PathDynamicCampaignSavedGames) and the profiles of the hidden-desktop test copies
//! under `<game root>/reconstruction/qa-runs/*/game/Profile/data/`. With no such file
//! the test says so and passes: the files are the player's, not part of the repo.
//!
//! No Mind decoder is given here (aa-mind sits above this crate), so each army
//! record is walked up to the end of its MetaGame component.
use aa_mapworld::campaign_save::{self, METAGAME_SLOT, MODE_SAVED_CAMPAIGN};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const FOLDER: &str = "DynamicCampaignSave";

fn game_root() -> PathBuf {
    std::env::var_os("AA_GAME_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.."))
}

fn saves_in(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("rsg")) {
            out.push(p);
        }
    }
}

/// The original campaign saves this computer holds.
pub fn original_saves() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(home) = std::env::var_os("USERPROFILE") {
        saves_in(&Path::new(&home).join("Documents").join("A&A").join("data").join(FOLDER), &mut out);
    }
    if let Ok(rd) = std::fs::read_dir(game_root().join("reconstruction").join("qa-runs")) {
        for e in rd.flatten() {
            saves_in(&e.path().join("game").join("Profile").join("data").join(FOLDER), &mut out);
        }
    }
    out.sort();
    out
}

#[test]
fn reads_every_original_campaign_save() {
    let saves = original_saves();
    if saves.is_empty() {
        eprintln!("no original World War II campaign save on this computer");
        return;
    }
    for path in saves {
        let bytes = std::fs::read(&path).unwrap();
        let save = campaign_save::read(bytes, 64 << 20, None).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(save.mode, MODE_SAVED_CAMPAIGN, "{}", path.display());
        assert!(!save.metagame.is_empty());
        assert!(!save.armies.is_empty(), "{}: no army", path.display());
        let mut handles = BTreeSet::new();
        for a in &save.armies {
            assert_eq!(campaign_save::component_length(&a.component).unwrap(), a.component.len());
            assert!(handles.insert(a.serialized_id), "army handle {:#x} twice", a.serialized_id);
        }
        eprintln!(
            "{}: METAGAME {} bytes, {} things, {} armies (slot {METAGAME_SLOT})",
            path.file_name().unwrap().to_string_lossy(),
            save.metagame.len(),
            save.world.dynamic_things.placements.len(),
            save.armies.len()
        );
    }
}
