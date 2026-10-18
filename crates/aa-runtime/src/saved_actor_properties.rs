//! Native4D4CF0 build-target list refresh on actual saved Actor storage.
//!
//! Actor+108/+10C is a derived list of build-target ActorData identities,
//! distinct from PropertiesComponent effects and numeric refresh581420.
//! Native list clears, allocations and stores are immediate. Reached missing
//! canonical ActorData+504 completion remains an explicit source dependency.
use crate::catalog::Catalog;
use aa_actors::{ActorId, components::ComponentSlot, loading::ActorShell};
use aa_body::{
    Error, Result,
    actor_property_list::{self, NodeId, State, World},
};
use aa_mapworld::restore::{
    ActorPropertiesContext, RestoredComponent, RestoredRecord, RuntimeDefinition,
};
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "saved_actor_properties_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug)]
enum Node {
    Owned(ActorId, NodeId),
    Source(ActorId, usize),
}

struct Borrowed<'a, 'world, 'source> {
    catalog: &'a Catalog<'source>,
    context: ActorPropertiesContext<'world>,
    parent_visits: BTreeSet<ActorId>,
}

fn error(text: impl std::fmt::Display) -> Error {
    Error::Callback(text.to_string())
}

pub fn refresh(
    catalog: &Catalog<'_>,
    context: ActorPropertiesContext<'_>,
) -> std::result::Result<(), String> {
    let actor = context.owner;
    actor_property_list::refresh(&mut Borrowed::new(catalog, context), actor)
        .map_err(|error| error.to_string())
}

impl Borrowed<'_, '_, '_> {
    fn new<'a, 'world, 'source>(
        catalog: &'a Catalog<'source>,
        context: ActorPropertiesContext<'world>,
    ) -> Borrowed<'a, 'world, 'source> {
        let parent_visits = [context.owner].into_iter().collect();
        Borrowed {
            catalog,
            context,
            parent_visits,
        }
    }
    fn shell(&self, actor: ActorId) -> Result<&ActorShell<RestoredComponent>> {
        if actor == self.context.owner {
            return Ok(self.context.owner_actor);
        }
        self.context
            .registry
            .get(actor)
            .and_then(|entry| match &entry.value {
                RestoredRecord::Actor(entity) => entity.shell.actor.as_ref(),
                _ => None,
            })
            .ok_or_else(|| {
                error(format!(
                    "4D4CF0 actor {:#010x} is not a current restored Actor",
                    actor.raw()
                ))
            })
    }
    fn shell_mut(&mut self, actor: ActorId) -> Result<&mut ActorShell<RestoredComponent>> {
        if actor == self.context.owner {
            return Ok(self.context.owner_actor);
        }
        self.context
            .registry
            .value_mut(actor)
            .and_then(|record| match record {
                RestoredRecord::Actor(entity) => entity.shell.actor.as_mut(),
                _ => None,
            })
            .ok_or_else(|| {
                error(format!(
                    "4D4CF0 actor {:#010x} is not a current restored Actor",
                    actor.raw()
                ))
            })
    }
    fn source(&self, actor: ActorId) -> Result<&RuntimeDefinition> {
        if actor == self.context.owner {
            return Ok(self.context.owner_definition);
        }
        self.context
            .registry
            .get(actor)
            .and_then(|entry| match &entry.value {
                RestoredRecord::Actor(entity) if entity.shell.actor.is_some() => {
                    Some(&entity.definition)
                }
                _ => None,
            })
            .ok_or_else(|| {
                error(format!(
                    "4D4CF0 ActorData owner {:#010x} is unavailable",
                    actor.raw()
                ))
            })
    }
    fn list(&self, actor: ActorId) -> Result<&State<String>> {
        Ok(&self.shell(actor)?.actor_list_108)
    }
    fn list_mut(&mut self, actor: ActorId) -> Result<&mut State<String>> {
        Ok(&mut self.shell_mut(actor)?.actor_list_108)
    }
    fn org(&self, actor: ActorId) -> Result<&aa_actors::ground_states::OrganizationState<String>> {
        if actor == self.context.owner
            && let Some(org) = self.context.organization
        {
            if org.header.owner != actor {
                return Err(error("4D4CF0 active Organization owner mismatch"));
            }
            return Ok(org);
        }
        match self
            .shell(actor)?
            .components
            .get(ComponentSlot::Organization)
        {
            Some(RestoredComponent::Organization(org)) if org.header.owner == actor => Ok(org),
            Some(RestoredComponent::Organization(_)) => {
                Err(error("4D4CF0 Organization belongs to a different Actor"))
            }
            Some(_) => Err(error("4D4CF0 Organization slot payload mismatch")),
            None => Err(error("4D4CF0 required Organization disappeared")),
        }
    }
    fn element_state(
        &self,
        actor: ActorId,
    ) -> Result<&aa_actors::ground_states::ElementState<String>> {
        match self.shell(actor)?.components.get(ComponentSlot::Element) {
            Some(RestoredComponent::Element(element)) if element.header.owner == actor => {
                Ok(element)
            }
            Some(RestoredComponent::Element(_)) => {
                Err(error("4D4E50 Element belongs to a different Actor"))
            }
            Some(_) => Err(error("4D4E50 Element slot payload mismatch")),
            None => Err(error("4D4E50 required Element disappeared")),
        }
    }
    fn local_node(actor: ActorId, node: Option<Node>) -> Result<Option<NodeId>> {
        node.map(|node| match node {
            Node::Owned(owner, node) if owner == actor => Ok(node),
            _ => Err(error(
                "4D4CF0 list link targets a different owner allocation",
            )),
        })
        .transpose()
    }
}

impl World for Borrowed<'_, '_, '_> {
    type Actor = ActorId;
    type Node = Node;
    type Definition = ActorId;
    type Organization = ActorId;
    type Element = ActorId;
    type Value = String;
    fn actor_head(&mut self, actor: ActorId) -> Result<Option<Node>> {
        Ok(self.list(actor)?.head().map(|n| Node::Owned(actor, n)))
    }
    fn actor_tail(&mut self, actor: ActorId) -> Result<Option<Node>> {
        Ok(self.list(actor)?.tail().map(|n| Node::Owned(actor, n)))
    }
    fn store_actor_head(&mut self, actor: ActorId, node: Option<Node>) -> Result<()> {
        let node = Self::local_node(actor, node)?;
        self.list_mut(actor)?.set_head(node);
        Ok(())
    }
    fn store_actor_tail(&mut self, actor: ActorId, node: Option<Node>) -> Result<()> {
        let node = Self::local_node(actor, node)?;
        self.list_mut(actor)?.set_tail(node);
        Ok(())
    }
    fn node_value(&mut self, node: Node) -> Result<String> {
        match node {
            Node::Owned(actor, node) => Ok(self.list(actor)?.value(node)?.clone()),
            Node::Source(actor, index) => {
                let source = self
                    .catalog
                    .native_builder_data(self.context.precision)
                    .map_err(error)?;
                let id = &self.source(actor)?.proof.id;
                source
                    .definition(id)
                    .map_err(error)?
                    .ids
                    .get(index)
                    .cloned()
                    .ok_or_else(|| error("ActorData+504 node value outside actual source list"))
            }
        }
    }
    fn node_next(&mut self, node: Node) -> Result<Option<Node>> {
        match node {
            Node::Owned(actor, node) => {
                Ok(self.list(actor)?.next(node)?.map(|n| Node::Owned(actor, n)))
            }
            Node::Source(actor, index) => {
                let source = self
                    .catalog
                    .native_builder_data(self.context.precision)
                    .map_err(error)?;
                let id = &self.source(actor)?.proof.id;
                let ids = &source.definition(id).map_err(error)?.ids;
                if index >= ids.len() {
                    return Err(error("ActorData+504 node next outside actual source list"));
                }
                Ok((index + 1 < ids.len()).then_some(Node::Source(actor, index + 1)))
            }
        }
    }
    fn store_node_value(&mut self, node: Node, value: String) -> Result<()> {
        match node {
            Node::Owned(actor, node) => self.list_mut(actor)?.set_value(node, value),
            Node::Source(_, _) => Err(error(
                "ActorData+504 source list is immutable during actor refresh",
            )),
        }
    }
    fn store_node_next(&mut self, node: Node, next: Option<Node>) -> Result<()> {
        match node {
            Node::Owned(actor, node) => {
                let next = Self::local_node(actor, next)?;
                self.list_mut(actor)?.set_next(node, next)
            }
            Node::Source(_, _) => Err(error(
                "ActorData+504 source list is immutable during actor refresh",
            )),
        }
    }
    fn allocate_node(&mut self, actor: ActorId) -> Result<Node> {
        Ok(Node::Owned(actor, self.list_mut(actor)?.allocate()?))
    }
    fn free_node(&mut self, actor: ActorId, node: Node) -> Result<()> {
        match node {
            Node::Owned(owner, node) if owner == actor => self.list_mut(actor)?.free(node),
            _ => Err(error("4D4CF0 free targets a different owner allocation")),
        }
    }
    fn has_source_component_a8(&mut self, actor: ActorId) -> Result<bool> {
        match self.shell(actor)?.components.get(ComponentSlot::Builder) {
            None => Ok(false),
            Some(RestoredComponent::Builder(_)) => Ok(true),
            Some(_) => Err(error("4D4CF0 Builder slot payload mismatch")),
        }
    }
    fn definition(&mut self, actor: ActorId) -> Result<ActorId> {
        self.source(actor)?;
        Ok(actor)
    }
    fn definition_flags(&mut self, definition: ActorId) -> Result<u32> {
        let id = self.source(definition)?.proof.id.clone();
        self.catalog
            .canonical_component_mask(&id, self.context.precision)
            .map_err(error)
    }
    fn definition_head_504(&mut self, definition: ActorId) -> Result<Option<Node>> {
        let source = self
            .catalog
            .native_builder_data(self.context.precision)
            .map_err(error)?;
        let id = &self.source(definition)?.proof.id;
        let definition_source = source.definition(id).map_err(error)?;
        Ok((!definition_source.ids.is_empty()).then_some(Node::Source(definition, 0)))
    }
    fn organization(&mut self, actor: ActorId) -> Result<Option<ActorId>> {
        if actor == self.context.owner && self.context.organization.is_some() {
            return Ok(Some(actor));
        }
        match self
            .shell(actor)?
            .components
            .get(ComponentSlot::Organization)
        {
            None => Ok(None),
            Some(RestoredComponent::Organization(_)) => Ok(Some(actor)),
            Some(_) => Err(error("4D4CF0 Organization slot payload mismatch")),
        }
    }
    fn organization_member_count(&mut self, org: ActorId) -> Result<i32> {
        i32::try_from(self.org(org)?.members.len())
            .map_err(|_| error("4D4CF0 member count exceeds native signed domain"))
    }
    fn organization_member(&mut self, org: ActorId, index: i32) -> Result<Option<ActorId>> {
        let index = usize::try_from(index).map_err(|_| error("4D4CF0 negative member index"))?;
        let id = *self
            .org(org)?
            .members
            .get(index)
            .ok_or_else(|| error("4D4CF0 member index outside actual array"))?;
        Ok((!id.is_null()).then_some(id))
    }
    fn element(&mut self, actor: ActorId) -> Result<Option<ActorId>> {
        match self.shell(actor)?.components.get(ComponentSlot::Element) {
            None => Ok(None),
            Some(RestoredComponent::Element(_)) => Ok(Some(actor)),
            Some(_) => Err(error("4D4E50 Element slot payload mismatch")),
        }
    }
    fn element_parent(&mut self, element: ActorId) -> Result<Option<ActorId>> {
        let parent = self.element_state(element)?.organization;
        if let Some(parent) = parent
            && !self.parent_visits.insert(parent)
        {
            return Err(error(
                "4D4E50 saved Element parent cycle (portable invalid-graph refusal)",
            ));
        }
        Ok(parent)
    }
}
