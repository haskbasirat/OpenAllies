//! Lookup indexes equal the scans they replace, on the real installed data.
use crate::resource_map_cache::normalized_key;
use aa_rules::sources::{SourceLimits, V14Sources};

fn files() -> V14Sources {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits { max_member_bytes: 64 * 1024 * 1024 },
    )
    .unwrap()
}
fn utf16(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// `V14Sources::rows_with_lookup_key(&normalized_key(ids)?)` is the depot's
/// `catalog().into_iter().filter(normalized_key(row) == Ok(key))`.
#[test]
#[ignore = "requires actual AA_GAME_ROOT archive and v14 runtime"]
fn indexed_depot_rows_equal_the_full_catalog_filter() {
    let files = files();
    let rows = files.catalog();
    assert!(rows.len() > 1000, "{}", rows.len());
    for row in &rows {
        assert_eq!(
            V14Sources::lookup_key(&row.logical_path).ok_or(()),
            normalized_key(&utf16(&row.logical_path)).map_err(|_| ()),
            "{}",
            row.logical_path
        );
    }
    let mut queries: Vec<Vec<u16>> = Vec::new();
    for row in rows.iter().step_by(3) {
        queries.push(utf16(&row.logical_path));
        queries.push(utf16(&row.logical_path.to_ascii_uppercase()));
        queries.push(utf16(&row.logical_path.replace('/', "\\")));
        let mut cut = utf16(&row.logical_path);
        cut.push(0);
        cut.extend(utf16("ignored"));
        queries.push(cut);
    }
    queries.extend([utf16("missing/file.tga"), utf16(""), vec![0x00e9]]);
    let mut compared = 0;
    for ids in &queries {
        let Ok(key) = normalized_key(ids) else { continue };
        let full: Vec<_> = files
            .catalog()
            .into_iter()
            .filter(|v| normalized_key(&utf16(&v.logical_path)).as_ref() == Ok(&key))
            .map(|v| (v.logical_path, v.size, format!("{:?}", v.origin)))
            .collect();
        let fast: Vec<_> = files
            .rows_with_lookup_key(&key)
            .into_iter()
            .map(|v| (v.logical_path, v.size, format!("{:?}", v.origin)))
            .collect();
        assert_eq!(fast, full, "{key}");
        compared += 1;
    }
    assert!(compared > 1000, "{compared}");
}
