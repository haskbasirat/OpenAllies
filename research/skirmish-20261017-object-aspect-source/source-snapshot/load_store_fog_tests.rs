use super::*;
use serde_json::{Value, json};

struct Probe {
    input: Value,
    width: i32,
    tiles: [u32; 2],
    flags: [u32; 3],
    words: [u32; 3],
    next: [Option<usize>; 3],
    events: Vec<Value>,
    outcome: String,
}
impl Probe {
    fn new(input: &Value) -> Self {
        let flags = input["flags"].as_u64().unwrap() as u32;
        Self {
            input: input.clone(),
            width: 2,
            tiles: [0x12345678, 0xabcdef01],
            flags: [flags, 0, flags],
            words: [0x10203040, 0x10203041, 0x10203042],
            next: [Some(1), Some(2), None],
            events: Vec::new(),
            outcome: "complete".into(),
        }
    }
    fn index(actor: ActorId) -> usize {
        actor.raw() as usize - 1
    }
    fn memory(&mut self, actor: ActorId) -> Result<()> {
        let i = Self::index(actor);
        let mode = if self.words[i] & 0xffff0000 == 0xffff0000 {
            "visible"
        } else {
            "explored"
        };
        self.events
            .push(json!({"actor":i,"mode":mode,"flags":self.flags[i],"word":self.words[i]}));
        if self.input["stop"] == mode {
            self.outcome = format!("4EBC70:{mode}");
            return Err(Error::Unsupported("test required native4EBC70 entry"));
        }
        if self.input["mutate"] == true && self.events.len() == 1 {
            self.flags[i] ^= 0x10;
            self.next[0] = Some(2);
            self.width = 1;
        }
        Ok(())
    }
    fn result(&self) -> Value {
        json!({"outcome":self.outcome,"tiles":self.tiles,
            "actors":(0..3).map(|i|json!({"flags":self.flags[i],"word":self.words[i]})).collect::<Vec<_>>(),
            "events":self.events,"width":self.width,"next_first":self.next[0].unwrap()})
    }
}
impl World for Probe {
    type Node = usize;
    fn tile_count(&mut self) -> Result<i32> {
        Ok(self.width)
    }
    fn tile_explored(&mut self, tile: i32) -> Result<()> {
        self.tiles[tile as usize] = (self.tiles[tile as usize] & 0xffff0000) | 0xffff;
        Ok(())
    }
    fn tile_visible(&mut self, tile: i32) -> Result<()> {
        self.tiles[tile as usize] = (self.tiles[tile as usize] & 0xffff) | 0xffff0000;
        Ok(())
    }
    fn first(&mut self, tile: i32) -> Result<Option<usize>> {
        Ok((tile == 0).then_some(0))
    }
    fn next(&mut self, node: usize) -> Result<Option<usize>> {
        Ok(self.next[node])
    }
    fn actor(&mut self, node: usize) -> Result<ActorId> {
        Ok(ActorId::from_raw(node as u32 + 1))
    }
    fn flags(&mut self, actor: ActorId) -> Result<u32> {
        Ok(self.flags[Self::index(actor)])
    }
    fn set_flags(&mut self, actor: ActorId, flags: u32) -> Result<()> {
        self.flags[Self::index(actor)] = flags;
        Ok(())
    }
    fn is_object(&mut self, _: ActorId) -> Result<bool> {
        Ok(true)
    }
    fn associated_current(&mut self, _: ActorId) -> Result<bool> {
        Ok(self.input["associated"].as_bool().unwrap())
    }
    fn explore(&mut self, actor: ActorId) -> Result<()> {
        let i = Self::index(actor);
        self.words[i] = (self.words[i] & 0xffff0000) | 0xffff;
        self.memory(actor)
    }
    fn show(&mut self, actor: ActorId) -> Result<()> {
        let i = Self::index(actor);
        self.words[i] = (self.words[i] & 0xffff) | 0xffff0000;
        self.memory(actor)
    }
}

#[test]
fn all_native_global_fog_rows_preserve_live_nodes_flags_and_failure_prefixes() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-load-store-fog/native-store-fog.json");
    let fixture: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1728);
    for row in rows {
        let mut world = Probe::new(&row["input"]);
        let result = all(&mut world, row["input"]["visible"].as_bool().unwrap());
        assert_eq!(
            result.is_ok(),
            row["native"]["outcome"] == "complete",
            "{row}"
        );
        assert_eq!(world.result(), row["native"], "{row}");
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT and genuine immediate-load source owners"]
fn actual_stopped_load_moves_same_actor_fields_through_memory_refresh() {
    use crate::{
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
    let actor_source = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = std::sync::Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let session = aa_netcode::lobby::Session::default();
    struct SessionRef<'a>(&'a aa_netcode::lobby::Session);
    impl crate::load_world_globals::SessionPhase for SessionRef<'_> {
        fn current_phase(&self) -> std::result::Result<u32, String> {
            Ok(self.0.field_f8)
        }
        fn application_state(&self) -> std::result::Result<u32, String> {
            Ok(self.0.state)
        }
    }
    let session = SessionRef(&session);
    let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let mut attempt = Attempt::load_immediate(
        &map,
        &actor_source,
        settings.clone(),
        text,
        aa_numeric::Precision::Double53,
        u64::MAX,
        aa_mapworld::restore::RestoreMode::StopOnError,
        LoadBindings {
            native_load_mode: Some(1),
            inactive_load: Some(inactive),
            resources: Some(resources),
        },
        &mut sources,
        &session,
        false,
    )
    .unwrap();
    assert!(
        attempt
            .report
            .lifecycle
            .iter()
            .any(|e| e.saved_id == 2696 && e.attempted)
    );
    let mut world = attempt.load_world.take().unwrap();
    //This is the real stopped registry prefix, moved directly into the real
    //ActorStore conversion. Scheduler/PostLoad/mission readiness is not claimed.
    let mut actors =
        ActorStore::from_restored_with_abilities(attempt.registry, attempt.abilities).unwrap();
    actors.property_allocations = world.property_allocations.take().unwrap();
    let mut catalog = Catalog::new(&actor_source);
    let bank = attempt.definitions;
    assert!(catalog.definition_bank.borrow().combat_source().is_err());
    catalog.settings = Some(settings);
    catalog.load_clock = Some(&attempt.clock);
    catalog.definition_text = Some(attempt.definition_text);
    catalog
        .bind_inactive_load(attempt.inactive_load.as_ref().unwrap())
        .unwrap();
    let keys: Vec<_> = actors
        .identities()
        .iter()
        .map(|e| (e.id, e.value))
        .collect();
    let addresses = |actors: &ActorStore| {
        keys.iter()
            .map(|(_, key)| {
                let object = actors.object(*key).unwrap();
                (
                    std::ptr::from_ref(object) as usize,
                    object
                        .object
                        .as_ref()
                        .map(|o| std::ptr::from_ref(&o.memory) as usize),
                )
            })
            .collect::<Vec<_>>()
    };
    let before_addresses = addresses(&actors);
    let before_flags: Vec<_> = keys
        .iter()
        .map(|(_, k)| actors.staged_placement(*k).unwrap().flags)
        .collect();
    let crate::load_world::SpatialOwner::Loading(Some(spatial)) = &world.spatial else {
        panic!("actual same pre-adoption index missing")
    };
    let services =
        load_zone_services::Context::with_kingdoms(&world.point, &session, &world.kingdoms)
            .with_source_fields(&catalog, &world.source_fields);
    ACTUAL_MEMORY_STOP.with(|c| *c.borrow_mut() = None);
    crate::load_store_memory_refresh::ACTUAL_INPUT.with(|v| *v.borrow_mut() = None);
    let result = reveal_all(
        &catalog,
        &bank,
        &mut actors,
        &mut world.zones,
        &world.zone_globals,
        spatial.index(),
        &services,
        &world.kingdoms,
        false,
    );
    eprintln!("global Fog next result: {result:?}");
    let stop = ACTUAL_MEMORY_STOP.with(|c| c.borrow_mut().take());
    let first = actors.key(aa_actors::ActorId::from_raw(28)).unwrap();
    let object = actors.object(first).unwrap();
    let memory = &object.object.as_ref().unwrap().memory;
    let records: Vec<_> = (0..memory.count())
        .filter_map(|i| memory.slot(i as u32).unwrap())
        .collect();
    assert!(
        !records.is_empty(),
        "first actual class9 memory must be published before the next boundary: {result:?}"
    );
    for id in &records {
        let record = memory.record(*id).unwrap();
        assert_eq!(record.timestamp_bits, attempt.clock.begin.0);
        assert_eq!(record.definition.as_deref(), Some("temperate_pine_trees"));
        assert_eq!(record.owner, aa_actors::ActorId::NULL);
        assert_eq!(record.stat_words, [Some(0); 5]);
        assert_eq!(
            record.position_bits,
            actors.staged_placement(first).unwrap().position_bits[..2]
        );
    }
    assert_eq!(
        addresses(&actors),
        before_addresses,
        "same RuntimeObject/memory allocations moved, never copied"
    );
    for ((_id, key), old) in keys.iter().zip(before_flags) {
        let flags = actors.staged_placement(*key).unwrap().flags;
        assert!(flags == old || flags == old & !(1 << 30));
    }
    let explored_prefix = world
        .zones
        .tiles
        .iter()
        .take_while(|t| t.fog & 0xffff == 0xffff)
        .count();
    assert!(explored_prefix > 0 && explored_prefix < world.zones.tiles.len());
    let snapshot = json!({"scope":"Same genuine stopped tutorial02 prefix moved directly into ActorStore, global explore only; no PostLoad/whole mission or battle claim",
        "actors":keys.len(),"source_precision":bank.combat_source().unwrap().source_precision(),
        "live_precision":aa_numeric::Precision::Double53,"result":format!("{result:?}"),"same_allocations":true,"moved_source_bank":true,"catalog_bank_empty":catalog.definition_bank.borrow().combat_source().is_err(),"required_prefix":stop,"actual_input":crate::load_store_memory_refresh::ACTUAL_INPUT.with(|v|v.borrow_mut().take()),"first_memory":records.iter().map(|id|{let r=memory.record(*id).unwrap();json!({"id":id.raw(),"name":r.name_utf16,"position":r.position_bits,"definition":r.definition,"owner":r.owner.raw(),"aspect":r.aspect,"stats":r.stat_words,"flags":r.flags})}).collect::<Vec<_>>(),"explored_prefix":explored_prefix,
        "tile_count":world.zones.tiles.len(),"tiles_sha256":aa_rules::sources::sha256(&world.zones.tiles.iter().flat_map(|t|t.fog.to_le_bytes()).collect::<Vec<_>>()),
        "actor_words":keys.iter().filter_map(|(id,k)|actors.object(*k).unwrap().object.as_ref().map(|o|json!({"owner":id.raw(),"word54":o.words_50_to_54[1],"memory_count":o.memory.count(),"records":o.memory.allocated_records()}))).collect::<Vec<_>>()});
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../research/skirmish-20261017-object-aspect-source/actual-staged-refresh-aspect.json",
    );
    if let Some(output) = std::env::var_os("AA_CAPTURE_STORE_FOG") {
        serde_json::to_writer_pretty(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(output)
                .unwrap(),
            &snapshot,
        )
        .unwrap();
    } else {
        assert_eq!(
            snapshot,
            serde_json::from_slice::<Value>(&std::fs::read(path).unwrap()).unwrap()
        );
    }
}
