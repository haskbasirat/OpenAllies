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
//! The view-cell half of 0x4312F0 (0x42FAD0 per active block, the 0x42F670
//! tail) is `aa_runtime::view_streaming`, run on the SAME VWorld+94 VGrid and
//! On/Near lists. Its VThing leaves have no live client owner yet: [`Cells`]
//! reads the VGrid words and cells, and a cell that holds views (the only
//! case 0x42FAD0 does anything) is an explicit error naming the view owner.
use aa_numeric::{Binary, Precision};
use aa_runtime::{
    player_order_input::{terrain_map::TerrainMap, terrain_ray::Terrain},
    terrain_visibility::{CameraWords, TerrainVisibility, ViewCellHooks},
    view_grid_state,
    view_streaming::{self, Lists, Stats, TerrainBlockCell, ViewKind, Views},
};
type Result<T> = std::result::Result<T, String>;
type View = crate::live_scene::view_directory::ViewHandle;

/// B36518 and VWorld+0x94 (map, visibility, VGrid On/Near lists, B39BB8).
pub struct Owner {
    pub map: TerrainMap,
    pub vis: TerrainVisibility,
    lists: Lists<View>,
    stats: Stats,
}
pub enum Slot {
    /// Not built yet (visual heightfield not published).
    Pending,
    Built(Box<Owner>),
    /// The 0x43FE0F/0x43FE26 construction was refused (reported once).
    Failed(String),
}
impl Slot {
    pub fn map(&self) -> Option<&TerrainMap> {
        match self {
            Slot::Built(o) => Some(&o.map),
            _ => None,
        }
    }
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
    Ok(Owner { map, vis, lists: Lists::from_constructor(), stats: Stats::default() })
}

impl Owner {
    /// 0x4312F0 then 0x725340 on the SAME World, VWorld+BC heightfield and
    /// VWorld+94 VGrid, with the current camera's words.
    pub fn update_4312f0_725340(
        &mut self,
        world: &aa_runtime::live_world_frame::World<'_>,
        height: &aa_runtime::view_heightfield::State,
        grid: &view_grid_state::State<View>,
        camera: &CameraWords,
        pc: Precision,
    ) -> Result<()> {
        let mut hooks = Hooks { cells: Cells { grid }, lists: &mut self.lists, stats: &mut self.stats, planes: camera.planes, pc };
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

struct Hooks<'a, 'g> {
    cells: Cells<'g>,
    lists: &'a mut Lists<View>,
    stats: &'a mut Stats,
    /// VWorld+94 +A0..+FC as 0x42EED0 copied them this pass.
    planes: [[u32; 4]; 6],
    pc: Precision,
}
impl ViewCellHooks for Hooks<'_, '_> {
    fn sweep_42fad0(&mut self, index: [i32; 2], block_size_a0: i32, state_b8: u32) -> Result<()> {
        let block = TerrainBlockCell { column_ac: index[0], row_b0: index[1], state_b8, size_a0: block_size_a0 };
        view_streaming::sweep_42fad0(self.lists, &mut self.cells, block, &self.planes, self.pc)
    }
    fn traverse_42f670(&mut self) -> Result<()> {
        view_streaming::traverse_42f670(self.lists, self.stats, &mut self.cells)
    }
}

/// The VGrid side of `view_streaming::Views`: grid words and cell
/// occupancy. Every VThing leaf is the view owner's.
struct Cells<'g> {
    grid: &'g view_grid_state::State<View>,
}
fn view_owner<T>(leaf: &str) -> Result<T> {
    Err(format!("{leaf}: VThing leaf of the 42FAD0/42F670 view sweep has no live accessor yet (owner: view directory)"))
}
impl Views for Cells<'_> {
    type V = View;
    fn word(&mut self, _: View, offset: u32) -> Result<u32> {
        view_owner(&format!("VThing+{offset:X}"))
    }
    fn store(&mut self, _: View, offset: u32, _: u32) -> Result<()> {
        view_owner(&format!("VThing+{offset:X} store"))
    }
    fn kind(&mut self, _: View) -> Result<ViewKind> {
        view_owner("VThing vtable class")
    }
    fn attached_vfx_9c(&mut self, _: View) -> Result<bool> {
        view_owner("view+9C")
    }
    fn model_74_78(&mut self, _: View) -> Result<[u32; 2]> {
        view_owner("view+7C Model +74/+78")
    }
    fn height_431550(&mut self, _: [u32; 2]) -> Result<Binary> {
        view_owner("43D050 -> 431550")
    }
    fn grid_word(&mut self, offset: u32) -> Result<u32> {
        self.grid.word(offset)?.ok_or_else(|| format!("VGrid+{offset:X} has not been written"))
    }
    fn cell_views(&mut self, index: usize) -> Result<Vec<View>> {
        let cell = self.grid.cells()?.get(index).ok_or("42FB48 VGrid cell index is outside the allocation")?;
        if cell.is_empty() {
            return Ok(Vec::new());
        }
        view_owner(&format!("VGrid cell {index} node walk"))
    }
    fn update_0(&mut self, _: View) -> Result<()> {
        view_owner("view vt+0")
    }
    fn model_request_d0(&mut self, _: View, _: u32) -> Result<()> {
        view_owner("view vt+D0")
    }
    fn model_release_d4(&mut self, _: View) -> Result<()> {
        view_owner("view vt+D4")
    }
    fn show_e0(&mut self, _: View) -> Result<()> {
        view_owner("view vt+E0")
    }
    fn hide_e4(&mut self, _: View) -> Result<()> {
        view_owner("view vt+E4")
    }
    fn data_request_14(&mut self, _: View, _: u32, _: u32) -> Result<()> {
        view_owner("view+10 Data vt+14")
    }
    fn data_release_18(&mut self, _: View, _: u32) -> Result<()> {
        view_owner("view+10 Data vt+18")
    }
}
