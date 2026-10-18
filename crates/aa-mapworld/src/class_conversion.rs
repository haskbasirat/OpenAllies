//! Class-changed saved-record conversion, ported from the original AA.exe.
//!
//! A saved record whose definition now has a different runtime class is not a
//! load failure. `KKC_GWorld` record load `4FAE80` (called by the section
//! loader `4FC340` for USED_REQUIRED_OBJECTS, STATIC_THINGS and
//! DYNAMIC_THINGS, always with `4FC600`'s conversion vector) does, after the
//! generation check (`4FAF14`) and the Thing lookup (`427BD0`):
//!
//! * `4FAFA8`: definition class `[[definition+18]+1C]` equal to the saved
//!   class -> normal construction and Load (not this module);
//! * `4FAFB9..4FAFC3`: saved class == 2 (Object) AND a non-null conversion
//!   vector -> conversion: `4F26F0` builds a 0x30-byte record, `405B80`
//!   copies the IDS, record+4 = definition, record+8 = saved ID, `4EA050`
//!   (-> `4EF0C0`) reads the Thing and Object fields listed in
//!   [`ChangedClassRecord`], `4F9840` appends a copy to the vector; the
//!   record yields null (nothing is registered);
//! * any other mismatch: map warning `load_warning_changed_gclass|s`
//!   (`A2CAB8`, `4FB034..4FB077`) and null.
//!
//! In every null case `4FC340` seeks the chunk to the record's start plus its
//! DYNAMIC_THINGS_INDEX length (`4FC45E..4FC466` -> `858F50`), so the rest of
//! the record is never read; the read count may be smaller or larger than the
//! record. TEAMS/KINGDOMS (`4FCACA`, `4FCD52`) and records embedded in a
//! component Load (`4FC200`) pass no vector, so their mismatches are skipped.
//!
//! After every section, the PostLoad pass (`4EFBE0` -> vf DC), `4F1E10`,
//! `4F5290`, `4F1C20` and the "[4610] InitForLoadedMap" log, the LAST step of
//! `4FC600` (`4FD47C..4FD781`, [`replace_converted`]) creates every queued
//! record FRESH in vector order through `KKC_GWorld::CreateGThing` (`4F9C10`)
//! with an A25B1C creation-parameter block, and writes the map warning
//! `load_warning_replaced_changed_gclass|s` (`A2CE40`) or, for a null result,
//! `load_warning_cant_replace_changed_gclass|s` (`A2CE90`). The vector is then
//! destroyed (`96BE33` with `4F2740`).
//!
//! Partial failure: `8589B0` throws TGC_LoadWarningException ("Bad data
//! version", `41DFB0`) for a Thing version outside 0..=2 or an Object version
//! outside 0..=5. The conversion runs in `4FAE80` EH states 0..3, outside its
//! only try block (states 13..15), and `4FC340`/`4FC600` have no try block, so
//! the exception reaches the map resource loader `5A2B90` (try states 0..8,
//! catch `5A312F` on TGC_FileException): the map load fails, the record is not
//! appended and `4FC600`'s unwinding destroys the vector, so NO queued record
//! is created. [`ConversionError::BadDataVersion`] carries this outcome.
//!
//! No floating-point result survives: `59E300(0.0f)` multiplies the saved
//! scale word by +0.0 and the result is discarded (only x87 status flags could
//! differ), and the replacement loop copies words. The conversion is therefore
//! independent of the x87 precision control.
//!
//! Native containers become growable Rust structures: the conversion vector
//! (`4FC600` new[1], `4F9840`/`4F60C0` growth to max(need, 2*capacity), at
//! least 4) is a `Vec`; there is no count limit.
//!
//! Evidence: research/phase8-conversion (Unicorn oracle of the original code,
//! fixtures replayed by crates/aa-mapworld/tests/class_conversion.rs).
use crate::inventory::Placement;
use serde::Serialize;

/// Saved runtime class of a GObject record (the only convertible class).
pub const OBJECT_CLASS: u32 = 2;
/// `A2CAB8`, written at `4FB077` for a mismatch that is not converted.
pub const CHANGED_CLASS_WARNING: &str = "load_warning_changed_gclass|s";
/// `A2CE40`, written at `4FD721` when `4F9C10` returned non-null.
pub const REPLACED_WARNING: &str = "load_warning_replaced_changed_gclass|s";
/// `A2CE90`, written at `4FD674` when `4F9C10` returned null.
pub const CANT_REPLACE_WARNING: &str = "load_warning_cant_replace_changed_gclass|s";
/// `5A1040` call sites of the three warnings.
pub const CHANGED_CLASS_WARNING_SITE: u32 = 0x4FB077;
pub const CANT_REPLACE_WARNING_SITE: u32 = 0x4FD674;
pub const REPLACED_WARNING_SITE: u32 = 0x4FD721;
/// Thing part version limit (`4EF0C0` -> `8589B0(stream, 2)`).
pub const THING_VERSION_MAX: i32 = 2;
/// Object part version limit (`4EA050` -> `8589B0(stream, 5)`).
pub const OBJECT_VERSION_MAX: i32 = 5;

/// `4FAFA8..4FAFC3`: what `4FAE80` does once the definition is known.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ClassCheck {
    /// Same class: normal construction and Load (`4FB0A9` jump table).
    Matches,
    /// Saved class 2 with a conversion vector: [`read_changed_class_record`].
    Convert,
    /// Any other mismatch: [`CHANGED_CLASS_WARNING`] and a null result.
    Skip,
}

/// `4FAFA8..4FAFC3`. `conversion_context` is `4FAE80`'s fourth argument being
/// non-null (true from `4FC340`, false from `4FC200` and the TEAMS/KINGDOMS
/// loops of `4FC600`). The comparisons are exact 32-bit equality.
pub fn check_class(saved_class: u32, definition_class: u32, conversion_context: bool) -> ClassCheck {
    if definition_class == saved_class {
        ClassCheck::Matches
    } else if saved_class == OBJECT_CLASS && conversion_context {
        ClassCheck::Convert
    } else {
        ClassCheck::Skip
    }
}

/// A `5A1040` map warning whose text is the localized key (`4B9F40`)
/// formatted with the record IDS as its `%s`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NativeWarning {
    pub call_site: u32,
    pub key: &'static str,
    pub ids_utf16: Vec<u16>,
}

/// `4FAE80`'s first reads: `41D290` IDS (u16 count, UTF-16 units), the class
/// word and the saved ID word.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RecordPrefix {
    pub thing_id_utf16: Vec<u16>,
    pub class_code: u32,
    pub saved_id: u32,
    /// Bytes read.
    pub length: usize,
}

/// The 0x30-byte record of `4F26F0` as `4F9840` stores it (copy `4F5F20`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChangedClassRecord<D> {
    /// +00: the saved IDS (used only for the replacement warnings).
    pub thing_id_utf16: Vec<u16>,
    /// +04: the definition `427BD0` returned (translation applied).
    pub definition: D,
    /// +08: the saved ID (becomes the requested ID of the fresh creation).
    pub saved_id: u32,
    /// +0C: the saved Thing name. Carried but never used: the replacement
    /// block's name stays empty (`4FD574..4FD5B6`).
    pub name_utf16: Vec<u16>,
    /// +10: the Thing word read after the name. Carried, never used.
    pub word_10: u32,
    /// +14..+1C: saved position words.
    pub position_bits: [u32; 3],
    /// +20..+28: saved direction words; only x and y are passed on.
    pub direction_bits: [u32; 3],
    /// +2C: Object version >= 1 word (the saved variant), else 0xFFFFFFFF.
    pub variant: u32,
}

/// Words `4EF0C0`/`4EA050` read but do not store.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DiscardedReads {
    /// `4EF0C0` `8589B0` result (0..=2).
    pub thing_version: u32,
    /// The Thing flags word (`4EF136..4EF13F`, read into a dead stack slot).
    pub thing_flags: u32,
    /// The extra word of Thing version 1 (`4EF141..4EF155`).
    pub thing_legacy_word: Option<u32>,
    /// `4EA050` `8589B0` result (0..=5).
    pub object_version: u32,
    /// The saved scale multiplier: `59E300` multiplies it by +0.0f and stores
    /// the product in a dead stack slot.
    pub scale_multiplier_bits: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConversionRead<D> {
    pub record: ChangedClassRecord<D>,
    pub discarded: DiscardedReads,
    /// Bytes read from the record's first byte (prefix included).
    pub consumed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ConversionError {
    /// `8589B0` -> `41DFB0` "Bad data version" (TGC_LoadWarningException).
    /// Natively the map load fails (see the module documentation) and the
    /// conversion vector is discarded with every queued record.
    #[error(
        "Bad data version {version} (allowed 0..={max}) read by {reader:#X} at record offset {offset}; the native map load fails and no queued conversion is created"
    )]
    BadDataVersion {
        reader: u32,
        offset: usize,
        version: i32,
        max: i32,
    },
    /// The stream ended inside a read. The native chunk stream's error path
    /// (`858C90` -> `858BA0`) is not ported; this is never a success.
    #[error("read of {wanted} bytes at record offset {offset} passes the end of the supplied chunk data ({available} bytes)")]
    ReadPastChunk {
        offset: usize,
        wanted: usize,
        available: usize,
    },
    /// [`on_class_mismatch`] was called for a record whose classes match.
    #[error("record class {0} equals its definition class; it is not a class change")]
    NotAMismatch(u32),
    /// An inventory placement of class 2 without its decoded Object part.
    #[error("class-2 placement has no decoded Object part")]
    PlacementWithoutObject,
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], ConversionError> {
        match self.at.checked_add(n) {
            Some(end) if end <= self.bytes.len() => {
                let b = &self.bytes[self.at..end];
                self.at = end;
                Ok(b)
            }
            _ => Err(ConversionError::ReadPastChunk {
                offset: self.at,
                wanted: n,
                available: self.bytes.len(),
            }),
        }
    }
    fn u32(&mut self) -> Result<u32, ConversionError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    /// `41D290`: u16 unit count, then the units (no limit, no validation).
    fn string(&mut self) -> Result<Vec<u16>, ConversionError> {
        let n = usize::from(u16::from_le_bytes(self.take(2)?.try_into().unwrap()));
        Ok(self
            .take(2 * n)?
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .collect())
    }
    /// One 12-byte stream read (`4EF10B`/`4EF117`).
    fn vector(&mut self) -> Result<[u32; 3], ConversionError> {
        let b = self.take(12)?;
        let w = |i: usize| u32::from_le_bytes(b[4 * i..4 * i + 4].try_into().unwrap());
        Ok([w(0), w(1), w(2)])
    }
    /// `8589B0` on a reading stream: signed word, `v < 0 || v > max` throws.
    fn version(&mut self, max: i32, reader: u32) -> Result<u32, ConversionError> {
        let offset = self.at;
        let v = self.u32()? as i32;
        if v < 0 || v > max {
            return Err(ConversionError::BadDataVersion {
                reader,
                offset,
                version: v,
                max,
            });
        }
        Ok(v as u32)
    }
}

/// `4FAEDE..4FAEFE` on a stream positioned at the record's first byte.
pub fn read_prefix(stream: &[u8]) -> Result<RecordPrefix, ConversionError> {
    let mut r = Reader { bytes: stream, at: 0 };
    let thing_id_utf16 = r.string()?;
    let class_code = r.u32()?;
    let saved_id = r.u32()?;
    Ok(RecordPrefix {
        thing_id_utf16,
        class_code,
        saved_id,
        length: r.at,
    })
}

/// `4FAEDE..4FAEFE` then `4EA050` (`4EF0C0`, `8589B0`, `59E300`, variant
/// word), on `stream` positioned at the record's first byte. `stream` must
/// run to the end of the section's data chunk: natively nothing bounds these
/// reads by the record's index length.
pub fn read_changed_class_record<D>(
    stream: &[u8],
    definition: D,
) -> Result<ConversionRead<D>, ConversionError> {
    let mut r = Reader { bytes: stream, at: 0 };
    let thing_id_utf16 = r.string()?;
    let _class = r.u32()?;
    let saved_id = r.u32()?;
    // 4EF0C0
    let thing_version = r.version(THING_VERSION_MAX, 0x4EF0C0)?;
    let name_utf16 = r.string()?;
    let word_10 = r.u32()?;
    let position_bits = r.vector()?;
    let direction_bits = r.vector()?;
    let thing_flags = r.u32()?;
    let thing_legacy_word = if thing_version == 1 {
        Some(r.u32()?)
    } else {
        None
    };
    // 4EA050
    let object_version = r.version(OBJECT_VERSION_MAX, 0x4EA050)?;
    let scale_multiplier_bits = r.u32()?;
    let variant = if object_version >= 1 {
        r.u32()?
    } else {
        u32::MAX
    };
    Ok(ConversionRead {
        record: ChangedClassRecord {
            thing_id_utf16,
            definition,
            saved_id,
            name_utf16,
            word_10,
            position_bits,
            direction_bits,
            variant,
        },
        discarded: DiscardedReads {
            thing_version,
            thing_flags,
            thing_legacy_word,
            object_version,
            scale_multiplier_bits,
        },
        consumed: r.at,
    })
}

/// The same conversion built from an inventory [`Placement`]. The inventory
/// decoder reads a superset of these fields in the same order with the same
/// version limits (Thing <= 2, Object <= 5), so for a decoded class-2
/// placement the result equals [`read_changed_class_record`] on its bytes. A
/// class-2 record the decoder rejected (in `Inventory::unsupported`) must use
/// the byte path, which reads only what `4EA050` reads.
pub fn read_placement_record<D>(
    placement: &Placement,
    definition: D,
) -> Result<ConversionRead<D>, ConversionError> {
    let thing = &placement.thing;
    let object = placement
        .object
        .as_ref()
        .ok_or(ConversionError::PlacementWithoutObject)?;
    let variant = if object.version >= 1 {
        object
            .variant_index
            .map(|v| v as u32)
            .ok_or(ConversionError::PlacementWithoutObject)?
    } else {
        u32::MAX
    };
    let consumed = 2
        + 2 * placement.thing_id_utf16.len()
        + 8
        + 4
        + 2
        + 2 * thing.name_utf16.len()
        + 4
        + 12
        + 12
        + 4
        + if thing.version == 1 { 4 } else { 0 }
        + 4
        + 4
        + if object.version >= 1 { 4 } else { 0 };
    Ok(ConversionRead {
        record: ChangedClassRecord {
            thing_id_utf16: placement.thing_id_utf16.clone(),
            definition,
            saved_id: placement.serialized_id,
            name_utf16: thing.name_utf16.clone(),
            word_10: thing.word_1c,
            position_bits: thing.position_bits,
            direction_bits: thing.facing_bits,
            variant,
        },
        discarded: DiscardedReads {
            thing_version: thing.version,
            thing_flags: thing.serialized_flags,
            thing_legacy_word: thing.legacy_version_one_word,
            object_version: object.version,
            scale_multiplier_bits: object.scale_multiplier_bits,
        },
        consumed,
    })
}

/// `4FC600`'s conversion vector (`[esp+3C]`, created at `4FCE64..4FCEA8`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConversionQueue<D> {
    records: Vec<ChangedClassRecord<D>>,
}
impl<D> Default for ConversionQueue<D> {
    fn default() -> Self {
        Self {
            records: Vec::new(),
        }
    }
}
impl<D> ConversionQueue<D> {
    pub fn new() -> Self {
        Self::default()
    }
    /// `4F9840`: append a copy at the end; returns its index.
    pub fn push(&mut self, record: ChangedClassRecord<D>) -> usize {
        self.records.push(record);
        self.records.len() - 1
    }
    pub fn records(&self) -> &[ChangedClassRecord<D>] {
        &self.records
    }
    pub fn len(&self) -> usize {
        self.records.len()
    }
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
    pub fn into_records(self) -> Vec<ChangedClassRecord<D>> {
        self.records
    }
}

/// Result of [`on_class_mismatch`]. In both cases `4FAE80` yields null and
/// `4FC340` continues at the next index entry (record start + index length).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum MismatchOutcome {
    /// `4FAFC5..4FB032`: appended to the queue at `index`.
    Converted { index: usize, consumed: usize },
    /// `4FB034..4FB0A4`: warning only; `consumed` is the prefix length.
    Skipped {
        warning: NativeWarning,
        consumed: usize,
    },
}

/// Hook for the restoration record loop at the `4FAFA8` class compare, after
/// the generation check (`4FAF14`) and a non-null Thing lookup (`427BD0`).
/// `stream` starts at the record's first byte and runs to the end of the
/// section's data chunk; `queue` is `Some` for the 4FC340 sections and `None`
/// for TEAMS/KINGDOMS and embedded (`4FC200`) records.
pub fn on_class_mismatch<D>(
    stream: &[u8],
    definition: D,
    definition_class: u32,
    queue: Option<&mut ConversionQueue<D>>,
) -> Result<MismatchOutcome, ConversionError> {
    let prefix = read_prefix(stream)?;
    mismatch(prefix, definition_class, queue, |queue| {
        let read = read_changed_class_record(stream, definition)?;
        Ok((queue.push(read.record), read.consumed))
    })
}

/// [`on_class_mismatch`] for a decoded inventory placement (see
/// [`read_placement_record`] for when this equals the byte path).
pub fn on_placement_class_mismatch<D>(
    placement: &Placement,
    definition: D,
    definition_class: u32,
    queue: Option<&mut ConversionQueue<D>>,
) -> Result<MismatchOutcome, ConversionError> {
    let prefix = RecordPrefix {
        thing_id_utf16: placement.thing_id_utf16.clone(),
        class_code: placement.class_code,
        saved_id: placement.serialized_id,
        length: 2 + 2 * placement.thing_id_utf16.len() + 8,
    };
    mismatch(prefix, definition_class, queue, |queue| {
        let read = read_placement_record(placement, definition)?;
        Ok((queue.push(read.record), read.consumed))
    })
}

fn mismatch<D>(
    prefix: RecordPrefix,
    definition_class: u32,
    queue: Option<&mut ConversionQueue<D>>,
    convert: impl FnOnce(&mut ConversionQueue<D>) -> Result<(usize, usize), ConversionError>,
) -> Result<MismatchOutcome, ConversionError> {
    match (
        check_class(prefix.class_code, definition_class, queue.is_some()),
        queue,
    ) {
        (ClassCheck::Matches, _) => Err(ConversionError::NotAMismatch(prefix.class_code)),
        (ClassCheck::Convert, Some(queue)) => {
            let (index, consumed) = convert(queue)?;
            Ok(MismatchOutcome::Converted { index, consumed })
        }
        _ => Ok(MismatchOutcome::Skipped {
            warning: NativeWarning {
                call_site: CHANGED_CLASS_WARNING_SITE,
                key: CHANGED_CLASS_WARNING,
                ids_utf16: prefix.thing_id_utf16,
            },
            consumed: prefix.length,
        }),
    }
}

/// The creation-parameter block `4FD490..4FD5BF` passes to `4F9C10` (object
/// type `A25B1C`, 0x28 bytes). Its virtuals: +0 and +4 `6A71F0` (false), +8
/// `6A71F0` (false), +C `6A71D0` (true); `4F9C10` refuses Actor/Group
/// definitions (class 3/8, "... requires a GACD") and Kingdom definitions
/// (class 4, "... requires a GKCD") for such a block.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ObjectCreationParams<D> {
    /// +04: the queued definition.
    pub definition: D,
    /// +08..+10: record +14..+1C.
    pub position_bits: [u32; 3],
    /// +14/+18: record +20/+24 (constructor default (0, -1.0f) overwritten).
    pub direction_bits: [u32; 2],
    /// +1C: always empty (fresh allocator string, emptied at `4FD574`).
    pub name_utf16: Vec<u16>,
    /// +20: [`creation_variant`] (constructor default -1).
    pub variant: u32,
    /// +24: record +08, the saved ID (constructor default 0).
    pub requested_id: u32,
}
impl<D> ObjectCreationParams<D> {
    pub const VTABLE: u32 = 0xA25B1C;
}

/// The definition reads of `4FD535..4FD56E` in native order.
pub trait VariationSource<D> {
    type Error;
    /// Definition virtual +30: `4C9FA0` (false) for ThingData, `4F0500`
    /// (true) for ObjectData and its derived classes (Actor, DynamicDoodad).
    fn is_object_data(&mut self, definition: &D) -> Result<bool, Self::Error>;
    /// ObjectData+108 byte: `aspects_as_variations` (`49DC40`).
    fn variations_enabled(&mut self, definition: &D) -> Result<bool, Self::Error>;
    /// ObjectData+110: aspect count of the +10C list (AspectNormal first when
    /// variations are disabled, then every Aspect child; `49DC40`/`49C670`).
    fn aspect_count(&mut self, definition: &D) -> Result<i32, Self::Error>;
}

/// Definition virtual +30 for the data class `4BCA00`
/// (KKC_ThingDataManager::CreateThingData) constructs for a category's native
/// kind: `4F0500` (true) for kinds 1 (ObjectData A13318), 2 (ActorData
/// A05540), 3 (A0B218), 18 (A133B0), 19 (A09B30), 20 (A13448) and 21
/// (DynamicDoodadData A0F6E0); `4C9FA0` (false) for kinds 0 and 4..=17, 22,
/// 23. `None` for a kind `4BCA00` has no case for.
pub fn native_kind_is_object_data(native_kind: u32) -> Option<bool> {
    match native_kind {
        1 | 2 | 3 | 18 | 19 | 20 | 21 => Some(true),
        0 | 4..=17 | 22 | 23 => Some(false),
        _ => None,
    }
}

/// `4FD535..4FD56E`: the variant passed on. Signed compare: a saved variant
/// below the aspect count (including any negative value) is kept, any other
/// becomes 0; without an ObjectData definition, with variations disabled or
/// with no aspects it is -1.
pub fn creation_variant<D, S: VariationSource<D> + ?Sized>(
    source: &mut S,
    definition: &D,
    saved_variant: u32,
) -> Result<u32, S::Error> {
    if !source.is_object_data(definition)? {
        return Ok(u32::MAX);
    }
    if !source.variations_enabled(definition)? {
        return Ok(u32::MAX);
    }
    let count = source.aspect_count(definition)?;
    if count <= 0 {
        return Ok(u32::MAX);
    }
    Ok(if saved_variant as i32 >= count {
        0
    } else {
        saved_variant
    })
}

/// The fresh-creation seam: `KKC_GWorld::CreateGThing` (`4F9C10`), ported by
/// the restoration agent (world/actor creation `4F9C10`/`4FBF40`). `Ok(None)`
/// is the native null result (the "can't replace" warning follows); `Err` is
/// a Rust-side unported path and stops the replacement loop.
pub trait ReplacementWorld<D>: VariationSource<D> {
    type Created;
    fn create_thing(
        &mut self,
        params: &ObjectCreationParams<D>,
    ) -> Result<Option<Self::Created>, Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Replacement<C> {
    /// Index in the conversion queue.
    pub index: usize,
    pub saved_id: u32,
    pub created: Option<C>,
    pub warning: NativeWarning,
}

/// A Rust-side error stopped [`replace_converted`] at queue entry `index`;
/// `completed` lists the entries already created (their side effects stay).
#[derive(Debug)]
pub struct ReplacementError<C, E> {
    pub completed: Vec<Replacement<C>>,
    pub index: usize,
    pub error: E,
}

/// Result of [`replace_converted`].
pub type ReplacementResult<C, E> = Result<Vec<Replacement<C>>, ReplacementError<C, E>>;

/// `4FD47C..4FD781`: for each queued record in order, build the parameter
/// block, create it fresh with `4F9C10`, write the replaced / can't replace
/// warning. Runs once, as the last step of the map load (`4FC600`), after
/// every section and the PostLoad pass.
pub fn replace_converted<D, W: ReplacementWorld<D> + ?Sized>(
    queue: ConversionQueue<D>,
    world: &mut W,
) -> ReplacementResult<W::Created, W::Error> {
    let mut completed = Vec::with_capacity(queue.len());
    for (index, record) in queue.into_records().into_iter().enumerate() {
        let variant = match creation_variant(world, &record.definition, record.variant) {
            Ok(v) => v,
            Err(error) => {
                return Err(ReplacementError {
                    completed,
                    index,
                    error,
                });
            }
        };
        let params = ObjectCreationParams {
            definition: record.definition,
            position_bits: record.position_bits,
            direction_bits: [record.direction_bits[0], record.direction_bits[1]],
            name_utf16: Vec::new(),
            variant,
            requested_id: record.saved_id,
        };
        let created = match world.create_thing(&params) {
            Ok(c) => c,
            Err(error) => {
                return Err(ReplacementError {
                    completed,
                    index,
                    error,
                });
            }
        };
        let warning = if created.is_some() {
            NativeWarning {
                call_site: REPLACED_WARNING_SITE,
                key: REPLACED_WARNING,
                ids_utf16: record.thing_id_utf16,
            }
        } else {
            NativeWarning {
                call_site: CANT_REPLACE_WARNING_SITE,
                key: CANT_REPLACE_WARNING,
                ids_utf16: record.thing_id_utf16,
            }
        };
        completed.push(Replacement {
            index,
            saved_id: record.saved_id,
            created,
            warning,
        });
    }
    Ok(completed)
}
