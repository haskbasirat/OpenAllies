//!Storage-independent bounded56EC90 dispatch used by held and live allocations.
//!The nonzero Actor138 path retains its required source430/Denizen/schedule
//!boundary; no null owner or optional child is guessed here.
pub(crate) trait Fields {
    type Error;
    fn word138(&mut self) -> Result<u32, Self::Error>;
    fn structure(&mut self) -> Result<bool, Self::Error>;
    fn remote_range(&mut self) -> Result<u32, Self::Error>;
    fn attached_path(&mut self) -> Result<(), Self::Error>;
    fn radius(&mut self) -> Result<u32, Self::Error>;
    fn flags(&mut self) -> Result<u32, Self::Error>;
    fn positive(&mut self, radius: u32) -> Result<bool, Self::Error>;
    fn zone_change(&mut self, enabled: bool, radius: u32) -> Result<(), Self::Error>;
}
pub(crate) fn after<W: Fields>(world: &mut W) -> Result<(), W::Error> {
    let structure = world.structure()?;
    let word138 = world.word138()?;
    if structure && word138 != 0 {
        return Ok(());
    }
    if structure {
        let radius = world.remote_range()?;
        return world.zone_change(true, radius);
    }
    if word138 != 0 {
        return world.attached_path();
    }
    //56ED79 reads flags first; the live radius is read for comparison only
    //when alive, then reread for56EB40's argument even on the disabled path.
    let enabled = if world.flags()? & 1 == 0 {
        let radius = world.radius()?;
        world.positive(radius)?
    } else {
        false
    };
    let radius = world.radius()?;
    world.zone_change(enabled, radius)
}

#[cfg(test)]
#[path = "detection_after_core_tests.rs"]
mod tests;
