//! Native City5640A0/564300 admission and candidate publication. Every read
//! remains lazy; the caller supplies the same actual component, source, GTID
//! lists and queue owner. Source+290/+291 are distinct reached byte reads.
use aa_numeric::Precision;

/// Native FCOMP0/TEST AH,41/JNP accepts ordered greater and unordered.
/// The producer must execute actual4D01E0, with no binary32 result spill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueComparison { Less, Equal, Greater, Unordered }
impl ValueComparison {
    pub fn admitted(self)->bool {matches!(self,Self::Greater|Self::Unordered)}
}
pub trait World {
    type Error;
    type Definition:Copy;
    fn city_owner(&mut self)->Result<u32,Self::Error>;
    fn actor_flags(&mut self,actor:u32)->Result<u32,Self::Error>;
    fn actor_owner_108(&mut self,actor:u32)->Result<u32,Self::Error>;
    fn kingdom_nonmajor_190(&mut self,kingdom:u32)->Result<u32,Self::Error>;
    ///4549C0: actual Element+80, then its organization+10 when nonnull.
    fn element_organization(&mut self,actor:u32)->Result<Option<u32>,Self::Error>;
    fn actor_value_4d01e0(&mut self,actor:u32,pc:Precision)->Result<ValueComparison,Self::Error>;
    fn body_present_60(&mut self,actor:u32)->Result<bool,Self::Error>;
    fn definition(&mut self,actor:u32)->Result<Self::Definition,Self::Error>;
    fn source_byte(&mut self,definition:Self::Definition,offset:u32)->Result<u8,Self::Error>;
    fn actor_reference_14(&mut self,actor:u32)->Result<u32,Self::Error>;
    fn accepted_contains_10(&mut self,reference:u32)->Result<bool,Self::Error>;
    fn candidates_contains_18(&mut self,reference:u32)->Result<bool,Self::Error>;
    fn candidates_prepend_18(&mut self,reference:u32)->Result<(),Self::Error>;
    fn pending_c(&mut self)->Result<bool,Self::Error>;
    ///Actual50AC60(embedded Updatable+8,0), same queue/RNG owner.
    fn schedule_50ac60(&mut self)->Result<(),Self::Error>;
}

/// Native5640A0 only. True authorizes the caller's subsequent list operation;
/// it does not itself append or schedule anything.
pub fn eligible<W:World+?Sized>(world:&mut W,target:u32,pc:Precision)->Result<bool,W::Error>{
    if target==0 || target==world.city_owner()? {return Ok(false)}
    if world.actor_flags(target)?&1!=0{return Ok(false)}
    if world.actor_owner_108(target)?==0{return Ok(false)}
    let owner=world.actor_owner_108(target)?;
    if world.kingdom_nonmajor_190(owner)?!=0{return Ok(false)}
    if world.element_organization(target)?.is_some(){return Ok(false)}
    if world.actor_flags(target)?&(1<<12)!=0{return Ok(false)}
    if !world.actor_value_4d01e0(target,pc)?.admitted(){return Ok(false)}
    let body=world.body_present_60(target)?;
    //564115 reads the source pointer even on the Body-null branch.
    let definition=world.definition(target)?;
    if body && world.source_byte(definition,0x290)?==0
        && world.source_byte(definition,0x291)?==0 {return Ok(false)}
    let reference=world.actor_reference_14(target)?;
    Ok(!world.accepted_contains_10(reference)?)
}

/// Actual564300: eligible prefix, GTID re-read, pending candidate dedup, a
/// second GTID re-read, immediate prepend, then the actual scheduling gate.
/// A schedule error retains the already published candidate allocation.
pub fn admit<W:World+?Sized>(world:&mut W,target:u32,pc:Precision)->Result<(),W::Error>{
    if !eligible(world,target,pc)?{return Ok(())}
    let reference=world.actor_reference_14(target)?;
    if world.candidates_contains_18(reference)?{return Ok(())}
    let reference=world.actor_reference_14(target)?;
    world.candidates_prepend_18(reference)?;
    if !world.pending_c()?{world.schedule_50ac60()?}
    Ok(())
}

#[cfg(test)]
#[path="zone_city_admission_tests.rs"]
mod tests;
