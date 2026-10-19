//! Native kernels of the action-mask publication 0x4CFCC0 branches 2/3:
//! 0x76C7F0 (Session owner views), 0x4ECD70 (VWorldState selection command
//! masks), 0x50C8E0 (selected-view HUD notification) with 0x50C7D0, and the
//! 0x4F5290 binding of the existing `aa_actors::world_load_tail` kernel.
//!
//! Proof: research/phase8-orders-input/action-mask-publication/
//! (native_action_mask_publication.py, Unicorn over both pinned images at
//! PC24/PC53/PC64; every instruction of 4CFCC0, 529D70, 76C7F0, 765E00,
//! 4ECD70, 50C8E0, 4D1E80/4D1E90/4E5240/887B50/6A71D0/969B10 executed).
//!
//! 0x4ECD70 (thiscall, VWorldState): zero +10/+14/+18/+1C; a NULL +4 head
//! returns. Phase 1, for index 0..0x27: with a head whose thing differs from
//! the tail's, `[[B36530]+0x298][index]+0x3C` (non-group command) skips the
//! index; otherwise the 64-bit bit (0x96D710) is ORed into +18/+1C and, per
//! node from +4 (head first), thing vt+0x40(kingdom +0, index): true ORs the
//! bit into +10/+14 (index 0x23 additionally needs 0x5845A0 on thing+0xB4,
//! whose false skips the node), false clears it from +18/+1C. Phase 2, with
//! a non-NULL kingdom and a head thing differing from the tail's: count (up
//! to 2) the nodes whose vt+0x20(1) holds and either vt+0xB4 is false or
//! vt+0x134(kingdom) holds; a non-NULL +20 group and a count of 2 OR 0x40
//! (group +18 byte set) or 0x20 into +14 and +1C (bit 0x26 / 0x25).
//! Every Thing primary vtable has vt+0x40 = 0x887B50 (`xor al,al; ret 8`),
//! so phase 1 of a real selection always ends with zero words.
use super::{ActionMaskPublication, DynGlobals, PlayerRef, Result};
use crate::{actor_world::{ActorAccess, Spatial}, catalog::Catalog, player_order_input::command_masks::MaskLeaves,
    player_order_world::{Banks, DefinitionBankLoan, Globals}};
use aa_actors::{ActorId, post_load, world_load_tail};

/// view+0x54 bit 17: the view is selected (0x4CFD11 / 0x50C8FD `shr 0x11`).
pub const SELECTED_BIT_17: u32 = 1 << 17;
/// 0x50C8E5: `[B36534]+0xD8 == 6` suppresses 0x50C8E0.
pub const SUPPRESSED_PHASE: u32 = 6;
/// `cmp ebx, 0x27` at 0x4ECE58.
pub const COMMAND_COUNT: u32 = 0x27;
/// research (0x23): additionally 0x5845A0 on thing+0xB4.
pub const RESEARCH: u32 = 0x23;
/// 0x4ECEDA / 0x4ECEE1: group_behavior_disable (0x26) / enable (0x25) in
/// the high words.
pub const GROUP_DISABLE_HIGH: u32 = 0x40;
pub const GROUP_ENABLE_HIGH: u32 = 0x20;

// --- 0x4ECD70 ----------------------------------------------------------------

/// Every native read of 0x4ECD70 on one VWorldState.
pub trait SelectionMaskLeaves {
    type Node: Copy;
    type Thing: Copy + Eq;
    type Kingdom: Copy;
    type Research: Copy;
    type Group: Copy;
    /// VWorldState +0.
    fn kingdom_0(&mut self) -> Result<Option<Self::Kingdom>>;
    /// +4 head / +8 tail node.
    fn head_4(&mut self) -> Result<Option<Self::Node>>;
    fn tail_8(&mut self) -> Result<Option<Self::Node>>;
    /// node +0 (NULL: None) / node +4.
    fn thing(&mut self, node: Self::Node) -> Result<Option<Self::Thing>>;
    fn next(&mut self, node: Self::Node) -> Result<Option<Self::Node>>;
    /// +10, +14, +18, +1C.
    fn masks(&mut self) -> &mut [u32; 4];
    /// `[[B36530]+0x298][index]+0x3C != 0`.
    fn non_group_3c(&mut self, index: u32) -> Result<bool>;
    /// thing vt+0x40(kingdom, index), AL.
    fn virtual_40(&mut self, thing: Self::Thing, kingdom: Option<Self::Kingdom>, index: u32) -> Result<bool>;
    /// thing +0xB4 (None: NULL).
    fn research_b4(&mut self, thing: Self::Thing) -> Result<Option<Self::Research>>;
    fn research_available_5845a0(&mut self, research: Self::Research) -> Result<bool>;
    /// thing vt+0x20(argument), vt+0xB4(), vt+0x134(kingdom), AL.
    fn virtual_20(&mut self, thing: Self::Thing, argument: u8) -> Result<bool>;
    fn virtual_b4(&mut self, thing: Self::Thing) -> Result<bool>;
    fn virtual_134(&mut self, thing: Self::Thing, kingdom: Self::Kingdom) -> Result<bool>;
    /// +20 (None: NULL) and its +18 byte.
    fn group_20(&mut self) -> Result<Option<Self::Group>>;
    fn group_byte_18(&mut self, group: Self::Group) -> Result<bool>;
}

fn tail_thing<L: SelectionMaskLeaves>(l: &mut L, address: &str) -> Result<Option<L::Thing>> {
    let tail = l.tail_8()?.ok_or_else(|| format!("0x{address}: VWorldState+8 tail is NULL under a non-NULL head (native NULL read)"))?;
    l.thing(tail)
}
fn node_thing<L: SelectionMaskLeaves>(l: &mut L, node: L::Node, address: &str) -> Result<L::Thing> {
    l.thing(node)?.ok_or_else(|| format!("0x{address}: a VWorldState selection node holds a NULL thing (native NULL vtable read)"))
}

/// 0x4ECD70 in native order, writing the caller's +10..+1C in place (an
/// error leaves every earlier store observable, as a native fault would).
pub fn selection_masks_4ecd70<L: SelectionMaskLeaves>(l: &mut L) -> Result<()> {
    *l.masks() = [0; 4];
    if l.head_4()?.is_none() {
        return Ok(());
    }
    for index in 0..COMMAND_COUNT {
        if let Some(head) = l.head_4()? {
            let first = l.thing(head)?;
            if first != tail_thing(l, "4ECDAC")? && l.non_group_3c(index)? {
                continue;
            }
        }
        let bit = 1u64 << index; // 0x96D710 __allshl(1, index)
        let (low, high) = (bit as u32, (bit >> 32) as u32);
        let m = l.masks();
        m[2] |= low;
        m[3] |= high;
        let mut node = l.head_4()?;
        while let Some(n) = node {
            let kingdom = l.kingdom_0()?;
            let thing = node_thing(l, n, "4ECDF7")?;
            if l.virtual_40(thing, kingdom, index)? {
                let available = if index == RESEARCH {
                    let research = l.research_b4(thing)?
                        .ok_or("0x5845A3: research test on a NULL thing+0xB4 (native NULL read)")?;
                    l.research_available_5845a0(research)?
                } else {
                    true
                };
                if available {
                    let m = l.masks();
                    m[0] |= low;
                    m[1] |= high;
                }
            } else {
                let m = l.masks();
                m[2] &= !low;
                m[3] &= !high;
            }
            node = l.next(n)?;
        }
    }
    let Some(kingdom) = l.kingdom_0()? else { return Ok(()) };
    let Some(head) = l.head_4()? else { return Ok(()) };
    let first = l.thing(head)?;
    if first == tail_thing(l, "4ECE77")? {
        return Ok(());
    }
    let mut count = 0u32;
    let mut node = l.head_4()?;
    while let Some(n) = node {
        if count >= 2 {
            break;
        }
        let thing = node_thing(l, n, "4ECE8C")?;
        if l.virtual_20(thing, 1)? && (!l.virtual_b4(thing)? || l.virtual_134(thing, kingdom)?) {
            count += 1;
        }
        node = l.next(n)?;
    }
    let Some(group) = l.group_20()? else { return Ok(()) };
    if count <= 1 {
        return Ok(());
    }
    let bit = if l.group_byte_18(group)? { GROUP_DISABLE_HIGH } else { GROUP_ENABLE_HIGH };
    let m = l.masks();
    m[1] |= bit;
    m[3] |= bit;
    Ok(())
}

// --- 0x76C7F0 ----------------------------------------------------------------

/// Native reads of 0x76C7F0 on the Session.
pub trait OwnerViews {
    type Player: Copy;
    /// Session +0xBC nodes, head first.
    fn players_bc(&mut self) -> Result<Vec<Self::Player>>;
    /// 0x765E00 (NULL without a GWorld, else player +0x28).
    fn kingdom_765e00(&mut self, player: Self::Player) -> Result<Option<ActorId>>;
    /// player +0x50 non-NULL.
    fn has_view_state_50(&mut self, player: Self::Player) -> Result<bool>;
    /// 0x4ECD70 on player +0x50.
    fn rebuild_4ecd70(&mut self, player: Self::Player) -> Result<()>;
}

/// 0x76C7F0(owner): the second 0x765E00 is called only when the first
/// result differs from `owner`.
pub fn owner_views_76c7f0<W: OwnerViews>(w: &mut W, owner: Option<ActorId>) -> Result<()> {
    for player in w.players_bc()? {
        let refresh = w.kingdom_765e00(player)? == owner || w.kingdom_765e00(player)?.is_none();
        if refresh && w.has_view_state_50(player)? {
            w.rebuild_4ecd70(player)?;
        }
    }
    Ok(())
}

// --- 0x50C8E0 ----------------------------------------------------------------

/// Native reads and calls of 0x50C8E0(actor).
pub trait SelectedView {
    type View: Copy + Eq;
    /// `[B36534]+0xD8`.
    fn world_phase_d8(&mut self) -> Result<u32>;
    /// Thing+0xC (None: NULL).
    fn thing_view_c(&mut self, actor: ActorId) -> Result<Option<Self::View>>;
    /// view +0x54.
    fn view_flags_54(&mut self, view: Self::View) -> Result<u32>;
    /// actor vf108 (0x4D1E80: Actor+0xE8).
    fn owner_108(&mut self, actor: ActorId) -> Result<Option<ActorId>>;
    /// 0x4F5290 on `[B36534]`.
    fn local_kingdom_4f5290(&mut self) -> Result<Option<ActorId>>;
    /// `[[B36538]+0x13C]+0x08` views, head first.
    fn ui_selection_8(&mut self) -> Result<Vec<Self::View>>;
    /// 0x437EB0 on `[B36538]+0x13C`.
    fn command_masks_437eb0(&mut self) -> Result<()>;
}

/// 0x50C7D0(view): the first `+08` node whose view equals `view`.
pub fn contains_50c7d0<V: Eq>(list: &[V], view: Option<V>) -> bool {
    view.is_some_and(|v| list.contains(&v))
}

/// 0x50C8E0(actor) in native order (every field re-read where native does).
pub fn selected_view_50c8e0<W: SelectedView>(w: &mut W, actor: ActorId) -> Result<()> {
    if w.world_phase_d8()? == SUPPRESSED_PHASE {
        return Ok(());
    }
    let Some(view) = w.thing_view_c(actor)? else { return Ok(()) };
    if w.view_flags_54(view)? & SELECTED_BIT_17 == 0 {
        return Ok(());
    }
    if w.owner_108(actor)?.is_none() || w.local_kingdom_4f5290()?.is_none() {
        return Ok(());
    }
    let owner = w.owner_108(actor)?;
    if w.local_kingdom_4f5290()? != owner {
        return Ok(());
    }
    let view = w.thing_view_c(actor)?;
    let list = w.ui_selection_8()?;
    if contains_50c7d0(&list, view) {
        w.command_masks_437eb0()?;
    }
    Ok(())
}

// --- 0x4F5290 ------------------------------------------------------------------

/// The 0x4F5290 reads (a subset of [`ActionMaskPublication`]).
pub trait LocalKingdomReads {
    fn application_state_e4(&self) -> Result<u32>;
    fn world_word_48(&self) -> Result<u32>;
    fn map_settings_mode_a8(&self) -> Result<u32>;
    fn local_player_76ac60(&self) -> Result<Option<PlayerRef>>;
    fn player_name_24(&self, player: PlayerRef) -> Result<Vec<u16>>;
    fn player_kingdom_765e00(&self, player: PlayerRef) -> Result<Option<ActorId>>;
    fn world_kingdom_slots(&self) -> Result<Vec<Option<ActorId>>>;
}
impl<T: ActionMaskPublication + ?Sized> LocalKingdomReads for T {
    fn application_state_e4(&self) -> Result<u32> {
        ActionMaskPublication::application_state_e4(self)
    }
    fn world_word_48(&self) -> Result<u32> {
        ActionMaskPublication::world_word_48(self)
    }
    fn map_settings_mode_a8(&self) -> Result<u32> {
        ActionMaskPublication::map_settings_mode_a8(self)
    }
    fn local_player_76ac60(&self) -> Result<Option<PlayerRef>> {
        ActionMaskPublication::local_player_76ac60(self)
    }
    fn player_name_24(&self, player: PlayerRef) -> Result<Vec<u16>> {
        ActionMaskPublication::player_name_24(self, player)
    }
    fn player_kingdom_765e00(&self, player: PlayerRef) -> Result<Option<ActorId>> {
        ActionMaskPublication::player_kingdom_765e00(self, player)
    }
    fn world_kingdom_slots(&self) -> Result<Vec<Option<ActorId>>> {
        ActionMaskPublication::world_kingdom_slots(self)
    }
}

/// `aa_actors::world_load_tail::local_kingdom` (0x4F5290, already proved by
/// research/phase8-saved-components/native-load-tail.json and composed here
/// by the `selected` / `publish` suites) over these reads and the current
/// kingdoms' Thing+0x18 names (0x4BEDF0).
pub fn local_kingdom_4f5290<R: LocalKingdomReads + ?Sized>(
    reads: &R,
    name_18: &dyn Fn(ActorId) -> Result<Vec<u16>>,
    precision: aa_numeric::Precision,
) -> Result<Option<ActorId>> {
    world_load_tail::local_kingdom(&mut LocalKingdom { reads, name_18, precision })
        .map(|k| k.map(ActorId::from_raw))
        .map_err(|e| e.to_string())
}
struct LocalKingdom<'a, R: ?Sized> {
    reads: &'a R,
    name_18: &'a dyn Fn(ActorId) -> Result<Vec<u16>>,
    precision: aa_numeric::Precision,
}
fn tail_error(e: String) -> post_load::Error {
    post_load::Error::World(e)
}
fn not_4f5290<T>(leaf: &str) -> post_load::Result<T> {
    Err(post_load::Error::World(format!("{leaf} is not read by 0x4F5290")))
}
impl<R: LocalKingdomReads + ?Sized> world_load_tail::LoadTailWorld for LocalKingdom<'_, R> {
    type Player = PlayerRef;
    fn map_settings_mode(&self) -> post_load::Result<u32> {
        self.reads.map_settings_mode_a8().map_err(tail_error)
    }
    fn world_start_flag(&mut self) -> post_load::Result<bool> {
        not_4f5290("59E520")
    }
    fn world_start(&mut self) -> post_load::Result<()> {
        not_4f5290("4F1E10")
    }
    fn app_state(&self) -> post_load::Result<u32> {
        self.reads.application_state_e4().map_err(tail_error)
    }
    fn world_word_48(&self) -> post_load::Result<u32> {
        self.reads.world_word_48().map_err(tail_error)
    }
    fn local_player(&self) -> post_load::Result<Option<PlayerRef>> {
        self.reads.local_player_76ac60().map_err(tail_error)
    }
    fn player_name(&self, player: &PlayerRef) -> post_load::Result<Vec<u16>> {
        self.reads.player_name_24(*player).map_err(tail_error)
    }
    fn player_kingdom(&self, player: &PlayerRef) -> post_load::Result<Option<post_load::Id>> {
        self.reads.player_kingdom_765e00(*player).map(|k| k.map(ActorId::raw)).map_err(tail_error)
    }
    fn world_kingdom_slots(&self) -> post_load::Result<Vec<Option<post_load::Id>>> {
        self.reads.world_kingdom_slots().map(|v| v.into_iter().map(|k| k.map(ActorId::raw)).collect()).map_err(tail_error)
    }
    fn object_name_18(&self, object: post_load::Id) -> post_load::Result<Vec<u16>> {
        (self.name_18)(ActorId::from_raw(object)).map_err(tail_error)
    }
    fn map_size_bits(&self) -> post_load::Result<[u32; 2]> {
        not_4f5290("4F1C20 map size")
    }
    fn kingdom_actors(&self, _: post_load::Id) -> post_load::Result<Vec<post_load::Id>> {
        not_4f5290("4F1C20 Kingdom+284")
    }
    fn kingdom_word_1f8(&self, _: post_load::Id) -> post_load::Result<Option<post_load::Id>> {
        not_4f5290("4F1C20 Kingdom+1F8")
    }
    fn object_flags(&self, _: post_load::Id) -> post_load::Result<u32> {
        not_4f5290("4F1C20 Object+8")
    }
    fn has_settlement(&self, _: post_load::Id) -> post_load::Result<bool> {
        not_4f5290("4F1C20 Actor+98")
    }
    fn organization_of(&self, _: post_load::Id) -> post_load::Result<post_load::Id> {
        not_4f5290("4CE6C0")
    }
    fn definition_word_1b4(&self, _: post_load::Id) -> post_load::Result<i32> {
        not_4f5290("ThingData+1B4")
    }
    fn object_position(&self, _: post_load::Id) -> post_load::Result<[u32; 2]> {
        not_4f5290("Object+20/+24")
    }
    fn set_look_at(&mut self, _: [u32; 2]) -> post_load::Result<()> {
        not_4f5290("GWorld+22C/+230")
    }
    fn log_look_at(&mut self, _: [u32; 2]) -> post_load::Result<()> {
        not_4f5290("404F60")
    }
    fn precision(&self) -> aa_numeric::Precision {
        self.precision
    }
}

// --- live 0x4ECD70 binding -------------------------------------------------------

/// The Thing classes whose primary vtables are bound (4F9C10: 2..=10, see
/// player_order_input::command_mask_world). Every one of them has
/// vt+0x40 = 0x887B50 (`xor al, al; ret 8`).
fn thing_class(actors: &ActorAccess<'_>, thing: ActorId) -> Result<u32> {
    let key = actors.key(thing).ok_or("0x4ECD70 selection thing: current generation")?;
    let d = &actors.object(key).map_err(|e| e.to_string())?.definition;
    if d.native_class != d.proof.category.runtime_class {
        return Err("0x4ECD70 selection thing class differs from its source category".into());
    }
    match d.native_class {
        2..=10 => Ok(d.native_class),
        c => Err(format!("0x4ECD70 selection thing class {c} has no 4F9C10 primary vtable")),
    }
}

/// [`SelectionMaskLeaves`] over one Player+0x50 VWorldState
/// ([`crate::player_view_state::State`]) and the SAME in-frame simulation
/// loans: vt+0x20 / vt+0xB4 / vt+0x134, thing+0xB4 / 0x5845A0 and the
/// definitions' +0x3C are the command-mask leaves of
/// `player_order_input::command_mask_world` (the 0x437EB0 bindings).
struct ViewState<'v, 'a, 'store, 'src, P, G, F> {
    state: &'v mut crate::player_view_state::State,
    actors: &'a ActorAccess<'store>,
    catalog: &'a Catalog<'src>,
    banks: &'a dyn Banks,
    live: crate::player_order_input::command_mask_world::Live<'a, 'src, usize, ActorAccess<'store>, P, G, F>,
}
impl<P, G, F> SelectionMaskLeaves for ViewState<'_, '_, '_, '_, P, G, F>
where
    P: Spatial,
    G: Globals,
    F: FnMut(usize) -> Result<Option<ActorId>>,
{
    type Node = crate::player_selection::NodeId;
    type Thing = ActorId;
    type Kingdom = ActorId;
    type Research = ActorId;
    type Group = ();
    fn kingdom_0(&mut self) -> Result<Option<ActorId>> {
        Ok(self.state.kingdom)
    }
    fn head_4(&mut self) -> Result<Option<Self::Node>> {
        Ok(self.state.selected.head())
    }
    fn tail_8(&mut self) -> Result<Option<Self::Node>> {
        Ok(self.state.selected.tail())
    }
    fn thing(&mut self, node: Self::Node) -> Result<Option<ActorId>> {
        Ok(Some(self.state.selected.node(node)?.actor))
    }
    fn next(&mut self, node: Self::Node) -> Result<Option<Self::Node>> {
        Ok(self.state.selected.node(node)?.next)
    }
    fn masks(&mut self) -> &mut [u32; 4] {
        &mut self.state.mask_words_10_1c
    }
    fn non_group_3c(&mut self, index: u32) -> Result<bool> {
        self.live.non_group_3c(index)?.ok_or_else(|| format!("0x4ECDB7 [[B36530]+0x298][{index}] definition is NULL"))
    }
    fn virtual_40(&mut self, thing: ActorId, _: Option<ActorId>, _: u32) -> Result<bool> {
        if matches!(thing_class(self.actors, thing)?, 3 | 4) {
            crate::player_order_world::object_with_bank(self.catalog, self.banks, self.actors, thing).map_err(|e| e.to_string())?;
        }
        Ok(false)
    }
    fn research_b4(&mut self, thing: ActorId) -> Result<Option<ActorId>> {
        self.live.research_b4(thing)
    }
    fn research_available_5845a0(&mut self, research: ActorId) -> Result<bool> {
        self.live.research_available_5845a0(research)
    }
    fn virtual_20(&mut self, thing: ActorId, argument: u8) -> Result<bool> {
        MaskLeaves::virtual_20(&mut self.live, thing, argument)
    }
    fn virtual_b4(&mut self, thing: ActorId) -> Result<bool> {
        self.live.virtual_b4(thing)
    }
    fn virtual_134(&mut self, thing: ActorId, kingdom: ActorId) -> Result<bool> {
        self.live.virtual_134(thing, kingdom)
    }
    fn group_20(&mut self) -> Result<Option<()>> {
        if self.state.current_group.is_null() {
            Ok(None)
        } else {
            Err("0x4ECEC0 positive VWorldState+20 group: its +18 byte has no bound producer".into())
        }
    }
    fn group_byte_18(&mut self, _: ()) -> Result<bool> {
        Err("0x4ECECC VWorldState+20 group +18 byte has no bound producer".into())
    }
}

/// 0x4ECD70 on one live VWorldState over the SAME in-frame loans. The
/// native 4ECFF0 / 4ED080 / 4ED0D0 selection paths call the same function.
pub fn rebuild_view_state_4ecd70<P: Spatial>(
    catalog: &Catalog<'_>,
    bank: &DefinitionBankLoan<'_>,
    actors: &ActorAccess<'_>,
    spatial: &P,
    globals: &mut dyn Globals,
    state: &mut crate::player_view_state::State,
) -> Result<()> {
    let banks: &dyn Banks = bank;
    let mut globals = DynGlobals(globals);
    let live = crate::player_order_input::command_mask_world::Live::with_banks(
        catalog, banks, actors, spatial, &mut globals,
        Err("0x5C0E30 is not read by 0x4ECD70".into()),
        |_: usize| -> Result<Option<ActorId>> { Err("view+0x14 is not read by 0x4ECD70".into()) },
    );
    selection_masks_4ecd70(&mut ViewState { state, actors, catalog, banks, live })
}

#[cfg(test)]
#[path = "action_mask_publication_tests.rs"]
mod tests;
