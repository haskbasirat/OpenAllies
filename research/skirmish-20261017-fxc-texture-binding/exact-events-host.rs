use aa_rules::resources::{Provenance,SourceIdentity};use aa_numeric::Precision;use std::collections::BTreeMap;pub type Result<T>=std::result::Result<T,String>;
pub mod resource_objects{//Opaque same-Application Resource identities. Cloning a Handle only retains the
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
mod view_fx_command_list{// 43B870's retained E0/E4/E8 intrusive list. Keys replace native pointers;
// payloads move once, and the native count retains u32 wrapping semantics.
use std::sync::{Arc, Weak};

#[derive(Clone, Debug)]
pub struct Key {
    slot: usize,
    owner: Weak<()>,
}
impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot && Weak::ptr_eq(&self.owner, &other.owner)
    }
}
impl Eq for Key {}

}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum ListKind {
    Created = 0,
    Destroyed = 1,
    Selected = 2,
    Commanded = 3,
    CommandedAggressive = 4,
    Constructed = 5,
}
impl ListKind {
    pub const ALL: [Self; 6] = [
        Self::Created,
        Self::Destroyed,
        Self::Selected,
        Self::Commanded,
        Self::CommandedAggressive,
        Self::Constructed,
    ];
    pub fn offset(self) -> u32 {
        0xbc + (self as u32) * 8
    }
    pub fn node_name(self) -> &'static str {
        match self {
            Self::Created => "WhenCreated",
            Self::Destroyed => "WhenDestroyed",
            Self::Selected => "WhenSelected",
            Self::Commanded => "WhenCommanded",
            Self::CommandedAggressive => "WhenCommandedAggressive",
            Self::Constructed => "WhenConstructed",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum FxcKind {
    CreateVfx,
    PlaySfx,
    StopVfx,
    ///42B130/42CF10 owns the actual object_animation_IDS CString at+3C.
    PlayObjectAnimation,
    Equipment,
    StopSfx,
    ChangeAspect,
    ChangeSkin,
    CreateRfx,
    PlayHitEffect,
    ///Explicit incomplete laboratory marker; the native factory never emits it.
    RequestOnly {
        native_type: u8,
    },
}
/// Exact native list scope. ActorData's state lists are not ObjectData View
/// lists and have their own local reference-map context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphKind {
    RequestOnly,
    View(ListKind),
    ActorEnter,
    ActorLeave,
    ActorAnimation,
    ///Selected ActorData component/AbilityDefinition request lists. The
    ///payload is the actual native first head-pair offset, never a View list.
    ActorRequest(u32),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct NodeId(usize);
impl NodeId {
    pub fn index(self) -> usize {
        self.0
    }
}
/// The actual source-registered identity, never a renderer object or a pointer.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Reference {
    pub id: String,
    pub source: Vec<Provenance>,
    pub ordinal: Option<u32>,
    pub cutscene: bool,
}
#[derive(Debug, serde::Serialize)]
pub struct Node {
    words: BTreeMap<u32, u32>,
    bytes: BTreeMap<u32, u8>,
    references: BTreeMap<u32, Option<Reference>>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    texts: BTreeMap<u32, String>,
    ///SAME application-owned Texture allocations; never source-ID stand-ins.
    #[serde(skip)]
    textures: BTreeMap<u32, Option<crate::resource_objects::Handle>>,
    immediate: Vec<NodeId>,
    chain: Vec<NodeId>,
    source: Vec<Provenance>,
    complete: bool,
    ///Portable intra-arena pointers, distinct from source definitions/words.
    #[serde(skip)]
    node_links: BTreeMap<u32, NodeId>,
}
impl Node {
    ///42AEF0: +3C..+65 are unwritten until their actual loader stores.
    fn common(source: Vec<Provenance>) -> Self {
        Self {
            words: BTreeMap::from([
                (0x18, 0),
                (0x1c, 0),
                (0x24, 0),
                (0x28, 0),
                (0x2c, 0),
                (0x30, u32::MAX),
                (0x34, 42),
            ]),
            bytes: BTreeMap::from([(4, 0), (0x38, 0)]),
            references: BTreeMap::from([(0x20, None)]),
            texts: BTreeMap::new(),
            textures: BTreeMap::new(),
            immediate: Vec::new(),
            chain: Vec::new(),
            source,
            complete: false,
            node_links: BTreeMap::new(),
        }
    }
    fn create_vfx(source: Vec<Provenance>) -> Self {
        let mut node = Self::common(source);
        node.words.insert(0x68, 0);
        node
    }
    ///Owned42B130 CString, never a source definition pointer.
    pub fn text(&self, offset: u32) -> Option<&str> {
        self.texts.get(&offset).map(String::as_str)
    }
    pub fn texture(&self, offset:u32)->Option<&Option<crate::resource_objects::Handle>>{
        self.textures.get(&offset)
    }
    pub fn word(&self, offset: u32) -> Option<u32> {
        self.words.get(&offset).copied()
    }
    pub fn byte(&self, offset: u32) -> Option<u8> {
        self.bytes.get(&offset).copied()
    }
    pub fn reference(&self, offset: u32) -> Option<&Option<Reference>> {
        self.references.get(&offset)
    }
    pub fn immediate(&self) -> &[NodeId] {
        &self.immediate
    }
    pub fn chain(&self) -> &[NodeId] {
        &self.chain
    }
    pub fn source(&self) -> &[Provenance] {
        &self.source
    }
    pub fn complete(&self) -> bool {
        self.complete
    }
}
/// A draft owns every allocated command, including a loader's partial prefix.
/// Its nodes cannot be submitted to FXManager as completed definitions.
#[derive(Debug, serde::Serialize)]
pub struct Draft {
    nodes: Vec<Node>,
    kinds: Vec<FxcKind>,
    roots: Vec<NodeId>,
    first_vfx: BTreeMap<String, NodeId>,
    warnings: Vec<NodeId>,
}
impl Draft {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            kinds: Vec::new(),
            roots: Vec::new(),
            first_vfx: BTreeMap::new(),
            warnings: Vec::new(),
        }
    }
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }
    pub fn warnings(&self) -> &[NodeId] {
        &self.warnings
    }
    fn store(&mut self, id: NodeId, offset: u32, value: u32) {
        self.nodes[id.0].words.insert(offset, value);
    }
    fn byte(&mut self, id: NodeId, offset: u32, value: u8) {
        self.nodes[id.0].bytes.insert(offset, value);
    }
    fn reference(&mut self, id: NodeId, offset: u32, value: Option<Reference>) {
        self.nodes[id.0].references.insert(offset, value);
    }
}
#[derive(Debug)]
pub struct Failure {
    pub error: String,
    pub partial: Draft,
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl std::error::Error for Failure {}
/// Required source services. None/empty are actual source answers, not defaults.
pub trait Inputs {
    type Source: Clone;
    fn request_only(&self) -> bool {
        false
    }
    fn provenance(&mut self, node: &Self::Source) -> Result<Vec<Provenance>>;
    fn children(&mut self, node: &Self::Source, name: &str) -> Result<Vec<Self::Source>>;
    fn text(&mut self, node: &Self::Source, name: &str) -> Result<String>;
    fn integer(&mut self, node: &Self::Source, name: &str) -> Result<i32>;
    fn scalar(&mut self, node: &Self::Source, name: &str) -> Result<u32>;
    fn flag(&mut self, node: &Self::Source, name: &str) -> Result<bool>;
    fn point(&mut self, node: &Self::Source, name: &str) -> Result<[u32; 3]>;
    fn target_index(&mut self, ids: &str) -> Result<u32>;
    fn find(&mut self, manager: &str, ids: &str) -> Result<Option<Reference>>;
    fn enum_value(&mut self, domain: &str, ids: &str) -> Result<i32>;
    ///5B5130 is GetFixed, distinct from the common loader's5B4AE0 Float.
    fn fixed(&mut self,_node:&Self::Source,_name:&str)->Result<u32>{
        Err("FX subtype requires actual5B5130 GetFixed source".into())
    }
    ///4763C0 expands against current GameData directory, then5A77E0 uses
    ///the SAME retained Resource Application. The node keeps that exact handle.
    fn texture_reference(&mut self,_node:&Self::Source,_ids:&str)->Result<Option<crate::resource_objects::Handle>>{
        Err("FX subtype requires actual4763C0/5A77E0 Texture allocation owner".into())
    }
    ///42CD9x: push actual settings5A6D10, set reading1, rendering0, in order.
    fn radar_texture_policy(&mut self,_texture:&crate::resource_objects::Handle)->Result<()>{
        Err("FX CreateRFX requires actual5A6D10/5A9330/5A6340 Texture effects".into())
    }
    fn color_490e50(&mut self,_node:&Self::Source,_name:&str,_alpha:u8)->Result<u32>{
        Err("FX CreateRFX requires actual490E50 color source".into())
    }
}
fn scalar<I: Inputs>(input: &mut I, source: &I::Source, name: &str) -> Result<u32> {
    let bits = input.scalar(source, name)?;
    if !f32::from_bits(bits).is_finite() {
        return Err(format!("FX {name} non-finite getter domain is not proved"));
    }
    Ok(bits)
}
fn load_node<I: Inputs>(input: &mut I, source: &I::Source, draft: &mut Draft) -> Result<NodeId> {
    let kind = input.text(source, "type")?;
    let kind = input.enum_value("FXC", &kind)?;
    if kind < 0 {
        return Err("Unknown FXC type (42B990 native throws)".into());
    }
    if !matches!(kind, 0..=9) {
        return Err(format!("42B990 FXC type{kind} LoadData is required"));
    }
    let provenance = input.provenance(source)?;
    draft.nodes.try_reserve(1).map_err(|e| e.to_string())?;
    draft.kinds.try_reserve(1).map_err(|e| e.to_string())?;
    let id = NodeId(draft.nodes.len());
    draft.kinds.push(match kind {
        0 => FxcKind::CreateVfx,
        1 => FxcKind::StopVfx,
        3 => FxcKind::PlaySfx,
        8 => FxcKind::PlayObjectAnimation,
        2 => FxcKind::Equipment,
        4 => FxcKind::StopSfx,
        5 => FxcKind::ChangeAspect,
        6 => FxcKind::ChangeSkin,
        7 => FxcKind::CreateRfx,
        9 => FxcKind::PlayHitEffect,
        _ => unreachable!(),
    });
    draft.nodes.push(match kind {
        0 => Node::create_vfx(provenance),
        1 => actor_state::construct_stop_vfx(provenance),
        3 => sound::construct_play_sfx(provenance),
        8 => construct_play_object_animation(provenance),
        _ => subtypes::construct(kind as u8,provenance)?,
    });
    load_base(input, source, draft, id)?;
    match kind {
        0 => load_create_vfx(input, source, draft, id),
        1 => actor_state::load_stop_vfx(input, source, draft, id),
        3 => sound::load_play_sfx(input, source, draft, id),
        8 => load_play_object_animation(input, source, draft, id),
        _ => subtypes::load(kind as u8,input,source,draft,id),
    }
}
///42B130 writes the shared constructor and allocates an empty CString+3C.
fn construct_play_object_animation(source: Vec<Provenance>) -> Node {
    let mut node = Node::common(source);
    node.texts.insert(0x3c, String::new());
    node
}
///42CF10 returns from42C1B0 before the required String getter and CString
///assignment. Native performs no animation lookup or empty-name validation.
fn load_play_object_animation<I: Inputs>(
    input: &mut I,
    source: &I::Source,
    draft: &mut Draft,
    id: NodeId,
) -> Result<NodeId> {
    let value = input.text(source, "object_animation_IDS")?;
    draft.nodes[id.0].texts.insert(0x3c, value);
    draft.nodes[id.0].complete = true;
    Ok(id)
}
fn load_base<I: Inputs>(
    input: &mut I,
    source: &I::Source,
    draft: &mut Draft,
    id: NodeId,
) -> Result<()> {
    for (name, immediate) in [("ChainImmediate", true), ("Chain", false)] {
        let children = input.children(source, name)?;
        for child in children {
            let child = load_node(input, &child, draft)?;
            let list = if immediate {
                &mut draft.nodes[id.0].immediate
            } else {
                &mut draft.nodes[id.0].chain
            };
            list.try_reserve(1).map_err(|e| e.to_string())?;
            list.insert(0, child);
        }
    }
    //4C5BF0 commits each token's bit before looking up the next token.
    let targets = input.text(source, "target_flags")?;
    for token in targets.split([',', ' ']).filter(|s| !s.is_empty()) {
        let (name, offset) = if let Some(name) = token.strip_prefix(['!', '-']) {
            (name, 0x1c)
        } else {
            (token.strip_prefix('+').unwrap_or(token), 0x18)
        };
        let index = input.target_index(name)?;
        let previous = draft.nodes[id.0]
            .word(offset)
            .expect("constructor target word");
        draft.store(id, offset, previous | 1u32.wrapping_shl(index));
    }
    let terrain = input.text(source, "terrain_group_filter")?;
    let reference = if terrain.is_empty() {
        None
    } else {
        input.find("TerrainGroup", &terrain)?
    };
    let missing = !terrain.is_empty() && reference.is_none();
    draft.reference(id, 0x20, reference);
    if missing {
        return Err(format!(
            "FXC filter specifies unknown terrain group '{terrain}'"
        ));
    }
    let value = input.integer(source, "frequency")?;
    draft.store(id, 0x24, value as u32);
    let value = scalar(input, source, "delay")?;
    draft.store(id, 0x28, value);
    let value = scalar(input, source, "health")?;
    draft.store(id, 0x2c, value);
    let value = input.text(source, "health_trigger_type")?;
    let value = input.enum_value("HealthTrigger", &value)?;
    draft.store(id, 0x30, value as u32);
    let value = input.flag(source, "stop_on_destructed")?;
    draft.byte(id, 0x38, u8::from(value));
    if f32::from_bits(draft.nodes[id.0].word(0x2c).unwrap()) < 0.0 {
        draft.store(id, 0x2c, 0xbf800000);
    }
    let state = input.text(source, "actor_state")?;
    let value = if state.is_empty() {
        42
    } else {
        input
            .find("ActorState", &state)?
            .and_then(|r| r.ordinal)
            .map(|n| n as i32)
            .unwrap_or(-1)
    };
    draft.store(id, 0x34, value as u32);
    if value < 0 {
        return Err(format!("Unknown actor state '{state}' specified FXC"));
    }
    if value == 8 {
        return Err("Use MoverComponent [WhenStartMoving] instead of actor state8".into());
    }
    Ok(())
}
fn load_create_vfx<I: Inputs>(
    input: &mut I,
    source: &I::Source,
    draft: &mut Draft,
    id: NodeId,
) -> Result<NodeId> {
    //42C5E0 starts only after the complete common loader returns.
    let vfx = input.text(source, "vfx_ids")?;
    let reference = input.find("VFX", &vfx)?;
    let cutscene = reference.as_ref().is_some_and(|r| r.cutscene);
    let missing = reference.is_none();
    draft.reference(id, 0x3c, reference);
    if missing {
        return Err(format!("Could not find vfx data: {vfx}"));
    }
    if cutscene {
        return Err(format!(
            "Cannot create cutscene VFX '{vfx}' using FX system"
        ));
    }
    let duration = scalar(input, source, "duration")?;
    draft.store(id, 0x60, duration);
    //The getter's binary32 is held unspilled in x87 for the comparison. For
    //the admitted finite binary32 getter domain this is exactly nonzero.
    draft.byte(id, 0x64, u8::from(duration & 0x7fffffff != 0));
    let at = input.text(source, "at")?;
    let at = input.enum_value("AT", &at)?;
    draft.store(id, 0x40, at as u32);
    if at < 0 {
        return Err(format!("Unknown AT value for creating '{vfx}'"));
    }
    let attached = input.flag(source, "attached")?;
    draft.byte(id, 0x65, u8::from(attached));
    let location = input.text(source, "location")?;
    let location = if location.is_empty() {
        0
    } else {
        input
            .find("Bone", &location)?
            .and_then(|r| r.ordinal)
            .map(|n| n as i32)
            .unwrap_or(-1)
    };
    draft.store(id, 0x44, location as u32);
    if location < 0 {
        return Err(format!("Unknown LOCATION for creating '{vfx}'"));
    }
    for (name, base) in [("translation", 0x48), ("orientation", 0x54)] {
        let value = input.point(source, name)?;
        if value.iter().any(|v| !f32::from_bits(*v).is_finite()) {
            return Err(format!("FX {name} non-finite point domain is not proved"));
        }
        for (index, word) in value.into_iter().enumerate() {
            draft.store(id, base + index as u32 * 4, word);
        }
    }
    if attached && matches!(at, 2..=6) {
        draft.warnings.push(id);
        draft.byte(id, 0x65, 0);
    }
    //42C853..C875 remembers the FIRST CreateVFX for an authored IDS in this
    //shared command-reference context. This is not a canonical VFX dedup.
    draft
        .first_vfx
        .entry(vfx.to_ascii_lowercase())
        .or_insert(id);
    draft.nodes[id.0].complete = true;
    Ok(id)
}
/// Laboratory/core entry with explicit input services; no Catalog completion
/// is implied. Failure retains allocated nodes and committed child prepends.
pub fn compile_inputs<I: Inputs>(
    input: &mut I,
    roots: Vec<I::Source>,
) -> std::result::Result<Draft, Failure> {
    let mut draft = Draft::new();
    if let Err(error) = append_roots(input, roots, &mut draft) {
        return Err(Failure {
            error,
            partial: draft,
        });
    }
    Ok(draft)
}
///49DC40 lends the same node/reference owner to each successive View list.
fn append_roots<I: Inputs>(input: &mut I, roots: Vec<I::Source>, draft: &mut Draft) -> Result<()> {
    for source in roots {
        let result = load_node(input, &source, draft).and_then(|id| {
            draft.roots.try_reserve(1).map_err(|e| e.to_string())?;
            draft.roots.insert(0, id);
            Ok(())
        });
        if let Err(error) = result {
            return Err(error);
        }
    }
    Ok(())
}
mod sound{use super::*;pub(super) fn construct_play_sfx(source: Vec<Provenance>) -> Node {
    Node {
        words: BTreeMap::from([
            (0x18, 0),
            (0x1c, 0),
            (0x24, 0),
            (0x28, 0),
            (0x2c, 0),
            (0x30, u32::MAX),
            (0x34, 42),
        ]),
        bytes: BTreeMap::from([(4, 0), (0x38, 0)]),
        references: BTreeMap::from([(0x20, None)]),
        texts: BTreeMap::new(),
        textures: BTreeMap::new(),
        immediate: Vec::new(),
        chain: Vec::new(),
        source,
        complete: false,
        node_links: BTreeMap::new(),
    }
}

///42C3C0's derived body, after the same actual42C1B0 base loader returns.
/// A failed required input preserves every preceding committed store.
pub(super) fn load_play_sfx<I: Inputs>(
    input: &mut I,
    source: &I::Source,
    draft: &mut Draft,
    id: NodeId,
) -> Result<NodeId> {
    let looping = input.flag(source, "looping")?;
    draft.byte(id, 0x40, u8::from(looping));
    let audio = input.text(source, "audio_ids")?;
    let reference = input.find("Audio", &audio)?;
    let missing = reference.is_none();
    draft.reference(id, 0x44, reference);
    if missing {
        return Err(format!("Could not find audio data: {audio}"));
    }
    if looping {
        //404ED0 warning is recorded before the native correction store.
        draft.warnings.push(id);
        draft.byte(id, 0x40, 0);
    }
    let at = input.text(source, "at")?;
    let at = input.enum_value("AT", &at)?;
    draft.store(id, 0x3c, at as u32);
    if at < 0 {
        return Err(format!("Unknown AT value for creating '{audio}'"));
    }
    draft.nodes[id.0].complete = true;
    Ok(id)
}

}
mod actor_state{use super::*;pub(super) fn construct_stop_vfx(source: Vec<Provenance>) -> Node {
    Node {
        words: BTreeMap::from([
            (0x18, 0),
            (0x1c, 0),
            (0x24, 0),
            (0x28, 0),
            (0x2c, 0),
            (0x30, u32::MAX),
            (0x34, 42),
        ]),
        bytes: BTreeMap::from([(4, 0), (0x38, 0)]),
        references: BTreeMap::from([(0x20, None)]),
        texts: BTreeMap::new(),
        textures: BTreeMap::new(),
        immediate: Vec::new(),
        chain: Vec::new(),
        source,
        complete: false,
        node_links: BTreeMap::new(),
    }
}
///42C900 runs the shared base loader first, then Find in the SAME local
///first-CreateVFX reference map. The pointer store precedes attached validation.
pub(super) fn load_stop_vfx<I: Inputs>(
    input: &mut I,
    source: &I::Source,
    draft: &mut Draft,
    id: NodeId,
) -> Result<NodeId> {
    let name = input.text(source, "vfx_ids")?;
    let target = draft
        .first_vfx
        .get(&name.to_ascii_lowercase())
        .copied()
        .ok_or_else(|| format!("STOP_VFX requires prior CREATE_VFX '{name}' (42C974)"))?;
    draft.nodes[id.0].node_links.insert(0x3c, target);
    if draft.nodes[target.0].byte(0x65) != Some(1) {
        return Err(format!(
            "STOP_VFX requires attached CREATE_VFX '{name}' (42C98C)"
        ));
    }
    draft.nodes[id.0].complete = true;
    Ok(id)
}
}
mod subtypes{//Exact derived42B6E0 types2/4/5/6/7/9 after common42C1B0.
//Texture fields retain actual Resource handles and required current effects.
use super::{Draft, Inputs, Node, NodeId, Result};
use aa_numeric::{Binary, Precision};
use aa_rules::resources::Provenance;

pub(super) fn construct(kind: u8, source: Vec<Provenance>) -> Result<Node> {
    if !matches!(kind, 2 | 4 | 5 | 6 | 7 | 9) {
        return Err("unexpected FX subtype constructor".into());
    }
    let mut node = Node::common(source);
    if kind == 5 {
        node.texts.insert(0x3c, String::new());
    } //42B270 actual CString
    if kind == 7 {
        //42B090 writes all8 bytes; their completed DWORDs are zero.
        node.words.insert(0x40, 0);
        node.words.insert(0x44, 0);
    }
    Ok(node)
}
pub(super) fn load<I: Inputs>(
    kind: u8,
    input: &mut I,
    source: &I::Source,
    draft: &mut Draft,
    id: NodeId,
) -> Result<NodeId> {
    match kind {
        2 => equipment(input, source, draft, id)?,
        4 => stop_sfx(input, source, draft, id)?,
        5 => change_aspect(input, source, draft, id)?,
        6 => change_skin(input, source, draft, id)?,
        7 => create_rfx(input, source, draft, id)?,
        9 => hit_effect(input, source, draft, id)?,
        _ => return Err("unexpected FX derived loader".into()),
    }
    draft.nodes[id.0].complete = true;
    Ok(id)
}
///42C9D0: every destination store precedes its dependent validation.
fn equipment<I: Inputs>(input: &mut I, s: &I::Source, d: &mut Draft, id: NodeId) -> Result<()> {
    let action = input.text(s, "action")?;
    let value = input.enum_value("EquipmentAction", &action)?;
    d.store(id, 0x3c, value as u32);
    if value < 0 {
        return Err(format!(
            "Unknown ACTION '{action}' when modifying equipment"
        ));
    }
    let location = input.text(s, "location")?;
    let value = input
        .find("Bone", &location)?
        .and_then(|r| r.ordinal)
        .map(|v| v as i32)
        .unwrap_or(-1);
    d.store(id, 0x40, value as u32);
    if value < 0 {
        return Err(format!(
            "Unknown LOCATION '{location}' when modifying equipment"
        ));
    }
    d.reference(id, 0x44, None);
    let equipment_type = input.text(s, "equipment_type_ids")?;
    if !equipment_type.is_empty() {
        let r = input.find("EquipmentType", &equipment_type)?;
        let missing = r.is_none();
        d.reference(id, 0x44, r);
        if missing {
            return Err(format!("Unknown equipment type '{equipment_type}'"));
        }
    }
    d.reference(id, 0x48, None);
    let equipment = input.text(s, "equipment_IDS")?;
    if matches!(d.nodes[id.0].word(0x3c), Some(4 | 5)) {
        if equipment.is_empty() {
            return Err(format!("Must specify equipment for action '{action}'"));
        }
        let r = input.find("Equipment", &equipment)?;
        let missing = r.is_none();
        d.reference(id, 0x48, r);
        if missing {
            return Err(format!("Could not find equipment data '{equipment}'"));
        }
    } else if !equipment.is_empty() {
        return Err(format!("Action '{action}' should not specify equipment"));
    }
    Ok(())
}
///42C530 stores even a null Audio result before native throws.
fn stop_sfx<I: Inputs>(input: &mut I, s: &I::Source, d: &mut Draft, id: NodeId) -> Result<()> {
    let text = input.text(s, "audio_ids")?;
    let r = input.find("Audio", &text)?;
    let missing = r.is_none();
    d.reference(id, 0x3c, r);
    if missing {
        return Err(format!("Could not find audio data: {text}"));
    }
    Ok(())
}
///42CC30 assigns CString before its required nonempty test.
fn change_aspect<I: Inputs>(input: &mut I, s: &I::Source, d: &mut Draft, id: NodeId) -> Result<()> {
    let text = input.text(s, "aspect_ids")?;
    let empty = text.is_empty();
    d.nodes[id.0].texts.insert(0x3c, text);
    if empty {
        return Err("CHANGE_ASPECT requires nonempty aspect_ids".into());
    }
    Ok(())
}
///42CC80: empty is an actual null store; missing positive texture warns.
fn change_skin<I: Inputs>(input: &mut I, s: &I::Source, d: &mut Draft, id: NodeId) -> Result<()> {
    let text = input.text(s, "skin")?;
    let r = if text.is_empty() {
        None
    } else {
        input.texture_reference(s, &text)?
    };
    let missing = !text.is_empty() && r.is_none();
    d.nodes[id.0].textures.insert(0x3c, r);
    if missing {
        d.warnings.push(id);
    }
    Ok(())
}
///42CD60 requires the same newly found Texture before policy/colour reads.
fn create_rfx<I: Inputs>(input: &mut I, s: &I::Source, d: &mut Draft, id: NodeId) -> Result<()> {
    let text = input.text(s, "radar_icon")?;
    if text.is_empty() {
        return Err("No radar_icon defined for CreateRFX".into());
    }
    let r = input.texture_reference(s, &text)?;
    d.nodes[id.0].textures.insert(0x3c, r);
    let texture = d.nodes[id.0]
        .textures
        .get(&0x3c)
        .and_then(Option::as_ref)
        .ok_or("CreateRFX5A6D10 requires its actual nonnull Texture allocation")?;
    input.radar_texture_policy(texture)?;
    for (name, offset) in [("src_color", 0x40), ("dst_color", 0x44)] {
        let word = input.color_490e50(s, name, 0xff)?;
        d.store(id, offset, word);
    }
    //490E90 divisor1/remainder0 is always satisfied, including signedbits.
    let count = input.integer(s, "blink_count")?;
    d.store(id, 0x48, count as u32);
    let at = input.text(s, "at")?;
    let at = input.enum_value("AT", &at)?;
    d.store(id, 0x4c, at as u32);
    if at < 0 {
        return Err("Unknown AT value for CreateRFX".into());
    }
    Ok(())
}
///42CF40 stores pivot before null validation, then rawPoint and six Fixeds.
fn hit_effect<I: Inputs>(input: &mut I, s: &I::Source, d: &mut Draft, id: NodeId) -> Result<()> {
    let pivot = input.text(s, "pivot")?;
    let r = input.find("Bone", &pivot)?;
    let missing = r.is_none();
    d.reference(id, 0x3c, r);
    if missing {
        return Err(format!("Pivot '{pivot}' unrecognized"));
    }
    let axis = input.point(s, "axis")?;
    if axis.iter().any(|v| !f32::from_bits(*v).is_finite()) {
        return Err("HitEffect finite axis domain unproved".into());
    }
    for (i, value) in axis.into_iter().enumerate() {
        d.store(id, 0x40 + 4 * i as u32, value);
    }
    for (name, offset) in [
        ("max_angle", 0x4c),
        ("max_impulse", 0x50),
        ("max_damage", 0x54),
        ("vel_decay", 0x58),
        ("vel_falloff", 0x5c),
        ("stiffness", 0x60),
    ] {
        let value = input.fixed(s, name)?;
        if !f32::from_bits(value).is_finite() {
            return Err(format!("HitEffect {name} finite fixed domain unproved"));
        }
        d.store(id, offset, value);
    }
    Ok(())
}
///490D30â†’490E50: all range checks precede three x87*255/truncate reads.
pub(super) fn color_490e50(point: [u32; 3], alpha: u8, pc: Precision) -> Result<u32> {
    for bits in point {
        let v = f32::from_bits(bits);
        if !v.is_finite() || !(0.0..=1.0).contains(&v) {
            return Err("native color channel must be finite0..1".into());
        }
    }
    let mut bytes = [0u8; 4];
    bytes[3] = alpha;
    for (i, bits) in point.into_iter().enumerate() {
        //490E50 swaps the inner RGB byte order: DWORD is nativeAARRGGBB.
        bytes[2 - i] = Binary::from_f32_bits(bits)
            .and_then(|v| v.mul(&Binary::from_f32_bits(0x437f0000)?, pc))
            .and_then(|v| v.truncate_i32())
            .map_err(|e| e.to_string())? as u8;
    }
    Ok(u32::from_le_bytes(bytes))
}
}
#[derive(Debug)]
pub struct Graph {
    draft: Draft,
    /// Completed earlier list heads share the same immutable node arena and
    /// first-VFX reference context; no command payload is duplicated.
    prior_roots: Vec<Vec<NodeId>>,
    owner: String,
    source: Provenance,
    precision: Precision,
    sources: Vec<SourceIdentity>,
    kind: GraphKind,
    request_only: bool,
}
/// Opaque owned428480 event. No caller can construct a placeholder event;
/// the actual constructor/embedded FXL producer is the sole future entry.
/// The manager can already own its true empty State<Command> constructor.
#[derive(Debug)]
pub struct Command {
    graph: std::sync::Arc<Graph>,
    source_view_list: Option<ListKind>,
    words: BTreeMap<u32, u32>,
    bytes: BTreeMap<u32, u8>,
    source: Option<aa_actors::ActorId>,
    target: Option<aa_actors::ActorId>,
    kingdom: Option<aa_actors::ActorId>,
    //45C700(this+5C,this), then624100(NULL): actual self-parented FXL,
    //with no attached definition and no instantiated effect children yet.
    fxl_definition: Option<NodeId>,
    fxl_parent_is_self: bool,
    //Actual native+68 intrusive link; no independent raw pointer mirror.
    view_next: Option<crate::view_fx_command_list::Key>,
}
impl Command {
    ///The original compiled head in the same immutable arena. The selected
    ///head is not a copied Graph or another set of Resource references.
    pub fn roots(&self)->&[NodeId]{match self.source_view_list{
        Some(kind)=>self.graph.view_list_roots(kind).expect("validated immutable View list"),
        None=>self.graph.roots(),
    }}
    pub fn graph(&self) -> &std::sync::Arc<Graph> {
        &self.graph
    }
    pub fn word(&self, offset: u32) -> Option<u32> {
        //Legacy constructor diagnostics can project null. A positive typed
        //link has no native-address representation in this 64-bit owner.
        if offset == 0x68 {
            return self.view_next.is_none().then_some(0);
        }
        self.words.get(&offset).copied()
    }
    pub fn byte(&self, offset: u32) -> Option<u8> {
        self.bytes.get(&offset).copied()
    }
    pub fn source(&self) -> Option<aa_actors::ActorId> {
        self.source
    }
    pub fn target(&self) -> Option<aa_actors::ActorId> {
        self.target
    }
    pub fn kingdom(&self) -> Option<aa_actors::ActorId> {
        self.kingdom
    }
    pub fn fxl_definition(&self) -> Option<NodeId> {
        self.fxl_definition
    }
    pub fn fxl_parent_is_self(&self) -> bool {
        self.fxl_parent_is_self
    }
    pub fn view_next(&self) -> Option<&crate::view_fx_command_list::Key> {
        self.view_next.as_ref()
    }
    pub(crate) fn set_view_next(&mut self, next: Option<crate::view_fx_command_list::Key>) {
        self.view_next = next;
    }
}
///4296B0's real current owners, read lazily in native order. The caller must
/// provide current OS thread identity and retained App+48, not a guessed role.
pub trait CreationInputs {
    fn muted(&mut self) -> Result<bool>;
    ///428F40: actual Kingdom virtualA8 or Actor virtualB4/108, else null.
    fn creator_kingdom(&mut self) -> Result<Option<aa_actors::ActorId>>;
    fn current_position(&mut self) -> Result<[u32; 3]>;
    fn current_thing(&mut self) -> Result<Option<aa_actors::ActorId>>;
    fn current_os_thread_id(&mut self) -> Result<u32>;
    fn application_os_thread_id(&mut self) -> Result<u32>;
    fn view_time(&mut self) -> Result<u32>;
    fn world_time(&mut self) -> Result<u32>;
}
/// The nonnull/successful allocation domain of4296B0 and428480. No command
/// is manufactured from authored metadata: `graph` has already completed its
/// native compilation. Queue submission50C800 is the caller's next operation.
pub fn construct_created(
    graph: std::sync::Arc<Graph>,
    input: &mut impl CreationInputs,
) -> Result<Option<Command>> {
    construct_created_inner(graph,None,input)
}
pub(crate) fn construct_created_list(graph:std::sync::Arc<Graph>,kind:ListKind,input:&mut impl CreationInputs)->Result<Option<Command>>{
    construct_created_inner(graph,Some(kind),input)
}
fn construct_created_inner(graph:std::sync::Arc<Graph>,list:Option<ListKind>,input:&mut impl CreationInputs)->Result<Option<Command>>{
    graph.require_runtime_fields()?;
    if match list{Some(kind)=>graph.view_list_roots(kind)?,None=>graph.roots()}.is_empty() {
        return Ok(None);
    }
    if input.muted()? {
        return Ok(None);
    }
    let kingdom = input.creator_kingdom()?;
    //Native copies target coordinates before source coordinates, then reads
    //the current Thing GTID for both arguments; callbacks may observe order.
    let target_position = input.current_position()?;
    let source_position = input.current_position()?;
    let target = input.current_thing()?;
    let source = input.current_thing()?;
    let current = input.current_os_thread_id()?;
    let app = input.application_os_thread_id()?;
    let timestamp = if current == app {
        input.view_time()?
    } else {
        input.world_time()?
    };
    let mut command=finish_command(
        graph,
        kingdom,
        source,
        target,
        source_position,
        target_position,
        timestamp,
        1,
    )?;
    if let Some(command)=&mut command{command.source_view_list=list;}
    Ok(command)
}
///429080 reads the source Thing+0C View, then View+54 and its virtual+BC
///before allocation. A positive attachment needs the actual45C740 owner.
///The method name is historical; it never denotes ObjectData flags.
pub trait ExistingInputs: CreationInputs {
    fn source_definition_attachment(&mut self) -> Result<()>;
}
///The actual nonnull42A590 form. Unlike Created, positions precede428F40,
///the caller supplies byte58, and queue50C800 receives flag0 separately.
pub fn construct_existing(
    graph: std::sync::Arc<Graph>,
    input: &mut impl ExistingInputs,
    byte58: u8,
) -> Result<Option<Command>> {
    construct_existing_inner(graph,None,input,byte58)
}
pub(crate) fn construct_existing_list(graph:std::sync::Arc<Graph>,kind:ListKind,input:&mut impl ExistingInputs,byte58:u8)->Result<Option<Command>>{
    construct_existing_inner(graph,Some(kind),input,byte58)
}
fn construct_existing_inner(graph:std::sync::Arc<Graph>,list:Option<ListKind>,input:&mut impl ExistingInputs,byte58:u8)->Result<Option<Command>>{
    graph.require_runtime_fields()?;
    //42A590 itself copies both positions even if429080 subsequently returns.
    let _source_copy = input.current_position()?;
    let _target_copy = input.current_position()?;
    if match list{Some(kind)=>graph.view_list_roots(kind)?,None=>graph.roots()}.is_empty() {
        return Ok(None);
    }
    if input.muted()? {
        return Ok(None);
    }
    let source_position = input.current_position()?;
    let target_position = input.current_position()?;
    let kingdom = input.creator_kingdom()?;
    input.source_definition_attachment()?;
    let target = input.current_thing()?;
    let source = input.current_thing()?;
    let current = input.current_os_thread_id()?;
    let app = input.application_os_thread_id()?;
    let timestamp = if current == app {
        input.view_time()?
    } else {
        input.world_time()?
    };
    let mut command=finish_command(
        graph,
        kingdom,
        source,
        target,
        source_position,
        target_position,
        timestamp,
        byte58,
    )?;
    if let Some(command)=&mut command{command.source_view_list=list;}
    Ok(command)
}
fn finish_command(
    graph: std::sync::Arc<Graph>,
    kingdom: Option<aa_actors::ActorId>,
    source: Option<aa_actors::ActorId>,
    target: Option<aa_actors::ActorId>,
    source_position: [u32; 3],
    target_position: [u32; 3],
    timestamp: u32,
    byte58: u8,
) -> Result<Option<Command>> {
    let mut words = BTreeMap::from([
        (8, 0),
        (0x30, 0),
        (0x34, u32::MAX),
        (0x38, timestamp),
        (0x3c, 0xbf800000),
        (0x40, 0xbf800000),
        (0x44, 0xbf800000),
        (0x48, 0),
        (0x4c, 0),
        (0x50, 0),
        (0x54, 42),
    ]);
    for (base, position) in [(0x14, source_position), (0x24, target_position)] {
        for (i, value) in position.into_iter().enumerate() {
            words.insert(base + 4 * i as u32, value);
        }
    }
    Ok(Some(Command {
        graph,
        source_view_list: None,
        words,
        bytes: BTreeMap::from([(0x58, byte58)]),
        source,
        target,
        kingdom,
        fxl_definition: None,
        fxl_parent_is_self: true,
        view_next: None,
    }))
}impl Graph{
fn nodes(&self) -> &[Node] {
        self.draft.nodes()
    }
fn roots(&self) -> &[NodeId] {
        self.draft.roots()
    }
fn kind(&self) -> GraphKind {
        if self.request_only {
            GraphKind::RequestOnly
        } else {
            self.kind
        }
    }
fn require_runtime_fields(&self) -> Result<()> {
        if self.request_only || self.draft.nodes.iter().any(|n| !n.complete) {
            return Err("resource-request-only or incomplete FX graph cannot execute or submit commands".into());
        }
        Ok(())
    }
fn list_kind(&self) -> Option<ListKind> {
        match self.kind {
            GraphKind::View(kind) => Some(kind),
            _ => None,
        }
    }
fn view_list_roots(&self, kind: ListKind) -> Result<&[NodeId]> {
        match self.kind {
            GraphKind::View(last) if kind == last => Ok(&self.draft.roots),
            GraphKind::View(last) if (kind as usize) < (last as usize) => self
                .prior_roots
                .get(kind as usize)
                .map(Vec::as_slice)
                .ok_or_else(|| "compiled prior View FX list absent".into()),
            _ => Err("compiled graph does not own this actual View FX list".into()),
        }
    }
}
mod regression{use super::*;use serde_json::{Value,json};


struct Fixture {
    case: Value,
    events: Vec<Value>,
}
impl Fixture {
    fn get(&mut self, node: usize, name: &str) -> Result<Value> {
        self.events.push(json!(["get", node, name]));
        if self.case["fail"] == name {
            return Err(format!("required {name}"));
        }
        Ok(self.case["nodes"][node][name].clone())
    }
}
impl Inputs for Fixture {
    type Source = usize;
    fn request_only(&self) -> bool {
        self.case["request_only"].as_bool().unwrap_or(false)
    }
    fn provenance(&mut self, _: &usize) -> Result<Vec<Provenance>> {
        Ok(Vec::new())
    }
    fn children(&mut self, node: &usize, name: &str) -> Result<Vec<usize>> {
        self.events.push(json!(["children", node, name]));
        Ok(self.case["nodes"][*node][name]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect())
    }
    fn text(&mut self, node: &usize, name: &str) -> Result<String> {
        Ok(self.get(*node, name)?.as_str().unwrap().into())
    }
    fn integer(&mut self, node: &usize, name: &str) -> Result<i32> {
        Ok(self.get(*node, name)?.as_i64().unwrap() as i32)
    }
    fn scalar(&mut self, node: &usize, name: &str) -> Result<u32> {
        Ok(self.get(*node, name)?.as_u64().unwrap() as u32)
    }
    fn flag(&mut self, node: &usize, name: &str) -> Result<bool> {
        Ok(self.get(*node, name)?.as_bool().unwrap())
    }
    fn point(&mut self, node: &usize, name: &str) -> Result<[u32; 3]> {
        Ok(self
            .get(*node, name)?
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap())
    }
    fn target_index(&mut self, ids: &str) -> Result<u32> {
        self.events.push(json!(["target", ids]));
        match ids {
            "friend" => Ok(0),
            "enemy" => Ok(1),
            "body" => Ok(12),
            _ => Err(format!("target {ids}")),
        }
    }
    fn find(&mut self, manager: &str, ids: &str) -> Result<Option<Reference>> {
        self.events.push(json!(["find", manager, ids]));
        if let Some(lookup) = self.case.get("lookups") {
            let v = &lookup[manager][ids];
            if v.is_null() {
                return Ok(None);
            }
            return Ok(Some(Reference {
                id: v["id"].as_str().unwrap().into(),
                source: Vec::new(),
                ordinal: v["ordinal"].as_u64().map(|n| n as u32),
                cutscene: v["cutscene"].as_bool().unwrap(),
            }));
        }
        Ok((ids != "missing").then(|| Reference {
            id: ids.into(),
            source: Vec::new(),
            ordinal: Some(match ids {
                "head" => 3,
                "moving" => 8,
                _ => 0,
            }),
            cutscene: ids == "cutscene",
        }))
    }
    fn enum_value(&mut self, domain: &str, ids: &str) -> Result<i32> {
        self.events.push(json!(["enum", domain, ids]));
        if self.case["fail"] == format!("enum:{domain}") {
            return Err(format!("required enum:{domain}"));
        }
        let names: &[&str] = match domain {
            "FXC" => &aa_rules::abilities::FXC_TYPES,
            "AT" => &[
                "SELF",
                "TARGET_THING",
                "SOURCE_GROUND",
                "TARGET_GROUND",
                "SOURCE_ABILITY_POSITION",
                "TARGET_HIT_POSITION",
                "MAP_CENTER",
            ],
            "HealthTrigger" => &[
                "ABSOLUTE_BELOW",
                "ABSOLUTE_ABOVE",
                "RELATIVE_DECREASE",
                "RELATIVE_INCREASE",
            ],
            _ => panic!(),
        };
        Ok(names
            .iter()
            .position(|n| n.eq_ignore_ascii_case(ids))
            .map(|n| n as i32)
            .unwrap_or(-1))
    }
}
fn snapshot(draft: &Draft) -> Value {
    json!({"nodes":draft.nodes.iter().map(|n|json!({"words":n.words,"bytes":n.bytes,
        "references":n.references.iter().map(|(k,v)|(k.to_string(),v.as_ref().map(|r|r.id.clone()))).collect::<BTreeMap<_,_>>(),
        "immediate":n.immediate,"chain":n.chain,"complete":n.complete})).collect::<Vec<_>>(),"roots":draft.roots,"warnings":draft.warnings})
}
struct CreatedInput {
    case: Value,
    events: Vec<Value>,
    positions: usize,
    things: usize,
    clock_reads: Vec<&'static str>,
}
impl CreationInputs for CreatedInput {
    fn muted(&mut self) -> Result<bool> {
        Ok(self.case["muted"].as_bool().unwrap())
    }
    fn creator_kingdom(&mut self) -> Result<Option<aa_actors::ActorId>> {
        let kind = self.case["kind"].as_str().unwrap();
        if kind == "Null" {
            return Ok(None);
        }
        self.events.push(json!("A8"));
        if kind != "Kingdom" {
            self.events.push(json!("B4"));
            if kind != "Actor" {
                return Ok(None);
            }
            self.events.push(json!("108"));
        }
        let value = self.case[if kind == "Kingdom" {
            "thing_id"
        } else {
            "kingdom_id"
        }]
        .as_u64()
        .unwrap() as u32;
        Ok((value != 0).then_some(aa_actors::ActorId::from_raw(value)))
    }
    fn current_position(&mut self) -> Result<[u32; 3]> {
        self.positions += 1;
        let key = if self.case["kind"] == "Null" {
            "global_position"
        } else {
            "position"
        };
        Ok(self.case[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap())
    }
    fn current_thing(&mut self) -> Result<Option<aa_actors::ActorId>> {
        self.things += 1;
        Ok((self.case["kind"] != "Null")
            .then(|| aa_actors::ActorId::from_raw(self.case["thing_id"].as_u64().unwrap() as u32)))
    }
    fn current_os_thread_id(&mut self) -> Result<u32> {
        self.events.push(json!("current_os_thread_id"));
        Ok(self.case["thread_id"].as_u64().unwrap() as u32)
    }
    fn application_os_thread_id(&mut self) -> Result<u32> {
        Ok(self.case["app_thread"].as_u64().unwrap() as u32)
    }
    fn view_time(&mut self) -> Result<u32> {
        self.clock_reads.push("view");
        Ok(self.case["view_time"].as_u64().unwrap() as u32)
    }
    fn world_time(&mut self) -> Result<u32> {
        self.clock_reads.push("world");
        Ok(self.case["world_time"].as_u64().unwrap() as u32)
    }
}
impl ExistingInputs for CreatedInput {
    fn source_definition_attachment(&mut self) -> Result<()> {
        if self.case["source_flags"].as_u64().unwrap() & 0xC0000 != 0 {
            self.events.push(json!("source_BC"));
        }
        //The paired native fixture's explicit virtualBC service returns false.
        Ok(())
    }
}
#[test]
fn original_v14_created_event_constructor_embedded_null_and_thread_clock_selection_match() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-fx-graph");
    let fixture: Value =
        serde_json::from_slice(&std::fs::read(root.join("native-fx-command.json")).unwrap())
            .unwrap();
    let graph_fixture: Value =
        serde_json::from_slice(&std::fs::read(root.join("native-fx-graph.json")).unwrap()).unwrap();
    let ordinary = &graph_fixture["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["input"]["name"] == "ordinary")
        .unwrap()["input"];
    for row in fixture["rows"].as_array().unwrap() {
        let case = &row["input"];
        let mut input = Fixture {
            case: ordinary.clone(),
            events: Vec::new(),
        };
        let mut draft = compile_inputs(&mut input, vec![0]).unwrap();
        if !case["nonempty"].as_bool().unwrap() {
            draft.roots.clear();
        }
        //Explicit laboratory source receipt; only the native constructor's
        //transport of an already compiled graph is under test in these rows.
        let source = Provenance {
            path: "fixture".into(),
            sha256: "fixture".into(),
            span: aa_rules::tgi::Span { start: 0, end: 0 },
        };
        let graph = std::sync::Arc::new(Graph {
            draft,
            prior_roots: Vec::new(),
            owner: "fixture".into(),
            source,
            precision: Precision::Double53,
            sources: Vec::new(),
            kind: GraphKind::View(ListKind::Created),
            request_only: false,
        });
        let mut input = CreatedInput {
            case: case.clone(),
            events: Vec::new(),
            positions: 0,
            things: 0,
            clock_reads: Vec::new(),
        };
        let command = construct_created(graph, &mut input).unwrap();
        let command=command.map(|c| {
            let mut words=c.words.clone();words.insert(0xc,c.kingdom().map_or(0,aa_actors::ActorId::raw));
            words.insert(0x10,c.source().map_or(0,aa_actors::ActorId::raw));words.insert(0x20,c.target().map_or(0,aa_actors::ActorId::raw));
            assert_eq!(input.positions,2);assert_eq!(input.things,2);
            assert_eq!(input.clock_reads,if case["thread_id"]==case["app_thread"] {vec!["view"]} else {vec!["world"]});
            words.remove(&0x68);
            json!({"words":words,"byte58":c.byte(0x58),"flag":case["flag"],"fxl_definition":c.fxl_definition(),"fxl_parent_is_self":c.fxl_parent_is_self(),"word68":c.word(0x68)})
        });
        assert_eq!(
            json!({"command":command,"events":input.events}),
            row["output"],
            "{case}"
        );
    }
}
#[test]
fn original_v14_existing_event_and_source_predicate_order_match() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research");
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(root.join("development10/native-existing-fx-command.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 864);
    let graph_fixture: Value = serde_json::from_slice(
        &std::fs::read(root.join("skirmish-20261016-fx-graph/native-fx-graph.json")).unwrap(),
    )
    .unwrap();
    let ordinary = &graph_fixture["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["input"]["name"] == "ordinary")
        .unwrap()["input"];
    for row in fixture["rows"].as_array().unwrap() {
        let case = &row["input"];
        let mut input = Fixture {
            case: ordinary.clone(),
            events: Vec::new(),
        };
        let mut draft = compile_inputs(&mut input, vec![0]).unwrap();
        if !case["nonempty"].as_bool().unwrap() {
            draft.roots.clear();
        }
        let graph = std::sync::Arc::new(Graph {
            draft,
            prior_roots: Vec::new(),
            owner: "fixture".into(),
            source: Provenance {
                path: "fixture".into(),
                sha256: "fixture".into(),
                span: aa_rules::tgi::Span { start: 0, end: 0 },
            },
            precision: Precision::Double53,
            sources: Vec::new(),
            kind: GraphKind::View(ListKind::Constructed),
            request_only: false,
        });
        let mut input = CreatedInput {
            case: case.clone(),
            events: Vec::new(),
            positions: 0,
            things: 0,
            clock_reads: Vec::new(),
        };
        let command =
            construct_existing(graph, &mut input, case["flag"].as_u64().unwrap() as u8).unwrap();
        let command=command.map(|c| {
            let mut words=c.words.clone();
            words.insert(0xc,c.kingdom().map_or(0,aa_actors::ActorId::raw));
            words.insert(0x10,c.source().map_or(0,aa_actors::ActorId::raw));
            words.insert(0x20,c.target().map_or(0,aa_actors::ActorId::raw));
            assert_eq!(input.positions,4);assert_eq!(input.things,2);
            assert_eq!(input.clock_reads,if case["thread_id"]==case["app_thread"] {vec!["view"]}else{vec!["world"]});
            words.remove(&0x68);
            json!({"words":words,"byte58":c.byte(0x58),"flag":0,"fxl_definition":c.fxl_definition(),"fxl_parent_is_self":c.fxl_parent_is_self(),"word68":c.word(0x68)})
        });
        if command.is_none() {
            assert_eq!(input.positions, 2);
        }
        assert_eq!(
            json!({"command":command,"events":input.events}),
            row["output"],
            "{case}"
        );
    }
}
#[test]
fn completed_view_arena_selects_prior_created_head_without_cloning_or_constructed_fallback(){
    //The same native constructor-input transport as the existing event replay.
    //No World/resource/readiness is synthesized by this controlled fixture.
    let graph_fixture:Value=serde_json::from_str(include_str!("E:/Games/Axis and Alies/rust-rebuild/research/skirmish-20261016-fx-graph/native-fx-graph.json")).unwrap();
    let ordinary=graph_fixture["rows"].as_array().unwrap().iter().find(|r|r["input"]["name"]=="ordinary").unwrap()["input"].clone();
    let mut source=Fixture{case:ordinary,events:vec![]};
    let mut draft=compile_inputs(&mut source,vec![0]).unwrap();
    let created=std::mem::take(&mut draft.roots);
    let mut prior=vec![vec![];5];prior[0]=created.clone();
    let graph=std::sync::Arc::new(Graph{draft,prior_roots:prior,owner:"native event transport".into(),source:Provenance{path:"native event transport".into(),sha256:String::new(),span:aa_rules::tgi::Span{start:0,end:0}},precision:Precision::Double53,sources:vec![],kind:GraphKind::View(ListKind::Constructed),request_only:false});
    let events:Value=serde_json::from_str(include_str!("E:/Games/Axis and Alies/rust-rebuild/research/skirmish-20261016-fx-graph/native-fx-command.json")).unwrap();
    let case=events["rows"].as_array().unwrap().iter().find(|r|r["input"]["kind"]=="Actor"&&r["input"]["nonempty"]==true&&r["input"]["muted"]==false).unwrap()["input"].clone();
    let mut input=CreatedInput{case:case.clone(),events:vec![],positions:0,things:0,clock_reads:vec![]};
    let command=construct_created_list(graph.clone(),ListKind::Created,&mut input).unwrap().unwrap();
    assert!(std::sync::Arc::ptr_eq(command.graph(),&graph));assert_eq!(command.roots(),created);assert_eq!(input.positions,2);assert_eq!(input.things,2);
    assert!(graph.roots().is_empty(),"selected list must not mutate the retained arena");
    let mut empty=CreatedInput{case,events:vec![],positions:0,things:0,clock_reads:vec![]};
    assert!(construct_existing_list(graph,ListKind::Constructed,&mut empty,0).unwrap().is_none());
    assert_eq!(empty.positions,2);assert_eq!(empty.things,0);assert!(empty.clock_reads.is_empty());
}
}
