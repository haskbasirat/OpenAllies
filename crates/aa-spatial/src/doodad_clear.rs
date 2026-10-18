//! Native4F5560. The overlap list is collected before any deletion. Handles
//! are resolved afresh in list order; stale generations are required errors,
//! since the original would dereference a null Thing at that point.
use crate::{Result, geometry::Placement};

pub trait World {
    type Object: Copy;
    fn collect_overlaps(&mut self, placement: &Placement) -> Result<Vec<u32>>;
    fn resolve(&mut self, reference: u32) -> Result<Self::Object>;
    fn runtime_class(&mut self, thing: Self::Object) -> Result<u32>;
    fn flags(&mut self, thing: Self::Object) -> Result<u32>;
    ///54C200 on the SAME process TriggerManager (B36520), not on the Thing.
    fn referenced_by_trigger(&mut self, thing: Self::Object) -> Result<bool>;
    fn definition_ids(&mut self, thing: Self::Object) -> Result<String>;
    fn current_gtid(&mut self, thing: Self::Object) -> Result<u32>;
    ///Modern diagnostics transport; native eager argument reads remain above.
    fn log_purge(&mut self, reference: u32, ids: String) -> Result<()>;
    ///4F5490 on the captured World and current Thing.
    fn destroy(&mut self, thing: Self::Object) -> Result<()>;
}

pub fn clear<W: World>(world: &mut W, placement: &Placement) -> Result<()> {
    let references = world.collect_overlaps(placement)?;
    for reference in references {
        let thing = world.resolve(reference)?;
        if matches!(world.runtime_class(thing)?, 2 | 9)
            && world.flags(thing)? & (1 << 10) == 0
            && !world.referenced_by_trigger(thing)?
        {
            //4F5613..19: IDS is read before the current GTID.
            let ids = world.definition_ids(thing)?;
            let reference = world.current_gtid(thing)?;
            world.log_purge(reference, ids)?;
            world.destroy(thing)?;
        }
    }
    Ok(())
}
