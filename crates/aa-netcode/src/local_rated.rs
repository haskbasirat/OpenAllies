//!770280 over the retained Session, nonnull native GameSpy manager projection
//!and current selected Creator. The manager's other fields and Internet rated
//!validation remain required on the positive BC branch, before any rule write.
use super::*;

fn requires_online_validation(mode: u32, internet_byte: u8) -> bool {
    internet_byte != 0 && matches!(mode, 0 | 2)
}
fn reads_rule(mode: u32, game_type: u32) -> bool {
    matches!(mode, 0 | 1 | 2 | 5) && matches!(game_type, 1 | 2)
}
fn commit_if_different(
    changed_c9: &mut bool,
    current: u32,
    rated: u8,
    assign: impl FnOnce() -> Result<()>,
) -> Result<()> {
    if current as u8 != rated {
        *changed_c9 = true;
        assign()?;
    }
    Ok(())
}
impl LocalBootstrap {
    ///Native770280 is reached after player publication and at launch finalize.
    ///`manager` is the SAME application owner's proved nonnull B3654C/+BC
    ///projection; offline does not mean a fabricated absent GameSpy manager.
    pub fn update_rated_770280(
        &mut self,
        catalog: &aa_runtime::catalog::Catalog<'_>,
        bank: &aa_runtime::definitions::Bank,
        manager: &aa_runtime::game_spy_manager_gate::State,
    ) -> Result<()> {
        //Session170 is this retained, constructor-created PropertyTable. It is
        //never inferred from its entry count and is not a nullable substitute.
        let _properties = &self.session.properties;
        let mode = self.session.launch_mode;
        if requires_online_validation(mode, manager.current_byte_bc()) {
            return Err(Error::Unsupported(
                "770280 positive GameSpyBC rated validation79CB60/property/CRC owners",
            ));
        }
        //The native branch initializes its candidate byte to zero. No online
        //validation was executed, so this is its actual computed value.
        let rated = 0u8;
        if !reads_rule(mode, self.session.launch_game_type) {
            return Ok(());
        }
        if mode != 2 {
            return Err(Error::Unsupported(
                "770280 current Creator owner for non-load launch mode",
            ));
        }
        let selection = self
            .load_selection
            .as_mut()
            .ok_or_else(|| missing("770280 selected Load Creator constructor is unbound"))?;
        selection.creator.validate(catalog, bank).map_err(missing)?;
        let Some(record) = selection.creator.rules().find_record("RATED") else {
            return Ok(());
        };
        //Native reads the value byte even on a non-bool descriptor. It sets
        //C9 first;511AF0 subsequently rejects wrong descriptor kind.
        let current = record.value();
        let ids = record.ids().to_owned();
        commit_if_different(&mut self.session.changed[1], current, rated, || {
            selection
                .creator
                .assign_bool_rule_current(catalog, bank, &ids, rated)
                .map(|_| ())
                .map_err(missing)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_rated_outer_branches_replay() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-local-bootstrap/rated/native-rated.json"
        ))
        .unwrap();
        assert_eq!(fixture["passed"], true);
        for row in fixture["cases"].as_array().unwrap() {
            let mode = row["mode"].as_u64().unwrap() as u32;
            let game_type = row["game_type"].as_u64().unwrap() as u32;
            let bc = row["internet_bc"].as_u64().unwrap() as u8;
            assert_eq!(requires_online_validation(mode, bc), row["online_reached"]);
            if !requires_online_validation(mode, bc) {
                assert_eq!(reads_rule(mode, game_type), row["rule_lookup_reached"]);
                let mut changed = false;
                let mut invoked = false;
                if reads_rule(mode, game_type) && row["variant"] != "missing" {
                    commit_if_different(
                        &mut changed,
                        row["before_word"].as_u64().unwrap() as u32,
                        0,
                        || {
                            invoked = true;
                            Ok(())
                        },
                    )
                    .unwrap();
                }
                assert_eq!(changed, row["changed_c9"]);
                assert_eq!(invoked, changed);
            }
        }
    }
    #[test]
    fn native_c9_store_precedes_reached_assignment_failure() {
        let mut dirty = false;
        let result = commit_if_different(&mut dirty, 0xaabbcc80, 0, || {
            Err(missing("explicit reached setter failure"))
        });
        assert!(result.is_err());
        assert!(dirty);
        commit_if_different(&mut dirty, 0xaabbcc00, 0, || {
            panic!("equality must not invoke setter")
        })
        .unwrap();
        assert!(dirty); //no native clear of a previously set C9
    }
}
