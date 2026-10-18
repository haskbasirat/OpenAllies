use aa_formats::rwd::normalized_member_path;
use aa_formats::{ArchiveError, RwdArchive};
use std::collections::BTreeMap;
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const REAL: &[u8] = include_bytes!("fixtures/avars-version.original.tgi");
const HEADER: &[u8] = include_bytes!("fixtures/data-header.original.bin");

fn record(name: &str, offset: u64, size: u64) -> Vec<u8> {
    let mut out = vec![0; 64];
    for (i, word) in name.encode_utf16().enumerate() {
        out[i * 2..i * 2 + 2].copy_from_slice(&word.to_le_bytes());
    }
    for value in [offset, size, 5_346_237_856_730_520, size] {
        out.extend(value.to_le_bytes());
    }
    out
}
fn fixture(members: &[(&str, &[u8])], trailing: bool) -> Vec<u8> {
    let mut out = b"TGCK\x02\0\0\0\x03\0\0\0".to_vec();
    out.extend(HEADER);
    let files_at = out.len() as u64;
    let mut footer = (members.len() as u32).to_le_bytes().to_vec();
    for (i, (name, body)) in members.iter().enumerate() {
        let words: Vec<_> = name.encode_utf16().collect();
        footer.extend((words.len() as u16).to_le_bytes());
        for word in words {
            footer.extend(word.to_le_bytes());
        }
        footer.extend(((out.len() as u64) - files_at).to_le_bytes());
        footer.extend((body.len() as u64).to_le_bytes());
        footer.extend(0_u32.to_le_bytes());
        footer.extend((i as u32 + 10).to_le_bytes());
        out.extend(*body);
    }
    if trailing {
        footer.push(0);
    }
    let footer_at = out.len() as u64;
    out.extend(&footer);
    out.extend(record("Header", 12, HEADER.len() as u64));
    out.extend(record("Files", files_at, footer_at - files_at));
    out.extend(record("Footer", footer_at, footer.len() as u64));
    out
}
fn archive(data: Vec<u8>) -> Result<RwdArchive<Cursor<Vec<u8>>>, ArchiveError> {
    RwdArchive::new(Cursor::new(data))
}
fn le64(b: &[u8], p: usize) -> u64 {
    u64::from_le_bytes(b[p..p + 8].try_into().unwrap())
}
fn footer_at(b: &[u8]) -> usize {
    le64(b, b.len() - 96 + 64) as usize
}

#[test]
fn real_member_bytes_and_metadata_are_preserved() {
    let data = fixture(
        &[
            ("AVars_version.tgi", REAL),
            ("Mixed\\ÅFolder\\Tiny.bin", b"\0\xff\x12"),
        ],
        false,
    );
    let mut r = archive(data.clone()).unwrap();
    assert_eq!(r.archive_size(), data.len() as u64);
    assert_eq!(r.chunks()[0].offset(), 12);
    assert_eq!(r.chunks()[0].size(), 18);
    assert_eq!(r.chunks()[0].opaque(), 5_346_237_856_730_520);
    assert_eq!(r.chunks()[0].name_utf16()[0], b'H' as u16);
    assert_eq!(r.entries()[1].original_name(), "Mixed\\ÅFolder\\Tiny.bin");
    assert_eq!(r.entries()[1].path(), "Mixed/ÅFolder/Tiny.bin");
    assert_eq!(
        r.entries()[1].name_utf16(),
        "Mixed\\ÅFolder\\Tiny.bin"
            .encode_utf16()
            .collect::<Vec<_>>()
    );
    assert_eq!(r.entries()[0].source_index(), 10);
    assert_eq!(r.entries()[0].flags(), 0);
    assert_eq!(r.entries()[0].relative_offset(), 0);
    assert_eq!(r.entries()[0].offset(), 30);
    let i = r.find("avars_VERSION.TGI").unwrap().unwrap();
    let mut got = Vec::new();
    assert_eq!(r.copy_entry(i, &mut got).unwrap(), REAL.len() as u64);
    assert_eq!(got, REAL);
    assert_eq!(r.find("not-found").unwrap(), None);
    assert!(matches!(
        r.copy_entry(99, &mut Vec::new()),
        Err(ArchiveError::EntryNotFound(99))
    ));
    assert_eq!(r.into_inner().into_inner(), data);
}

#[test]
fn empty_archives_and_empty_members_are_valid() {
    assert!(archive(fixture(&[], false)).unwrap().entries().is_empty());
    let mut r = archive(fixture(&[("zero.bin", b"")], false)).unwrap();
    assert_eq!(r.copy_entry(0, &mut Vec::new()).unwrap(), 0);
}

#[test]
fn traversal_device_and_alias_paths_are_rejected_portably() {
    for name in [
        "",
        "/root",
        "\\root",
        "../outside",
        "a/../../b",
        "a\\..\\b",
        "C:/out",
        "C:out",
        "//server/share",
        "a:stream",
        "a/./b",
        "a//b",
        "a/",
        "NUL",
        "nul.txt",
        "a/CON.txt",
        "COM1",
        "lpt9.bin",
        "COM¹.txt",
        "file.",
        "file ",
        "a\0b",
        "a\tb",
        "a?b",
        "a|b",
        "a*b",
    ] {
        assert!(normalized_member_path(name).is_err(), "accepted {name:?}");
        assert!(
            archive(fixture(&[(name, b"x")], false)).is_err(),
            "indexed {name:?}"
        );
    }
    assert_eq!(normalized_member_path("a\\b.ext").unwrap(), "a/b.ext");
}

#[test]
fn malformed_headers_counts_names_and_chunks_fail_without_large_allocations() {
    let original = fixture(&[("ok", b"x")], false);
    for length in 0..12 {
        assert!(archive(original[..length].to_vec()).is_err());
    }
    let mut b = original.clone();
    b[0] = b'X';
    assert!(archive(b).is_err());
    let mut b = original.clone();
    b[4..8].copy_from_slice(&3_u32.to_le_bytes());
    assert!(matches!(
        archive(b),
        Err(ArchiveError::UnsupportedVersion(3))
    ));
    for count in [0, u32::MAX] {
        let mut b = original.clone();
        b[8..12].copy_from_slice(&count.to_le_bytes());
        assert!(archive(b).is_err());
    }
    let table = original.len() - 288;
    let mut b = original.clone();
    b[table + 64..table + 72].copy_from_slice(&0_u64.to_le_bytes());
    assert!(archive(b).is_err());
    let mut b = original.clone();
    b[table + 72..table + 80].copy_from_slice(&u64::MAX.to_le_bytes());
    b[table + 88..table + 96].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(archive(b).is_err());
    let mut b = original.clone();
    b[table + 88] ^= 1;
    assert!(matches!(
        archive(b),
        Err(ArchiveError::UnsupportedCompression)
    ));
    let mut b = original.clone();
    b[table..table + 2].copy_from_slice(&0xD800_u16.to_le_bytes());
    assert!(matches!(archive(b), Err(ArchiveError::InvalidUtf16)));
    let mut b = original.clone();
    b[table + 96..table + 160].copy_from_slice(&record("Footer", 0, 0)[..64]);
    assert!(archive(b).is_err());
    let mut b = original.clone();
    b[table + 192 + 64..table + 192 + 72].copy_from_slice(&30_u64.to_le_bytes());
    assert!(archive(b).is_err());
    let mut b = original.clone();
    b[table + 96..table + 160].copy_from_slice(&record("Alien", 0, 0)[..64]);
    assert!(archive(b).is_err());
}

#[test]
fn corrupt_footer_bounds_flags_and_duplicates_are_rejected() {
    let original = fixture(&[("ok", b"x")], false);
    let f = footer_at(&original);
    let metadata = f + 4 + 2 + 4;
    let mut b = original.clone();
    b[f..f + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(archive(b).is_err());
    let mut b = original.clone();
    b[f + 4..f + 6].copy_from_slice(&u16::MAX.to_le_bytes());
    assert!(archive(b).is_err());
    let mut b = original.clone();
    b[f + 6..f + 8].copy_from_slice(&0xD800_u16.to_le_bytes());
    assert!(matches!(archive(b), Err(ArchiveError::InvalidUtf16)));
    for at in [metadata, metadata + 8] {
        let mut b = original.clone();
        b[at..at + 8].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(archive(b).is_err());
    }
    let mut b = original.clone();
    b[metadata + 16..metadata + 20].copy_from_slice(&1_u32.to_le_bytes());
    assert!(matches!(archive(b), Err(ArchiveError::UnsupportedFlags(1))));
    assert!(archive(fixture(&[("a", b"x"), ("A", b"y")], false)).is_err());
    assert!(archive(fixture(&[("a/b", b"x"), ("a\\b", b"y")], false)).is_err());
    assert!(archive(fixture(&[("ok", b"x")], true)).is_err());
}

struct FailSink;
impl Write for FailSink {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "fixture rejection",
        ))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
struct TruncatePayload(Cursor<Vec<u8>>);
impl Read for TruncatePayload {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        self.0.read(b)
    }
}
impl Seek for TruncatePayload {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        if matches!(to, SeekFrom::Start(30)) {
            self.0.get_mut().truncate(30 + REAL.len() - 1);
        }
        self.0.seek(to)
    }
}
#[test]
fn sink_errors_and_source_truncation_never_claim_success() {
    let bytes = fixture(&[("real.tgi", REAL)], false);
    assert!(
        archive(bytes.clone())
            .unwrap()
            .copy_entry(0, &mut FailSink)
            .is_err()
    );
    let mut r = RwdArchive::new(TruncatePayload(Cursor::new(bytes))).unwrap();
    assert!(r.copy_entry(0, &mut Vec::new()).is_err());
}

static COUNTER: AtomicU64 = AtomicU64::new(0);
fn test_dir() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "aa-formats-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    root
}
#[test]
fn extraction_creates_exact_member_and_refuses_overwrite() {
    let root = test_dir();
    let mut r = archive(fixture(&[("sub/Exact.tgi", REAL)], false)).unwrap();
    let target = r.extract_one(0, &root).unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), REAL);
    assert!(r.extract_one(0, &root).is_err());
    assert_eq!(std::fs::read(&target).unwrap(), REAL);
    // Remove only the exact fixture files/directories we created; no recursion.
    std::fs::remove_file(target).unwrap();
    std::fs::remove_dir(root.join("sub")).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn extraction_refuses_existing_file_as_directory() {
    let root = test_dir();
    std::fs::write(root.join("sub"), b"preserve").unwrap();
    let mut r = archive(fixture(&[("sub/file", b"bad")], false)).unwrap();
    assert!(r.extract_one(0, &root).is_err());
    assert_eq!(std::fs::read(root.join("sub")).unwrap(), b"preserve");
    std::fs::remove_file(root.join("sub")).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[cfg(unix)]
#[test]
fn extraction_refuses_symlink_parent() {
    let root = test_dir();
    let other = test_dir();
    std::os::unix::fs::symlink(&other, root.join("link")).unwrap();
    let mut r = archive(fixture(&[("link/file", b"bad")], false)).unwrap();
    assert!(r.extract_one(0, &root).is_err());
    assert!(!other.join("file").exists());
    std::fs::remove_file(root.join("link")).unwrap();
    std::fs::remove_dir(root).unwrap();
    std::fs::remove_dir(other).unwrap();
}

struct Sparse {
    ranges: BTreeMap<u64, Vec<u8>>,
    size: u64,
    pos: u64,
    read_bytes: u64,
}
impl Seek for Sparse {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        self.pos = match to {
            SeekFrom::Start(n) => n,
            SeekFrom::End(n) => self.size.checked_add_signed(n).unwrap(),
            SeekFrom::Current(n) => self.pos.checked_add_signed(n).unwrap(),
        };
        Ok(self.pos)
    }
}
impl Read for Sparse {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let n = out.len().min(self.size.saturating_sub(self.pos) as usize);
        for (i, b) in out[..n].iter_mut().enumerate() {
            let p = self.pos + i as u64;
            *b = self
                .ranges
                .range(..=p)
                .next_back()
                .and_then(|(start, bytes)| bytes.get((p - start) as usize))
                .copied()
                .unwrap_or(0);
        }
        self.pos += n as u64;
        self.read_bytes += n as u64;
        Ok(n)
    }
}
#[test]
fn u64_offsets_above_four_gib_do_not_require_payload_allocation() {
    let files_size = 5_u64 * 1024 * 1024 * 1024;
    let footer_offset = 12 + files_size;
    let mut footer = 1_u32.to_le_bytes().to_vec();
    footer.extend(1_u16.to_le_bytes());
    footer.extend((b'x' as u16).to_le_bytes());
    footer.extend((files_size - 1).to_le_bytes());
    footer.extend(1_u64.to_le_bytes());
    footer.extend(0_u32.to_le_bytes());
    footer.extend(77_u32.to_le_bytes());
    let table_offset = footer_offset + footer.len() as u64;
    let mut table = record("Files", 12, files_size);
    table.extend(record("Footer", footer_offset, footer.len() as u64));
    let ranges = BTreeMap::from([
        (0, b"TGCK\x02\0\0\0\x02\0\0\0".to_vec()),
        (footer_offset - 1, vec![0x7f]),
        (footer_offset, footer),
        (table_offset, table.clone()),
    ]);
    let sparse = Sparse {
        ranges,
        size: table_offset + table.len() as u64,
        pos: 0,
        read_bytes: 0,
    };
    let mut r = RwdArchive::new(sparse).unwrap();
    assert!(r.entries()[0].offset() > u64::from(u32::MAX));
    let mut got = Vec::new();
    r.copy_entry(0, &mut got).unwrap();
    assert_eq!(got, [0x7f]);
    assert!(r.into_inner().read_bytes < 1024);
}

#[test]
#[ignore = "optional read-only check: set AA_GAME_ROOT to the user's installed game"]
fn installed_archive_matches_small_recovered_source_member() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("set AA_GAME_ROOT"));
    let mut r = RwdArchive::open(root.join("Data.rwd")).unwrap();
    assert_eq!(r.archive_size(), 644_491_987);
    assert_eq!(r.entries().len(), 13_428);
    let i = r.find("AVars_version.tgi").unwrap().unwrap();
    assert_eq!(r.entries()[i].offset(), 22_419);
    assert_eq!(r.entries()[i].source_index(), 3);
    let mut actual = Vec::new();
    r.copy_entry(i, &mut actual).unwrap();
    assert_eq!(actual, REAL);
}

#[test]
#[ignore = "optional metadata-only check: set AA_GAME_ROOT to the installed game"]
fn all_ten_installed_archives_have_recovered_member_counts() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("set AA_GAME_ROOT"));
    for (name, expected) in [
        ("Data.rwd", 13428),
        ("Music.rwd", 34),
        ("Speech.rwd", 100),
        ("Video.rwd", 42),
        ("Video_En.rwd", 10),
        ("skins/GBR.rwd", 226),
        ("skins/GER.rwd", 220),
        ("skins/JPN.rwd", 233),
        ("skins/RUS.rwd", 229),
        ("skins/USA.rwd", 226),
    ] {
        let r = RwdArchive::open(root.join(name)).unwrap();
        assert_eq!(r.entries().len(), expected, "{name}");
        assert_eq!(r.chunks().len(), 3);
        assert!(r.chunks().iter().all(|c| c.size() == c.stored_size()));
    }
}
