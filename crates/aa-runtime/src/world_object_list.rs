//! Owned GWorld+8 list, native 4F1F70, used during saved Object Init(NULL).
//!
//! The constructor's empty head is the actual 4F7D44 store. Entries retain
//! portable object identities, not copies of actor state. This list is separate
//! from 4F9510's later publication/classification lists. Native insertion allows
//! duplicates; no gameplay count cap, flag default or success callback is added.
//! Successful allocation and first-match removal are proven; native allocation
//! failure/SEH remains separate. Original/v14 insertion evidence:
//! research/skirmish-20261016-live-map-startup/native-world-object-list.json.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("world-object list allocation failed")]
    Allocation,
    #[error("world-object list portable node identity exhausted")]
    Exhausted,
    #[error("world-object list link references a freed or absent node")]
    InvalidLink,
}
pub type Result<T> = std::result::Result<T, Error>;

/// Allocation identity, independent of GTID, object identity and list order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeId(usize);
impl NodeId {
    pub fn ordinal(self) -> usize {
        self.0
    }
}

/// The native node+4 points to the *link word* that owns this node, rather than
/// to a previous node. Head = world+8; Next(node) = that node's +8 link word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviousLink {
    Head,
    Next(NodeId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Node<T> {
    pub object: T,
    pub previous_link: PreviousLink,
    pub next: Option<NodeId>,
}

/// One mutable owner of world+8 and all its nodes. Not Clone: duplicating a
/// world's runtime lists would create a second mutable authority.
#[derive(Debug)]
pub struct WorldObjectList<T> {
    head: Option<NodeId>,
    nodes: Vec<Node<T>>,
    freed: std::collections::BTreeSet<usize>,
}

impl<T: Copy> WorldObjectList<T> {
    /// 4F7D00 constructor prefix's world+8=null, not an inferred load default.
    pub fn constructed() -> Self {
        Self {
            head: None,
            nodes: Vec::new(),
            freed: std::collections::BTreeSet::new(),
        }
    }

    pub fn head(&self) -> Option<NodeId> {
        self.head
    }

    pub fn node(&self, id: NodeId) -> Option<&Node<T>> {
        if self.freed.contains(&id.0) {return None;}
        id.0.checked_sub(1).and_then(|index| self.nodes.get(index))
    }

    /// Read-only allocation order for checkpoint/ownership inspection. It is
    /// distinct from live head-first traversal, and contains repeated object
    /// IDs and historical freed allocations. Only node(id) exposes live nodes.
    pub fn nodes(&self) -> &[Node<T>] {
        &self.nodes
    }
    ///4F3F10→4BB800. Remove only the first matching live object when actual
    ///Thing+8 bit8 is set; preserve stable identities of every other allocation.
    pub fn remove_first(&mut self,object:T,actual_flags:u32)->Result<bool> where T:PartialEq {
        if actual_flags & (1<<8)==0 {return Ok(false);}
        let mut current=self.head;
        while let Some(id)=current {
            let node=*self.node(id).ok_or(Error::InvalidLink)?;
            if node.object==object {
                match node.previous_link {
                    PreviousLink::Head=>self.head=node.next,
                    PreviousLink::Next(previous)=>{
                        self.node(previous).ok_or(Error::InvalidLink)?;
                        self.nodes[previous.0-1].next=node.next;
                    }
                }
                if let Some(next)=node.next {
                    self.node(next).ok_or(Error::InvalidLink)?;
                    self.nodes[next.0-1].previous_link=node.previous_link;
                }
                self.freed.insert(id.0);
                return Ok(true);
            }
            current=node.next;
        }
        Ok(false)
    }

    /// 4F1F70. The caller must read the actual current object's +8 flags at
    /// this call boundary. Only bit8 gates insertion; no definition/saved/UI
    /// flags may be substituted. The native function performs no GTID lookup.
    pub fn insert(&mut self, object: T, actual_flags: u32) -> Result<Option<NodeId>> {
        if actual_flags & (1 << 8) == 0 {
            return Ok(None);
        }
        let previous_head = self.head;
        let id = NodeId(self.nodes.len().checked_add(1).ok_or(Error::Exhausted)?);
        self.nodes.try_reserve(1).map_err(|_| Error::Allocation)?;
        self.nodes.push(Node {
            object,
            previous_link: PreviousLink::Head,
            next: previous_head,
        });
        if let Some(previous) = previous_head {
            // Only this owner creates links; a foreign/dangling node cannot be
            // installed through the public API.
            self.nodes[previous.0 - 1].previous_link = PreviousLink::Next(id);
        }
        self.head = Some(id);
        Ok(Some(id))
    }
}
