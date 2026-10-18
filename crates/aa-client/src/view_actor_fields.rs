//!453F10 tail after the separately retained455050 base. This owns onlyBC+
//!scalar fields and the two actual growable lists, not base/Thing/Data/Init.
use anyhow::{Result,Context,ensure};
use aa_numeric::Binary;

pub(crate) trait Sources{
    fn global_word(&mut self,address:u32)->Result<u32>;
    fn actor_data_38(&mut self)->Result<bool>;
    fn turret_count_1fc(&mut self)->Result<i32>;
    fn turret_pitch_57dbe0(&mut self,index:usize)->Result<u32>;
    fn secondary_count_244(&mut self)->Result<i32>;
    fn source_word_23c(&mut self)->Result<u32>;
}
pub(crate) struct Turret{words:[u32;2],enabled:u8,pitch:u32}
pub(crate) struct Secondary{words:[u32;2],enabled:u8}
impl Turret{
    pub(crate) fn word(&self,offset:usize)->Result<Option<u32>>{Ok(match offset{0|4=>Some(self.words[offset/4]),12=>Some(self.pitch),8=>None,_=>anyhow::bail!("turret constructor scalar outside10 bytes")})}
    pub(crate) fn enabled(&self)->u8{self.enabled}
}
impl Secondary{
    pub(crate) fn word(&self,offset:usize)->Result<Option<u32>>{Ok(match offset{0|4=>Some(self.words[offset/4]),8=>None,_=>anyhow::bail!("secondary constructor scalar outside0C bytes")})}
    pub(crate) fn enabled(&self)->u8{self.enabled}
}
struct Array<T>{items:Vec<T>,native_capacity:usize,allocated:bool}
impl<T> Array<T>{
    fn new(capacity:usize)->Self{Self{items:Vec::new(),native_capacity:capacity,allocated:false}}
    ///766DE0/4849F0. Initial2/3 and doubling are growable allocation behavior,
    ///never artificial item ceilings. Unused native pointer slots stay opaque.
    fn append(&mut self,item:T)->Result<()>{
        let next=self.items.len().checked_add(1).context("portable view list length overflow")?;
        let capacity=if next>self.native_capacity{
            self.native_capacity.checked_mul(2).context("portable view list capacity overflow")?.max(next)
        }else{self.native_capacity};
        if !self.allocated||capacity!=self.native_capacity{
            self.items.try_reserve_exact(capacity.saturating_sub(self.items.len())).context("portable view list allocation failed")?;
            self.native_capacity=capacity;self.allocated=true;
        }
        self.items.push(item);Ok(())
    }
}
///Same tail allocation retained after every required source error. Unknown
///native fields/padding have no default value and cannot be read as zero.
pub(crate) struct Fields{
    words:[Option<u32>;39],turrets:Option<Array<Turret>>,secondary:Option<Array<Secondary>>,complete:bool,
    ///Allocated Track storage remains private and unconstructed if42D7B0
    ///refuses. Native144 is not published until every constructor returns.
    pending_tracks:Option<Vec<std::mem::MaybeUninit<[u32;29]>>>,
}
pub(crate) struct Failure{pub fields:Fields,pub error:anyhow::Error}
impl std::fmt::Debug for Failure{
 fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.debug_struct("VActorTailFailure").field("error",&self.error).finish_non_exhaustive()}
}
impl Fields{
    pub(crate) fn construct(s:&mut impl Sources)->std::result::Result<Self,Failure>{
        let mut fields=Self{words:[None;39],turrets:None,secondary:None,complete:false,pending_tracks:None};
        match fields.construct_inner(s){Ok(())=>Ok(fields),Err(error)=>Err(Failure{fields,error})}
    }
    fn store(&mut self,offset:usize,word:u32){self.words[(offset-0xbc)/4]=Some(word)}
    fn construct_inner(&mut self,s:&mut impl Sources)->Result<()>{
        for base in [0xbc,0xc8]{for(i,address)in [0xb322a4,0xb322a8,0xb322ac].into_iter().enumerate(){let word=s.global_word(address)?;self.store(base+4*i,word)}}
        for(i,address)in [0xb43cf8,0xb43cfc,0xb43d00].into_iter().enumerate(){let word=s.global_word(address)?;self.store(0xf0+4*i,word)}
        for offset in (0xfc..=0x124).step_by(4){self.store(offset,0)}
        self.turrets=Some(Array::new(2));self.secondary=Some(Array::new(3));
        for offset in [0x144,0x148,0x150,0x154]{self.store(offset,0)}
        if s.actor_data_38()?{
            let mut index=0usize;let mut count=s.turret_count_1fc()?;
            while (index as i64)<i64::from(count){
                self.turrets.as_mut().unwrap().append(Turret{words:[0;2],enabled:0,pitch:0})?;
                //Allocation+append precede the real57DBE0/45DC90 load. The
                //finite source binary32 is spilled unchanged, including-0.
                let pitch=Binary::from_f32_bits(s.turret_pitch_57dbe0(index)?)?.store_f32()?;
                self.turrets.as_mut().unwrap().items[index].pitch=pitch;
                index+=1;count=s.turret_count_1fc()?;
            }
            let mut index=0usize;let mut count=s.secondary_count_244()?;
            while (index as i64)<i64::from(count){
                self.secondary.as_mut().unwrap().append(Secondary{words:[0;2],enabled:0})?;
                index+=1;count=s.secondary_count_244()?;
            }
            let word=s.source_word_23c()?;self.store(0x140,word);
        }
        self.store(0x14c,0);self.complete=true;Ok(())
    }
    pub(crate) fn require_constructed(&self)->Result<()>{ensure!(self.complete,"actual453F10 scalar tail is incomplete");Ok(())}
    ///Only the scalar stores reached by451F90/451F00. Constructor lists and
    ///their opaque pointers remain separate owned fields.
    pub(crate) fn store_init_word(&mut self,offset:usize,word:u32)->Result<()>{
        self.require_constructed()?;
        ensure!(matches!(offset,0xc8|0xcc|0xd0|0xd4|0xd8|0xdc|0xe0|0xe4|0xe8|0xec|0xf0|0xf4|0xf8|0xfc|0x118|0x11c|0x120|0x124|0x148|0x14c),"VActor Init scalar outside proven451F90 stores");
        self.store(offset,word);Ok(())
    }
    pub(crate) fn construct_pending_tracks(&mut self,count:i32)->Result<std::convert::Infallible>{
        self.require_constructed()?;
        ensure!(count>0&&self.word(0x148)?==Some(count as u32),"451F90 Track allocation count store differs");
        ensure!(self.pending_tracks.is_none(),"451F90 Track constructor prefix already retained");
        let count=count as usize;count.checked_mul(0x74).and_then(|v|v.checked_add(4)).context("portable Track array address overflow")?;
        let mut storage=Vec::new();storage.try_reserve_exact(count).context("portable Track array allocation failed")?;
        storage.resize_with(count,std::mem::MaybeUninit::uninit);self.pending_tracks=Some(storage);
        anyhow::bail!("42D7B0 actual VActor Track constructor required after retained allocation and148 count")
    }
    pub(crate) fn word(&self,offset:usize)->Result<Option<u32>>{
        ensure!((0xbc..=0x154).contains(&offset)&&offset%4==0,"VActor scalar outside tail");
        for(start,array)in [(0x128,self.turrets.as_ref().map(|a|(a.items.len(),a.native_capacity,a.allocated))),(0x134,self.secondary.as_ref().map(|a|(a.items.len(),a.native_capacity,a.allocated)))]{
            if (start..=start+8).contains(&offset){
                return Ok(array.and_then(|(len,cap,allocated)|match offset-start{0=>(!allocated).then_some(0),4=>u32::try_from(len).ok(),8=>u32::try_from(cap).ok(),_=>None}));
            }
        }
        Ok(self.words[(offset-0xbc)/4])
    }
    pub(crate) fn turrets(&self)->Option<&[Turret]>{self.turrets.as_ref().map(|a|a.items.as_slice())}
    pub(crate) fn secondary(&self)->Option<&[Secondary]>{self.secondary.as_ref().map(|a|a.items.as_slice())}
}

#[cfg(test)]#[path="view_actor_fields_tests.rs"]mod tests;
