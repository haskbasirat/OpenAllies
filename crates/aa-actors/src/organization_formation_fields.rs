//! Storage-independent577F30/57BF50/57BFF0 over the current Organization.
use super::*;
pub trait FormationFields {
    fn word(&mut self, offset: u32) -> Result<u32>;
    fn put_word(&mut self, offset: u32, value: u32) -> Result<()>;
    fn byte(&mut self, offset: u32) -> Result<u8>;
    fn put_byte(&mut self, offset: u32, value: u8) -> Result<()>;
    fn state_group4(&mut self) -> Result<Option<StateWords>>;
    fn layout_info(&mut self) -> Result<LayoutInfo>;
    fn formation_facts(&mut self, formation: u32) -> Result<(u8, u32)>;
    fn has_applied_property(&mut self, property: u32, source: ActorId) -> Result<bool>;
    fn remove_property(&mut self, property: u32, source: ActorId) -> Result<()>;
    fn apply_property(&mut self, property: u32, kind: u32, source: ActorId) -> Result<()>;
    fn formation_refresh(&mut self) -> Result<()>;
    fn owner_object_164(&mut self) -> Result<Option<u32>>;
    fn object_refresh(&mut self, object: u32) -> Result<()>;
}
pub fn formation_byte_fields(fields: &mut impl FormationFields, value: u8) -> Result<()> {
    fields.put_byte(0x8c, value)?;
    let Some(descriptor) = fields.state_group4()? else {
        return Ok(());
    };
    let old = fields.byte(0x8d)?;
    match descriptor.word_24 {
        0 => fields.put_byte(0x8d, 0)?,
        1 => fields.put_byte(0x8d, 1)?,
        2 => {
            let current = fields.byte(0x8c)?;
            fields.put_byte(0x8d, current)?;
        }
        _ => {}
    }
    if fields.byte(0x8d)? == 0 {
        fields.put_byte(0x8e, 0)?;
    } else if old == 0 {
        fields.put_byte(0x8e, 1)?;
    }
    Ok(())
}
fn select_secondary_fields(
    fields: &mut impl FormationFields,
    formation: u32,
    owner: ActorId,
    info: &LayoutInfo,
) -> Result<()> {
    let current = fields.word(0x74)?;
    if formation == current {
        return Ok(());
    }
    if current != 0 {
        let (_, property) = fields.formation_facts(current)?;
        if property != 0 {
            fields.remove_property(property, owner)?;
        }
        fields.put_word(0x74, 0)?;
    } else if fields.has_applied_property(info.property_50, owner)? {
        fields.remove_property(info.property_50, owner)?;
    }
    if formation != 0 {
        fields.put_word(0x74, formation)?;
        let (_, property) = fields.formation_facts(formation)?;
        if property != 0 {
            fields.apply_property(property, 4, owner)?;
        }
    } else {
        fields.apply_property(info.property_50, 4, owner)?;
    }
    fields.formation_refresh()?;
    if let Some(object) = fields.owner_object_164()? {
        fields.object_refresh(object)?;
    }
    Ok(())
}
pub fn set_formation_fields(
    fields: &mut impl FormationFields,
    formation: u32,
    owner: ActorId,
) -> Result<()> {
    fields.put_word(0x70, formation)?;
    let Some(descriptor) = fields.state_group4()? else {
        return Ok(());
    };
    let info = fields.layout_info()?;
    let indexed = |index: i32| -> Result<u32> {
        usize::try_from(index)
            .ok()
            .and_then(|i| info.formations.get(i).copied())
            .ok_or_else(|| {
                LoadError::Unsupported(format!("LayoutData formation index {index} outside +34"))
            })
    };
    let secondary = match descriptor.word_20 {
        0 => Some(fields.word(0x70)?),
        1 => Some(0),
        2 => Some(indexed(info.formation_44)?),
        3 => Some(indexed(info.formation_48)?),
        _ => None,
    };
    if let Some(f) = secondary {
        select_secondary_fields(fields, f, owner, &info)?;
    }
    let desired = fields.word(0x70)?;
    let value = if desired != 0 {
        fields.formation_facts(desired)?.0
    } else {
        0
    };
    formation_byte_fields(fields, value)
}
pub(super) struct Held<'a, D, W> {
    pub org: &'a mut OrganizationState<D>,
    pub world: &'a mut W,
}
impl<D, W: OrganizationWorld<D>> FormationFields for Held<'_, D, W> {
    fn word(&mut self, o: u32) -> Result<u32> {
        word(self.org, o)
    }
    fn put_word(&mut self, o: u32, v: u32) -> Result<()> {
        self.org.words.insert(o, v);
        Ok(())
    }
    fn byte(&mut self, o: u32) -> Result<u8> {
        Ok(self.org.bytes.get(&o).copied().unwrap_or(0))
    }
    fn put_byte(&mut self, o: u32, v: u8) -> Result<()> {
        self.org.bytes.insert(o, v);
        Ok(())
    }
    fn state_group4(&mut self) -> Result<Option<StateWords>> {
        self.world.state_group4()
    }
    fn layout_info(&mut self) -> Result<LayoutInfo> {
        self.world.layout_info(&self.org.layout.definition)
    }
    fn formation_facts(&mut self, f: u32) -> Result<(u8, u32)> {
        self.world.formation_facts(f)
    }
    fn has_applied_property(&mut self, p: u32, s: ActorId) -> Result<bool> {
        self.world.has_applied_property(p, s)
    }
    fn remove_property(&mut self, p: u32, s: ActorId) -> Result<()> {
        self.world.remove_property(p, s)
    }
    fn apply_property(&mut self, p: u32, k: u32, s: ActorId) -> Result<()> {
        self.world.apply_property(p, k, s)
    }
    fn formation_refresh(&mut self) -> Result<()> {
        self.world.formation_refresh(self.org)
    }
    fn owner_object_164(&mut self) -> Result<Option<u32>> {
        self.world.owner_object_164()
    }
    fn object_refresh(&mut self, o: u32) -> Result<()> {
        self.world.object_refresh(o)
    }
}
