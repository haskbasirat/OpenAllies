//! Pinned whole-file sources; no definition merging or native loader-stage ordering.
use aa_formats::{ArchiveError, RwdArchive};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};
use thiserror::Error;
pub const BASELINE_LEDGER_SHA256: &str =
    "4712a2b4f12c0177f02dd40ea482f3135432e35d98ae8ed9dfea8cccd370132f";
pub const V14_NATIVE_SHA256: &str =
    "3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1";
const LEDGER: &[u8] = include_bytes!("../../../research/evidence/v14-baseline.json");
/// Per-input decode work limit, not a game RAM or VRAM limit. Caller adjustable.
#[derive(Clone, Copy, Debug)]
pub struct SourceLimits {
    pub max_member_bytes: u64,
}
#[derive(Debug, Error)]
pub enum SourceError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Archive(#[from] ArchiveError),
    #[error(transparent)]
    Ledger(#[from] serde_json::Error),
    #[error("source integrity: {0}")]
    Integrity(String),
    #[error("resource not found: {0}")]
    Missing(String),
    #[error("resource exceeds bounded rule-reader limit: {0}")]
    TooLarge(String),
    #[error("source buffer allocation failed")]
    Allocation,
}
pub type Result<T> = std::result::Result<T, SourceError>;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileIdentity {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}
#[derive(Deserialize)]
struct Ledger {
    v14_files: Vec<FileIdentity>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Origin {
    OriginalArchive { member_index: usize },
    V14Loose,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CatalogEntry {
    pub logical_path: String,
    pub size: u64,
    pub origin: Origin,
}
#[derive(Debug)]
pub struct ResolvedFile {
    pub logical_path: String,
    pub origin: Origin,
    pub sha256: String,
    pub bytes: Vec<u8>,
}
struct Overlay {
    identity: FileIdentity,
    logical_path: String,
    bytes: Vec<u8>,
}
pub struct V14Sources {
    archive: RwdArchive<File>,
    overlays: Vec<Overlay>,
    lookup: BTreeMap<String, usize>,
    limits: SourceLimits,
}
pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn v14_overlay_identities() -> Result<Vec<FileIdentity>> {
    if sha256(LEDGER) != BASELINE_LEDGER_SHA256 {
        return Err(SourceError::Integrity("compiled ledger changed".into()));
    }
    let ledger: Ledger = serde_json::from_slice(LEDGER)?;
    let rows: Vec<_> = ledger
        .v14_files
        .into_iter()
        .filter(|f| f.path.starts_with("Data/"))
        .collect();
    if rows.len() != 23 {
        return Err(SourceError::Integrity("expected 23 Data overlays".into()));
    }
    Ok(rows)
}
impl V14Sources {
    /// Check archive and all23 overlays. Ignore other loose files/Profile.
    /// Windows file handles deny concurrent writes/deletion while read.
    /// The caller supplies its current per-input budget, for example derived
    /// from observed aa-resources headroom. No fixed game-memory allowance.
    pub fn open(
        archive_path: impl AsRef<Path>,
        runtime_root: impl AsRef<Path>,
        limits: SourceLimits,
    ) -> Result<Self> {
        let rows = v14_overlay_identities()?;
        let ledger: Ledger = serde_json::from_slice(LEDGER)?;
        let original = ledger
            .v14_files
            .iter()
            .find(|f| f.path == "Data.rwd")
            .ok_or_else(|| SourceError::Integrity("archive identity absent".into()))?;
        let mut file = checked_open(archive_path.as_ref())?;
        verify_file(&mut file, original)?;
        file.seek(SeekFrom::Start(0))?;
        let archive = RwdArchive::new(file)?;
        reject_reparse(runtime_root.as_ref())?;
        let root = runtime_root.as_ref().canonicalize()?;
        let mut overlays = Vec::with_capacity(rows.len());
        let mut lookup = BTreeMap::new();
        for row in rows {
            let relative = aa_formats::rwd::normalized_member_path(&row.path)?;
            let path = root.join(&relative);
            reject_reparse(&path)?;
            if !path.canonicalize()?.starts_with(&root) {
                return Err(SourceError::Integrity(format!(
                    "overlay escaped root: {}",
                    row.path
                )));
            }
            let mut source = checked_open(&path)?;
            if source.metadata()?.len() != row.size {
                return Err(SourceError::Integrity(format!(
                    "overlay size mismatch: {}",
                    row.path
                )));
            }
            if row.size > limits.max_member_bytes {
                return Err(SourceError::TooLarge(row.path));
            }
            let mut bytes = buffer_for(row.size)?;
            bytes.resize(
                usize::try_from(row.size).map_err(|_| SourceError::Allocation)?,
                0,
            );
            source.read_exact(&mut bytes)?;
            if sha256(&bytes) != row.sha256 {
                return Err(SourceError::Integrity(format!(
                    "overlay hash mismatch: {}",
                    row.path
                )));
            }
            let logical_path = relative.strip_prefix("Data/").unwrap().to_owned();
            if lookup
                .insert(logical_path.to_ascii_lowercase(), overlays.len())
                .is_some()
            {
                return Err(SourceError::Integrity("duplicate overlay".into()));
            }
            overlays.push(Overlay {
                identity: row,
                logical_path,
                bytes,
            });
        }
        Ok(Self {
            archive,
            overlays,
            lookup,
            limits,
        })
    }
    /// Footer order, replaced in place; added files follow pinned ledger order.
    /// Catalog order is NOT native evaluation order.
    pub fn catalog(&self) -> Vec<CatalogEntry> {
        let mut result = Vec::new();
        for (index, entry) in self.archive.entries().iter().enumerate() {
            if let Some(&o) = self.lookup.get(&entry.path().to_ascii_lowercase()) {
                let row = &self.overlays[o];
                result.push(CatalogEntry {
                    logical_path: row.logical_path.clone(),
                    size: row.identity.size,
                    origin: Origin::V14Loose,
                });
            } else {
                result.push(CatalogEntry {
                    logical_path: entry.path().into(),
                    size: entry.size(),
                    origin: Origin::OriginalArchive {
                        member_index: index,
                    },
                });
            }
        }
        for row in &self.overlays {
            if self
                .archive
                .find(&row.logical_path)
                .expect("validated overlay path")
                .is_none()
            {
                result.push(CatalogEntry {
                    logical_path: row.logical_path.clone(),
                    size: row.identity.size,
                    origin: Origin::V14Loose,
                });
            }
        }
        result
    }
    /// Relative to Data. Slash/ASCII-case-insensitive lookup; spelling preserved.
    pub fn read(&mut self, path: &str) -> Result<ResolvedFile> {
        let key = aa_formats::rwd::normalized_member_path(path)?.to_ascii_lowercase();
        if let Some(&i) = self.lookup.get(&key) {
            let r = &self.overlays[i];
            return Ok(ResolvedFile {
                logical_path: r.logical_path.clone(),
                origin: Origin::V14Loose,
                sha256: r.identity.sha256.clone(),
                bytes: {
                    let mut bytes = buffer_for(r.identity.size)?;
                    bytes.extend_from_slice(&r.bytes);
                    bytes
                },
            });
        }
        self.read_original(path)
    }
    /// Explicit stock inspection. Never substitutes for an invalid v14 overlay.
    pub fn read_original(&mut self, path: &str) -> Result<ResolvedFile> {
        let index = self
            .archive
            .find(path)?
            .ok_or_else(|| SourceError::Missing(path.into()))?;
        let e = &self.archive.entries()[index];
        if e.size() > self.limits.max_member_bytes {
            return Err(SourceError::TooLarge(path.into()));
        }
        let name = e.path().to_owned();
        let mut bytes = buffer_for(e.size())?;
        self.archive.copy_entry(index, &mut bytes)?;
        Ok(ResolvedFile {
            logical_path: name,
            origin: Origin::OriginalArchive {
                member_index: index,
            },
            sha256: sha256(&bytes),
            bytes,
        })
    }
}
fn buffer_for(bytes: u64) -> Result<Vec<u8>> {
    let size = usize::try_from(bytes).map_err(|_| SourceError::Allocation)?;
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(size)
        .map_err(|_| SourceError::Allocation)?;
    Ok(buffer)
}
fn verify_file(file: &mut File, row: &FileIdentity) -> Result<()> {
    if file.metadata()?.len() != row.size {
        return Err(SourceError::Integrity(format!(
            "size mismatch: {}",
            row.path
        )));
    }
    let mut h = Sha256::new();
    let mut buffer = vec![0; 1024 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        h.update(&buffer[..n]);
    }
    if format!("{:x}", h.finalize()) != row.sha256 {
        return Err(SourceError::Integrity(format!(
            "hash mismatch: {}",
            row.path
        )));
    }
    Ok(())
}
fn checked_open(path: &Path) -> Result<File> {
    reject_reparse(path)?;
    let mut o = OpenOptions::new();
    o.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        o.share_mode(1);
    }
    let f = o.open(path)?;
    if !f.metadata()?.is_file() {
        return Err(SourceError::Integrity("not a regular source file".into()));
    }
    Ok(f)
}
fn reject_reparse(path: &Path) -> Result<()> {
    for p in path.ancestors() {
        if p.as_os_str().is_empty() {
            continue;
        }
        let m = fs::symlink_metadata(p)?;
        let linked = m.file_type().is_symlink();
        #[cfg(windows)]
        let linked = {
            use std::os::windows::fs::MetadataExt;
            linked || m.file_attributes() & 0x400 != 0
        };
        if linked {
            return Err(SourceError::Integrity(format!(
                "reparse/symlink source: {}",
                p.display()
            )));
        }
    }
    Ok(())
}
