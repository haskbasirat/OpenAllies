//! GWorld::Load `4FC600` after the PostLoad pass ([`crate::post_load`]),
//! `4FD3F3..4FD47C`, before the class-changed replacement tail (conversion
//! agent, research/phase8-conversion):
//! 1. world start `4F1E10` when map settings `[GWorld+28]+A8` is 1 or 4 and
//!    `59E520(settings)` is set; `4F1E10` itself is
//!    `aa_economy::world_start::world_start` (economy agent; runs once,
//!    GWorld `+274`);
//! 2. VWorldState::InitForLoadedMap: the local kingdom `4F5290`, the look-at
//!    point `4F1C20`, GWorld `+22C/+230`, and the `404F60` log
//!    "[4610] VWorldState::InitForLoadedMap, lookat=(%g,%g)".
//!
//! Oracle: research/phase8-saved-components/native-load-tail.json
//! (native_load_tail.py).
use crate::post_load::{Error, Id, Result};
use aa_numeric::{Binary, Precision};

/// `404F60(B38B70, A2AD90, 0x100, A2B3F8, A2B440, x, y)`.
pub const LOOK_AT_FUNCTION: &str = "KKC_VWorldState::InitForLoadedMap";
pub const LOOK_AT_FORMAT: &str = "[4610] VWorldState::InitForLoadedMap, lookat=(%g,%g)";
pub const LOOK_AT_LINE: u32 = 0x100;

/// Services of `4FD3F3..4FD47C`.
pub trait LoadTailWorld {
    /// A net session player (`[B36558]+8`).
    type Player;
    /// `[GWorld+28]+A8` (the map settings mode).
    fn map_settings_mode(&self) -> Result<u32>;
    /// `59E520` on the map settings (`[[+90]+4]` -> `511860`: a byte of the
    /// `[B3B198]` table).
    fn world_start_flag(&mut self) -> Result<bool>;
    /// `4F1E10` (`aa_economy::world_start::world_start`).
    fn world_start(&mut self) -> Result<()>;
    /// `[B36550]+E4` (application state; 3 = no local kingdom).
    fn app_state(&self) -> Result<u32>;
    /// GWorld `+48`.
    fn world_word_48(&self) -> Result<u32>;
    /// `76AC60`: `[B36558]+8`.
    fn local_player(&self) -> Result<Option<Self::Player>>;
    /// Player `+24` (name CString).
    fn player_name(&self, player: &Self::Player) -> Result<Vec<u16>>;
    /// Player `+28` (`765E00` with a non-null GWorld).
    fn player_kingdom(&self, player: &Self::Player) -> Result<Option<Id>>;
    /// GWorld `+148/+14C` (null entries as None).
    fn world_kingdom_slots(&self) -> Result<Vec<Option<Id>>>;
    /// Object `+18` (`4BEDF0`: the name CString).
    fn object_name_18(&self, object: Id) -> Result<Vec<u16>>;
    /// `[GWorld+30]+C/+10` (binary32 bits).
    fn map_size_bits(&self) -> Result<[u32; 2]>;
    /// Kingdom `+284` list (actor GTIDs, head first).
    fn kingdom_actors(&self, kingdom: Id) -> Result<Vec<Id>>;
    /// Kingdom `+1F8` (None: null).
    fn kingdom_word_1f8(&self, kingdom: Id) -> Result<Option<Id>>;
    /// Object `+8`.
    fn object_flags(&self, object: Id) -> Result<u32>;
    /// Actor `+98` (Settlement component) non-null.
    fn has_settlement(&self, actor: Id) -> Result<bool>;
    /// `4CE6C0`: the actor itself or its Element organization.
    fn organization_of(&self, actor: Id) -> Result<Id>;
    /// ThingData `+1B4` of the object.
    fn definition_word_1b4(&self, object: Id) -> Result<i32>;
    /// Object `+20/+24` (binary32 bits).
    fn object_position(&self, object: Id) -> Result<[u32; 2]>;
    /// GWorld `+22C/+230`.
    fn set_look_at(&mut self, xy: [u32; 2]) -> Result<()>;
    /// The `404F60` log (x and y passed as doubles).
    fn log_look_at(&mut self, xy: [u32; 2]) -> Result<()>;
    fn precision(&self) -> Precision;
}

/// `4FD3F3..4FD47C`.
pub fn after_post_load<W: LoadTailWorld>(world: &mut W) -> Result<()> {
    let mode = world.map_settings_mode()?;
    if (mode == 1 || mode == 4) && world.world_start_flag()? {
        world.world_start()?;
    }
    let kingdom = local_kingdom(world)?;
    let xy = look_at(world, kingdom)?;
    world.set_look_at(xy)?;
    world.log_look_at(xy)
}

/// `4F5290`: no local kingdom in application state 3; by local player name
/// (`4F48E0`) when GWorld `+48` is 0 or the map settings mode is non-zero;
/// else the local player's kingdom (`76AC70`).
pub fn local_kingdom<W: LoadTailWorld>(world: &mut W) -> Result<Option<Id>> {
    if world.app_state()? == 3 {
        return Ok(None);
    }
    let by_name = world.world_word_48()? == 0 || world.map_settings_mode()? != 0;
    if by_name {
        let player = world
            .local_player()?
            .ok_or(Error::NullDereference("4F5304: [[B36558]+8]+24 with no local player"))?;
        let name = world.player_name(&player)?;
        return kingdom_by_name(world, &name);
    }
    // 76AC70 (the application state test repeats).
    if world.app_state()? == 3 {
        return Ok(None);
    }
    match world.local_player()? {
        Some(player) => world.player_kingdom(&player),
        None => Ok(None),
    }
}

/// `4F48E0`: the first non-null GWorld `+148` entry whose `+18` name equals
/// `name` (`wcscmp`).
pub fn kingdom_by_name<W: LoadTailWorld>(world: &mut W, name: &[u16]) -> Result<Option<Id>> {
    for kingdom in world.world_kingdom_slots()?.into_iter().flatten() {
        if world.object_name_18(kingdom)? == name {
            return Ok(Some(kingdom));
        }
    }
    Ok(None)
}

/// `4F1C20`: without a kingdom, the map centre (`[GWorld+30]+C/+10` x 0.5 in
/// the x87, stored binary32). With one, over its `+284` actors not flagged
/// bit 22: the highest ThingData `+1B4` (strictly above -1) among actors with
/// a Settlement component wins, else among the flag bit 29 actors, else among
/// the organizations (`4CE6C0`) of the others; with no candidate, the `+1F8`
/// object's position, else the map centre.
pub fn look_at<W: LoadTailWorld>(world: &mut W, kingdom: Option<Id>) -> Result<[u32; 2]> {
    let pc = world.precision();
    let size = world.map_size_bits()?;
    let half = Binary::from_f32_bits(0x3f00_0000)?;
    let centre = [
        Binary::from_f32_bits(size[0])?.mul(&half, pc)?.store_f32()?,
        Binary::from_f32_bits(size[1])?.mul(&half, pc)?.store_f32()?,
    ];
    let Some(kingdom) = kingdom else {
        return Ok(centre);
    };
    let (mut settlement, mut flagged, mut organization) = (-1i32, -1i32, -1i32);
    let (mut settlement_xy, mut flagged_xy, mut organization_xy) = ([0u32; 2], [0u32; 2], [0u32; 2]);
    for actor in world.kingdom_actors(kingdom)? {
        let flags = world.object_flags(actor)?;
        if flags & (1 << 22) != 0 {
            continue;
        }
        if world.has_settlement(actor)? {
            let p = world.definition_word_1b4(actor)?;
            if p > settlement {
                settlement = p;
                settlement_xy = world.object_position(actor)?;
            }
        } else if flags & (1 << 29) != 0 {
            let p = world.definition_word_1b4(actor)?;
            if p > flagged {
                flagged = p;
                flagged_xy = world.object_position(actor)?;
            }
        } else {
            let org = world.organization_of(actor)?;
            let p = world.definition_word_1b4(org)?;
            if p > organization {
                organization_xy = world.object_position(org)?;
                organization = p;
            }
        }
    }
    if settlement >= 0 {
        return Ok(settlement_xy);
    }
    if flagged >= 0 {
        return Ok(flagged_xy);
    }
    if organization >= 0 {
        return Ok(organization_xy);
    }
    match world.kingdom_word_1f8(kingdom)? {
        Some(object) => world.object_position(object),
        None => Ok(centre),
    }
}
