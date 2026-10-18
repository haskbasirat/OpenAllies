use aa_rules::resources::Provenance;use aa_numeric::Precision;use std::collections::BTreeMap;
pub type Result<T>=std::result::Result<T,String>;
pub mod resource_objects{
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
mod sound{use super::*;
pub(super) fn construct_play_sfx(source: Vec<Provenance>) -> Node {
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

pub(super) fn find_audio(bound: &Bound<'_>, ids: &str) -> Result<Option<Reference>> {
    if ids.is_empty() {
        return Ok(None);
    }
    if !ids.is_ascii() {
        return Err("Audio lookup non-ASCII case-folding domain is not proved".into());
    }
    let ids = bound.translated("Audio", ids)?;
    if !ids.is_ascii() {
        return Err("translated Audio IDS non-ASCII case-folding domain is not proved".into());
    }
    let registry = bound
        .actors
        .native_data_registry("Audio")
        .ok_or("actual FX Audio registry missing")?;
    if registry.mode != Some(0) {
        return Err("actual FX Audio manager mode differs from482090".into());
    }
    let Some(entry) = registry
        .entries
        .iter()
        .find(|e| e.ids.eq_ignore_ascii_case(&ids))
    else {
        return Ok(None);
    };
    if !bound.before_owner(&entry.source)? {
        return Ok(None);
    }
    let node = bound
        .actors
        .native_data_node(entry)
        .map_err(|e| e.to_string())?;
    let actual_ids = match bound.value(&node, "IDS")? {
        aa_rules::native::TypedValue::String { value, .. } => value,
        _ => return Err("Audio IDS is not an actual string".into()),
    };
    if !actual_ids.eq_ignore_ascii_case(&entry.ids) {
        return Err("actual FX Audio registered/source IDS differs".into());
    }
    Ok(Some(Reference {
        id: entry.ids.clone(),
        source: node.sources(),
        ordinal: None,
        cutscene: false,
    }))
}

mod regression{
use super::super::*;
use serde_json::{Value, json};
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
        if self.case["fail"] == format!("find:{manager}") {
            return Err(format!("required find:{manager}"));
        }
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
        Ok((!matches!(ids, "missing" | "")).then(|| Reference {
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
#[test]
fn original_v14_play_sfx_full_state_order_and_partial_stores_match() {
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/skirmish-20261017-fx-sound/native-fx-sound.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let case = &row["input"];
        let roots = case["roots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        let mut input = Fixture {
            case: case.clone(),
            events: Vec::new(),
        };
        let result = compile_inputs(&mut input, roots);
        let failed = result.is_err();
        let draft = match result {
            Ok(d) => d,
            Err(f) => f.partial,
        };
        let mut actual = snapshot(&draft);
        actual["failed"] = json!(failed);
        actual["events"] = json!(input.events);
        let mut expected = row["output"].clone();
        expected["failed"] = json!(!expected["error"].is_null());
        expected.as_object_mut().unwrap().remove("error");
        assert_eq!(actual, expected, "{} / {}", case["name"], row["precision"]);
    }
}
}
}
mod actor_state{use super::*;
pub(super) fn construct_stop_vfx(source: Vec<Provenance>) -> Node {
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
pub(super) fn active_fields(
    kind: FxcKind,
    permanent: impl FnOnce() -> Result<bool>,
    duration: impl FnOnce() -> Result<u8>,
) -> Result<bool> {
    match kind {
        FxcKind::PlaySfx | FxcKind::StopVfx | FxcKind::PlayObjectAnimation
        | FxcKind::StopSfx | FxcKind::ChangeAspect | FxcKind::ChangeSkin
        | FxcKind::CreateRfx | FxcKind::PlayHitEffect => Ok(false),
        FxcKind::Equipment => Ok(true), //native9FC2B4+20=6A71D0
        FxcKind::RequestOnly { .. } => Err("request-only FXC virtual20 fields are unbound".into()),
        FxcKind::CreateVfx => {
            if !permanent()? {
                return Ok(false);
            }
            Ok(duration()? == 0)
        }
    }
}

}
use aa_rules::{actors::{ActorCatalog,ResolvedNode},native::{ScalarType,TypedValue},targets::TargetDomain};
struct Bound<'a> {
    request_only: bool,
    actors: &'a ActorCatalog,
    pc: Precision,
    owner: Provenance,
    targets: Option<TargetDomain>,
}
impl<'a> Bound<'a> {
    fn new(actors: &'a ActorCatalog, pc: Precision, owner: Provenance) -> Result<Self> {
        Ok(Self {
            request_only: false,
            actors,
            pc,
            owner,
            targets: None,
        })
    }
    fn value(&self, node: &ResolvedNode<'_>, name: &str) -> Result<TypedValue> {
        node.field(name, self.pc)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Node doesn't contain field. ({name})"))
    }
    fn before_owner(&self, source: &Provenance) -> Result<bool> {
        let position = |p: &Provenance| -> Result<(usize, usize)> {
            Ok((
                self.actors
                    .sources
                    .iter()
                    .position(|s| s.path.eq_ignore_ascii_case(&p.path))
                    .ok_or("FX source position missing")?,
                p.span.start,
            ))
        };
        Ok(position(source)? <= position(&self.owner)?)
    }
    ///48D2D0 inserts each one-hop translation when its root is processed.
    ///42C070/4BDB70 see only roots already reached at this owner's LoadData.
    fn translated(&self, manager: &str, ids: &str) -> Result<String> {
        let mut found = None;
        for node in self
            .actors
            .resolved_definitions("Translate")
            .map_err(|e| e.to_string())?
        {
            let source = node
                .sources()
                .first()
                .cloned()
                .ok_or("FX Translate source missing")?;
            if !self.before_owner(&source)? {
                continue;
            }
            let text = |name: &str| -> Result<String> {
                match self.value(&node, name)? {
                    TypedValue::String { value, .. } => Ok(value),
                    _ => Err(format!("Translate {name} is not string")),
                }
            };
            let kind = text("node")?;
            let same = if manager == "VFX" {
                aa_rules::actors::native_load::THING_TAGS
                    .iter()
                    .any(|k| k.eq_ignore_ascii_case(&kind))
            } else {
                kind.eq_ignore_ascii_case(manager)
            };
            if same && text("ids_old")?.eq_ignore_ascii_case(ids) {
                if found.is_some() {
                    return Err("FX duplicate historical translation root".into());
                }
                found = Some(text("ids_new")?);
            }
        }
        Ok(found.unwrap_or_else(|| ids.into()))
    }
    fn canonical_node(&self, source: &Provenance) -> Result<ResolvedNode<'a>> {
        for tag in aa_rules::actors::native_load::THING_TAGS {
            for node in self
                .actors
                .resolved_definitions(tag)
                .map_err(|e| e.to_string())?
            {
                if node.sources().first() == Some(source) {
                    return Ok(node);
                }
            }
        }
        Err("FX canonical source root is absent".into())
    }
}
impl<'a> Inputs for Bound<'a> {
    type Source = ResolvedNode<'a>;
    fn request_only(&self) -> bool {
        self.request_only
    }
    fn provenance(&mut self, node: &Self::Source) -> Result<Vec<Provenance>> {
        Ok(node.sources())
    }
    fn children(&mut self, node: &Self::Source, name: &str) -> Result<Vec<Self::Source>> {
        node.children(name).map_err(|e| e.to_string())
    }
    fn text(&mut self, node: &Self::Source, name: &str) -> Result<String> {
        match self.value(node, name)? {
            TypedValue::String { value, .. } => Ok(value),
            _ => Err(format!("GetString rejects FX {name}")),
        }
    }
    fn integer(&mut self, node: &Self::Source, name: &str) -> Result<i32> {
        self.value(node, name)?.integer().map_err(|e| e.to_string())
    }
    fn flag(&mut self, node: &Self::Source, name: &str) -> Result<bool> {
        self.value(node, name)?.boolean().map_err(|e| e.to_string())
    }
    fn scalar(&mut self, node: &Self::Source, name: &str) -> Result<u32> {
        match self.value(node, name)? {
            TypedValue::Scalar(s)
                if s.field_type == ScalarType::Float && f32::from_bits(s.bits).is_finite() =>
            {
                Ok(s.bits)
            }
            _ => Err(format!(
                "FX GetFloat {name} requires proved finite binary32 input"
            )),
        }
    }
    fn fixed(&mut self,node:&Self::Source,name:&str)->Result<u32>{
        let value=self.value(node,name)?.fixed_bits().map_err(|e|e.to_string())?;
        if !f32::from_bits(value).is_finite(){return Err(format!("FX Fixed {name} non-finite domain unproved"))}
        Ok(value)
    }
    fn color_490e50(&mut self,node:&Self::Source,name:&str,alpha:u8)->Result<u32>{
        subtypes::color_490e50(self.point(node,name)?,alpha,self.pc)
    }
    fn point(&mut self, node: &Self::Source, name: &str) -> Result<[u32; 3]> {
        match self.value(node, name)? {
            TypedValue::Point { bits, .. }
                if bits.len() == 3 && bits.iter().all(|b| f32::from_bits(*b).is_finite()) =>
            {
                Ok(bits.try_into().unwrap())
            }
            _ => Err(format!(
                "FX GetPoint3 {name} requires three finite binary32 inputs"
            )),
        }
    }
    fn target_index(&mut self, ids: &str) -> Result<u32> {
        if self.targets.is_none() {
            self.targets = Some(
                self.actors
                    .native_target_domain(self.pc)
                    .map_err(|e| e.to_string())?,
            );
        }
        self.targets
            .as_ref()
            .unwrap()
            .index(ids)
            .map_err(|e| e.to_string())
    }
    fn find(&mut self, manager: &str, ids: &str) -> Result<Option<Reference>> {
        if manager == "Audio" {
            return sound::find_audio(self, ids);
        }
        if manager == "VFX" {
            let ids = self.translated(manager, ids)?;
            let Some(found) = self
                .actors
                .actors
                .iter()
                .find(|a| a.id.eq_ignore_ascii_case(&ids))
            else {
                return Ok(None);
            };
            if !self.before_owner(&found.source)? {
                return Ok(None);
            }
            let node = self.canonical_node(&found.source)?;
            let category = self.text(&node, "category")?;
            if !category.to_ascii_uppercase().starts_with("VFX") {
                return Ok(None);
            }
            let cutscene = self.flag(&node, "cutscene")?;
            return Ok(Some(Reference {
                id: found.id.clone(),
                source: node.sources(),
                ordinal: None,
                cutscene,
            }));
        }
        if manager == "Bone" {
            //486650 is a distinct mode0 manager, not the generic catalog's
            //registry.486870 stores current+28 at Data+C before93D4E0.
            let ids = self.translated(manager, ids)?;
            let mut roots = self
                .actors
                .resolved_definitions("Bone")
                .map_err(|e| e.to_string())?;
            let position = |n: &ResolvedNode<'_>| -> Result<(usize, usize)> {
                let source = n
                    .sources()
                    .first()
                    .cloned()
                    .ok_or("Bone root source missing")?;
                Ok((
                    self.actors
                        .sources
                        .iter()
                        .position(|s| s.path.eq_ignore_ascii_case(&source.path))
                        .ok_or("Bone source index missing")?,
                    source.span.start,
                ))
            };
            let mut ordered = roots
                .drain(..)
                .map(|n| Ok((position(&n)?, n)))
                .collect::<Result<Vec<_>>>()?;
            ordered.sort_by_key(|(position, _)| *position);
            let mut seen = std::collections::BTreeSet::new();
            for (ordinal, (_, node)) in ordered.into_iter().enumerate() {
                let value = self.text(&node, "IDS")?;
                if !seen.insert(value.to_ascii_lowercase()) {
                    return Err(
                        "native Bone mode0 duplicate IDS requires source load failure".into(),
                    );
                }
                if value.eq_ignore_ascii_case(&ids) {
                    let source = node.sources();
                    if !self.before_owner(source.first().ok_or("Bone root source missing")?)? {
                        return Ok(None);
                    }
                    return Ok(Some(Reference {
                        id: value,
                        source,
                        ordinal: Some(u32::try_from(ordinal).map_err(|e| e.to_string())?),
                        cutscene: false,
                    }));
                }
            }
            return Ok(None);
        }
        let registry = self
            .actors
            .native_data_registry(manager)
            .ok_or_else(|| format!("actual FX {manager} registry missing"))?;
        let ids = self.translated(manager, ids)?;
        let Some(entry) = registry
            .entries
            .iter()
            .find(|entry| entry.ids.eq_ignore_ascii_case(&ids))
        else {
            return Ok(None);
        };
        if !self.before_owner(&entry.source)? {
            return Ok(None);
        }
        let node = self
            .actors
            .native_data_node(entry)
            .map_err(|e| e.to_string())?;
        let index = registry
            .entries
            .iter()
            .position(|e| e.path == entry.path)
            .ok_or("FX generic source ordinal missing")?;
        if manager == "ActorState" {
            if registry.mode != Some(2) {
                return Err("FX ActorState native mode2 manager differs".into());
            }
            //477FC0/474270/473490 pre-register this fixed Data+C index;
            //source load order does not select it (see load_actor_registration).
            let descriptor = aa_actors::state::DESCRIPTORS
                .iter()
                .find(|d| d.name.eq_ignore_ascii_case(&entry.ids))
                .ok_or("FX ActorState is absent from native fixed descriptors")?;
            return Ok(Some(Reference {
                id: entry.ids.clone(),
                source: node.sources(),
                ordinal: Some(u32::from(descriptor.index)),
                cutscene: false,
            }));
        }
        Ok(Some(Reference {
            id: entry.ids.clone(),
            source: node.sources(),
            ordinal: Some(u32::try_from(index).map_err(|e| e.to_string())?),
            cutscene: false,
        }))
    }
    fn enum_value(&mut self, domain: &str, ids: &str) -> Result<i32> {
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
            "EquipmentAction" => &["HIDE","UNHIDE","FLIP","REMOVE","ADD","REPLACE"],
            _ => return Err(format!("FX unknown enum domain {domain}")),
        };
        Ok(names
            .iter()
            .position(|v| v.eq_ignore_ascii_case(ids))
            .map(|v| v as i32)
            .unwrap_or(-1))
    }
}


#[path="E:/Games/Axis and Alies/rust-rebuild/crates/aa-runtime/src/fx_graph_subtypes.rs"] mod subtypes;
mod regression{
use super::*;
use serde_json::{Value, json};
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
#[test]
fn original_v14_create_vfx_graph_full_state_order_and_partial_stores_match() {
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/skirmish-20261016-fx-graph/native-fx-graph.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let case = &row["input"];
        let roots = case["roots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        let mut input = Fixture {
            case: case.clone(),
            events: Vec::new(),
        };
        let result = compile_inputs(&mut input, roots);
        let failed = result.is_err();
        let draft = match result {
            Ok(d) => d,
            Err(f) => f.partial,
        };
        let mut actual = snapshot(&draft);
        actual["failed"] = json!(failed);
        actual["events"] = json!(input.events);
        let mut expected = row["output"].clone();
        expected["failed"] = json!(!expected["error"].is_null());
        expected.as_object_mut().unwrap().remove("error");
        assert_eq!(actual, expected, "{} / {}", case["name"], row["precision"]);
    }
}
#[test]
fn native_separate_lists_share_one_node_and_reference_context() {
    let fixture: Value = serde_json::from_str(include_str!(
        "E:/Games/Axis and Alies/rust-rebuild/research/development10/native-fx-prior-lists.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 18);
    for row in fixture["rows"].as_array().unwrap() {
        let case = &row["input"];
        let mut input = Fixture {
            case: case.clone(),
            events: Vec::new(),
        };
        let mut draft = Draft::new();
        let mut prior = Vec::new();
        let mut failed = false;
        for (i, group) in case["groups"].as_array().unwrap().iter().enumerate() {
            if i != 0 {
                prior.push(std::mem::take(&mut draft.roots));
            }
            let roots = group
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as usize)
                .collect();
            if append_roots(&mut input, roots, &mut draft).is_err() {
                failed = true;
                break;
            }
        }
        let mut actual = snapshot(&draft);
        actual["prior_roots"] = json!(
            prior
                .iter()
                .map(|v| v.iter().map(|n| n.index()).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
        actual["failed"] = json!(failed);
        actual["events"] = json!(input.events);
        let mut expected = row["output"].clone();
        expected["failed"] = json!(!expected["error"].is_null());
        expected.as_object_mut().unwrap().remove("error");
        assert_eq!(actual, expected, "{} / {}", case["name"], row["precision"]);
    }
}
#[test]
fn request_only_native_factory_common_prefix_and_unbound_fields() {
    let doc: Value = serde_json::from_str(include_str!(
        "E:/Games/Axis and Alies/rust-rebuild/research/skirmish-20261017-data-pre-request/native-request-only-fxc.json"
    ))
    .unwrap();
    assert_eq!(doc["rows"].as_array().unwrap().len(), 90);
    let fixture: Value = serde_json::from_str(include_str!(
        "E:/Games/Axis and Alies/rust-rebuild/research/skirmish-20261016-fx-graph/native-fx-graph.json"
    ))
    .unwrap();
    let ordinary = &fixture["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["input"]["name"] == "ordinary")
        .unwrap()["input"];
    for row in doc["rows"].as_array().unwrap() {
        let output = &row["output"];
        let kind = output["kind"].as_u64().unwrap() as usize;
        assert_eq!(output["vf24"], json!(u8::from(kind == 0)));
        assert_eq!(output["vf2c"], json!(u8::from(kind == 3)));
        assert_eq!(output["loader_common_first"], true);
        let common = Node::common(vec![]);
        for (offset, word) in &common.words {
            assert_eq!(json!(word), output["words"][format!("{offset:x}")]);
        }
        for (offset, byte) in &common.bytes {
            assert_eq!(json!(byte), output["bytes"][format!("{offset:x}")]);
        }
        for offset in ["8", "c", "10", "14", "20"] {
            assert_eq!(output["words"][offset], 0);
        }
        if matches!(kind, 0 | 1 | 3 | 8) {
            continue;
        }
        let mut case = ordinary.clone();
        case["nodes"][0]["type"] = json!(aa_rules::abilities::FXC_TYPES[kind]);
        //The six subtypes now execute their complete derived loader. A
        //base getter failure still precedes every subtype read in both modes.
        case["request_only"] = json!(true);
        case["fail"] = json!("health");
        let mut input = Fixture {
            case,
            events: vec![],
        };
        let failure = compile_inputs(&mut input, vec![0]).unwrap_err();
        assert!(failure.error.contains("health"));
        assert!(failure.partial.roots.is_empty());
        assert_eq!(failure.partial.nodes[0].word(0x24), Some(1));
        assert_eq!(failure.partial.nodes[0].word(0x28), Some(0));
        assert_eq!(failure.partial.nodes[0].word(0x2c), Some(0));
        assert!(!failure.partial.nodes[0].complete());
    }
}
#[test]
fn full_play_object_animation_loader_matches_native_order_and_failure_prefix() {
    let fixture: Value = serde_json::from_str(include_str!(
        "E:/Games/Axis and Alies/rust-rebuild/research/skirmish-20261017-fx-object-animation/native-fx-object-animation.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 90);
    for row in fixture["rows"].as_array().unwrap() {
        let case = &row["input"];
        let roots = case["roots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        let mut input = Fixture {
            case: case.clone(),
            events: vec![],
        };
        let result = compile_inputs(&mut input, roots);
        let failed = result.is_err();
        let draft = match result {
            Ok(d) => d,
            Err(f) => f.partial,
        };
        let mut actual = snapshot(&draft);
        for (i, node) in draft.nodes.iter().enumerate() {
            if !node.texts.is_empty() {
                actual["nodes"][i]["texts"] = json!(node.texts);
            }
            if draft.kinds[i] == FxcKind::PlayObjectAnimation {
                assert!(node.word(0x3c).is_none());
                assert!(node.reference(0x3c).is_none());
                assert!(node.text(0x3c).is_some());
            }
        }
        actual["failed"] = json!(failed);
        actual["events"] = json!(input.events);
        let mut expected = row["output"].clone();
        expected["failed"] = json!(!expected["error"].is_null());
        expected.as_object_mut().unwrap().remove("error");
        assert_eq!(actual, expected, "{} / {}", case["name"], row["precision"]);
    }
    let node = construct_play_object_animation(vec![]);
    assert_eq!(node.text(0x3c), Some(""));
    assert!(!node.complete());
}
}
