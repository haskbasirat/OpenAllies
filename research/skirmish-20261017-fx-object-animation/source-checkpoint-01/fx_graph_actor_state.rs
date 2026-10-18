//! ActorData464F99..464FE7 WhenEnter compilation and4D0A00 root scan.
//! Enter/Leave share the actual local reference context. The bounded42B8E0
//! synthesizer preserves traversal/prepend/link order; unrelated ActorData
//! loading and native allocation failure are not implied.
use super::*;
/// A non-Clone capability produced only by the actual ActorEnter root scan.
/// It retains the same immutable compiled graph and captured native arguments.
#[derive(Debug)]
pub struct MatchedState {
    pub(crate) graph: std::sync::Arc<Graph>,
    pub(crate) state: u32,
    pub(crate) active_only: bool,
}
impl Graph {
    ///The common4D0A00 scan using the current moved Bank owner.
    pub fn match_actor_state_in_bank(
        self: &std::sync::Arc<Self>,
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
        state: u32,
        active_only: bool,
    ) -> Result<Option<MatchedState>> {
        self.require_runtime_fields()?;
        if !matches!(self.kind, GraphKind::ActorEnter | GraphKind::ActorLeave) {
            return Err("state scan requires ActorEnter/Leave graph".into());
        }
        self.require_in_bank(catalog, bank)?;
        let input = Bound::new(catalog.actors, self.precision, self.source.clone())?;
        if scan(
            &self.draft.roots,
            state,
            active_only,
            |id| self.node_active(&input, id),
            |id| {
                self.draft
                    .nodes
                    .get(id.0)
                    .and_then(|n| n.word(0x34))
                    .ok_or("compiled state FXC+34 absent".into())
            },
        )? {
            Ok(Some(MatchedState {
                graph: std::sync::Arc::clone(self),
                state,
                active_only,
            }))
        } else {
            Ok(None)
        }
    }
    pub fn match_actor_enter(
        self: &std::sync::Arc<Self>,
        catalog: &Catalog<'_>,
        state: u32,
        active_only: bool,
    ) -> Result<Option<MatchedState>> {
        if self.actor_enter_matches(catalog, state, active_only)? {
            Ok(Some(MatchedState {
                graph: std::sync::Arc::clone(self),
                state,
                active_only,
            }))
        } else {
            Ok(None)
        }
    }
}

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

impl Graph {
    ///464F99..465002 shares one local reference map across both state lists,
    ///validates each completed list, then recursively synthesizes StopVFX.
    pub fn compile_actor_leave_in_bank(
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
        definition: &RuntimeDefinition,
    ) -> std::result::Result<Self, Failure> {
        Self::compile_actor_leave_mode(catalog, bank, definition, false)
    }
    pub(crate) fn compile_actor_leave_request_in_bank(
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
        definition: &RuntimeDefinition,
    ) -> std::result::Result<Self, Failure> {
        Self::compile_actor_leave_mode(catalog, bank, definition, true)
    }
    fn compile_actor_leave_mode(
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
        definition: &RuntimeDefinition,
        request_only: bool,
    ) -> std::result::Result<Self, Failure> {
        let source =
            ObjectFxSource::bind_in_bank(catalog, bank, definition).map_err(|error| Failure {
                error,
                partial: Draft::new(),
            })?;
        let mut enter =
            Self::compile_actor_enter_source(catalog, definition, source, request_only)?;
        let enter_roots = std::mem::take(&mut enter.draft.roots);
        let mut input =
            Bound::new(catalog.actors, enter.precision, enter.source.clone()).map_err(|error| {
                Failure {
                    error,
                    partial: Draft::new(),
                }
            })?;
        input.request_only = request_only;
        let actor = catalog
            .actors
            .resolved_actor(&enter.owner)
            .map_err(|e| Failure {
                error: e.to_string(),
                partial: Draft::new(),
            })?;
        let roots = actor.children("WhenLeave").map_err(|e| Failure {
            error: e.to_string(),
            partial: Draft::new(),
        })?;
        if let Err(error) = append_roots(&mut input, roots, &mut enter.draft) {
            return Err(Failure {
                error,
                partial: enter.draft,
            });
        }
        if enter
            .draft
            .roots
            .iter()
            .any(|id| enter.draft.nodes[id.0].word(0x34) == Some(42))
        {
            return Err(Failure {
                error: "WhenLeave FXC requires a concrete actor_state (464FC9)".into(),
                partial: enter.draft,
            });
        }
        //42B8E0 reads children in head order, recurses Immediate then Chain,
        //and only then considers the parent. Synthesized nodes PREPEND.
        if let Err(error) = synthesize_stops(&input, &enter_roots, &mut enter.draft) {
            return Err(Failure {
                error,
                partial: enter.draft,
            });
        }
        enter.prior_roots.push(enter_roots);
        enter.kind = GraphKind::ActorLeave;
        Ok(enter)
    }
}
pub(super) fn synthesize_stops(
    input: &Bound<'_>,
    roots: &[NodeId],
    draft: &mut Draft,
) -> Result<()> {
    append_synthetic_stops(roots, draft, &mut |draft, id| {
        active_fields(
            FxcKind::CreateVfx,
            || {
                let reference = draft.nodes[id.0]
                    .reference(0x3c)
                    .and_then(Option::as_ref)
                    .ok_or("Stop source VFX reference absent")?;
                let source = input.canonical_node(
                    reference
                        .source
                        .first()
                        .ok_or("Stop source VFX provenance absent")?,
                )?;
                let permanent = input
                    .value(&source, "permanent")?
                    .boolean()
                    .map_err(|e| e.to_string())?;
                match input.value(&source, "duration")? {
                    TypedValue::Scalar(v)
                        if v.field_type == ScalarType::Float
                            && f32::from_bits(v.bits).is_finite() => {}
                    _ => return Err("Stop source VFX duration finite domain required".into()),
                }
                Ok(permanent)
            },
            || {
                draft.nodes[id.0]
                    .byte(0x64)
                    .ok_or("Stop source duration flag absent".into())
            },
        )
    })
}
fn append_synthetic_stops(
    roots: &[NodeId],
    draft: &mut Draft,
    active: &mut impl FnMut(&Draft, NodeId) -> Result<bool>,
) -> Result<()> {
    for &id in roots {
        let immediate = draft.nodes[id.0].immediate.clone();
        let chain = draft.nodes[id.0].chain.clone();
        append_synthetic_stops(&immediate, draft, active)?;
        append_synthetic_stops(&chain, draft, active)?;
        if draft.kinds[id.0] != FxcKind::CreateVfx || draft.nodes[id.0].byte(0x65) != Some(1) {
            continue;
        }
        if !active(draft, id)? {
            continue;
        }
        let trigger = draft.nodes[id.0]
            .word(0x30)
            .ok_or("Stop source health trigger absent")?;
        if matches!(trigger, 2 | 3) {
            continue;
        }
        let mut node = construct_stop_vfx(draft.nodes[id.0].source.clone());
        node.node_links.insert(0x3c, id);
        for offset in [0x24, 0x28, 0x2c, 0x18, 0x1c, 0x34] {
            node.words.insert(
                offset,
                draft.nodes[id.0]
                    .word(offset)
                    .ok_or("Stop source common word absent")?,
            );
        }
        node.references.insert(
            0x20,
            draft.nodes[id.0]
                .reference(0x20)
                .cloned()
                .ok_or("Stop source terrain filter absent")?,
        );
        if trigger == 0 {
            node.words.insert(0x30, 1);
        } else if trigger == 1 {
            node.words.insert(0x30, 0);
        }
        node.complete = true;
        let stop = NodeId(draft.nodes.len());
        draft.nodes.push(node);
        draft.kinds.push(FxcKind::StopVfx);
        draft.roots.insert(0, stop);
        draft.nodes[id.0].node_links.insert(0x68, stop);
    }
    Ok(())
}

impl Graph {
    ///464BD0 has a fresh local FXC reference map after ObjectData::LoadData.
    ///42BF10 completes the entire Enter list before its state42 validation.
    pub fn compile_actor_enter(
        catalog: &Catalog<'_>,
        definition: &RuntimeDefinition,
    ) -> std::result::Result<Self, Failure> {
        let failure = |error: String| Failure {
            error,
            partial: Draft::new(),
        };
        if !matches!(definition.native_class, 3 | 4) {
            return Err(failure("WhenEnter requires actual ActorData class".into()));
        }
        let source = ObjectFxSource::bind(catalog, definition).map_err(failure)?;
        Self::compile_actor_enter_source(catalog, definition, source, false)
    }
    ///Identical compiler with the SAME moved source Bank as the current Actor.
    pub fn compile_actor_enter_in_bank(
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
        definition: &RuntimeDefinition,
    ) -> std::result::Result<Self, Failure> {
        let source =
            ObjectFxSource::bind_in_bank(catalog, bank, definition).map_err(|error| Failure {
                error,
                partial: Draft::new(),
            })?;
        Self::compile_actor_enter_source(catalog, definition, source, false)
    }
    fn compile_actor_enter_source(
        catalog: &Catalog<'_>,
        definition: &RuntimeDefinition,
        source: ObjectFxSource,
        request_only: bool,
    ) -> std::result::Result<Self, Failure> {
        let failure = |error: String| Failure {
            error,
            partial: Draft::new(),
        };
        if !matches!(definition.native_class, 3 | 4) {
            return Err(failure("WhenEnter requires actual ActorData class".into()));
        }
        let node = catalog
            .actors
            .resolved_actor(source.canonical_id())
            .map_err(|e| failure(e.to_string()))?;
        let roots = node
            .children("WhenEnter")
            .map_err(|e| failure(e.to_string()))?;
        let mut input = Bound::new(
            catalog.actors,
            source.source_precision(),
            source.definition_source().clone(),
        )
        .map_err(failure)?;
        input.request_only = request_only;
        let mut draft = Draft::new();
        if let Err(error) = append_roots(&mut input, roots, &mut draft) {
            return Err(Failure {
                error,
                partial: draft,
            });
        }
        for id in &draft.roots {
            if draft.nodes[id.0].word(0x34) == Some(42) {
                return Err(Failure {
                    error: "WhenEnter FXC requires a concrete actor_state (464FC9)".into(),
                    partial: draft,
                });
            }
        }
        Ok(Self {
            draft,
            prior_roots: Vec::new(),
            owner: source.canonical_id().into(),
            source: source.definition_source().clone(),
            precision: source.source_precision(),
            sources: catalog.actors.sources.clone(),
            kind: GraphKind::ActorEnter,
            request_only,
        })
    }
    ///4D0A00 reads vf20 before+34 only when active_only is nonzero. No
    ///position/worker/time/presentation effect is reached without a match.
    pub fn actor_enter_matches(
        &self,
        catalog: &Catalog<'_>,
        state: u32,
        active_only: bool,
    ) -> Result<bool> {
        self.require_runtime_fields()?;
        if self.kind != GraphKind::ActorEnter {
            return Err("4D0A00 requires actual ActorData WhenEnter graph".into());
        }
        self.require_catalog(catalog)?;
        let input = Bound::new(catalog.actors, self.precision, self.source.clone())?;
        scan(
            &self.draft.roots,
            state,
            active_only,
            |id| self.node_active(&input, id),
            |id| {
                self.draft
                    .nodes
                    .get(id.0)
                    .and_then(|n| n.word(0x34))
                    .ok_or("compiled FXC state field absent".into())
            },
        )
    }
    fn node_active(&self, input: &Bound<'_>, id: NodeId) -> Result<bool> {
        node_active_fields(&self.draft, input, id)
    }
    ///Read the real immutable FXC virtual20 against its bound source owner.
    pub fn actor_state_node_active_in_bank(
        &self,
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
        id: NodeId,
    ) -> Result<bool> {
        self.require_runtime_fields()?;
        if !matches!(self.kind, GraphKind::ActorEnter | GraphKind::ActorLeave) {
            return Err("state node predicate requires actual Actor state graph".into());
        }
        self.require_in_bank(catalog, bank)?;
        let input = Bound::new(catalog.actors, self.precision, self.source.clone())?;
        self.node_active(&input, id)
    }
}
///Shared immutable virtual20 predicate over the SAME compiled arena.
pub(super) fn node_active_fields(draft: &Draft, input: &Bound<'_>, id: NodeId) -> Result<bool> {
    let node = draft.nodes.get(id.0).ok_or("FXC arena node absent")?;
    active_fields(
        draft
            .kinds
            .get(id.0)
            .copied()
            .ok_or("FXC constructor class absent")?,
        || {
            let reference = node
                .reference(0x3c)
                .and_then(Option::as_ref)
                .ok_or("CreateVFX source reference absent")?;
            let source = input.canonical_node(
                reference
                    .source
                    .first()
                    .ok_or("VFX canonical source absent")?,
            )?;
            //494F30 always reads permanent then duration. In the finite
            //source domain Data+24==1 iff permanent is true; all other
            //branches produce0,2 or3, independent of model-kind virtual.
            let permanent = input
                .value(&source, "permanent")?
                .boolean()
                .map_err(|e| e.to_string())?;
            match input.value(&source, "duration")? {
                TypedValue::Scalar(value)
                    if value.field_type == ScalarType::Float
                        && f32::from_bits(value.bits).is_finite() => {}
                _ => return Err("VFX duration requires proved finite binary32 source".into()),
            }
            Ok(permanent)
        },
        || {
            node.byte(0x64)
                .ok_or("CreateVFX duration flag absent".into())
        },
    )
}
/// Immutable graph domain: the native virtual predicate does not mutate its
/// source list. Required failures retain the exact completed callback prefix.
fn scan(
    roots: &[NodeId],
    state: u32,
    active_only: bool,
    mut active: impl FnMut(NodeId) -> Result<bool>,
    mut node_state: impl FnMut(NodeId) -> Result<u32>,
) -> Result<bool> {
    for &id in roots {
        if active_only && !active(id)? {
            continue;
        }
        if node_state(id)? == state {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Actual6A71F0 or428360 leaf selection and short-circuit reads.
fn active_fields(
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

#[cfg(test)]
#[path = "fx_graph_actor_leave_tests.rs"]
mod leave_tests;
#[cfg(test)]
#[path = "fx_graph_actor_state_tests.rs"]
mod tests;
