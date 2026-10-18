//! Sole VWorldState+4/+8 selection-list owner from native4ED6D0.
//! Nodes have stable allocation identities; order delivery captures next before
//! an actor callback and never substitutes a cloned selected-actor vector.
use aa_actors::ActorId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeId(usize);
impl NodeId {
    pub fn raw(self) -> usize {
        self.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Node {
    pub actor: ActorId,
    pub next: Option<NodeId>,
}
/// This list moves with its actual player's VWorldState. It has no Clone or
/// Default and cannot be created by passing a viewer's placement list.
#[derive(Debug)]
pub struct List {
    head: Option<NodeId>,
    tail: Option<NodeId>,
    nodes: Vec<Option<Node>>,
}
impl List {
    /// Actual null head/tail stores by4ED6D0. No selected actor is inferred.
    pub fn from_vworld_constructor() -> Self {
        Self {
            head: None,
            tail: None,
            nodes: Vec::new(),
        }
    }
    pub fn head(&self) -> Option<NodeId> {
        self.head
    }
    pub fn tail(&self) -> Option<NodeId> {
        self.tail
    }
    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }
    pub fn node(&self, id: NodeId) -> Result<Node, String> {
        self.nodes
            .get(id.0)
            .and_then(|n| *n)
            .ok_or_else(|| "native selection traversal cursor is no longer owned".into())
    }
    pub(crate) fn find(&self, actor: ActorId) -> Result<Option<NodeId>, String> {
        let mut current = self.head;
        let mut remaining = self.nodes.len();
        while let Some(id) = current {
            if remaining == 0 {
                return Err("native selection list contains a link cycle".into());
            }
            remaining -= 1;
            let node = self.node(id)?;
            if node.actor == actor {
                return Ok(Some(id));
            }
            current = node.next;
        }
        Ok(None)
    }
    /// Successful8-byte allocation/head insertion of4ECFF0. Eligibility,
    /// duplicate guard and actual Object4C byte mutation belong to its caller.
    pub(crate) fn prepend(&mut self, actor: ActorId) -> Result<NodeId, String> {
        self.nodes
            .len()
            .checked_add(1)
            .ok_or("selection allocation identity exhausted")?;
        self.nodes
            .try_reserve(1)
            .map_err(|_| "selection allocation failed")?;
        let id = NodeId(self.nodes.len());
        self.nodes.push(Some(Node {
            actor,
            next: self.head,
        }));
        self.head = Some(id);
        if self.tail.is_none() {
            self.tail = Some(id)
        }
        Ok(id)
    }
    ///4F1430 reached by4ED080: retire only this allocation, never renumber
    /// later nodes. An earlier captured next handle still identifies its node.
    pub(crate) fn remove(&mut self, id: NodeId) -> Result<ActorId, String> {
        let mut current = self.head;
        let mut previous: Option<NodeId> = None;
        let mut remaining = self.nodes.len();
        while let Some(key) = current {
            if remaining == 0 {
                return Err("native selection list contains a link cycle".into());
            }
            remaining -= 1;
            let node = self.node(key)?;
            if key == id {
                if let Some(prior) = previous {
                    self.nodes[prior.0]
                        .as_mut()
                        .ok_or("selection predecessor is absent")?
                        .next = node.next;
                } else {
                    self.head = node.next;
                }
                if self.tail == Some(id) {
                    self.tail = previous;
                }
                self.nodes[id.0] = None;
                return Ok(node.actor);
            }
            previous = Some(key);
            current = node.next;
        }
        Err("selection node is not in this list".into())
    }
    ///4ED257..4ED265: the VWorldState destructor updates head (and the final
    ///tail) before freeing this allocation. It does not call actor deselection
    ///services. Captured node identities are never renumbered or reused.
    pub(crate) fn retire_head_for_view_destructor(&mut self)->Result<(),String> {
        let id=self.head.ok_or("4ED210 selection head is null")?;
        let next=self.node(id)?.next;
        self.head=next;
        if next.is_none() { self.tail=None; }
        self.nodes[id.0]=None;
        Ok(())
    }
}
/// Scoped references into one validated player's actual VWorldState. Application
/// owners implement this on their borrowed view; there is no detached selection
/// or implicit current player. Native admin+284 remains a separate required owner.
pub trait View {
    fn player_id(&self) -> u32;
    fn list(&self) -> &List;
    fn list_mut(&mut self) -> &mut List;
    fn kingdom(&self) -> Option<ActorId>;
    fn mask_words_mut(&mut self) -> &mut [u32; 4];
    fn look_at_mut(&mut self) -> &mut [u32; 2];
    fn group_is_null(&self) -> Result<bool, String>;
    fn clear_group(&mut self);
}
/// Native4ECFF0/4ED080/4ED0D0 data accesses. The concrete live adapter owns
/// every field; native-memory fixture hosts can replay the same coordinator.
/// Required callbacks have no successful defaults.
pub trait World {
    fn application_state(&mut self) -> Result<u32, String>;
    fn is_actor(&mut self, actor: ActorId) -> Result<bool, String>;
    fn element_parent(&mut self, actor: ActorId) -> Result<Option<ActorId>, String>;
    fn change_byte_4c(&mut self, actor: ActorId, increment: bool) -> Result<(), String>;
    fn update_group(&mut self, view: &mut impl View) -> Result<(), String>;
    fn rebuild_masks(&mut self, view: &mut impl View) -> Result<(), String>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Null,
    AlreadySelected,
    AttachedElement,
    Selected,
}
pub fn select(
    world: &mut impl World,
    view: &mut impl View,
    actor: Option<ActorId>,
) -> Result<Outcome, String> {
    let Some(actor) = actor else {
        return Ok(Outcome::Null);
    };
    if view.list().find(actor)?.is_some() {
        return Ok(Outcome::AlreadySelected);
    }
    if world.application_state()? != 3
        && world.is_actor(actor)?
        && world.element_parent(actor)?.is_some()
    {
        return Ok(Outcome::AttachedElement);
    }
    world.change_byte_4c(actor, true)?;
    view.list_mut().prepend(actor)?;
    world.update_group(view)?;
    world.rebuild_masks(view)?;
    Ok(Outcome::Selected)
}
pub fn deselect(
    world: &mut impl World,
    view: &mut impl View,
    actor: ActorId,
) -> Result<bool, String> {
    let Some(node) = view.list().find(actor)? else {
        return Ok(false);
    };
    world.change_byte_4c(actor, false)?;
    view.list_mut().remove(node)?;
    world.update_group(view)?;
    world.rebuild_masks(view)?;
    Ok(true)
}
pub fn clear(world: &mut impl World, view: &mut impl View) -> Result<(), String> {
    let mut current = view.list().head();
    while let Some(id) = current {
        let node = view.list().node(id)?;
        current = node.next;
        world.change_byte_4c(node.actor, false)?;
    }
    while let Some(id) = view.list().head() {
        view.list_mut().remove(id)?;
    }
    world.update_group(view)?;
    view.mask_words_mut().fill(0);
    Ok(())
}
