//!Ordinary local LOAD_MAP confirmation in68F629..68F6BE. The replacement
//!frontend supplies that explicit selection, retaining its actual current
//!AppState read. Native staging UI navigation before confirmation is omitted;
//!Host/Creator/header/settings/seed remain the existing concrete path.
use super::*;
use aa_runtime::{
    application_mode::GameAppState, application_state, catalog::Catalog, definitions::Bank,
    resource_creator::SourceResource,
};
///Native captures current10C in ESI before constructing its LaunchState.
///This move-only value is that register capture, not a new application owner.
pub struct OrdinaryEntry {
    current_application: u32,
}
impl OrdinaryEntry {
    pub fn capture_current_application(state: &application_state::State) -> Result<Self> {
        let current = state.current();
        if current == 7 {
            return Err(Error::Unsupported(
                "68F629 multiplayer/currentApp7 map confirmation",
            ));
        }
        Ok(Self {
            current_application: current,
        })
    }
    pub fn into_host_input(
        self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        application: &GameAppState,
        selected: &Arc<SourceResource>,
        options: GameOptions,
    ) -> Result<launch::HostInput> {
        if application.mode() != 1 {
            return Err(Error::Unsupported(
                "ordinary selected-map confirmation requires actual current GameApp mode1",
            ));
        }
        selected
            .parsed_creator()
            .validate(catalog, bank)
            .map_err(missing)?;
        //The actual source-backed successful SourceResource carries its real
        //Header90; no filename lookup, reconstructed header or null stand-in.
        let _same_header = selected.header();
        let state = ordinary_launch(self.current_application)?;
        Ok(launch::HostInput::from_ui_choices(
            state,
            2,
            2,
            false,
            options,
            Some(selected.ids().to_vec()),
        ))
    }
}
fn ordinary_launch(current: u32) -> Result<LaunchState> {
    if current == 7 {
        return Err(Error::Unsupported(
            "68F629 multiplayer selected-map confirmation",
        ));
    }
    let mut state = constructor_launch_state();
    //Actual40F7D0→7841E0 then784130(4,1). No784160 mode-reset prefix.
    state.kind = 6;
    state.field_18 = 4;
    state.field_1c = 1;
    Ok(state)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    #[test]
    fn native_ordinary_confirmation_preserves_ctor_fields_and_rejects_multiplayer() {
        let f: Value = serde_json::from_str(include_str!(
            "../../../research/development10/application-state-menus/native-selected-confirm.json"
        ))
        .unwrap();
        let mut n = 0;
        for row in f["rows"].as_array().unwrap() {
            if row["op"] != "confirm" {
                continue;
            }
            let current = row["current"].as_u64().unwrap() as u32;
            if current == 7 {
                assert!(ordinary_launch(current).is_err());
                continue;
            }
            let s = ordinary_launch(current).unwrap();
            assert_eq!(
                json!({"kind":s.kind,"flag04":u8::from(s.flag_04),"text_length":s.text.len(),"field0c":s.field_0c,"field10":s.field_10,"flag14":u8::from(s.flag_14),"flag15":u8::from(s.flag_15),"flag16":u8::from(s.flag_16),"field18":s.field_18,"field1c":s.field_1c,"flag20":u8::from(s.flag_20),"field24":s.field_24}),
                row["output"]["launch"]
            );
            assert_eq!(row["output"]["game_type"], 2);
            n += 1;
        }
        assert_eq!(n, 9);
    }
}
