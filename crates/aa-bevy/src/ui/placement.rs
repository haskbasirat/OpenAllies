//! Placement mode after Unpack or a construct page item (the game
//! interface's placement state, `0x622800` on the SharedUIStates placement
//! base `0x6894B0`; ghost list `0x693AE0` / `0x693D10` / `0x693EF0`; the
//! facing `0x688AA0` / `0x688990`).
//!
//! * PerformActorCommand (`0x612A70`) enters it for both: Unpack (command
//!   11) through the state's virtual +0x5C (`0x6212D0`) with the truck's
//!   unpacked structure (ActorData +0x534); a construct item (command 4,
//!   behaviour flag 0x10000, the item chosen) through virtual +0x58
//!   (`0x6204C0` -> +0x5C with the item, no truck). `0x6212D0` makes the
//!   ghost (`0x689150` -> `0x688DC0`) and starts the HUD's impact preview
//!   (with a truck `0x602A80`, without one `0x603B50` -> `0x603A00`).
//! * While it is armed, a ghost of the structure follows the cursor on the
//!   ground.
//!   - Every update (`0x688860`) it is moved there (`0x693D10`: a
//!     tile_locked building snaps to the half-cell grid, `0x46EE00`; an
//!     axis_locked one faces native -Y) and its placement is tested.
//!   - Legal: the plain ghost (AVar VFXGhosting `ghost_vfx`:
//!     VFXAlphaModifier alpha 0.5).
//!   - Illegal: VFXIllegalPlacement `illegal_placement_vfx` (VFXTintModifier
//!     colour 1,0,0, instantaneous) is added (`0x693A10`) and removed again
//!     once it is legal (`0x693A80`).
//! * The facing starts at native (0, -1) (`0x6894B0`: facing y = -1.0).
//! * A left press on the world (`0x6887C0`) at an illegal spot does not
//!   place: the placement error is reported and the mode stays. At a legal
//!   spot it fixes the ghost at the press point. Dragging further than the
//!   AVar drag threshold (`0x688AA0`, Manhattan pixels) shows VFXSelectFacing
//!   (`facing_arrow_vfx`) at the press point and turns the ghost to face the
//!   cursor (`0x688990`). Releasing places the building with that facing:
//!   the commit (`0x621350`) sends the stored command with the ghost's point
//!   and, the command's behaviour flag 0x20 set (unpack, construct), its
//!   facing (Command::Deploy for the truck, Command::Construct for the
//!   selection), then leaves the state.
//! * A right press cancels the mode.
//! * The legality test is the ghost list's (`0x6202C0` -> `0x6887A0` ->
//!   `0x693EF0`) for both: the structure's footprint fits there. For Unpack
//!   the truck must also be able to drive there (the checks Command::Deploy
//!   makes).
//!
//! The ghost and the arrow are drawn by the battle view (effects owner)
//! from [`Ghost`].
use aa_play::{ActorId, DefId, KingdomId};
use bevy::prelude::*;

/// The facing a placement starts with (`0x6894B0`: native (0, -1)).
pub const DEFAULT_FACING: f32 = -std::f32::consts::FRAC_PI_2;

/// The placement in progress (input owner).
#[derive(Resource, Default, Debug)]
pub struct Placement {
    pub active: Option<Active>,
}

#[derive(Clone, Debug)]
pub struct Active {
    /// The unpacking truck (Unpack); None for a construct item, which the
    /// selection builds.
    pub truck: Option<ActorId>,
    pub building: DefId,
    /// Ghost position (native ground point, after the placement snap) and
    /// facing (rad, 0 = native +X).
    pub at: Vec2,
    pub facing: f32,
    /// The placement is legal there (`0x693EF0`).
    pub legal: bool,
    /// The refusal reason of an illegal spot.
    pub problem: Option<String>,
    /// Left press at a legal spot: window pixel and ground point.
    pub press: Option<(Vec2, Vec2)>,
    /// Dragged past the threshold: the facing follows the cursor.
    pub turning: bool,
}

/// What the battle view draws for the placement this frame.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct Ghost {
    pub model: Option<GhostModel>,
    /// VFXSelectFacing (`facing_arrow_vfx`, a projected UI decal,
    /// FacingArrow/FacingArrow.tga, texture_size 2) at the press point
    /// (native), turned to the facing, while the facing is dragged.
    pub arrow: Option<(Vec3, f32)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GhostModel {
    /// The building definition (its `model`, drawn in `kingdom`'s colour).
    pub def: DefId,
    pub kingdom: KingdomId,
    /// Native position (z = ground height) and facing.
    pub at: Vec3,
    pub facing: f32,
    /// `ghost_vfx`: alpha 0.5.
    pub alpha: f32,
    /// `illegal_placement_vfx`: tinted (1, 0, 0) while illegal.
    pub illegal: bool,
}

/// The legality test of a spot: the building's footprint fits there and,
/// for Unpack, the truck can drive there to unpack (the checks
/// Command::Deploy makes).
pub fn problem(world: &aa_play::World, truck: Option<ActorId>, building: DefId, at: Vec2) -> Option<String> {
    if let Err(e) = aa_play::deploy::spot_clear(world, building, at, truck) {
        return Some(e);
    }
    truck.and_then(|t| aa_play::movement::deploy_spot_problem(world, t, at))
}

/// The ghost of this frame from the placement state.
pub fn ghost_of(world: &aa_play::World, local: KingdomId, p: &Placement) -> Ghost {
    let Some(a) = &p.active else { return Ghost::default() };
    let z = world.map.height_at(a.at);
    let model = GhostModel { def: a.building, kingdom: local, at: Vec3::new(a.at.x, a.at.y, z), facing: a.facing, alpha: 0.5, illegal: !a.legal };
    let arrow = a.press.filter(|_| a.turning).map(|(_, g)| (Vec3::new(g.x, g.y, world.map.height_at(g)), a.facing));
    Ghost { model: Some(model), arrow }
}
