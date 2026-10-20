//! Probe: the map's saved kingdom/team relation tables (ignored).
#[test]
#[ignore]
fn probe_relations() {
    let root = std::path::PathBuf::from(std::env::var("AA_GAME_ROOT").unwrap_or_else(|_| r"E:\Games\Axis and Alies".into()));
    for map in ["(2v2)Berlin", "DeveloperMaps/(3v3)Firefight"] {
        let bytes = std::fs::read(root.join(format!(r"reconstruction\assets\original\Data\Maps\{map}.RMP"))).unwrap();
        let archive = aa_mapworld::MapArchive::from_bytes(bytes).unwrap();
        let inv = aa_mapworld::WorldInventory::decode(&archive, 1 << 30).unwrap();
        eprintln!("== {map}");
        for (i, s) in inv.kingdoms.as_ref().unwrap().slots.iter().enumerate() {
            let Some(r) = &s.record else { continue };
            let p = &r.payload;
            eprintln!(
                "k{i} {} 188={} 18c={} 190={} team={:?} 1d0={:?} 1dc={:?}",
                String::from_utf16_lossy(&r.thing.name_utf16),
                p.word_188,
                p.word_18c,
                p.word_190,
                p.team_handle,
                p.words_1d0,
                p.words_1dc
            );
        }
        for (i, s) in inv.teams.as_ref().unwrap().slots.iter().enumerate() {
            let Some(r) = &s.record else { continue };
            eprintln!("t{i} {} serial={:#x} 4c={:?} 58={:?}", String::from_utf16_lossy(&r.thing.name_utf16), r.serialized_id, r.words_4c, r.words_58);
        }
    }
}
