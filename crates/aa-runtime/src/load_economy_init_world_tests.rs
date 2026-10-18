use super::*;
use aa_world::queues::{Clock, Kind, QueueSnapshot, Queues};
use serde_json::Value;

fn native_fixture() -> Value {
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-load-economy-init/native-economy-init.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

struct ActualQueue {
    queues: Queues,
    rng: NativeRng,
    supply: bool,
    reads: u32,
}
impl World for ActualQueue {
    fn scheduled(&mut self) -> Result<bool> {
        Ok(self.queues.owner_head(1).is_some())
    }
    fn supply_required(&mut self) -> Result<bool> {
        self.reads += 1;
        Ok(self.supply)
    }
    fn add_periodic(&mut self) -> Result<()> {
        self.queues
            .add_periodic(1, 1, 0, &mut self.rng)
            .map_err(error)?;
        Ok(())
    }
}

#[test]
fn original_init_existing_matches_actual_queue_rng_and_lazy_source_reads() {
    let fixture = native_fixture();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 48);
    for row in rows {
        let input = &row["input"];
        let pc = match input["precision"].as_u64().unwrap() {
            24 => Precision::Single24,
            53 => Precision::Double53,
            64 => Precision::Extended64,
            _ => unreachable!(),
        };
        let mut world = ActualQueue {
            queues: Queues::new(
                serde_json::from_value::<Clock>(input["clock"].clone()).unwrap(),
                pc,
            )
            .unwrap(),
            rng: NativeRng::new(input["seed"].as_u64().unwrap() as u32),
            supply: input["supply_required"].as_bool().unwrap(),
            reads: 0,
        };
        for (id, count) in input["counts"].as_array().unwrap().iter().enumerate() {
            world
                .queues
                .initialize(
                    id as u8,
                    if id < 3 {
                        Kind::Periodic
                    } else {
                        Kind::Deferred
                    },
                    count.as_i64().unwrap() as i32,
                )
                .unwrap();
        }
        let pending = input["scheduled"].as_bool().unwrap();
        if pending {
            world.queues.add_periodic(1, 1, 99, &mut world.rng).unwrap();
        }
        let before = &row["output"]["before"];
        assert_eq!(world.rng.state, before["rng"].as_u64().unwrap() as u32);
        assert_eq!(
            world.queues.snapshot(),
            serde_json::from_value::<Vec<QueueSnapshot>>(before["queues"].clone()).unwrap()
        );
        assert!(initialize(&mut world).unwrap());
        let after = &row["output"]["after"];
        assert_eq!(world.rng.state, after["rng"].as_u64().unwrap() as u32);
        assert_eq!(
            world.queues.snapshot(),
            serde_json::from_value::<Vec<QueueSnapshot>>(after["queues"].clone()).unwrap()
        );
        assert_eq!(
            world.queues.owner_head(1).is_some(),
            after["pending"].as_bool().unwrap()
        );
        assert_eq!(world.reads, u32::from(!pending));
    }
}

#[test]
#[ignore = "requires original/v14 source assets in AA_GAME_ROOT"]
fn actual_restored_economy_init_same_component_rng_and_queues() {
    use crate::{
        combat_source_completion as completion,
        combat_source_receipt::VerifiedStartupReceipt,
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        restore_lifecycle::{ScopedManagedLoad, TaskKind},
        settings::Settings,
    };
    use aa_actors::ActorId;
    use aa_mapworld::restore::RestoredRecord;
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use serde_json::json;
    use std::{cell::RefCell, sync::Arc};
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
    let reference = native_fixture();
    let mut captured = Vec::new();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
            .unwrap()
            .begin(TaskKind::Normal)
            .unwrap();
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
                let receipt =
                    VerifiedStartupReceipt::verify_embedded_reference(catalog, &mut sources)?;
                completion::install(
                    catalog,
                    &mut sources,
                    receipt.context(),
                    receipt.csv(),
                    receipt.globals(),
                )?;
                completion::compute(catalog)
            },
        )
        .unwrap();
        //Explicit diagnostic arithmetic PC; the actual source owner remains53.
        //No serialized/gameplay field, event, flow vector or RNG is replaced.
        attempt.queues.precision = pc;
        let id = ActorId::from_raw(784);
        let mut held = std::mem::replace(
            attempt.registry.value_mut(id).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Actor(actor) = &mut held else {
            panic!("actual784 Actor")
        };
        let mut catalog = Catalog::new(&actors);
        catalog.settings = Some(attempt.settings.clone());
        catalog.native_load_mode = attempt.native_load_mode;
        catalog.definition_bank = RefCell::new(std::mem::take(&mut attempt.definitions));
        catalog.load_owners = RefCell::new(std::mem::take(&mut attempt.scheduler_owners));
        catalog.load_queues = RefCell::new(Some(&mut attempt.queues));
        let target = UpdatableTarget::Component {
            owner: id.raw(),
            slot: ComponentSlot::Economy.index() as u8,
        };
        let owner = catalog.load_owners.borrow_mut().restoring(target).unwrap();
        let source = catalog
            .actors
            .native_economy_inputs(&actor.definition.proof.id, Precision::Double53)
            .unwrap();
        let required = source
            .resources
            .as_ref()
            .unwrap()
            .supply_required
            .boolean()
            .unwrap();
        let record = Record {
            owner: id,
            shell: &mut actor.shell,
            definition: &actor.definition,
            registry: &mut attempt.registry,
        };
        let mut record = record;
        let state = |r: &Record<'_>| match r
            .shell
            .actor
            .as_ref()
            .unwrap()
            .components
            .get(ComponentSlot::Economy)
            .unwrap()
        {
            RestoredComponent::Economy(v) => json!(v),
            _ => unreachable!(),
        };
        let component = state(&record);
        let pointer = match record
            .shell
            .actor
            .as_ref()
            .unwrap()
            .components
            .get(ComponentSlot::Economy)
            .unwrap()
        {
            RestoredComponent::Economy(v) => &**v as *const _ as usize,
            _ => unreachable!(),
        };
        let before_queues = catalog.load_queues.borrow().as_deref().unwrap().snapshot();
        assert!(
            before_queues
                .iter()
                .all(|q| q.buckets.iter().all(Vec::is_empty)),
            "actual decoded pre-Init queues are constructor-empty"
        );
        let seed = attempt.rng.state;
        let clock = catalog.load_queues.borrow().as_deref().unwrap().clock;
        assert!(init_existing(&catalog, &mut record, &mut attempt.rng, pc).unwrap());
        after_init(&catalog, &mut record).unwrap();
        assert_eq!(
            state(&record),
            component,
            "same Economy payload: live links belong only to Queues"
        );
        let after_pointer = match record
            .shell
            .actor
            .as_ref()
            .unwrap()
            .components
            .get(ComponentSlot::Economy)
            .unwrap()
        {
            RestoredComponent::Economy(v) => &**v as *const _ as usize,
            _ => unreachable!(),
        };
        assert_eq!(pointer, after_pointer);
        let queues = catalog.load_queues.borrow();
        let queues = queues.as_deref().unwrap();
        let mut snapshot = queues.snapshot();
        for queue in &mut snapshot {
            for bucket in &mut queue.buckets {
                for entry in bucket {
                    assert_eq!(entry.0, owner);
                    entry.0 = 1;
                }
            }
        }
        let output = json!({"before":{"rng":seed,"queues":before_queues},"after":{"rng":attempt.rng.state,"queues":snapshot,"pending":queues.owner_head(owner).is_some()},"result":true});
        let input = json!({"precision":pc.significand_bits(),"scheduled":false,"supply_required":required,"seed":seed,"counts":before_queues.iter().map(|q|q.buckets.len()).collect::<Vec<_>>(),"clock":clock});
        if let Some(rows) = reference.get("actual_rows").and_then(Value::as_array) {
            if let Some(native) = rows.iter().find(|r| r["input"] == input) {
                assert_eq!(native["output"], output);
            } else {
                assert!(std::env::var_os("AA_ECONOMY_INIT_CAPTURE").is_some(),
                    "fresh actual input requires native capture/replay: {input}");
            }
        } else {
            assert!(
                std::env::var_os("AA_ECONOMY_INIT_CAPTURE").is_some(),
                "actual native fixture required"
            );
        }
        captured.push(json!({"actor":id.raw(),"definition":record.definition.proof,"source_precision":"Double53","source_value348":source.resources.as_ref().unwrap().supply_required,"component":component,"input":input,"output":output,"same_allocation":true,"restore_report":attempt.report,"scope":"actual decoded held component diagnostic; full immediate lifecycle is separately tested"}));
        println!(
            "{pc:?}: actual784 Economy Init/AfterInit,source348={required},same component/RNG/queues"
        );
    }
    if let Some(path) = std::env::var_os("AA_ECONOMY_INIT_CAPTURE") {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file, &json!({"cases":captured})).unwrap();
    }
}
