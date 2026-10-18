use super::*;
use aa_actors::creation::{ActorCreationRecord, CreationRecord, ThingDefinition};
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use serde_json::{Value, json};

#[test]
#[ignore = "requires AA_GAME_ROOT; direct actual Character constructor/service fixture, not a completed staged mission"]
fn actual_source_character_keeps_one_allocation_queue_and_rng() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let settings = crate::settings::Settings::load_v14(&mut files, Limits::default()).unwrap();
    let text = crate::localization::Localization::load_v14(&mut files, Limits::default()).unwrap();
    let source = ActorCatalog::load_v14(&mut files).unwrap();
    let a = std::cell::RefCell::new(aa_orders::net::sync::Synchronizer::new(64));
    let s = std::cell::RefCell::new(aa_orders::net::sync::Synchronizer::new(64));
    let mut gate_a = || Err("disabled fixture SyncA gate must not run".into());
    let mut gate_s = || Err("disabled fixture SyncS gate must not run".into());
    let mut catalog = Catalog::new(&source);
    catalog.settings = Some(settings);
    catalog.definition_text = Some(text.clone());
    catalog.replace_current_text(text);
    catalog
        .bind_configuration_sync(crate::live_clocked::SyncBinding::Shared {
            owner: &a,
            gate: &mut gate_a,
        })
        .unwrap();
    catalog
        .bind_configuration_sync_s(crate::live_clocked::SyncBinding::Shared {
            owner: &s,
            gate: &mut gate_s,
        })
        .unwrap();
    let bank = crate::definitions::Bank::default();
    let mut captures = vec![];
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        for name in ["usa_infantry", "gbr_infantry", "ger_infantry"] {
            let binding = catalog.actor_bindings(name, Precision::Double53).unwrap();
            let mut store = actor_world::ActorStore::new();
            let mut random = aa_simulation::NativeRng::new(0x12345678);
            let record = ActorCreationRecord {
                base: CreationRecord {
                    definition: ThingDefinition {
                        key: name.into(),
                        flags: binding.definition.flags,
                    },
                    position_bits: [0; 3],
                    direction_bits: [0, 0xbf800000],
                    name_utf16: vec![],
                    saved_id: ActorId::NULL,
                },
                variant: -1,
                owner_word: 0,
                word_30: 0,
                request_nonblocking: false,
            };
            let key = store
                .construct_new_bound(
                    &catalog,
                    Arc::clone(&binding),
                    record,
                    0,
                    &mut random,
                    Precision::Single24,
                    pc,
                )
                .unwrap();
            let id = store.initialize_object(key, &mut random, pc).unwrap();
            store
                .construct_ground(key, Slot::Character, F32::ZERO, pc)
                .unwrap();
            let mut owners = crate::scheduler_owners::Owners::new();
            //The exact constructor completed above. This is its explicit
            //Updatable registration transport, as used by AssemblyFactory.
            let owner = owners
                .register(crate::scheduler_owners::Target::Component {
                    object: key,
                    slot: Slot::Character as u8,
                })
                .unwrap();
            let clock = aa_simulation::NativeClock::stock();
            let mut queues = aa_world::queues::Queues::new(
                aa_world::queues::Clock {
                    begin_bits: clock.begin.0,
                    end_bits: clock.end.0,
                    step_bits: clock.step.0,
                },
                pc,
            )
            .unwrap();
            queues
                .initialize(3, aa_world::queues::Kind::Deferred, 160)
                .unwrap();
            //Explicit unused index fixture: this actor still owns staged pose,
            //so activation reads that same pose and makes no index query.
            let mut spatial = crate::spatial_world::SpatialWorld::new(
                aa_spatial::collision::Grid {
                    dimensions: [1, 1],
                    coordinate_shift: 0,
                    cell_size_bits: 1f32.to_bits(),
                },
                vec![0],
                pc,
            )
            .unwrap();
            let (_, mut actors) = store.split_minds();
            let mut w = Bound {
                actors: &mut actors,
                spatial: &mut spatial,
                queues: &mut queues,
                owners: &owners,
                random: &mut random,
                clock: &clock,
                bank: &bank,
                catalog: &catalog,
                binding,
                key,
                id,
            };
            let address = borrowed::World::character(&w).unwrap() as *const _ as usize;
            let initial = borrowed::World::character(&w).unwrap();
            let initial_key = initial.animation.clone().unwrap();
            let first = borrowed::StanceWorld::first_idle(&w).unwrap().unwrap();
            let seed = w.random.state;
            let flags = w.actors.placement(key, w.spatial).unwrap().flags;
            let read = |field| {
                source
                    .native_field(name, field, Precision::Double53)
                    .unwrap()
                    .unwrap()
                    .integer()
                    .unwrap()
            };
            let native = json!({"active":false,"head":0,"word28":initial.word_28,"same_animation":initial_key==first,"flags":flags,"seed":seed,"now":clock.begin.0,"elapsed":initial.elapsed.0,"length":w.actors.animations.get(&first).unwrap().source.timing.length.0,"min":read("View.idle_loops_min"),"max":read("View.idle_loops_max")});
            borrowed::default_stance(&mut w, pc).unwrap();
            assert_eq!(
                borrowed::World::character(&w).unwrap() as *const _ as usize,
                address
            );
            let c = borrowed::World::character(&w).unwrap();
            let state = json!({"animation":if c.animation.as_deref()==Some(first.as_str()){1}else{2},"elapsed":c.elapsed.0,"rate":c.rate.0,"word28":c.word_28,"head":w.queues.owner_head(owner).is_some(),"active":active(w.actors,key).unwrap()==Some(Slot::Character),"rng":w.random.state});
            assert!(state["head"].as_bool().unwrap());
            let seed_after = w.random.state;
            borrowed::default_stance(&mut w, pc).unwrap();
            assert_eq!(w.random.state, seed_after);
            assert_eq!(
                borrowed::World::character(&w).unwrap() as *const _ as usize,
                address
            );
            captures.push(json!({"actor":name,"precision":pc,"input":native,"state":state}));
        }
    }
    if let Some(path) = std::env::var_os("AA_STANCE_CAPTURE") {
        std::fs::write(path, serde_json::to_vec_pretty(&captures).unwrap()).unwrap();
        return;
    }
    let f: Value = serde_json::from_slice(
        &std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../research/skirmish-20261017-character-stance/native-actual-stance.json"
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(f["rows"].as_array().unwrap().len(), captures.len());
    for (c, row) in captures.iter().zip(f["rows"].as_array().unwrap()) {
        assert_eq!(c["state"], row["output"]["state"]);
    }
}
