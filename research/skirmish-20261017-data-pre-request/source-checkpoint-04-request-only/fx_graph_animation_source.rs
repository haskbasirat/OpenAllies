//!461F70's ONE contextual FX compiler per actual Animation definition. The
//!same Draft/firstVFX map spans fixed and timed lists; it is moved once into
//!Graph. No Catalog lifetime or copied compiled payload is retained here.
use super::*;
use crate::definitions::Bank;
#[path = "fx_graph_permanence_fields.rs"]
mod permanence;
#[derive(Debug)]
pub(crate) struct Session {
    definition: RuntimeDefinition,
    animation: Vec<Provenance>,
    precision: Precision,
    universe: Vec<SourceIdentity>,
    draft: Option<Draft>,
    lists: Vec<Vec<NodeId>>,
    failed: Option<String>,
}
fn contains(scopes: &[Provenance], node: &ResolvedNode<'_>) -> Result<()> {
    let positions = node.sources();
    let first = positions.first().ok_or("Animation FX source node absent")?;
    if !scopes.iter().any(|s| {
        s.path == first.path
            && s.sha256 == first.sha256
            && s.span.start <= first.span.start
            && s.span.end >= first.span.end
    }) {
        return Err("Animation FX node is outside actual owning source scopes".into());
    }
    Ok(())
}
impl Session {
    pub(crate) fn new(
        catalog: &Catalog<'_>,
        bank: &Bank,
        definition: &RuntimeDefinition,
        animation: &ResolvedNode<'_>,
    ) -> Result<Self> {
        let source = ObjectFxSource::bind_in_bank(catalog, bank, definition)?;
        if definition.proof.category.native_kind != 2 {
            return Err("Animation FX owner requires actual ActorData".into());
        }
        contains(&definition.proof.template_chain, animation)?;
        Ok(Self {
            definition: definition.clone(),
            animation: animation.sources(),
            precision: source.source_precision(),
            universe: catalog.actors.sources.clone(),
            draft: Some(Draft::new()),
            lists: vec![],
            failed: None,
        })
    }
    fn require(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<()> {
        if let Some(error) = &self.failed {
            return Err(error.clone());
        }
        if self.draft.is_none() {
            return Err("Animation FX compiler was already moved into Graph".into());
        }
        let owner = ObjectFxSource::bind_in_bank(catalog, bank, &self.definition)?;
        if owner.source_precision() != self.precision || catalog.actors.sources != self.universe {
            return Err("Animation FX compiler source owner/PC changed".into());
        }
        Ok(())
    }
    ///The concrete461F70 caller supplies the actual native list name/node at
    ///each reached call. Empty children come from this real ResolvedNode.
    pub(crate) fn append(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        node: &ResolvedNode<'_>,
        name: &str,
    ) -> Result<usize> {
        self.require(catalog, bank)?;
        let result: Result<usize> = (|| {
            contains(&self.animation, node)?;
            let roots = node.children(name).map_err(|e| e.to_string())?;
            //Bound contains readonly source/target metadata. Recreating that
            //view does not reset the SAME mutable Draft.first_vfx context.
            let mut bound = Bound::new(
                catalog.actors,
                self.precision,
                self.definition.proof.source.clone(),
            )?;
            bound.request_only = true;
            let draft = self.draft.as_mut().unwrap();
            append_roots(&mut bound, roots, draft)?;
            let index = self.lists.len();
            self.lists.push(std::mem::take(&mut draft.roots));
            Ok(index)
        })();
        if let Err(error) = &result {
            self.failed = Some(error.clone());
        }
        result
    }
    pub(crate) fn append_timed(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        node: &ResolvedNode<'_>,
    ) -> Result<usize> {
        self.require(catalog, bank)?;
        let result: Result<usize> = (|| {
            contains(&self.animation, node)?;
            let mut bound = Bound::new(
                catalog.actors,
                self.precision,
                self.definition.proof.source.clone(),
            )?;
            bound.request_only = true;
            let draft = self.draft.as_mut().unwrap();
            let id = load_node(&mut bound, node, draft)?;
            draft.roots.insert(0, id);
            let index = self.lists.len();
            self.lists.push(std::mem::take(&mut draft.roots));
            Ok(index)
        })();
        if let Err(error) = &result {
            self.failed = Some(error.clone());
        }
        result
    }
    pub(crate) fn roots(&self, index: usize) -> Result<&[NodeId]> {
        self.lists
            .get(index)
            .map(Vec::as_slice)
            .ok_or_else(|| "Animation FX list was not loaded".into())
    }
    pub(crate) fn node(&self, id: NodeId) -> Result<&Node> {
        self.draft
            .as_ref()
            .ok_or("Animation FX compiler already consumed")?
            .nodes
            .get(id.0)
            .ok_or_else(|| "Animation FX node was not allocated".into())
    }
    pub(crate) fn node_kind(&self, id: NodeId) -> Result<FxcKind> {
        self.draft
            .as_ref()
            .ok_or("Animation FX compiler already consumed")?
            .kinds
            .get(id.0)
            .copied()
            .ok_or_else(|| "Animation FX constructor kind absent".into())
    }
    ///428A30 visits each root, immediate list, then chain list. Only the
    ///CreateVFX virtual24 branch reads the shared native virtual20 fields.
    pub(crate) fn permanent(
        &self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        index: usize,
    ) -> Result<bool> {
        self.require(catalog, bank)?;
        struct Topology<'a, 'b> {
            draft: &'a Draft,
            input: &'a Bound<'b>,
        }
        impl permanence::World for Topology<'_, '_> {
            type Node = NodeId;
            fn is_vfx(&self, id: NodeId) -> Result<bool> {
                Ok(*self
                    .draft
                    .kinds
                    .get(id.0)
                    .ok_or("FXC constructor class absent")?
                    == FxcKind::CreateVfx)
            }
            fn active(&self, id: NodeId) -> Result<bool> {
                actor_state::node_active_fields(self.draft, self.input, id)
            }
            fn immediate(&self, id: NodeId) -> Result<&[NodeId]> {
                Ok(self
                    .draft
                    .nodes
                    .get(id.0)
                    .ok_or("FXC arena node absent")?
                    .immediate())
            }
            fn chain(&self, id: NodeId) -> Result<&[NodeId]> {
                Ok(self
                    .draft
                    .nodes
                    .get(id.0)
                    .ok_or("FXC arena node absent")?
                    .chain())
            }
        }
        let input = Bound::new(
            catalog.actors,
            self.precision,
            self.definition.proof.source.clone(),
        )?;
        permanence::scan(
            &Topology {
                draft: self.draft.as_ref().unwrap(),
                input: &input,
            },
            self.roots(index)?,
        )
    }
    ///Consume only after actual source ratios/permanent checks have passed.
    ///On a prior child error the caller retains this Session; it is not ready.
    pub(crate) fn finish(&mut self, catalog: &Catalog<'_>, bank: &Bank) -> Result<Graph> {
        self.require(catalog, bank)?;
        if self.lists.len() < 3 {
            return Err("Animation FX source has not loaded all3 native fixed lists".into());
        }
        let mut lists = std::mem::take(&mut self.lists);
        let mut draft = self.draft.take().unwrap();
        draft.roots = lists.pop().unwrap();
        Ok(Graph {
            draft,
            prior_roots: lists,
            owner: self.definition.proof.id.clone(),
            source: self.definition.proof.source.clone(),
            precision: self.precision,
            sources: self.universe.clone(),
            kind: GraphKind::ActorAnimation,
            request_only: true,
        })
    }
}
impl Graph {
    pub(crate) fn animation_list_roots(&self, index: usize) -> Result<&[NodeId]> {
        if self.kind != GraphKind::ActorAnimation {
            return Err("graph is not actual Actor Animation FX".into());
        }
        if index == self.prior_roots.len() {
            Ok(&self.draft.roots)
        } else {
            self.prior_roots
                .get(index)
                .map(Vec::as_slice)
                .ok_or_else(|| "Animation FX list index absent".into())
        }
    }
}
