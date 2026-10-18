// Generated exact production source slices; no alternative compiler.
use aa_rules::resources::Provenance;
use std::collections::BTreeMap;
pub type Result<T> = std::result::Result<T,String>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum FxcKind {
    CreateVfx,
    PlaySfx,
    StopVfx,
    ///42B130/42CF10 owns the actual object_animation_IDS CString at+3C.
    PlayObjectAnimation,
    ///Constructor-proven false vf24/vf2C; subtype fields remain UNBOUND.
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
    if !matches!(kind, 0 | 1 | 3 | 8)
        && !(input.request_only() && matches!(kind, 2 | 4 | 5 | 6 | 7 | 9))
    {
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
        _ => FxcKind::RequestOnly {
            native_type: kind as u8,
        },
    });
    draft.nodes.push(match kind {
        0 => Node::create_vfx(provenance),
        1 => actor_state::construct_stop_vfx(provenance),
        3 => sound::construct_play_sfx(provenance),
        8 => construct_play_object_animation(provenance),
        _ => Node::common(provenance),
    });
    load_base(input, source, draft, id)?;
    match kind {
        0 => load_create_vfx(input, source, draft, id),
        1 => actor_state::load_stop_vfx(input, source, draft, id),
        3 => sound::load_play_sfx(input, source, draft, id),
        8 => load_play_object_animation(input, source, draft, id),
        //Only common fields are projected; complete stays false.
        _ => Ok(id),
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

mod sound { use super::*;
///42ADD0 writes only the common constructor fields. +3C/+40/+44 are
/// genuinely unwritten until their loader stores; CreateVFX's +68 is absent.
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
mod actor_state {use super::*;
///42AF90 actual constructor. The source-node link at3C is initially null.
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
mod tests {
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
fn full_play_object_animation_loader_matches_native_order_and_failure_prefix() {
    let fixture: Value = serde_json::from_str(include_str!(
        "native-fx-object-animation.json"
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
