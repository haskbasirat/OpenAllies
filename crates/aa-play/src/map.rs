//! The battle map: heights, passability and speed limits per cell, start
//! locations, the original placed things and the map's player slots, all from
//! the original RMP (aa-mapworld) and its extracted JSON (aa-assets).
//! Owner: core.
use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Cell flags, same bits as the original flags20 word.
pub const BLOCKED: u32 = 0x1;
pub const NOT_LAND_PASSABLE: u32 = 0x100;
pub const NOT_WATER_PASSABLE: u32 = 0x200;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlacedThing {
    pub thing_id: String,
    pub position: [f32; 3],
    pub yaw: f32,
    /// Map kingdom slot that owns it, if any.
    pub owner_slot: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapSlot {
    pub nation: Option<String>,
    pub team: u32,
    pub color: Option<u32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GameMap {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub cell_size: f32,
    /// (width+1)*(height+1) corner heights, row-major.
    pub heights: Vec<f32>,
    /// width*height cell flags (BLOCKED, NOT_LAND_PASSABLE, ...).
    pub flags: Vec<u32>,
    /// width*height land speed limits (m/s) from the terrain variations.
    pub speed_limit: Vec<f32>,
    pub start_locations: Vec<Vec2>,
    pub placed: Vec<PlacedThing>,
    pub slots: Vec<MapSlot>,
}

impl GameMap {
    /// Implemented by the core owner.
    pub fn load(game_root: &std::path::Path, map_id: &str) -> anyhow::Result<Self> {
        let _ = (game_root, map_id);
        anyhow::bail!("GameMap::load is not implemented yet")
    }
    pub fn cell_of(&self, p: Vec2) -> Option<(u32, u32)> {
        let (x, y) = ((p.x / self.cell_size).floor(), (p.y / self.cell_size).floor());
        (x >= 0.0 && y >= 0.0 && (x as u32) < self.width && (y as u32) < self.height).then_some((x as u32, y as u32))
    }
    pub fn cell_flags(&self, p: Vec2) -> u32 {
        self.cell_of(p).map_or(BLOCKED, |(x, y)| self.flags[(y * self.width + x) as usize])
    }
    /// Bilinear ground height at a native ground point.
    pub fn height_at(&self, p: Vec2) -> f32 {
        if self.heights.is_empty() {
            return 0.0;
        }
        let gx = (p.x / self.cell_size).clamp(0.0, self.width as f32);
        let gy = (p.y / self.cell_size).clamp(0.0, self.height as f32);
        let (x0, y0) = (gx.floor() as u32, gy.floor() as u32);
        let (x1, y1) = ((x0 + 1).min(self.width), (y0 + 1).min(self.height));
        let (tx, ty) = (gx - x0 as f32, gy - y0 as f32);
        let h = |x: u32, y: u32| self.heights[(y * (self.width + 1) + x) as usize];
        (h(x0, y0) * (1.0 - tx) + h(x1, y0) * tx) * (1.0 - ty) + (h(x0, y1) * (1.0 - tx) + h(x1, y1) * tx) * ty
    }
}
