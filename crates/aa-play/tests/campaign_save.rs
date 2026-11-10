//! Every saved World War II campaign of the original on this computer, each army
//! record walked to its end with its Mind (aa-mind's 536340 / 528C20 readers): see
//! `aa_mapworld::campaign_save`. The files are read, never written; with none on
//! this computer the test says so and passes.
use aa_mapworld::campaign_save::{self, MODE_SAVED_CAMPAIGN};
use aa_mapworld::saved_world::MindDecoder;
use aa_mind::construct::Kind;
use std::path::{Path, PathBuf};

const FOLDER: &str = "DynamicCampaignSave";

/// The concrete Mind 0x4CF480 makes for a component mask (bit 7, then 23, else
/// the Element's), as the saved-game reader of this crate selects it.
struct Minds;
impl MindDecoder for Minds {
    fn decode_mind(&mut self, mask: u32, bytes: &[u8]) -> aa_mapworld::Result<(usize, Option<i32>)> {
        let kind = if mask & (1 << 7) != 0 {
            Kind::Regiment
        } else if mask & (1 << 23) != 0 {
            Kind::Aircraft
        } else {
            Kind::Element
        };
        let (saved, consumed) = aa_mind::saved_load::decode(kind, bytes).map_err(|e| aa_mapworld::Error::Format(format!("{kind:?}: {e}")))?;
        Ok((consumed, saved.updatable().and_then(|u| u.serialized_id)))
    }
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

fn original_saves() -> Vec<PathBuf> {
    let root = std::env::var_os("AA_GAME_ROOT").map(PathBuf::from).unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.."));
    let mut out = Vec::new();
    if let Some(home) = std::env::var_os("USERPROFILE") {
        saves_in(&Path::new(&home).join("Documents").join("A&A").join("data").join(FOLDER), &mut out);
    }
    if let Ok(rd) = std::fs::read_dir(root.join("reconstruction").join("qa-runs")) {
        for e in rd.flatten() {
            saves_in(&e.path().join("game").join("Profile").join("data").join(FOLDER), &mut out);
        }
    }
    out.sort();
    out
}

#[test]
fn every_army_record_of_the_original_campaign_saves_is_walked_whole() {
    let saves = original_saves();
    if saves.is_empty() {
        eprintln!("no original World War II campaign save on this computer");
        return;
    }
    for path in saves {
        let bytes = std::fs::read(&path).unwrap();
        let save = campaign_save::read(bytes, 64 << 20, Some(&mut Minds)).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(save.mode, MODE_SAVED_CAMPAIGN);
        assert!(!save.armies.is_empty());
        assert!(save.armies.iter().all(|a| a.walked_to_end), "{}", path.display());
        eprintln!("{}: {} armies walked whole", path.file_name().unwrap().to_string_lossy(), save.armies.len());
    }
}
