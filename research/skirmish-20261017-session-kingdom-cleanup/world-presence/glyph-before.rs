//! Lazy glyph/presentation loan over the SAME local roster, Sim and selected
//! standalone application owner. No cached Session facts or online bool exist.
use super::*;
pub struct GlyphRoster<'a, 'owner> {
    film: FilmRoster<'a, 'owner>,
    standalone: &'a aa_runtime::standalone_network::Standalone,
}
impl<'owner> PlayersBorrow<'owner> {
    pub fn glyph_roster<'a>(
        &'a mut self,
        sim: &'a aa_runtime::live_sim::Sim,
        standalone: &'a aa_runtime::standalone_network::Standalone,
    ) -> GlyphRoster<'a, 'owner> {
        GlyphRoster {
            film: self.film_roster(sim),
            standalone,
        }
    }
}
impl aa_runtime::player_glyph::PlayerRead for GlyphRoster<'_, '_> {
    type Player = PlayerHandle;
    fn ai_flag_0c(&mut self, player: &Self::Player) -> std::result::Result<bool, String> {
        self.film
            .players
            .player_ai(player)
            .map_err(|e| e.to_string())
    }
    fn online_manager_present(&mut self) -> std::result::Result<bool, String> {
        self.standalone.online_manager_present()
    }
    fn peer_present_04(&mut self, player: &Self::Player) -> std::result::Result<bool, String> {
        Ok(self
            .film
            .players
            .player_link(player)
            .map_err(|e| e.to_string())?
            != PeerLink::Null)
    }
    fn online_peer_glyph(
        &mut self,
        player: &Self::Player,
        _: &aa_runtime::catalog::Catalog<'_>,
        _: &aa_runtime::definitions::Bank,
    ) -> std::result::Result<Option<aa_runtime::player_glyph::Glyph>, String> {
        //Validate the exact allocation even on an explicitly unsupported
        //positive service. The offline native guard does not call this getter.
        self.film
            .players
            .player_link(player)
            .map_err(|e| e.to_string())?;
        Err("792DC0 online account/level owner for actual PeerLink is unbound".into())
    }
    fn session_state_e4(&mut self) -> std::result::Result<u32, String> {
        SessionPhase::application_state(self.film.players)
    }
    fn kingdom_present_28(&mut self, player: &Self::Player) -> std::result::Result<bool, String> {
        Ok(aa_runtime::film_session::Roster::kingdom_28(&mut self.film, player)?.is_some())
    }
}
impl aa_runtime::load_kingdom_glyph::Players for GlyphRoster<'_, '_> {
    fn roster_len(&mut self) -> std::result::Result<usize, String> {
        Ok(self.film.players.player_count())
    }
    fn roster_player(&mut self, index: usize) -> std::result::Result<Self::Player, String> {
        self.film
            .players
            .player_at(index)
            .map_err(|e| e.to_string())
    }
    fn display_name_8(&mut self, player: &Self::Player) -> std::result::Result<&[u16], String> {
        self.film
            .players
            .player_display_name_8(player)
            .map_err(|e| e.to_string())
    }
    fn kingdom_765e00(
        &mut self,
        player: &Self::Player,
    ) -> std::result::Result<Option<aa_runtime::load_tail_world::KingdomId>, String> {
        aa_runtime::film_session::Roster::kingdom_28(&mut self.film, player)
    }
}
impl aa_runtime::film_header_entries::Players for GlyphRoster<'_, '_> {
    type Player = PlayerHandle;
    fn first(&mut self) -> std::result::Result<Option<Self::Player>, String> {
        aa_runtime::film_header_entries::Players::first(&mut self.film)
    }
    fn next(&mut self, player: &Self::Player) -> std::result::Result<Option<Self::Player>, String> {
        aa_runtime::film_header_entries::Players::next(&mut self.film, player)
    }
    fn ai_byte(&mut self, player: &Self::Player) -> std::result::Result<u8, String> {
        aa_runtime::film_header_entries::Players::ai_byte(&mut self.film, player)
    }
    fn kingdom_765e00(
        &mut self,
        player: &Self::Player,
    ) -> std::result::Result<Option<aa_runtime::load_tail_world::KingdomId>, String> {
        aa_runtime::film_header_entries::Players::kingdom_765e00(&mut self.film, player)
    }
    fn name_24(&mut self, player: &Self::Player) -> std::result::Result<&[u16], String> {
        aa_runtime::film_header_entries::Players::name_24(&mut self.film, player)
    }
    fn display_name_08(&mut self, player: &Self::Player) -> std::result::Result<&[u16], String> {
        aa_runtime::film_header_entries::Players::display_name_08(&mut self.film, player)
    }
    fn is_local(&mut self, player: &Self::Player) -> std::result::Result<bool, String> {
        aa_runtime::film_header_entries::Players::is_local(&mut self.film, player)
    }
}
impl aa_runtime::film_session::Roster for GlyphRoster<'_, '_> {
    type Player = PlayerHandle;
    fn count(&mut self) -> std::result::Result<usize, String> {
        aa_runtime::film_session::Roster::count(&mut self.film)
    }
    fn player_at(&mut self, index: usize) -> std::result::Result<Self::Player, String> {
        aa_runtime::film_session::Roster::player_at(&mut self.film, index)
    }
    fn id(&mut self, player: &Self::Player) -> std::result::Result<u32, String> {
        aa_runtime::film_session::Roster::id(&mut self.film, player)
    }
    fn name_24(&mut self, player: &Self::Player) -> std::result::Result<&[u16], String> {
        aa_runtime::film_session::Roster::name_24(&mut self.film, player)
    }
    fn kingdom_28(
        &mut self,
        player: &Self::Player,
    ) -> std::result::Result<Option<aa_runtime::load_tail_world::KingdomId>, String> {
        aa_runtime::film_session::Roster::kingdom_28(&mut self.film, player)
    }
    fn ai_implicit(&mut self, player: &Self::Player) -> std::result::Result<(u8, u8), String> {
        aa_runtime::film_session::Roster::ai_implicit(&mut self.film, player)
    }
}
