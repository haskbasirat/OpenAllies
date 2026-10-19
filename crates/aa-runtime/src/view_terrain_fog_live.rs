//! Production [`super::Reads`] over the retained live World: the SAME GMap
//! tile fog words (aa-zones tile +14, kept by the live Detection tick), zone
//! sets, BattleTerrain variations, Kingdom shells (+190/+228) and SVar
//! manager (FogOfWar, CheatFog) the simulation owns, plus the UI-thread
//! words of one VWorld update ([`FrameWords`]).
use super::{FogVariables, Result, Uvar};
use crate::actor_world::Source;
use crate::live_world_frame::PresentationParts;
use aa_actors::ActorId;

/// UI-thread words of one VWorld update, read from their actual owners
/// before the pass. A failure is returned only where the native code reads
/// that word, never turned into a default.
#[derive(Clone, Debug)]
pub struct FrameWords {
    /// 0x5C0E50 (UI B3653C): Session+E4 3 -> NULL, 0x76AC70, else UI+1E0.
    pub kingdom_5c0e50: Result<Option<ActorId>>,
    /// B36550 Session+E4 (None: NULL Session).
    pub application_state_e4: Result<Option<u32>>,
    /// 0x40FCA0(2): Session+E4 == 3, else 0x40EAA0 (DevMode).
    pub debug_gate_40fca0: Result<bool>,
    /// The bytes of the UVar records DebugObjectBlocking / DebugWaterBlocking
    /// (0x4B2E60 lookups B40464 / B40460; a missing record is an error).
    pub debug_object_blocking: Result<u8>,
    pub debug_water_blocking: Result<u8>,
    /// UVar bytes 22, 25, 27, 29, 31, 100, 102.
    pub show_blocking_22: Result<u8>,
    pub settlement_debug_25: Result<u8>,
    pub guard_debug_27: Result<u8>,
    pub control_debug_29: Result<u8>,
    pub fog_debug_31: Result<u8>,
    pub terrain_grid_100: Result<u8>,
    pub alpha_transitions_102: Result<u8>,
}

/// [`super::Reads`] over one World's presentation loans.
pub struct LiveReads<'a> {
    parts: &'a PresentationParts<'a>,
    words: &'a FrameWords,
    vars: &'a FogVariables,
}
impl<'a> LiveReads<'a> {
    pub fn new(parts: &'a PresentationParts<'a>, words: &'a FrameWords, vars: &'a FogVariables) -> Self {
        Self { parts, words, vars }
    }
    fn kingdom_word(&self, k: ActorId, offset: u32) -> Result<u32> {
        kingdom_word(self.parts, k, offset)
    }
    fn xy(&self, tile: usize) -> Result<(i32, i32)> {
        let w = self.parts.zones.geometry.width;
        let t = i32::try_from(tile).map_err(|_| "GMap tile index exceeds the native integer")?;
        if w <= 0 {
            return Err("GMap +1C width is not positive".into());
        }
        Ok((t % w, t / w))
    }
}

/// Kingdom+offset from the Kingdom's derived shell in the SAME ActorStore.
pub fn kingdom_word(parts: &PresentationParts<'_>, k: ActorId, offset: u32) -> Result<u32> {
    let key = parts.actors.key(k).ok_or("local Kingdom GTID is not live")?;
    let entity = parts.actors.entity(key).map_err(|e| e.to_string())?;
    let Source::Special(saved) = &entity.source else {
        return Err("local Kingdom derived owner is absent".into());
    };
    let shell = saved.derived.as_ref().ok_or("local Kingdom derived fields are absent")?;
    shell.words.get(&offset).copied().ok_or_else(|| format!("Kingdom+{offset:X} is unwritten"))
}

impl super::Reads for LiveReads<'_> {
    type Kingdom = ActorId;
    fn debug_gate_40fca0(&mut self) -> Result<bool> {
        self.words.debug_gate_40fca0.clone()
    }
    fn debug_object_blocking(&mut self) -> Result<u8> {
        self.words.debug_object_blocking.clone()
    }
    fn debug_water_blocking(&mut self) -> Result<u8> {
        self.words.debug_water_blocking.clone()
    }
    fn uvar_byte(&mut self, slot: Uvar) -> Result<u8> {
        let w = &self.words;
        match slot {
            Uvar::ShowBlocking22 => w.show_blocking_22.clone(),
            Uvar::SettlementDebug25 => w.settlement_debug_25.clone(),
            Uvar::GuardDebug27 => w.guard_debug_27.clone(),
            Uvar::ControlDebug29 => w.control_debug_29.clone(),
            Uvar::FogDebug31 => w.fog_debug_31.clone(),
            Uvar::TerrainGrid100 => w.terrain_grid_100.clone(),
            Uvar::AlphaTransitions102 => w.alpha_transitions_102.clone(),
        }
    }
    fn tile_blocking_20(&mut self, _tile: usize) -> Result<u32> {
        Err("7203F0 blocking debug view reads GMap tile +20, which has no live owner here".into())
    }
    fn tile_zone_set_4(&mut self, tile: usize) -> Result<bool> {
        Ok(self.parts.zones.tile(tile).map_err(|e| e.to_string())?.set.is_some())
    }
    fn zone_debug_54ef80(&mut self, _tile: usize, bit: u8) -> Result<bool> {
        Err(format!("7203F0 zone debug view: 54EF80 zone-set vt+1C query (bit {bit}) is not ported"))
    }
    /// 0x40CFA0: the World (B36534) is present; 0x4F0930 reads the CheatFog
    /// byte `[[B39AB0]+1BC]`, then GWorld+275.
    fn god_40cfa0(&mut self) -> Result<bool> {
        Ok(self.parts.variables.cheat_fog_enabled() || *self.parts.runtime_fields.fog_cheat_word() != 0)
    }
    fn kingdom_5c0e50(&mut self) -> Result<Option<ActorId>> {
        self.words.kingdom_5c0e50.clone()
    }
    fn fog_of_war(&mut self) -> Result<u32> {
        Ok(self.parts.variables.fog_integer() as u32)
    }
    fn tile_fog_14(&mut self, tile: usize) -> Result<u32> {
        Ok(self.parts.zones.tile(tile).map_err(|e| e.to_string())?.fog)
    }
    fn kingdom_190(&mut self, k: ActorId) -> Result<u32> {
        self.kingdom_word(k, 0x190)
    }
    fn kingdom_228(&mut self, k: ActorId) -> Result<u32> {
        self.kingdom_word(k, 0x228)
    }
    /// The tile's Variation allocation identity (tile+0 is its pointer).
    fn tile_variation_0(&mut self, tile: usize) -> Result<u64> {
        let (x, y) = self.xy(tile)?;
        let v = self.parts.terrain.variation_definition_at(x, y).map_err(|e| e.to_string())?;
        Ok(v as *const _ as usize as u64)
    }
    fn fade_time_65(&mut self) -> Result<u32> {
        Ok(self.vars.fade_time_65)
    }
    fn application_state_e4(&mut self) -> Result<Option<u32>> {
        self.words.application_state_e4.clone()
    }
}

impl<K: Copy + Eq> super::TerrainFog<K> {
    /// [`super::TerrainFog::for_map`] for the TerrainMap built from this
    /// World (GMap+8 gtile size and the GMap tile counts).
    pub fn for_world(map: &crate::player_order_input::terrain_map::TerrainMap, parts: &PresentationParts<'_>, quad_size_ac: i32) -> Result<Self> {
        let g = parts.zones.geometry;
        if g.tile_size_bits != parts.terrain.grid.cell_size {
            return Err("zone GMap and BattleTerrain gtile sizes differ".into());
        }
        Self::for_map(map, parts.terrain.grid.cell_size, quad_size_ac, [g.width, g.height])
    }
}
