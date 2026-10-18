//! Saved4CF0F0 ->570020 over the actual owned Economy allocation. Focused
//! components and ancestor callbacks stay visible through short borrowed
//! views; every Result path returns the same Box and keeps earlier writes.
use crate::saved_organization_actions::SavedActions;
use crate::{economy_refresh as refresh, saved_economy_values::View};
use aa_actors::{ActorId, components::ComponentSlot, ground_states::EconomyState};
use aa_mapworld::restore::{RestoreDefinitions, RestoredComponent, RestoredRecord};
use aa_numeric::Precision;

fn error(message: impl std::fmt::Display) -> aa_combat::Error {
    aa_combat::Error::World(message.to_string())
}

// Historical isolated native entry boundary used only by its original probe.
// Production now continues through the actual refresh coordinator below.
#[cfg(test)]
fn entry(
    actor: ActorId,
    economy: Option<&EconomyState>,
    owner_word: impl FnOnce(ActorId) -> aa_combat::Result<u32>,
) -> aa_combat::Result<()> {
    let Some(economy) = economy else {
        return Ok(());
    };
    // A mismatched owned component is a broken runtime identity, not evidence
    // that the requested actor has no Economy or that refresh succeeded.
    if economy.header.owner != actor {
        return Err(error("570020 saved Economy header owner identity mismatch"));
    }
    //570041 dispatches GActor virtual+108 (4D1E80 reads Actor+E8), and
    //57004B returns immediately on null before reading flags or vectors.
    if owner_word(economy.header.owner)? == 0 {
        return Ok(());
    }
    Err(error(
        "570020 saved Economy nonnull-owner resource refresh is unbound",
    ))
}

pub(crate) fn refresh_member(
    actions: &mut SavedActions<'_, '_, '_>,
    actor: ActorId,
) -> aa_combat::Result<()> {
    let pc = actions.context.precision;
    refresh_borrowed(actions, actor, &mut [], pc)
}

fn refresh_error(message: impl std::fmt::Display) -> refresh::Error {
    refresh::Error::World(message.to_string())
}

/// `held` contains actual Economies temporarily executing above this frame,
/// not copies or fabricated absent slots. Source/value callbacks cannot
/// remove actors, so the original slot remains available for reinsertion.
fn refresh_borrowed(
    actions: &mut SavedActions<'_, '_, '_>,
    actor: ActorId,
    held: &mut [(ActorId, &mut EconomyState)],
    pc: Precision,
) -> aa_combat::Result<()> {
    if held.iter().any(|(id, _)| *id == actor) {
        return Err(error(
            "570020 recursive refresh revisits an active Economy; invalid parent cycle",
        ));
    }
    let economy = match actions
        .component(actor, ComponentSlot::Economy)
        .map_err(error)?
    {
        None => None,
        Some(RestoredComponent::Economy(value)) => Some(&**value),
        Some(_) => return Err(error("570020 saved Economy slot payload mismatch")),
    };
    let Some(economy) = economy else {
        return Ok(());
    };
    if economy.header.owner != actor {
        return Err(error("570020 saved Economy header owner identity mismatch"));
    }
    // Capture the one native initial virtual+108 result. The coordinator's
    // first owner() consumes this value; removal callbacks reread Actor+E8.
    let owner = actions.word(actor, 0xe8).map_err(error)?;
    if owner == 0 {
        return Ok(());
    }
    let source_pc = actions.source_precision().map_err(error)?;
    let count = actions
        .catalog
        .resource_domain_count(source_pc)
        .map_err(error)? as usize;
    let Some(RestoredComponent::Economy(mut component)) = actions
        .shell_mut(actor)
        .map_err(error)?
        .components
        .remove(ComponentSlot::Economy)
    else {
        unreachable!("validated Economy slot")
    };
    let result = {
        let mut world = Refresh {
            actions,
            held,
            storage_owner: actor,
            first_owner: Some(ActorId::from_raw(owner)),
            resource_count: count,
        };
        refresh::refresh(&mut component, &mut world, pc).map_err(error)
    };
    let replaced = actions
        .shell_mut(actor)
        .expect("refresh callbacks retain actor identity")
        .components
        .insert(
            ComponentSlot::Economy,
            RestoredComponent::Economy(component),
        );
    assert!(
        replaced.is_none(),
        "refresh callbacks cannot replace the borrowed Economy"
    );
    result
}

struct Refresh<'a, 'held, 'catalog, 'world, 'source> {
    actions: &'a mut SavedActions<'catalog, 'world, 'source>,
    held: &'a mut [(ActorId, &'held mut EconomyState)],
    storage_owner: ActorId,
    first_owner: Option<ActorId>,
    resource_count: usize,
}
impl Refresh<'_, '_, '_, '_, '_> {
    fn adjust_flows(
        &mut self,
        owner: ActorId,
        values: &[aa_economy::F32Bits],
        add: bool,
        providers: bool,
    ) -> refresh::Result<()> {
        let catalog = self.actions.catalog;
        let pc = self.actions.context.precision;
        let source_pc = self.actions.source_precision().map_err(refresh_error)?;
        let definitions = catalog
            .native_resource_economy(source_pc)
            .map_err(refresh_error)?;
        let consumption_count = crate::saved_kingdom_flows::consumption_count(&definitions)?;
        let Some(RestoredRecord::Kingdom(record)) = self.actions.context.registry.value_mut(owner)
        else {
            return Err(refresh_error(
                "570020 actual owner Kingdom identity is missing",
            ));
        };
        let kingdom = record
            .runtime
            .as_mut()
            .and_then(|runtime| runtime.derived.as_mut())
            .ok_or_else(|| {
                refresh_error("570020 actual owner Kingdom derived storage is missing")
            })?;
        let mut cap = || crate::saved_kingdom_flows::provider_maximum(catalog, pc);
        let mut world = crate::saved_kingdom_flows::BorrowedKingdom {
            kingdom,
            resources: definitions.catalog(),
            consumption_count,
            provider_maximum: &mut cap,
        };
        let precision = match pc {
            Precision::Single24 => aa_economy::X87Precision::Single24,
            Precision::Double53 => aa_economy::X87Precision::Double53,
            Precision::Extended64 => aa_economy::X87Precision::Extended64,
        };
        if providers {
            aa_economy::borrowed_flows::adjust_providers(&mut world, values, add, precision)?;
        } else {
            aa_economy::borrowed_flows::adjust_consumption(&mut world, values, add, precision)?;
        }
        Ok(())
    }
}
impl refresh::World for Refresh<'_, '_, '_, '_, '_> {
    type Kingdom = ActorId;
    fn resource_count(&self) -> usize {
        self.resource_count
    }
    fn owner(
        &mut self,
        actor: ActorId,
        component: &mut EconomyState,
    ) -> refresh::Result<Option<ActorId>> {
        if actor != self.storage_owner || component.header.owner != actor {
            return Err(refresh_error("focused Economy owner identity mismatch"));
        }
        if let Some(owner) = self.first_owner.take() {
            return Ok(Some(owner));
        }
        let word = self.actions.word(actor, 0xe8).map_err(refresh_error)?;
        Ok((word != 0).then_some(ActorId::from_raw(word)))
    }
    fn upkeep_blocked(
        &mut self,
        actor: ActorId,
        component: &mut EconomyState,
    ) -> refresh::Result<bool> {
        let pc = self.actions.context.precision;
        View::new(self.actions, component, self.held, pc)
            .upkeep_blocked(actor)
            .map_err(refresh_error)
    }
    fn upkeep_values(
        &mut self,
        actor: ActorId,
        component: &mut EconomyState,
        pc: Precision,
    ) -> refresh::Result<Vec<aa_body::F32>> {
        View::new(self.actions, component, self.held, pc)
            .cost_or_upkeep(actor, aa_actors::economy_values::VectorKind::Upkeep)
            .map_err(refresh_error)
    }
    fn provides(&mut self, actor: ActorId, component: &mut EconomyState) -> refresh::Result<bool> {
        let pc = self.actions.context.precision;
        View::new(self.actions, component, self.held, pc)
            .provides(actor)
            .map_err(refresh_error)
    }
    fn provided_values(
        &mut self,
        actor: ActorId,
        component: &mut EconomyState,
        write_member_cache: bool,
        pc: Precision,
    ) -> refresh::Result<Vec<aa_body::F32>> {
        View::new(self.actions, component, self.held, pc)
            .production(actor, write_member_cache)
            .map_err(refresh_error)
    }
    fn cost_values(
        &mut self,
        actor: ActorId,
        component: &mut EconomyState,
        pc: Precision,
    ) -> refresh::Result<Vec<aa_body::F32>> {
        View::new(self.actions, component, self.held, pc)
            .cost_or_upkeep(actor, aa_actors::economy_values::VectorKind::Cost)
            .map_err(refresh_error)
    }
    fn kingdom_economy(&mut self, _: ActorId) -> refresh::Result<&mut aa_economy::Economy> {
        Err(refresh_error(
            "saved refresh uses actual borrowed Kingdom vectors, not a replacement Economy",
        ))
    }
    fn adjust_consumption(
        &mut self,
        owner: ActorId,
        values: &[aa_economy::F32Bits],
        add: bool,
    ) -> refresh::Result<()> {
        self.adjust_flows(owner, values, add, false)
    }
    fn adjust_providers(
        &mut self,
        owner: ActorId,
        values: &[aa_economy::F32Bits],
        add: bool,
    ) -> refresh::Result<()> {
        self.adjust_flows(owner, values, add, true)
    }
    fn parent_organization(
        &mut self,
        actor: ActorId,
        component: &mut EconomyState,
    ) -> refresh::Result<Option<ActorId>> {
        let pc = self.actions.context.precision;
        View::new(self.actions, component, self.held, pc)
            .parent_organization(actor)
            .map_err(refresh_error)
    }
    fn refresh_parent(
        &mut self,
        actor: ActorId,
        current: &mut EconomyState,
        pc: Precision,
    ) -> refresh::Result<()> {
        let mut held = Vec::new();
        held.try_reserve_exact(self.held.len() + 1)
            .map_err(|_| refresh::Error::Allocation)?;
        for (id, value) in self.held.iter_mut() {
            held.push((*id, &mut **value));
        }
        held.push((self.storage_owner, current));
        refresh_borrowed(self.actions, actor, &mut held, pc).map_err(refresh_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::cell::Cell;

    fn snapshot_economy(value: &EconomyState) -> Value {
        json!({"flags":[u8::from(value.flag_10),u8::from(value.flag_11)],
            "resources":value.resources.iter().map(|v|&v.values).collect::<Vec<_>>()})
    }
    fn snapshot_kingdom(actions: &SavedActions<'_, '_, '_>, id: ActorId) -> Value {
        let RestoredRecord::Kingdom(record) = &actions.context.registry.get(id).unwrap().value
        else {
            panic!("actual Kingdom")
        };
        let shell = record.runtime.as_ref().unwrap().derived.as_ref().unwrap();
        json!({"effective_provided":shell.vectors[&0x1a0].values,
            "raw_provided":shell.vectors[&0x1ac].values,
            "consumed":shell.vectors[&0x1b8].values,
            "stockpiled":shell.vectors[&0x1c4].values})
    }

    fn fixture() -> Value {
        serde_json::from_slice(
            &std::fs::read(
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../research/skirmish-20261016-saved-economy-entry/native-entry.json"),
            )
            .unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn native_entry_branches_keep_all_component_data_unchanged() {
        let fixture = fixture();
        let rows = fixture["cases"].as_array().unwrap();
        assert_eq!(rows.len(), 36);
        for row in rows {
            let actor = ActorId::from_raw(7);
            let mut economy = EconomyState::new(actor, 3).unwrap();
            economy.flag_10 = true;
            economy.flag_11 = true;
            economy.resources[2].values[1] = aa_body::F32(0x7fc12345);
            let before = economy.clone();
            let queried = Cell::new(0);
            let present = row["input"]["economy_present"].as_bool().unwrap();
            let owner = row["input"]["kingdom"].as_u64().unwrap() as u32;
            let result = entry(actor, present.then_some(&economy), |id| {
                assert_eq!(id, actor);
                queried.set(queried.get() + 1);
                Ok(owner)
            });
            assert_eq!(
                result.is_ok(),
                row["output"]["outcome"] == "returned",
                "{row}"
            );
            assert_eq!(queried.get(), usize::from(present));
            assert_eq!(economy, before);
            assert_eq!(row["output"]["component_unchanged"], true);
            assert_eq!(row["output"]["actor_unchanged"], true);
        }
    }
    #[test]
    fn identity_or_missing_owner_never_becomes_success() {
        let actor = ActorId::from_raw(7);
        let economy = EconomyState::new(ActorId::from_raw(8), 0).unwrap();
        let mismatch = entry(actor, Some(&economy), |_| {
            panic!("identity guard must precede owner query")
        });
        assert!(
            mismatch
                .unwrap_err()
                .to_string()
                .contains("identity mismatch")
        );
        let economy = EconomyState::new(actor, 0).unwrap();
        let missing = entry(actor, Some(&economy), |_| {
            Err(error("missing initialized E8"))
        });
        assert!(
            missing
                .unwrap_err()
                .to_string()
                .contains("missing initialized E8")
        );
        entry(actor, None, |_| panic!("absent Economy cannot query owner")).unwrap();
    }
    #[test]
    fn prior_native_null_owner_refresh_preserves_all_vectors_and_flags() {
        let old: Value = serde_json::from_slice(
            &std::fs::read(
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../research/phase7-economy-refresh/native-refresh-fixtures.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let mut count = 0;
        for row in old["cases"].as_array().unwrap().iter().filter(|row| {
            row["input"]["name"]
                .as_str()
                .unwrap()
                .starts_with("null_initial_owner")
        }) {
            let input = &row["input"];
            let mut economy = EconomyState::new(ActorId::from_raw(7), 5).unwrap();
            economy.flag_10 = input["flags"][0] == 1;
            economy.flag_11 = input["flags"][1] == 1;
            for (vector, words) in economy
                .resources
                .iter_mut()
                .zip(input["resources"].as_array().unwrap())
            {
                vector.values = words
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| aa_body::F32(v.as_u64().unwrap() as u32))
                    .collect();
            }
            entry(ActorId::from_raw(7), Some(&economy), |_| Ok(0)).unwrap();
            let actual = json!({"flags":[u8::from(economy.flag_10),u8::from(economy.flag_11)],
                "resources":economy.resources.iter().map(|v| &v.values).collect::<Vec<_>>()});
            assert_eq!(actual, row["output"]["component"]);
            assert_eq!(row["output"]["kingdoms"], input["kingdoms"]);
            count += 1;
        }
        assert_eq!(count, 3);
    }

    #[test]
    #[ignore = "requires AA_GAME_ROOT original/v14 assets"]
    fn actual_saved_refresh_retains_allocations_and_reports_missing_lifecycle() {
        use crate::{
            combat_source_completion,
            combat_source_receipt::VerifiedStartupReceipt,
            catalog::Catalog,
            localization::Localization,
            restoration::{Attempt, LoadBindings},
            settings::Settings,
        };
        use aa_mapworld::restore::{OrganizationAttachContext, RestoredRecord};
        use aa_numeric::Precision;
        use aa_rules::{
            actors::ActorCatalog,
            sources::{SourceLimits, V14Sources},
            tgi::Limits,
        };
        let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
        let mut sources = V14Sources::open(
            root.join("Data.rwd"),
            root.join("reconstruction/native-game/runtime-expanded-v14"),
            SourceLimits {
                max_member_bytes: 64 * 1024 * 1024,
            },
        )
        .unwrap();
        let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
        let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
        let resources = std::sync::Arc::new(
            aa_rules::resources::ResourceCatalog::load_v14(&mut sources).unwrap(),
        );
        let actors = ActorCatalog::load_v14(&mut sources).unwrap();
        let map = aa_mapworld::MapArchive::from_bytes(
            std::fs::read(
                root.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
            )
            .unwrap(),
        )
        .unwrap();
        let mut attempt = Attempt::load_with_source_setup(
            &map,
            &actors,
            settings,
            text,
            Precision::Double53,
            u64::MAX,
            aa_mapworld::restore::RestoreMode::InspectPartial,
            LoadBindings {
                native_load_mode: Some(1),
                resources: Some(resources.clone()),
                ..LoadBindings::default()
            },
            |catalog| {
                let receipt=VerifiedStartupReceipt::verify_embedded_reference(catalog,&mut sources)?;
                combat_source_completion::install(catalog,&mut sources,receipt.context(),receipt.csv(),receipt.globals())?;
                combat_source_completion::compute(catalog)
            },
        )
        .unwrap();
        let owner = attempt
            .registry
            .iter()
            .find_map(|entry| {
                let entity = entry.value.loaded()?;
                entity
                    .shell
                    .actor
                    .as_ref()?
                    .components
                    .present(ComponentSlot::Organization)
                    .then_some(entry.id)
            })
            .expect("real restored Organization");
        let ids: Vec<_> = attempt
            .registry
            .iter()
            .filter_map(|entry| entry.value.loaded()?.shell.actor.as_ref().map(|_| entry.id))
            .collect();
        let taken = std::mem::replace(
            attempt.registry.value_mut(owner).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Actor(mut entity) = taken else {
            panic!("Organization actor");
        };
        let actor = entity.shell.actor.as_mut().unwrap();
        let Some(RestoredComponent::Organization(mut organization)) =
            actor.components.remove(ComponentSlot::Organization)
        else {
            panic!("Organization payload");
        };
        let mut catalog = Catalog::new(&actors);
        catalog.native_load_mode = Some(1);
        catalog.settings = Some(attempt.settings.clone());
        catalog.definition_bank=std::cell::RefCell::new(std::mem::take(&mut attempt.definitions));
        catalog.bind_resource_catalog(resources.clone()).unwrap();
        let mut actions = SavedActions {
            catalog: &catalog,
            context: OrganizationAttachContext {
                inventory: &attempt.inventory,
                owner,
                owner_actor: actor,
                owner_definition: &entity.definition,
                owner_thing_flags: entity.shell.thing.flags,
                owner_thing_word_c: entity.shell.thing.word_c,
                owner_object_byte_4c: entity.shell.object.as_ref().unwrap().byte_4c,
                organization: &mut organization,
                registry: &mut attempt.registry,
                precision: Precision::Double53,
            }
            .into(),
            active_mind: None,
        };
        let mut counts = [0usize; 3];
        let mut candidate = None;
        let mut observed = Vec::new();
        for id in ids.iter().copied() {
            let ptr = match actions.component(id, ComponentSlot::Economy).unwrap() {
                Some(RestoredComponent::Economy(economy)) => {
                    Some(&**economy as *const EconomyState)
                }
                _ => None,
            };
            let before = match actions.component(id, ComponentSlot::Economy).unwrap() {
                Some(RestoredComponent::Economy(economy)) => Some((**economy).clone()),
                None => None,
                _ => panic!("actual Economy slot type"),
            };
            let words = actions.shell(id).unwrap().words.clone();
            let native_early_return = match &before {
                None => {
                    counts[0] += 1;
                    true
                }
                Some(economy) => {
                    assert_eq!(economy.header.owner, id);
                    candidate = Some(id);
                    let null = actions.word(id, 0xe8).unwrap() == 0;
                    counts[if null { 1 } else { 2 }] += 1;
                    null
                }
            };
            let kingdom = before.as_ref().and_then(|_| {
                let raw = actions.word(id, 0xe8).unwrap();
                (raw != 0).then_some(ActorId::from_raw(raw))
            });
            let before_kingdom = kingdom.map(|id| snapshot_kingdom(&actions, id));
            let parent = match actions.component(id, ComponentSlot::Element).unwrap() {
                Some(RestoredComponent::Element(e)) => e.organization,
                None => None,
                _ => panic!("Element payload"),
            };
            let parent_economy = parent.map(|id| {
                actions
                    .component(id, ComponentSlot::Economy)
                    .unwrap()
                    .is_some()
            });
            let result = refresh_member(&mut actions, id);
            if native_early_return {
                assert!(result.is_ok(), "{id:?}: {result:?}");
            }
            let after = match actions.component(id, ComponentSlot::Economy).unwrap() {
                Some(RestoredComponent::Economy(economy)) => Some((**economy).clone()),
                None => None,
                _ => panic!("retained Economy slot type"),
            };
            let after_ptr = match actions.component(id, ComponentSlot::Economy).unwrap() {
                Some(RestoredComponent::Economy(economy)) => {
                    Some(&**economy as *const EconomyState)
                }
                _ => None,
            };
            assert_eq!(
                after_ptr, ptr,
                "same canonical Economy allocation on every Result"
            );
            if native_early_return {
                assert_eq!(after, before);
            }
            assert_eq!(actions.shell(id).unwrap().words, words);
            observed.push(
                json!({"actor":id.raw(),"definition":actions.source_id(id).unwrap(),
                "early_return":native_early_return,"error":result.err().map(|e|e.to_string()),
                "parent":parent.map(ActorId::raw),"parent_economy_present":parent_economy,
                "before":before,"after":after,"before_kingdom":before_kingdom,
                "after_kingdom":kingdom.map(|id|snapshot_kingdom(&actions,id))}),
            );
        }
        let candidate = candidate.expect("actual saved Economy");
        println!(
            "actual saved entry census: absent={}, null owner={}, nonnull owner={}",
            counts[0], counts[1], counts[2]
        );
        if let Some(path) = std::env::var_os("AA_SAVED_REFRESH_OUT") {
            std::fs::write(path, serde_json::to_vec_pretty(&observed).unwrap()).unwrap();
        }
        for row in &observed {
            if !row["early_return"].as_bool().unwrap() && !row["error"].is_null() {
                assert!(
                    row["error"]
                        .as_str()
                        .is_some_and(|s| s.contains("saved Kingdom vector")),
                    "{row}"
                );
            }
        }
        assert_eq!(
            observed
                .iter()
                .filter(|v| !v["early_return"].as_bool().unwrap() && v["error"].is_null())
                .map(|v| v["actor"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            vec![4100, 8131]
        );
        assert_eq!(
            observed.iter().filter(|v| !v["error"].is_null()).count(),
            27
        );
        if std::env::var_os("AA_SAVED_REFRESH_CAPTURE").is_none() {
            let path=std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261016-saved-economy-integration/native-natural-hq-refresh.json");
            let fixture: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            for native in fixture["cases"].as_array().unwrap() {
                let row = observed
                    .iter()
                    .find(|row| row["actor"] == native["actor"])
                    .unwrap();
                let before = &row["before"];
                let after = &row["after"];
                assert_eq!(
                    native["input"]["flags"],
                    json!([
                        u8::from(before["flag_10"].as_bool().unwrap()),
                        u8::from(before["flag_11"].as_bool().unwrap())
                    ])
                );
                assert_eq!(
                    native["input"]["resources"],
                    json!(
                        before["resources"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|v| &v["values"])
                            .collect::<Vec<_>>()
                    )
                );
                assert_eq!(native["input"]["kingdoms"][0], row["before_kingdom"]);
                assert_eq!(
                    native["output"]["component"]["flags"],
                    json!([
                        u8::from(after["flag_10"].as_bool().unwrap()),
                        u8::from(after["flag_11"].as_bool().unwrap())
                    ])
                );
                assert_eq!(
                    native["output"]["component"]["resources"],
                    json!(
                        after["resources"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|v| &v["values"])
                            .collect::<Vec<_>>()
                    )
                );
                assert_eq!(native["output"]["kingdom"], row["after_kingdom"]);
            }
        }
        // Explicit initialized-storage fixture: execute only the already
        // proven original Init flow-reset fragment on the existing Kingdom
        // owners. Production refresh never performs this reset or asserts
        // that the rest of Kingdom Init has run.
        let independent = ids
            .iter()
            .copied()
            .find(|id| {
                actions.source_id(*id).unwrap() == "usa_corps"
                    && matches!(
                        actions.component(*id, ComponentSlot::Economy).unwrap(),
                        Some(RestoredComponent::Economy(_))
                    )
                    && match actions.component(*id, ComponentSlot::Element).unwrap() {
                        None => true,
                        Some(RestoredComponent::Element(e)) => e.organization.is_none(),
                        _ => false,
                    }
            })
            .expect("actual independent Economy actor");
        let base = match actions
            .component(independent, ComponentSlot::Economy)
            .unwrap()
        {
            Some(RestoredComponent::Economy(e)) => (**e).clone(),
            _ => unreachable!(),
        };
        let kingdom = ActorId::from_raw(actions.word(independent, 0xe8).unwrap());
        let kingdom_ids: Vec<_> = actions
            .context
            .registry
            .iter()
            .filter_map(|e| matches!(e.value, RestoredRecord::Kingdom(_)).then_some(e.id))
            .collect();
        let mut cases = Vec::new();
        for pc in [
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ] {
            actions.context.precision = pc;
            actions.context.source_precision = pc;
            let definitions = actions.catalog.native_resource_economy(pc).unwrap();
            let count = definitions.resource_count();
            for id in &kingdom_ids {
                let Some(RestoredRecord::Kingdom(record)) = actions.context.registry.value_mut(*id)
                else {
                    unreachable!()
                };
                crate::kingdom_economy::reset_flow_vectors(
                    record.runtime.as_mut().unwrap().derived.as_mut().unwrap(),
                    count as i32,
                )
                .unwrap();
            }
            let Some(RestoredComponent::Economy(mut component)) = actions
                .shell_mut(independent)
                .unwrap()
                .components
                .remove(ComponentSlot::Economy)
            else {
                unreachable!()
            };
            *component = base.clone();
            let ptr = &*component as *const EconomyState;
            let before = snapshot_economy(&component);
            let (blocked, provides, upkeep, provided, cost) = {
                let mut view = View::new(&mut actions, &mut component, &mut [], pc);
                assert!(view.parent_organization(independent).unwrap().is_none());
                let blocked = view.upkeep_blocked(independent).unwrap();
                let provides = view.provides(independent).unwrap();
                let upkeep = if blocked {
                    vec![aa_body::F32::ZERO; count]
                } else {
                    view.cost_or_upkeep(independent, aa_actors::economy_values::VectorKind::Upkeep)
                        .unwrap()
                };
                let provided = if provides {
                    view.production(independent, false).unwrap()
                } else {
                    vec![aa_body::F32::ZERO; count]
                };
                let cost = view
                    .cost_or_upkeep(independent, aa_actors::economy_values::VectorKind::Cost)
                    .unwrap();
                (blocked, provides, upkeep, provided, cost)
            };
            actions.shell_mut(independent).unwrap().components.insert(
                ComponentSlot::Economy,
                RestoredComponent::Economy(component),
            );
            let before_kingdom = snapshot_kingdom(&actions, kingdom);
            let input = json!({"name":format!("actual_{}_{}",independent.raw(),pc.significand_bits()),
                "precision":format!("{pc:?}"),"catalog":definitions.catalog(),
                "kingdoms":[before_kingdom,before_kingdom],
                "limits":{"provided_maximum":crate::saved_kingdom_flows::provider_maximum(actions.catalog,pc).unwrap(),"stockpiled_maximum":resources.stockpiled_resource_max.f32_bits},
                "flags":before["flags"],"resources":before["resources"],
                "block_reason":if blocked {"destructed"} else {"none"},
                "parent":false,"owners":[0],"provides":[provides],"upkeep":upkeep,"provided":provided,"cost":cost,"mutations":[]});
            refresh_member(&mut actions, independent).unwrap();
            let Some(RestoredComponent::Economy(after)) = actions
                .component(independent, ComponentSlot::Economy)
                .unwrap()
            else {
                unreachable!()
            };
            assert_eq!(&**after as *const EconomyState, ptr);
            let output = json!({"component":snapshot_economy(after),"kingdom":snapshot_kingdom(&actions,kingdom)});
            cases.push(json!({"actor":independent.raw(),"definition":actions.source_id(independent).unwrap(),"input":input,"rust_output":output}));
        }
        actions.context.precision = Precision::Double53;
        actions.context.source_precision = Precision::Double53;
        let directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../research/skirmish-20261016-saved-economy-integration");
        if std::env::var_os("AA_SAVED_REFRESH_CAPTURE").is_some() {
            std::fs::write(
                directory.join("actual-initialized-input.json"),
                serde_json::to_vec_pretty(&cases).unwrap(),
            )
            .unwrap();
        } else {
            let expected: Value = serde_json::from_slice(
                &std::fs::read(directory.join("native-initialized-refresh.json")).unwrap(),
            )
            .unwrap();
            for (case, expected) in cases.iter().zip(expected["cases"].as_array().unwrap()) {
                assert_eq!(case["input"], expected["input"]);
                assert_eq!(case["rust_output"], expected["output"]);
            }
            assert_eq!(cases.len(), expected["cases"].as_array().unwrap().len());
        }
        // Deliberate in-memory graph/error fixture, NOT a natural saved link
        // or gameplay-parity claim: point an existing Element at another real
        // Economy owner, then force a later parent flow read to fail. Both
        // actual allocations must return while child/parent prefix stores stay.
        let child = ActorId::from_raw(4100);
        let child_ptr = match actions.component(child, ComponentSlot::Economy).unwrap() {
            Some(RestoredComponent::Economy(e)) => &**e as *const EconomyState,
            _ => panic!("HQ Economy"),
        };
        let parent_ptr = match actions
            .component(independent, ComponentSlot::Economy)
            .unwrap()
        {
            Some(RestoredComponent::Economy(e)) => &**e as *const EconomyState,
            _ => panic!("corps Economy"),
        };
        let old_parent = match actions
            .shell_mut(child)
            .unwrap()
            .components
            .get_mut(ComponentSlot::Element)
        {
            Some(RestoredComponent::Element(e)) => e.organization.replace(independent),
            _ => panic!("HQ Element"),
        };
        for id in [child, independent] {
            let Some(RestoredComponent::Economy(e)) = actions
                .shell_mut(id)
                .unwrap()
                .components
                .get_mut(ComponentSlot::Economy)
            else {
                unreachable!()
            };
            e.flag_10 = false;
            e.flag_11 = false;
            e.resources[2].values.fill(aa_body::F32(0x3f800000));
            e.resources[1].values.fill(aa_body::F32(0x3f800000));
        }
        let Some(RestoredRecord::Kingdom(record)) = actions.context.registry.value_mut(kingdom)
        else {
            unreachable!()
        };
        record
            .runtime
            .as_mut()
            .unwrap()
            .derived
            .as_mut()
            .unwrap()
            .vectors
            .get_mut(&0x1b8)
            .unwrap()
            .values
            .clear();
        let failure = refresh_member(&mut actions, child).unwrap_err().to_string();
        assert!(failure.contains("saved Kingdom vector 1b8[0]"), "{failure}");
        let Some(RestoredComponent::Economy(child_after)) =
            actions.component(child, ComponentSlot::Economy).unwrap()
        else {
            unreachable!()
        };
        assert_eq!(&**child_after as *const EconomyState, child_ptr);
        assert_eq!(
            child_after.resources[2].values,
            vec![
                aa_body::F32(1132068864),
                aa_body::F32::ZERO,
                aa_body::F32::ZERO,
                aa_body::F32::ZERO,
                aa_body::F32::ZERO,
                aa_body::F32::ZERO,
                aa_body::F32::ZERO
            ]
        );
        assert!(!child_after.flag_11);
        let Some(RestoredComponent::Economy(parent_after)) = actions
            .component(independent, ComponentSlot::Economy)
            .unwrap()
        else {
            unreachable!()
        };
        assert_eq!(&**parent_after as *const EconomyState, parent_ptr);
        assert_eq!(
            json!(parent_after.resources[1].values),
            cases[1]["input"]["upkeep"]
        );
        assert!(!parent_after.flag_11, "failed add must not set the flag");
        assert!(
            parent_after.resources[2]
                .values
                .iter()
                .all(|v| v.0 == 0x3f800000),
            "parent cost occurs after failed upkeep add"
        );
        if let Some(RestoredComponent::Element(e)) = actions
            .shell_mut(child)
            .unwrap()
            .components
            .get_mut(ComponentSlot::Element)
        {
            e.organization = old_parent;
        }
        println!(
            "nested injected-parent error retained both allocations, child Cost store and parent Upkeep assignment"
        );
        // Explicit null/missing/identity probes use the same authoritative
        // loaded payload. All changes are to this test's in-memory world.
        actions.shell_mut(candidate).unwrap().words.insert(0xe8, 0);
        refresh_member(&mut actions, candidate).unwrap();
        actions.shell_mut(candidate).unwrap().words.remove(&0xe8);
        assert!(
            refresh_member(&mut actions, candidate)
                .unwrap_err()
                .to_string()
                .contains("uninitialized")
        );
        let shell = actions.shell_mut(candidate).unwrap();
        let Some(RestoredComponent::Economy(economy)) =
            shell.components.get_mut(ComponentSlot::Economy)
        else {
            panic!("Economy retained");
        };
        economy.header.owner = ActorId::NULL;
        assert!(
            refresh_member(&mut actions, candidate)
                .unwrap_err()
                .to_string()
                .contains("identity mismatch")
        );
        actions
            .shell_mut(candidate)
            .unwrap()
            .components
            .remove(ComponentSlot::Economy);
        // Absent Economy must not read the still-missing E8 word.
        refresh_member(&mut actions, candidate).unwrap();
    }
}
