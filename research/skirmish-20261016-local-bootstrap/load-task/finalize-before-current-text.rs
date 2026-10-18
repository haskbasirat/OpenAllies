//!776B20 concrete ordinary selected-map caller, sharing the retained Session,
//!BC roster, source cache, RNG and SAME current LoadCreator allocation.
use super::*;
pub trait Effects: implicit_ai::Effects {
    ///404F60 occurs before the implicit-player or ScenarioFind side effects.
    fn finalize_diagnostic(&mut self, file: &[u16]) -> Result<()>;
}
struct ActualPlayers<'a> {
    session: &'a SessionFields,
}
impl aa_runtime::creator_launch_parameters::Players for ActualPlayers<'_> {
    type Player = usize;
    fn first_kingdom_match(&mut self, name: &[u16]) -> std::result::Result<Option<usize>, String> {
        Ok(self
            .session
            .players
            .iter()
            .position(|p| crate::session::wcs_eq(&p.core.kingdom, name)))
    }
    fn application_state_e4(&mut self) -> std::result::Result<u32, String> {
        Ok(self.session.state)
    }
    fn ai_byte_0c(&mut self, p: &usize) -> std::result::Result<u8, String> {
        self.session
            .players
            .get(*p)
            .map(|p| u8::from(p.core.ai))
            .ok_or_else(|| "776B20 actual BC player disappeared".into())
    }
}
fn choose_seed(override_word: u32, rng: &mut NativeRng) -> u32 {
    if override_word == 0 {
        rng.next_u32() % u32::MAX
    } else {
        override_word
    }
}
impl LocalBootstrap {
    ///The actual whole ordinary existing-map FinalizeLaunchSettings path.
    ///A task/World-ready capability is not returned;775EE0 follows this call.
    ///The preexisting diagnostic/implicit API cannot be used to bypass native
    ///ordering: this entry requires the preceding7791F0 boundary itself.
    pub fn finalize_host_launch_776b20(
        &mut self,
        catalog: &aa_runtime::catalog::Catalog<'_>,
        bank: &aa_runtime::definitions::Bank,
        depot: &mut aa_runtime::resource_depot::Application,
        app: &aa_runtime::application_mode::GameAppState,
        manager: &aa_runtime::game_spy_manager_gate::State,
        text: &Localization,
        profiles: &mut dyn aa_runtime::load_sai_players::ProfileService,
        effects: &mut impl Effects,
    ) -> Result<()> {
        if self.host_input_stage()? != launch::InputStage::PlayersRefreshed {
            return Err(missing(
                "776B20 requires the actual completed7791F0 Host boundary",
            ));
        }
        if self.session.launch_mode != 2 || self.session.launch_game_type != 2 {
            return Err(Error::Unsupported(
                "776B20 concrete caller currently requires selected-map ordinary skirmish",
            ));
        }
        self.host_input.as_mut().unwrap().stage = launch::InputStage::Finalizing;
        effects.finalize_diagnostic(&self.session.launch_file_5c)?;
        if self.session.phase != 13 && self.session.launch_state.kind != 13 {
            self.implicit_ais(
                catalog,
                bank,
                manager,
                app.strings(),
                text,
                profiles,
                effects,
            )?;
        }
        self.finalize_selected_suffix(catalog, bank, depot, app)?;
        self.host_input.as_mut().unwrap().stage = launch::InputStage::Finalized;
        Ok(())
    }
    fn finalize_selected_suffix(
        &mut self,
        catalog: &aa_runtime::catalog::Catalog<'_>,
        bank: &aa_runtime::definitions::Bank,
        depot: &mut aa_runtime::resource_depot::Application,
        app: &aa_runtime::application_mode::GameAppState,
    ) -> Result<()> {
        //59F570 increments/reopens the actual cached resource before+50.
        let resource = depot
            .find_scenario(catalog, &self.session.launch_file_5c)
            .map_err(missing)?;
        let selection = self
            .load_selection
            .as_mut()
            .ok_or_else(|| missing("776B20 actual LoadCreator is unbound"))?;
        selection
            .creator
            .assign_source_resource(catalog, bank, resource)
            .map_err(missing)?;
        let override_word = self.session.seed_override_100.ok_or_else(|| {
            missing("776B20 reached uninitialized Session100; actual7710E0 writer required")
        })?;
        let seed = choose_seed(override_word, &mut self.session.rng);
        selection
            .creator
            .assign_rng_seed_776b20(catalog, bank, seed)
            .map_err(missing)?;
        aa_runtime::creator_launch_parameters::ordinary_skirmish(
            catalog,
            bank,
            &mut selection.creator,
            &mut ActualPlayers {
                session: &self.session,
            },
        )
        .map_err(missing)?;
        //511A60 is first-match/current IDS. Absent descriptors bypass their
        //later global read;511AF0 retains no-op behavior for wrong kinds.
        if let Some(record) = selection.creator.rules().find_record("MULTIPLAYER") {
            let ids = record.ids().to_owned();
            let value = u8::from(self.session.multiplayer);
            selection
                .creator
                .assign_bool_rule_current(catalog, bank, &ids, value)
                .map_err(missing)?;
        }
        if let Some(record) = selection.creator.rules().find_record("DynamicCampaign") {
            let ids = record.ids().to_owned();
            let value = u8::from(app.mode() == 4);
            selection
                .creator
                .assign_bool_rule_current(catalog, bank, &ids, value)
                .map_err(missing)?;
        }
        Ok(())
    }
    ///Continue a caller which already executed the actual separate776580
    ///entry. This method certifies only the suffix after that call; it makes
    ///no claim that a prior404F60 diagnostic was emitted in whole776B20 order.
    pub fn finalize_host_suffix_after_implicit(
        &mut self,
        catalog: &aa_runtime::catalog::Catalog<'_>,
        bank: &aa_runtime::definitions::Bank,
        depot: &mut aa_runtime::resource_depot::Application,
        app: &aa_runtime::application_mode::GameAppState,
    ) -> Result<()> {
        if self.host_input_stage()? != launch::InputStage::ImplicitAisCreated
            || self.session.launch_mode != 2
            || self.session.launch_game_type != 2
        {
            return Err(missing(
                "776B20 suffix requires actual ordinary776580 completion",
            ));
        }
        self.host_input.as_mut().unwrap().stage = launch::InputStage::Finalizing;
        self.finalize_selected_suffix(catalog, bank, depot, app)?;
        self.host_input.as_mut().unwrap().stage = launch::InputStage::Finalized;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_seed_branch_replays_all_96_native_rows() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-local-bootstrap/finalize/native-finalize.json"
        ))
        .unwrap();
        for row in fixture["cases"].as_array().unwrap() {
            let mut rng = NativeRng::new(row["rng_before"].as_u64().unwrap() as u32);
            assert_eq!(
                choose_seed(row["override"].as_u64().unwrap() as u32, &mut rng),
                row["seed"].as_u64().unwrap() as u32
            );
            assert_eq!(rng.state, row["rng_after"].as_u64().unwrap() as u32);
        }
    }
}
