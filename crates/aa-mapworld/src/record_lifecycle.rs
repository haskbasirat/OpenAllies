//! Original4FAE80 post-Load tail4FB4B7..4FB539, including the Init(false)
//! warning/destruction branch4FB570..4FB608. Earlier allocation/Load and common
//! string cleanup belong to the caller. Every reached service is mandatory.
use aa_numeric::Precision;

pub type Result<T> = std::result::Result<T, String>;

/// The out slot remains authoritative. Callbacks can null/replace it; the
/// coordinator rereads it only where the original instructions do. Handles
/// refer to actual owned allocations, never cloned restoration diagnostics.
pub trait World {
    type Record: Copy;
    fn current_record(&mut self) -> Result<Option<Self::Record>>;
    fn flags(&mut self, record: Self::Record) -> Result<u32>;
    fn clear_loading_flag(&mut self, record: Self::Record) -> Result<()>;
    /// Virtual+E8(NULL): the returned AL byte, not a guessed successful Init.
    fn init_null(&mut self, record: Self::Record) -> Result<u8>;
    ///542A00(&out) on world+110, after the editor path clears bit31.
    fn editor_insert_out_slot(&mut self) -> Result<()>;
    /// Virtual+EC(NULL,1). This can change the current out slot.
    fn post_create_null_one(&mut self, record: Self::Record) -> Result<()>;
    ///4F9510 on the current out value, reread after PostCreate.
    fn register_record(&mut self, record: Self::Record) -> Result<()>;
    /// Virtual+B0: AL!=0 identifies an Object for definition publication.
    fn object_predicate(&mut self, record: Self::Record) -> Result<u8>;
    ///4296B0(current_record,1) on current definition+BC.
    fn definition_publish_one(&mut self, record: Self::Record) -> Result<()>;
    /// Map warning5A1040 with the captured definition/source context and the
    /// current record's XY, following native string/format services. Precision
    /// is explicit for that owner; this coordinator invents no warning text.
    fn init_failed_warning(&mut self, record: Self::Record, pc: Precision) -> Result<()>;
    ///4F5490: native detach/destructor services, not just an arena removal.
    fn destroy_record(&mut self, record: Self::Record) -> Result<()>;
    ///4FB602 clears the out slot only after destruction successfully returns.
    fn clear_out_slot(&mut self) -> Result<()>;
}

fn required<W: World>(world: &mut W, native: &str) -> Result<W::Record> {
    world
        .current_record()?
        .ok_or_else(|| format!("{native} current record became null at a required dereference"))
}

/// Executes each record's immediate lifecycle before the next saved record is
/// loaded. No rollback: prior stores/effects remain on any later failure.
/// A null value at an unguarded native dereference is an explicit portable
/// refusal, never silently skipped. Success is not whole-world readiness.
pub fn finish(world: &mut impl World, pc: Precision) -> Result<()> {
    let Some(record) = world.current_record()? else {
        return Ok(());
    };
    if world.flags(record)? & (1 << 7) == 0 {
        if world.init_null(record)? == 0 {
            let warning_record = required(world, "4FB58F warning")?;
            world.init_failed_warning(warning_record, pc)?;
            let destroyed = required(world, "4FB5F7 -> 4F5490")?;
            world.destroy_record(destroyed)?;
            world.clear_out_slot()?;
            return Ok(());
        }
    } else {
        world.clear_loading_flag(record)?;
        world.editor_insert_out_slot()?;
    }
    let Some(record) = world.current_record()? else {
        return Ok(());
    };
    if world.flags(record)? & (1 << 7) == 0 {
        world.post_create_null_one(record)?;
        let registered = required(world, "4FB506 -> 4F9510")?;
        world.register_record(registered)?;
    }
    let Some(record) = world.current_record()? else {
        return Ok(());
    };
    if world.object_predicate(record)? != 0 {
        let published = required(world, "4FB526 -> 4296B0")?;
        world.definition_publish_one(published)?;
    }
    Ok(())
}
