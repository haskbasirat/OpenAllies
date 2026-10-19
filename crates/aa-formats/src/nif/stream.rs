//! The native NiStream input side: one method per native read primitive.
//!
//! Every method performs exactly the `NiBinaryStream::Read` calls of the
//! native helper it names (AA.exe, image base 0x400000), in the same order and
//! with the same requested sizes, so the optional read trace is directly
//! comparable with the native oracle (`research/phase8-nif`).
//!
//! The native in-memory stream (NiMemStream::Read 8D0BD0) silently clamps a
//! read at the end of the buffer and leaves the destination partly
//! uninitialised. That state cannot be reproduced, so any short read is an
//! explicit [`NifError::Truncated`] here. No shipped file reaches it.

use super::NifError;

/// One `NiBinaryStream::Read` call, as the native oracle records it:
/// absolute offset, requested length, delivered length, the loader's current
/// object index (NiStream+0x1A0) and the lengths of the link-ID list
/// (+0x16C) and link-count list (+0x17C) before the read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceRow {
    pub offset: u32,
    pub requested: u32,
    pub delivered: u32,
    pub object: u32,
    pub link_ids: u32,
    pub link_counts: u32,
}

/// Which list a native link push went to, in push order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Push {
    /// `NiStream::ReadLinkID` 880A40 / the IDs of `ReadMultipleLinkIDs` 880AA0.
    Id,
    /// The count of `ReadMultipleLinkIDs` 880AA0 or `NiStream` 880B70.
    Count,
}

pub(crate) struct Stream<'a> {
    data: &'a [u8],
    pos: usize,
    /// NiStream+0x1C.
    pub(crate) version: u32,
    /// NiStream+0x20. The constructor 8816B0 leaves it 0; it is only read
    /// from the file for versions >= 10.0.1.8.
    pub(crate) user_version: u32,
    /// NiStream+0x164 (IDs) and +0x174 (counts).
    pub(crate) link_ids: Vec<u32>,
    pub(crate) link_counts: Vec<u32>,
    /// Push interleaving, kept so the LinkObject pop order can be replayed.
    pub(crate) pushes: Vec<Push>,
    /// NiStream+0x1A0, the object currently being loaded.
    pub(crate) object: u32,
    trace: Option<Vec<TraceRow>>,
    /// Process globals the native loaders write and read within one object
    /// load for versions below 10.0.1.2: B43E0C (NiAVObject flags), B43DC8
    /// (NiProperty flags) and B44624 (NiTimeController flags).
    pub(crate) saved_av_flags: u16,
    pub(crate) saved_property_flags: u16,
    pub(crate) saved_controller_flags: u16,
    /// Number of serialized groups (880BB0); valid group IDs are 0..=groups.
    pub(crate) groups: u32,
}

impl<'a> Stream<'a> {
    pub(crate) fn new(data: &'a [u8], trace: bool) -> Self {
        Self {
            data,
            pos: 0,
            version: 0,
            user_version: 0,
            link_ids: Vec::new(),
            link_counts: Vec::new(),
            pushes: Vec::new(),
            object: 0,
            trace: trace.then(Vec::new),
            saved_av_flags: 0,
            saved_property_flags: 0,
            saved_controller_flags: 0,
            groups: 0,
        }
    }

    pub(crate) fn take_trace(&mut self) -> Option<Vec<TraceRow>> {
        self.trace.take()
    }

    /// One `NiBinaryStream::Read(buffer, n)`.
    pub(crate) fn read(&mut self, n: usize) -> Result<&'a [u8], NifError> {
        let remaining = self.data.len() - self.pos;
        let delivered = n.min(remaining);
        if let Some(trace) = &mut self.trace {
            trace.push(TraceRow {
                offset: self.pos as u32,
                requested: n as u32,
                delivered: delivered as u32,
                object: self.object,
                link_ids: self.link_ids.len() as u32,
                link_counts: self.link_counts.len() as u32,
            });
        }
        if delivered != n {
            return Err(NifError::Truncated { offset: self.pos, requested: n });
        }
        let bytes = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(bytes)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, NifError> {
        Ok(self.read(1)?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16, NifError> {
        let b = self.read(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, NifError> {
        let b = self.read(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// A binary32 read; the raw word is kept (no float conversion).
    pub(crate) fn f32(&mut self) -> Result<u32, NifError> {
        self.u32()
    }

    /// One Read of `n` bytes.
    pub(crate) fn bytes(&mut self, n: usize) -> Result<Vec<u8>, NifError> {
        Ok(self.read(n)?.to_vec())
    }

    /// One Read of `n` 16-bit words.
    pub(crate) fn u16s(&mut self, n: usize) -> Result<Vec<u16>, NifError> {
        let b = self.read(n.checked_mul(2).ok_or(NifError::Overflow)?)?;
        Ok(b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect())
    }

    /// One Read of `n` 32-bit words.
    pub(crate) fn u32s(&mut self, n: usize) -> Result<Vec<u32>, NifError> {
        let b = self.read(n.checked_mul(4).ok_or(NifError::Overflow)?)?;
        Ok(b.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
    }

    /// NiPoint3::LoadBinary 869420: three separate 4-byte reads.
    pub(crate) fn point3(&mut self) -> Result<[u32; 3], NifError> {
        Ok([self.f32()?, self.f32()?, self.f32()?])
    }

    /// NiMatrix3::LoadBinary 870C10: nine 4-byte reads, memory order.
    pub(crate) fn matrix3(&mut self) -> Result<[u32; 9], NifError> {
        let mut m = [0; 9];
        for word in &mut m {
            *word = self.f32()?;
        }
        Ok(m)
    }

    /// NiStream::LoadCString 880EE0: signed 32-bit length; a positive length
    /// is followed by one Read of that many bytes (stored NUL-terminated).
    /// A length <= 0 leaves the string NULL without further reads.
    pub(crate) fn cstring(&mut self) -> Result<Option<Vec<u8>>, NifError> {
        let len = self.u32()? as i32;
        if len <= 0 {
            return Ok(None);
        }
        Ok(Some(self.bytes(len as usize)?))
    }

    /// NiStream::ReadLinkID 880A40: one u32 appended to the link-ID list.
    pub(crate) fn link_id(&mut self) -> Result<u32, NifError> {
        let id = self.u32()?;
        self.link_ids.push(id);
        self.pushes.push(Push::Id);
        Ok(id)
    }

    /// NiStream::ReadMultipleLinkIDs 880AA0: u32 count appended to the
    /// link-count list, then that many link IDs.
    pub(crate) fn link_ids(&mut self) -> Result<Vec<u32>, NifError> {
        let count = self.u32()?;
        self.push_count(count);
        let mut ids = Vec::new();
        for _ in 0..count {
            ids.push(self.link_id()?);
        }
        Ok(ids)
    }

    /// NiStream 880B70: a count appended to the link-count list without a read.
    pub(crate) fn push_count(&mut self, count: u32) {
        self.link_counts.push(count);
        self.pushes.push(Push::Count);
    }

    pub(crate) fn at_least(&self, version: u32) -> bool {
        self.version >= version
    }

    pub(crate) fn below(&self, version: u32) -> bool {
        self.version < version
    }
}
