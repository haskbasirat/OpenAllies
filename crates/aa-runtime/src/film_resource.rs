//! Move-only writable film resource and non-owning typed Admin268 attachment.
//! One mutex protects the SAME growable TGCK stream; an attachment never clones
//! the stream or keeps a destroyed Session1C resource artificially alive.
use aa_metagame::chunk_file::ChunkFileWriter;
use std::collections::VecDeque;
use std::sync::{
    Arc, Mutex, MutexGuard, Weak,
    atomic::{AtomicBool, AtomicU32, Ordering},
};
use std::{
    fs::File,
    io::{Seek, SeekFrom, Write},
};

type Result<T> = std::result::Result<T, String>;
struct Stream {
    writer: Option<ChunkFileWriter>,
    opened: bool,
    current_chunk: Option<String>,
    orders_flushed: bool,
    file: Option<FileBackend>,
}
struct FileBackend {
    file: File,
    persisted: u64,
}
impl FileBackend {
    fn append(&mut self, mut bytes: &[u8]) -> std::io::Result<()> {
        while !bytes.is_empty() {
            match self.file.write(bytes) {
                Ok(0) => return Err(std::io::ErrorKind::WriteZero.into()),
                Ok(n) => {
                    self.persisted += n as u64;
                    bytes = &bytes[n..];
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
    fn persist(&mut self, view: aa_metagame::chunk_file::FlushedBytes<'_>) -> std::io::Result<()> {
        let prefix = view.prefix.len() as u64;
        let total = prefix + view.compressed_tail.len() as u64;
        if self.persisted > total {
            return Err(std::io::Error::other(
                "film backing stream position exceeds current codec output",
            ));
        }
        self.file.seek(SeekFrom::Start(self.persisted))?;
        if self.persisted < prefix {
            self.append(&view.prefix[self.persisted as usize..])?;
        }
        let tail_at = usize::try_from(self.persisted - prefix)
            .map_err(|_| std::io::Error::other("film backing offset exceeds address space"))?;
        self.append(&view.compressed_tail[tail_at..])?;
        self.file.flush()
    }
    fn finish(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        if self.persisted > bytes.len() as u64 {
            return Err(std::io::Error::other(
                "film final output shrank below persisted prefix",
            ));
        }
        let count = u32::from_le_bytes(
            bytes
                .get(8..12)
                .ok_or_else(|| std::io::Error::other("film final header missing"))?
                .try_into()
                .expect("four bytes"),
        ) as usize;
        let directory = count
            .checked_mul(0x60)
            .ok_or_else(|| std::io::Error::other("film directory size overflow"))?;
        let body_end = bytes
            .len()
            .checked_sub(directory)
            .ok_or_else(|| std::io::Error::other("film directory exceeds final output"))?;
        if self.persisted > body_end as u64 {
            return Err(std::io::Error::other(
                "film persisted prefix exceeds final body",
            ));
        }
        //859090 finishes the current compressor BEFORE rewriting the header,
        //then emits the final96-byte records one by one after the body.
        self.file.seek(SeekFrom::Start(self.persisted))?;
        self.append(&bytes[self.persisted as usize..body_end])?;
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(
            bytes
                .get(..12)
                .ok_or_else(|| std::io::Error::other("film final header missing"))?,
        )?;
        self.file.seek(SeekFrom::Start(body_end as u64))?;
        for record in bytes[body_end..].chunks_exact(0x60) {
            self.append(record)?;
        }
        //876720 closes through8766D0: drain876600, FlushFileBuffers, then
        //CloseHandle. File is unbuffered here; flush alone does not request
        //the native operating-system flush.
        self.file.sync_all()
    }
}
impl Stream {
    fn persist(&mut self) -> Result<()> {
        if let Some(file) = self.file.as_mut() {
            file.persist(
                self.writer
                    .as_ref()
                    .ok_or("film stream is closed")?
                    .flushed_bytes()
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| format!("film backing file write/flush failed: {e}"))?;
        }
        Ok(())
    }
}
/// Session1C retains this actual resource. No Clone/Default/raw stream handle.
pub struct Resource {
    core: Arc<Core>,
}
struct Core {
    stream: Arc<Mutex<Stream>>,
    metadata: Mutex<Option<Metadata>>,
    map_string_references: Mutex<VecDeque<crate::localization::EntryReference>>,
    depots: Mutex<Vec<crate::film_create::DepotId>>,
    retired: AtomicBool,
    flags_8: AtomicU32,
    requests_4: AtomicU32,
    references_14: AtomicU32,
}
pub struct MetadataLoan<'a> {
    guard: MutexGuard<'a, Option<Metadata>>,
}
impl MetadataLoan<'_> {
    pub fn ids(&self) -> &[u16] {
        &self.guard.as_ref().expect("validated actual metadata").ids
    }
    pub fn header(&self) -> &crate::film_header_owner::State {
        &self
            .guard
            .as_ref()
            .expect("validated actual metadata")
            .header
    }
    pub fn creator(&self) -> &crate::resource_creator::Creator {
        &self
            .guard
            .as_ref()
            .expect("validated actual metadata")
            .creator
    }
}
pub struct ReferencesLoan<'a> {
    guard: MutexGuard<'a, VecDeque<crate::localization::EntryReference>>,
}
impl ReferencesLoan<'_> {
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &crate::localization::EntryReference> {
        self.guard.iter()
    }
}

struct Metadata {
    ids: Vec<u16>,
    header: crate::film_header_owner::State,
    creator: Box<crate::resource_creator::Creator>,
}
/// NativeAdmin268 is a borrowed pointer, not an independent film allocation.
/// Weak keeps its identity unique after resource destruction/address reuse.
#[derive(Clone)]
pub struct RecorderHandle {
    stream: Weak<Mutex<Stream>>,
}
impl Resource {
    /// Fresh writable in-memory backend for5A00E0's open0x102 boundary. The
    /// caller owns5A3250's actual filename/depot/header producer; this does not
    /// certify those services or implicitly turn recording on/off.
    pub fn open_memory(path: &str) -> Self {
        Self::new(None, Some(ChunkFileWriter::create(path)), true)
    }
    fn new(metadata: Option<Metadata>, writer: Option<ChunkFileWriter>, opened: bool) -> Self {
        Self {
            core: Arc::new(Core {
                stream: Arc::new(Mutex::new(Stream {
                    writer,
                    opened,
                    current_chunk: None,
                    orders_flushed: false,
                    file: None,
                })),
                metadata: Mutex::new(metadata),
                map_string_references: Mutex::new(VecDeque::new()),
                depots: Mutex::new(Vec::new()),
                retired: AtomicBool::new(false),
                flags_8: AtomicU32::new(0x18),
                requests_4: AtomicU32::new(0),
                references_14: AtomicU32::new(0),
            }),
        }
    }
    ///Actual constructor fields remain in one shared allocation. The manager
    ///retains this same allocation; Session1C never obtains a metadata copy.
    pub(crate) fn from_constructed_fields(
        ids: Vec<u16>,
        header: crate::film_header_owner::State,
        creator: Box<crate::resource_creator::Creator>,
    ) -> Self {
        Self::new(
            Some(Metadata {
                ids,
                header,
                creator,
            }),
            None,
            false,
        )
    }
    fn require_live(&self) -> Result<()> {
        if self.core.retired.load(Ordering::Acquire) {
            Err("film resource allocation was retired by its manager".into())
        } else {
            Ok(())
        }
    }
    fn stream_loan(&self) -> Result<MutexGuard<'_, Stream>> {
        let guard = self
            .core
            .stream
            .lock()
            .map_err(|_| "film stream lock is poisoned")?;
        self.require_live()?;
        Ok(guard)
    }
    pub(crate) fn retain_for_manager(&self) -> Result<Self> {
        self.require_live()?;
        Ok(Self {
            core: self.core.clone(),
        })
    }
    pub(crate) fn same_allocation(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.core, &other.core)
    }
    pub(crate) fn base_flags_8(&self) -> Result<u32> {
        self.require_live()?;
        Ok(self.core.flags_8.load(Ordering::Acquire))
    }
    pub(crate) fn store_base_flags_8(&mut self, value: u32) -> Result<()> {
        self.require_live()?;
        self.core.flags_8.store(value, Ordering::Release);
        Ok(())
    }
    pub(crate) fn requests_4(&self) -> Result<u32> {
        self.require_live()?;
        Ok(self.core.requests_4.load(Ordering::Acquire))
    }
    pub(crate) fn store_requests_4(&mut self, value: u32) -> Result<()> {
        self.require_live()?;
        self.core.requests_4.store(value, Ordering::Release);
        Ok(())
    }
    pub(crate) fn references_14(&self) -> Result<u32> {
        self.require_live()?;
        Ok(self.core.references_14.load(Ordering::Acquire))
    }
    pub(crate) fn increment_references_14_wrapping(&mut self) -> Result<()> {
        self.require_live()?;
        self.core.references_14.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
    ///The concrete manager invokes this only at actual allocation retirement.
    ///An unclosed writable stream requires its real close producer first.
    pub(crate) fn retire_from_manager(&mut self) -> Result<()> {
        self.require_live()?;
        let stream = self
            .core
            .stream
            .try_lock()
            .map_err(|_| "film stream is already borrowed during retirement")?;
        if stream.writer.is_some() || stream.file.is_some() {
            return Err("film manager retirement requires actual open-stream close".into());
        }
        let mut metadata = self
            .core
            .metadata
            .try_lock()
            .map_err(|_| "film metadata is already borrowed during retirement")?;
        let mut refs = self
            .core
            .map_string_references
            .try_lock()
            .map_err(|_| "film MapString references are already borrowed during retirement")?;
        let mut depots = self
            .core
            .depots
            .try_lock()
            .map_err(|_| "film depots are already borrowed during retirement")?;
        self.core.retired.store(true, Ordering::Release);
        metadata.take();
        refs.clear();
        depots.clear();
        Ok(())
    }
    pub fn depots(&self) -> Result<MutexGuard<'_, Vec<crate::film_create::DepotId>>> {
        self.require_live()?;
        self.core
            .depots
            .try_lock()
            .map_err(|_| "film depot list is already borrowed".into())
    }
    pub(crate) fn depots_mut(
        &mut self,
    ) -> Result<MutexGuard<'_, Vec<crate::film_create::DepotId>>> {
        self.depots()
    }
    ///5A1120's existing-entry branch. First4BAA20 performs no display-value
    ///read: the supplied second CString is used only by the missing/create
    ///branch. Preserve duplicate references and prepend in native list order.
    pub fn retain_existing_localized_pair(
        &mut self,
        current: &crate::localization::Localization,
        key: &[u16],
        _supplied_text: Option<&[u16]>,
    ) -> Result<()> {
        let reference=current.reference_existing(key).map_err(|e|e.to_string())?
            .ok_or("5A1120 missing entry requires actual5A1040 warning then4BAA20 create/fallback service")?;
        self.require_live()?;
        self.core
            .map_string_references
            .try_lock()
            .map_err(|_| "film MapString references are already borrowed")?
            .push_front(reference);
        Ok(())
    }
    pub fn map_string_references(&self) -> Result<ReferencesLoan<'_>> {
        self.require_live()?;
        Ok(ReferencesLoan {
            guard: self
                .core
                .map_string_references
                .try_lock()
                .map_err(|_| "film MapString references are already borrowed")?,
        })
    }
    pub fn validate_map_string_references(
        &self,
        current: &crate::localization::Localization,
    ) -> Result<()> {
        for reference in self.map_string_references()?.iter() {
            current
                .validate_reference(reference)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    ///A scoped borrow of the SAME Header/Creator/IDS fields. Reentrant metadata
    ///borrowing fails explicitly instead of deadlocking or cloning authority.
    pub fn metadata(&self) -> Result<MetadataLoan<'_>> {
        self.require_live()?;
        let guard = self
            .core
            .metadata
            .try_lock()
            .map_err(|_| "film metadata is already borrowed")?;
        self.require_live()?;
        if guard.is_none() {
            return Err("film diagnostic stream has no constructed metadata".into());
        }
        Ok(MetadataLoan { guard })
    }
    ///Concrete59EA60. MAP's raw zero precedes the current compression read;
    ///then serialize this SAME retained Header and polymorphic Creator. The
    ///closed sink can mutate only stream/ref fields and read current text: it
    ///cannot reenter mutable metadata while that immutable loan is held.
    pub fn save_header_current(
        &mut self,
        catalog: &crate::catalog::Catalog<'_>,
        bank: &crate::definitions::Bank,
        text: &crate::localization::Localization,
        variables: &crate::film_variables::State,
    ) -> Result<()> {
        self.open_chunk("MAP", 0)?;
        self.write(&0u32.to_le_bytes())?;
        self.open_chunk("MAP_HEADER", variables.compressed_byte())?;
        let fields = self.metadata()?;
        let mut sink = HeaderSink {
            core: &self.core,
            text,
        };
        crate::film_current_header::write_current(&mut sink, fields.header(), catalog, bank)?;
        crate::film_creator::write_current(&mut sink, fields.creator())
    }
    ///The backend for the fresh5A00E0 open boundary. The caller still must
    ///resolve the actual resource/depot path before invoking this service;
    ///this method certifies no Windows filesystem publication or locking.
    pub fn open_memory_backend(&mut self, path: &str) -> Result<()> {
        self.require_live()?;
        let mut stream = self.stream_loan()?;
        if stream.opened {
            return Err("fresh film stream open boundary was already entered".into());
        }
        stream.writer = Some(ChunkFileWriter::create(path));
        stream.opened = true;
        Ok(())
    }
    ///Concrete file transport for5A00E0. The backend supplies the SAME File
    ///opened with its actual0x102 path/share policy; this method does not
    ///guess work-depot resolution or open permissions. Initial TGCK header,
    ///later flush suffixes and final directory go to that one retained file.
    pub(crate) fn open_file_backend(&mut self, file: File, label: &str) -> Result<()> {
        let mut stream = self.stream_loan()?;
        if stream.opened {
            return Err("fresh film stream open boundary was already entered".into());
        }
        stream.file = Some(FileBackend { file, persisted: 0 });
        stream.writer = Some(ChunkFileWriter::create(label));
        stream.opened = true;
        stream.persist()
    }
    ///Only the concrete5A3250 caller may apply the final recorded-map name
    ///after its publication service has succeeded.
    pub(crate) fn set_recording_name_5a3250(&mut self, name: &[u16]) -> Result<()> {
        self.require_live()?;
        self.core
            .metadata
            .try_lock()
            .map_err(|_| "film metadata is already borrowed")?
            .as_mut()
            .ok_or("film resource has no actual Header")?
            .header
            .set_recording_name_5a3250(name);
        Ok(())
    }
    pub fn open_chunk(&mut self, name: &str, compressed: u8) -> Result<()> {
        self.require_live()?;
        let mut stream = self.stream_loan()?;
        stream
            .writer
            .as_mut()
            .ok_or("film resource is closed")?
            .open_chunk(name, compressed != 0)
            .map_err(|e| e.to_string())?;
        stream.current_chunk = Some(name.into());
        stream.orders_flushed = false;
        stream.persist()
    }
    pub fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.require_live()?;
        let mut stream = self.stream_loan()?;
        stream
            .writer
            .as_mut()
            .ok_or("film resource is closed")?
            .write(bytes)
            .map_err(|e| e.to_string())?;
        stream.persist()
    }
    /// Flush the sole backing stream without closing the ongoing Orders chunk.
    pub fn flush(&mut self) -> Result<()> {
        self.require_live()?;
        let mut stream = self.stream_loan()?;
        stream
            .writer
            .as_mut()
            .ok_or("film resource is closed")?
            .flush()
            .map_err(|e| e.to_string())?;
        stream.persist()?;
        //Film wrapper856DC0 dispatches the actual backing child's +44 to
        //8766D0 (FlushFileBuffers). Keep this at the explicit flush boundary,
        //after codec output is written and before publishing Orders readiness.
        if let Some(file) = stream.file.as_mut() {
            file.file
                .sync_all()
                .map_err(|e| format!("film backing file flush failed: {e}"))?;
        }
        if stream.current_chunk.as_deref() == Some("Orders") {
            stream.orders_flushed = true;
        }
        Ok(())
    }
    /// Called only after76C310 has opened Orders and completed its flush.
    pub fn recorder_handle(&self) -> Result<RecorderHandle> {
        self.require_live()?;
        let stream = self.stream_loan()?;
        if !stream.orders_flushed {
            return Err("film Orders stream has not completed its native flush".into());
        }
        Ok(RecorderHandle {
            stream: Arc::downgrade(&self.core.stream),
        })
    }
    /// Normal resource close finalizes the SAME writer/directory. Any extant
    /// nonowning Admin attachment then refuses further writes.
    pub fn close(&mut self) -> Result<Vec<u8>> {
        self.require_live()?;
        let mut stream = self.stream_loan()?;
        let writer = stream.writer.take().ok_or("film resource already closed")?;
        stream.orders_flushed = false;
        let bytes = writer.close().map_err(|e| e.to_string())?;
        if let Some(file) = stream.file.as_mut() {
            file.finish(&bytes)
                .map_err(|e| format!("film backing file close failed: {e}"))?;
        }
        stream.file.take();
        Ok(bytes)
    }
}
struct HeaderSink<'a> {
    core: &'a Core,
    text: &'a crate::localization::Localization,
}
impl crate::film_header::Sink for HeaderSink<'_> {
    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        let mut stream = self
            .core
            .stream
            .lock()
            .map_err(|_| "film stream lock is poisoned")?;
        if self.core.retired.load(Ordering::Acquire) {
            return Err("film resource allocation was retired".into());
        }
        stream
            .writer
            .as_mut()
            .ok_or("film resource is closed")?
            .write(bytes)
            .map_err(|e| e.to_string())?;
        stream.persist()
    }
    fn register_map_string(&mut self, key: &[u16]) -> Result<()> {
        self.register_localized_pair(key, None)
    }
    fn register_localized_pair(&mut self, key: &[u16], _text: Option<&[u16]>) -> Result<()> {
        let reference=self.text.reference_existing(key).map_err(|e|e.to_string())?
            .ok_or("5A1120 missing entry requires actual5A1040 warning then4BAA20 create/fallback service")?;
        let mut refs = self
            .core
            .map_string_references
            .try_lock()
            .map_err(|_| "film MapString refs are already borrowed")?;
        if self.core.retired.load(Ordering::Acquire) {
            return Err("film resource allocation was retired".into());
        }
        refs.push_front(reference);
        Ok(())
    }
}
impl RecorderHandle {
    /// Actual virtual+4 payload append used by the order mirror. The caller
    /// retains the native length-prefix protocol; this adds no packet framing.
    pub fn write(&self, bytes: &[u8]) -> Result<()> {
        let owner = self
            .stream
            .upgrade()
            .ok_or("film recorder's resource has been destroyed")?;
        let mut stream = owner.lock().map_err(|_| "film stream lock is poisoned")?;
        stream
            .writer
            .as_mut()
            .ok_or("film recorder's resource is closed")?
            .write(bytes)
            .map_err(|e| e.to_string())?;
        stream.persist()
    }
    pub fn is_live(&self) -> bool {
        self.stream
            .upgrade()
            .and_then(|owner| owner.lock().ok().map(|stream| stream.writer.is_some()))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod ownership_tests {
    use super::*;
    #[test]
    fn manager_retains_same_allocation_and_retirement_invalidates_session_alias() {
        //Backend storage/lifetime verification, not a native manager/depot
        //publication oracle or a fabricated positive Header constructor.
        let mut session = Resource::open_memory("explicit-shared-backend.RFM");
        let mut manager = session.retain_for_manager().unwrap();
        assert!(manager.same_allocation(&session));
        assert_eq!(session.base_flags_8().unwrap(), 0x18);
        assert_eq!(session.requests_4().unwrap(), 0);
        session.store_base_flags_8(8).unwrap();
        session.store_requests_4(7).unwrap();
        assert_eq!(manager.base_flags_8().unwrap(), 8);
        assert_eq!(manager.requests_4().unwrap(), 7);
        session.open_chunk("Orders", 0).unwrap();
        session.flush().unwrap();
        let recorder = session.recorder_handle().unwrap();
        drop(session);
        assert!(recorder.is_live());
        let mut alias = manager.retain_for_manager().unwrap();
        assert!(manager.retire_from_manager().is_err());
        manager.close().unwrap();
        assert!(!recorder.is_live());
        manager.retire_from_manager().unwrap();
        assert!(alias.open_memory_backend("forbidden-reopen").is_err());
        assert!(alias.base_flags_8().is_err());
        assert!(alias.depots().is_err());
        assert!(alias.write(&[1]).is_err());
    }
    #[test]
    fn same_file_flush_continue_close_and_real_write_error() {
        //Native format-compatible backend test; no full depot/open/share
        //policy or whole film factory claim. Both files are unique test output.
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("aa-rust-film-{}-{stamp}.RFM", std::process::id()));
        let file = std::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let mut resource = Resource::new(None, None, false);
        resource
            .open_file_backend(file, "explicit-file-backend")
            .unwrap();
        assert_eq!(std::fs::read(&path).unwrap().len(), 12);
        resource.open_chunk("MAP", 0).unwrap();
        resource.write(&0u32.to_le_bytes()).unwrap();
        resource.open_chunk("MAP_HEADER", 1).unwrap();
        resource.write(&[42; 97]).unwrap();
        resource.flush().unwrap();
        let first = std::fs::read(&path).unwrap();
        assert!(first.len() > 16);
        resource.open_chunk("Orders", 1).unwrap();
        resource.write(&[5; 103]).unwrap();
        resource.flush().unwrap();
        let recorder = resource.recorder_handle().unwrap();
        recorder.write(&[7; 79]).unwrap();
        resource.flush().unwrap();
        let final_bytes = resource.close().unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), final_bytes);
        assert_eq!(
            u32::from_le_bytes(final_bytes[8..12].try_into().unwrap()),
            3
        );
        assert!(!recorder.is_live());
        let readonly = std::fs::File::open(&path).unwrap();
        let mut denied = Resource::new(None, None, false);
        assert!(
            denied
                .open_file_backend(readonly, "explicit-readonly-failure")
                .is_err()
        );
        assert!(denied.recorder_handle().is_err());
        assert!(denied.retire_from_manager().is_err());
        drop(denied);
        drop(resource);
        std::fs::remove_file(&path).unwrap();
    }
}
