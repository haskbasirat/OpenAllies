//! The film writer against the original's own bytes: every chunk of every real
//! film, decoded and written again, is the same chunk.
//!
//! ```text
//! AA_GAME_ROOT=<game root> cargo test -p aa-parity --release --test film_writer -- --ignored --nocapture
//! ```
//! Reads the QA films, and every film of `AA_FILM_CORPUS=<dir>` when set
//! (`target/film-corpus/films`), each with the registries it decodes on (v14
//! here; a film of another data set is skipped by name, its orders do not decode).
use aa_mapworld::MapArchive;
use aa_orders::payload::Field;
use aa_parity::film::{decode_film_header, decode_session_players, split_orders};
use aa_parity::film_writer::{self, OrdersWriter, WireDecree};
use aa_parity::registries::Registries;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const QA: &str = "reconstruction/qa-runs/compare-20261022/game/Profile/data/Films";

fn root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"))
}

fn films(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != "rejected") {
                films(&path, out);
            }
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(film_writer::FILM_EXTENSION))
        {
            out.push(path);
        }
    }
}

/// Decode one packet into decrees, without the typed layer.
fn decrees(body: &[u8], registries: &Registries) -> Result<Vec<WireDecree>, String> {
    let registry = aa_orders::native::registry().map_err(|e| e.to_string())?;
    let packet = aa_orders::Packet::new(body.to_vec()).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    let mut gate = aa_orders::Gate::default();
    aa_orders::dispatch(
        Some(&packet),
        None,
        &registry,
        false,
        &mut gate,
        |descriptor, cursor, _| {
            let fields: Vec<Field> = aa_orders::payload::decode(descriptor, cursor, registries)
                .map_err(|e| e.to_string())?;
            out.push(WireDecree { descriptor, fields });
            Ok(())
        },
    )
    .map_err(|e| e.to_string())?;
    Ok(out)
}

#[derive(Default)]
struct Totals {
    films: usize,
    headers: usize,
    players: usize,
    records: usize,
    skipped_orders: usize,
    no_directory: usize,
    decrees: BTreeMap<usize, usize>,
    /// 86 hq_attach (31) with the scalar flag: this port's marker.
    hq_attach_with_scalar: usize,
    hq_attach: usize,
}

fn check(path: &Path, registries: &Registries, totals: &mut Totals) {
    let bytes = std::fs::read(path).unwrap();
    let archive = MapArchive::from_bytes(bytes).unwrap();
    if archive.directory().is_empty() {
        totals.no_directory += 1;
        return;
    }
    totals.films += 1;
    // MAP_HEADER, byte for byte.
    let chunk = archive.decode_chunk("MAP_HEADER", 1 << 30).unwrap();
    let (header, creator) = decode_film_header(&chunk).unwrap();
    assert_eq!(
        film_writer::map_header_bytes(&header, &creator),
        chunk.bytes,
        "MAP_HEADER of {}",
        path.display()
    );
    totals.headers += 1;
    // SessionPlayers, byte for byte. A name can hold units a String keeps.
    let chunk = archive.decode_chunk("SessionPlayers", 1 << 30).unwrap();
    let players = decode_session_players(&chunk.bytes).unwrap();
    assert_eq!(
        film_writer::session_players_bytes(&players),
        chunk.bytes,
        "SessionPlayers of {}",
        path.display()
    );
    totals.players += 1;
    // Orders, record by record: the same fields make the same packet.
    let chunk = archive.decode_chunk("Orders", 1 << 30).unwrap();
    let records = split_orders(&chunk.bytes).unwrap();
    let mut decoded = Vec::with_capacity(records.len());
    for body in &records {
        match decrees(body, registries) {
            Ok(d) => decoded.push(d),
            Err(_) => {
                totals.skipped_orders += 1;
                return;
            }
        }
    }
    let mut writer = OrdersWriter::new().unwrap();
    let mut rebuilt = Vec::with_capacity(chunk.bytes.len());
    for (record, decrees) in decoded.iter().enumerate() {
        for d in decrees {
            *totals.decrees.entry(d.descriptor).or_default() += 1;
            if let (86, Some(Field::Bounded { value: flags, .. }), Some(Field::Def { index: 31, .. })) = (d.descriptor, d.fields.first(), d.fields.get(1)) {
                totals.hq_attach += 1;
                totals.hq_attach_with_scalar += usize::from(flags & 0x10 != 0);
            }
            writer.decree(d, registries).unwrap_or_else(|e| {
                panic!("{} record {record}: {e}", path.display());
            });
        }
        rebuilt.extend(writer.record().unwrap());
        totals.records += 1;
    }
    assert_eq!(rebuilt, chunk.bytes, "Orders of {}", path.display());
}

/// Every film of the QA copy and of the corpus: MAP_HEADER, SessionPlayers and
/// the Orders chunk come out as the original wrote them.
#[test]
#[ignore = "reads the original data and the real films; set AA_GAME_ROOT"]
fn chunks_of_real_films_are_written_back_byte_for_byte() {
    let root = root();
    let registries = Registries::load_v14(&root, true).unwrap();
    let mut paths = Vec::new();
    films(&root.join(QA), &mut paths);
    if let Some(dir) = std::env::var_os("AA_FILM_CORPUS") {
        films(Path::new(&dir), &mut paths);
    }
    assert!(!paths.is_empty());
    let mut totals = Totals::default();
    for path in &paths {
        check(path, &registries, &mut totals);
    }
    println!(
        "{} films with a directory, {} without; MAP_HEADER equal in {}, SessionPlayers in {}; \
         {} records equal; {} films' orders not on v14 (not compared)",
        totals.films,
        totals.no_directory,
        totals.headers,
        totals.players,
        totals.records,
        totals.skipped_orders
    );
    println!("decrees written back, by descriptor: {:?}", totals.decrees);
    println!("hq_attach decrees: {}, with the scalar flag: {}", totals.hq_attach, totals.hq_attach_with_scalar);
    assert_eq!(totals.hq_attach_with_scalar, 0, "the Attach marker of aa-play's wire must not occur in a real film");
    // The pause and the speed do not reach a film, which is why the recorder
    // writes neither.
    for timeline in [73, 75, 76, 80, 81] {
        assert_eq!(totals.decrees.get(&timeline), None, "a real film with decree {timeline}");
    }
    assert_eq!(totals.headers, totals.films);
    assert_eq!(totals.players, totals.films);
    assert!(totals.records > 0);
}

/// A chunk file from [`film_writer::FilmFile`] reads back through the film
/// reader's container, with the same chunks in the native order.
#[test]
fn film_file_is_a_chunk_file_of_the_native_order() {
    let dir = std::env::temp_dir().join(format!("aa-film-writer-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("t.RFM");
    let mut file = film_writer::FilmFile::create(&path, b"header", b"strings", b"players").unwrap();
    // Open: the bare chunk file header, no directory yet.
    assert_eq!(std::fs::read(&path).unwrap().len(), 12);
    file.write_record(&[1, 0, 0, 0, 5]).unwrap();
    let written = file.close().unwrap();
    let archive = MapArchive::from_bytes(std::fs::read(&written).unwrap()).unwrap();
    let names: Vec<String> = archive.directory().iter().map(|r| r.name.clone()).collect();
    assert_eq!(
        names,
        ["MAP", "MAP_HEADER", "MAPSTRINGS", "SessionPlayers", "Orders"]
    );
    assert_eq!(archive.decode_chunk("MAP", 1 << 20).unwrap().bytes, [0; 4]);
    assert_eq!(
        archive.decode_chunk("Orders", 1 << 20).unwrap().bytes,
        [1, 0, 0, 0, 5]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
