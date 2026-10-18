//! Native DynamicDoodad saved construction and two-slot Load. Disturbable and
//! Breakable components use their own factory; these are not GActor slot IDs.
use crate::{
    Error, Result,
    inventory::Reader,
    restore_components::{UpdatableRegistration, UpdatableTarget},
};

#[derive(Clone, Debug)]
pub struct Definition {
    /// Canonical owning source definition. Inline component definitions retain
    /// the pair (owning definition, slot), never a fabricated registry ordinal.
    pub owner_id: String,
    pub component_mask: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Component<O = u32> {
    pub owner: O,
    pub definition_owner: String,
    pub slot: u8,
    /// Constructor/Load receipt only. Once bound to a live scheduler owner,
    /// Queues owns the real cancellation chain; runtime queries use that owner.
    pub scheduled_head: Option<u32>,
    pub registered_updatable_id: Option<i32>,
    /// Disturbable+18 starts zero and is not read by its saved Load.
    /// Breakable words18/1C/20/24/28/2C are restored in this exact order.
    pub words: Vec<u32>,
}
impl<O> Component<O> {
    /// Actual component constructor fields. The source of the two Breakable
    /// point words is a separate startup owner; callers must supply that loan.
    pub fn from_constructor(owner: O, definition: &Definition, slot: u8, point: [u32; 2]) -> Result<Self> {
        if slot > 1 { return Err(Error::Format("DynamicDoodad constructor slot outside 0..2".into())); }
        Ok(Self {
            owner, definition_owner: definition.owner_id.clone(), slot,
            scheduled_head: None, registered_updatable_id: None,
            words: if slot == 0 { vec![0] } else { vec![0, 0, 0, point[0], point[1], 0x3f800000] },
        })
    }
}
impl Component<u32> {
    fn new(owner: u32, definition: &Definition, slot: u8) -> Self {
        Self {
            owner,
            definition_owner: definition.owner_id.clone(),
            slot,
            scheduled_head: None,
            registered_updatable_id: None,
            words: if slot == 0 {
                vec![0]
            } else {
                vec![0, 0, 0, 0, 0, 0x3f800000]
            },
        }
    }
}
#[derive(Clone, Debug)]
pub struct DynamicDoodad<O = u32> {
    /// 4DC6F0 leaves both component pointers null after GObject construction.
    pub components: [Option<Component<O>>; 2],
    pub source_version: Option<u32>,
    pub saved_mask: Option<u32>,
    pub original_definition_mask: Option<u32>,
}
impl<O> Default for DynamicDoodad<O> {
    fn default() -> Self {
        Self { components: [None, None], source_version: None, saved_mask: None, original_definition_mask: None }
    }
}
impl DynamicDoodad<u32> {
    ///4DCD10 calls4DCBF0 for slot0 then1. Saved bits load after construction;
    /// newly added definition bits construct only when absent in the saved
    /// definition mask. Registration occurs before any later component bytes.
    pub fn load(
        &mut self,
        bytes: &[u8],
        definition: &Definition,
        owner: u32,
        world_word_48: Option<u32>,
        mut register: impl FnMut(i32, &UpdatableTarget) -> std::result::Result<(), String>,
        registrations: &mut Vec<UpdatableRegistration>,
    ) -> Result<usize> {
        let mut r = Reader::new(bytes);
        self.source_version = Some(r.version(0)?);
        let saved = r.u32()?;
        self.saved_mask = Some(saved);
        let defaults = r.u32()?;
        self.original_definition_mask = Some(defaults);
        for slot in 0..2u8 {
            if self.components[slot as usize].is_some() {
                return Err(Error::Format(
                    "DynamicDoodad Load requires an empty constructed slot".into(),
                ));
            }
            let bit = 1 << slot;
            let present = saved & bit != 0;
            if !present && (definition.component_mask & bit == 0 || defaults & bit != 0) {
                continue;
            }
            if definition.component_mask & bit == 0 {
                return Err(Error::Format(format!(
                    "saved DynamicDoodad slot{slot} has no resolved definition"
                )));
            }
            self.components[slot as usize] = Some(Component::new(owner, definition, slot));
            if !present {
                continue;
            }
            let component = self.components[slot as usize]
                .as_mut()
                .ok_or_else(|| Error::Format("DynamicDoodad component disappeared".into()))?;
            let breakable_version = if slot == 1 { Some(r.version(1)?) } else { None };
            r.version(0)?; //5903E0 inherited component layer.
            component.scheduled_head = None; //50BCD0 calls50B250 before reading.
            let updatable_version = r.version(1)?;
            if updatable_version != 0 {
                let id = r.u32()? as i32;
                if id >= 0 {
                    let target = UpdatableTarget::DynamicDoodad { owner, slot };
                    register(id, &target).map_err(Error::Format)?;
                    component.registered_updatable_id = Some(id);
                    registrations.push(UpdatableRegistration {
                        serialized_id: id,
                        target,
                    });
                }
            }
            r.version(0)?; //590409 remaining Disturbable/base version.
            if let Some(version) = breakable_version {
                for word in &mut component.words[..5] {
                    *word = r.u32()?;
                }
                if version >= 1 {
                    component.words[5] = r.u32()?;
                }
                let mode = world_word_48.ok_or_else(|| {
                    Error::Format("Breakable Load requires initialized GWorld+48 mode".into())
                })?;
                if mode == 0 {
                    component
                        .words
                        .copy_from_slice(&[0, 0, 0, 0, 0, 0x3f800000]);
                }
            }
        }
        Ok(r.at)
    }
}
