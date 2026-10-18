use super::super::*;
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
        if self.case["fail"] == format!("find:{manager}") {
            return Err(format!("required find:{manager}"));
        }
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
        Ok((!matches!(ids, "missing" | "")).then(|| Reference {
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
        if self.case["fail"] == format!("enum:{domain}") {
            return Err(format!("required enum:{domain}"));
        }
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
fn original_v14_play_sfx_full_state_order_and_partial_stores_match() {
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/skirmish-20261017-fx-sound/native-fx-sound.json"),
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

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 sources; compilation only, not audio playback"]
fn actual_farm_inherited_selected_sound_binds_registered_audio_at_each_pc() {
    use aa_rules::sources::{SourceLimits, V14Sources};
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
    let ids = String::from_utf16(&placement.thing_id_utf16).unwrap();
    let actor = actors
        .actors
        .iter()
        .find(|a| a.id.eq_ignore_ascii_case(&ids))
        .unwrap();
    let node = actors.resolved_actor(&actor.id).unwrap();
    let roots = node
        .child("View")
        .unwrap()
        .unwrap()
        .children("WhenSelected")
        .unwrap();
    assert_eq!(roots.len(), 1);
    let mut capture = None;
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let mut input = Bound::new(&actors, pc, actor.source.clone()).unwrap();
        let root = &roots[0];
        assert_eq!(input.text(root, "type").unwrap(), "PLAY_SFX");
        assert!(input.find("Audio", "").unwrap().is_none());
        assert!(input.find("Audio", "audio_é").is_err());
        assert!(input.find("Audio", "missing-test-audio").unwrap().is_none());
        let reference = input
            .find("Audio", "CITY_BUILDING_SELECT")
            .unwrap()
            .unwrap();
        assert_eq!(reference.id, "city_building_select");
        assert!(!reference.source.is_empty());
        let mut raw = serde_json::Map::new();
        raw.insert("ChainImmediate".into(), json!([]));
        raw.insert("Chain".into(), json!([]));
        let mut lookups = json!({"Audio":{},"VFX":{},"TerrainGroup":{},"ActorState":{},"Bone":{}});
        for name in [
            "type",
            "target_flags",
            "terrain_group_filter",
            "health_trigger_type",
            "actor_state",
            "audio_ids",
            "at",
        ] {
            raw.insert(name.into(), json!(input.text(root, name).unwrap()));
        }
        raw.insert(
            "frequency".into(),
            json!(input.integer(root, "frequency").unwrap()),
        );
        for name in ["delay", "health"] {
            raw.insert(name.into(), json!(input.scalar(root, name).unwrap()));
        }
        for name in ["stop_on_destructed", "looping"] {
            raw.insert(name.into(), json!(input.flag(root, name).unwrap()));
        }
        for (manager, field) in [
            ("Audio", "audio_ids"),
            ("TerrainGroup", "terrain_group_filter"),
            ("ActorState", "actor_state"),
        ] {
            let id = raw[field].as_str().unwrap();
            if !id.is_empty() {
                lookups[manager][id] = json!(input.find(manager, id).unwrap());
            }
        }
        let result = compile_inputs(&mut input, roots.clone()).unwrap();
        assert_eq!(result.roots().len(), 1);
        let compiled = &result.nodes()[result.roots()[0].index()];
        assert!(compiled.complete());
        assert_eq!(
            compiled.reference(0x44).unwrap().as_ref().unwrap().id,
            "city_building_select"
        );
        assert_eq!(compiled.word(0x3c), Some(0));
        assert_eq!(compiled.byte(0x40), Some(0));
        assert_eq!(compiled.word(0x68), None);
        if pc == Precision::Double53 {
            capture = Some(
                json!({"schema":1,"scope":"actual tutorial02 saved2696 inherited selected sound input; no playback", 
                "definition":actor.id,"definition_source":actor.source,"source_identities":actors.sources,
                "input":{"name":"actual_farm_selected","nodes":[raw],"roots":[0],"lookups":lookups},
                "compiled":snapshot(&result)}),
            );
        }
    }
    if let Some(path) = std::env::var_os("AA_CAPTURE_FX_SOUND") {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file, &capture.unwrap()).unwrap();
    }
}
