//! Concrete773E30 dependencies over the StartedMission's retained owners.
//! This procedure is deliberately not a world-ready/activation capability.
use super::*;
use aa_runtime::{
    application_mode::GameAppState,
    catalog::Catalog,
    creator_labels,
    definitions::Bank,
    player_glyph,
    resource_creator::Creator,
    standalone_network::Standalone,
    world_frame_inputs::{AiPlayers, WorldFrameInputs},
};

///The real diagnostic request is retained in native call order, including
///requests preceding a later required-owner failure. It is not a ready bit.
#[derive(Debug, PartialEq, Eq)]
pub enum Diagnostic {
    PreparePlayers,
    BindPlayer { player: Vec<u16>, kingdom: Vec<u16> },
    SetupAi { profile: String, kingdom: Vec<u16> },
}

impl LocalBootstrap {
    ///Actual773E30, with the genuine StartedMission facade's same actors,
    ///current Creator88, live Localization and sole SAI/Arena. No injectable
    ///successful Effects object can certify this concrete procedure.
    pub fn prepare_actual_players(
        &mut self,
        frame: &mut WorldFrameInputs,
        catalog: &Catalog<'_>,
        application: &GameAppState,
        standalone: &Standalone,
        glyph_variables: &mut player_glyph::State,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<()> {
        let parts = frame.begin_players_parts(catalog).map_err(missing)?;
        let mut world = begin_playing::Context {
            actors: parts.actors,
            kingdom_slots: parts.kingdom_slots,
            live_text: parts.live_text,
            application,
        };
        let mut effects = Actual {
            world: parts.world,
            catalog,
            bank: parts.bank,
            creator: parts.creator,
            ai: parts.ai,
            application,
            standalone,
            glyph_variables,
            diagnostics,
        };
        self.prepare_players_for_begin_playing(&mut world, &mut effects)
    }
    ///Actual application variant: the offline gate is borrowed from the SAME
    ///managed graph as its pre-start SAI/Arena, not supplied from another app.
    pub(crate) fn prepare_retained_players(
        &mut self,
        frame: &mut WorldFrameInputs,
        catalog: &Catalog<'_>,
        application: &GameAppState,
        glyph_variables: &mut player_glyph::State,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<()> {
        let parts = frame.begin_players_parts(catalog).map_err(missing)?;
        let mut world = begin_playing::Context {
            actors: parts.actors,
            kingdom_slots: parts.kingdom_slots,
            live_text: parts.live_text,
            application,
        };
        let mut effects = Actual {
            world: parts.world,
            catalog,
            bank: parts.bank,
            creator: parts.creator,
            ai: parts.ai,
            application,
            standalone: parts.standalone,
            glyph_variables,
            diagnostics,
        };
        self.prepare_players_for_begin_playing(&mut world, &mut effects)
    }
}
struct Actual<'a, 'source> {
    world: aa_runtime::world_revision::Witness,
    catalog: &'a Catalog<'source>,
    bank: &'a Bank,
    creator: &'a mut Creator,
    ai: AiPlayers<'a>,
    application: &'a GameAppState,
    standalone: &'a Standalone,
    glyph_variables: &'a mut player_glyph::State,
    diagnostics: &'a mut Vec<Diagnostic>,
}
impl begin_playing::Effects for Actual<'_, '_> {
    type Creator = creator_labels::KingdomHandle;
    type Glyph = player_glyph::Glyph;
    fn start_diagnostic(&mut self) -> Result<()> {
        self.diagnostics.push(Diagnostic::PreparePlayers);
        Ok(())
    }
    fn create_local_sai(
        &mut self,
        owner: &mut LocalBootstrap,
        player: &PlayerHandle,
        world: &mut begin_playing::Context<'_>,
    ) -> Result<()> {
        let mut service = self.ai.add_context(self.catalog, self.bank, world.actors);
        let mut effects = AiDiagnostic {
            diagnostics: self.diagnostics,
        };
        owner.create_sai_in_created_world_766960(
            player,
            world.actors,
            &self.world,
            self.application,
            &mut service,
            &mut effects,
        )
    }
    fn bind_diagnostic(&mut self, player: &[u16], kingdom: &[u16]) -> Result<()> {
        self.diagnostics.push(Diagnostic::BindPlayer {
            player: player.to_vec(),
            kingdom: kingdom.to_vec(),
        });
        Ok(())
    }
    fn editor_nation_text(
        &mut self,
        _: aa_runtime::load_tail_world::KingdomId,
        _: &mut begin_playing::Context<'_>,
    ) -> Result<Vec<u16>> {
        Err(missing(
            "773E30 editor Kingdom234/Nation10 current field owner is required",
        ))
    }
    fn mode_zero_option(&mut self) -> Result<u32> {
        //Only the null/non-null result is tested by773E30; it is not an option
        //integer or a fabricated Campaign pointer. The real positive owner
        //remains required by the application Campaign table.
        self.application
            .campaigns()
            .current_4f1310(self.application)
            .map(|value| u32::from(value.is_some()))
            .map_err(missing)
    }
    fn creator_set_name(
        &mut self,
        kingdom: &[u16],
        name: &[u16],
        table: &mut Localization,
    ) -> Result<Self::Creator> {
        creator_labels::set_name(self.creator, kingdom, name, table).map_err(missing)
    }
    fn player_glyph(
        &mut self,
        owner: &LocalBootstrap,
        player: &PlayerHandle,
    ) -> Result<Option<Self::Glyph>> {
        self.glyph_variables
            .glyph(
                self.catalog,
                self.bank,
                &mut PlayerRead {
                    owner,
                    world: &self.world,
                    standalone: self.standalone,
                },
                player,
            )
            .map_err(missing)
    }
    fn glyph_index(&mut self, glyph: &Self::Glyph) -> Result<u16> {
        Ok(glyph.index())
    }
    fn creator_set_glyph(&mut self, handle: &Self::Creator, text: &[u16]) -> Result<()> {
        creator_labels::set_glyph(self.creator, handle, text).map_err(missing)
    }
}
struct AiDiagnostic<'a> {
    diagnostics: &'a mut Vec<Diagnostic>,
}
impl sai_setup::Effects for AiDiagnostic<'_> {
    fn editor_nation_text(
        &mut self,
        _: aa_runtime::load_tail_world::KingdomId,
    ) -> Result<Option<Vec<u16>>> {
        Err(missing(
            "766960 editor Kingdom234/Nation10 current field owner is required",
        ))
    }
    fn diagnostic(&mut self, profile: &str, kingdom: &[u16]) -> Result<()> {
        self.diagnostics.push(Diagnostic::SetupAi {
            profile: profile.into(),
            kingdom: kingdom.to_vec(),
        });
        Ok(())
    }
}
///A lazy loan of the same player/peer and positive current World identity.
///This raw28 getter does not certify765E10/VWorld completion.
struct PlayerRead<'a> {
    owner: &'a LocalBootstrap,
    world: &'a aa_runtime::world_revision::Witness,
    standalone: &'a Standalone,
}
impl player_glyph::PlayerRead for PlayerRead<'_> {
    type Player = PlayerHandle;
    fn ai_flag_0c(&mut self, p: &PlayerHandle) -> std::result::Result<bool, String> {
        self.owner
            .player(p)
            .map(|v| v.core.ai)
            .map_err(|e| e.to_string())
    }
    fn online_manager_present(&mut self) -> std::result::Result<bool, String> {
        self.standalone.online_manager_present()
    }
    fn peer_present_04(&mut self, p: &PlayerHandle) -> std::result::Result<bool, String> {
        self.owner
            .player(p)
            .map(|v| v.core.link != PeerLink::Null)
            .map_err(|e| e.to_string())
    }
    fn online_peer_glyph(
        &mut self,
        p: &PlayerHandle,
        _: &Catalog<'_>,
        _: &Bank,
    ) -> std::result::Result<Option<player_glyph::Glyph>, String> {
        self.owner.player(p).map_err(|e| e.to_string())?;
        Err("792DC0 actual online account/PeerLink service is required".into())
    }
    fn session_state_e4(&mut self) -> std::result::Result<u32, String> {
        Ok(self.owner.session.state)
    }
    fn kingdom_present_28(&mut self, p: &PlayerHandle) -> std::result::Result<bool, String> {
        self.owner.sim.require_created_world_witness(self.world)?;
        let player = self.owner.player(p).map_err(|e| e.to_string())?;
        Ok(player.core.field_28.is_some())
    }
}
