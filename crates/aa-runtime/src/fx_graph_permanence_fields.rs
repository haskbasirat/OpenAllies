//!428A30 immutable recursive topology, with native virtual24/20 services.
//!The bound virtual20 source fields are shared with Actor state predicates.
type Result<T> = std::result::Result<T, String>;
pub(super) trait World {
    type Node: Copy;
    fn is_vfx(&self, node: Self::Node) -> Result<bool>;
    fn active(&self, node: Self::Node) -> Result<bool>;
    fn immediate(&self, node: Self::Node) -> Result<&[Self::Node]>;
    fn chain(&self, node: Self::Node) -> Result<&[Self::Node]>;
}
pub(super) fn scan<W: World>(world: &W, roots: &[W::Node]) -> Result<bool> {
    for &id in roots {
        if world.is_vfx(id)? && world.active(id)? {
            return Ok(true);
        }
        if scan(world, world.immediate(id)?)? || scan(world, world.chain(id)?)? {
            return Ok(true);
        }
    }
    Ok(false)
}
#[cfg(test)]
#[path = "fx_graph_permanence_fields_tests.rs"]
mod tests;
