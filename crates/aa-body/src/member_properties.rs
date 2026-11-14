//! Native property copy `4D0660(source, no_source, need40, need41)` on an
//! actor: every property instance of the source's `+78` list (`+14` count read
//! once, `+10` slots, instance `+C` definition), in order, whose definition
//! passes the optional byte filters (`+40` when `need40`, `+41` when
//! `need41`) and applies to this actor (`5035E0`) is added to this actor's
//! `+78` properties with kind 3 (`5817A0`), from the source actor, or from
//! no actor when `no_source`. Nothing happens unless both actors have `+78`.
//!
//! Oracle: research/phase8-combat/native_member_properties.py.
use crate::Result;

pub trait World {
    /// Actor `+78` properties.
    fn properties(&mut self, actor: u32) -> Result<Option<u32>>;
    /// Properties `+14` instance count.
    fn instance_count(&mut self, properties: u32) -> Result<i32>;
    /// Properties `+10[index]` instance `+C` definition.
    fn instance_definition(&mut self, properties: u32, index: i32) -> Result<Option<u32>>;
    /// Definition bytes `+40` and `+41`.
    fn definition_bytes(&mut self, definition: u32) -> Result<[u8; 2]>;
    /// `5035E0(actor, definition)`.
    fn applicable(&mut self, actor: u32, definition: u32) -> Result<bool>;
    /// `5817A0(definition, 3, source)` on the actor's `+78`.
    fn add(&mut self, properties: u32, definition: u32, kind: u32, source: Option<u32>) -> Result<()>;
}

/// Native `4D0660(source, no_source, need40, need41)` on `actor`.
pub fn copy_properties(
    world: &mut impl World,
    actor: u32,
    source: u32,
    no_source: bool,
    need40: bool,
    need41: bool,
) -> Result<()> {
    if world.properties(actor)?.is_none() {
        return Ok(());
    }
    let Some(list) = world.properties(source)? else {
        return Ok(());
    };
    let count = world.instance_count(list)?;
    let mut index = 0;
    while index < count {
        let definition = world.instance_definition(list, index)?;
        index += 1;
        let Some(definition) = definition else {
            // The +40/+41 filters read through the definition before its
            // null test; the native faults there when a filter is set.
            if need40 || need41 {
                return Err(crate::Error::Callback("4D0660 null property definition under a filter".into()));
            }
            continue;
        };
        let [byte40, byte41] = world.definition_bytes(definition)?;
        if (need40 && byte40 == 0) || (need41 && byte41 == 0) {
            continue;
        }
        let Some(properties) = world.properties(actor)? else {
            continue;
        };
        if world.applicable(actor, definition)? {
            world.add(properties, definition, 3, (!no_source).then_some(source))?;
        }
    }
    Ok(())
}
