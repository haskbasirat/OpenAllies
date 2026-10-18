//! Empty-descriptor `552EE0` -> `558040(mode0)` -> `558580` actor scan.
//! Mode0 visits the circle's tile RECTANGLE; it does not test the circle
//! against actors or tile squares. This distinction matters to4FBF40.
use crate::{Error,Result,collision::{self,Descriptor,Shape},geometry::{Placement,tile_rectangle},
    index::{ActorKey,NodeKey,SpatialIndex},index_world::CollisionView,query::{self,QueryGeometry,QueryPolicy}};

///Native stack iterator state. The actual index must remain at the same
///owner address until finish; this pointer is compared only, never dereferenced
///or serialized. The cursor cannot be sent to another thread or cloned.
#[must_use="finish the native rectangle iterator on every exit"]
pub struct Cursor{
    owner:*const SpatialIndex,
    rect:crate::geometry::TileRect,
    cell:[i32;2],
    node:Option<NodeKey>,
    head:Option<ActorKey>,
    finished:bool,
    descriptor:Descriptor,
}
impl Cursor{
    pub fn begin(index:&mut SpatialIndex,geometry:&QueryGeometry,position:[u32;2],radius:u32,source_facing:[u32;2])->Result<Self>{
        let placement=Placement{shape:Shape{kind:1,radius_bits:radius,bounds:[0;4]},facing:source_facing,position};
        let _boundary=query::outside(&placement,geometry.extent_bits,index.precision())?;
        let rect=tile_rectangle(index.grid(),&placement,index.precision())?;
        let cell=rect.min;
        let node=index.tile(cell)?.first_node;
        Ok(Self{owner:index as *const _,rect,cell,node,head:None,finished:false,descriptor:Descriptor::default()})
    }
    fn validate(&self,index:&SpatialIndex)->Result<()>{
        if !std::ptr::eq(self.owner,index){return Err(Error::World("native rectangle iterator index owner moved or differs".into()))}Ok(())
    }
    ///558580 yields one current Actor. Later callbacks run before the next
    ///node is filtered, preserving native short circuits and error order.
    pub fn next<P:QueryPolicy>(&mut self,index:&mut SpatialIndex,policy:&mut P)->Result<Option<u32>>{
        self.validate(index)?;
        if self.finished{return Ok(None)}
        loop{
            while let Some(node)=self.node{
                let value=index.node(node)?.clone();self.node=value.next;let actor=value.actor;
                if query::mark(index,&mut self.head,actor)?{continue}
                if collision::excluded(actor,&mut self.descriptor,&mut CollisionView{index,policy})
                    .map_err(|e|Error::World(e.to_string()))?{continue}
                let reference=index.member(actor)?.reference;
                if index.resolve_reference(reference)!=Some(actor){return Err(Error::World("558580 stale query result would dereference null before B4".into()))}
                if !index.member(actor)?.actor_category{continue}
                return match index.resolve_reference(reference){
                    Some(current)if current==actor=>Ok(Some(reference)),
                    _=>Err(Error::World("558580 current result changed after ordinary B4".into())),
                }
            }
            self.cell[0]+=1;
            if self.cell[0]>self.rect.max[0]{self.cell[0]=self.rect.min[0];self.cell[1]+=1;}
            if self.cell[1]>self.rect.max[1]{self.finish(index)?;return Ok(None)}
            self.node=index.tile(self.cell)?.first_node;
        }
    }
    ///Native virtual8/end cleanup and557FD0's later idempotent cleanup.
    pub fn finish(&mut self,index:&mut SpatialIndex)->Result<()>{
        self.validate(index)?;query::clear(index,&mut self.head)?;self.finished=true;Ok(())
    }
}

/// Closed ordinary empty-descriptor domain. Uses the same intrusive query
/// marks and current tile lists, with cleanup on every success/failure exit.
pub fn collect<P:QueryPolicy>(index:&mut SpatialIndex,geometry:&QueryGeometry,
    position:[u32;2],radius:u32,source_facing:[u32;2],policy:&mut P)->Result<Vec<u32>>{
    let mut cursor=Cursor::begin(index,geometry,position,radius,source_facing)?;
    let result=(||{
        let mut found=Vec::new();while let Some(reference)=cursor.next(index,policy)?{found.push(reference);}
        Ok(found)
    })();
    let cleanup=cursor.finish(index);
    match(result,cleanup){(Ok(value),Ok(()))=>Ok(value),(Err(e),Ok(()))|(Ok(_),Err(e))=>Err(e),
        (Err(e),Err(c))=>Err(Error::World(format!("{e}; rectangle query cleanup failed: {c}")))}
}

#[cfg(test)]
#[path="rectangle_actors_tests.rs"]
mod tests;
