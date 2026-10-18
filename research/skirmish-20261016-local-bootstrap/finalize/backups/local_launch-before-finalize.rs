//! Actual Host input retention759A60 and75B20E..75B274 reached fields.
//! UI choices are explicit arguments, never guessed original UI defaults.
//! Selected map/GameSettings/77A950 remain their source-backed next producers.
use super::*;

/// Original Host command values read by this bounded local path. No Default:
/// the replacement frontend must supply its actual user's choices.
pub struct HostInput {
    state: LaunchState,
    map_mode: u32,
    game_type: u32,
    multiplayer: bool,
    options: GameOptions,
    map_file: Option<Vec<u16>>,
}
impl HostInput {
    pub fn from_ui_choices(
        state: LaunchState,
        map_mode: u32,
        game_type: u32,
        multiplayer: bool,
        options: GameOptions,
        map_file: Option<Vec<u16>>,
    ) -> Self {
        Self {
            state,
            map_mode,
            game_type,
            multiplayer,
            options,
            map_file,
        }
    }
    pub fn map_file(&self) -> Option<&[u16]> {
        self.map_file.as_deref()
    }
    pub fn map_mode(&self) -> u32 {
        self.map_mode
    }
    pub fn game_type(&self) -> u32 {
        self.game_type
    }
    pub fn launch_state(&self) -> &LaunchState {
        &self.state
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputStage {
    CopiedBeforeReset,
    InstallingFields,
    FieldsInstalled,
    InstallingFile,
    FileInstalled,
    InstallingSettings,
    SettingsInstalled,
    RefreshingPlayers,
    PlayersRefreshed,
    CreatingImplicitAis,
    ImplicitAisCreated,
}
pub(super) struct LoadSelection {
    pub(super) creator: Box<aa_runtime::resource_creator::Creator>,
    /// Unbound until actual5A46D0 supplies the current Session68 value. The
    /// native allocation exists earlier; this is not its null-pointer state.
    pub(super) game_settings_68: Option<aa_runtime::game_settings_value::Owner>,
}
pub(super) struct Retained {
    arguments: HostInput,
    /// Actual759A60 copies source38/3C/40/44/48 to NodeCC..DC.
    node_options: GameOptions,
    pub(super) stage: InputStage,
}
impl Retained {
    fn copy_from_input(input: HostInput) -> Self {
        Self {
            node_options: input.options.clone(),
            arguments: input,
            stage: InputStage::CopiedBeforeReset,
        }
    }
}
pub trait Effects {
    ///771240 rejected-currently-counting-down native UI message76DA60.
    fn mode_rejected_counting_down(&mut self) -> Result<()>;
    ///7745C0 logs BEFORE writing54/copying actual LaunchState4145C0.
    fn set_new_state_diagnostic(&mut self, state: u32) -> Result<()>;
}
pub(super) fn set_seed(
    session: &mut SessionFields,
    seed: u32,
    effects: &mut impl Effects,
) -> Result<()> {
    if session.launch_countdown < 1 {
        session.seed_override_100 = Some(seed);
        Ok(())
    } else {
        effects.mode_rejected_counting_down()
    }
}
impl LocalBootstrap {
    ///76E5EB..76E614 constructs the SAME CreatorLoad retained by Launch+1C.
    ///This source-backed constructor must precede the genuine fresh Reset.
    pub fn bind_load_creator_constructor(
        &mut self,
        catalog: &aa_runtime::catalog::Catalog<'_>,
        variables: &aa_runtime::creator_variables::State,
    ) -> Result<()> {
        if self.progress != Progress::Constructed
            || self.host_input.is_some()
            || self.load_selection.is_some()
        {
            return Err(missing(
                "CreatorLoad constructor requires untouched local startup owner",
            ));
        }
        let creator =
            aa_runtime::resource_creator::Creator::from_load_constructor(catalog, variables)
                .map_err(missing)?;
        self.load_selection = Some(LoadSelection {
            creator,
            game_settings_68: None,
        });
        Ok(())
    }
    pub fn selected_load_creator(&self) -> Result<&aa_runtime::resource_creator::Creator> {
        self.load_selection
            .as_ref()
            .map(|v| v.creator.as_ref())
            .ok_or_else(|| missing("actual LaunchState CreatorLoad constructor is unbound"))
    }
    pub fn current_game_settings_68(&self) -> Result<&aa_runtime::game_settings_value::Owner> {
        self.load_selection
            .as_ref()
            .and_then(|v| v.game_settings_68.as_ref())
            .ok_or_else(|| missing("actual Session68 GameSettings copy has not run"))
    }
    ///75B35F..75B36C→76B500: copy the actual selected-source GameSettings
    ///into Session68 FIRST, then5144A0 assigns the SAME LoadCreator. This
    ///bounded API requires the caller's actual selected source resource key;
    ///it does not claim the later ScenarioFind/+50 writer or77A950 launch.
    pub fn install_host_selected_game_settings(
        &mut self,
        catalog: &aa_runtime::catalog::Catalog<'_>,
        source: &Arc<aa_runtime::resource_creator::SourceResource>,
    ) -> Result<()> {
        let retained = self
            .host_input
            .as_ref()
            .ok_or_else(|| missing("selected GameSettings has no retained Host input"))?;
        if retained.stage != InputStage::FileInstalled {
            return Err(missing(
                "Host GameSettings copy is not at its native boundary",
            ));
        }
        if self.session.launch_mode != 2 {
            return Err(Error::Unsupported(
                "76B500 requires another actual Creator subtype for this map mode",
            ));
        }
        if self.session.launch_file_5c != source.ids() {
            return Err(missing(
                "Host GameSettings source resource differs from actual selected file5C",
            ));
        }
        if self.load_selection.is_none() {
            return Err(missing(
                "76B500 actual CreatorLoad constructor owner is unbound",
            ));
        }
        let bank = catalog
            .definition_bank
            .try_borrow()
            .map_err(|_| missing("actual definition Bank already borrowed"))?;
        self.host_input.as_mut().unwrap().stage = InputStage::InstallingSettings;
        let current = aa_runtime::game_settings_value::Owner::copy_source_header_5a46d0(
            catalog, &bank, source,
        )
        .map_err(missing)?;
        let selected = self.load_selection.as_mut().unwrap();
        selected.game_settings_68 = Some(current);
        selected
            .creator
            .assign_current_game_settings(
                catalog,
                &bank,
                selected.game_settings_68.as_ref().unwrap(),
            )
            .map_err(missing)?;
        self.host_input.as_mut().unwrap().stage = InputStage::SettingsInstalled;
        Ok(())
    }
    ///771180, the actual Session5C CString writer. Resource lookup and
    ///selected GameSettings assignment are later native producers.
    pub fn set_launch_file_771180(
        &mut self,
        file: &[u16],
        effects: &mut impl Effects,
    ) -> Result<()> {
        if self.session.launch_countdown < 1 {
            self.session.launch_file_5c = file.to_vec();
            Ok(())
        } else {
            effects.mode_rejected_counting_down()
        }
    }
    pub fn launch_file_5c(&self) -> &[u16] {
        &self.session.launch_file_5c
    }
    ///75B34C..75B35F ordinary selected-map slice. Random-mode configuration
    ///has an earlier positive producer and cannot be skipped through this API.
    pub fn install_host_selected_file(&mut self, effects: &mut impl Effects) -> Result<()> {
        let r = self
            .host_input
            .as_mut()
            .ok_or_else(|| missing("selected file has no actual pre-reset Host input"))?;
        if r.stage != InputStage::FieldsInstalled {
            return Err(missing(
                "Host file assignment is not at its native boundary",
            ));
        }
        if r.arguments.map_mode == 0 {
            return Err(Error::Unsupported(
                "75B27F random configuration before771180",
            ));
        }
        let file = r
            .arguments
            .map_file
            .as_ref()
            .ok_or_else(|| missing("native Host input has no selected map branch"))?
            .clone();
        r.stage = InputStage::InstallingFile;
        self.set_launch_file_771180(&file, effects)?;
        self.host_input.as_mut().unwrap().stage = InputStage::FileInstalled;
        Ok(())
    }
    ///7710E0 low-level writer reached by75DA40. Order-flow validation belongs
    ///to the real dispatcher; this setter adds no authority from Node+8.
    pub fn set_seed_7710e0(&mut self, seed: u32, effects: &mut impl Effects) -> Result<()> {
        set_seed(&mut self.session, seed, effects)
    }
    pub fn seed_override_100(&self) -> Result<u32> {
        self.session
            .seed_override_100
            .ok_or_else(|| missing("actual Session100 seed writer has not run"))
    }
    ///75AF56 clears169,759A60 copies the actual command into Node94,
    ///then751770 runs. The diagnostic reset API cannot retrofit these fields.
    pub fn reset_node_fresh_with_host_input(
        &mut self,
        input: HostInput,
        os: &mut dyn ResetOs,
        source: &InitialProperties,
        notifications: &Notifications,
    ) -> Result<()> {
        if self.progress != Progress::Constructed
            || self.host_input.is_some()
            || self.host_node.is_some()
        {
            return Err(missing("759A60 Host input requires untouched native owner"));
        }
        self.session.flags_168_16e[1] = Some(false);
        self.host_input = Some(Retained::copy_from_input(input));
        self.reset_node_fresh(os, source, notifications)
    }
    pub fn host_input(&self) -> Result<&HostInput> {
        self.host_input
            .as_ref()
            .map(|r| &r.arguments)
            .ok_or_else(|| missing("actual759A60 Host command has not been retained"))
    }
    pub fn host_input_stage(&self) -> Result<InputStage> {
        self.host_input
            .as_ref()
            .map(|r| r.stage)
            .ok_or_else(|| missing("actual759A60 Host command has not been retained"))
    }
    /// Native75B20E..75B274 after the SAME selfHost prefix. This reaches
    ///map selection next and deliberately does not assign SessionE4/F8.
    pub fn install_host_launch_fields(&mut self, effects: &mut impl Effects) -> Result<()> {
        if self.progress != Progress::LocalHostCreated {
            return Err(missing("75B20E requires actual completed selfHost prefix"));
        }
        let retained = self
            .host_input
            .as_mut()
            .ok_or_else(|| missing("75B20E has no actual pre-reset Host input"))?;
        if retained.stage != InputStage::CopiedBeforeReset {
            return Err(missing(
                "partial/completed Host field slice cannot be retried",
            ));
        }
        retained.stage = InputStage::InstallingFields;
        if self.session.launch_countdown < 1 {
            self.session.launch_mode = retained.arguments.map_mode;
        } else {
            effects.mode_rejected_counting_down()?;
        }
        effects.set_new_state_diagnostic(retained.arguments.game_type)?;
        self.session.launch_game_type = retained.arguments.game_type;
        self.session
            .launch_state
            .clone_from(&retained.arguments.state);
        self.session.multiplayer = retained.arguments.multiplayer;
        self.session
            .options
            .name
            .clone_from(&retained.node_options.name);
        self.session
            .options
            .password
            .clone_from(&retained.node_options.password);
        self.session.options.max_players = retained.node_options.max_players;
        self.session.options.field_d8 = retained.node_options.field_d8;
        self.session.options.allow_observers = retained.node_options.allow_observers;
        retained.stage = InputStage::FieldsInstalled;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    struct Log {
        calls: Vec<&'static str>,
        fail: bool,
    }
    impl Effects for Log {
        fn mode_rejected_counting_down(&mut self) -> Result<()> {
            self.calls.push("rejected");
            Ok(())
        }
        fn set_new_state_diagnostic(&mut self, _: u32) -> Result<()> {
            self.calls.push("state");
            if self.fail {
                Err(missing("required diagnostic"))
            } else {
                Ok(())
            }
        }
    }
    fn units(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }
    #[test]
    fn local_host_fields_replay_native_original_and_v14() {
        let f: Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-local-bootstrap/host-tail/native-host-fields.json"
        ))
        .unwrap();
        assert_eq!(f["passed"], true);
        assert_eq!(f["cases"].as_array().unwrap().len(), 24);
        for row in f["cases"].as_array().unwrap() {
            let mut owner = LocalBootstrap::from_native_constructors();
            let mut state = constructor_launch_state();
            state.kind = row["kind"].as_u64().unwrap() as u32;
            state.field_0c = 0x11223344;
            state.field_10 = 4;
            state.field_18 = 12;
            let options = GameOptions {
                name: units("User game\0tail"),
                password: units("secret"),
                max_players: 100,
                field_d8: 0x12345678,
                allow_observers: false,
            };
            owner.host_input = Some(Retained::copy_from_input(HostInput::from_ui_choices(
                state,
                row["input_mode"].as_u64().unwrap() as u32,
                row["input_game_type"].as_u64().unwrap() as u32,
                row["input_multiplayer"] == true,
                options,
                Some(units("explicit map input")),
            )));
            // Isolated fixture drives this reached prefix; production cannot
            //obtain this enum except the genuine earlier Host procedure.
            owner.progress = Progress::LocalHostCreated;
            owner.session.launch_countdown = row["countdown"].as_i64().unwrap() as i32;
            let mut log = Log {
                calls: Vec::new(),
                fail: false,
            };
            owner.install_host_launch_fields(&mut log).unwrap();
            let e = &row["after"];
            assert_eq!(owner.launch_mode(), e["mode"].as_u64().unwrap() as u32);
            assert_eq!(
                owner.launch_game_type(),
                e["game_type"].as_u64().unwrap() as u32
            );
            assert_eq!(
                serde_json::to_value(&owner.session.launch_state).unwrap(),
                e["state"]
            );
            assert_eq!(
                serde_json::to_value(&owner.session.options).unwrap(),
                e["options"]
            );
            assert_eq!(owner.session.multiplayer, e["multiplayer"] == true);
            assert_eq!(owner.current_phase().unwrap(), 0);
            assert_eq!(owner.application_state().unwrap(), 0);
            assert_eq!(
                log.calls,
                if row["countdown"] == 0 {
                    vec!["state"]
                } else {
                    vec!["rejected", "state"]
                }
            );
            assert_eq!(
                owner.host_input_stage().unwrap(),
                InputStage::FieldsInstalled
            );
        }
    }
    #[test]
    fn local_host_fields_refuse_missing_inputs_and_keep_prior_mode_store() {
        let mut owner = LocalBootstrap::from_native_constructors();
        owner.progress = Progress::LocalHostCreated;
        let mut log = Log {
            calls: Vec::new(),
            fail: true,
        };
        assert!(owner.install_host_launch_fields(&mut log).is_err());
        assert!(log.calls.is_empty());
        owner.host_input = Some(Retained::copy_from_input(HostInput::from_ui_choices(
            constructor_launch_state(),
            2,
            2,
            false,
            GameOptions {
                name: vec![],
                password: vec![],
                max_players: -1,
                field_d8: u32::MAX,
                allow_observers: true,
            },
            None,
        )));
        assert!(owner.install_host_launch_fields(&mut log).is_err());
        assert_eq!(owner.launch_mode(), 2);
        assert_eq!(owner.launch_game_type(), 0);
        assert_eq!(owner.current_phase().unwrap(), 0);
        assert_eq!(
            owner.host_input_stage().unwrap(),
            InputStage::InstallingFields
        );
        assert!(owner.install_host_launch_fields(&mut log).is_err());
    }
    #[test]
    fn local_seed_override_uses_actual_writer_and_preserves_rejected_old_word() {
        let f: Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-local-bootstrap/host-tail/native-seed.json"
        ))
        .unwrap();
        assert_eq!(f["passed"], true);
        assert_eq!(f["cases"].as_array().unwrap().len(), 72);
        for row in f["cases"].as_array().unwrap() {
            let mut owner = LocalBootstrap::from_native_constructors();
            assert!(owner.seed_override_100().is_err());
            owner.session.seed_override_100 = Some(0xa5a5a5a5); //explicit old-native allocation probe, not ctor default
            owner.session.launch_countdown = row["countdown"].as_i64().unwrap() as i32;
            let mut log = Log {
                calls: Vec::new(),
                fail: false,
            };
            owner
                .set_seed_7710e0(row["supplied_seed"].as_u64().unwrap() as u32, &mut log)
                .unwrap();
            assert_eq!(
                owner.seed_override_100().unwrap(),
                row["after"].as_u64().unwrap() as u32
            );
            assert_eq!(
                log.calls.len(),
                usize::from(row["countdown"].as_i64().unwrap() > 0)
            );
        }
    }
    #[test]
    fn local_launch_file_replays_native_whole_setter_and_rejection() {
        let f: Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-local-bootstrap/host-tail/native-launch-file.json"
        ))
        .unwrap();
        assert_eq!(f["passed"], true);
        assert_eq!(f["cases"].as_array().unwrap().len(), 54);
        for row in f["cases"].as_array().unwrap() {
            let mut owner = LocalBootstrap::from_native_constructors();
            assert!(owner.launch_file_5c().is_empty());
            owner.session.launch_file_5c = units("old file");
            owner.session.launch_countdown = row["countdown"].as_i64().unwrap() as i32;
            let input: Vec<u16> = row["input"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect();
            let after: Vec<u16> = row["after"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect();
            let mut log = Log {
                calls: Vec::new(),
                fail: false,
            };
            owner.set_launch_file_771180(&input, &mut log).unwrap();
            assert_eq!(owner.launch_file_5c(), after);
            assert_eq!(
                log.calls.len(),
                usize::from(row["countdown"].as_i64().unwrap() > 0)
            );
        }
    }
}
