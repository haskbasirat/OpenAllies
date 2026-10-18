//!Opaque same-Application Resource identities. Cloning a Handle only retains the
//!same allocation capability; native Find/reference counts remain explicit
//!operations on the one ResourceManager owner. No decoder/readiness payload is
//!copied or manufactured by this type.
use aa_rules::sources::{CatalogEntry, Origin};
use std::sync::{Arc, Weak};
type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Texture,
    Model,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    logical_path: String,
    size: u64,
    origin: Origin,
}
impl SourceFile {
    pub fn logical_path(&self) -> &str {
        &self.logical_path
    }
    pub fn size(&self) -> u64 {
        self.size
    }
    pub fn origin(&self) -> &Origin {
        &self.origin
    }
    pub(crate) fn from_catalog(entry: &CatalogEntry) -> Self {
        Self {
            logical_path: entry.logical_path.clone(),
            size: entry.size,
            origin: entry.origin.clone(),
        }
    }
}
struct Identity {
    kind: Kind,
    ids: Vec<u16>,
    source: SourceFile,
}
///An identity-only shared reference to ONE manager-owned allocation. A raw
///pointer, caller-supplied index or resource-loaded claim cannot create one.
#[derive(Clone)]
pub struct Handle {
    application: Weak<()>,
    identity: Arc<Identity>,
}
impl Handle {
    pub fn kind(&self) -> Kind {
        self.identity.kind
    }
    pub fn logical_ids(&self) -> &[u16] {
        &self.identity.ids
    }
    pub fn source_file(&self) -> &SourceFile {
        &self.identity.source
    }
    pub fn same_identity(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.identity, &other.identity)
            && Weak::ptr_eq(&self.application, &other.application)
    }
    pub(crate) fn require_application(&self, origin: &Arc<()>) -> Result<()> {
        if self
            .application
            .upgrade()
            .as_ref()
            .is_none_or(|actual| !Arc::ptr_eq(actual, origin))
        {
            return Err("Resource belongs to a retired/different Application manager".into());
        }
        Ok(())
    }
    ///Only the real Application factory may issue a Resource allocation.
    pub(crate) fn new(origin: &Arc<()>, kind: Kind, ids: Vec<u16>, source: SourceFile) -> Self {
        Self {
            application: Arc::downgrade(origin),
            identity: Arc::new(Identity { kind, ids, source }),
        }
    }
}

///5A60E0's field projection. Slots54/58 are intentionally unavailable:
///the constructor never stores them, even though a zeroed host allocation
///would happen to contain zero. Native Task has its own shared owner.
pub(crate) struct Entry {
    pub(crate) handle: Handle,
    pub(crate) requests4: u32,
    pub(crate) flags8: u32,
    pub(crate) references14: u32,
    pub(crate) task: Arc<crate::resource_task::Task>,
    pub(crate) locations: Vec<SourceFile>,
    pub(crate) texture: Option<TextureFields>,
    pub(crate) model: Option<ModelFields>,
}
pub(crate) struct TextureFields {
    pub(crate) source3c: Option<()>,
    pub(crate) writable_child40: Option<Handle>,
    pub(crate) rect44: [u32; 4],
    pub(crate) word54: Option<u32>,
    pub(crate) word58: Option<u32>,
    pub(crate) render_count60: usize,
    pub(crate) render_slots: Vec<[u32; 9]>,
}
impl Entry {
    ///Real5AA890→5A60E0 constructor, successful-allocation domain. Static
    ///packed-render defaults are5A4C40's [2,0,0,0,0,0,8,0x22,0]; their native
    ///global and constructor proof is pinned by native-resource-constructors.
    ///No positive native texture object or loaded flag is constructed.
    pub(crate) fn texture(
        origin: &Arc<()>,
        ids: Vec<u16>,
        source: SourceFile,
        priority: &crate::resource_task::DefaultPriority,
    ) -> Result<Self> {
        let handle = Handle::new(origin, Kind::Texture, ids, source);
        let task = crate::resource_task::Task::construct_for_resource(&handle, priority)?;
        task.retain_in_resource_5aa944()?;
        Ok(Self {
            handle,
            requests4: 0,
            flags8: 0x200010,
            references14: 0,
            task: Arc::new(task),
            locations: Vec::new(),
            model: None,
            texture: Some(TextureFields {
                source3c: None,
                writable_child40: None,
                rect44: [0, 0, 0x3f800000, 0x3f800000],
                word54: None,
                word58: None,
                render_count60: 0,
                render_slots: vec![[2, 0, 0, 0, 0, 0, 8, 0x22, 0]],
            }),
        })
    }
    ///5AAAE0's first selected physical depot. With no prior loaded texture,
    ///5AA660 clears flags1/2 and absent-location bit4, then5ACF60 relocates
    ///this same allocation to the active-name cache. No second Entry is made.
    pub(crate) fn attach_selected_source(&mut self, source: SourceFile) -> Result<()> {
        if !self.locations.is_empty() || self.requests4 != 0 || self.flags8 & 1 != 0 {
            return Err(
                "Resource selected-depot attachment needs actual prior-location/unload services"
                    .into(),
            );
        }
        self.locations.push(source);
        self.flags8 &= !0x16;
        Ok(())
    }
    pub(crate) fn increment_reference14(&mut self) -> u32 {
        self.references14 = self.references14.wrapping_add(1);
        self.references14
    }
}
///Native extension table installed by9D0900 and searched by5AC5F0. Other
///classes stay explicit; a file extension is never a decoder-ready claim.
pub(crate) fn class_for_name(ids: &[u16]) -> Result<Option<Kind>> {
    let key = crate::resource_map_cache::normalized_key(ids)?;
    let name = key.rsplit('/').next().unwrap_or("");
    let extension = name.rsplit_once('.').map(|(_, v)| v).unwrap_or("");
    Ok(match extension {
        "tga" | "dds" | "png" => Some(Kind::Texture),
        "nif" => Some(Kind::Model),
        _ => None,
    })
}
///85A550/859B70/85ABB0's admitted ASCII path composition. Actual source
///aliases contain no embeddedNUL; other counted-path forms remain required.
pub(crate) fn subfolder_candidate(ids: &[u16], subfolder: &[u16]) -> Result<Vec<u16>> {
    if ids.contains(&0) || subfolder.contains(&0) || ids.iter().chain(subfolder).any(|&v| v > 127) {
        return Err("native Resource counted path is outside proved plain ASCII domain".into());
    }
    let split = ids
        .iter()
        .rposition(|&v| v == u16::from(b'/') || v == u16::from(b'\\'));
    let (dir, base) = if let Some(i) = split {
        (&ids[..i], &ids[i + 1..])
    } else if let Some(i) = ids.iter().rposition(|&v| v == u16::from(b':')) {
        (&ids[..=i], &ids[i + 1..])
    } else {
        (&[][..], ids)
    };
    fn join(a: &[u16], b: &[u16]) -> Vec<u16> {
        let mut out = a.to_vec();
        if b.first().is_some_and(|&v| v == 47 || v == 92) {
            if out.last().is_some_and(|&v| v == 47 || v == 92) {
                out.pop();
            }
        } else if out.last().is_some_and(|&v| v != 47 && v != 92) {
            out.push(47);
        }
        out.extend_from_slice(b);
        out
    }
    Ok(join(&join(dir, subfolder), base))
}

///The concrete client implementation reads its SAME retained renderer-manager
///and pipeline epoch at this exact native73A560 boundary. An unavailable current
///renderer/flag/shader owner is an error, never a default false projection.
pub trait ModelPolicy {
    fn use_hardware_model_73a560(&mut self) -> Result<bool>;
    fn hardware_model_suffix_40f7b0(&mut self) -> Result<Vec<u16>> {
        Err("actual current AVar50 RendererHBPPSuffix is required after73A560true".into())
    }
}
pub(crate) struct ModelFields {
    pub(crate) source3c: Option<()>,
    pub(crate) source40: Option<()>,
    pub(crate) word44: u32,
    pub(crate) value48: u32,
    pub(crate) words4c_60: [u32; 6],
    pub(crate) radius64: u32,
    pub(crate) factors68_74: [u32; 4],
    pub(crate) words78_7c: [u32; 2],
    pub(crate) slots80: Vec<(Arc<[u16]>, Arc<[u16]>)>,
    pub(crate) count84: usize,
    pub(crate) name8c: Arc<[u16]>,
}
impl Entry {
    ///5A7DB0 real NiAVObjResource constructor. Positive NiAVObject pointers are
    ///NULL; radius64's constructor1 is not used by457A40 while flags8bit2 is0.
    pub(crate) fn model(
        origin: &Arc<()>,
        ids: Vec<u16>,
        source: SourceFile,
        priority: &crate::resource_task::DefaultPriority,
    ) -> Result<Self> {
        let handle = Handle::new(origin, Kind::Model, ids, source);
        let task = crate::resource_task::Task::construct_for_resource(&handle, priority)?;
        task.retain_in_resource_5aa944()?;
        Ok(Self {
            handle,
            requests4: 0,
            flags8: 0x410,
            references14: 0,
            task: Arc::new(task),
            locations: Vec::new(),
            texture: None,
            model: Some(ModelFields {
                source3c: None,
                source40: None,
                word44: 0,
                value48: 0x41200000,
                words4c_60: [0; 6],
                radius64: 0x3f800000,
                factors68_74: [0x3f800000; 4],
                words78_7c: [0; 2],
                slots80: vec![(Arc::from([]), Arc::from([]))],
                count84: 0,
                name8c: Arc::from([]),
            }),
        })
    }
}
///Native5A6FD0 after its current73A560 and40F7B0(index50) callbacks.
///859C60 strips the LAST dot
///in the entire path;85A660 gets the basename extension, even in dotted dirs.
pub(crate) fn model_variant(
    ids: &[u16],
    hardware: bool,
    suffix: Option<&[u16]>,
) -> Result<Vec<u16>> {
    if ids.contains(&0) || ids.iter().any(|&v| v > 127) {
        return Err("native Model path outside proved plain ASCII counted domain".into());
    }
    if !hardware {
        return Ok(ids.to_vec());
    }
    let prefix = ids
        .iter()
        .rposition(|&v| v == 46)
        .map_or(ids, |i| &ids[..i]);
    let base = ids
        .iter()
        .rposition(|&v| v == 47 || v == 92 || v == 58)
        .map_or(ids, |i| &ids[i + 1..]);
    let ext = base
        .iter()
        .rposition(|&v| v == 46)
        .map_or(&[][..], |i| &base[i + 1..]);
    let mut out = prefix.to_vec();
    let suffix =
        suffix.ok_or("Model hardware path actual current RendererHBPPSuffix is unbound")?;
    out.extend_from_slice(suffix);
    if !ext.is_empty() {
        out.push(46);
        out.extend_from_slice(ext);
    }
    Ok(out)
}

pub(crate) fn current_model_variant(
    ids: &[u16],
    policy: &mut impl ModelPolicy,
) -> Result<Vec<u16>> {
    if policy.use_hardware_model_73a560()? {
        let suffix = policy.hardware_model_suffix_40f7b0()?;
        model_variant(ids, true, Some(&suffix))
    } else {
        model_variant(ids, false, None)
    }
}


