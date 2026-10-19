//!7791F0 fresh ordinary-human refresh over real Session/Creator owners.
//! The77E5E0 info lists are retained constructor-empty owners, not absent
//! callbacks. Positive info-list population and Nation/Faction writers remain
//! required when their actual native branches are reached.
use super::*;

pub(super) struct StagingData {
    major_count_04: u32,
    kingdom_count_08: u32,
    info_14: Vec<Information>,
    teams_1c: Vec<Team>,
}
enum Information {}
enum Team {}
impl StagingData {
    pub(super) fn from_native_constructor() -> Self {
        Self {
            major_count_04: 0,
            kingdom_count_08: 0,
            info_14: Vec::new(),
            teams_1c: Vec::new(),
        }
    }
    fn require_constructor_lists(&self) -> Result<()> {
        if self.major_count_04 != 0
            || self.kingdom_count_08 != 0
            || !self.info_14.is_empty()
            || !self.teams_1c.is_empty()
        {
            return Err(Error::Unsupported(
                "positive Session14 staging information producer",
            ));
        }
        Ok(())
    }
    fn record_info_77e430(&self) -> Result<Option<bool>> {
        self.require_constructor_lists()?;
        //77DFD0,77E0C0,77E430 walk these actual zero-length owned arrays.
        Ok(None)
    }
    pub(super) fn position_77e1f0(&self, name: &[u16]) -> Result<Option<(u32, u32)>> {
        if prefix(name).is_empty() {
            return Ok(Some((u32::MAX, u32::MAX)));
        }
        self.require_constructor_lists()?;
        //Nonempty name and actual+4/+8==0 return false, outputs untouched.
        Ok(None)
    }
}
fn prefix(s: &[u16]) -> &[u16] {
    &s[..s.iter().position(|&u| u == 0).unwrap_or(s.len())]
}
fn first_available<'a>(
    records: impl Iterator<Item = (usize, &'a [u16], u8, u8)>,
    claimed: impl Fn(&[u16]) -> bool,
) -> Option<usize> {
    records
        .filter(|(_, _, independent, human)| *independent == 0 && *human != 0)
        .find_map(|(index, id, _, _)| (!claimed(id)).then_some(index))
}
fn reference_ids(
    catalog: &aa_runtime::catalog::Catalog<'_>,
    manager: &str,
    ids: &[u16],
) -> Result<Option<Vec<u16>>> {
    let ids = prefix(ids);
    if ids.is_empty() {
        return Ok(None);
    }
    let text = String::from_utf16(ids)
        .map_err(|_| missing("definition reference contains unsupported UTF16"))?;
    let registry = catalog
        .actors
        .native_data_registry(manager)
        .ok_or_else(|| missing(format!("{manager} registry is absent")))?;
    let Some(found) = registry.find(&text) else {
        return Ok(None);
    };
    if !found.reloads.is_empty() {
        return Err(Error::Unsupported(
            "assignment reloaded native definition timing",
        ));
    }
    catalog
        .actors
        .native_data_node(found)
        .map_err(|e| missing(e.to_string()))?;
    Ok(Some(found.ids.encode_utf16().collect()))
}
fn reference_present(
    catalog: &aa_runtime::catalog::Catalog<'_>,
    manager: &str,
    ids: &[u16],
) -> Result<bool> {
    Ok(reference_ids(catalog, manager, ids)?.is_some())
}
pub trait Effects {
    fn assigning(
        &mut self,
        player_id: u32,
        player_name: &[u16],
        kingdom: &[u16],
        nation: &[u16],
        faction: &[u16],
    ) -> Result<()>;
    fn check_nation(&mut self, kingdom: &[u16], nation: &[u16]) -> Result<()>;
    fn check_faction(&mut self, kingdom: &[u16], faction: &[u16]) -> Result<()>;
}
impl LocalBootstrap {
    ///The initial Host75B371 refresh, after actual Session68/current Creator
    ///copy and before77A950. A reached required service retains all preceding
    ///player/Handicap/changed-byte stores; a partial refresh cannot be retried.
    pub fn refresh_host_players_7791f0(
        &mut self,
        catalog: &aa_runtime::catalog::Catalog<'_>,
        effects: &mut impl Effects,
    ) -> Result<()> {
        if self.host_input_stage()? != launch::InputStage::SettingsInstalled {
            return Err(missing(
                "7791F0 is not at the completed selected-settings boundary",
            ));
        }
        self.host_input.as_mut().unwrap().stage = launch::InputStage::RefreshingPlayers;
        if !matches!(self.session.launch_mode, 0 | 1 | 2 | 5) {
            self.host_input.as_mut().unwrap().stage = launch::InputStage::PlayersRefreshed;
            return Ok(());
        }
        for player in &mut self.session.players {
            player.core.kingdom.clear();
            player.core.flag_2c = true;
        }
        let data = &self.assignment_data_14;
        let flag_169 = self.session.flags_168_16e[1]
            .ok_or_else(|| missing("actual Session169 is uninitialized"))?;
        if !self.session.multiplayer && flag_169 {
            data.require_constructor_lists()?;
            let wanted = data.kingdom_count_08.wrapping_add(1) as i32;
            if (self.session.players.len() as i64) < i64::from(wanted) {
                return Err(Error::Unsupported(
                    "779290 implicit AI SessionPlayer creation",
                ));
            }
        }
        let selected = self
            .load_selection
            .as_mut()
            .ok_or_else(|| missing("7791F0 actual selected Creator is unbound"))?;
        let current = selected
            .game_settings_68
            .as_ref()
            .ok_or_else(|| missing("7791F0 actual Session68 is unbound"))?;
        let bank = catalog
            .definition_bank
            .try_borrow()
            .map_err(|_| missing("actual Bank already borrowed"))?;
        current.validate(catalog, &bank).map_err(missing)?;
        selected.creator.validate(catalog, &bank).map_err(missing)?;
        let ordinary =
            !matches!(self.session.launch_game_type, 3 | 4) && self.session.launch_kind_60 != 5;
        if ordinary {
            //The actual common-mode4981A0 payload flag read by511860. Saved
            //mode restoration is a separate positive path, never guessedfalse.
            let saved = aa_runtime::map_resource_mode::world_rule_mode(
                current.rules().to_serialized().mode,
            )
            .map_err(missing)?
            .saved_state;
            if saved && matches!(self.session.launch_kind_60, 3 | 4) {
                return Err(Error::Unsupported("7788B0 saved-source player assignment"));
            }
            //Both requested-assignment passes call77DDD0 on the SAME actual
            //constructor-empty Team array. Its count0 returns empty for every
            //requested index before any player/Creator write.
            data.require_constructor_lists()?;
        }
        for i in 0..self.session.players.len() {
            let p = &self.session.players[i].core;
            if !p.kingdom.is_empty()
                || !(p.flag_38 || p.field_40 != u32::MAX || p.field_3c != u32::MAX)
            {
                continue;
            }
            if !ordinary {
                self.session.players[i].core.kingdom.clear();
                self.session.players[i].core.flag_2c = true;
                continue;
            }
            let parameters = selected.creator.parameters();
            let index = first_available(
                parameters
                    .kingdoms
                    .iter()
                    .enumerate()
                    .map(|(i, k)| (i, k.id.as_slice(), k.byte_10, k.byte_28)),
                |id| {
                    self.session
                        .players
                        .iter()
                        .any(|p| prefix(&p.core.kingdom) == prefix(id))
                },
            );
            let Some(index) = index else {
                self.session.players[i].core.kingdom.clear();
                self.session.players[i].core.flag_2c = true;
                continue;
            };
            if data.record_info_77e430()? == Some(false) {
                return Err(missing(
                    "constructor information returned unexpected closed record",
                ));
            }
            let k = &selected.creator.parameters().kingdoms[index];
            let id = k.id.clone();
            effects.assigning(p.id, &p.name, &id, &p.text_30, &p.text_34)?;
            if p.ai {
                return Err(Error::Unsupported("777F00 AI old/new Handicap assignment"));
            }
            aa_runtime::creator_human_assignment::clear_handicap(
                catalog,
                &bank,
                &mut selected.creator,
                index,
            )
            .map_err(missing)?;
            let p = &mut self.session.players[i].core;
            p.kingdom = id.clone();
            p.flag_2c = true;
            let mut team = u32::MAX;
            let mut position = u32::MAX;
            let located = data.position_77e1f0(&id)?;
            if let Some((t, k)) = located {
                team = t;
                position = k;
            }
            p.flag_38 = located.is_none();
            p.field_40 = team;
            p.field_3c = position;
            aa_runtime::creator_human_assignment::clear_handicap(
                catalog,
                &bank,
                &mut selected.creator,
                index,
            )
            .map_err(missing)?;
            self.session.changed[0] = true;
            effects.check_nation(&id, &p.text_30)?;
            let base = current
                .parameters()
                .kingdoms
                .iter()
                .find(|k| prefix(&k.id) == prefix(&id));
            if let Some(base) = base {
                if !reference_present(catalog, "Nation", &base.nation)? {
                    if !prefix(&p.text_30).is_empty() {
                        return Err(Error::Unsupported("777CD0 nonempty Nation Find/write"));
                    }
                    aa_runtime::creator_human_assignment::store_null_nation(
                        catalog,
                        &bank,
                        &mut selected.creator,
                        index,
                    )
                    .map_err(missing)?;
                    let nested_faction = reference_ids(
                        catalog,
                        "Faction",
                        &selected.creator.parameters().kingdoms[index].faction,
                    )?
                    .unwrap_or_default();
                    effects.check_faction(&id, &nested_faction)?;
                    aa_runtime::creator_human_assignment::clear_faction_null_nation(
                        catalog,
                        &bank,
                        &mut selected.creator,
                        index,
                    )
                    .map_err(missing)?;
                    aa_runtime::creator_human_assignment::mark_modified(
                        catalog,
                        &bank,
                        &mut selected.creator,
                        index,
                    )
                    .map_err(missing)?;
                }
            }
            effects.check_faction(&id, &p.text_34)?;
            if let Some(base) = base {
                let active = &selected.creator.parameters().kingdoms[index];
                if !reference_present(catalog, "Nation", &active.nation)? {
                    aa_runtime::creator_human_assignment::clear_faction_null_nation(
                        catalog,
                        &bank,
                        &mut selected.creator,
                        index,
                    )
                    .map_err(missing)?;
                } else if !reference_present(catalog, "Faction", &base.faction)? {
                    if !prefix(&p.text_34).is_empty() {
                        return Err(Error::Unsupported(
                            "777610 nonempty Faction Find/allowed-Nation write",
                        ));
                    }
                    aa_runtime::creator_human_assignment::clear_faction_and_mark(
                        catalog,
                        &bank,
                        &mut selected.creator,
                        index,
                    )
                    .map_err(missing)?;
                }
            }
        }
        //7744C0 walks the SAME roster. Human records never enter its destroy
        //list; positive unassigned AI destruction needs its true owned service.
        if self
            .session
            .players
            .iter()
            .any(|p| p.core.kingdom.is_empty() && p.core.ai)
        {
            return Err(Error::Unsupported(
                "7744C0 unassigned AI removal/destructor",
            ));
        }
        self.host_input.as_mut().unwrap().stage = launch::InputStage::PlayersRefreshed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fresh_refresh_selection_replays_native_whole_function_rows() {
        let f: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-local-bootstrap/player-refresh/native-refresh.json"
        ))
        .unwrap();
        assert_eq!(f["passed"], true);
        assert_eq!(f["cases"].as_array().unwrap().len(), 120);
        for row in f["cases"].as_array().unwrap() {
            let records: Vec<(Vec<u16>, u8, u8)> = row["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| {
                    (
                        r["id"].as_str().unwrap().encode_utf16().collect(),
                        r["independent"].as_u64().unwrap() as u8,
                        r["human"].as_u64().unwrap() as u8,
                    )
                })
                .collect();
            let index = first_available(
                records
                    .iter()
                    .enumerate()
                    .map(|(i, r)| (i, r.0.as_slice(), r.1, r.2)),
                |id| prefix(id).is_empty(),
            );
            let kingdom = index.map(|i| records[i].0.clone()).unwrap_or_default();
            assert_eq!(serde_json::to_value(kingdom).unwrap(), row["kingdom"]);
            let data = StagingData::from_native_constructor();
            assert_eq!(data.record_info_77e430().unwrap(), None);
            assert_eq!(data.position_77e1f0(&[65]).unwrap(), None);
            assert_eq!(
                data.position_77e1f0(&[]).unwrap(),
                Some((u32::MAX, u32::MAX))
            );
            assert_eq!(row["constructor_staging_lists_unchanged"], true);
        }
    }
}
