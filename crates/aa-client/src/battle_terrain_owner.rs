//! The battle TerrainMap B36518 and its VWorld+0x94 visibility owner on the
//! simulation thread, for the presentation job (AA.exe):
//!
//! - 0x43FE0F (0x721330) / 0x43FE26 (0x723E00): the map is built right after
//!   the visual heightfield publication (`TerrainMap::for_world`), and
//!   0x430290 pairs its visibility owner (`TerrainVisibility::for_map`).
//! - Per VWorld update (0x43EC60): 0x4312F0 at 0x43ED0A, then 0x725340 at
//!   0x43ED15. The orders input of a UI frame runs before them, so 0x71D840
//!   sees the previous update's block residency, as in the original.
//!
//! The view-cell half of 0x4312F0 (0x42FAD0 per active block before 0x720080,
//! the 0x42F670 tail) is the view owner's `BlockObjects`
//! (`battle_view_startup::State::stream_views_4312f0` lends its
//! `StreamingObjects` over the SAME directory, VGrid and heightfield).
use aa_numeric::Precision;
use aa_runtime::{
    player_order_input::{terrain_map::TerrainMap, terrain_ray::Terrain},
    terrain_visibility::{CameraWords, TerrainVisibility, ViewCellHooks},
    view_streaming::{BlockObjects, TerrainBlockCell},
};
type Result<T> = std::result::Result<T, String>;

/// B36518 and VWorld+0x94 (map and its terrain visibility).
pub struct Owner {
    pub map: TerrainMap,
    pub vis: TerrainVisibility,
}
pub enum Slot {
    /// Not built yet (visual heightfield not published).
    Pending,
    Built(Box<Owner>),
    /// The 0x43FE0F/0x43FE26 construction was refused (reported once).
    Failed,
}

/// 0x43FE0F / 0x43FE26 over the live owners; `quad` reads UVar0
/// TerrainQuadSize after AVar0 (0x72422B).
pub fn build_43fe0f(
    world: &aa_runtime::live_world_frame::World<'_>,
    height: &aa_runtime::view_heightfield::State,
    grid: &aa_runtime::view_grid_variables::State,
    quad: impl FnOnce() -> Result<i32>,
    pc: Precision,
) -> Result<Owner> {
    let mut map = TerrainMap::for_world(world, height, grid, quad, pc)?;
    let vis = TerrainVisibility::for_map(&mut map)?;
    Ok(Owner { map, vis })
}

impl Owner {
    /// 0x4312F0 then 0x725340 on the SAME World and VWorld+BC heightfield,
    /// with the current camera's words and the view owner's objects.
    pub fn update_4312f0_725340(
        &mut self,
        world: &aa_runtime::live_world_frame::World<'_>,
        height: &aa_runtime::view_heightfield::State,
        camera: &CameraWords,
        objects: &mut dyn BlockObjects,
        pc: Precision,
    ) -> Result<()> {
        let mut hooks = Hooks { objects, planes: camera.planes };
        self.vis.update_4312f0(&mut self.map, world, height, camera, &mut hooks, pc)?;
        self.map.update_725340()?;
        Ok(())
    }
    /// TerrainMap+0xBC (`B36518 != 0 && +BC` for 0x59A6E0 / 0x43C550).
    pub fn loaded_bc(&self) -> Result<bool> {
        Ok(Terrain::loaded(&self.map)? != 0)
    }
    /// Active visibility blocks and blocks in applied state 2 (diagnostics).
    pub fn status(&self) -> String {
        let [x, y] = self.map.blocks_xy();
        let count = (x.max(0) as usize) * (y.max(0) as usize);
        let visible = (0..count).filter(|&c| self.vis.block(c).is_some_and(|b| b.state_b8 == 2)).count();
        format!("blocks={x}x{y} active={} state2={visible}", self.vis.active().len())
    }
}

/// `ViewCellHooks` over the view owner's `BlockObjects`.
struct Hooks<'a> {
    objects: &'a mut dyn BlockObjects,
    /// VWorld+94 +A0..+FC as 0x42EED0 copied them this pass.
    planes: [[u32; 4]; 6],
}
impl ViewCellHooks for Hooks<'_> {
    fn sweep_42fad0(&mut self, index: [i32; 2], block_size_a0: i32, state_b8: u32) -> Result<()> {
        let block = TerrainBlockCell { column_ac: index[0], row_b0: index[1], state_b8, size_a0: block_size_a0 };
        self.objects.sweep_42fad0(block, &self.planes)
    }
    fn traverse_42f670(&mut self) -> Result<()> {
        self.objects.traverse_42f670()
    }
}
