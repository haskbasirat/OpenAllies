//! Application services of the native action-mask publication 0x4CFCC0
//! branches 2 and 3 (the third branch's view+0x54 read included).
//!
//! 0x4D1350 rebuilds Actor+F0/F4 and tail-jumps to 0x4CFCC0 only when the
//! mask changed ([`aa_actors::action_publication::changed`]). Its branches
//! 2 and 3 publish the new mask outside the simulation's own owners:
//!
//! * branch 2 (owner `vf108` non-NULL and `(i8)Object+0x4C > 0`): 0x76C7F0
//!   on `[B36550]` (Session) walks the `+0xBC` player list; a player whose
//!   0x765E00 kingdom is the owner or NULL and whose `+0x50` VWorldState is
//!   non-NULL gets 0x4ECD70 (its selection command masks +0x10..+0x1C);
//! * branch 3 (Thing+0xC view non-NULL and view `+0x54` bit 17, selected):
//!   0x50C8E0 checks `[B36534]+0xD8 != 6`, bit 17 again, a non-NULL owner,
//!   0x4F5290 local kingdom == owner and 0x50C7D0 (the view is in the
//!   UIWorldState `[[B36538]+0x13C]+0x08` list), then 0x437EB0 recomputes
//!   the HUD's UIWorldState +0x20..+0x2C masks.
//!
//! The kernels are in [`native`]; this trait names every native read and
//! call they need from the Session, the VWorld/UI and the World owners. A
//! simulation callback without an installed service keeps returning its
//! explicit error naming the native call (nothing is skipped or defaulted).
//! Proof: research/phase8-orders-input/action-mask-publication/.
use crate::{actor_world::{ActorAccess, ObjectKey, Spatial}, catalog::Catalog, player_order_world::{Banks, DefinitionBankLoan, Globals}};
use aa_actors::ActorId;

#[path = "action_mask_publication.rs"]
pub mod native;

pub type Result<T> = std::result::Result<T, String>;

/// One Session `+0xBC` player node, as the Session owner identifies it
/// (for example the network player id). Never a raw native pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerRef(pub u32);

/// Every application read and call of 0x4CFCC0 branches 2/3. Implementors
/// read the SAME live owners at the call (no snapshot); a read whose owner
/// is not bound returns an error naming it. Reads take `&self`; the two
/// mutating services (0x4ECD70's VWorldState and 0x437EB0) take `&mut self`.
pub trait ActionMaskPublication {
    // --- GWorld [B36534] ---------------------------------------------------
    /// `[B36534]+0xD8` (the dispatcher phase; 6 suppresses 0x50C8E0).
    fn world_phase_d8(&self) -> Result<u32>;
    /// `[B36534]+0x48` (0x4F5290: 0 selects the local player's name lookup).
    fn world_word_48(&self) -> Result<u32>;
    /// `[[B36534]+0x28]+0xA8` (0x4F5290: non-zero selects the name lookup).
    fn map_settings_mode_a8(&self) -> Result<u32>;
    /// `[B36534]+0x148/+0x14C` kingdom slots in order (NULL slots as None);
    /// 0x4F48E0 compares each kingdom's `+0x18` name.
    fn world_kingdom_slots(&self) -> Result<Vec<Option<ActorId>>>;
    // --- Session [B36550] / net [B36558] ----------------------------------
    /// `[B36550]+0xE4` (application state; 3 = no local kingdom).
    fn application_state_e4(&self) -> Result<u32>;
    /// 0x76AC60: `[[B36558]+8]` (None: NULL).
    fn local_player_76ac60(&self) -> Result<Option<PlayerRef>>;
    /// Player `+0x24` name (UTF-16, no terminator).
    fn player_name_24(&self, player: PlayerRef) -> Result<Vec<u16>>;
    /// 0x765E00: NULL without a GWorld, else player `+0x28`.
    fn player_kingdom_765e00(&self, player: PlayerRef) -> Result<Option<ActorId>>;
    /// The Session `+0xBC` player nodes, head first.
    fn session_players_bc(&self) -> Result<Vec<PlayerRef>>;
    /// Player `+0x50` (None: NULL) and the Session/application reads that
    /// actor vt+0x20 (0x4CF120: 0x40EAA0, 0x76AC90) needs while 0x4ECD70
    /// holds that VWorldState.
    fn player_view_state_50(&mut self, player: PlayerRef)
        -> Result<Option<(&mut crate::player_view_state::State, &mut dyn Globals)>>;
    // --- VWorld [B36538] ---------------------------------------------------
    /// view `+0x54` of the view published at the actor's Thing+0xC
    /// (`thing_c`, the ViewDirectory's portable handle; never 0).
    fn view_flags_54(&self, actor: ActorId, key: ObjectKey, thing_c: usize) -> Result<u32>;
    /// The UIWorldState `[[B36538]+0x13C]+0x08` selection nodes' views
    /// (Thing+0xC handles), head first. A NULL VWorld or UIWorldState is a
    /// native fault (0x50C944 / 0x50C7D0) and must be an error.
    fn ui_selection_8(&self) -> Result<Vec<usize>>;
    /// 0x437EB0 on that UIWorldState, now, inside this simulation callback:
    /// recompute +0x20..+0x2C (see [`HudLoans::command_masks_437eb0`]).
    fn command_masks_437eb0(&mut self, loans: &mut HudLoans<'_, '_, '_, '_>) -> Result<()>;
}

/// The VWorld part: view+0x54 at an actor's Thing+0xC (the client's
/// ViewDirectory entry; see aa-client view_flags_service.rs).
pub trait ViewFlags {
    fn view_flags_54(&self, actor: ActorId, key: ObjectKey, thing_c: usize) -> Result<u32>;
}
/// The GWorld / Session part (the netcode Session players and the World).
pub trait SessionPublication {
    fn world_phase_d8(&self) -> Result<u32>;
    fn world_word_48(&self) -> Result<u32>;
    fn map_settings_mode_a8(&self) -> Result<u32>;
    fn world_kingdom_slots(&self) -> Result<Vec<Option<ActorId>>>;
    fn application_state_e4(&self) -> Result<u32>;
    fn local_player_76ac60(&self) -> Result<Option<PlayerRef>>;
    fn player_name_24(&self, player: PlayerRef) -> Result<Vec<u16>>;
    fn player_kingdom_765e00(&self, player: PlayerRef) -> Result<Option<ActorId>>;
    fn session_players_bc(&self) -> Result<Vec<PlayerRef>>;
    fn player_view_state_50(&mut self, player: PlayerRef)
        -> Result<Option<(&mut crate::player_view_state::State, &mut dyn Globals)>>;
}
/// The game UIWorldState part ([B36538]+0x13C).
pub trait UiPublication {
    /// +0x08 nodes' views (Thing+0xC handles), head first.
    fn ui_selection_8(&self) -> Result<Vec<usize>>;
    /// 0x437EB0 now: [`HudLoans::command_masks_437eb0`] with the UI's own
    /// +0x08 list, +0x4C, [+0x40]+0x18 and 0x5C0E30, then store +0x20..+0x2C.
    fn command_masks_437eb0(&mut self, loans: &mut HudLoans<'_, '_, '_, '_>) -> Result<()>;
}
/// The three parts lent together for one simulation callback. A part that
/// is absent (`ui: None`) is an explicit error when native reaches it.
pub struct Composed<'a> {
    pub view: &'a dyn ViewFlags,
    pub session: &'a mut dyn SessionPublication,
    pub ui: Option<&'a mut dyn UiPublication>,
}
fn ui_missing<T>() -> Result<T> {
    Err("[B36538]+0x13C game UIWorldState owner is not lent to this callback (0x50C944 / 0x50C7D0)".into())
}
impl ActionMaskPublication for Composed<'_> {
    fn world_phase_d8(&self) -> Result<u32> {
        self.session.world_phase_d8()
    }
    fn world_word_48(&self) -> Result<u32> {
        self.session.world_word_48()
    }
    fn map_settings_mode_a8(&self) -> Result<u32> {
        self.session.map_settings_mode_a8()
    }
    fn world_kingdom_slots(&self) -> Result<Vec<Option<ActorId>>> {
        self.session.world_kingdom_slots()
    }
    fn application_state_e4(&self) -> Result<u32> {
        self.session.application_state_e4()
    }
    fn local_player_76ac60(&self) -> Result<Option<PlayerRef>> {
        self.session.local_player_76ac60()
    }
    fn player_name_24(&self, player: PlayerRef) -> Result<Vec<u16>> {
        self.session.player_name_24(player)
    }
    fn player_kingdom_765e00(&self, player: PlayerRef) -> Result<Option<ActorId>> {
        self.session.player_kingdom_765e00(player)
    }
    fn session_players_bc(&self) -> Result<Vec<PlayerRef>> {
        self.session.session_players_bc()
    }
    fn player_view_state_50(&mut self, player: PlayerRef)
        -> Result<Option<(&mut crate::player_view_state::State, &mut dyn Globals)>> {
        self.session.player_view_state_50(player)
    }
    fn view_flags_54(&self, actor: ActorId, key: ObjectKey, thing_c: usize) -> Result<u32> {
        self.view.view_flags_54(actor, key, thing_c)
    }
    fn ui_selection_8(&self) -> Result<Vec<usize>> {
        match &self.ui {
            Some(ui) => ui.ui_selection_8(),
            None => ui_missing(),
        }
    }
    fn command_masks_437eb0(&mut self, loans: &mut HudLoans<'_, '_, '_, '_>) -> Result<()> {
        match &mut self.ui {
            Some(ui) => ui.command_masks_437eb0(loans),
            None => ui_missing(),
        }
    }
}

/// The simulation owners 0x437EB0 reads when 0x50C8E0 runs it inside a
/// simulation callback: the SAME Catalog, moved definition Bank, ActorStore
/// loan and spatial owner as the rebuilding 0x4D1350.
pub struct HudLoans<'l, 'bank, 'store, 'src> {
    pub(crate) catalog: &'l Catalog<'src>,
    pub(crate) bank: &'l DefinitionBankLoan<'bank>,
    pub(crate) actors: &'l ActorAccess<'store>,
    pub(crate) spatial: &'l dyn Spatial,
}

/// [`Spatial`] over a borrowed trait object (read-only here).
pub(crate) struct DynSpatial<'a>(pub(crate) &'a dyn Spatial);
impl Spatial for DynSpatial<'_> {
    fn placement(&self, id: ActorId) -> crate::actor_world::Result<crate::actor_world::PlacementState> {
        self.0.placement(id)
    }
    fn publish(&mut self, _: ActorId, _: crate::actor_world::PlacementState) -> crate::actor_world::Result<()> {
        Err(crate::actor_world::Error::Missing("action-mask publication cannot publish placement"))
    }
}
/// [`Globals`] over a borrowed trait object; every method forwards.
pub(crate) struct DynGlobals<'a>(pub(crate) &'a mut dyn Globals);
impl Globals for DynGlobals<'_> {
    fn can_process_40eaa0(&mut self) -> crate::player_order_world::Result<bool> {
        self.0.can_process_40eaa0()
    }
    fn session_phase_76ac90(&mut self) -> crate::player_order_world::Result<u32> {
        self.0.session_phase_76ac90()
    }
    fn swap_right_click_with_alt_right_click_4be150(&mut self) -> crate::player_order_world::Result<bool> {
        self.0.swap_right_click_with_alt_right_click_4be150()
    }
    fn hotkey_alt_right_click_command_4b2e60(&mut self) -> crate::player_order_world::Result<Vec<u16>> {
        self.0.hotkey_alt_right_click_command_4b2e60()
    }
}

impl HudLoans<'_, '_, '_, '_> {
    /// The existing 0x437EB0 port (player_order_input::command_masks) over
    /// these loans and the UI's own UIWorldState words: `selected` is its
    /// +0x08 list head first, `thing_14` the views' +0x14 Things,
    /// `kingdom_4c` its +0x4C word, `group_40_byte_18` the +0x40 group's
    /// +0x18 byte (None for NULL), `local_kingdom_5c0e30` the 0x5C0E30 read
    /// and `globals` the 0x40EAA0/0x76AC90 Session reads of actor vt+0x20.
    /// Returns +0x20, +0x24, +0x28, +0x2C; the caller stores them.
    #[allow(clippy::too_many_arguments)]
    pub fn command_masks_437eb0<V, F, G>(
        &mut self,
        globals: &mut G,
        local_kingdom_5c0e30: Result<Option<ActorId>>,
        selected: &[V],
        thing_14: F,
        kingdom_4c: Option<ActorId>,
        group_40_byte_18: Option<bool>,
    ) -> Result<[u32; 4]>
    where
        V: Copy + Eq + std::fmt::Debug,
        F: FnMut(V) -> Result<Option<ActorId>>,
        G: Globals,
    {
        use crate::player_order_input::{command_mask_world::Live, command_masks};
        let spatial = DynSpatial(self.spatial);
        let banks: &dyn Banks = self.bank;
        let mut live = Live::with_banks(self.catalog, banks, self.actors, &spatial, globals, local_kingdom_5c0e30, thing_14);
        command_masks::command_masks_437eb0(&mut live, selected, kingdom_4c, group_40_byte_18)
    }
}
