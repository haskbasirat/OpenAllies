//! Port of the NIF stream reader statically linked into AA.exe (Gamebryo
//! NiStream, engine 10.1.0.0). Addresses are AA.exe virtual addresses.
//!
//! * `NiStream::Load(char*, int)` 87FEF0 wraps the bytes in NiMemStream and
//!   calls `LoadStream` 881BD0 through `Load(NiBinaryStream*)` 87FF30.
//! * `LoadHeader` 8810F0: header line (GetLine 876020, 0x80 limit),
//!   "File Format" check, file version against 3.3.0.11..=10.1.0.0, user
//!   version (read only for versions >= 10.0.1.8) against 0..=0, object count.
//! * Versions >= 5.0.0.1 read the class table 881960 (u16 count, sized class
//!   names, a u16 class index per object); versions > 5.0.0.5 read the group
//!   table 880BB0. Older versions name each object inline (881AF0).
//! * Every object's `LoadBinary` (vtable slot 3), then the top-level object
//!   list 8812F0, then `LinkObject` (slot 4) for every object, which pops the
//!   link IDs and counts each `LoadBinary` pushed, in push order.
//!
//! Only reading and link resolution are ported. LoadStream also runs every
//! object's `PostLinkObject` (slot 11) and the registered post-process
//! function (NiParticle 89B090 registers 8E93B0, which replaces old-style
//! particle systems). A file with such objects reports them in
//! [`NifFile::requires_native_post_process`]; consumers must refuse it.
//!
//! The shipped data contains NIF 4.2.2.0, 10.0.1.0 and 10.1.0.0 files; the
//! loaders below follow every native version branch for the classes those
//! files use. Every other registered class is an explicit
//! [`NifError::UnportedClass`].

mod classes;
mod link;
mod object;
mod rtti;
mod stream;

pub use object::{f32_value, Field, Fields, NifObject, Value, NULL_LINK};
pub use rtti::{base_of, is_kind_of, RTTI};
pub use stream::{Push, TraceRow};

use stream::Stream;

pub const VERSION_4_2_2_0: u32 = 0x0402_0200;
pub const VERSION_10_0_1_0: u32 = 0x0a00_0100;
pub const VERSION_10_1_0_0: u32 = 0x0a01_0000;
/// B323C4 / B323C8: the native accepted file-version range.
pub const MIN_VERSION: u32 = 0x0303_000b;
pub const MAX_VERSION: u32 = 0x0a01_0000;
/// B44174 / B44178 are never written: the accepted user version is 0..=0.
pub const MIN_USER_VERSION: u32 = 0;
pub const MAX_USER_VERSION: u32 = 0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NifError {
    /// A read past the end of the data (the native stream would clamp).
    Truncated { offset: usize, requested: usize },
    Overflow,
    /// LoadHeader error code 2.
    NotNif,
    /// LoadHeader error code 3.
    VersionTooOld(u32),
    /// LoadHeader error code 4.
    UnknownVersion(u32),
    UserVersionTooOld(u32),
    UnknownUserVersion(u32),
    /// Error code 5: "<class>: cannot find create function."
    NoCreateFunction(Vec<u8>),
    /// A class the native registry creates whose loader is not ported.
    UnportedClass(&'static str),
    /// A native path that exists but is not ported (with the address).
    Unsupported(&'static str),
    /// A class index, group ID or link ID the native code would use without
    /// a bounds check (undefined behaviour), or a class name longer than the
    /// native 0x100-byte stack buffer.
    OutOfRange(&'static str),
}

impl std::fmt::Display for NifError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NifError::Truncated { offset, requested } => {
                write!(f, "NIF read of {requested} bytes at {offset} passes the end of the data")
            }
            NifError::Overflow => write!(f, "NIF size arithmetic overflows"),
            NifError::NotNif => write!(f, "Not a NIF file"),
            NifError::VersionTooOld(v) => write!(f, "NIF version {v:#010x} is too old"),
            NifError::UnknownVersion(v) => write!(f, "Unknown NIF version {v:#010x}"),
            NifError::UserVersionTooOld(v) => write!(f, "NIF user defined version {v} is too old"),
            NifError::UnknownUserVersion(v) => write!(f, "Unknown NIF user defined version {v}"),
            NifError::NoCreateFunction(name) => {
                write!(f, "{}: cannot find create function.", String::from_utf8_lossy(name))
            }
            NifError::UnportedClass(c) => write!(f, "native NIF loader for {c} is not ported"),
            NifError::Unsupported(what) => write!(f, "native NIF path not ported: {what}"),
            NifError::OutOfRange(what) => write!(f, "NIF {what} is out of the native range"),
        }
    }
}

impl std::error::Error for NifError {}

/// The result of a native-equivalent `NiStream::Load`.
#[derive(Clone, Debug)]
pub struct NifFile {
    pub header: Vec<u8>,
    pub version: u32,
    pub user_version: u32,
    pub objects: Vec<NifObject>,
    /// The top-level objects (NiStream+0x148), as object indices.
    pub top_level: Vec<Option<usize>>,
    /// The link-ID and link-count lists, in push order.
    pub link_ids: Vec<u32>,
    pub link_counts: Vec<u32>,
    /// Per object, the kinds of its pushes in order (LinkObject pop order).
    pub pushes: Vec<Vec<Push>>,
    /// Indices of objects that the native post-process step 8E93B0 converts
    /// (old-style particle systems). Empty for every file it leaves alone.
    pub requires_native_post_process: Vec<usize>,
    /// Per object, the NiNode child array after every LinkObject (empty for
    /// other classes): see `link::link_children`.
    pub children: Vec<Vec<Option<usize>>>,
    /// Per object, the NiAVObject parent (+0x24) after every LinkObject.
    pub parents: Vec<Option<usize>>,
}

impl NifFile {
    /// `NiStream::GetObjectFromLinkID` 8803A0: NULL for 0xFFFFFFFF.
    pub fn target(&self, link: u32) -> Option<usize> {
        (link != NULL_LINK).then_some(link as usize)
    }

    pub fn object(&self, link: u32) -> Option<&NifObject> {
        self.target(link).map(|i| &self.objects[i])
    }
}

/// Load options; `trace` records every native read.
#[derive(Clone, Copy, Debug, Default)]
pub struct LoadOptions {
    pub trace: bool,
}

/// A load with its read trace.
pub struct TracedLoad {
    pub result: Result<NifFile, NifError>,
    pub trace: Vec<TraceRow>,
}

pub fn load(data: &[u8]) -> Result<NifFile, NifError> {
    load_stream(&mut Stream::new(data, false))
}

pub fn load_traced(data: &[u8]) -> TracedLoad {
    let mut stream = Stream::new(data, true);
    let result = load_stream(&mut stream);
    TracedLoad { result, trace: stream.take_trace().unwrap_or_default() }
}

/// GetLine 876020 with the 0x80 limit used by LoadHeader: one 1-byte Read
/// per character; stops at '\n', at end of data or after 127 stored
/// characters; '\r' is dropped and does not count.
fn header_line(s: &mut Stream) -> Result<Vec<u8>, NifError> {
    let mut line = Vec::new();
    let mut stored = 1usize;
    if 0x80 <= stored {
        return Ok(line);
    }
    loop {
        let c = s.u8()?;
        if c == b'\n' {
            break;
        }
        if c != b'\r' {
            line.push(c);
            stored += 1;
        }
        if stored >= 0x80 {
            break;
        }
    }
    Ok(line)
}

/// LoadHeader 8810F0.
fn load_header(s: &mut Stream) -> Result<(Vec<u8>, u32), NifError> {
    let line = header_line(s)?;
    // strstr on the NUL-terminated line.
    let text = line.split(|&b| b == 0).next().unwrap_or(&[]);
    if !text.windows(11).any(|w| w == b"File Format") {
        return Err(NifError::NotNif);
    }
    s.version = s.u32()?;
    if s.version < MIN_VERSION {
        return Err(NifError::VersionTooOld(s.version));
    }
    if s.version > MAX_VERSION {
        return Err(NifError::UnknownVersion(s.version));
    }
    if s.version >= 0x0a00_0108 {
        s.user_version = s.u32()?;
    }
    if s.user_version < MIN_USER_VERSION {
        return Err(NifError::UserVersionTooOld(s.user_version));
    }
    if s.user_version > MAX_USER_VERSION {
        return Err(NifError::UnknownUserVersion(s.user_version));
    }
    let count = s.u32()?;
    Ok((line, count))
}

/// Class-name lookup in the loader map (881600; hash 8EB090, exact compare
/// 8EB0C0 on the NUL-terminated name).
fn create(name: &[u8]) -> Result<&'static classes::Class, NifError> {
    let name = name.split(|&b| b == 0).next().unwrap_or(&[]);
    classes::by_name(name).ok_or_else(|| NifError::NoCreateFunction(name.to_vec()))
}

/// A sized class name read into the native 0x100-byte stack buffer.
fn class_name(s: &mut Stream) -> Result<Vec<u8>, NifError> {
    let len = s.u32()?;
    if len > 0xff {
        return Err(NifError::OutOfRange("class name length"));
    }
    s.bytes(len as usize)
}

fn load_stream(s: &mut Stream) -> Result<NifFile, NifError> {
    let (header, count) = load_header(s)?;
    s.object = 0;
    let count = count as usize;
    let mut created: Vec<(&'static classes::Class, Vec<u8>)> = Vec::new();
    if s.version >= 0x0500_0001 {
        // 881960: the class table, then one class index per object.
        let types = s.u16()?;
        let mut table = Vec::with_capacity(types as usize);
        for _ in 0..types {
            let name = class_name(s)?;
            table.push((create(&name)?, name));
        }
        for _ in 0..count {
            let index = s.u16()? as usize;
            let entry = table.get(index).ok_or(NifError::OutOfRange("object class index"))?;
            created.push((entry.0, entry.1.clone()));
        }
    }
    if s.version > 0x0500_0005 {
        // 880BB0: group count, then one size word per group.
        let groups = s.u32()?;
        for _ in 0..groups {
            s.u32()?;
        }
        s.groups = groups;
    }
    let mut objects = Vec::with_capacity(count);
    let mut pushes = Vec::with_capacity(count);
    for index in 0..count {
        s.object = index as u32;
        let push_start = s.pushes.len();
        let (class, stream_name) = if s.version < 0x0500_0001 {
            // 881AF0: the class name precedes each object.
            let name = class_name(s)?;
            (create(&name)?, name)
        } else {
            created[index].clone()
        };
        let mut fields = Fields::default();
        let load = class.load.ok_or(NifError::UnportedClass(class.name))?;
        load(s, &mut fields)?;
        objects.push(NifObject { class: class.name, stream_name, fields });
        pushes.push(s.pushes[push_start..].to_vec());
    }
    s.object = count as u32;
    // 8812F0: top-level objects; -1 is NULL, other IDs index the objects.
    let roots = s.u32()?;
    let mut top_level = Vec::new();
    for _ in 0..roots {
        let id = s.u32()?;
        top_level.push(if id == NULL_LINK {
            None
        } else if (id as usize) < count {
            Some(id as usize)
        } else {
            return Err(NifError::OutOfRange("top-level link ID"));
        });
    }
    // LinkObject: GetObjectFromLinkID 8803A0 indexes the object array
    // without a bounds check.
    if s.link_ids.iter().any(|&id| id != NULL_LINK && id as usize >= count) {
        return Err(NifError::OutOfRange("link ID"));
    }
    let (children, parents) = link::link_children(&objects)?;
    let requires_native_post_process = objects
        .iter()
        .enumerate()
        .filter(|(_, o)| classes::OLD_PARTICLE_CLASSES.contains(&o.class))
        .map(|(i, _)| i)
        .collect();
    Ok(NifFile {
        header,
        version: s.version,
        user_version: s.user_version,
        objects,
        top_level,
        link_ids: std::mem::take(&mut s.link_ids),
        link_counts: std::mem::take(&mut s.link_counts),
        pushes,
        requires_native_post_process,
        children,
        parents,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(version: u32, user: Option<u32>, objects: u32) -> Vec<u8> {
        let mut b = b"Gamebryo File Format, Version x\n".to_vec();
        b.extend_from_slice(&version.to_le_bytes());
        if let Some(u) = user {
            b.extend_from_slice(&u.to_le_bytes());
        }
        b.extend_from_slice(&objects.to_le_bytes());
        b
    }

    fn table(b: &mut Vec<u8>, names: &[&str], indices: &[u16]) {
        b.extend_from_slice(&(names.len() as u16).to_le_bytes());
        for n in names {
            b.extend_from_slice(&(n.len() as u32).to_le_bytes());
            b.extend_from_slice(n.as_bytes());
        }
        for i in indices {
            b.extend_from_slice(&i.to_le_bytes());
        }
    }

    #[test]
    fn load_header_errors_follow_8810f0() {
        assert_eq!(load(b"Some other file\n\0\0\0\0").unwrap_err(), NifError::NotNif);
        assert_eq!(load(&header(0x0303_000a, None, 0)).unwrap_err(), NifError::VersionTooOld(0x0303_000a));
        assert_eq!(load(&header(0x0a01_0001, Some(0), 0)).unwrap_err(), NifError::UnknownVersion(0x0a01_0001));
        assert_eq!(load(&header(0x0a01_0000, Some(1), 0)).unwrap_err(), NifError::UnknownUserVersion(1));
        // Below 10.0.1.8 there is no user-version word: the object count follows.
        let mut b = header(0x0a00_0100, None, 0);
        table(&mut b, &[], &[]);
        b.extend_from_slice(&0u32.to_le_bytes()); // groups (> 5.0.0.5)
        b.extend_from_slice(&0u32.to_le_bytes()); // top-level count
        let file = load(&b).unwrap();
        assert_eq!((file.version, file.user_version, file.objects.len()), (0x0a00_0100, 0, 0));
        assert!(matches!(load(&b[..b.len() - 1]).unwrap_err(), NifError::Truncated { .. }));
    }

    #[test]
    fn class_table_lookup_is_exact_and_unported_classes_are_explicit() {
        let mut b = header(0x0a01_0000, Some(0), 1);
        table(&mut b, &["ninode"], &[0]);
        assert_eq!(load(&b).unwrap_err(), NifError::NoCreateFunction(b"ninode".to_vec()));
        let mut b = header(0x0a01_0000, Some(0), 1);
        table(&mut b, &["NiLODNode"], &[0]);
        b.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(load(&b).unwrap_err(), NifError::UnportedClass("NiLODNode"));
        let mut b = header(0x0a01_0000, Some(0), 1);
        table(&mut b, &["NiNode"], &[1]);
        assert_eq!(load(&b).unwrap_err(), NifError::OutOfRange("object class index"));
    }
}
