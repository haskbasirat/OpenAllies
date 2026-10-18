//!Native NiSequenceResource5A5FB0 over the SAME generic Resource entry/task.
//!This constructor leaves real Ni/source objects NULL. No KF decoder, loaded
//!flag, sequence duration or modern animation playback is manufactured here.
use crate::resource_objects::{Entry,Handle,Kind,SourceFile};
use std::sync::Arc;

pub(crate) struct Fields{
    pub(crate) source3c:Option<crate::resource_ni_cache::EntryHandle>,
    pub(crate) source40:Option<()>,
    pub(crate) word44:u32,
    pub(crate) word48:u32,
    pub(crate) value4c:u32,
}
impl Entry {
    ///9D0710 registers this actual5A5FB0 factory under B3D9A4, with base
    ///B3D968.5AA890 supplies the one base/task owner, then5A5FEE..6001
    ///writes these derived fields and vtableA45290.
    pub(crate) fn sequence(origin:&Arc<()>,ids:Vec<u16>,source:SourceFile,
        priority:&crate::resource_task::DefaultPriority)->Result<Self,String>{
        let handle=Handle::new(origin,Kind::Sequence,ids,source);
        let task=crate::resource_task::Task::construct_for_resource(&handle,priority)?;
        task.retain_in_resource_5aa944()?;
        Ok(Self{handle,requests4:0,flags8:0x10,memory0c:0,memory10:0,references14:0,task:Arc::new(task),locations:Vec::new(),
            texture:None,model:None,sound:None,sequence:Some(Fields{source3c:None,source40:None,word44:0,word48:0,value4c:0x3f800000})})
    }
}
#[cfg(test)]
#[path="resource_sequence_tests.rs"]
mod tests;
