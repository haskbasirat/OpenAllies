//!59B400's AudioResource constructor in the SAME ResourceManager/task arena.
//!The constructor has no decoded/raw audio allocation. Byte43 is unwritten
//!and deliberately has no manufactured value. No playback is performed.
use crate::resource_objects::{Entry, Handle, Kind, SourceFile};
use std::sync::Arc;

pub(crate) struct Fields {
    pub(crate) raw3c: Option<()>,
    pub(crate) standard_rate40: u8,
    pub(crate) streaming41: u8,
    pub(crate) byte42: u8,
}
impl Entry {
    ///9D02F0 registers59B400 under B3D76C.9D0900 selects this same class
    ///for WAV and MP3.5AA890 constructs and retains the actual common Task.
    pub(crate) fn sound(
        origin: &Arc<()>,
        ids: Vec<u16>,
        source: SourceFile,
        priority: &crate::resource_task::DefaultPriority,
    ) -> Result<Self, String> {
        let handle = Handle::new(origin, Kind::Sound, ids, source);
        let task = crate::resource_task::Task::construct_for_resource(&handle, priority)?;
        task.retain_in_resource_5aa944()?;
        Ok(Self {
            handle,
            requests4: 0,
            flags8: 0x10,
            memory0c: 0,
            memory10: 0,
            references14: 0,
            task: Arc::new(task),
            locations: Vec::new(),
            texture: None,
            model: None,
            sequence: None,
            sound: Some(Fields {
                raw3c: None,
                standard_rate40: 0,
                streaming41: 0,
                byte42: 0,
            }),
        })
    }
}

#[cfg(test)]
#[path = "resource_sound_tests.rs"]
mod tests;
