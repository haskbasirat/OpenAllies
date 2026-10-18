//!44FDB0 tail after the separately retained453F10 Actor constructor.
//!Native pointer identity and unwritten array slots/padding stay opaque.
use anyhow::{Context,Result,ensure};

//No element is present after this constructor. A future positive list writer
//must install its actual resource owner; unused allocation slots are not values.
struct ResourceSlot{_owner:usize}
struct Array{items:Vec<ResourceSlot>,native_capacity:usize,allocated:bool}
struct EmptyLink;

pub(crate) struct Fields{
    array:Option<Array>,head_164:Option<EmptyLink>,block_168:Option<EmptyLink>,
    byte_16c:Option<u8>,word_170:Option<u32>,complete:bool,
}
pub(crate) struct Failure{pub fields:Fields,pub error:anyhow::Error}
impl std::fmt::Debug for Failure{
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{
        f.debug_struct("VOrganizationTailFailure").field("error",&self.error).finish_non_exhaustive()
    }
}
impl Fields{
    ///The native allocation is fourteen pointer slots, not an item ceiling.
    ///Rust owns growable storage. Allocation refusal retains the preceding
    ///158/15C/160 stores; it is not a claim about native OOM/unwinding.
    pub(crate) fn construct()->std::result::Result<Self,Failure>{
        Self::construct_with(|items|items.try_reserve_exact(14).context("portable Organization view allocation failed"))
    }
    fn construct_with(allocate:impl FnOnce(&mut Vec<ResourceSlot>)->Result<()>)->std::result::Result<Self,Failure>{
        let mut fields=Self{array:None,head_164:None,block_168:None,byte_16c:None,word_170:None,complete:false};
        let result=(||{
            fields.array=Some(Array{items:Vec::new(),native_capacity:14,allocated:false});
            let array=fields.array.as_mut().unwrap();
            allocate(&mut array.items)?;
            array.allocated=true;
            fields.head_164=Some(EmptyLink);fields.block_168=Some(EmptyLink);
            fields.byte_16c=Some(0);fields.word_170=Some(0);fields.complete=true;
            Ok(())
        })();
        match result{Ok(())=>Ok(fields),Err(error)=>Err(Failure{fields,error})}
    }
    pub(crate) fn require_constructed(&self)->Result<()>{ensure!(self.complete,"actual44FDB0 scalar tail is incomplete");Ok(())}
    pub(crate) fn word(&self,offset:usize)->Result<Option<u32>>{
        ensure!((0x158..=0x170).contains(&offset)&&offset%4==0,"Organization scalar outside tail");
        Ok(match offset{
            0x158=>self.array.as_ref().and_then(|a|(!a.allocated).then_some(0)),
            0x15c=>self.array.as_ref().and_then(|a|u32::try_from(a.items.len()).ok()),
            0x160=>self.array.as_ref().and_then(|a|u32::try_from(a.native_capacity).ok()),
            0x164=>self.head_164.as_ref().map(|_|0),0x168=>self.block_168.as_ref().map(|_|0),
            //Only16C is written;16D..16F have no constructor value.
            0x16c=>None,0x170=>self.word_170,_=>unreachable!(),
        })
    }
    pub(crate) fn byte_16c(&self)->Option<u8>{self.byte_16c}
}

#[cfg(test)]#[path="view_organization_fields_tests.rs"]mod tests;
