//! Wire schemas of every original order payload (all 358 descriptors).
//!
//! Each native order class `T` has one `T::Serialize` writer and one reader
//! (often inlined into the decree handler); both visit the same fields in the
//! same order. A schema here is one function written against [`Ser`], used for
//! both directions: [`decode`] records a flat ordered list of typed
//! [`Field`]s, [`encode`] consumes such a list and reproduces the original
//! 0x40D420 bit packing. Control flow inside a schema depends only on values
//! already visited or on an explicit [`SchemaContext`] lookup (game data the
//! native decoder reads, e.g. team-command flags), never on guesses.
//!
//! Primitive field kinds and their native readers/writers:
//! * [`Field::Uint`]: raw scalar, 0x40D5D0 / 0x40D420 (width 1..=32).
//! * [`Field::Text`]: CString, 16-bit code-unit count then ONE `16*count`-bit
//!   buffer read (0x410B50; writer inline). Splitting the buffer changes bits.
//! * [`Field::Bounded`]: 0x411FA0 with a literal possibility count `n`.
//! * [`Field::Def`]: 0x411FA0 with `[registry+0x28]+1` possibilities; index
//!   `count` is the null sentinel (writer 0x76A880 family uses `def+0xC`).
//! * [`Field::Thing`]: 16-bit thing-registry slot (0xB364C0+0x20004), readers
//!   0x76A4B0/0x76A5A0/0x76A600/0x76A530/0x76A670/0x76A6E0/0x76A750 differ only
//!   in the type-check virtual; writer 0x76A500 writes `thing+0x14` low16, 0 for null.
//! * [`Field::Player`]: 32-bit SessionPlayer+0x20 id (0x765F70 / 0x765F10, 0 = null).
//! * [`Field::NetAddr`]: two 32-bit fields, IPv4 then port (0x76A810 / 0x76A7C0).
//!
//! Errors are nontransactional like the rest of this crate: a failed decode or
//! encode may have consumed or written earlier fields.
use crate::commands::WireWriter;
use crate::{Cursor, Error, Result, registry_bits};
use serde::{Deserialize, Serialize};

pub mod complex;
mod editor;
mod game;
mod lobby;
mod schemas;
pub use schemas::{SCHEMAS, schema};

/// A definition manager, identified by the address of its global pointer in
/// the original image (e.g. 0xB3AA58 = ActorCommand definitions). The runtime
/// supplies its current count through [`SchemaContext::registry_count`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Registry(pub u32);

/// Type-check virtual used by the native thing-reference reader. The wire is
/// identical (16-bit slot); resolution differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThingCheck {
    /// 0x76A4B0: no type check.
    Any,
    /// 0x76A5A0: virtual +0xB4.
    B4,
    /// 0x76A600: virtual +0xA8.
    A8,
    /// 0x76A530: virtual +0xB0.
    B0,
    /// 0x76A670: virtual +0xAC.
    Ac,
    /// 0x76A6E0: virtual +0xBC.
    Bc,
    /// 0x76A750: virtual +0xC0.
    C0,
}

/// Native lookups that resolve a CString to an object whose own name the
/// writer serializes again. Keys are C strings: the lookup sees only the code
/// units before the first NUL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NameKind {
    /// 0x76A9C0 -> 0x41EEC0 (random-map configuration); empty name -> null.
    RandomMapConfig,
    /// 0x959070 container lookup 0x93E170 ([[0xB36530]+0x820]+8).
    MetagameContainer,
    /// 0x959070 element lookup 0x95D900 inside the container.
    MetagameElement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Field {
    Uint { width: u8, value: u32 },
    Text(Vec<u16>),
    /// Same wire as [`Field::Text`]; resolved by name before re-serialization.
    Named { kind: NameKind, units: Vec<u16> },
    Bounded { n: u32, value: u32 },
    /// Raw wire index; `index == count` is the native null sentinel.
    Def { registry: Registry, index: u32 },
    Thing { check: ThingCheck, slot: u16 },
    Player(u32),
    NetAddr { ip: u32, port: u32 },
}

/// Game data that a native decoder consults while decoding. Every lookup is
/// explicit; an unavailable value is an error, never a default.
pub trait SchemaContext {
    /// `[registry]+0x28` count of a definition manager.
    fn registry_count(&self, registry: Registry) -> Result<usize>;
    /// `[[0xB36530]+0x270]+0x24[index]+0x24`: flag word of the TeamCommand
    /// definition selected by the first field of 0x509090.
    fn team_command_flags(&self, index: u32) -> Result<u32>;
    /// `def+0x20` of the definition at `index` of `registry` (for example the
    /// WorldRule value kind switched on by 0x511E50: 0 bool, 1/3 int, 2 float).
    fn definition_kind(&self, registry: Registry, index: u32) -> Result<u32>;
    /// Caller policy for native lists whose entries occupy zero bits (a
    /// definition list over an empty registry). Such a list cannot be bounded
    /// by the packet length, so the host must choose an explicit limit.
    fn zero_width_list_budget(&self) -> usize;
}

/// Host-side reference resolution performed by a request's native reader
/// before the original writer re-serializes the decree from resolved objects.
pub trait Resolver {
    /// Thing reader type check; `Some(slot)` is the resolved thing's +0x14 low16.
    fn thing(&self, check: ThingCheck, slot: u16) -> Option<u16>;
    /// 0x76B8F0 live-player lookup (id and film byte 0).
    fn player(&self, id: u32) -> bool;
    /// Index below count, nonnull entry and (where the reader checks) virtual +0x38.
    fn definition(&self, registry: Registry, index: u32) -> bool;
    /// Name lookup on the NUL-terminated `key`; returns the resolved object's
    /// own name (what the writer serializes) or None when nothing is found.
    fn named(&self, kind: NameKind, key: &[u16]) -> Option<Vec<u16>>;
}

/// Re-serialization of a decoded order from its resolved native object (the
/// request -> decree conversion of ProcessRequest and every film mirror).
/// KKC_EditorReplaceTrigger (293/296) is refused: the native reader files the
/// elements into three lists (0x542A70, copy 0x540F90) and the writer
/// 0x6B7170 emits them regrouped, with element fields the copy does not keep;
/// that regrouping is not ported.
pub fn reserialize(
    descriptor: usize,
    fields: &[Field],
    resolver: &dyn Resolver,
    context: &dyn SchemaContext,
) -> Result<Vec<Field>> {
    match descriptor {
        293 | 296 => Err(Error::UnsupportedReserialization(descriptor)),
        _ => normalize_for_decree(fields, resolver, context),
    }
}

/// The value changes a host's request->decree re-encoding applies: an
/// unresolved thing writes slot 0 (0x76A500), an unresolved player writes id 0
/// (0x765F10), an unresolved definition writes the count sentinel (0x76A880
/// family), and an address is re-serialized from its sockaddr (port is the
/// 16-bit sin_port, 0x781F30). Everything else is copied unchanged.
pub fn normalize_for_decree(
    fields: &[Field],
    resolver: &dyn Resolver,
    context: &dyn SchemaContext,
) -> Result<Vec<Field>> {
    fields
        .iter()
        .map(|f| {
            Ok(match *f {
                Field::Thing { check, slot } => Field::Thing {
                    check,
                    slot: resolver.thing(check, slot).unwrap_or(0),
                },
                Field::Player(id) => Field::Player(if resolver.player(id) { id } else { 0 }),
                Field::Def { registry, index } => Field::Def {
                    registry,
                    index: if resolver.definition(registry, index) {
                        index
                    } else {
                        def_count(context, registry)?
                    },
                },
                Field::NetAddr { ip, port } => Field::NetAddr {
                    ip,
                    port: port & 0xffff,
                },
                Field::Named { kind, ref units } => {
                    let key: Vec<u16> = units.iter().copied().take_while(|&u| u != 0).collect();
                    let units = resolver.named(kind, &key).ok_or(Error::SchemaContext(
                        "native writer dereferences an unresolved named reference",
                    ))?;
                    Field::Named { kind, units }
                }
                ref other => other.clone(),
            })
        })
        .collect()
}

/// One visitor for both directions; see the module documentation.
pub trait Ser {
    fn reading(&self) -> bool;
    fn context(&self) -> &dyn SchemaContext;
    fn uint(&mut self, width: u8) -> Result<u32>;
    fn text(&mut self) -> Result<Vec<u16>>;
    /// CString resolved by a native name lookup (see [`NameKind`]).
    fn named(&mut self, kind: NameKind) -> Result<Vec<u16>>;
    fn bounded(&mut self, n: u32) -> Result<u32>;
    /// Returns the raw index (`== count` for null).
    fn def(&mut self, registry: Registry) -> Result<u32>;
    fn thing(&mut self, check: ThingCheck) -> Result<u16>;
    fn player(&mut self) -> Result<u32>;
    fn net_addr(&mut self) -> Result<(u32, u32)>;
    fn bool(&mut self) -> Result<bool> {
        Ok(self.uint(1)? == 1)
    }
    /// Called before a native loop of `count` entries of at least
    /// `bits_each` bits. Reading rejects counts the remaining bits cannot hold
    /// (and zero-width lists above the caller budget); writing does nothing.
    fn list_len(&mut self, count: u32, bits_each: usize) -> Result<()>;
    /// Bit width of one [`Ser::def`] of `registry` under the current count.
    fn def_bits(&self, registry: Registry) -> Result<usize> {
        let count = def_count(self.context(), registry)?;
        Ok(registry_bits(count as usize + 1)? as usize)
    }
}

/// Native "definition vector" pattern (0x50F420/0x50E050/0x9526F0...): a
/// 32-bit entry count, then that many [`Ser::def`] references.
pub fn def_list(s: &mut dyn Ser, registry: Registry) -> Result<u32> {
    let count = s.uint(32)?;
    let bits = s.def_bits(registry)?;
    s.list_len(count, bits)?;
    for _ in 0..count {
        s.def(registry)?;
    }
    Ok(count)
}

pub type Schema = fn(&mut dyn Ser) -> Result<()>;

fn def_count(context: &dyn SchemaContext, registry: Registry) -> Result<u32> {
    let count = context.registry_count(registry)?;
    let possibilities = count.checked_add(1).ok_or(Error::RegistrySize)?;
    registry_bits(possibilities)?;
    u32::try_from(count).map_err(|_| Error::RegistrySize)
}

/// Decoding visitor over the native cursor; records every field.
pub struct Reader<'a, 'b> {
    pub cursor: &'a mut Cursor<'b>,
    pub context: &'a dyn SchemaContext,
    pub fields: Vec<Field>,
}
impl Ser for Reader<'_, '_> {
    fn reading(&self) -> bool {
        true
    }
    fn context(&self) -> &dyn SchemaContext {
        self.context
    }
    fn uint(&mut self, width: u8) -> Result<u32> {
        let value = self.cursor.read_uint(width)?;
        self.fields.push(Field::Uint { width, value });
        Ok(value)
    }
    fn text(&mut self) -> Result<Vec<u16>> {
        let count = self.cursor.read_uint(16)? as usize;
        let raw = self.cursor.read_raw_bits(count * 16)?;
        let units: Vec<u16> = raw
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        self.fields.push(Field::Text(units.clone()));
        Ok(units)
    }
    fn named(&mut self, kind: NameKind) -> Result<Vec<u16>> {
        let units = self.text()?;
        self.fields.pop();
        self.fields.push(Field::Named { kind, units: units.clone() });
        Ok(units)
    }
    fn bounded(&mut self, n: u32) -> Result<u32> {
        let value = self.cursor.read_possibility(n as usize)?;
        self.fields.push(Field::Bounded { n, value });
        Ok(value)
    }
    fn def(&mut self, registry: Registry) -> Result<u32> {
        let count = def_count(self.context, registry)?;
        let index = self.cursor.read_possibility(count as usize + 1)?;
        self.fields.push(Field::Def { registry, index });
        Ok(index)
    }
    fn thing(&mut self, check: ThingCheck) -> Result<u16> {
        let slot = self.cursor.read_uint(16)? as u16;
        self.fields.push(Field::Thing { check, slot });
        Ok(slot)
    }
    fn player(&mut self) -> Result<u32> {
        let id = self.cursor.read_uint(32)?;
        self.fields.push(Field::Player(id));
        Ok(id)
    }
    fn list_len(&mut self, count: u32, bits_each: usize) -> Result<()> {
        let remaining = self.cursor.remaining_bits();
        let needed = (count as u64) * (bits_each as u64);
        if needed > remaining as u64
            || (bits_each == 0 && count as usize > self.context.zero_width_list_budget())
        {
            return Err(Error::ListLength {
                requested: count as u64,
                remaining,
            });
        }
        Ok(())
    }
    fn net_addr(&mut self) -> Result<(u32, u32)> {
        let ip = self.cursor.read_uint(32)?;
        let port = self.cursor.read_uint(32)?;
        self.fields.push(Field::NetAddr { ip, port });
        Ok((ip, port))
    }
}

/// Encoding visitor: consumes `fields` in order and writes them.
pub struct Writer<'a> {
    pub writer: &'a mut WireWriter,
    pub context: &'a dyn SchemaContext,
    pub fields: std::slice::Iter<'a, Field>,
}
impl Writer<'_> {
    fn next(&mut self, expected: &'static str) -> Result<&Field> {
        self.fields.next().ok_or(Error::SchemaField { expected })
    }
    /// 16-bit code-unit count then one `16*count`-bit buffer write.
    fn write_text(&mut self, units: Vec<u16>) -> Result<Vec<u16>> {
        let count = u16::try_from(units.len()).map_err(|_| Error::SchemaField {
            expected: "text of at most 65535 code units",
        })?;
        self.writer.write_uint(u32::from(count), 16)?;
        let bytes: Vec<u8> = units.iter().flat_map(|u| u.to_le_bytes()).collect();
        self.writer.write_raw_bits(&bytes, units.len() * 16)?;
        Ok(units)
    }
}
impl Ser for Writer<'_> {
    fn reading(&self) -> bool {
        false
    }
    fn context(&self) -> &dyn SchemaContext {
        self.context
    }
    fn uint(&mut self, width: u8) -> Result<u32> {
        match *self.next("uint")? {
            Field::Uint { width: w, value } if w == width => {
                if width < 32 && value >> width != 0 {
                    return Err(Error::SchemaField { expected: "uint within width" });
                }
                self.writer.write_uint(value, width)?;
                Ok(value)
            }
            _ => Err(Error::SchemaField { expected: "uint" }),
        }
    }
    fn text(&mut self) -> Result<Vec<u16>> {
        let Field::Text(units) = self.next("text")?.clone() else {
            return Err(Error::SchemaField { expected: "text" });
        };
        self.write_text(units)
    }
    fn named(&mut self, kind: NameKind) -> Result<Vec<u16>> {
        match self.next("named text")?.clone() {
            Field::Named { kind: k, units } if k == kind => self.write_text(units),
            _ => Err(Error::SchemaField { expected: "named text" }),
        }
    }
    fn bounded(&mut self, n: u32) -> Result<u32> {
        match *self.next("bounded")? {
            Field::Bounded { n: m, value } if m == n && value < n => {
                self.writer.write_uint(value, registry_bits(n as usize)?)?;
                Ok(value)
            }
            _ => Err(Error::SchemaField { expected: "bounded" }),
        }
    }
    fn def(&mut self, registry: Registry) -> Result<u32> {
        let count = def_count(self.context, registry)?;
        match *self.next("def")? {
            Field::Def { registry: r, index } if r == registry && index <= count => {
                self.writer
                    .write_uint(index, registry_bits(count as usize + 1)?)?;
                Ok(index)
            }
            _ => Err(Error::SchemaField { expected: "def" }),
        }
    }
    fn thing(&mut self, check: ThingCheck) -> Result<u16> {
        match *self.next("thing")? {
            Field::Thing { check: c, slot } if c == check => {
                self.writer.write_uint(u32::from(slot), 16)?;
                Ok(slot)
            }
            _ => Err(Error::SchemaField { expected: "thing" }),
        }
    }
    fn player(&mut self) -> Result<u32> {
        match *self.next("player")? {
            Field::Player(id) => {
                self.writer.write_uint(id, 32)?;
                Ok(id)
            }
            _ => Err(Error::SchemaField { expected: "player" }),
        }
    }
    fn list_len(&mut self, _count: u32, _bits_each: usize) -> Result<()> {
        Ok(())
    }
    fn net_addr(&mut self) -> Result<(u32, u32)> {
        match *self.next("net address")? {
            Field::NetAddr { ip, port } => {
                self.writer.write_uint(ip, 32)?;
                self.writer.write_uint(port, 32)?;
                Ok((ip, port))
            }
            _ => Err(Error::SchemaField { expected: "net address" }),
        }
    }
}

/// Decode one payload of `descriptor` (registry ordinal) at the cursor.
pub fn decode(
    descriptor: usize,
    cursor: &mut Cursor<'_>,
    context: &dyn SchemaContext,
) -> Result<Vec<Field>> {
    let schema = schema(descriptor)?;
    let mut reader = Reader {
        cursor,
        context,
        fields: Vec::new(),
    };
    schema(&mut reader)?;
    Ok(reader.fields)
}

/// Encode `fields` as the payload of `descriptor`. Every field must be used.
pub fn encode(
    descriptor: usize,
    fields: &[Field],
    writer: &mut WireWriter,
    context: &dyn SchemaContext,
) -> Result<()> {
    let schema = schema(descriptor)?;
    let mut visitor = Writer {
        writer,
        context,
        fields: fields.iter(),
    };
    schema(&mut visitor)?;
    if visitor.fields.next().is_some() {
        return Err(Error::SchemaField {
            expected: "no further fields",
        });
    }
    Ok(())
}
