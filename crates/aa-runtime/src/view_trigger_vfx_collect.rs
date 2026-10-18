//!Private ordered54BDE0/540A60/546840 collection kernel. The public owner
//!supplies concrete canonical source lookup and actual Data request services.
use aa_triggers::{decode::{Element,Value},manager::Manager};
use std::collections::VecDeque;
type Result<T>=std::result::Result<T,String>;

fn qualifying(element:&Element)->bool{
    matches!(aa_triggers::element_schema(element.opcode),Some((2,_)))
        &&(82..=84).contains(&element.opcode)
}
fn ids(element:&Element)->Result<&[u16]>{
    match element.parameters.first().map(|p|&p.value){
        Some(Value::Text(text))=>Ok(text),
        _=>Err("546840 qualifying VFX parameter has no current CString owner at+1C".into()),
    }
}
pub(super) fn collect<T>(
    entries:&mut VecDeque<T>,manager:&Manager,
    mut find:impl FnMut(&[u16])->Result<Option<T>>,
)->Result<()>{
    entries.clear();
    for trigger in manager.definitions(){
        for element in &trigger.actions{
            if !qualifying(element){continue}
            if let Some(effect)=find(ids(element)?)?{entries.push_front(effect)}
        }
    }
    Ok(())
}
pub(super) fn request<T>(
    entries:&VecDeque<T>,requested:&mut usize,
    mut apply:impl FnMut(&T)->Result<()>,
)->Result<()>{
    for effect in entries{
        apply(effect)?;
        *requested+=1;
    }
    Ok(())
}

#[cfg(test)]
#[path="view_trigger_vfx_collect_tests.rs"]
mod tests;
