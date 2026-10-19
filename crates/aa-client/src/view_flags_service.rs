//! Client side of `aa_runtime::view_flags_service` (native 0x4CFCC0
//! branches 2/3): the application owners lend themselves for ONE simulation
//! callback as an [`ActionMaskPublication`]:
//!
//! * [`DirectoryFlags`]: view+0x54 of the view published at Thing+0xC,
//!   read from the SAME `live_scene::view_directory::ViewDirectory` entry
//!   (its `native_fields.flags()`, the word `ViewDirectory::flags_54` reads);
//! * [`SessionPublication`]: GWorld [B36534] words, the Session [B36550]
//!   +0xBC players (+0x28 kingdom, +0x50 VWorldState) and [B36558]+8, plus
//!   the 0x40EAA0/0x76AC90 reads of actor vt+0x20 inside 0x4ECD70;
//! * [`UiPublication`]: the game UIWorldState [[B36538]+0x13C] +0x08 list
//!   (0x50C7D0) and its 0x437EB0 recompute of +0x20..+0x2C, run now with
//!   the in-frame [`HudLoans`].
//!
//! Nothing here caches a flag, a list or a kingdom: every read goes to its
//! owner at the call. An unbound owner returns its error to the simulation.
// The frame owners construct ClientPublication once they lend these owners
// (view_world_owner / battle_orders_input / netcode, see the research README).
#![allow(dead_code)]
use aa_actors::ActorId;
use aa_runtime::{
    actor_world::ObjectKey,
    player_order_world::Globals,
    player_view_state,
    view_flags_service::{ActionMaskPublication, HudLoans, PlayerRef, Result},
};

/// view+0x54 of the view at an actor's Thing+0xC. The implementation must
/// validate that the handle names this actor's current published view with
/// completed Init (as `ViewDirectory::flags_for_current` does) and read its
/// native fields' flags.
pub trait DirectoryFlags {
    fn flags_54_for_thing_c(&self, actor: ActorId, key: ObjectKey, thing_c: usize) -> anyhow::Result<u32>;
}

/// Session / GWorld owners of 0x76C7F0, 0x4F5290 and 0x50C8E0.
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
        -> Result<Option<(&mut player_view_state::State, &mut dyn Globals)>>;
}

/// The game UIWorldState of [B36538]+0x13C.
pub trait UiPublication {
    /// +0x08 nodes' views (ViewHandle::raw, the Thing+0xC words), head first.
    fn selection_8(&self) -> Result<Vec<usize>>;
    /// 0x437EB0: `loans.command_masks_437eb0(globals, 0x5C0E30, +0x08 list,
    /// view+0x14 things, +0x4C, [+0x40]+0x18)` and store +0x20..+0x2C.
    fn command_masks_437eb0(&mut self, loans: &mut HudLoans<'_, '_, '_, '_>) -> Result<()>;
}

/// The composed service, lent for one simulation callback.
pub struct ClientPublication<'a, D: ?Sized, S: ?Sized, U: ?Sized> {
    pub directory: &'a D,
    pub session: &'a mut S,
    pub ui: Option<&'a mut U>,
}

fn ui_missing<T>() -> Result<T> {
    Err("[B36538]+0x13C game UIWorldState owner is not lent to this callback (0x50C944 / 0x50C7D0)".into())
}

impl<D: DirectoryFlags + ?Sized, S: SessionPublication + ?Sized, U: UiPublication + ?Sized> ActionMaskPublication
    for ClientPublication<'_, D, S, U>
{
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
        -> Result<Option<(&mut player_view_state::State, &mut dyn Globals)>> {
        self.session.player_view_state_50(player)
    }
    fn view_flags_54(&self, actor: ActorId, key: ObjectKey, thing_c: usize) -> Result<u32> {
        self.directory.flags_54_for_thing_c(actor, key, thing_c).map_err(|e| format!("{e:#}"))
    }
    fn ui_selection_8(&self) -> Result<Vec<usize>> {
        match &self.ui {
            Some(ui) => ui.selection_8(),
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
