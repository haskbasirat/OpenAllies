//! Original Area and Camera derived saved state. Native base Object fields are
//! restored separately; these objects are not units or synthetic Actor slots.
use crate::{
    Error, Result,
    inventory::Reader,
    restore::RestoreDefinitions,
    restore_components::{UpdatableRegistration, UpdatableTarget},
};
#[derive(Clone, Debug, Default)]
pub struct Camera {
    pub source_version: Option<u32>,
    pub word_60: u32,
    pub label_utf16: Vec<u16>,
    pub source_localization_key: Option<Vec<u16>>,
}
impl Camera {
    pub fn load(&mut self, bytes: &[u8], definitions: &impl RestoreDefinitions) -> Result<usize> {
        let mut r = Reader::new(bytes);
        let version = r.version(2)?;
        self.source_version = Some(version);
        if version == 0 {
            return Err(Error::Format("legacy Camera v0 direction/position conversion requires native transform callbacks".into()));
        }
        let text = r.string()?;
        if version == 1 {
            self.source_localization_key = Some(text.clone());
            self.label_utf16 = definitions.localize(&text).map_err(Error::Format)?;
        } else {
            self.source_localization_key = None;
            self.label_utf16 = text;
        }
        self.word_60 = r.u32()?;
        Ok(r.at)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AreaEntry {
    pub saved_handle: u32,
    pub byte: u8,
}
#[derive(Clone, Debug, Default)]
pub struct Area {
    pub source_version: Option<u32>,
    pub scheduled_head: Option<u32>,
    pub registered_updatable_id: Option<i32>,
    /// Original Shape68 record; exact type word and five binary32/parameter
    /// words are retained without inferring a shape from preview bounds.
    pub shape_words: [u32; 6],
    pub label_utf16: Vec<u16>,
    pub source_localization_key: Option<Vec<u16>>,
    /// Native84 head-first linked list, including duplicate saved IDs/bytes.
    pub entries: Vec<AreaEntry>,
}
impl Area {
    pub fn load(
        &mut self,
        bytes: &[u8],
        world_version: u32,
        owner: u32,
        definitions: &impl RestoreDefinitions,
        trace: &mut Vec<UpdatableRegistration>,
    ) -> Result<usize> {
        let mut r = Reader::new(bytes);
        if world_version >= 22 {
            self.scheduled_head = None;
            let version = r.version(1)?;
            if version != 0 {
                let id = r.u32()? as i32;
                if id >= 0 {
                    let target = UpdatableTarget::Area { owner };
                    definitions
                        .register_updatable(id, &target)
                        .map_err(Error::Format)?;
                    self.registered_updatable_id = Some(id);
                    trace.push(UpdatableRegistration {
                        serialized_id: id,
                        target,
                    });
                }
            }
        }
        let version = r.version(2)?;
        self.source_version = Some(version);
        let text = r.string()?;
        if version < 2 {
            self.source_localization_key = Some(text.clone());
            self.label_utf16 = definitions.localize(&text).map_err(Error::Format)?;
        } else {
            self.source_localization_key = None;
            self.label_utf16 = text;
        }
        for word in &mut self.shape_words {
            *word = r.u32()?;
        }
        if version >= 1 {
            let count = r.u32()? as i32;
            for _ in 0..count.max(0) {
                let entry = AreaEntry {
                    saved_handle: r.u32()?,
                    byte: r.take(1)?[0],
                };
                self.entries.try_reserve(1).map_err(|_| Error::Allocation)?;
                self.entries.insert(0, entry);
            }
        }
        Ok(r.at)
    }
}
#[derive(Clone, Debug)]
pub enum SpecialObject {
    Area(Area),
    Camera(Camera),
}
