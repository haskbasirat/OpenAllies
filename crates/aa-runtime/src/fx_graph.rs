//!42B990/42BF10 FXC compilation into owned, immutable command definitions.
//! A source node is not a compiled command. A node joins its parent's native
//! head-first list only after its LoadData returns; failures retain the draft.
use crate::{
    catalog::Catalog,
    definition_fx::{ListKind, Source as ObjectFxSource},
};
use aa_mapworld::restore::RuntimeDefinition;
use aa_numeric::Precision;
use aa_rules::{
    actors::{ActorCatalog, ResolvedNode},
    native::{ScalarType, TypedValue},
    resources::{Provenance, SourceIdentity},
    targets::TargetDomain,
};
use std::collections::BTreeMap;

pub type Result<T> = std::result::Result<T, String>;
#[path = "fx_graph_texture.rs"]
mod texture;
pub use texture::SourceGraphs;
/// Actual constructor/vtable class selected by42B6E0. This is retained by
/// the immutable arena; it is not guessed from source field layout later.
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
#[path = "fx_graph_actor_request_source.rs"]
pub(crate) mod actor_request_source;
#[path = "fx_graph_actor_state.rs"]
mod actor_state;
#[path = "fx_graph_animation_source.rs"]
pub(crate) mod animation_source;
pub use actor_state::MatchedState;
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
/// Completed graph owner; only the source-bound producer can construct it.
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
///Borrowed capability to an actual reference field in a completed graph.
///A public diagnostic Reference value cannot manufacture this source handle.
pub struct RegisteredReference<'g> {
    graph: &'g Graph,
    reference: &'g Reference,
    kind: FxcKind,
    offset: u32,
}
impl RegisteredReference<'_> {
    pub fn require_in_bank(
        &self,
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
    ) -> Result<()> {
        self.graph.require_in_bank(catalog, bank)
    }
    pub fn reference(&self) -> &Reference {
        self.reference
    }
    pub fn require_audio_reference(
        &self,
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
    ) -> Result<()> {
        self.require_in_bank(catalog, bank)?;
        if self.kind != FxcKind::PlaySfx || self.offset != 0x44 {
            return Err("compiled reference is not actual PlaySFX Audio44".into());
        }
        Ok(())
    }
    pub fn require_vfx_reference(
        &self,
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
    ) -> Result<()> {
        self.require_in_bank(catalog, bank)?;
        if self.kind != FxcKind::CreateVfx || self.offset != 0x3c {
            return Err("compiled reference is not actual CreateVFX Data3C".into());
        }
        Ok(())
    }
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
}
impl Graph {
    pub fn nodes(&self) -> &[Node] {
        self.draft.nodes()
    }
    pub fn roots(&self) -> &[NodeId] {
        self.draft.roots()
    }
    pub fn owner(&self) -> &str {
        &self.owner
    }
    pub fn kind(&self) -> GraphKind {
        if self.request_only {
            GraphKind::RequestOnly
        } else {
            self.kind
        }
    }
    pub fn is_request_only(&self) -> bool {
        self.request_only
    }
    pub(super) fn require_runtime_fields(&self) -> Result<()> {
        if self.request_only || self.draft.nodes.iter().any(|n| !n.complete) {
            return Err("resource-request-only or incomplete FX graph cannot execute or submit commands".into());
        }
        Ok(())
    }
    pub fn list_kind(&self) -> Option<ListKind> {
        match self.kind {
            GraphKind::View(kind) => Some(kind),
            _ => None,
        }
    }
    ///Identity only, minted from an actually allocated completed Graph node.
    pub fn node_id(&self, index: usize) -> Result<NodeId> {
        self.draft.nodes.get(index).ok_or("FXC arena node absent")?;
        Ok(NodeId(index))
    }
    pub fn node_kind(&self, id: NodeId) -> Option<FxcKind> {
        self.draft.kinds.get(id.0).copied()
    }
    pub fn source_precision(&self) -> Precision {
        self.precision
    }
    pub fn require_catalog(&self, catalog: &Catalog<'_>) -> Result<()> {
        let bank = catalog
            .definition_bank
            .try_borrow()
            .map_err(|_| "FX graph nested Bank borrow")?;
        self.require_in_bank(catalog, &bank)
    }
    ///The same source identity check after the authoritative Bank is moved.
    pub fn require_in_bank(
        &self,
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
    ) -> Result<()> {
        catalog.require_source_owner()?;
        if catalog.actors.sources != self.sources {
            return Err("FX graph source universe differs".into());
        }
        let current = catalog
            .actors
            .actors
            .iter()
            .find(|a| a.id.eq_ignore_ascii_case(&self.owner))
            .ok_or("FX graph owner missing")?;
        if current.source != self.source {
            return Err("FX graph owner source changed".into());
        }
        if bank.combat_source()?.source_precision() != self.precision {
            return Err("FX graph Bank precision differs".into());
        }
        Ok(())
    }
    pub fn compile(
        catalog: &Catalog<'_>,
        definition: &RuntimeDefinition,
        kind: ListKind,
    ) -> std::result::Result<Self, Failure> {
        let failure = |error: String| Failure {
            error,
            partial: Draft::new(),
        };
        let source = ObjectFxSource::bind(catalog, definition).map_err(failure)?;
        Self::compile_bound(catalog, kind, source, false)
    }
    ///Same compiler after the SAME source Bank moves into Staged/World.
    pub fn compile_in_bank(
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
        definition: &RuntimeDefinition,
        kind: ListKind,
    ) -> std::result::Result<Self, Failure> {
        let source =
            ObjectFxSource::bind_in_bank(catalog, bank, definition).map_err(|error| Failure {
                error,
                partial: Draft::new(),
            })?;
        Self::compile_bound(catalog, kind, source, false)
    }
    ///The SAME compiler/arena, restricted to resource requests only.
    pub fn compile_request_in_bank(
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
        d: &RuntimeDefinition,
        kind: ListKind,
    ) -> std::result::Result<Self, Failure> {
        let source = ObjectFxSource::bind_in_bank(catalog, bank, d).map_err(|error| Failure {
            error,
            partial: Draft::new(),
        })?;
        Self::compile_bound(catalog, kind, source, true)
    }
    pub fn view_list_roots(&self, kind: ListKind) -> Result<&[NodeId]> {
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
    fn require_node_kind(&self, id: NodeId) -> Result<FxcKind> {
        if !self.draft.nodes.get(id.0).is_some_and(Node::complete) {
            return Err("FXC reference requires complete actual node".into());
        }
        self.draft
            .kinds
            .get(id.0)
            .copied()
            .ok_or_else(|| "FXC constructor kind absent".into())
    }
    pub fn registered_reference(
        &self,
        id: NodeId,
        offset: u32,
    ) -> Result<Option<RegisteredReference<'_>>> {
        let kind = self.require_node_kind(id)?;
        self.draft.nodes[id.0]
            .reference(offset)
            .ok_or("FXC reference field was never stored".into())
            .map(|reference| {
                reference.as_ref().map(|reference| RegisteredReference {
                    graph: self,
                    reference,
                    kind,
                    offset,
                })
            })
    }
    fn compile_bound(
        catalog: &Catalog<'_>,
        kind: ListKind,
        source: ObjectFxSource,
        request_only: bool,
    ) -> std::result::Result<Self, Failure> {
        Self::compile_bound_textures(catalog,kind,source,request_only,None)
    }
    fn compile_bound_textures<'a>(
        catalog: &'a Catalog<'_>,kind:ListKind,source:ObjectFxSource,request_only:bool,
        textures:Option<Box<dyn texture::Textures+'a>>,
    ) -> std::result::Result<Self,Failure>{
        let failure = |error: String| Failure {
            error,
            partial: Draft::new(),
        };
        let mut inputs = Bound::new(
            catalog.actors,
            source.source_precision(),
            source.definition_source().clone(),
        )
        .map_err(failure)?;
        inputs.request_only = request_only;
        inputs.textures = textures;
        let actor = catalog
            .actors
            .resolved_actor(aa_rules::actors::ThingRef::registered(source.canonical_id()))
            .map_err(|e| failure(e.to_string()))?;
        let view = actor.child("View").map_err(|e| failure(e.to_string()))?;
        let mut draft = Draft::new();
        let mut prior_roots = Vec::new();
        for current in ListKind::ALL.into_iter().take(kind as usize + 1) {
            if current != ListKind::Created {
                prior_roots.push(std::mem::take(&mut draft.roots));
            }
            let roots = match &view {
                Some(view) => view
                    .children(current.node_name())
                    .map_err(|e| failure(e.to_string()))?,
                None => Vec::new(),
            };
            if let Err(error) = append_roots(&mut inputs, roots, &mut draft) {
                return Err(Failure {
                    error,
                    partial: draft,
                });
            }
        }
        Ok(Self {
            draft,
            prior_roots,
            owner: source.canonical_id().into(),
            source: source.definition_source().clone(),
            precision: source.source_precision(),
            sources: catalog.actors.sources.clone(),
            kind: GraphKind::View(kind),
            request_only,
        })
    }
}

struct Bound<'a> {
    request_only: bool,
    actors: &'a ActorCatalog,
    pc: Precision,
    owner: Provenance,
    targets: Option<TargetDomain>,
    textures: Option<Box<dyn texture::Textures+'a>>,
}
impl<'a> Bound<'a> {
    fn new(actors: &'a ActorCatalog, pc: Precision, owner: Provenance) -> Result<Self> {
        Ok(Self {
            request_only: false,
            actors,
            pc,
            owner,
            targets: None,
            textures: None,
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
///The SAME parse-time DataManager Finds as FXC LoadData, lent to another
///native source loader that runs inside its owner's LoadData (ObjectAnimation
///49D0A0/49CF80). Both callers use identical manager slots and calls:
///416370 on [B36530]+584 (42C3C0), 4BDB70 on [B36530]+C and 42C070 on
///[B36530]+580 (42C5E0). No second registry search exists here.
pub(crate) struct OwnerLookups<'a>(Bound<'a>);
impl<'a> OwnerLookups<'a> {
    pub(crate) fn new(actors: &'a ActorCatalog, pc: Precision, owner: Provenance) -> Result<Self> {
        Bound::new(actors, pc, owner).map(Self)
    }
    ///416370 Audio Find.
    pub(crate) fn audio(&mut self, ids: &str) -> Result<Option<Reference>> {
        self.0.find("Audio", ids)
    }
    ///The Audio manager+20 name read by a 410C40 unknown-IDS diagnostic.
    pub(crate) fn audio_manager(&self) -> Result<String> {
        sound::audio_manager_name(&self.0)
    }
    ///4BDB70 VFX Thing Find.
    pub(crate) fn vfx(&mut self, ids: &str) -> Result<Option<Reference>> {
        self.0.find("VFX", ids)
    }
    ///42C070 Bone Find, then the found Bone Data+C.
    pub(crate) fn bone(&mut self, ids: &str) -> Result<Option<u32>> {
        match self.0.find("Bone", ids)? {
            None => Ok(None),
            Some(found) => found.ordinal.map(Some).ok_or_else(|| "Bone Data+C ordinal absent".into()),
        }
    }
}
impl<'a> Inputs for Bound<'a> {
    type Source = ResolvedNode<'a>;
    fn texture_reference(&mut self,_:&Self::Source,ids:&str)->Result<Option<crate::resource_objects::Handle>>{
        self.textures.as_mut().ok_or("FX Texture reference requires the actual source ResourceManager loan")?.find(ids)
    }
    fn radar_texture_policy(&mut self,handle:&crate::resource_objects::Handle)->Result<()>{
        self.textures.as_mut().ok_or("FX radar policy requires the actual source ResourceManager loan")?.radar(handle)
    }
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

#[path = "fx_graph_sound.rs"]
mod sound;
#[path="fx_graph_subtypes.rs"]
mod subtypes;

#[cfg(test)]
#[path = "fx_graph_tests.rs"]
mod tests;

///4293C0/4286E0's separate event: copied source position, target=NULL and
/// native global target position, current state word54 and active-only byte58.
pub trait StateCreationInputs: CreationInputs {
    fn global_position(&mut self) -> Result<[u32; 3]>;
}
pub fn construct_actor_state(
    matched: MatchedState,
    input: &mut impl StateCreationInputs,
) -> Result<Option<Command>> {
    matched.graph.require_runtime_fields()?;
    if !matches!(
        matched.graph.kind(),
        GraphKind::ActorEnter | GraphKind::ActorLeave
    ) {
        return Err("state event requires matched ActorEnter/ActorLeave graph".into());
    }
    //4D0A49 copies the current position before4293C0 tests mute.
    let position = input.current_position()?;
    if input.muted()? {
        return Ok(None);
    }
    let kingdom = input.creator_kingdom()?;
    let source = input.current_thing()?;
    let current = input.current_os_thread_id()?;
    let app = input.application_os_thread_id()?;
    let timestamp = if current == app {
        input.view_time()?
    } else {
        input.world_time()?
    };
    //4286E0 reads this actual global after selecting the real clock.
    let target_position = input.global_position()?;
    let mut command = finish_command(
        matched.graph,
        kingdom,
        source,
        None,
        position,
        target_position,
        timestamp,
        u8::from(matched.active_only),
    )?;
    if let Some(command) = command.as_mut() {
        command.words.insert(0x54, matched.state);
    }
    Ok(command)
}
