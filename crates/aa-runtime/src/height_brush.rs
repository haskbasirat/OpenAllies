//!Same-endpoint551140 used by4EB500, over the retained BattleTerrain base map.
//!Source shape/facing comes from the caller's canonical Actor/Object getter.
//!The original temporary pending array is private to the synchronous kernel;
//!committed samples and dirty bounds remain in their one terrain owner.
use crate::{
    battle_terrain::{BattleTerrain, HeightBrushTerrain},
    height_brush_geometry,
};
use aa_heightfield::brush::{self, Settings};
use aa_numeric::Precision;
use aa_spatial::{collision::Shape, position::HeightEdit};
pub trait Effects {
    ///Actual current[B39990+64]+4/+8/+C, read once at each550B00 prepare.
    fn orientation(&mut self) -> Result<[u32; 3], String>;
    ///Actual immutable B3A8C4 source owner, a native six-word value projection.
    fn point_shape(&mut self) -> Result<Shape, String>;
    ///Actual per-caller settings+1C origin, read only after accepted raster.
    fn falloff_distance(&mut self) -> Result<u32, String>;
    fn protected_diagnostic(
        &mut self,
        cell: [i32; 2],
        area: u8,
        requested: u8,
        current: u32,
        target: u32,
        result: u32,
    ) -> Result<(), String>;
    ///Actual50C310 phase/MapEC/attached-VWorld child, after all committed stores.
    fn notify(
        &mut self,
        center: [u32; 2],
        radius: u32,
        map: &crate::battle_terrain::MapNotification,
    ) -> Result<(), String>;
}
struct Bound<'a, E> {
    terrain: HeightBrushTerrain<'a>,
    effects: &'a mut E,
}
fn error(e: impl std::fmt::Display) -> brush::Error {
    brush::Error::Service(e.to_string())
}
impl<E: Effects> height_brush_geometry::Sources for Bound<'_, E> {
    fn orientation(&mut self) -> brush::Result<[u32; 3]> {
        self.effects.orientation().map_err(error)
    }
    fn point_shape(&mut self) -> brush::Result<Shape> {
        self.effects.point_shape().map_err(error)
    }
}
impl<E: Effects> brush::Effects for Bound<'_, E> {
    fn falloff_distance(&mut self) -> brush::Result<u32> {
        self.effects.falloff_distance().map_err(error)
    }
    fn terrain_slope(&mut self, xy: [u32; 2]) -> brush::Result<Option<u32>> {
        self.terrain.slope(xy).map_err(error)
    }
    fn protected_diagnostic(
        &mut self,
        cell: [i32; 2],
        area: u8,
        requested: u8,
        current: u32,
        target: u32,
        result: u32,
    ) -> brush::Result<()> {
        self.effects
            .protected_diagnostic(cell, area, requested, current, target, result)
            .map_err(error)
    }
    fn notify(&mut self, center: [u32; 2], radius: u32) -> brush::Result<()> {
        self.effects
            .notify(center, radius, self.terrain.notification)
            .map_err(error)
    }
}
///4EB500 supplies the same currentXY twice. General moving/swept551140 and
///mode2 smoothing remain separate reached requirements in the shared kernel.
pub fn apply(
    terrain: &mut BattleTerrain,
    edit: &HeightEdit,
    effects: &mut impl Effects,
    pc: Precision,
) -> Result<bool, String> {
    let (field, state, terrain) = terrain.height_brush_parts();
    let mut geometry = height_brush_geometry::Placed {
        placement: &edit.placement,
        orientation: None,
    };
    let s = edit.settings_words;
    brush::apply(
        field,
        state,
        &mut geometry,
        edit.placement.position,
        edit.placement.position,
        Settings {
            mode: s[0],
            height: s[1],
            slope: s[2],
            minimum: s[3],
            maximum: s[4],
            protect: edit.flags[0],
            smooth_edge: edit.flags[1],
            strength: edit.strength_bits,
            area: u8::from(edit.trailing_flag),
        },
        &mut Bound { terrain, effects },
        pc,
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
#[path = "height_brush_tests.rs"]
mod tests;
