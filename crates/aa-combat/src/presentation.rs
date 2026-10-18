//! Native damage-number queue `50D0B0` (aa-body's
//! `Action::PresentationDamaged`): when not in session mode 6, with the
//! ViewShowDamage view byte set and the actor definition `+54` bit1 set, an
//! entry `{0, actor GTID, damage record}` is appended (`50D070`; the native
//! vector growth `50CF30` is an allocation detail, not a limit).
use crate::{Error, Result};
use aa_body::damage::DamageRecord;
use serde::{Deserialize, Serialize};

/// The first nine words of the native 0x4C-byte entry; the rest are not
/// written by this producer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageNumber {
    pub word0: u32,
    pub actor_id: u32,
    pub record: DamageRecord,
}

pub trait World {
    fn session_mode(&self) -> Result<u32>;
    /// `[B39940]+88` byte (ViewShowDamage).
    fn show_damage(&self) -> Result<bool>;
    /// Actor `+0C` definition `+54` flags, `None` when the definition is null.
    fn definition_flags54(&self, actor: u32) -> Result<Option<u32>>;
    fn actor_id(&self, actor: u32) -> Result<u32>;
}

/// Native `50D0B0(actor, record)`.
pub fn queue_damage_number(
    queue: &mut Vec<DamageNumber>,
    actor: u32,
    record: &DamageRecord,
    world: &impl World,
) -> Result<()> {
    if world.session_mode()? == 6 || !world.show_damage()? {
        return Ok(());
    }
    let Some(flags) = world.definition_flags54(actor)? else {
        return Ok(());
    };
    if flags & 2 == 0 {
        return Ok(());
    }
    queue
        .try_reserve(1)
        .map_err(|e| Error::World(format!("damage number allocation failed: {e}")))?;
    queue.push(DamageNumber {
        word0: 0,
        actor_id: world.actor_id(actor)?,
        record: *record,
    });
    Ok(())
}
