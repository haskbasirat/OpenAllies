//!Native76D820 value traversal. Registration validity and publication are
//!checked by Inventory before this shared private traversal starts. Values
//!are borrowed from the same mutable SVar owner only at their native reads.
use crate::{launch_settings_dump::Sink,load_view_variables::{State,CurrentValue}};
type Result<T> = std::result::Result<T,String>;
trait Read {
    fn count(&self)->usize;
    fn name(&self,index:usize)->Result<(&str,u32)>;
    fn kind(&self,index:usize)->Result<u32>;
    fn word(&self,index:usize,component:usize)->Result<u32>;
    fn text(&self,index:usize)->Result<&[u16]>;
}
impl Read for State {
    fn count(&self)->usize {self.registrations().count()}
    fn name(&self,index:usize)->Result<(&str,u32)> {
        let r=self.registrations().nth(index).ok_or("current SVar registration index is absent")?;
        Ok((r.name,r.ordinal()))
    }
    fn kind(&self,index:usize)->Result<u32>{self.registrations().nth(index).map(|r|r.kind()).ok_or_else(||"current SVar type index is absent".into())}
    fn word(&self,index:usize,component:usize)->Result<u32>{
        match self.current(index)? {
            CurrentValue::Bool(v) if component==0=>Ok(u32::from(*v)),
            CurrentValue::Fixed(v)|CurrentValue::Int(v) if component==0=>Ok(*v),
            CurrentValue::Point3(v)=>v.get(component).copied().ok_or_else(||"current SVar Point3 component differs".into()),
            _=>Err("current SVar scalar kind differs from its native registration".into()),
        }
    }
    fn text(&self,index:usize)->Result<&[u16]>{match self.current(index)?{CurrentValue::CString(v)=>Ok(v),_=>Err("current SVar String kind differs".into())}}
}
fn traverse(values:&impl Read,sink:&mut impl Sink)->Result<()> {
    sink.marker("SyncSettings")?;
    let mut index=0;
    while index!=values.count() {
        let (name,ordinal)=values.name(index)?;
        sink.int(name,ordinal)?;
        //Native reads Var+24 and its value only AFTER the name logger.
        match values.kind(index)? {
            1=>sink.boolean("var->GetValueFlag()",values.word(index,0)? as u8!=0)?,
            2=>sink.float("var->GetValueFloat()",values.word(index,0)?)?,
            3=>sink.int("var->GetValueInt()",values.word(index,0)?)?,
            4=>sink.text("var->GetValueString()",values.text(index)?)?,
            5=>{
                sink.float("var->GetValuePoint3().x",values.word(index,0)?)?;
                sink.float("var->GetValuePoint3().y",values.word(index,1)?)?;
                sink.float("var->GetValuePoint3().z",values.word(index,2)?)?;
            },
            _=>{}, //Native jump-table bounds branch skips an unknown kind.
        }
        index+=1;
    }
    Ok(())
}
pub(super) fn synchronize(values:&State,sink:&mut impl Sink)->Result<()> {traverse(values,sink)}

#[cfg(test)]
#[path="current_svar_sync_tests.rs"]
mod tests;
