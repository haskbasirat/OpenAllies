//!ActorData+25C/+260/+264 TrackInfo list, written by ActorData::LoadData
//!464BD0's MoverComponent branch 465DA4..465F74 (original and v14 bytes equal).
//!4624B0 constructs +25C with one 4BDDB0 entry, +260=0, +264=1; 45E880 grows
//!it (no cap). For every `TrackInfo` child the entry is appended (+260 is
//!incremented) BEFORE any field is read, then, in this order:
//!
//! 1. 5B54D0 `track_type` and 5B54D0 `track_texture` (CString copies);
//! 2. 405D60 formats track_texture with ActorData+B0 (`resource_prefix`), then
//!    4763C0 makes it GameData-source-relative;
//! 3. +00 = 497220 lookup of track_type in the B3B54C table (-1 when absent);
//! 4. +0C = 5A77E0(path, 0) Texture;
//! 5. +04 byte = 5B5380 `track_texture_mirrored`;
//! 6. +08 = 5B4AE0 Float `track_placement_pcg` (fstp dword);
//! 7. +10/+14 = 5B4D80 Point2 `track_offset`;
//! 8. +18/+1C = 5B4D80 Point2 `track_size`;
//! 9. +20 = 5B4AE0 Float `fade_sec` (fstp dword);
//!10. +00 < 0 raises 410C40 "Unknown track type '%s' on object '%s'" (throws).
//!
//!B3B54C is built by static init 9CD190 -> 4BDFF0(this=B3B53C, 1): DISCRETE=0
//!then CONTINUOUS=1 (4BDEE0). The lookup hashes with 405030 (folds a-z) and
//!compares with _wcsicmp; AA.exe only ever calls setlocale for LC_NUMERIC
//!(7F1965/7F1A39), so the CRT stays in the C LC_CTYPE (A-Z folding only).
//!Proof: research/development10/view-actor-tracks/native-track-info.json.
//!Bytes +05..+07 of an entry are never written (4BDDB0 and 45E880 skip them).
use super::super::{Data, Result, State};
use crate::{
    catalog::Catalog,
    definitions::Bank,
    resource_depot::Resources,
    resource_objects::{Handle, TexturePolicy},
};
use aa_mapworld::restore::RuntimeDefinition;
use aa_numeric::Precision;
use aa_rules::{
    actors::ResolvedNode,
    native::{ScalarType, TypedValue},
    resources::Provenance,
};

///B3B54C values registered by 4BDFF0.
pub const DISCRETE: i32 = 0;
pub const CONTINUOUS: i32 = 1;

///One 0x24-byte ActorData+25C entry. `texture` is the +0C 5A77E0 pointer
///(None: NULL). There is no value for the unwritten bytes +05..+07.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackInfo<T> {
    ///+00 (497220 value or -1)
    pub track_type: i32,
    ///+04 byte
    pub mirrored: u8,
    ///+08 binary32
    pub placement_pcg_bits: u32,
    ///+0C
    pub texture: Option<T>,
    ///+10/+14 binary32
    pub offset_bits: [u32; 2],
    ///+18/+1C binary32
    pub size_bits: [u32; 2],
    ///+20 binary32
    pub fade_sec_bits: u32,
}
impl<T> TrackInfo<T> {
    ///4BDDB0: +00/+04 byte/+08/+0C/+10/+14/+18/+1C/+20 = 0.
    pub const fn constructed() -> Self {
        Self {
            track_type: 0,
            mirrored: 0,
            placement_pcg_bits: 0,
            texture: None,
            offset_bits: [0; 2],
            size_bits: [0; 2],
            fade_sec_bits: 0,
        }
    }
}

///The 464BD0 reads of one TrackInfo child, in native call order.
pub trait Source {
    type Texture;
    ///5B54D0 GetString.
    fn string(&mut self, key: &'static str) -> Result<String>;
    ///405D60(out, format, ActorData+B0) then 4763C0(GameData, &out).
    fn path(&mut self, format: &str) -> Result<String>;
    ///5A77E0(path, 0).
    fn texture(&mut self, path: &str) -> Result<Option<Self::Texture>>;
    ///5B5380 GetFlag.
    fn flag(&mut self, key: &'static str) -> Result<bool>;
    ///5B4AE0 GetFloat, stored by fstp dword.
    fn float(&mut self, key: &'static str) -> Result<u32>;
    ///5B4D80 GetPoint2.
    fn point(&mut self, key: &'static str) -> Result<[u32; 2]>;
}

///497220 over the 4BDFF0 table. Under the C LC_CTYPE _wcsicmp folds only
///A-Z, so a non-ASCII name never equals a registered one; a NUL would end
///the native compare and hash early and is outside the admitted domain.
pub fn track_type_497220(name: &str) -> Result<i32> {
    if name.contains('\0') {
        return Err("497220 track_type with an embedded NUL is outside the admitted domain".into());
    }
    Ok(if name.eq_ignore_ascii_case("DISCRETE") {
        DISCRETE
    } else if name.eq_ignore_ascii_case("CONTINUOUS") {
        CONTINUOUS
    } else {
        -1
    })
}

///465E1D..465F66 for the already appended `entry`. Each store lands in
///`entry` before the next read, so a failing read leaves the native prefix.
pub fn read_entry<S: Source>(s: &mut S, object: &str, entry: &mut TrackInfo<S::Texture>) -> Result<()> {
    let kind = s.string("track_type")?;
    let format = s.string("track_texture")?;
    let path = s.path(&format)?;
    entry.track_type = track_type_497220(&kind)?;
    entry.texture = s.texture(&path)?;
    entry.mirrored = u8::from(s.flag("track_texture_mirrored")?);
    entry.placement_pcg_bits = s.float("track_placement_pcg")?;
    entry.offset_bits = s.point("track_offset")?;
    entry.size_bits = s.point("track_size")?;
    entry.fade_sec_bits = s.float("fade_sec")?;
    if entry.track_type < 0 {
        return Err(format!(
            "464BD0 410C40 \"Unknown track type '{kind}' on object '{object}'\""
        ));
    }
    Ok(())
}

///The prepared +25C list of one canonical ActorData allocation. Entries that
///were appended before a failure stay (with their Texture pointers).
#[derive(Debug)]
pub struct Prepared {
    sources: Vec<Vec<Provenance>>,
    entries: Vec<TrackInfo<Handle>>,
    complete: bool,
    failed: Option<String>,
}

///Production reads over the SAME resolved TrackInfo child and source PC.
struct Child<'n, 'a, 'c, 'r, 'p, P> {
    node: &'n ResolvedNode<'a>,
    pc: Precision,
    folder: &'n str,
    prefix: Option<&'n str>,
    catalog: &'n Catalog<'c>,
    bank: &'n Bank,
    resources: &'r mut Resources<'p>,
    policy: &'r mut P,
}
impl<P> Child<'_, '_, '_, '_, '_, P> {
    fn value(&self, key: &str) -> Result<TypedValue> {
        self.node
            .field(key, self.pc)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("TrackInfo {key}: native getter throws \"Node doesn't contain field.\""))
    }
}
impl<P: TexturePolicy> Source for Child<'_, '_, '_, '_, '_, P> {
    type Texture = Handle;
    fn string(&mut self, key: &'static str) -> Result<String> {
        match self.value(key)? {
            TypedValue::String { value, .. } if value.is_ascii() && !value.contains('\0') => Ok(value),
            _ => Err(format!("TrackInfo {key} requires a native plain String source")),
        }
    }
    fn path(&mut self, format: &str) -> Result<String> {
        //405D60 reads Data+B0 only through a conversion in the format.
        let prefix = match (format.contains('%'), self.prefix) {
            (false, prefix) => prefix.unwrap_or_default(),
            (true, Some(prefix)) => prefix,
            (true, None) => {
                return Err("405D60 TrackInfo format needs ActorData+B0 without a View resource_prefix source".into());
            }
        };
        let expanded = super::super::path::expand(format, prefix)?;
        super::super::path::relative(self.folder, &expanded)
    }
    fn texture(&mut self, path: &str) -> Result<Option<Handle>> {
        if path.is_empty() {
            return Err("5A77E0 with an empty TrackInfo texture name is unproven".into());
        }
        let ids: Vec<u16> = path.encode_utf16().collect();
        self.resources
            .texture_5a77e0_with_policy(self.catalog, self.bank, &ids, 0, &mut *self.policy)
    }
    fn flag(&mut self, key: &'static str) -> Result<bool> {
        match self.value(key)? {
            TypedValue::Boolean { value, .. } => Ok(value),
            _ => Err(format!("TrackInfo {key} requires a native Flag source")),
        }
    }
    fn float(&mut self, key: &'static str) -> Result<u32> {
        match self.value(key)? {
            //The getter's float10 came from this binary32; fstp dword keeps it.
            TypedValue::Scalar(v) if v.field_type == ScalarType::Float && f32::from_bits(v.bits).is_finite() => Ok(v.bits),
            _ => Err(format!("TrackInfo {key} requires a finite native Float source")),
        }
    }
    fn point(&mut self, key: &'static str) -> Result<[u32; 2]> {
        match self.value(key)? {
            TypedValue::Point { bits, .. } if bits.len() == 2 => Ok([bits[0], bits[1]]),
            _ => Err(format!("TrackInfo {key} requires a native Point2 source")),
        }
    }
}

impl State {
    ///464BD0 465DA4..465F74 for THIS actor, inside the pre-World
    ///source-resource interval (5A77E0 runs at LoadData, never at view Init).
    ///Call it in the MoverComponent branch before that component's FX lists;
    ///an absent MoverComponent keeps the 4624B0 empty list.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_actor_track_infos(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        d: &RuntimeDefinition,
        data: &Data,
        actor: &ResolvedNode<'_>,
        pc: Precision,
        resources: &mut Resources<'_>,
        texture_policy: &mut impl TexturePolicy,
    ) -> Result<()> {
        if let Some(p) = &self.entry(data)?.actor_tracks {
            return match &p.failed {
                Some(error) => Err(error.clone()),
                None if p.complete => Ok(()),
                None => Err("recursive ActorData TrackInfo preparation".into()),
            };
        }
        self.entry_mut(data)?.actor_tracks = Some(Prepared {
            sources: vec![],
            entries: vec![],
            complete: false,
            failed: None,
        });
        let result = self.read_track_infos(catalog, bank, d, data, actor, pc, resources, texture_policy);
        let p = self.entry_mut(data)?.actor_tracks.as_mut().expect("same prepared TrackInfo list");
        match &result {
            Ok(()) => p.complete = true,
            Err(error) => p.failed = Some(error.clone()),
        }
        result
    }
    #[allow(clippy::too_many_arguments)]
    fn read_track_infos(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        d: &RuntimeDefinition,
        data: &Data,
        actor: &ResolvedNode<'_>,
        pc: Precision,
        resources: &mut Resources<'_>,
        texture_policy: &mut impl TexturePolicy,
    ) -> Result<()> {
        let Some(mover) = actor.child("MoverComponent").map_err(|e| e.to_string())? else {
            return Ok(());
        };
        //ActorData+B0 is the 49DC40 View resource_prefix copy.
        let prefix = match actor.child("View").map_err(|e| e.to_string())? {
            Some(view) => match view.field("resource_prefix", pc).map_err(|e| e.to_string())? {
                Some(TypedValue::String { value, .. }) if value.is_ascii() && !value.contains('\0') => Some(value),
                _ => return Err("ActorData+B0 requires the native View resource_prefix String".into()),
            },
            None => None,
        };
        let normalized = d.proof.source.path.replace('\\', "/");
        let folder = normalized.rsplit_once('/').map_or("", |(f, _)| f).to_owned();
        let children = mover.children("TrackInfo").map_err(|e| e.to_string())?;
        i32::try_from(children.len()).map_err(|_| "ActorData260 TrackInfo count exceeds the native signed domain")?;
        for child in &children {
            let p = self.entry_mut(data)?.actor_tracks.as_mut().expect("same prepared TrackInfo list");
            p.sources.push(child.sources());
            p.entries.push(TrackInfo::constructed());
            let mut entry = TrackInfo::constructed();
            let mut reader = Child {
                node: child,
                pc,
                folder: &folder,
                prefix: prefix.as_deref(),
                catalog,
                bank,
                resources: &mut *resources,
                policy: &mut *texture_policy,
            };
            let result = read_entry(&mut reader, &d.proof.id, &mut entry);
            let p = self.entry_mut(data)?.actor_tracks.as_mut().expect("same prepared TrackInfo list");
            *p.entries.last_mut().expect("just appended") = entry;
            result?;
        }
        Ok(())
    }
    ///45E710(i): the prepared entry i of THIS view's canonical ActorData.
    ///Count +260 stays the source TrackInfo list the view already read.
    pub fn actor_view_track_info_45e710(
        &self,
        view: &super::ActorView,
        index: usize,
    ) -> Result<TrackInfo<Handle>> {
        let fields = self.actor_view_fields(view)?;
        if !fields.actor {
            return Err("non-ActorData25C is unbound".into());
        }
        let p = self
            .entry(&view.data)?
            .actor_tracks
            .as_ref()
            .ok_or("ActorData25C requires the source-prepared 464BD0 TrackInfo list")?;
        if let Some(error) = &p.failed {
            return Err(error.clone());
        }
        if !p.complete {
            return Err("ActorData25C TrackInfo preparation is unfinished".into());
        }
        if p.sources != fields.tracks {
            return Err("ActorData25C prepared TrackInfo sources differ from the +260 source list".into());
        }
        p.entries
            .get(index)
            .cloned()
            .ok_or_else(|| "45E710 index outside the actual ActorData+260 list".into())
    }
}

#[cfg(test)]
#[path = "data_actor_track_info_tests.rs"]
mod tests;
