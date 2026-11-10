//! Every map the original ships (`Data.rwd`, `*.RMP`) is read with our map
//! reader and written again with the editor's map file writer; the result
//! must be the shipped file, byte for byte. This proves the container
//! layout, the record fields and the zlib 1.1.4 level-6 deflater on 1,656
//! compressed chunks of real data.
use aa_editor::map_file::{write_map, ChunkSource};
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
    for (index, path) in &maps {
        let mut original = Vec::new();
        rwd.copy_entry(*index, &mut original).unwrap();
        let archive = MapArchive::from_bytes(original.clone()).unwrap();
        let decoded: Vec<_> = archive
            .directory()
            .iter()
            .map(|row| archive.decode_chunk(&row.name, u64::MAX).unwrap())
            .collect();
        let sources: Vec<ChunkSource<'_>> = decoded
            .iter()
            .map(|c| {
                let flags = c.directory.source_timestamp;
                assert_eq!(c.encoding == Encoding::Zlib, flags & 1 != 0, "{path} {}: flag bit 0 vs encoding", c.directory.name);
                ChunkSource { name: &c.directory.name, flags, payload: &c.bytes }
            })
            .collect();
        chunks += sources.len();
        compressed += sources.iter().filter(|s| s.flags & 1 != 0).count();
        let written = write_map(&sources).unwrap();
        if written != original {
            let at = written.iter().zip(&original).position(|(a, b)| a != b).unwrap_or(written.len().min(original.len()));
            failures.push(format!("{path}: first difference at byte {at} (written {}, shipped {})", written.len(), original.len()));
        }
    }
    eprintln!("{} maps, {chunks} chunks ({compressed} compressed) re-encoded", maps.len());
    assert!(failures.is_empty(), "{} of {} maps differ:\n{}", failures.len(), maps.len(), failures.join("\n"));
}

/// The original's map save hands the compressor many small writes (one per
/// saved field) rather than one buffer. The streaming path (deflate with no
/// flush on each piece) must give the same bytes on the shipped chunks.
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
