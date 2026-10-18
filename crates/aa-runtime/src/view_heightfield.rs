//! Native431A90/431BC0/431550 visual heightfield owned by VWorld+BC.
//! This is distinct from the simulation's original550210 heightmap. Every
//! sample is taken through that SAME current World's4F0E50 surface at0x122.
use aa_numeric::{Binary,Precision};
use std::cmp::Ordering;
type Result<T>=std::result::Result<T,String>;
fn b(bits:u32)->Result<Binary>{Binary::from_f32_bits(bits).map_err(|e|e.to_string())}
fn n<T>(result:std::result::Result<T,aa_numeric::Error>)->Result<T>{result.map_err(|e|e.to_string())}

struct Fields{
    heights:Vec<u32>,step:u32,inverse:u32,columns:Option<i32>,rows:Option<i32>,limits:[u32;2],
    //431910(10,1) constructs ten16-byte records via79E130; only their
    //first two DWORDs are written. This is initial capacity, never a cap.
    auxiliary:Vec<[Option<u32>;4]>,
    auxiliary_count:usize,
}
impl Fields{
    fn constructed()->Self{Self{heights:Vec::new(),step:0,inverse:0,columns:None,rows:None,limits:[0;2],
        auxiliary:(0..10).map(|_|[Some(0),Some(0),None,None]).collect(),auxiliary_count:0}}
}
trait Source{
    fn quad_size(&mut self)->Result<i32>;
    fn base_dimensions(&self)->[i32;2];
    fn base_cell_size(&self)->u32;
    fn base_limits(&self)->[u32;2];
    fn sample(&mut self,position:[u32;2],flags:u32,pc:Precision)->Result<Binary>;
}
fn initialize(f:&mut Fields,source:&mut impl Source,pc:Precision)->Result<()>{
    let quad=source.quad_size()?;
    let exact=Binary::from_i32(quad);
    f.step=n(exact.store_f32())?;
    let inverse=n(Binary::from_i32(1).div(&exact,pc))?;
    f.inverse=n(inverse.store_f32())?;
    //431BEA is FST, not FSTP: dimensions use the unspilled inverse.
    let factor=n(inverse.mul(&b(source.base_cell_size())?,pc))?;
    let base=source.base_dimensions();
    let dimension=|v:i32|->Result<i32>{
        n(n(Binary::from_i32(v.wrapping_sub(1)).mul(&factor,pc))?.truncate_i32())?
            .checked_add(1).ok_or_else(||"visual heightfield dimension exceeds native integer representation".into())
    };
    f.columns=Some(dimension(base[0])?);
    f.rows=Some(dimension(base[1])?);
    f.limits=source.base_limits();
    let columns=f.columns.unwrap();let rows=f.rows.unwrap();
    if columns<0||rows<0{return Err("visual heightfield negative native dimensions require allocation failure handling".into())}
    let count=(columns as usize).checked_mul(rows as usize).ok_or("visual heightfield allocation count overflow")?;
    f.heights.try_reserve(count).map_err(|_|"visual heightfield allocation failed")?;
    let step=b(f.step)?;
    let mut y=0;
    for _ in 0..rows{
        let mut x=0;
        for _ in 0..columns{
            let height=source.sample([x,y],0x122,pc)?;
            f.heights.push(n(height.store_f32())?);
            x=n(n(b(x)?.add(&step,pc))?.store_f32())?;
        }
        y=n(n(b(y)?.add(&step,pc))?.store_f32())?;
    }
    Ok(())
}
fn sample(f:&Fields,position:[u32;2],pc:Precision)->Result<Binary>{
    let columns=f.columns.ok_or("visual heightfield columns are unwritten")?;
    let rows=f.rows.ok_or("visual heightfield rows are unwritten")?;
    if columns<2||rows<2{return Err("431550 requires both sampled dimensions at least two".into())}
    if f.heights.len()!=(columns as usize).checked_mul(rows as usize).ok_or("visual heightfield dimensions overflow")?{
        return Err("431550 visual sample storage is incomplete".into())
    }
    let zero=Binary::from_i32(0);let one=Binary::from_i32(1);
    let step=b(f.step)?;let inverse=b(f.inverse)?;
    let max_x=b(n(n(Binary::from_i32(columns-1).mul(&step,pc))?.store_f32())?)?;
    let mut x=b(position[0])?;
    if x.compare(&zero)==Ordering::Less{x=zero.clone()}
    else if x.compare(&max_x)==Ordering::Greater{x=max_x;}
    let max_y=n(Binary::from_i32(rows-1).mul(&step,pc))?;
    let mut y=position[1];let original_y=b(y)?;
    if original_y.compare(&zero)==Ordering::Less{y=0}
    else if original_y.compare(&max_y)==Ordering::Greater{y=n(max_y.store_f32())?;}
    let scaled_x=n(x.mul(&inverse,pc))?;
    let scaled_y=n(b(y)?.mul(&inverse,pc))?;
    let ix=n(scaled_x.truncate_i32())?.clamp(0,columns-2);
    let iy=n(scaled_y.truncate_i32())?.clamp(0,rows-2);
    let tx=n(scaled_x.sub(&Binary::from_i32(ix),pc))?;
    let ty=n(scaled_y.sub(&Binary::from_i32(iy),pc))?;
    let at=|x:i32,y:i32|->Result<Binary>{b(f.heights[(y as usize)*(columns as usize)+x as usize])};
    let minus_x=n(one.sub(&tx,pc))?;
    let lower=n(n(minus_x.mul(&at(ix,iy)?,pc))?.add(&n(tx.mul(&at(ix+1,iy)?,pc))?,pc))?;
    let upper=n(n(minus_x.mul(&at(ix,iy+1)?,pc))?.add(&n(tx.mul(&at(ix+1,iy+1)?,pc))?,pc))?;
    n(n(upper.mul(&ty,pc))?.add(&n(n(one.sub(&ty,pc))?.mul(&lower,pc))?,pc))
}

///One retained visual allocation. Initialization failure preserves the
///samples/scalars already written and cannot be retried as fresh success.
pub struct State{fields:Fields,world:Option<crate::world_revision::Witness>,attempted:bool,complete:bool,identity:std::sync::Arc<()>}
///Nonowning correlation with the actual completed visual sample allocation.
///A World revision by itself does not identify one particular heightfield.
pub struct SampleWitness(std::sync::Weak<()>);
impl SampleWitness{
    pub fn is_for(&self,height:&State,world:&crate::world_revision::ConstructedWorldRevision)->bool{
        height.complete&&height.world.as_ref().is_some_and(|w|w.is_for(world))
            &&self.0.upgrade().is_some_and(|id|std::sync::Arc::ptr_eq(&id,&height.identity))
    }
}
///Read-only rendering snapshot source. Values are the same completed431BC0
///sample storage; this loan neither resamples nor advances the simulation.
pub struct Samples<'a>{pub step_bits:u32,pub columns:usize,pub rows:usize,pub heights:&'a[u32]}
impl State{
    pub fn from_constructor()->Self{Self{fields:Fields::constructed(),world:None,attempted:false,complete:false,identity:std::sync::Arc::new(())}}
    pub fn initialize_for_world(&mut self,world:&crate::live_world_frame::World<'_>,quad:impl FnOnce()->Result<i32>,pc:Precision)->Result<()>{
        if self.attempted{return Err("visual heightfield initialization requires the same fresh allocation".into())}
        self.attempted=true;
        let parts=world.presentation_parts()?;
        self.world=Some(parts.revision.witness());
        let mut source=Current{surface:parts.terrain.surface().map_err(|e|e.to_string())?,base:&parts.terrain.base.state,quad:Some(quad)};
        initialize(&mut self.fields,&mut source,pc)?;
        self.complete=true;Ok(())
    }
    pub fn sample_for_world(&self,world:&crate::world_revision::ConstructedWorldRevision,position:[u32;2],pc:Precision)->Result<Binary>{
        if !self.complete||!self.world.as_ref().is_some_and(|w|w.is_for(world)){
            return Err("visual height query requires the same completely sampled World allocation".into())
        }
        sample(&self.fields,position,pc)
    }
    pub fn samples_for_world(&self,world:&crate::world_revision::ConstructedWorldRevision)->Result<Samples<'_>>{
        if !self.complete||!self.world.as_ref().is_some_and(|w|w.is_for(world)){
            return Err("visual sample loan requires the same completely sampled World allocation".into())
        }
        let columns=usize::try_from(self.fields.columns.ok_or("visual columns are unwritten")?).map_err(|_|"negative visual columns")?;
        let rows=usize::try_from(self.fields.rows.ok_or("visual rows are unwritten")?).map_err(|_|"negative visual rows")?;
        if columns.checked_mul(rows)!=Some(self.fields.heights.len()){
            return Err("visual sample loan storage is incomplete".into())
        }
        Ok(Samples{step_bits:self.fields.step,columns,rows,heights:&self.fields.heights})
    }
    pub fn witness_for_world(&self,world:&crate::world_revision::ConstructedWorldRevision)->Result<SampleWitness>{
        self.samples_for_world(world)?;
        Ok(SampleWitness(std::sync::Arc::downgrade(&self.identity)))
    }
}
struct Current<'a,F>{surface:aa_spatial::index_world::GroundSurface<'a>,base:&'a aa_heightfield::binding::InitializedHeightfield,quad:Option<F>}
impl<F:FnOnce()->Result<i32>> Source for Current<'_,F>{
    fn quad_size(&mut self)->Result<i32>{self.quad.take().ok_or("visual heightfield UVar getter reused")?()}
    fn base_dimensions(&self)->[i32;2]{[self.base.field.width,self.base.field.height]}
    fn base_cell_size(&self)->u32{self.base.field.cell_size_bits}
    fn base_limits(&self)->[u32;2]{[self.base.minimum_bits,self.base.maximum_bits]}
    fn sample(&mut self,position:[u32;2],flags:u32,pc:Precision)->Result<Binary>{self.surface.sample(position,flags,pc).map_err(|e|e.to_string())}
}
#[cfg(test)]
#[path="view_heightfield_tests.rs"]
mod tests;
