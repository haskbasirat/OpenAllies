use super::*;
struct Missing;
impl Reentry for Missing {}

#[test]
fn native_state_count_order_and_lazy_rereads_match() {
    use serde_json::{Value, json};
    struct World {
        input: Value,
        bits: u64,
        count: i32,
        trace: Vec<Value>,
    }
    impl actor_registration::World for World {
        fn registered_state_count(&mut self) -> Result<i32> {
            Ok(self.count)
        }
        fn state_bits(&mut self) -> Result<StateBits> {
            Ok(StateBits {
                low: self.bits as u32,
                high: (self.bits >> 32) as u32,
            })
        }
        fn state_reenter(&mut self, index: u32) -> Result<()> {
            self.trace.push(json!(["enter", index, 0]));
            if self.input["stop"].as_u64() == Some(index as u64) {
                return Err(Error::Unsupported("fixture state effect"));
            }
            if let Some(mutations) = self.input["mutations"].as_array() {
                for m in mutations {
                    if m["index"].as_u64() == Some(index as u64) {
                        self.bits = m["bits"].as_u64().unwrap();
                        if let Some(c) = m["count"].as_i64() {
                            self.count = c as i32;
                        }
                    }
                }
            }
            Ok(())
        }
        fn grid_register(&mut self) -> Result<()> {
            self.trace.push(json!(["grid"]));
            if self.input["stop_grid"] == true {
                return Err(Error::Unsupported("fixture grid effect"));
            }
            Ok(())
        }
    }
    let proof: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-load-actor-registration/native-registration.json"
    ))
    .unwrap();
    for table in proof["table_runs"].as_array().unwrap() {
        assert_eq!(
            table["records"].as_array().unwrap().len(),
            aa_actors::state::DESCRIPTORS.len()
        );
        for (row, d) in table["records"]
            .as_array()
            .unwrap()
            .iter()
            .zip(aa_actors::state::DESCRIPTORS)
        {
            assert_eq!(row["name"], d.name);
            assert_eq!(row["index"], d.index);
            assert_eq!(
                row["group"].as_u64().unwrap() as u32,
                d.group.map(u32::from).unwrap_or(u32::MAX)
            );
            assert_eq!(row["exclusive"], d.exclusive);
        }
    }
    for row in proof["cases"].as_array().unwrap() {
        let input = row["input"].clone();
        let mut w = World {
            bits: input["bits"].as_u64().unwrap(),
            count: input["count"].as_i64().unwrap() as i32,
            input,
            trace: vec![],
        };
        let result = actor_registration::register(&mut w);
        assert_eq!(
            result.is_ok(),
            row["output"]["result"] == "complete",
            "{row}"
        );
        assert_eq!(json!(w.trace), row["output"]["trace"], "{row}");
        assert_eq!(json!(w.bits), row["output"]["bits"], "{row}");
        assert_eq!(
            json!(w.count as u32),
            row["output"]["stored_count"],
            "{row}"
        );
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn actual_first_kingdom_registration_uses_the_same_source_record_and_spatial_membership() {
    use crate::{
        battle_terrain::{BattleTerrain, Source as TerrainSource},
        combat_source_completion as completion,
        combat_source_receipt::VerifiedStartupReceipt,
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        restore_lifecycle::{ScopedManagedLoad, TaskKind},
        settings::Settings,
    };
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use std::sync::Arc;
    struct PartialThenRetire;
    impl Reentry for PartialThenRetire {
        fn state_reenter(
            &mut self,
            _: &Catalog<'_>,
            record: &mut Record<'_>,
            index: u32,
            source: aa_rules::states::ActorStateInputs,
            _: Precision,
        ) -> Result<()> {
            assert_eq!(index, 0);
            assert_eq!(source.id, "mind_attacking");
            let flags = record.word(8)?;
            record.set_word(8, flags ^ 2)?;
            record.registry.remove(record.owner).map_err(error)?;
            Err(Error::Unsupported(
                "fixture partial state write before required stop",
            ))
        }
    }
    struct SessionFixture;
    impl SessionPhase for SessionFixture {
        fn current_phase(&self) -> std::result::Result<u32, String> {
            panic!("4D62F0 grid path must not read SessionF8")
        }
        fn application_state(&self) -> std::result::Result<u32, String> {
            Ok(3)
        }
    }
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let header =
        aa_mapworld::header::MapHeader::decode(&map.decode_chunk("MAP_HEADER", u64::MAX).unwrap())
            .unwrap();
    let world = WorldState::decode(&map.decode_chunk("GWORLD", u64::MAX).unwrap()).unwrap();
    let mut outputs = vec![];
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
            .unwrap()
            .begin(TaskKind::Normal)
            .unwrap();
        let mut point = None;
        let mut attempt = Attempt::load_with_source_setup(
            &map,
            &actors,
            settings.clone(),
            text.clone(),
            Precision::Double53,
            u64::MAX,
            aa_mapworld::restore::RestoreMode::InspectPartial,
            LoadBindings {
                native_load_mode: Some(1),
                inactive_load: Some(inactive),
                resources: Some(Arc::clone(&resources)),
            },
            |catalog| {
                let proof =
                    VerifiedStartupReceipt::verify_embedded_reference(catalog, &mut sources)?;
                point = Some(PointShape::from_verified_startup_reference(
                    catalog, &proof,
                )?);
                completion::install(
                    catalog,
                    &mut sources,
                    proof.context(),
                    proof.csv(),
                    proof.globals(),
                )?;
                completion::compute(catalog)
            },
        )
        .unwrap();
        let mut catalog = Catalog::new(&actors);
        catalog.settings = Some(attempt.settings.clone());
        catalog.definition_bank = std::cell::RefCell::new(std::mem::take(&mut attempt.definitions));
        let id = ActorId::from_raw(781);
        let mut held = std::mem::replace(
            attempt.registry.value_mut(id).unwrap(),
            RestoredRecord::Loading,
        );
        let ids = state_ids(&catalog).unwrap();
        assert_eq!(ids.len(), 42);
        let terrain_source = TerrainSource::bind(&mut sources, &actors, &settings, pc).unwrap();
        let terrain = BattleTerrain::load(&map, &terrain_source).unwrap();
        let mut spatial = LoadSpatial::new(&terrain.bound, pc).unwrap();
        let mut zones = crate::load_zone_services::before_first_record(
            &terrain.bound,
            spatial.index(),
            &header,
        )
        .unwrap();
        assert!(zones.tiles.iter().all(|t| t.set.is_none()));
        let point = point.unwrap();
        let session = SessionFixture;
        let shell_address = match &held {
            RestoredRecord::Kingdom(k) => &k.runtime.as_ref().unwrap().base as *const _ as usize,
            _ => panic!("actual Kingdom"),
        };
        assert!(
            register(
                &catalog,
                id,
                &mut held,
                &mut attempt.registry,
                GridContext {
                    zones: &mut zones,
                    spatial: &spatial,
                    point: &point,
                    session: &session,
                    world: &world,
                    city: None,
                    live_world48: None,
                },
                &mut Missing,
                pc
            )
            .unwrap_err()
            .to_string()
            .contains("spatial binding")
        );
        {
            let RestoredRecord::Kingdom(k) = &mut held else {
                unreachable!()
            };
            let r = k.runtime.as_mut().unwrap();
            assert_eq!(r.definition.native_class, 4);
            assert_eq!(r.definition.proof.id, "kingdom");
            assert_eq!(
                r.base.actor.as_ref().unwrap().components.state,
                StateBits { low: 0, high: 0 }
            );
            let mut record = Record {
                owner: id,
                shell: &mut r.base,
                definition: &r.definition,
                registry: &mut attempt.registry,
            };
            spatial.refresh_membership(&catalog, &mut record).unwrap();
        }
        register(
            &catalog,
            id,
            &mut held,
            &mut attempt.registry,
            GridContext {
                zones: &mut zones,
                spatial: &spatial,
                point: &point,
                session: &session,
                world: &world,
                city: None,
                live_world48: None,
            },
            &mut Missing,
            pc,
        )
        .unwrap();
        let RestoredRecord::Kingdom(k) = &mut held else {
            unreachable!()
        };
        let r = k.runtime.as_mut().unwrap();
        assert_eq!(&r.base as *const _ as usize, shell_address);
        // Deliberate guard probe: a reached positive state is required. This
        // synthetic state mutation is not claimed as an original save value.
        r.base.actor.as_mut().unwrap().components.state.low = 1;
        let error = register(
            &catalog,
            id,
            &mut held,
            &mut attempt.registry,
            GridContext {
                zones: &mut zones,
                spatial: &spatial,
                point: &point,
                session: &session,
                world: &world,
                city: None,
                live_world48: None,
            },
            &mut Missing,
            pc,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("4D4A90"), "{error}");
        let RestoredRecord::Kingdom(k) = &mut held else {
            unreachable!()
        };
        let r = k.runtime.as_mut().unwrap();
        r.base.actor.as_mut().unwrap().components.state.low = 0;
        assert!(matches!(
            attempt.registry.get(id).unwrap().value,
            RestoredRecord::Loading
        ));
        outputs.push(serde_json::json!({"precision":format!("{pc:?}"),"id":781,"source_pc":"Double53","definition":"kingdom","states":ids.len(),"saved_state_bits":0,"same_held_allocation":true,"membership_key":spatial.key(id).unwrap(),"zone_tiles":zones.tiles.len(),"world48":world.word_48,"session_e4_fixture":3,"state_zero_registration_completed":true,"positive_state_required":error}));
        let flags_before = r.base.thing.flags;
        r.base.actor.as_mut().unwrap().components.state.low = 1;
        let owner_error = register(
            &catalog,
            id,
            &mut held,
            &mut attempt.registry,
            GridContext {
                zones: &mut zones,
                spatial: &spatial,
                point: &point,
                session: &session,
                world: &world,
                city: None,
                live_world48: None,
            },
            &mut PartialThenRetire,
            pc,
        )
        .unwrap_err()
        .to_string();
        assert!(
            owner_error.contains("fixture partial state write"),
            "{owner_error}"
        );
        assert!(owner_error.contains("ownership changed"), "{owner_error}");
        assert!(attempt.registry.get(id).is_none());
        let RestoredRecord::Kingdom(k) = &held else {
            unreachable!()
        };
        assert_eq!(
            k.runtime.as_ref().unwrap().base.thing.flags,
            flags_before ^ 2
        );
    }
    if let Some(path) = std::env::var_os("AA_ACTOR_REGISTRATION_CAPTURE") {
        let f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(f,&serde_json::json!({"scope":"Actual source-completed held saved Kingdom diagnostic prefix with real terrain/index/initial zones/World48/verified PointShape; SessionE4=3 is an explicit test fixture. Immediate fullInit omitted; no fullworld/battle or actualSession ownership claim.","cases":outputs})).unwrap();
    }
}
