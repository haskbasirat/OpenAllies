//! Fog of war over the live battle owners (AA.exe, image base 0x400000). The
//! kernels and their Unicorn oracle are in `aa_runtime::view_terrain_fog`
//! (research/phase8-fog); this module binds them to the SAME World, Session,
//! UI variables and renderer the battle presentation job already lends.
//!
//! * [`TerrainFogOwner`]: the fog fields of TerrainMap B36518 (0x721330
//!   +138/+13C/+140) and of its blocks (0x723960), stepped once per VWorld
//!   update 0x43EC60 by [`update_43ed0a`]: after 0x4312F0 (0x43ED0A) the
//!   0x42FE10 grid lifecycle (0x720670 cells, 0x720AD0 grid, 0x71CCB0 +141,
//!   0x721030 release) and the 0x7208D0 fog half per active block (0x7203F0
//!   cells, 0x71C290 blend of 0x71BEF0 samples); after 0x725340 (0x43ED15)
//!   its fog part (+138/+13C from 0x59ABA0, +140 local Kingdom, B285D4,
//!   B403C8/B403C9). The composed 0x71C290 pixels then reach
//!   `SceneRenderer::set_fog` on the UI thread through the UI bridge (the
//!   bridge Context lends the renderer), only when they change.
//! * [`frame_words`]: the Session/UI words of one update, read from their
//!   actual owners before the pass.
//! * [`LiveThing`] / [`LiveView`]: production reads of the object fog leaves
//!   (view vt+50 0x45BF20, vt+54 0x45BD40 with 0x45BBD0 and the Thing-less
//!   0x4282A0/0x42A530/0x54F3E0 path; GThing vt+5C 0x4EEF70, vt+60 0x4EAB20,
//!   vt+74, 0x4EA350, 0x4EA430; the remembered aspect choice 0x4EAE20 /
//!   0x4EACC0) for the per-frame VObject update 0x459E10.
//! * [`FogView`] implements live_battle_visibility::CurrentView and
//!   [`FogPresentation`] live_battle_scene::Presentation.
//!
//! The call sites are in other owners' files (battle_terrain_owner.rs and
//! battle_presentation.rs); until they land these items are unused.
#![allow(dead_code)]
use crate::live_scene::battle::{Appearance, Presentation, native::{CurrentView, NativeVisibility}};
use aa_actors::{ActorId, detection_memory::RecordId};
use aa_netcode::local_bootstrap::world_ready::ActualActivatedSession;
use aa_numeric::Precision;
use aa_runtime::{
    actor_world::{ActorStore, PlacementState, RuntimeObject},
    live_world_frame::{PresentationParts, World},
    player_order_input::terrain_map::TerrainMap,
    simulation_execution::Execution,
    spatial_world::SpatialWorld,
    terrain_visibility::TerrainVisibility,
    view_terrain_fog::{
        FogImage, FogVariables, Globals, TerrainFog,
        live::{FrameWords, LiveReads, kingdom_word},
        object::{self, Apparent, GMapWords, ThingReads, ViewReads, ViewWords},
    },
};
type Result<T> = std::result::Result<T, String>;

// ------------------------------------------------------- frame words --

/// UVar records read by name (the 0x4B2E60 lookups and the registered
/// `[[B39940]+4*slot]` cells), in this order.
const UVAR_NAMES: [&str; 10] = [
    "DebugObjectBlocking",
    "DebugWaterBlocking",
    "ViewShowBlocking",
    "ViewShowSettlementDebug",
    "ViewShowGuardDebug",
    "ViewShowControlDebug",
    "ViewShowFogDebug",
    "RenderDrawTerrainGrid",
    "RenderDrawAlphaTransitions",
    "PerformanceDisableDoodads",
];

/// The Session/UI words of one VWorld update from their actual owners: the
/// activated Session (B36550 E4, Node+8, 76AC70), `ui_kingdom_1e0` (UI
/// B3653C +1E0, read by 0x5C0E50 only when 0x76AC70 is NULL), the UI-thread
/// UVar manager (one bridge call for every byte; the reads are pure, and
/// each failure is returned only where the native code reads that byte) and
/// 0x40EAA0 (DevMode, reached only when Session+E4 != 3).
pub fn frame_words(
    session: &ActualActivatedSession,
    ui_kingdom_1e0: Result<Option<ActorId>>,
    bridge: &crate::battle_ui_bridge::Client,
    execution: &mut Execution<'_>,
) -> FrameWords {
    let e4 = session.session_state_e4();
    // 0x5C0E50: 0x40CEC0 (E4 == 3) -> NULL; 0x76AC70 when non-NULL; else
    // UI+1E0. 0x76AC70 is NULL for E4 == 3 or a NULL Node+8 local player.
    let kingdom_5c0e50 = if e4 == 3 {
        Ok(None)
    } else {
        match session.local_player_kingdom_76ac70() {
            Ok(Some(k)) => Ok(Some(k)),
            Ok(None) => ui_kingdom_1e0,
            Err(e) => Err(format!("5C0E50 76AC70: {e}")),
        }
    };
    // 0x40FCA0(B39690, 2, ...): bit 1 passes with E4 == 3, else needs 0x40EAA0.
    let debug_gate_40fca0 = if e4 == 3 {
        Ok(true)
    } else {
        session.host().can_process_40eaa0(|| bridge.call(&mut *execution, |c| c.ui.uvars.dev_mode_current()))
    };
    let uvars = bridge.call(&mut *execution, |c| Ok(UVAR_NAMES.map(|name| c.ui.uvars.view_bool_raw(name))));
    let byte = |index: usize| -> Result<u8> {
        let values = uvars.as_ref().map_err(|e| format!("UVar manager (UI thread): {e}"))?;
        values[index].ok_or_else(|| format!("UVar {} has no record; the native read dereferences its NULL cell", UVAR_NAMES[index]))
    };
    FrameWords {
        kingdom_5c0e50,
        application_state_e4: Ok(Some(e4)),
        debug_gate_40fca0,
        debug_object_blocking: byte(0),
        debug_water_blocking: byte(1),
        show_blocking_22: byte(2),
        settlement_debug_25: byte(3),
        guard_debug_27: byte(4),
        control_debug_29: byte(5),
        fog_debug_31: byte(6),
        terrain_grid_100: byte(7),
        alpha_transitions_102: byte(8),
        // 0x5C0E10: the activated Session is B36550, then 0x76AC60.
        local_player_5c0e10: Ok(session.local_player_present_76ac60()),
        performance_disable_doodads: uvars.as_ref().map(|v| v[9]).map_err(|e| format!("UVar manager (UI thread): {e}")),
    }
}

// ------------------------------------------------------- terrain fog --

/// The fog half of the B36518 TerrainMap owner. A refused construction is
/// retained and reported at every update; the TerrainMap itself is unaffected.
pub enum TerrainFogSlot {
    Built(Box<TerrainFogOwner>),
    Failed(String),
}
impl TerrainFogSlot {
    /// With the TerrainMap (0x43FE0F / 0x43FE26) on the SAME World.
    pub fn for_map(map: &TerrainMap, world: &World<'_>) -> Self {
        match TerrainFogOwner::for_map(map, world) {
            Ok(owner) => Self::Built(Box::new(owner)),
            Err(error) => Self::Failed(format!("terrain fog 721330/723960: {error}")),
        }
    }
    pub fn owner_mut(&mut self) -> Result<&mut TerrainFogOwner> {
        match self {
            Self::Built(owner) => Ok(owner),
            Self::Failed(error) => Err(error.clone()),
        }
    }
    pub fn status(&self) -> String {
        match self {
            Self::Built(owner) => owner.status(),
            Self::Failed(error) => format!("fog refused: {error}"),
        }
    }
}

pub struct TerrainFogOwner {
    fog: TerrainFog<ActorId>,
    globals: Globals,
    vars: FogVariables,
    /// The image revision last handed to the renderer.
    published: Option<u64>,
}
impl TerrainFogOwner {
    /// 0x721330 (+138/+13C/+140 = 0) and 0x723960 (block fog fields) for the
    /// map built from this World; B403C8/B403C9/B285D4 are the process
    /// image's; the three AVars are the World's Settings projection.
    pub fn for_map(map: &TerrainMap, world: &World<'_>) -> Result<Self> {
        let parts = world.presentation_parts()?;
        Ok(Self {
            fog: TerrainFog::for_built_world(map, &parts)?,
            globals: Globals::from_fresh_process_image(),
            vars: FogVariables::from_settings(parts.settings)?,
            published: None,
        })
    }
    /// The fog part of 0x4312F0's 0x42FF50 pass, right after
    /// `TerrainVisibility::update_4312f0` succeeded on the SAME map and World.
    /// `quad_size_uvar0` is the current UVar0 TerrainQuadSize, read by
    /// 0x720AD0 only when a block's grid is created.
    pub fn after_4312f0(
        &mut self,
        vis: &TerrainVisibility,
        map: &TerrainMap,
        world: &World<'_>,
        words: &FrameWords,
        quad_size_uvar0: &mut dyn FnMut() -> Result<i32>,
        pc: Precision,
    ) -> Result<()> {
        let parts = world.presentation_parts()?;
        let mut reads = LiveReads::new(&parts, words, &self.vars);
        self.fog.after_42ff50(vis, map, &self.globals, &self.vars, quad_size_uvar0, &mut reads, pc)
    }
    /// The fog part of 0x725340 (0x43ED15). `milliseconds_59aba0` is
    /// 0x59ABA0 on the global application timer B396E0.
    pub fn after_725340(&mut self, words: &FrameWords, milliseconds_59aba0: Result<u32>, pc: Precision) -> Result<()> {
        let fog_debug_31 = words.fog_debug_31.clone();
        self.fog.clock.update_725340(
            &mut self.globals,
            milliseconds_59aba0,
            words.kingdom_5c0e50.clone(),
            self.vars.gray_percentage_64,
            fog_debug_31,
            pc,
        )
    }
    /// The composed image when it changed since the last [`Self::mark_published`].
    pub fn changed_image(&self) -> Result<Option<FogImage>> {
        let revision = self.fog.revision();
        if self.published == Some(revision) {
            return Ok(None);
        }
        self.fog.compose().map(Some)
    }
    pub fn mark_published(&mut self, revision: u64) {
        self.published = Some(revision);
    }
    /// Hands a changed image to `SceneRenderer::set_fog` on the UI thread.
    /// Returns whether it was sent.
    pub fn publish(&mut self, bridge: &crate::battle_ui_bridge::Client, execution: &mut Execution<'_>) -> Result<bool> {
        let Some(image) = self.changed_image()? else { return Ok(false) };
        let revision = image.revision;
        bridge.call(execution, move |c| c.renderer.set_fog(Some(&image)).map_err(|e| format!("SceneRenderer::set_fog: {e:#}")))?;
        self.mark_published(revision);
        Ok(true)
    }
    pub fn fog(&self) -> &TerrainFog<ActorId> {
        &self.fog
    }
    pub fn globals(&self) -> &Globals {
        &self.globals
    }
    pub fn status(&self) -> String {
        let grids = self.fog.blocks.iter().filter(|b| b.grid.is_some()).count();
        format!(
            "fog grids={grids} revision={} b285d4={} b403c8={} kingdom_140={:?}",
            self.fog.revision(),
            self.globals.explored_b285d4,
            self.globals.fog_debug_b403c8,
            self.fog.clock.kingdom_140.map(|k| k.raw())
        )
    }
}

/// One VWorld update's fog steps, called right after 0x4312F0 and 0x725340
/// ran on `map`/`vis` (43EC60 at 0x43ED0A..0x43ED15), then the renderer
/// hand-off. Nothing here runs when the terrain pass itself was refused.
#[allow(clippy::too_many_arguments)]
pub fn update_43ed0a(
    slot: &mut TerrainFogSlot,
    vis: &TerrainVisibility,
    map: &TerrainMap,
    session: &ActualActivatedSession,
    ui_kingdom_1e0: Result<Option<ActorId>>,
    milliseconds_59aba0: Result<u32>,
    bridge: &crate::battle_ui_bridge::Client,
    execution: &mut Execution<'_>,
    pc: Precision,
) -> Result<()> {
    let owner = slot.owner_mut()?;
    let words = frame_words(session, ui_kingdom_1e0, bridge, execution);
    let mut quad = || bridge.call(&mut *execution, |c| c.ui.uvars.terrain_quad_size_current());
    owner.after_4312f0(vis, map, session.world(), &words, &mut quad, pc)?;
    owner.after_725340(&words, milliseconds_59aba0, pc)?;
    owner.publish(bridge, execution)?;
    Ok(())
}

// ------------------------------------------------------ object reads --

/// Data vt+38 (ActorData) / vt+40 (DynamicDoodadData) and the Data+18
/// category word +0C that 0x4BB3C0 compares (the registry index).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DataVirtuals {
    pub actor_38: bool,
    pub doodad_40: bool,
    pub category_0c: u32,
}
impl DataVirtuals {
    /// By native data kind, from each Data class's vtable (the same slots
    /// research/skirmish-20261019-view-order-world proved): ThingData
    /// A20770 / ObjectData A13318 vt+38 and vt+40 4C9FA0 (0); ActorData
    /// A05540 vt+38 4F0500 (1); DynamicDoodadData A0F6E0 vt+40 4F0500 (1).
    pub fn for_category(category: &aa_rules::native::CategoryDescriptor) -> Result<Self> {
        let (actor_38, doodad_40) = match category.native_kind {
            0 | 1 => (false, false),
            2 => (true, false),
            21 => (false, true),
            k => return Err(format!("Data vt+38/+40 of native data kind {k} is not proven")),
        };
        Ok(Self { actor_38, doodad_40, category_0c: category.registry_index })
    }
}

fn object_of<'p>(parts: &'p PresentationParts<'_>, t: ActorId) -> Result<&'p RuntimeObject> {
    let key = parts.actors.key(t).ok_or_else(|| format!("Thing GTID {:#x} is not live", t.raw()))?;
    parts.actors.object(key).map_err(|e| e.to_string())
}

/// One GThing's leaves over the SAME World owners and frame words.
pub struct LiveThing<'a> {
    pub parts: &'a PresentationParts<'a>,
    pub words: &'a FrameWords,
    pub thing: ActorId,
}
impl LiveThing<'_> {
    fn object(&self) -> Result<&RuntimeObject> {
        object_of(self.parts, self.thing)
    }
    /// The 0x4F9C10 runtime classes whose vtables carry these slots (class 5
    /// Team has a short vtable without them).
    fn class(&self) -> Result<u32> {
        match self.object()?.definition.native_class {
            c @ (2 | 3 | 4 | 6 | 7 | 8 | 9 | 10) => Ok(c),
            c => Err(format!("Thing runtime class {c} has no proven 4F9C10 vtable for the fog leaves")),
        }
    }
    fn data(&self) -> Result<DataVirtuals> {
        DataVirtuals::for_category(&self.object()?.definition.proof.category)
    }
    fn memory(&self) -> Result<&aa_actors::detection_memory::State> {
        Ok(&self.object()?.object.as_ref().ok_or("GObject fields (+58/+5C detection memory) are absent")?.memory)
    }
    fn fog_view(&self) -> FogView<'_> {
        FogView { parts: self.parts, words: self.words }
    }
}
impl ThingReads for LiveThing<'_> {
    type Kingdom = ActorId;
    type Record = RecordId;
    /// 0x4EEF70 (vt+5C of every supported class): dead (Thing+8 bit 7) is
    /// false; on MainThread (the VWorld update runs in the UI frame 0x5C92D0
    /// that App 0x410E70 runs from MainThread, the B364D4 object, so the
    /// 0x4EEF8A comparison is equal) Session+E4 == 3 or 0x4F0930 is true;
    /// else vt+70.
    fn visible_5c(&mut self, k: Option<ActorId>, a: bool) -> Result<bool> {
        self.class()?;
        if self.flags_8()? & (1 << 7) != 0 {
            return Ok(false);
        }
        let e4 = self.words.application_state_e4.clone()?.ok_or("4EEF9C reads Session+E4 of a NULL B36550 (native NULL dereference)")?;
        if e4 == 3 || self.parts.variables.cheat_fog_enabled() || *self.parts.runtime_fields.fog_cheat_word() != 0 {
            return Ok(true);
        }
        NativeVisibility::new(&self.fog_view()).visible_to(self.parts.actors, self.thing, k, a).map_err(|e| format!("{e:#}"))
    }
    fn remembered_60(&mut self, k: Option<ActorId>, a: bool) -> Result<bool> {
        self.class()?;
        object::remembered_4eab20(self, k, a)
    }
    fn explored_74(&mut self, k: Option<ActorId>, a: bool) -> Result<bool> {
        NativeVisibility::new(&self.fog_view()).explored_by(self.parts.actors, self.thing, k, a).map_err(|e| format!("{e:#}"))
    }
    fn flags_8(&mut self) -> Result<u32> {
        let key = self.parts.actors.key(self.thing).ok_or("Thing GTID is not live")?;
        Ok(self.parts.actors.placement(key, self.parts.spatial).map_err(|e| e.to_string())?.flags)
    }
    fn data_actor_38(&mut self) -> Result<bool> {
        Ok(self.data()?.actor_38)
    }
    fn data_doodad_40(&mut self) -> Result<bool> {
        Ok(self.data()?.doodad_40)
    }
    fn data_mask_174(&mut self) -> Result<u32> {
        self.object()?.definition.component_mask.ok_or_else(|| "ActorData+174 component mask is absent".into())
    }
    fn data_16d(&mut self) -> Result<u8> {
        let id = self.object()?.definition.proof.id.clone();
        self.parts.source_fields.byte(&id, 0x16d).map_err(|e| e.to_string())
    }
    /// vt+A8: 0x6A71D0 (true) for the Kingdom class 4 only.
    fn vt_a8(&mut self) -> Result<bool> {
        Ok(self.class()? == 4)
    }
    /// vt+8C 0x4EA3D0: Thing+58 > 0.
    fn vt_8c(&mut self) -> Result<bool> {
        Ok(self.memory()?.count() > 0)
    }
    fn kingdom_190(&mut self, k: ActorId) -> Result<u32> {
        kingdom_word(self.parts, k, 0x190)
    }
    fn kingdom_18c(&mut self, k: ActorId) -> Result<u32> {
        kingdom_word(self.parts, k, 0x18c)
    }
    fn memory_slot_5c(&mut self, index: u32) -> Result<Option<RecordId>> {
        self.memory()?.slot(index).map_err(|e| e.to_string())
    }
}

/// The VObject words the two view virtuals read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectView {
    /// view+14 (None: NULL).
    pub thing_14: Option<ActorId>,
    /// view+10 Data.
    pub data: DataVirtuals,
    /// view+54.
    pub flags_54: u32,
    /// view+38/+3C (read only by the Thing-less path; None: not lent).
    pub position_38: Option<[u32; 2]>,
}
impl ObjectView {
    fn words(&self) -> ViewWords {
        ViewWords { thing_14: self.thing_14.is_some(), flags_54: self.flags_54, position_38: self.position_38 }
    }
}

/// One view's reads over the SAME World owners and frame words.
pub struct LiveView<'a> {
    pub parts: &'a PresentationParts<'a>,
    pub words: &'a FrameWords,
    pub view: ObjectView,
}
impl LiveView<'_> {
    fn thing(&self) -> Result<LiveThing<'_>> {
        let thing = self.view.thing_14.ok_or("view+14 GThing is NULL")?;
        Ok(LiveThing { parts: self.parts, words: self.words, thing })
    }
}
impl ViewReads for LiveView<'_> {
    type Kingdom = ActorId;
    fn doodads_45bbd0(&mut self) -> Result<bool> {
        let words = self.words;
        let view = self.view;
        let thing = view.thing_14.map(|t| LiveThing { parts: self.parts, words, thing: t });
        object::doodads_45bbd0(
            &mut || {
                words.performance_disable_doodads.clone()?.ok_or_else(|| {
                    "4B2E60 found no UVar PerformanceDisableDoodads record; 45BC20 dereferences its NULL result".to_string()
                })
            },
            &mut || match &thing {
                Some(t) => {
                    let key = t.parts.actors.key(t.thing).ok_or("Thing GTID is not live")?;
                    Ok(Some(t.parts.actors.placement(key, t.parts.spatial).map_err(|e| e.to_string())?.flags))
                }
                None => Ok(None),
            },
            &mut || Ok(view.data.category_0c),
            &mut || Ok(view.data.doodad_40),
        )
    }
    /// 0x40CFA0: the World (B36534) is present; 0x4F0930 reads the CheatFog
    /// byte `[[B39AB0]+1BC]`, then GWorld+275.
    fn god_40cfa0(&mut self) -> Result<bool> {
        Ok(self.parts.variables.cheat_fog_enabled() || *self.parts.runtime_fields.fog_cheat_word() != 0)
    }
    fn local_player_5c0e10(&mut self) -> Result<bool> {
        self.words.local_player_5c0e10.clone()
    }
    fn kingdom_5c0e50(&mut self) -> Result<Option<ActorId>> {
        self.words.kingdom_5c0e50.clone()
    }
    fn thing_visible_5c(&mut self, k: ActorId, a: bool) -> Result<bool> {
        self.thing()?.visible_5c(Some(k), a)
    }
    fn thing_remembered_60(&mut self, k: ActorId, a: bool) -> Result<bool> {
        self.thing()?.remembered_60(Some(k), a)
    }
    fn gmap(&mut self) -> Result<GMapWords> {
        GMapWords::from_zones(self.parts.zones)
    }
    fn fog_of_war(&mut self) -> Result<u32> {
        Ok(self.parts.variables.fog_integer() as u32)
    }
    fn tile_fog_14(&mut self, tile: usize) -> Result<u32> {
        Ok(self.parts.zones.tile(tile).map_err(|e| e.to_string())?.fog)
    }
    fn kingdom_190(&mut self, k: ActorId) -> Result<u32> {
        kingdom_word(self.parts, k, 0x190)
    }
    fn kingdom_228(&mut self, k: ActorId) -> Result<u32> {
        kingdom_word(self.parts, k, 0x228)
    }
}

/// View vt+50 0x45BF20(a).
pub fn visible_vf50(parts: &PresentationParts<'_>, words: &FrameWords, view: ObjectView, a: bool) -> Result<bool> {
    let v = view.words();
    object::visible_45bf20(&mut LiveView { parts, words, view }, &v, a)
}
/// View vt+54 0x45BD40(a).
pub fn remembered_vf54(parts: &PresentationParts<'_>, words: &FrameWords, view: ObjectView, a: bool) -> Result<bool> {
    let v = view.words();
    object::remembered_45bd40(&mut LiveView { parts, words, view }, &v, a)
}
/// 0x45C0F0's show decision (after its +60 handle refresh).
pub fn shown_45c0f0(parts: &PresentationParts<'_>, words: &FrameWords, view: ObjectView) -> Result<bool> {
    let v = view.words();
    object::shown_45c0f0(&mut LiveView { parts, words, view }, &v)
}
/// 0x4EAE20(k) / 0x4EACC0(k) on `thing` (0x459E10 passes 0x5C0E50()). A
/// changed result than view+B4 triggers vt+10C in 0x459E10 (the view owner's).
pub fn apparent_4eae20(parts: &PresentationParts<'_>, words: &FrameWords, thing: ActorId, k: Option<ActorId>) -> Result<Apparent<RecordId>> {
    object::apparent_4eae20(&mut LiveThing { parts, words, thing }, k)
}

// ------------------------------------------- CurrentView / Presentation --

/// live_battle_visibility::CurrentView over the SAME World owners and the
/// update's frame words.
pub struct FogView<'a> {
    pub parts: &'a PresentationParts<'a>,
    pub words: &'a FrameWords,
}
impl CurrentView for FogView<'_> {
    fn kingdom(&self) -> anyhow::Result<Option<ActorId>> {
        self.words.kingdom_5c0e50.clone().map_err(anyhow::Error::msg)
    }
    /// `[[B39AB0]+108]` FogOfWar != 0.
    fn fog_enabled(&self) -> anyhow::Result<bool> {
        Ok(self.parts.variables.fog_enabled())
    }
    /// 0x4EEF8A's GetCurrentThreadId() == [B364D4]+48 for the VWorld update
    /// callers (UI frame on MainThread): equal.
    fn current_thread_is_simulation(&self) -> anyhow::Result<bool> {
        Ok(true)
    }
    fn application_state(&self) -> anyhow::Result<u32> {
        self.words
            .application_state_e4
            .clone()
            .map_err(anyhow::Error::msg)?
            .ok_or_else(|| anyhow::anyhow!("4EEF9C reads Session+E4 of a NULL B36550 (native NULL dereference)"))
    }
    /// 0x4F0930's first read, the CheatFog byte `[[B39AB0]+1BC]`.
    fn editor_option_enabled(&self) -> anyhow::Result<bool> {
        Ok(self.parts.variables.cheat_fog_enabled())
    }
    /// 0x4F0930's second read, GWorld+275.
    fn world_editor_override(&self) -> anyhow::Result<bool> {
        Ok(*self.parts.runtime_fields.fog_cheat_word() != 0)
    }
}

/// live_battle_scene::Presentation: `visible` is the 0x45C0F0 decision for
/// the actor's own Thing+C view (an actor without a view is not drawn by
/// the original); the replacement colour is the view's +C8 (VActor vt+70
/// 0x451F00 copies Kingdom+1E8 Color +18..+20, or AVar30
/// ReplacementColorDefault for a NULL owner, only while 0x454780's vt+58
/// holds; VObject vt+70 0x943800 is `ret 4`), which the view field owner
/// retains: it is read through `team_color`.
pub struct FogPresentation<'a> {
    pub parts: &'a PresentationParts<'a>,
    pub words: &'a FrameWords,
    pub directory: &'a crate::live_scene::view_directory::ViewDirectory,
    pub team_color: &'a dyn Fn(ActorId) -> anyhow::Result<Option<[f32; 3]>>,
}
impl Presentation for FogPresentation<'_> {
    fn appearance(
        &self,
        actors: &ActorStore,
        spatial: &SpatialWorld,
        actor: ActorId,
        object: &RuntimeObject,
        _placement: PlacementState,
    ) -> anyhow::Result<Appearance> {
        anyhow::ensure!(
            std::ptr::eq(actors, self.parts.actors) && std::ptr::eq(spatial, self.parts.spatial),
            "fog presentation reads another ActorStore/spatial owner than its World"
        );
        let Some(flags_54) = self.directory.flags_for_current(actors, actor)? else {
            return Ok(Appearance { visible: false, team_color: None });
        };
        let view = ObjectView {
            thing_14: Some(actor),
            data: DataVirtuals::for_category(&object.definition.proof.category).map_err(anyhow::Error::msg)?,
            flags_54,
            position_38: None,
        };
        let visible = shown_45c0f0(self.parts, self.words, view).map_err(anyhow::Error::msg)?;
        let team_color = if visible { (self.team_color)(actor)? } else { None };
        Ok(Appearance { visible, team_color })
    }
}
