//!776580 over the actual retained BC players and selected Creator. Native
//!773710 publication,770280,766560 and7664D0 preserve their partial effects.
//!The pre-World ProfileService loans the SAME application's source Arena.
use super::*;
pub trait Effects {
    fn no_profiles_404ed0(&mut self) -> Result<()>;
    fn creating_player(&mut self, id: u32, ai: bool, implicit: bool) -> Result<()>;
    fn kingdom_diagnostic(&mut self, index: usize, id: &[u16]) -> Result<()>;
    fn player_diagnostic(&mut self, name: &[u16], kingdom: &[u16], has_profile: bool)
    -> Result<()>;
}
fn eligible(independent: u8, ai_allowed: u8, phase_seven: bool) -> bool {
    independent == 0 && (phase_seven || ai_allowed != 0)
}
struct Names<'a> {
    owner: &'a mut LocalBootstrap,
    text: &'a Localization,
}
impl crate::lobby::player_name::World for Names<'_> {
    type Player = usize;
    fn first_named(&mut self, name: &[u16]) -> Result<Option<usize>> {
        Ok(self
            .owner
            .session
            .players
            .iter()
            .position(|p| crate::session::wcs_eq(&p.core.name, name)))
    }
    fn duplicate_name(&mut self, name: &[u16], number: u32) -> Result<Vec<u16>> {
        let key: Vec<u16> = crate::lobby::DUPLICATE_NAME_KEY.encode_utf16().collect();
        let format = self
            .text
            .display(&key)
            .map_err(|e| missing(e.to_string()))?;
        crate::lobby::player_name::format(&format, name, number)
    }
    fn store_name_7664f0(&mut self, player: &usize, name: Vec<u16>) -> Result<()> {
        let p = self
            .owner
            .session
            .players
            .get_mut(*player)
            .ok_or_else(|| missing("766560 actual target player is absent"))?;
        p.core.name = name;
        p.core.flag_2c = true;
        Ok(())
    }
}
impl LocalBootstrap {
    ///The ordinary existing-map Host reaches776580 before776B20 binds+50.
    ///No task/World/battle-ready capability is returned by this caller slice.
    pub fn create_host_implicit_ais_776580(
        &mut self,
        catalog: &aa_runtime::catalog::Catalog<'_>,
        bank: &aa_runtime::definitions::Bank,
        manager: &aa_runtime::game_spy_manager_gate::State,
        strings: &aa_runtime::application_strings::State,
        text: &Localization,
        profiles: &mut dyn aa_runtime::load_sai_players::ProfileService,
        effects: &mut impl Effects,
    ) -> Result<()> {
        if self.host_input_stage()? != launch::InputStage::PlayersRefreshed {
            return Err(missing(
                "776580 is not at the actual fresh Host finalize boundary",
            ));
        }
        self.host_input.as_mut().unwrap().stage = launch::InputStage::CreatingImplicitAis;
        self.implicit_ais(catalog, bank, manager, strings, text, profiles, effects)?;
        self.host_input.as_mut().unwrap().stage = launch::InputStage::ImplicitAisCreated;
        Ok(())
    }
    pub(super) fn implicit_ais(
        &mut self,
        catalog: &aa_runtime::catalog::Catalog<'_>,
        bank: &aa_runtime::definitions::Bank,
        manager: &aa_runtime::game_spy_manager_gate::State,
        strings: &aa_runtime::application_strings::State,
        text: &Localization,
        profiles: &mut dyn aa_runtime::load_sai_players::ProfileService,
        effects: &mut impl Effects,
    ) -> Result<()> {
        if !matches!(self.session.launch_mode, 0 | 1 | 2 | 5) {
            return Ok(());
        }
        if profiles.profile_count().map_err(missing)? == 0 {
            effects.no_profiles_404ed0()?;
            return Ok(());
        }
        if self.session.launch_mode != 2 {
            return Err(Error::Unsupported("776580 non-load current Creator owner"));
        }
        self.selected_load_creator()?
            .validate(catalog, bank)
            .map_err(missing)?;
        let phase_seven = self.session.launch_state.kind == 7
            || (self.session.launch_state.kind == 6 && self.session.launch_state.field_18 == 7);
        let mut i = 0;
        while i < self.selected_load_creator()?.parameters().kingdoms.len() {
            let row = &self.selected_load_creator()?.parameters().kingdoms[i];
            if eligible(row.byte_10, row.byte_29, phase_seven)
                && !self
                    .session
                    .players
                    .iter()
                    .any(|p| crate::session::wcs_eq(&p.core.kingdom, &row.id))
            {
                let id = self
                    .session
                    .last_player_id
                    .ok_or_else(|| missing("776580 SessionE8 has no Reset producer"))?
                    .wrapping_add(1);
                self.session.last_player_id = Some(id);
                //74B420 returns actual Node2C8. Only the proved selfHost
                //allocation is admitted, never false→null or guessed peer ID.
                let node = self
                    .host_node
                    .as_ref()
                    .ok_or_else(|| missing("776580 has no retained Node"))?;
                if node.stage() != crate::local_host::Stage::SelfHost || !node.server_is_self() {
                    return Err(Error::Unsupported(
                        "776580 actual foreign/null server allocation",
                    ));
                }
                let core = PlayerFields::from_native_fields(
                    id,
                    PeerLink::Node,
                    true,
                    true,
                    self.node_host,
                );
                let view = construct_view(None, text)?;
                let slot = self.session.players.len();
                self.session.players.push(OwnedPlayer {
                    allocation_lifetime: Arc::new(()),
                    core,
                    vector_44: Vec::new(),
                    view: Some(view),
                });
                self.session.created = Some(id);
                self.session.changed[0] = true;
                effects.creating_player(id, true, true)?;
                self.update_rated_770280(catalog, bank, manager)?;
                //Actual7766E0 reads parameter+2C AFTER player publication and
                //rated update; +14 belongs to another parameter string.
                let profile_key = &self.selected_load_creator()?.parameters().kingdoms[i].string_2c;
                let end = profile_key
                    .iter()
                    .position(|&u| u == 0)
                    .unwrap_or(profile_key.len());
                let selected = profiles
                    .find_profile(&profile_key[..end])
                    .map_err(missing)?;
                let (profile_id, display) = match selected {
                    Some(profile) => (
                        profile.ids().encode_utf16().collect(),
                        profile.name().encode_utf16().collect(),
                    ),
                    None => (Vec::new(), strings.default_b43b48().to_vec()),
                };
                let player = &mut self.session.players[slot].core;
                player.text_14 = profile_id;
                player.flag_2c = true; //7664B0, staticB43B40 on missing
                crate::lobby::player_name::set(&mut Names { owner: self, text }, &slot, &display)?;
                let kingdom = self.selected_load_creator()?.parameters().kingdoms[i]
                    .id
                    .clone();
                let player = &mut self.session.players[slot].core;
                player.kingdom = kingdom;
                player.flag_2c = true; //7664D0
            }
            i += 1;
        }
        for (i, row) in self
            .selected_load_creator()?
            .parameters()
            .kingdoms
            .iter()
            .enumerate()
        {
            effects.kingdom_diagnostic(i, &row.id)?;
        }
        for player in &self.session.players {
            effects.player_diagnostic(
                &player.core.name,
                &player.core.kingdom,
                !player.core.text_14.is_empty(),
            )?;
        }
        self.session.changed[0] = true;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_implicit_eligibility_and_actual_ai_constructor_fields_replay() {
        let doc: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-local-bootstrap/implicit-ai/native-implicit.json"
        ))
        .unwrap();
        assert_eq!(doc["passed"], true);
        for row in doc["cases"].as_array().unwrap() {
            let phase = row["launch_kind"] == 7;
            let selected = row["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| {
                    eligible(
                        r["independent"].as_u64().unwrap() as u8,
                        r["enabled"].as_u64().unwrap() as u8,
                        phase,
                    )
                })
                .count();
            assert_eq!(selected + 1, row["players"].as_array().unwrap().len());
            for p in row["players"].as_array().unwrap().iter().skip(1) {
                let actor = PlayerFields::from_native_fields(
                    p["id"].as_u64().unwrap() as u32,
                    PeerLink::Node,
                    true,
                    true,
                    PeerLink::Node,
                );
                assert!(actor.ai && actor.implicit && !actor.film && actor.flag_2d);
                assert_eq!(actor.status, p["status"].as_u64().unwrap() as u32);
            }
        }
    }
}
