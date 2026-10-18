//! Move-only writable film resource and non-owning typed Admin268 attachment.
//! One mutex protects the SAME growable TGCK stream; an attachment never clones
//! the stream or keeps a destroyed Session1C resource artificially alive.
use aa_metagame::chunk_file::ChunkFileWriter;
use std::sync::{Arc, Mutex, Weak};

type Result<T> = std::result::Result<T, String>;
struct Stream {
    writer: Option<ChunkFileWriter>,
    current_chunk: Option<String>,
    orders_flushed: bool,
}
/// Session1C retains this actual resource. No Clone/Default/raw stream handle.
pub struct Resource {
    stream: Arc<Mutex<Stream>>,
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
        Self {
            stream: Arc::new(Mutex::new(Stream {
                writer: Some(ChunkFileWriter::create(path)),
                current_chunk: None,
                orders_flushed: false,
            })),
        }
    }
    pub fn open_chunk(&mut self, name: &str, compressed: u8) -> Result<()> {
        let mut stream = self
            .stream
            .lock()
            .map_err(|_| "film stream lock is poisoned")?;
        stream
            .writer
            .as_mut()
            .ok_or("film resource is closed")?
            .open_chunk(name, compressed != 0)
            .map_err(|e| e.to_string())?;
        stream.current_chunk = Some(name.into());
        stream.orders_flushed = false;
        Ok(())
    }
    pub fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.stream
            .lock()
            .map_err(|_| "film stream lock is poisoned")?
            .writer
            .as_mut()
            .ok_or("film resource is closed")?
            .write(bytes)
            .map_err(|e| e.to_string())
    }
    /// Flush the sole backing stream without closing the ongoing Orders chunk.
    pub fn flush(&mut self) -> Result<()> {
        let mut stream = self
            .stream
            .lock()
            .map_err(|_| "film stream lock is poisoned")?;
        stream
            .writer
            .as_mut()
            .ok_or("film resource is closed")?
            .flush()
            .map_err(|e| e.to_string())?;
        if stream.current_chunk.as_deref() == Some("Orders") {
            stream.orders_flushed = true;
        }
        Ok(())
    }
    /// Called only after76C310 has opened Orders and completed its flush.
    pub fn recorder_handle(&self) -> Result<RecorderHandle> {
        let stream = self
            .stream
            .lock()
            .map_err(|_| "film stream lock is poisoned")?;
        if !stream.orders_flushed {
            return Err("film Orders stream has not completed its native flush".into());
        }
        Ok(RecorderHandle {
            stream: Arc::downgrade(&self.stream),
        })
    }
    /// Normal resource close finalizes the SAME writer/directory. Any extant
    /// nonowning Admin attachment then refuses further writes.
    pub fn close(&mut self) -> Result<Vec<u8>> {
        let mut stream = self
            .stream
            .lock()
            .map_err(|_| "film stream lock is poisoned")?;
        let writer = stream.writer.take().ok_or("film resource already closed")?;
        stream.orders_flushed = false;
        writer.close().map_err(|e| e.to_string())
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
            .map_err(|e| e.to_string())
    }
    pub fn is_live(&self) -> bool {
        self.stream
            .upgrade()
            .and_then(|owner| owner.lock().ok().map(|stream| stream.writer.is_some()))
            .unwrap_or(false)
    }
}
