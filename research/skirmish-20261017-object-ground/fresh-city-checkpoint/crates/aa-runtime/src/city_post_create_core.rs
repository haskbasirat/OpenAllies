//!5644F0's captured layout, current-owner reads and ordered creation requests.
use aa_numeric::{Binary,Precision};
pub type Result<T>=std::result::Result<T,String>;
#[derive(Clone,Debug)]
pub(crate) struct Row<D>{pub definition:D,pub offset:[u32;2],pub orientation:[u32;2]}
#[derive(Clone,Debug,PartialEq,Eq)]
pub(crate) struct Request<D>{pub definition:D,pub position:[u32;3],pub direction:[u32;2],pub owner:u32,pub parent:u32}
pub(crate) trait World {
    type Definition:Clone;
    type Layout:Copy;
    type Created:Copy;
    fn clear_values(&mut self)->Result<()>;
    fn select_layout(&mut self,argument:u32)->Result<Self::Layout>;
    fn row_count(&mut self,layout:Self::Layout)->Result<usize>;
    fn row(&mut self,layout:Self::Layout,index:usize)->Result<Row<Self::Definition>>;
    fn position(&mut self)->Result<[u32;2]>;
    fn facing(&mut self)->Result<[u32;2]>;
    fn owner_108(&mut self)->Result<u32>;
    fn current_id(&mut self)->Result<u32>;
    fn is_actor_data(&mut self,definition:&Self::Definition)->Result<bool>;
    fn create(&mut self,request:Request<Self::Definition>,actor:bool)->Result<Option<Self::Created>>;
    fn append_settlement(&mut self,created:Self::Created)->Result<()>;
}
fn b(v:u32)->Result<Binary>{Binary::from_f32_bits(v).map_err(|e|e.to_string())}
fn store(v:aa_numeric::Result<Binary>)->Result<u32>{v.map_err(|e|e.to_string())?.store_f32().map_err(|e|e.to_string())}
fn n<T>(v:aa_numeric::Result<T>)->Result<T>{v.map_err(|e|e.to_string())}
pub(crate) fn position(offset:[u32;2],facing:[u32;2],at:[u32;2],p:Precision)->Result<[u32;2]>{
    let x=n(b(offset[1])?.mul(&b(facing[0])?,p))?;
    let xx=n(b(offset[0])?.mul(&b(facing[1])?,p))?;
    let x=store(x.add(&xx,p))?;
    let y=n(b(offset[1])?.mul(&b(facing[1])?,p))?;
    let yy=n(b(offset[0])?.mul(&b(facing[0])?,p))?;
    let y=store(y.sub(&yy,p))?;
    Ok([store(b(x)?.add(&b(at[0])?,p))?,store(b(y)?.add(&b(at[1])?,p))?])
}
pub(crate) fn direction(o:[u32;2],f:[u32;2],p:Precision)->Result<[u32;2]>{
    let a=n(b(f[1])?.mul(&b(0)?,p))?;
    let ax=n(b(f[0])?.mul(&b(0x3f800000)?,p))?;
    let a=n(a.sub(&ax,p))?;
    let d=n(b(f[1])?.mul(&b(0xbf800000)?,p))?;
    let dx=n(b(f[0])?.mul(&b(0)?,p))?;
    let d=n(d.sub(&dx,p))?;
    let x=n(d.mul(&b(o[0])?,p))?;
    let xx=n(a.mul(&b(o[1])?,p))?;
    let y=n(d.mul(&b(o[1])?,p))?;
    let yy=n(a.mul(&b(o[0])?,p))?;
    Ok([store(x.add(&xx,p))?,store(y.sub(&yy,p))?])
}
pub(crate) fn run<W:World>(w:&mut W,argument:u32,with_values:bool,p:Precision)->Result<()> {
    if with_values {w.clear_values()?;}
    let layout=w.select_layout(argument)?;
    let count=w.row_count(layout)?;
    for index in 0..count {
        let row=w.row(layout,index)?;
        let facing=w.facing()?;
        let at=w.position()?;
        let xy=position(row.offset,facing,at,p)?;
        let owner=w.owner_108()?;
        let parent=w.current_id()?;
        let facing=w.facing()?;
        let direction=direction(row.orientation,facing,p)?;
        let actor=w.is_actor_data(&row.definition)?;
        let made=w.create(Request{definition:row.definition,position:[xy[0],xy[1],0],direction,owner,parent},actor)?;
        //The Object4F9C10 branch discards its return and reloads the zero
        //local; only4FA030's returned Actor can join the City list.
        if with_values&&actor {if let Some(made)=made {w.append_settlement(made)?;}}
    }
    Ok(())
}
#[cfg(test)]
#[path="fresh_city_post_create_core_tests.rs"]
mod tests;
