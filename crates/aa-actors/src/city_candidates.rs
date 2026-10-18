//! City-owned native +18/+1C list. 564160 initializes both links to null;
//! 564300/4D0900 prepends generation-bearing actor references. This owner is
//! separate from the saved +10/+14 list and has no fixed native pool limit.
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeId(usize);
impl NodeId {
    pub fn raw(self) -> usize { self.0 }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Node { pub reference:u32, pub next:Option<NodeId> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    head:Option<NodeId>, tail:Option<NodeId>, next_id:usize,
    nodes:BTreeMap<NodeId,Node>,
}
impl State {
    /// The actual City564160 constructor, not an inferred empty loaded list.
    pub fn constructed()->Self {
        Self{head:None,tail:None,next_id:1,nodes:BTreeMap::new()}
    }
    pub fn head(&self)->Option<NodeId>{self.head}
    pub fn tail(&self)->Option<NodeId>{self.tail}
    pub fn allocated_nodes(&self)->usize{self.nodes.len()}
    pub fn node(&self,id:NodeId)->Result<&Node,String>{
        self.nodes.get(&id).ok_or_else(||"City candidate node is not owned".into())
    }
    pub fn contains(&self,reference:u32)->Result<bool,String>{
        let mut node=self.head;
        let mut remaining=self.nodes.len();
        while let Some(id)=node {
            if remaining==0{return Err("City candidate list is cyclic".into())}
            remaining-=1;
            let current=self.node(id)?;
            if current.reference==reference{return Ok(true)}
            node=current.next;
        }
        Ok(false)
    }
    /// Successful 6F7810(8)/4D0900 allocation, then caller stores +18 and
    /// initializes +1C only if it was null. No callback runs inside these stores.
    pub fn prepend(&mut self,reference:u32)->Result<NodeId,String>{
        let id=NodeId(self.next_id);
        let next=self.next_id.checked_add(1)
            .ok_or_else(||"City candidate allocation identity exhausted".to_owned())?;
        self.nodes.insert(id,Node{reference,next:self.head});
        self.next_id=next;
        self.head=Some(id);
        if self.tail.is_none(){self.tail=Some(id)}
        Ok(id)
    }
    /// Native head unlink/free shape used by the City destructor5641C0.
    /// Future tick/removal ports must consume this same owner.
    pub fn pop_front(&mut self)->Result<Option<u32>,String>{
        let Some(id)=self.head else{return Ok(None)};
        let node=*self.node(id)?;
        self.head=node.next;
        if self.head.is_none(){self.tail=None}
        self.nodes.remove(&id).ok_or_else(||"City candidate free identity missing".to_owned())?;
        Ok(Some(node.reference))
    }
}
