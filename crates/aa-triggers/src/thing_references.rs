//! Native54C200 ->540C40 ->543A90: protect Things mentioned directly by any
//! trigger parameter. This queries the existing manager definitions in their
//! loaded list order, independently of enabled/repeat/update state.
use crate::{decode::{Element, Trigger, Value}, manager::{Error, Manager, Result}};

pub trait Thing {
    fn current_gtid(&mut self) -> Result<u32>;
    ///Actual4BEDF0 name, including its CString ownership operation. No folded
    ///case, localized IDS, or definition name may replace this field.
    fn name(&mut self) -> Result<Vec<u16>>;
}

fn nul_prefix(text: &[u16]) -> &[u16] {
    &text[..text.iter().position(|c| *c == 0).unwrap_or(text.len())]
}

///543A90. Definitions are immutable in Manager while this query runs; native
///4BEDF0 only owns a copy of the Thing name and cannot modify their array.
pub fn element_references<T: Thing>(element: &Element, thing: &mut T) -> Result<bool> {
    for parameter in &element.parameters {
        let argument = crate::argument_index(&parameter.argument).ok_or_else(|| Error::State(
            format!("543A90 unregistered argument {}", parameter.argument)))?;
        if !matches!(argument, 5 | 11 | 10) { continue; }
        match parameter.item_index {
            12 | 31 | 30 => {
                let Value::Word(reference) = parameter.value else {
                    return Err(Error::State("543A90 direct reference lacks its actual word".into()));
                };
                if reference == thing.current_gtid()? { return Ok(true); }
            }
            15 => {
                //Native fetches the current name before rereading the entry's
                //text and executing the case-sensitive UTF16 wcscmp.
                let name = thing.name()?;
                let Value::Text(ref text) = parameter.value else {
                    return Err(Error::State("543A90 named reference lacks its actual CString".into()));
                };
                if nul_prefix(text) == nul_prefix(&name) { return Ok(true); }
            }
            _ => {}
        }
    }
    Ok(false)
}

///540C40 reads events (+14), conditions (+1C), actions (+24) in that order.
pub fn trigger_references<T: Thing>(trigger: &Trigger, thing: &mut T) -> Result<bool> {
    for list in [&trigger.events, &trigger.conditions, &trigger.actions] {
        for element in list {
            if element_references(element, thing)? { return Ok(true); }
        }
    }
    Ok(false)
}

///54C200 returns the first matching SAME manager-owned Trigger. An ordinal
///represents that retained allocation without exposing a 32-bit pointer.
pub fn first_referencing<T: Thing>(manager: &Manager, thing: &mut T) -> Result<Option<usize>> {
    for (ordinal, trigger) in manager.definitions().iter().enumerate() {
        if trigger_references(trigger, thing)? { return Ok(Some(ordinal)); }
    }
    Ok(None)
}
