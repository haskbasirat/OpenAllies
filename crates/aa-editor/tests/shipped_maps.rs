//! Every map the original ships (`Data.rwd`, `*.RMP`) is read with our map
//! reader and written again with the editor's map file writer (the shared
//! chunk file writer of aa-metagame and the zlib 1.1.4 deflater of
//! aa-formats); the result must be the shipped file, byte for byte. This
//! proves the container layout, the record fields and the level-6 deflater
//! on 1,656 compressed chunks of real data.
use aa_editor::map_file::{rewrite_map, stale_flag_bits, write_map, ChunkSource};
use aa_mapworld::archive::Encoding;
use aa_mapworld::MapArchive;

#[test]
#[ignore = "requires AA_GAME_ROOT (the original game's Data.rwd)"]
fn every_shipped_map_re_encodes_byte_for_byte() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let mut rwd = aa_formats::rwd::RwdArchive::open(root.join("Data.rwd")).unwrap();
    let maps: Vec<(usize, String)> = rwd
        .entries()
        .iter()
        .enumerate()
        .filter(|(_, e)| e.path().to_ascii_lowercase().ends_with(".rmp"))
        .map(|(i, e)| (i, e.path().to_owned()))
        .collect();
    assert!(!maps.is_empty(), "no maps in Data.rwd");
    let (mut chunks, mut compressed, mut failures) = (0usize, 0usize, Vec::new());
    let mut stale_values = std::collections::BTreeSet::new();
    for (index, path) in &maps {
        let mut original = Vec::new();
        rwd.copy_entry(*index, &mut original).unwrap();
        let archive = MapArchive::from_bytes(original.clone()).unwrap();
        let decoded: Vec<_> = archive
            .directory()
            .iter()
            .map(|row| archive.decode_chunk(&row.name, u64::MAX).unwrap())
            .collect();
        // bits 1..63 of the flags word: one value shared by every record
        let stale = stale_flag_bits(&original).unwrap_or_else(|| panic!("{path}: records disagree in flag bits 1..63"));
        let sources: Vec<ChunkSource<'_>> = decoded
            .iter()
            .map(|c| {
                let flags = c.directory.source_timestamp;
                assert_eq!(c.encoding == Encoding::Zlib, flags & 1 != 0, "{path} {}: flag bit 0 vs encoding", c.directory.name);
                assert_eq!(flags & !1, stale, "{path} {}", c.directory.name);
                ChunkSource { name: &c.directory.name, compressed: flags & 1 != 0, payload: &c.bytes }
            })
            .collect();
        chunks += sources.len();
        compressed += sources.iter().filter(|s| s.compressed).count();
        stale_values.insert(stale);
        let written = write_map(path, &sources, stale).unwrap();
        assert!(rewrite_map(path, &original).unwrap() == written, "{path}: rewrite_map");
        if written != original {
            let at = written.iter().zip(&original).position(|(a, b)| a != b).unwrap_or(written.len().min(original.len()));
            failures.push(format!("{path}: first difference at byte {at} (written {}, shipped {})", written.len(), original.len()));
        }
    }
    eprintln!(
        "{} maps, {chunks} chunks ({compressed} compressed) re-encoded; {} distinct stale flag values",
        maps.len(),
        stale_values.len()
    );
    assert!(failures.is_empty(), "{} of {} maps differ:\n{}", failures.len(), maps.len(), failures.join("\n"));
}

/// The original's map save may hand the compressor many small writes rather
/// than one buffer (its write sizes are not traced; this is an assumption).
/// The streaming path (deflate with no flush on each piece) must give the
/// same bytes on the shipped chunks either way.
#[test]
#[ignore = "requires AA_GAME_ROOT (the original game's Data.rwd)"]
fn shipped_chunks_compress_the_same_in_small_writes() {
    use aa_editor::deflate::Deflater;
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let mut rwd = aa_formats::rwd::RwdArchive::open(root.join("Data.rwd")).unwrap();
    let maps: Vec<usize> = (0..rwd.entries().len())
        .filter(|&i| rwd.entries()[i].path().to_ascii_lowercase().ends_with(".rmp"))
        .collect();
    let pieces = [4usize, 1, 2, 4, 8, 3, 4, 300, 4, 2, 64, 4];
    let (mut chunks, mut failures) = (0usize, Vec::new());
    for index in maps {
        let mut original = Vec::new();
        rwd.copy_entry(index, &mut original).unwrap();
        let path = rwd.entries()[index].path().to_owned();
        let archive = MapArchive::from_bytes(original.clone()).unwrap();
        for row in archive.directory() {
            let chunk = archive.decode_chunk(&row.name, u64::MAX).unwrap();
            if chunk.encoding != Encoding::Zlib {
                continue;
            }
            let shipped = &original[row.packed_offset as usize..(row.packed_offset + row.packed_bytes) as usize];
            let mut z = Deflater::new(6).unwrap();
            let (mut at, mut k) = (0, 0);
            while at < chunk.bytes.len() {
                let n = pieces[k % pieces.len()].min(chunk.bytes.len() - at);
                z.write(&chunk.bytes[at..at + n]);
                at += n;
                k += 1;
            }
            chunks += 1;
            if z.finish() != shipped {
                failures.push(format!("{path} {}", row.name));
            }
        }
    }
    eprintln!("{chunks} compressed chunks written in small pieces");
    assert!(failures.is_empty(), "{} chunks differ:\n{}", failures.len(), failures.join("\n"));
}
