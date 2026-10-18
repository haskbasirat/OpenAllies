use super::*;
use serde_json::{Value, json};
struct Fixture {
    case: Value,
    events: Vec<Value>,
}
impl Fixture {
    fn get(&mut self, node: usize, name: &str) -> Result<Value> {
        self.events.push(json!(["get", node, name]));
        if self.case["fail"] == name {
            return Err(format!("required {name}"));
        }
        Ok(self.case["nodes"][node][name].clone())
    }
}
impl Inputs for Fixture {
    type Source = usize;
    fn request_only(&self) -> bool {
        self.case["request_only"].as_bool().unwrap_or(false)
    }
    fn provenance(&mut self, _: &usize) -> Result<Vec<Provenance>> {
        Ok(Vec::new())
    }
    fn children(&mut self, node: &usize, name: &str) -> Result<Vec<usize>> {
        self.events.push(json!(["children", node, name]));
        Ok(self.case["nodes"][*node][name]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect())
    }
    fn text(&mut self, node: &usize, name: &str) -> Result<String> {
        Ok(self.get(*node, name)?.as_str().unwrap().into())
    }
    fn integer(&mut self, node: &usize, name: &str) -> Result<i32> {
        Ok(self.get(*node, name)?.as_i64().unwrap() as i32)
    }
    fn scalar(&mut self, node: &usize, name: &str) -> Result<u32> {
        Ok(self.get(*node, name)?.as_u64().unwrap() as u32)
    }
    fn flag(&mut self, node: &usize, name: &str) -> Result<bool> {
        Ok(self.get(*node, name)?.as_bool().unwrap())
    }
    fn point(&mut self, node: &usize, name: &str) -> Result<[u32; 3]> {
        Ok(self
            .get(*node, name)?
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap())
    }
    fn target_index(&mut self, ids: &str) -> Result<u32> {
        self.events.push(json!(["target", ids]));
        match ids {
            "friend" => Ok(0),
            "enemy" => Ok(1),
            "body" => Ok(12),
            _ => Err(format!("target {ids}")),
        }
    }
    fn find(&mut self, manager: &str, ids: &str) -> Result<Option<Reference>> {
        self.events.push(json!(["find", manager, ids]));
        if let Some(lookup) = self.case.get("lookups") {
            let v = &lookup[manager][ids];
            if v.is_null() {
                return Ok(None);
            }
            return Ok(Some(Reference {
                id: v["id"].as_str().unwrap().into(),
                source: Vec::new(),
                ordinal: v["ordinal"].as_u64().map(|n| n as u32),
                cutscene: v["cutscene"].as_bool().unwrap(),
            }));
        }
        Ok((ids != "missing").then(|| Reference {
            id: ids.into(),
            source: Vec::new(),
            ordinal: Some(match ids {
                "head" => 3,
                "moving" => 8,
                _ => 0,
            }),
            cutscene: ids == "cutscene",
        }))
    }
    fn enum_value(&mut self, domain: &str, ids: &str) -> Result<i32> {
        self.events.push(json!(["enum", domain, ids]));
        let names: &[&str] = match domain {
            "FXC" => &aa_rules::abilities::FXC_TYPES,
            "AT" => &[
                "SELF",
                "TARGET_THING",
                "SOURCE_GROUND",
                "TARGET_GROUND",
                "SOURCE_ABILITY_POSITION",
                "TARGET_HIT_POSITION",
                "MAP_CENTER",
            ],
            "HealthTrigger" => &[
                "ABSOLUTE_BELOW",
                "ABSOLUTE_ABOVE",
                "RELATIVE_DECREASE",
                "RELATIVE_INCREASE",
            ],
            _ => panic!(),
        };
        Ok(names
            .iter()
            .position(|n| n.eq_ignore_ascii_case(ids))
            .map(|n| n as i32)
            .unwrap_or(-1))
    }
}
fn snapshot(draft: &Draft) -> Value {
    json!({"nodes":draft.nodes.iter().map(|n|json!({"words":n.words,"bytes":n.bytes,
        "references":n.references.iter().map(|(k,v)|(k.to_string(),v.as_ref().map(|r|r.id.clone()))).collect::<BTreeMap<_,_>>(),
        "immediate":n.immediate,"chain":n.chain,"complete":n.complete})).collect::<Vec<_>>(),"roots":draft.roots,"warnings":draft.warnings})
}
#[test]
fn original_v14_create_vfx_graph_full_state_order_and_partial_stores_match() {
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/skirmish-20261016-fx-graph/native-fx-graph.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let case = &row["input"];
        let roots = case["roots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        let mut input = Fixture {
            case: case.clone(),
            events: Vec::new(),
        };
        let result = compile_inputs(&mut input, roots);
        let failed = result.is_err();
        let draft = match result {
            Ok(d) => d,
            Err(f) => f.partial,
        };
        let mut actual = snapshot(&draft);
        actual["failed"] = json!(failed);
        actual["events"] = json!(input.events);
        let mut expected = row["output"].clone();
        expected["failed"] = json!(!expected["error"].is_null());
        expected.as_object_mut().unwrap().remove("error");
        assert_eq!(actual, expected, "{} / {}", case["name"], row["precision"]);
    }
}
struct CreatedInput {
    case: Value,
    events: Vec<Value>,
    positions: usize,
    things: usize,
    clock_reads: Vec<&'static str>,
}
impl CreationInputs for CreatedInput {
    fn muted(&mut self) -> Result<bool> {
        Ok(self.case["muted"].as_bool().unwrap())
    }
    fn creator_kingdom(&mut self) -> Result<Option<aa_actors::ActorId>> {
        let kind = self.case["kind"].as_str().unwrap();
        if kind == "Null" {
            return Ok(None);
        }
        self.events.push(json!("A8"));
        if kind != "Kingdom" {
            self.events.push(json!("B4"));
            if kind != "Actor" {
                return Ok(None);
            }
            self.events.push(json!("108"));
        }
        let value = self.case[if kind == "Kingdom" {
            "thing_id"
        } else {
            "kingdom_id"
        }]
        .as_u64()
        .unwrap() as u32;
        Ok((value != 0).then_some(aa_actors::ActorId::from_raw(value)))
    }
    fn current_position(&mut self) -> Result<[u32; 3]> {
        self.positions += 1;
        let key = if self.case["kind"] == "Null" {
            "global_position"
        } else {
            "position"
        };
        Ok(self.case[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap())
    }
    fn current_thing(&mut self) -> Result<Option<aa_actors::ActorId>> {
        self.things += 1;
        Ok((self.case["kind"] != "Null")
            .then(|| aa_actors::ActorId::from_raw(self.case["thing_id"].as_u64().unwrap() as u32)))
    }
    fn current_os_thread_id(&mut self) -> Result<u32> {
        self.events.push(json!("current_os_thread_id"));
        Ok(self.case["thread_id"].as_u64().unwrap() as u32)
    }
    fn application_os_thread_id(&mut self) -> Result<u32> {
        Ok(self.case["app_thread"].as_u64().unwrap() as u32)
    }
    fn view_time(&mut self) -> Result<u32> {
        self.clock_reads.push("view");
        Ok(self.case["view_time"].as_u64().unwrap() as u32)
    }
    fn world_time(&mut self) -> Result<u32> {
        self.clock_reads.push("world");
        Ok(self.case["world_time"].as_u64().unwrap() as u32)
    }
}
#[test]
fn original_v14_created_event_constructor_embedded_null_and_thread_clock_selection_match() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-fx-graph");
    let fixture: Value =
        serde_json::from_slice(&std::fs::read(root.join("native-fx-command.json")).unwrap())
            .unwrap();
    let graph_fixture: Value =
        serde_json::from_slice(&std::fs::read(root.join("native-fx-graph.json")).unwrap()).unwrap();
    let ordinary = &graph_fixture["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["input"]["name"] == "ordinary")
        .unwrap()["input"];
    for row in fixture["rows"].as_array().unwrap() {
        let case = &row["input"];
        let mut input = Fixture {
            case: ordinary.clone(),
            events: Vec::new(),
        };
        let mut draft = compile_inputs(&mut input, vec![0]).unwrap();
        if !case["nonempty"].as_bool().unwrap() {
            draft.roots.clear();
        }
        //Explicit laboratory source receipt; only the native constructor's
        //transport of an already compiled graph is under test in these rows.
        let source = Provenance {
            path: "fixture".into(),
            sha256: "fixture".into(),
            span: aa_rules::tgi::Span { start: 0, end: 0 },
        };
        let graph = std::sync::Arc::new(Graph {
            draft,
            prior_roots: Vec::new(),
            owner: "fixture".into(),
            source,
            precision: Precision::Double53,
            sources: Vec::new(),
            kind: GraphKind::View(ListKind::Created),
            request_only: false,
        });
        let mut input = CreatedInput {
            case: case.clone(),
            events: Vec::new(),
            positions: 0,
            things: 0,
            clock_reads: Vec::new(),
        };
        let command = construct_created(graph, &mut input).unwrap();
        let command=command.map(|c| {
            let mut words=c.words.clone();words.insert(0xc,c.kingdom().map_or(0,aa_actors::ActorId::raw));
            words.insert(0x10,c.source().map_or(0,aa_actors::ActorId::raw));words.insert(0x20,c.target().map_or(0,aa_actors::ActorId::raw));
            assert_eq!(input.positions,2);assert_eq!(input.things,2);
            assert_eq!(input.clock_reads,if case["thread_id"]==case["app_thread"] {vec!["view"]} else {vec!["world"]});
            words.remove(&0x68);
            json!({"words":words,"byte58":c.byte(0x58),"flag":case["flag"],"fxl_definition":c.fxl_definition(),"fxl_parent_is_self":c.fxl_parent_is_self(),"word68":c.word(0x68)})
        });
        assert_eq!(
            json!({"command":command,"events":input.events}),
            row["output"],
            "{case}"
        );
    }
}
fn capture_node<'a>(
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
        "vfx_ids",
        "at",
        "location",
    ] {
        value.insert(name.into(), json!(input.text(source, name)?));
    }
    for name in ["frequency"] {
        value.insert(name.into(), json!(input.integer(source, name)?));
    }
    for name in ["delay", "health", "duration"] {
        value.insert(name.into(), json!(input.scalar(source, name)?));
    }
    for name in ["stop_on_destructed", "attached"] {
        value.insert(name.into(), json!(input.flag(source, name)?));
    }
    for name in ["translation", "orientation"] {
        value.insert(name.into(), json!(input.point(source, name)?));
    }
    for name in ["ChainImmediate", "Chain"] {
        let mut ids = Vec::new();
        for child in input.children(source, name)? {
            ids.push(capture_node(&child, input, nodes, lookups)?);
        }
        value.insert(name.into(), json!(ids));
    }
    for (manager, field) in [
        ("VFX", "vfx_ids"),
        ("TerrainGroup", "terrain_group_filter"),
        ("ActorState", "actor_state"),
        ("Bone", "location"),
    ] {
        let ids = value[field].as_str().unwrap();
        if !ids.is_empty() {
            lookups[manager][ids] = json!(input.find(manager, ids)?);
        }
    }
    nodes[index] = Value::Object(value);
    Ok(index)
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 sources; source census, not runtime battle"]
fn actual_created_source_compilation_census() {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
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
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut all = Vec::new();
    for tag in aa_rules::actors::native_load::THING_TAGS {
        all.extend(actors.resolved_definitions(tag).unwrap());
    }
    let mut inputs = Vec::new();
    let mut results = Vec::new();
    for actor in &actors.actors {
        let node = all
            .iter()
            .find(|n| n.sources().first() == Some(&actor.source))
            .unwrap();
        let Some(view) = node.child("View").unwrap() else {
            continue;
        };
        let roots = view.children("WhenCreated").unwrap();
        if roots.is_empty() {
            continue;
        }
        let mut bound = Bound::new(&actors, Precision::Double53, actor.source.clone()).unwrap();
        let result = compile_inputs(&mut bound, roots.clone());
        let error = result.as_ref().err().map(ToString::to_string);
        let draft = match result {
            Ok(d) => d,
            Err(f) => f.partial,
        };
        results.push(
            json!({"id":actor.id,"source":actor.source,"error":error,"compiled":snapshot(&draft)}),
        );
        let mut bound = Bound::new(&actors, Precision::Double53, actor.source.clone()).unwrap();
        let mut nodes = Vec::new();
        let mut ids = Vec::new();
        let mut capture_error = None;
        let mut lookups = json!({"VFX":{},"TerrainGroup":{},"ActorState":{},"Bone":{}});
        for root in &roots {
            match capture_node(root, &mut bound, &mut nodes, &mut lookups) {
                Ok(id) => ids.push(id),
                Err(e) => {
                    capture_error = Some(e);
                    break;
                }
            }
        }
        if capture_error.is_none() {
            inputs.push(json!({"name":format!("actual_{}",actor.id),"nodes":nodes,"roots":ids,"lookups":lookups}));
        }
    }
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let inventory = aa_mapworld::WorldInventory::decode(&map, u64::MAX).unwrap();
    let mut map_created = Vec::new();
    for placement in &inventory.dynamic_things.placements {
        let id = String::from_utf16(&placement.thing_id_utf16).unwrap();
        if let Some(result) = results
            .iter()
            .find(|r| r["id"].as_str().unwrap().eq_ignore_ascii_case(&id))
        {
            map_created.push(json!({"saved_id":placement.serialized_id,"class":placement.class_code,"id":id,"error":result["error"]}));
        }
    }
    let report = json!({"scope":"actual source census and bounded native CREATE_VFX compilation; not runtime command execution", "source_precision":"Double53",
        "source_identities":actors.sources,"results":results,"inputs":inputs,"tutorial02_created":map_created});
    if let Some(path) = std::env::var_os("AA_CAPTURE_FX_GRAPH") {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file, &report).unwrap();
    } else {
        panic!("set explicit new AA_CAPTURE_FX_GRAPH path for census");
    }
}

impl ExistingInputs for CreatedInput {
    fn source_definition_attachment(&mut self) -> Result<()> {
        if self.case["source_flags"].as_u64().unwrap() & 0xC0000 != 0 {
            self.events.push(json!("source_BC"));
        }
        //The paired native fixture's explicit virtualBC service returns false.
        Ok(())
    }
}
#[test]
fn original_v14_existing_event_and_source_predicate_order_match() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research");
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(root.join("development10/native-existing-fx-command.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 864);
    let graph_fixture: Value = serde_json::from_slice(
        &std::fs::read(root.join("skirmish-20261016-fx-graph/native-fx-graph.json")).unwrap(),
    )
    .unwrap();
    let ordinary = &graph_fixture["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["input"]["name"] == "ordinary")
        .unwrap()["input"];
    for row in fixture["rows"].as_array().unwrap() {
        let case = &row["input"];
        let mut input = Fixture {
            case: ordinary.clone(),
            events: Vec::new(),
        };
        let mut draft = compile_inputs(&mut input, vec![0]).unwrap();
        if !case["nonempty"].as_bool().unwrap() {
            draft.roots.clear();
        }
        let graph = std::sync::Arc::new(Graph {
            draft,
            prior_roots: Vec::new(),
            owner: "fixture".into(),
            source: Provenance {
                path: "fixture".into(),
                sha256: "fixture".into(),
                span: aa_rules::tgi::Span { start: 0, end: 0 },
            },
            precision: Precision::Double53,
            sources: Vec::new(),
            kind: GraphKind::View(ListKind::Constructed),
            request_only: false,
        });
        let mut input = CreatedInput {
            case: case.clone(),
            events: Vec::new(),
            positions: 0,
            things: 0,
            clock_reads: Vec::new(),
        };
        let command =
            construct_existing(graph, &mut input, case["flag"].as_u64().unwrap() as u8).unwrap();
        let command=command.map(|c| {
            let mut words=c.words.clone();
            words.insert(0xc,c.kingdom().map_or(0,aa_actors::ActorId::raw));
            words.insert(0x10,c.source().map_or(0,aa_actors::ActorId::raw));
            words.insert(0x20,c.target().map_or(0,aa_actors::ActorId::raw));
            assert_eq!(input.positions,4);assert_eq!(input.things,2);
            assert_eq!(input.clock_reads,if case["thread_id"]==case["app_thread"] {vec!["view"]}else{vec!["world"]});
            words.remove(&0x68);
            json!({"words":words,"byte58":c.byte(0x58),"flag":0,"fxl_definition":c.fxl_definition(),"fxl_parent_is_self":c.fxl_parent_is_self(),"word68":c.word(0x68)})
        });
        if command.is_none() {
            assert_eq!(input.positions, 2);
        }
        assert_eq!(
            json!({"command":command,"events":input.events}),
            row["output"],
            "{case}"
        );
    }
}

#[test]
fn native_separate_lists_share_one_node_and_reference_context() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/development10/native-fx-prior-lists.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 18);
    for row in fixture["rows"].as_array().unwrap() {
        let case = &row["input"];
        let mut input = Fixture {
            case: case.clone(),
            events: Vec::new(),
        };
        let mut draft = Draft::new();
        let mut prior = Vec::new();
        let mut failed = false;
        for (i, group) in case["groups"].as_array().unwrap().iter().enumerate() {
            if i != 0 {
                prior.push(std::mem::take(&mut draft.roots));
            }
            let roots = group
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as usize)
                .collect();
            if append_roots(&mut input, roots, &mut draft).is_err() {
                failed = true;
                break;
            }
        }
        let mut actual = snapshot(&draft);
        actual["prior_roots"] = json!(
            prior
                .iter()
                .map(|v| v.iter().map(|n| n.index()).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
        actual["failed"] = json!(failed);
        actual["events"] = json!(input.events);
        let mut expected = row["output"].clone();
        expected["failed"] = json!(!expected["error"].is_null());
        expected.as_object_mut().unwrap().remove("error");
        assert_eq!(actual, expected, "{} / {}", case["name"], row["precision"]);
    }
}

///This checks the strictly narrower projection against real constructor/vtable
///answers; it never labels the seven subtype LoadData bodies completed.
#[test]
fn request_only_native_factory_common_prefix_and_unbound_fields() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-data-pre-request/native-request-only-fxc.json"
    ))
    .unwrap();
    assert_eq!(doc["rows"].as_array().unwrap().len(), 90);
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-fx-graph/native-fx-graph.json"
    ))
    .unwrap();
    let ordinary = &fixture["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["input"]["name"] == "ordinary")
        .unwrap()["input"];
    let mut full = Fixture {
        case: ordinary.clone(),
        events: vec![],
    };
    let full = compile_inputs(&mut full, vec![0]).unwrap();
    let expected = &full.nodes[0];
    for row in doc["rows"].as_array().unwrap() {
        let output = &row["output"];
        let kind = output["kind"].as_u64().unwrap() as usize;
        assert_eq!(output["vf24"], json!(u8::from(kind == 0)));
        assert_eq!(output["vf2c"], json!(u8::from(kind == 3)));
        assert_eq!(output["loader_common_first"], true);
        let common = Node::common(vec![]);
        for (offset, word) in &common.words {
            assert_eq!(json!(word), output["words"][format!("{offset:x}")]);
        }
        for (offset, byte) in &common.bytes {
            assert_eq!(json!(byte), output["bytes"][format!("{offset:x}")]);
        }
        for offset in ["8", "c", "10", "14", "20"] {
            assert_eq!(output["words"][offset], 0);
        }
        if matches!(kind, 0 | 1 | 3) {
            continue;
        }
        let mut case = ordinary.clone();
        case["nodes"][0]["type"] = json!(aa_rules::abilities::FXC_TYPES[kind]);
        let mut default = Fixture {
            case: case.clone(),
            events: vec![],
        };
        let failure = compile_inputs(&mut default, vec![0]).unwrap_err();
        assert!(failure.error.contains("LoadData is required"));
        assert!(failure.partial.nodes.is_empty());
        case["request_only"] = json!(true);
        //Any accidental type-specific read is a real failure, rather than a
        //successful fabricated answer. The shared common fields remain live.
        case["fail"] = json!("vfx_ids");
        let mut input = Fixture {
            case: case.clone(),
            events: vec![],
        };
        let draft = compile_inputs(&mut input, vec![0]).unwrap();
        assert_eq!(
            draft.kinds,
            vec![FxcKind::RequestOnly {
                native_type: kind as u8
            }]
        );
        assert_eq!(draft.roots, vec![NodeId(0)]);
        let value = &draft.nodes[0];
        assert!(!value.complete());
        assert!(value.word(0x3c).is_none());
        assert!(value.word(0x68).is_none());
        assert!(value.reference(0x3c).is_none());
        for offset in [0x18, 0x1c, 0x24, 0x28, 0x2c, 0x30, 0x34] {
            assert_eq!(value.word(offset), expected.word(offset));
        }
        assert_eq!(value.byte(0x38), expected.byte(0x38));
        assert_eq!(value.reference(0x20), expected.reference(0x20));
        case["fail"] = json!("health");
        let mut input = Fixture {
            case,
            events: vec![],
        };
        let failure = compile_inputs(&mut input, vec![0]).unwrap_err();
        assert!(failure.error.contains("health"));
        assert!(failure.partial.roots.is_empty());
        assert_eq!(failure.partial.nodes[0].word(0x24), Some(1));
        assert_eq!(failure.partial.nodes[0].word(0x28), Some(0));
        assert_eq!(failure.partial.nodes[0].word(0x2c), Some(0));
        assert!(!failure.partial.nodes[0].complete());
    }
}
#[test]
fn request_only_and_incomplete_graphs_refuse_every_command_constructor_before_callbacks() {
    struct NoReads;
    impl CreationInputs for NoReads {
        fn muted(&mut self) -> Result<bool> {
            panic!("no muted read")
        }
        fn creator_kingdom(&mut self) -> Result<Option<aa_actors::ActorId>> {
            panic!("no Kingdom read")
        }
        fn current_position(&mut self) -> Result<[u32; 3]> {
            panic!("no pose read")
        }
        fn current_thing(&mut self) -> Result<Option<aa_actors::ActorId>> {
            panic!("no Thing read")
        }
        fn current_os_thread_id(&mut self) -> Result<u32> {
            panic!("no thread read")
        }
        fn application_os_thread_id(&mut self) -> Result<u32> {
            panic!("no App read")
        }
        fn view_time(&mut self) -> Result<u32> {
            panic!("no View read")
        }
        fn world_time(&mut self) -> Result<u32> {
            panic!("no World read")
        }
    }
    impl ExistingInputs for NoReads {
        fn source_definition_attachment(&mut self) -> Result<()> {
            panic!("no attachment read")
        }
    }
    impl StateCreationInputs for NoReads {
        fn global_position(&mut self) -> Result<[u32; 3]> {
            panic!("no global pose read")
        }
    }
    for (request_only, populated) in [(true, false), (true, true), (false, true)] {
        let mut draft = Draft::new();
        if populated {
            draft.nodes.push(Node::common(vec![]));
            draft.kinds.push(FxcKind::RequestOnly { native_type: 2 });
            draft.roots.push(NodeId(0));
        }
        //Explicit laboratory owner for a guard test, never a source receipt.
        let graph = std::sync::Arc::new(Graph {
            draft,
            prior_roots: vec![],
            owner: "guard fixture".into(),
            source: Provenance {
                path: "guard fixture".into(),
                sha256: String::new(),
                span: aa_rules::tgi::Span { start: 0, end: 0 },
            },
            precision: Precision::Double53,
            sources: vec![],
            kind: GraphKind::ActorEnter,
            request_only,
        });
        if request_only {
            assert_eq!(graph.kind(), GraphKind::RequestOnly);
        }
        assert!(
            construct_created(graph.clone(), &mut NoReads)
                .unwrap_err()
                .contains("cannot execute")
        );
        assert!(
            construct_existing(graph.clone(), &mut NoReads, 0)
                .unwrap_err()
                .contains("cannot execute")
        );
        assert!(
            construct_actor_state(
                MatchedState {
                    graph,
                    state: 0,
                    active_only: false
                },
                &mut NoReads
            )
            .unwrap_err()
            .contains("cannot execute")
        );
    }
}
