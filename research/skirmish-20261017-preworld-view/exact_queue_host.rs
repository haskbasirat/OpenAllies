//!Exact-source task/Header/queue/callback field test host.
pub mod resource_objects {
//Opaque same-Application Resource identities. Cloning a Handle only retains the
//same allocation capability; native Find/reference counts remain explicit
//operations on the one ResourceManager owner. No decoder/readiness payload is
//copied or manufactured by this type.
use aa_rules::sources::{CatalogEntry, Origin};
use std::sync::{Arc, Weak};
type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Texture,
    Model,
    Sequence,
    Sound,
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
#[derive(Debug)]
struct Identity {
    kind: Kind,
    ids: Vec<u16>,
    source: Option<SourceFile>,
}
///An identity-only shared reference to ONE manager-owned allocation. A raw
///pointer, caller-supplied index or resource-loaded claim cannot create one.
#[derive(Clone, Debug)]
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
    pub fn source_file(&self) -> Option<&SourceFile> {
        self.identity.source.as_ref()
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
            identity: Arc::new(Identity {
                kind,
                ids,
                source: Some(source),
            }),
        }
    }
    fn new_default(origin: &Arc<()>, ids: Vec<u16>) -> Self {
        Self {
            application: Arc::downgrade(origin),
            identity: Arc::new(Identity {
                kind: Kind::Texture,
                ids,
                source: None,
            }),
        }
    }
}

}
pub mod world_revision {
// Process B3CC94, zero in the PE loader's uninitialized section tail.
// World constructor4F8072..4F809C increments it with native u32 wrapping.
// Kept by the actual managed application; each World retains its own stamp.
#[derive(Debug)]
pub(crate) struct Counter {
    value: u32,
}
impl Counter {
    pub(crate) fn fresh_application() -> Self {
        Self { value: 0 }
    }
    pub(crate) fn construct(&mut self) -> ConstructedWorldRevision {
        self.value = self.value.wrapping_add(1);
        ConstructedWorldRevision {
            value: self.value,
            identity: std::sync::Arc::new(()),
        }
    }
}
/// Native revision value created by the same application's World constructor.
/// No Clone/Default/public scalar constructor can manufacture this receipt.
#[derive(Debug)]
pub struct ConstructedWorldRevision {
    value: u32,
    identity: std::sync::Arc<()>,
}
/// Nonowning identity metadata for a read of the SAME retained World. It
/// cannot keep a destroyed World allocation alive or be made from a scalar.
/// The caller must derive it from the current validated load/frame owner.
#[derive(Debug, Clone)]
pub struct Witness {
    value: u32,
    identity: std::sync::Weak<()>,
}
impl Witness {
    pub(crate) fn require_live(&self)->Result<(),String>{
        self.identity.upgrade().map(|_|()).ok_or_else(||"World revision witness belongs to a destroyed allocation".into())
    }
    pub(crate) fn value(&self) -> u32 {
        self.value
    }
    pub(crate) fn owner_identity(&self) -> &std::sync::Weak<()> {
        &self.identity
    }
    ///Readonly correlation with the actual retained World allocation. This
    ///does not certify a completed frame or promote render data to a receipt.
    pub fn is_for(&self, world: &ConstructedWorldRevision) -> bool {
        self.value == world.value
            && self
                .identity
                .upgrade()
                .is_some_and(|identity| std::sync::Arc::ptr_eq(&identity, &world.identity))
    }
}
impl ConstructedWorldRevision {
    pub fn value(&self) -> u32 {
        self.value
    }
    pub fn witness(&self) -> Witness {
        Witness {
            value: self.value,
            identity: std::sync::Arc::downgrade(&self.identity),
        }
    }
    /// Stable pointer identity metadata. Equal numeric native revisions in
    /// two applications still name different actual World allocations.
    pub(crate) fn owner_identity(&self) -> &std::sync::Arc<()> {
        &self.identity
    }
}

}
pub mod resource_map_cache {type Result<T>=std::result::Result<T,String>;
pub(crate) fn normalized_key(ids: &[u16]) -> Result<String> {
    let end = ids.iter().position(|&u| u == 0).unwrap_or(ids.len());
    if ids[..end].iter().any(|&u| u > 127) {
        return Err("resource name outside proved ASCII native lookup domain".into());
    }
    let text = String::from_utf16(&ids[..end]).map_err(|e| e.to_string())?;
    Ok(text.replace('\\', "/").to_ascii_lowercase())
}}
#[path="../../crates/aa-runtime/src/execution_context.rs"] pub mod execution_context;
#[path="../../crates/aa-runtime/src/simulation_execution.rs"] pub mod simulation_execution;
#[path="../../crates/aa-runtime/src/view_init_prefix.rs"] pub mod view_init_prefix;
#[path="../../crates/aa-runtime/src/application_loading.rs"] pub mod application_loading;
#[path="../../crates/aa-runtime/src/view_resource_task.rs"] pub mod view_resource_task;
#[path="../../crates/aa-runtime/src/resource_task.rs"] pub mod resource_task;
#[path="../../crates/aa-runtime/src/resource_task_guards.rs"] pub mod resource_task_guards;
#[path="../../crates/aa-runtime/src/ui_world_init.rs"] pub mod ui_world_init;
