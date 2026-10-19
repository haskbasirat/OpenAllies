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
///The World ground the visual samples come from (4F0E50 on GWorld).
trait Ground{
    fn sample(&mut self,position:[u32;2],flags:u32,pc:Precision)->Result<Binary>;
}
///431E10 on the +28/+2C/+30 rect array: count+1 first (4319A0 grows the
///array when the count passes its capacity; capacity is allocation policy,
///never a limit), then the four words at index count-1: origin x, origin y,
///width, height. The only native caller is TerrainBlock 71CC00 (block
///heights changed), which passes the block origin and block size twice.
fn append_rect_431e10(f:&mut Fields,origin:[u32;2],width:u32,height:u32)->Result<()>{
    let count=f.auxiliary_count.checked_add(1).ok_or("431E10 rect count overflow")?;
    if count>f.auxiliary.len(){
        f.auxiliary.try_reserve(1).map_err(|_|"431E10 rect array growth failed")?;
        f.auxiliary.push([None;4]);
    }
    f.auxiliary_count=count;
    f.auxiliary[count-1]=[Some(origin[0]),Some(origin[1]),Some(width),Some(height)];
    Ok(())
}
///4318E0: 431390 for each of the +2C rects in index order (the count is read
///once), then +2C = 0. A refused rect keeps the count; the samples already
///stored stay (native has no failure edge).
fn drain_rects_4318e0(f:&mut Fields,ground:&mut impl Ground,pc:Precision)->Result<usize>{
    let count=f.auxiliary_count;
    for index in 0..count{
        let [Some(x),Some(y),Some(w),Some(h)]=f.auxiliary[index] else{
            return Err("4318E0 rect record below the count is unwritten".into())
        };
        resample_431390(f,[x,y,w,h],ground,pc)?;
    }
    f.auxiliary_count=0;
    Ok(count)
}
///CRT floor 96D5E0 / ceil 96D4C0 of the binary64 argument then 96CC4C. Both
///CRT paths (SSE2 at 96D620, x87 at 977575) are exact; the oracle runs both.
///A result outside signed 32 bits is refused (native would keep the low word
///of the 64-bit conversion); world rects never reach that domain.
fn integral(value:&Binary,ceiling:bool)->Result<i32>{
    let truncated=n(value.truncate_i32()).map_err(|e|format!("431390 rect coordinate: {e}"))?;
    let back=Binary::from_i32(truncated);
    match value.compare(&back){
        Ordering::Equal=>Ok(truncated),
        Ordering::Less if !ceiling=>truncated.checked_sub(1).ok_or_else(||"431390 floor outside signed 32 bits".into()),
        Ordering::Greater if ceiling=>truncated.checked_add(1).ok_or_else(||"431390 ceil outside signed 32 bits".into()),
        _=>Ok(truncated),
    }
}
///`fstp qword` of an x87 value: round the significand to binary64.
fn binary64(value:Binary)->Result<Binary>{n(value.mul(&Binary::from_i32(1),Precision::Double53))}
///431390(rect): resample the visual heights covered by one world rect.
///x0/y0 = floor(origin*inverse), x1/y1 = ceil((size+origin)*inverse), each
///product stored binary64 before the CRT call, clamped to 0..columns-1 /
///0..rows-1. Rows y0..y1, columns x0..x1: the row start is binary32(y*step),
///each row restarts at binary32(binary32(x0)*step) and advances by a binary32
///+step store; the sample is 4F0E50(x,y,0x122) stored binary32 into
///heights[columns*y+x], then +20 = h unless +20 < h and +24 = h unless
///+24 > h (an unordered or equal compare stores h).
fn resample_431390(f:&mut Fields,rect:[u32;4],ground:&mut impl Ground,pc:Precision)->Result<()>{
    let columns=f.columns.ok_or("431390 visual columns are unwritten")?;
    let rows=f.rows.ok_or("431390 visual rows are unwritten")?;
    if columns<1||rows<1{return Err("431390 requires a sampled heightfield of at least one cell".into())}
    if f.heights.len()!=(columns as usize).checked_mul(rows as usize).ok_or("visual heightfield dimensions overflow")?{
        return Err("431390 visual sample storage is incomplete".into())
    }
    let inverse=b(f.inverse)?;let step=b(f.step)?;
    let [x,y,w,h]=rect;
    let x0=integral(&binary64(n(b(x)?.mul(&inverse,pc))?)?,false)?;
    let y0=integral(&binary64(n(b(y)?.mul(&inverse,pc))?)?,false)?;
    let x1=integral(&binary64(n(n(b(w)?.add(&b(x)?,pc))?.mul(&inverse,pc))?)?,true)?;
    let y1=integral(&binary64(n(n(b(h)?.add(&b(y)?,pc))?.mul(&inverse,pc))?)?,true)?;
    let clamp=|v:i32,last:i32|if v<0{0}else if v>last{last}else{v};
    let (x0,x1)=(clamp(x0,columns-1),clamp(x1,columns-1));
    let (y0,y1)=(clamp(y0,rows-1),clamp(y1,rows-1));
    let mut row_y=n(n(Binary::from_i32(y0).mul(&step,pc))?.store_f32())?;
    if y0>y1{return Ok(())}
    let x0_word=n(Binary::from_i32(x0).store_f32())?;
    let mut cell_y=y0;
    loop{
        let mut cell_x=x0;
        let mut row_x=n(n(b(x0_word)?.mul(&step,pc))?.store_f32())?;
        if cell_x<=x1{
            loop{
                let height=n(ground.sample([row_x,row_y],0x122,pc)?.store_f32())?;
                let index=(columns as usize)*(cell_y as usize)+cell_x as usize;
                f.heights[index]=height;
                let sample=f32::from_bits(height);
                if !(f32::from_bits(f.limits[0])<sample){f.limits[0]=height;}
                if !(f32::from_bits(f.limits[1])>sample){f.limits[1]=height;}
                cell_x+=1;
                row_x=n(n(b(row_x)?.add(&step,pc))?.store_f32())?;
                if cell_x>x1{break}
            }
        }
        cell_y+=1;
        row_y=n(n(b(row_y)?.add(&step,pc))?.store_f32())?;
        if cell_y>y1{break}
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
    ///431E10 on this SAME completed allocation (TerrainBlock 71CC00 is its
    ///only native caller; that port must call this with the block origin and
    ///the TerrainMap+A0 block size as width and height).
    pub fn append_rect_431e10(&mut self,world:&crate::world_revision::ConstructedWorldRevision,origin:[u32;2],width:u32,height:u32)->Result<()>{
        if !self.complete||!self.world.as_ref().is_some_and(|w|w.is_for(world)){
            return Err("431E10 requires the same completely sampled World allocation".into())
        }
        append_rect_431e10(&mut self.fields,origin,width,height)
    }
    ///+2C, the rects 4318E0 will resample.
    pub fn pending_rects(&self)->usize{self.fields.auxiliary_count}
    ///+20/+24 sample limits (431BC0 copies the base heightfield limits;
    ///431390 widens them).
    pub fn limit_words(&self)->[u32;2]{self.fields.limits}
    ///4318E0 at 43ECFF (ecx = VWorld+BC) over the SAME World's ground
    ///surface. Returns the number of resampled rects.
    pub fn update_rects_4318e0(&mut self,world:&crate::live_world_frame::World<'_>,pc:Precision)->Result<usize>{
        let parts=world.presentation_parts()?;
        if !self.complete||!self.world.as_ref().is_some_and(|w|w.is_for(parts.revision)){
            return Err("4318E0 requires the same completely sampled World allocation".into())
        }
        if self.fields.auxiliary_count==0{return Ok(0)}
        let mut ground=WorldGround(parts.terrain.surface().map_err(|e|e.to_string())?);
        drain_rects_4318e0(&mut self.fields,&mut ground,pc)
    }
}
struct WorldGround<'a>(aa_spatial::index_world::GroundSurface<'a>);
impl Ground for WorldGround<'_>{
    fn sample(&mut self,position:[u32;2],flags:u32,pc:Precision)->Result<Binary>{self.0.sample(position,flags,pc).map_err(|e|e.to_string())}
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
