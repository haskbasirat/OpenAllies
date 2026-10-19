//!ActorData+534: the MobileDeployment forward unpack target that Actor
//!request45DD80 forwards to. It is NOT a Settlement/HQ payload: packable
//!buildings (rus_corps, ger_corps, depots, airfields, ...) are the pack side
//!and keep a null+534; their trucks (rus_truck_corps, ...) hold the building.
//!
//!Native evidence (AA.exe, image base0x400000; original and v14 bytes equal):
//!*4624B0 ActorData constructor462D41..462D75 (ebx is the4624E1 zero):
//! +534/+538/+53C/+540=0, +544=0xBF800000, +548/+54C/+550/+554=0.
//!*464BD0 ActorData LoadData block46852F..4686D9: 5B4410
//! `MobileDeploymentComponent`(flag0) present ORs+174 bit19, then reads
//! String `unpack_to_actor_ids`(5B54D0). Empty: Fixed `pack_health_min`,
//! `packing_length`, `unpacking_length`(5B5130) stored at+54C/+550/+554.
//! Nonempty: operator new(0x10),473AD0 prepends a deferred reference at
//! GameData+824 (vtable A0347C, +C=this ActorData), then Point2
//! `packed_position_offset`->+538/+53C and `packed_orientation`->+540/+544
//! (5B4D80), then46864A..4686AB raises410C40 "Invalid packed orientation"
//! when both|x| and|y| are ordered and<=A032E0 (binary32 1e-4).
//!*475230 walks GameData+824 head first;461840 calls4605B0: this+534=
//! 4BDBA0(IDS,0x2B any category ActorData), target+548=this, with the
//! Body/MobileDeployment/Element/soft-shape410C40 gates. That deferred pass is
//! ported and Unicorn-proven by aa_rules::native_mobile_deployment
//! (research/skirmish-20261016-mobile-actions).
//!*45DD80 (ActorData vtable A05540+14) tail45DF0B..45DF29: when+174 bit19
//! and+534 is nonnull, (+534)->vf14(delta,0), i.e. the target's own45DD80.
//!Proof of the composed chain: research/skirmish-20261018-actor-deployment-534.
use aa_numeric::Precision;
use aa_rules::{actors::ResolvedNode,native::TypedValue,native_mobile_deployment::NativeMobileDeploymentData};
type Result<T>=std::result::Result<T,String>;

///ActorData+174 bit19 (component enum slot19, MobileDeployment).
pub(crate) const MOBILE_DEPLOYMENT_BIT:u32=1<<19;
///A032E0, the binary32 compared by46865D/46867F.
pub(crate) const ORIENTATION_EPSILON_BITS:u32=0x38d1_b717;
pub(crate) const COMPONENT:&str="MobileDeploymentComponent";

///The ActorData words written by the46852F..4686D9 block, starting from the
///4624B0 constructor values. `reference` is the473AD0 deferred IDS.
#[derive(Clone,Debug,PartialEq,Eq)]
pub(crate) struct Block{
    pub(crate) reference:Option<String>,
    ///+538/+53C
    pub(crate) packed_position_offset_bits:[u32;2],
    ///+540/+544
    pub(crate) packed_orientation_bits:[u32;2],
    ///+54C/+550/+554
    pub(crate) pack_bits:[u32;3],
}
impl Block{
    pub(crate) const fn constructed()->Self{
        Self{reference:None,packed_position_offset_bits:[0,0],packed_orientation_bits:[0,0xbf80_0000],pack_bits:[0;3]}
    }
}
///The three typed readers the block calls, in call order.
pub(crate) trait Source{
    ///5B54D0
    fn string(&mut self,key:&'static str)->Result<String>;
    ///5B5130 (binary32 result stored by fstp dword)
    fn fixed(&mut self,key:&'static str)->Result<u32>;
    ///5B4D80
    fn point(&mut self,key:&'static str)->Result<[u32;2]>;
}
///46856C..468645: the reads and stores in native order. The deferred
///reference is queued before the two Point2 reads. Orientation validation is
///separate so a rejected row still exposes the stores made before410C40.
pub(crate) fn read_block(source:&mut impl Source)->Result<Block>{
    let mut block=Block::constructed();
    let reference=source.string("unpack_to_actor_ids")?;
    //468581: CString length -0xC, not text comparison.
    if reference.is_empty(){
        for(i,key)in["pack_health_min","packing_length","unpacking_length"].into_iter().enumerate(){block.pack_bits[i]=source.fixed(key)?;}
    }else{
        block.reference=Some(reference);
        block.packed_position_offset_bits=source.point("packed_position_offset")?;
        block.packed_orientation_bits=source.point("packed_orientation")?;
    }
    Ok(block)
}
///46864A..46868A for one component: fcom 0.0 then fchs only when C0 (less
///or unordered), then fcomp A032E0; exactly one of C0/C3 set continues.
///The f32 operands compare exactly at every x87 precision.
fn small(bits:u32)->bool{let v=f32::from_bits(bits);!v.is_nan()&&v.abs()<=f32::from_bits(ORIENTATION_EPSILON_BITS)}
pub(crate) fn orientation_rejected(bits:[u32;2])->bool{small(bits[0])&&small(bits[1])}
///Only the forward (nonempty IDS) branch reaches46864A.
pub(crate) fn validate_block(block:&Block,id:&str)->Result<()>{
    if block.reference.is_some()&&orientation_rejected(block.packed_orientation_bits){
        return Err(format!("464BD0 410C40 \"Invalid packed orientation (%i, %i) loading actor '%s'\" for {id}"));
    }
    Ok(())
}
///Production reader over the SAME resolved component node and source PC.
pub(crate) struct Node<'n,'a>{pub(crate) node:&'n ResolvedNode<'a>,pub(crate) pc:Precision}
impl Node<'_,'_>{
    fn required(&self,key:&str)->Result<TypedValue>{
        self.node.field(key,self.pc).map_err(|e|e.to_string())?.ok_or_else(||format!("MobileDeployment field {key} source absent"))
    }
}
impl Source for Node<'_,'_>{
    fn string(&mut self,key:&'static str)->Result<String>{
        match self.required(key)?{TypedValue::String{value,..}=>Ok(value),_=>Err(format!("MobileDeployment {key} requires native String source"))}
    }
    fn fixed(&mut self,key:&'static str)->Result<u32>{self.required(key)?.fixed_bits().map_err(|e|e.to_string())}
    fn point(&mut self,key:&'static str)->Result<[u32;2]>{
        match self.required(key)?{TypedValue::Point{bits,..}if bits.len()==2=>Ok([bits[0],bits[1]]),_=>Err(format!("MobileDeployment {key} requires native Point2 source"))}
    }
}
///+534 after the completed deferred pass. A clear bit19 means45DD80 never
///reads the word (and no4605B0 owner exists), so no provider is consulted.
///The provider's own reading of the same block must agree; any divergence is
///an explicit error, never a guessed link.
pub(crate) fn forward_534(mask:u32,id:&str,block:Option<&Block>,mobile:Option<&NativeMobileDeploymentData>)->Result<Option<String>>{
    let present=mask&MOBILE_DEPLOYMENT_BIT!=0;
    if present!=block.is_some(){return Err("ActorData+174 bit19 and MobileDeploymentComponent source differ".into())}
    let Some(block)=block else{return Ok(None)};
    let d=mobile.ok_or("45DD80 bit19 requires the completed475230/4605B0 deferred source pass")?.definition(id).map_err(|e|e.to_string())?;
    if !d.mobile||d.packed_position_offset_bits!=block.packed_position_offset_bits||d.pack_health_min_bits!=block.pack_bits[0]{
        return Err("4605B0 deferred source projection differs from464BD0 MobileDeployment block".into())
    }
    match(&block.reference,&d.unpack_to){
        (Some(_),Some(target))=>{require_finite_chain(id,mobile.unwrap())?;Ok(Some(target.clone()))}
        (None,None)=>Ok(None),
        (Some(_),None)=>Err("4605B0 queued MobileDeployment reference has no completed+534".into()),
        (None,Some(_))=>Err("ActorData+534 written without a queued473AD0 reference".into()),
    }
}
///A+534 chain that revisits an actor makes native45DD80 recurse through
///vf14 without end (stack exhaustion); such a chain is refused, not modelled.
pub(crate) fn require_finite_chain(id:&str,mobile:&NativeMobileDeploymentData)->Result<()>{
    let mut seen=std::collections::BTreeSet::new();let mut current=id.to_ascii_lowercase();
    loop{
        if !seen.insert(current.clone()){return Err(format!("45DD80 ActorData+534 chain from {id} is cyclic; native request recursion does not terminate"))}
        match &mobile.definition(&current).map_err(|e|e.to_string())?.unpack_to{Some(next)=>current=next.to_ascii_lowercase(),None=>return Ok(())}
    }
}
#[cfg(test)]
#[path="actor_deployment_534_tests.rs"]
mod tests;
