//! Custom game (`RequestCustom`) staging data: the definition registries and
//! the selected map's creator rows the StagingMenu reads, loaded from the
//! same v14 sources as the battle startup.

#[cfg(test)]
mod probe {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
    };
    fn root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).unwrap().join("reconstruction")
    }
    #[test]
    #[ignore = "requires installed original/v14 data"]
    fn probe_registries_and_berlin_rows() {
        let root = root();
        let t = std::time::Instant::now();
        let mut files = V14Sources::open(
            root.parent().unwrap().join("Data.rwd"),
            root.join("native-game/runtime-expanded-v14"),
            SourceLimits { max_member_bytes: u64::MAX },
        )
        .unwrap();
        eprintln!("open {:?}", t.elapsed());
        let t = std::time::Instant::now();
        let actors = ActorCatalog::load_v14(&mut files).unwrap();
        eprintln!("actors {:?}", t.elapsed());
        for m in ["Nation", "Faction", "Handicap", "SAI", "AIHandicap", "Team"] {
            match actors.native_data_registry(m) {
                Some(r) => eprintln!("{m}: {:?}", r.entries.iter().map(|e| e.ids.clone()).collect::<Vec<_>>()),
                None => eprintln!("{m}: absent"),
            }
        }
        let map = files.read("Maps/(2v2)Berlin.RMP").unwrap();
        let archive = aa_mapworld::archive::MapArchive::from_bytes(map.bytes.clone()).unwrap();
        let header = aa_mapworld::header::MapHeader::decode(&archive.decode_chunk("MAP_HEADER", u64::MAX).unwrap()).unwrap();
        let s = |v: &[u16]| String::from_utf16_lossy(v);
        eprintln!("name_key {} teams {:?}", s(&header.name_key), header.parameters.teams.iter().map(|t| (s(&t.id), s(&t.display_key))).collect::<Vec<_>>());
        for k in &header.parameters.kingdoms {
            eprintln!(
                "kingdom {} b10={} nation={} faction={} color={} handicap={} display={} s0c={:?} s2c={} b29={} b28={} team={} s58={} b31={} b32={}",
                s(&k.id), k.byte_10, s(&k.nation), s(&k.faction), s(&k.color), s(&k.handicap), s(&k.display_key),
                k.string_0c.as_deref().map(s), s(&k.string_2c), k.byte_29, k.byte_28, s(&k.string_14), s(&k.string_58), k.byte_31, k.byte_32
            );
        }
        let c = &header.creator.parameters;
        for k in &c.kingdoms {
            eprintln!("creator {} b10={} nation={} faction={} handicap={} b29={} b28={} team={}", s(&k.id), k.byte_10, s(&k.nation), s(&k.faction), s(&k.handicap), k.byte_29, k.byte_28, s(&k.string_14));
        }
    }
}
