//! Native BuilderComponent build targets at ActorData+504/+508.
//! 464BD0 creates references in authored/registration order; 473AD0 prepends
//! deferrals and 475230 calls them head first. Each 4604C0 prepends one target,
//! so completed per-owner lists retain authored order, including duplicates.
//! This is the Builder callback subsequence, not complete global LoadData.
use crate::{actors::{ActorCatalog, ResolvedNode}, native::{CATEGORIES, Error, Result, TypedValue}, resources::Provenance};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn invalid(message: impl Into<String>) -> Error { Error::Unsupported(message.into()) }
fn lookup_key(value: &str) -> String {
    value.split('\0').next().unwrap_or_default().to_ascii_lowercase()
}
/// Both standard ActorData and RandomActorData inherit 4624B0/464BD0;
/// native FindActor's virtual+38 accepts both (A05540/A09B30 -> 4F0500).
fn actor_data_kind(kind: u32) -> bool { matches!(kind, 2 | 19) }
fn text(node: &ResolvedNode<'_>, field: &str) -> Result<String> {
    match node.plain_string_field(field)? {
        Some(TypedValue::String { value, .. }) => Ok(value),
        _ => Err(invalid(format!("BuilderData requires string {field}"))),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActorInput {
    pub id: String,
    /// Only the native Body bit0 predicate required by 4604C0.
    pub body_present: bool,
    /// Builder flag18 and optional-node presence, never an inferred target list.
    pub builder_present: bool,
    pub targets: Vec<String>,
    #[serde(default, skip_deserializing)]
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inputs {
    /// Canonical registered ActorData identities in native registration order.
    pub actors: Vec<ActorInput>,
    /// Final typed FindActor(2B) identities, with current Translate applied once.
    pub lookups: BTreeMap<String, String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeHandle(pub usize);
#[derive(Clone, Debug, Serialize)]
pub struct BuilderNode {
    pub target: String,
    pub next: Option<NodeHandle>,
}
#[derive(Clone, Debug, Serialize)]
pub struct BuilderDefinition {
    pub id: String,
    pub body_present: bool,
    pub builder_present: bool,
    /// Head-first native +504 values. Stable and immutable after completion.
    pub ids: Vec<String>,
    pub head: Option<NodeHandle>,
    pub tail: Option<NodeHandle>,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum FailureKind { UnknownActor, MissingBody }
#[derive(Clone, Debug, Serialize)]
pub struct ResolutionFailure {
    pub owner: String,
    pub authored: String,
    pub kind: FailureKind,
}
#[derive(Clone, Debug, Serialize)]
pub struct NativeBuilderData {
    pub definitions: BTreeMap<String, BuilderDefinition>,
    pub nodes: Vec<BuilderNode>,
    /// Attempted callback order, including the failing reference if present.
    pub resolution_order: Vec<(u32, String, String)>,
    pub failure: Option<ResolutionFailure>,
}
impl NativeBuilderData {
    /// A canonical identity already obtained from registered data. No alias or
    /// Translate lookup occurs here, including for shadowed canonical owners.
    pub fn definition(&self, canonical: &str) -> Result<&BuilderDefinition> {
        self.definitions.get(&canonical.to_ascii_lowercase())
            .ok_or_else(|| invalid(format!("canonical Builder ActorData identity unavailable: {canonical}")))
    }
    pub fn node(&self, handle: NodeHandle) -> Result<&BuilderNode> {
        self.nodes.get(handle.0).ok_or_else(|| invalid("Builder list node outside this source arena"))
    }
    pub fn require_complete(&self) -> Result<()> {
        if let Some(failure) = &self.failure {
            return Err(invalid(match failure.kind {
                FailureKind::UnknownActor => format!("4604C0 unknown build target {:?} for {}; earlier Builder callbacks remain committed", failure.authored, failure.owner),
                FailureKind::MissingBody => format!("4604C0 build target {:?} has no Body for {}; earlier Builder callbacks remain committed", failure.authored, failure.owner),
            }));
        }
        Ok(())
    }
}
impl Inputs {
    /// Preserve successful earlier callbacks and their exact head/tail nodes
    /// when a later target fails. The failing callback has no allocation/store.
    /// Successful allocation is an explicit precondition; no heap failure or
    /// allocator reentrancy is fabricated by this immutable source producer.
    pub fn resolve_partial(&self) -> Result<NativeBuilderData> {
        let mut definitions = BTreeMap::new();
        for actor in &self.actors {
            if definitions.insert(actor.id.to_ascii_lowercase(), BuilderDefinition {
                id: actor.id.clone(), body_present: actor.body_present, builder_present: actor.builder_present,
                ids: vec![], head: None, tail: None, sources: actor.sources.clone(),
            }).is_some() { return Err(invalid("duplicate canonical Builder ActorData identity")); }
            if !actor.builder_present && !actor.targets.is_empty() {
                return Err(invalid("Builder targets supplied without BuilderComponent"));
            }
        }
        let mut arena = NativeBuilderData { definitions, nodes: vec![], resolution_order: vec![], failure: None };
        'callbacks: for owner in self.actors.iter().rev() {
            for authored in owner.targets.iter().rev() {
                arena.resolution_order.push((0x4604c0, owner.id.clone(), authored.clone()));
                let target = self.lookups.get(&lookup_key(authored))
                    .and_then(|id| arena.definitions.get(&id.to_ascii_lowercase()));
                let failure = match target {
                    None => Some(FailureKind::UnknownActor),
                    Some(target) if !target.body_present => Some(FailureKind::MissingBody),
                    Some(_) => None,
                };
                if let Some(kind) = failure {
                    arena.failure = Some(ResolutionFailure { owner: owner.id.clone(), authored: authored.clone(), kind });
                    break 'callbacks;
                }
                let target = target.unwrap().id.clone();
                let owner = arena.definitions.get_mut(&owner.id.to_ascii_lowercase()).unwrap();
                let handle = NodeHandle(arena.nodes.len());
                arena.nodes.push(BuilderNode { target, next: owner.head });
                let tail_was_null = owner.tail.is_none();
                owner.head = Some(handle);
                if tail_was_null { owner.tail = Some(handle); }
            }
        }
        for owner in arena.definitions.values_mut() {
            let mut node = owner.head;
            while let Some(handle) = node {
                let current = &arena.nodes[handle.0];
                owner.ids.push(current.target.clone());
                node = current.next;
            }
        }
        Ok(arena)
    }
    /// Only successful Builder reference-subsequence completion. An unrelated
    /// native deferred callback can still fail the global source lifecycle.
    pub fn complete(&self) -> Result<NativeBuilderData> {
        let arena = self.resolve_partial()?;
        arena.require_complete()?;
        Ok(arena)
    }
}
impl ActorCatalog {
    /// Extract provenance-bound registered owners and CURRENT typed reference
    /// lookup. This does not reconstruct historical partial LoadData failures
    /// in unrelated components or execute other global deferred callbacks.
    pub fn native_builder_inputs(&self) -> Result<Inputs> {
        if self.native_data_registry("NameList").is_none() {
            return Err(invalid("Builder data requires native registration order"));
        }
        let mut roots = Vec::new();
        for tag in crate::actors::native_load::THING_TAGS { roots.extend(self.resolved_definitions(tag)?); }
        let mut actors = Vec::new();
        let mut lookups = BTreeMap::new();
        for registered in &self.actors {
            let node = roots.iter().find(|node| node.sources().first() == Some(&registered.source))
                .ok_or_else(|| invalid(format!("canonical Builder source root unavailable: {}", registered.id)))?;
            let category = text(node, "category")?;
            if !CATEGORIES.iter().any(|c| c.name.eq_ignore_ascii_case(&category) && actor_data_kind(c.native_kind)) { continue; }
            let builder = node.child("BuilderComponent")?;
            let targets = match &builder {
                Some(builder) => builder.children("BuildActor")?.iter()
                    .map(|entry| text(entry, "actor_ids")).collect::<Result<Vec<_>>>()?,
                None => vec![],
            };
            for authored in &targets {
                let key = lookup_key(authored);
                if let Some(found) = self.find(&key)
                    && actor_data_kind(self.native_category(&key)?.native_kind)
                { lookups.insert(key, found.id.clone()); }
            }
            actors.push(ActorInput { id: registered.id.clone(), body_present: node.child("BodyComponent")?.is_some(),
                builder_present: builder.is_some(), targets, sources: node.sources() });
        }
        Ok(Inputs { actors, lookups })
    }
    pub fn native_builder_data(&self) -> Result<NativeBuilderData> { self.native_builder_inputs()?.complete() }
}
