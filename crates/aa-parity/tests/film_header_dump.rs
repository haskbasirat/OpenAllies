//! Prints what a film's MAP_HEADER holds, for reading the preview's fields
//! against the original's dialog.
//!
//! `AA_FILM=<path> cargo test -p aa-parity --release --test film_header_dump -- --ignored --nocapture`
use aa_parity::film::decode_film_header;

fn text(v: &[u16]) -> String {
    String::from_utf16_lossy(v)
}

#[test]
#[ignore = "reads a film named by AA_FILM"]
fn dump() {
    let path = std::env::var("AA_FILM").expect("AA_FILM");
    for path in path.split(';') {
        let bytes = std::fs::read(path).unwrap();
        let archive = aa_mapworld::MapArchive::from_bytes(bytes).unwrap();
        let chunk = archive.decode_chunk("MAP_HEADER", 1 << 30).unwrap();
        let (h, derived) = decode_film_header(&chunk).unwrap();
        println!("film {path}");
        println!("  version {} modes {} / {}", h.version, h.rules.mode, h.creator.rules.mode);
        println!("  words_0c_10 {:?}", h.words_0c_10);
        println!("  name_key {:?} description_key {:?}", text(&h.name_key), text(&h.description_key));
        println!(
            "  display {:?} word_50 {} word_54 {:#x}",
            h.display_extents_bits.map(|b| b.map(f32::from_bits)),
            f32::from_bits(h.word_50),
            h.word_54
        );
        println!(
            "  string_58 {:?} 5c {:?} 60 {:?} byte_64 {}",
            text(&h.string_58),
            text(&h.string_5c),
            text(&h.string_60),
            h.byte_64
        );
        println!("  string_38 {:?} 3c {:?} word_40 {:?}", text(&h.string_38), text(&h.string_3c), h.word_40);
        println!("  words_18_1c {:#x?}", h.words_18_1c);
        println!("  entries {}", h.entries.len());
        println!("  creator {:?} extents {:?}", text(&h.creator.class_name), h.creator.world_extents_bits.map(f32::from_bits));
        println!("  derived {:?}", match &derived {
            aa_mapworld::header::DerivedCreator::Base => "base".to_owned(),
            aa_mapworld::header::DerivedCreator::Load { source_resource_ids, .. } => format!("load {}", text(source_resource_ids)),
            aa_mapworld::header::DerivedCreator::Random(s) => format!(
                "random {} weights {:?} total {} objects {:?} heroes {}",
                text(&s.rmc),
                s.terrain_weights.iter().map(|(i, b)| (text(i), f32::from_bits(*b))).collect::<Vec<_>>(),
                f32::from_bits(s.total_terrain_weight_bits),
                s.object_counts.iter().map(|(i, c)| (text(i), *c)).collect::<Vec<_>>(),
                s.num_heroes
            ),
        });
        for (label, parameters) in [("header", &h.parameters), ("creator", &h.creator.parameters)] {
            println!("  {label} teams {:?}", parameters.teams.iter().map(|t| (text(&t.id), text(&t.display_key))).collect::<Vec<_>>());
            for k in &parameters.kingdoms {
                println!(
                    "  {label} kingdom id {:?} ind {} nation {:?} faction {:?} color {:?} handicap {:?} display {:?} 0c {:?} 2c {:?} b29 {} b28 {} team {:?} defs {} 58 {:?} props {} b31 {} b32 {} res {}",
                    text(&k.id),
                    k.byte_10,
                    text(&k.nation),
                    text(&k.faction),
                    text(&k.color),
                    text(&k.handicap),
                    text(&k.display_key),
                    k.string_0c.as_deref().map(text),
                    text(&k.string_2c),
                    k.byte_29,
                    k.byte_28,
                    text(&k.string_14),
                    k.definitions.len(),
                    text(&k.string_58),
                    k.properties.len(),
                    k.byte_31,
                    k.byte_32,
                    k.resources.len()
                );
            }
        }
    }
}
