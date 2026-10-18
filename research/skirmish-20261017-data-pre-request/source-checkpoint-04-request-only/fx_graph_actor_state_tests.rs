use super::*;
use serde_json::{Value, json};
use std::{cell::RefCell, sync::Arc};
fn fixture(name: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/skirmish-20261017-actor-state-fx")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
struct EventInput<'a> {
    case: &'a Value,
    events: &'a RefCell<Vec<Value>>,
    positions: usize,
    globals: usize,
}
impl CreationInputs for EventInput<'_> {
    fn muted(&mut self) -> Result<bool> {
        Ok(self.case["muted"].as_bool().unwrap())
    }
    fn creator_kingdom(&mut self) -> Result<Option<aa_actors::ActorId>> {
        self.events.borrow_mut().push(json!("A8"));
        let field = if self.case["kind"] == "Kingdom" {
            "thing_id"
        } else {
            self.events.borrow_mut().extend([json!("B4"), json!("108")]);
            "kingdom_id"
        };
        Ok(Some(aa_actors::ActorId::from_raw(
            self.case[field].as_u64().unwrap() as u32,
        )))
    }
    fn current_position(&mut self) -> Result<[u32; 3]> {
        self.positions += 1;
        Ok(self.case["position"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap())
    }
    fn current_thing(&mut self) -> Result<Option<aa_actors::ActorId>> {
        Ok(Some(aa_actors::ActorId::from_raw(
            self.case["thing_id"].as_u64().unwrap() as u32,
        )))
    }
    fn current_os_thread_id(&mut self) -> Result<u32> {
        self.events.borrow_mut().push(json!("current_os_thread_id"));
        Ok(self.case["thread_id"].as_u64().unwrap() as u32)
    }
    fn application_os_thread_id(&mut self) -> Result<u32> {
        Ok(self.case["app_thread"].as_u64().unwrap() as u32)
    }
    fn view_time(&mut self) -> Result<u32> {
        Ok(self.case["view_time"].as_u64().unwrap() as u32)
    }
    fn world_time(&mut self) -> Result<u32> {
        Ok(self.case["world_time"].as_u64().unwrap() as u32)
    }
}
impl StateCreationInputs for EventInput<'_> {
    fn global_position(&mut self) -> Result<[u32; 3]> {
        self.globals += 1;
        Ok(self.case["global_position"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap())
    }
}
fn graph(nodes: &[Value]) -> Arc<Graph> {
    let mut draft = Draft::new();
    for node in nodes {
        draft.kinds.push(if node["kind"] == "create" {
            FxcKind::CreateVfx
        } else {
            FxcKind::PlaySfx
        });
        //Explicit completed-definition transport seam of this native event
        //constructor fixture. Source compilation is proved separately.
        let mut value = Node::create_vfx(Vec::new());
        value.complete = true;
        draft.nodes.push(value);
        draft.roots.push(NodeId(draft.roots.len()));
    }
    Arc::new(Graph {
        draft,
        prior_roots: Vec::new(),
        owner: "explicit-native-fixture".into(),
        source: aa_rules::resources::Provenance {
            path: "explicit-native-fixture".into(),
            sha256: String::new(),
            span: aa_rules::tgi::Span { start: 0, end: 0 },
        },
        precision: Precision::Double53,
        sources: Vec::new(),
        kind: GraphKind::ActorEnter,
        request_only: false,
    })
}
#[test]
fn native_actor_state_scan_predicates_and_full_event_constructor_match() {
    let doc = fixture("native-actor-state-fx.json");
    for row in doc["rows"].as_array().unwrap() {
        let c = &row["input"];
        let nodes = c["nodes"].as_array().unwrap();
        let graph = graph(nodes);
        let events = RefCell::new(Vec::new());
        let state = c["state"].as_u64().unwrap() as u32;
        let active = c["active"].as_u64().unwrap() != 0;
        let matched = scan(
            graph.roots(),
            state,
            active,
            |id| {
                events.borrow_mut().push(json!(["predicate", id.0]));
                let node = &nodes[id.0];
                active_fields(
                    graph.node_kind(id).unwrap(),
                    || Ok(node["class"].as_u64().unwrap() == 1),
                    || Ok(u8::from(node["duration"].as_bool().unwrap())),
                )
            },
            |id| {
                events.borrow_mut().push(json!(["state", id.0]));
                Ok(nodes[id.0]["state"].as_u64().unwrap() as u32)
            },
        )
        .unwrap();
        let mut input = EventInput {
            case: c,
            events: &events,
            positions: 0,
            globals: 0,
        };
        let command = if matched {
            construct_actor_state(
                MatchedState {
                    graph,
                    state,
                    active_only: active,
                },
                &mut input,
            )
            .unwrap()
        } else {
            None
        };
        assert_eq!(input.positions, usize::from(matched));
        assert_eq!(
            input.globals,
            usize::from(matched && !c["muted"].as_bool().unwrap())
        );
        let command=command.map(|c| {
            let mut words=c.words.clone();words.insert(0xc,c.kingdom().map_or(0,aa_actors::ActorId::raw));
            words.insert(0x10,c.source().map_or(0,aa_actors::ActorId::raw));words.insert(0x20,c.target().map_or(0,aa_actors::ActorId::raw));words.remove(&0x68);
            json!({"words":words,"byte58":c.byte(0x58),"flag":0,"fxl_definition":c.fxl_definition(),"fxl_parent_is_self":c.fxl_parent_is_self(),"word68":c.word(0x68)})
        });
        assert_eq!(
            json!({"matched":matched,"command":command,"events":events.into_inner()}),
            row["output"],
            "{row}"
        );
    }
}
#[test]
fn source_permanent_equivalence_and_lazy_native_reads() {
    let doc = fixture("native-vfx-active-source.json");
    for row in doc["rows"].as_array().unwrap() {
        let c = &row["input"];
        assert_eq!(
            row["output"]["class_word"].as_u64() == Some(1),
            c["permanent"].as_u64() != Some(0)
        );
    }
    assert!(
        !active_fields(
            FxcKind::PlaySfx,
            || panic!("native false leaf has no VFX reads"),
            || panic!("no duration")
        )
        .unwrap()
    );
    assert!(
        !active_fields(
            FxcKind::CreateVfx,
            || Ok(false),
            || panic!("non-permanent skips duration flag")
        )
        .unwrap()
    );
    let roots = [NodeId(0), NodeId(1)];
    let events = RefCell::new(Vec::new());
    let e = scan(
        &roots,
        1,
        true,
        |id| {
            events.borrow_mut().push(id.0);
            Err("required predicate source".into())
        },
        |_| panic!("native state read must not follow failed predicate"),
    )
    .unwrap_err();
    assert!(e.contains("required"));
    assert_eq!(events.into_inner(), vec![0]);
    assert!(
        scan(
            &roots,
            1,
            false,
            |_| panic!("inactive-only skips vf20"),
            |_| Ok(1)
        )
        .unwrap()
    );
}

fn snapshot(draft: &Draft) -> Value {
    json!({"nodes":draft.nodes.iter().map(|n|json!({"words":n.words,"bytes":n.bytes,
        "references":n.references.iter().map(|(k,v)|(k.to_string(),v.as_ref().map(|r|r.id.clone()))).collect::<std::collections::BTreeMap<_,_>>(),
        "immediate":n.immediate,"chain":n.chain,"complete":n.complete})).collect::<Vec<_>>(),"roots":draft.roots,"warnings":draft.warnings})
}
fn capture_source<'a>(
    source: &ResolvedNode<'a>,
    input: &mut Bound<'a>,
    nodes: &mut Vec<Value>,
    lookups: &mut Value,
) -> Result<usize> {
    let index = nodes.len();
    nodes.push(json!({}));
    let mut value = serde_json::Map::new();
    for name in [
        "type",
        "target_flags",
        "terrain_group_filter",
        "health_trigger_type",
        "actor_state",
    ] {
        value.insert(name.into(), json!(input.text(source, name)?));
    }
    value.insert(
        "frequency".into(),
        json!(input.integer(source, "frequency")?),
    );
    for name in ["delay", "health"] {
        value.insert(name.into(), json!(input.scalar(source, name)?));
    }
    value.insert(
        "stop_on_destructed".into(),
        json!(input.flag(source, "stop_on_destructed")?),
    );
    let kind = value["type"].as_str().unwrap();
    let fields = if kind == "CREATE_VFX" {
        for name in ["vfx_ids", "at", "location"] {
            value.insert(name.into(), json!(input.text(source, name)?));
        }
        value.insert("duration".into(), json!(input.scalar(source, "duration")?));
        value.insert("attached".into(), json!(input.flag(source, "attached")?));
        for name in ["translation", "orientation"] {
            value.insert(name.into(), json!(input.point(source, name)?));
        }
        vec![("VFX", "vfx_ids"), ("Bone", "location")]
    } else if kind == "PLAY_SFX" {
        for name in ["audio_ids", "at"] {
            value.insert(name.into(), json!(input.text(source, name)?));
        }
        value.insert("looping".into(), json!(input.flag(source, "looping")?));
        vec![("Audio", "audio_ids")]
    } else {
        return Err("capture only supports the actual completed FXC constructors".into());
    };
    for (manager, name) in fields.into_iter().chain([
        ("TerrainGroup", "terrain_group_filter"),
        ("ActorState", "actor_state"),
    ]) {
        let id = value[name].as_str().unwrap();
        if !id.is_empty() {
            lookups[manager][id] = json!(input.find(manager, id)?);
        }
    }
    for name in ["ChainImmediate", "Chain"] {
        let mut children = Vec::new();
        for child in input.children(source, name)? {
            children.push(capture_source(&child, input, nodes, lookups)?);
        }
        value.insert(name.into(), json!(children));
    }
    nodes[index] = json!(value);
    Ok(index)
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 data; source graph/match only, not battle"]
fn actual_farm_enter_graph_state_indices_and_active_filter_use_canonical_source() {
    use crate::{
        combat_source_completion as completion, combat_source_receipt::VerifiedStartupReceipt,
        settings::Settings,
    };
    use aa_mapworld::restore::RestoreDefinitions;
    use aa_rules::{
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
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(settings);
    let receipt =
        VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut sources).unwrap();
    completion::install(
        &catalog,
        &mut sources,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    completion::compute(&catalog).unwrap();
    let archive = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let inventory = aa_mapworld::WorldInventory::decode(&archive, u64::MAX).unwrap();
    let placement = inventory
        .dynamic_things
        .placements
        .iter()
        .find(|p| p.serialized_id == 2696)
        .unwrap();
    let definition = catalog
        .resolve(&placement.thing_id_utf16, Precision::Double53)
        .unwrap();
    let graph = Arc::new(Graph::compile_actor_enter(&catalog, &definition).unwrap());
    assert_eq!(graph.kind(), GraphKind::ActorEnter);
    assert_eq!(graph.list_kind(), None);
    assert!(!graph.roots().is_empty());
    let mut matches = Vec::new();
    for state in 0..42 {
        for active in [false, true] {
            matches.push(json!({"state":state,"active":active,"matched":graph.match_actor_enter(&catalog,state,active).unwrap().is_some()}));
        }
    }
    let source = catalog.actors.resolved_actor(&definition.proof.id).unwrap();
    let roots = source.children("WhenEnter").unwrap();
    let mut input = Bound::new(
        &actors,
        Precision::Double53,
        definition.proof.source.clone(),
    )
    .unwrap();
    let mut nodes = Vec::new();
    let mut ids = Vec::new();
    let mut lookups = json!({"VFX":{},"Audio":{},"TerrainGroup":{},"ActorState":{},"Bone":{}});
    for node in &roots {
        ids.push(capture_source(node, &mut input, &mut nodes, &mut lookups).unwrap());
    }
    let mut active_sources = serde_json::Map::new();
    for node in graph.nodes() {
        if let Some(Some(reference)) = node.reference(0x3c) {
            let source = input.canonical_node(&reference.source[0]).unwrap();
            let permanent = input.flag(&source, "permanent").unwrap();
            active_sources.insert(reference.id.clone(),json!({"permanent":permanent,"duration":input.scalar(&source,"duration").unwrap(),"source":reference.source}));
        }
    }
    if let Some(path) = std::env::var_os("AA_CAPTURE_ACTOR_STATE_FX") {
        let output = json!({"scope":"actual tutorial02 saved2696 ActorData WhenEnter compiled prefix and all42 state matches; no simulation readiness", "definition":definition.proof.id,
            "source":definition.proof.source,"source_precision":"Double53","source_identities":actors.sources,
            "input":{"name":"actual_farm_enter","nodes":nodes,"roots":ids,"lookups":lookups},"compiled":snapshot(&graph.draft),"active_sources":active_sources,"matches":matches});
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file, &output).unwrap();
    }
}
