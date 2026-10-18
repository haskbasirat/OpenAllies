//! Streaming RWD reader based on the installed archives and the existing
//! `reconstruction/tools/archive_extract.py` recovery.
//!
//! Integers are little endian. The 12-byte header is `TGCK`, version, chunk
//! count. EOF holds that many 96-byte chunk records: 32 UTF-16 code units,
//! offset, logical size, opaque u64, and stored size. `Footer` contains a u32
//! member count followed by UTF-16 names and `(offset, size, flags, index)`.
//! Member offsets are relative to `Files`. Unknown metadata is preserved.

use std::collections::HashMap;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const HEADER_SIZE: u64 = 12;
const CHUNK_RECORD_SIZE: u64 = 96;

#[derive(Debug)]
pub enum ArchiveError {
    Io(io::Error),
    Invalid(&'static str),
    UnsupportedVersion(u32),
    UnsupportedCompression,
    UnsupportedFlags(u32),
    InvalidUtf16,
    UnsafePath(String),
    DuplicatePath(String),
    EntryNotFound(usize),
    Allocation,
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "archive I/O: {e}"),
            Self::Invalid(s) => write!(f, "invalid RWD: {s}"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported TGCK version {v}"),
            Self::UnsupportedCompression => write!(f, "compressed TGCK chunks are not supported"),
            Self::UnsupportedFlags(v) => write!(f, "unsupported RWD member flags {v:#x}"),
            Self::InvalidUtf16 => write!(f, "invalid UTF-16 in archive metadata"),
            Self::UnsafePath(p) => write!(f, "unsafe archive path: {p:?}"),
            Self::DuplicatePath(p) => write!(f, "ambiguous archive path: {p:?}"),
            Self::EntryNotFound(i) => write!(f, "no member at index {i}"),
            Self::Allocation => write!(f, "unable to allocate archive metadata"),
        }
    }
}
impl std::error::Error for ArchiveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::Io(e) = self {
            Some(e)
        } else {
            None
        }
    }
}
impl From<io::Error> for ArchiveError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
type Result<T> = std::result::Result<T, ArchiveError>;

/// Exact chunk-table metadata. The opaque value has no assumed meaning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk {
    name: String,
    name_utf16: [u16; 32],
    offset: u64,
    size: u64,
    opaque: u64,
    stored_size: u64,
}
impl Chunk {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn name_utf16(&self) -> &[u16; 32] {
        &self.name_utf16
    }
    pub fn offset(&self) -> u64 {
        self.offset
    }
    pub fn size(&self) -> u64 {
        self.size
    }
    pub fn opaque(&self) -> u64 {
        self.opaque
    }
    pub fn stored_size(&self) -> u64 {
        self.stored_size
    }
}

/// Footer metadata, kept in original footer order. `source_index` is the
/// archive's field; it is distinct from this entry's position in `entries()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    original_name: String,
    name_utf16: Vec<u16>,
    path: String,
    relative_offset: u64,
    offset: u64,
    size: u64,
    flags: u32,
    source_index: u32,
}
impl Entry {
    pub fn original_name(&self) -> &str {
        &self.original_name
    }
    pub fn name_utf16(&self) -> &[u16] {
        &self.name_utf16
    }
    /// Original spelling/case, with backslashes converted to forward slashes.
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn relative_offset(&self) -> u64 {
        self.relative_offset
    }
    pub fn offset(&self) -> u64 {
        self.offset
    }
    pub fn size(&self) -> u64 {
        self.size
    }
    pub fn flags(&self) -> u32 {
        self.flags
    }
    pub fn source_index(&self) -> u32 {
        self.source_index
    }
}

/// Owns an archive reader opened for reading. The payload is never buffered in
/// full. Metadata allocation and parse bounds derive from the actual archive,
/// rather than an arbitrary archive-size or member-count limit.
pub struct RwdArchive<R> {
    reader: R,
    archive_size: u64,
    chunks: Vec<Chunk>,
    entries: Vec<Entry>,
    paths: HashMap<String, usize>,
}

impl RwdArchive<File> {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::new(File::open(path)?)
    }
}

impl<R: Read + Seek> RwdArchive<R> {
    pub fn new(mut reader: R) -> Result<Self> {
        let archive_size = reader.seek(SeekFrom::End(0))?;
        if archive_size < HEADER_SIZE {
            return Err(ArchiveError::Invalid("truncated header"));
        }
        reader.seek(SeekFrom::Start(0))?;
        let header = read_array::<12>(&mut reader)?;
        if &header[..4] != b"TGCK" {
            return Err(ArchiveError::Invalid("wrong magic"));
        }
        let version = u32_at(&header, 4);
        if version != 2 {
            return Err(ArchiveError::UnsupportedVersion(version));
        }
        let count = u64::from(u32_at(&header, 8));
        let table_size = count
            .checked_mul(CHUNK_RECORD_SIZE)
            .ok_or(ArchiveError::Invalid("chunk table overflow"))?;
        let table_offset = archive_size
            .checked_sub(table_size)
            .ok_or(ArchiveError::Invalid("chunk table exceeds archive"))?;
        if count == 0 || table_offset < HEADER_SIZE {
            return Err(ArchiveError::Invalid("invalid chunk table extent"));
        }
        reader.seek(SeekFrom::Start(table_offset))?;
        let mut chunks = Vec::<Chunk>::new();
        let mut names = HashMap::<String, usize>::new();
        for _ in 0..count {
            let raw = read_array::<96>(&mut reader)?;
            let name_utf16 =
                std::array::from_fn(|i| u16::from_le_bytes([raw[i * 2], raw[i * 2 + 1]]));
            let end = name_utf16.iter().position(|&c| c == 0).unwrap_or(32);
            let name =
                String::from_utf16(&name_utf16[..end]).map_err(|_| ArchiveError::InvalidUtf16)?;
            if name.is_empty() {
                return Err(ArchiveError::Invalid("empty chunk name"));
            }
            let chunk = Chunk {
                name: name.clone(),
                name_utf16,
                offset: u64_at(&raw, 64),
                size: u64_at(&raw, 72),
                opaque: u64_at(&raw, 80),
                stored_size: u64_at(&raw, 88),
            };
            if chunk.size != chunk.stored_size {
                return Err(ArchiveError::UnsupportedCompression);
            }
            check_extent(chunk.offset, chunk.stored_size, HEADER_SIZE, table_offset)?;
            if names.contains_key(&name) {
                return Err(ArchiveError::Invalid("duplicate chunk name"));
            }
            chunks
                .try_reserve(1)
                .map_err(|_| ArchiveError::Allocation)?;
            names.try_reserve(1).map_err(|_| ArchiveError::Allocation)?;
            names.insert(name, chunks.len());
            chunks.push(chunk);
        }
        let mut extents = Vec::new();
        extents
            .try_reserve_exact(chunks.len())
            .map_err(|_| ArchiveError::Allocation)?;
        for c in &chunks {
            if c.size != 0 {
                extents.push((c.offset, c.offset + c.size));
            }
        }
        extents.sort_unstable();
        if extents.windows(2).any(|w| w[0].1 > w[1].0) {
            return Err(ArchiveError::Invalid("overlapping chunks"));
        }
        let files = chunks
            .get(
                *names
                    .get("Files")
                    .ok_or(ArchiveError::Invalid("missing Files chunk"))?,
            )
            .unwrap();
        let footer = chunks
            .get(
                *names
                    .get("Footer")
                    .ok_or(ArchiveError::Invalid("missing Footer chunk"))?,
            )
            .unwrap();
        if footer.size < 4 {
            return Err(ArchiveError::Invalid("truncated footer count"));
        }
        reader.seek(SeekFrom::Start(footer.offset))?;
        let mut footer_reader = (&mut reader).take(footer.size);
        let file_count = u32::from_le_bytes(read_array::<4>(&mut footer_reader)?);
        // Every entry needs a length word and24 metadata bytes, even before
        // its name. Do not reserve the claimed count before reading records.
        if u64::from(file_count) > (footer.size - 4) / 26 {
            return Err(ArchiveError::Invalid("member count exceeds footer"));
        }
        let mut entries = Vec::new();
        let mut paths = HashMap::new();
        for _ in 0..file_count {
            let length = usize::from(u16::from_le_bytes(read_array::<2>(&mut footer_reader)?));
            let needed = (length as u64)
                .checked_mul(2)
                .and_then(|n| n.checked_add(24))
                .ok_or(ArchiveError::Invalid("member record overflow"))?;
            if needed > footer_reader.limit() {
                return Err(ArchiveError::Invalid(
                    "member name or metadata exceeds footer",
                ));
            }
            let mut name_utf16 = Vec::new();
            name_utf16
                .try_reserve_exact(length)
                .map_err(|_| ArchiveError::Allocation)?;
            for _ in 0..length {
                name_utf16.push(u16::from_le_bytes(read_array::<2>(&mut footer_reader)?));
            }
            let original_name =
                String::from_utf16(&name_utf16).map_err(|_| ArchiveError::InvalidUtf16)?;
            let path = normalized_member_path(&original_name)?;
            let raw = read_array::<24>(&mut footer_reader)?;
            let relative_offset = u64_at(&raw, 0);
            let size = u64_at(&raw, 8);
            let flags = u32_at(&raw, 16);
            if flags != 0 {
                return Err(ArchiveError::UnsupportedFlags(flags));
            }
            check_extent(relative_offset, size, 0, files.size)?;
            let offset = files
                .offset
                .checked_add(relative_offset)
                .ok_or(ArchiveError::Invalid("absolute member offset overflow"))?;
            let key = path.to_ascii_lowercase();
            if paths.contains_key(&key) {
                return Err(ArchiveError::DuplicatePath(path));
            }
            entries
                .try_reserve(1)
                .map_err(|_| ArchiveError::Allocation)?;
            paths.try_reserve(1).map_err(|_| ArchiveError::Allocation)?;
            paths.insert(key, entries.len());
            entries.push(Entry {
                original_name,
                name_utf16,
                path,
                relative_offset,
                offset,
                size,
                flags,
                source_index: u32_at(&raw, 20),
            });
        }
        if footer_reader.limit() != 0 {
            return Err(ArchiveError::Invalid("unparsed trailing footer bytes"));
        }
        Ok(Self {
            reader,
            archive_size,
            chunks,
            entries,
            paths,
        })
    }

    pub fn archive_size(&self) -> u64 {
        self.archive_size
    }
    pub fn chunks(&self) -> &[Chunk] {
        &self.chunks
    }
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    /// Slash-insensitive and ASCII-case-insensitive lookup. No Unicode case
    /// folding/normalization is invented; original metadata remains untouched.
    pub fn find(&self, name: &str) -> Result<Option<usize>> {
        Ok(self
            .paths
            .get(&normalized_member_path(name)?.to_ascii_lowercase())
            .copied())
    }
    pub fn into_inner(self) -> R {
        self.reader
    }

    /// Stream exactly one member into a caller-owned sink. A truncated source
    /// or failing sink returns an error; it is never reported as full success.
    pub fn copy_entry(&mut self, entry_index: usize, output: &mut impl Write) -> Result<u64> {
        let entry = self
            .entries
            .get(entry_index)
            .ok_or(ArchiveError::EntryNotFound(entry_index))?;
        self.reader.seek(SeekFrom::Start(entry.offset))?;
        let count = io::copy(&mut (&mut self.reader).take(entry.size), output)?;
        if count != entry.size {
            return Err(ArchiveError::Invalid(
                "member payload was truncated after indexing",
            ));
        }
        Ok(count)
    }

    /// Create one member beneath a caller-chosen output directory. Existing
    /// files, symlinks and Windows reparse points are refused. Original member
    /// spelling is retained. Use a directory controlled exclusively by the
    /// caller: portable std filesystem checks cannot prevent a concurrent
    /// hostile rename of an ancestor between validation and creation.
    ///
    /// On I/O failure a newly created partial file may remain; never treat an
    /// error as successful extraction. No existing file is overwritten.
    pub fn extract_one(
        &mut self,
        entry_index: usize,
        output_root: impl AsRef<Path>,
    ) -> Result<PathBuf> {
        let entry = self
            .entries
            .get(entry_index)
            .ok_or(ArchiveError::EntryNotFound(entry_index))?;
        let relative = entry.path.clone();
        let root = output_root.as_ref();
        create_checked_directory(root)?;
        let root = root.canonicalize()?;
        let target = root.join(relative);
        let parent = target
            .parent()
            .ok_or(ArchiveError::Invalid("missing output parent"))?;
        create_checked_directory(parent)?;
        if !parent.canonicalize()?.starts_with(&root) {
            return Err(ArchiveError::Invalid("output parent escaped root"));
        }
        reject_reparse_ancestors(&target)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)?;
        self.copy_entry(entry_index, &mut file)?;
        file.flush()?;
        Ok(target)
    }
}

fn read_array<const N: usize>(reader: &mut impl Read) -> Result<[u8; N]> {
    let mut value = [0; N];
    reader.read_exact(&mut value)?;
    Ok(value)
}
fn u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}
fn u64_at(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
}
fn check_extent(offset: u64, size: u64, lower: u64, upper: u64) -> Result<()> {
    if offset < lower || offset > upper || size > upper - offset {
        return Err(ArchiveError::Invalid(
            "offset/size outside containing region",
        ));
    }
    Ok(())
}

/// Validate portable archive member paths without silently sanitizing them.
pub fn normalized_member_path(name: &str) -> Result<String> {
    let path = name.replace('\\', "/");
    let invalid = path.is_empty()
        || path.starts_with('/')
        || path
            .chars()
            .any(|c| c.is_control() || matches!(c, ':' | '<' | '>' | '"' | '|' | '?' | '*'))
        || path.split('/').any(|part| {
            if part.is_empty() || part == "." || part == ".." || part.ends_with(['.', ' ']) {
                return true;
            }
            let stem = part
                .split('.')
                .next()
                .unwrap_or("")
                .trim_end_matches([' ', '.'])
                .to_ascii_uppercase();
            matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || ["COM", "LPT"].iter().any(|prefix| {
                    stem.strip_prefix(prefix).is_some_and(|n| {
                        matches!(
                            n,
                            "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                        )
                    })
                })
        });
    if invalid {
        return Err(ArchiveError::UnsafePath(name.to_owned()));
    }
    Ok(path)
}

fn reject_reparse_ancestors(path: &Path) -> Result<()> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => {
                #[cfg(windows)]
                let reparse = {
                    use std::os::windows::fs::MetadataExt;
                    metadata.file_attributes() & 0x400 != 0
                };
                #[cfg(not(windows))]
                let reparse = false;
                if metadata.file_type().is_symlink() || reparse {
                    return Err(ArchiveError::Invalid(
                        "output path contains a link or reparse point",
                    ));
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
fn create_checked_directory(path: &Path) -> Result<()> {
    reject_reparse_ancestors(path)?;
    fs::create_dir_all(path)?;
    reject_reparse_ancestors(path)?;
    if !path.is_dir() {
        return Err(ArchiveError::Invalid("output parent is not a directory"));
    }
    Ok(())
}
