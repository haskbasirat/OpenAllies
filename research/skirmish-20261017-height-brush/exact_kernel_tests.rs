//Standalone exact current kernel/test source, using pinned numeric/spatial
//and initialized-heightfield dependency artifacts. No rewritten body mirror.
extern crate self as aa_heightfield;
pub use old_heightfield::binding;
#[path="../../crates/aa-heightfield/src/brush.rs"]pub mod brush;
#[path="../../crates/aa-runtime/src/height_brush_geometry.rs"]mod height_brush_geometry;
#[path="../../crates/aa-runtime/src/height_brush_tests.rs"]mod tests;
