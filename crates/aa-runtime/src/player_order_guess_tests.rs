use super::*;
use crate::player_order_world::Error;
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-orders-input/guess-4da570/native-guess-4da570.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn opt(v: &Value) -> Option<u32> {
    v.as_u64().map(|x| x as u32)
}
fn units(v: &Value) -> Vec<u16> {
    v.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u16).collect()
}

/// The laboratory memory of one oracle case, answered through the trait.
struct Lab<'a> {
    case: &'a Value,
    variables: &'a Value,
    events: Vec<Value>,
}
impl Lab<'_> {
    fn object(&self, id: u32) -> &Value {
        &self.case["objects"][id.to_string()]
    }
    fn al(v: &Value) -> bool {
        u(v) & 0xff != 0
    }
}
impl World for Lab<'_> {
    type Object = u32;
    type Organization = u32;
    type Kingdom = u32;
    type Definition = u32;
    type Error = Error;
    fn resolve_10(&mut self, handle: u32) -> Result<Option<u32>, Error> {
        let slot = handle & 0xffff;
        Ok(self.case["table"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| u(&e["slot"]) == slot)
            .filter(|e| u(&e["generation"]) == handle >> 16)
            .and_then(|e| opt(&e["object"])))
    }
    fn kingdom_108(&mut self, target: u32) -> Result<Option<u32>, Error> {
        self.events.push(json!(["108", target]));
        let k = u(&self.object(target)["kingdom108"]);
        Ok((k != 0).then_some(k))
    }
    fn hostile_140(&mut self, actor: u32, kingdom: Option<u32>) -> Result<bool, Error> {
        let k = kingdom.unwrap_or(0);
        self.events.push(json!(["140", actor, k]));
        Ok(Self::al(&self.object(actor)["rel140"][k.to_string()]))
    }
    fn friendly_138(&mut self, actor: u32, kingdom: Option<u32>) -> Result<bool, Error> {
        let k = kingdom.unwrap_or(0);
        self.events.push(json!(["138", actor, k]));
        Ok(Self::al(&self.object(actor)["rel138"][k.to_string()]))
    }
    fn neutral_13c(&mut self, actor: u32, kingdom: u32) -> Result<bool, Error> {
        self.events.push(json!(["13c", actor, kingdom]));
        Ok(Self::al(&self.object(actor)["rel13c"][kingdom.to_string()]))
    }
    fn organization_7c(&mut self, object: u32) -> Result<Option<u32>, Error> {
        Ok((!self.object(object)["org"].is_null()).then_some(object))
    }
    fn members_28(&mut self, organization: u32) -> Result<Vec<Option<u32>>, Error> {
        Ok(self.object(organization)["org"].as_array().unwrap().iter().map(opt).collect())
    }
    fn element_flag_1c(&mut self, member: u32) -> Result<Option<bool>, Error> {
        Ok(opt(&self.object(member)["element"]).map(|b| b & 0xff != 0))
    }
    fn construction_9c(&mut self, object: u32) -> Result<bool, Error> {
        Ok(self.object(object)["construction"].as_bool().unwrap())
    }
    fn state_100(&mut self, object: u32) -> Result<u32, Error> {
        Ok(u(&self.object(object)["flags100"]))
    }
    fn body_60(&mut self, object: u32) -> Result<Option<[u32; 2]>, Error> {
        let b = &self.object(object)["body"];
        Ok((!b.is_null()).then(|| [u(&b[0]), u(&b[1])]))
    }
    fn swap_right_click_4be150(&mut self) -> Result<bool, Error> {
        self.events.push(json!(["swap"]));
        match opt(&self.variables["swap"]) {
            Some(byte) => Ok(byte & 0xff != 0),
            None => Err(Error::Required("laboratory: no SwapRightClickWithAltRightClick record".into())),
        }
    }
    fn hotkey_alt_right_click(&mut self) -> Result<Vec<u16>, Error> {
        self.events.push(json!(["hotkey"]));
        let h = &self.variables["hotkey"];
        if h.is_null() {
            return Err(Error::Required("laboratory: no HotkeyAltRightClickCommand record".into()));
        }
        Ok(units(h))
    }
    fn allowed_34(&mut self, actor: u32, index: u32) -> Result<bool, Error> {
        self.events.push(json!(["34", actor, index]));
        Ok(Self::al(&self.object(actor)["allowed34"][index as usize]))
    }
    fn definition(&mut self, index: u32) -> Result<Definition, Error> {
        let d = &self.case["definitions"][index as usize];
        Ok(Definition {
            type_0c: u(&d["type"]),
            requirements_2c: u(&d["flags"]),
            non_group_3c: u(&d["b3c"]) != 0,
            hot_actor_3f: u(&d["b3f"]) != 0,
            hot_location_40: u(&d["b40"]) != 0,
        })
    }
    fn builder_a8(&mut self, object: u32) -> Result<bool, Error> {
        Ok(self.object(object)["builder"].as_bool().unwrap())
    }
    fn builder_targets_504(&mut self, object: u32) -> Result<Vec<u32>, Error> {
        let d = u(&self.object(object)["def4"]);
        Ok(self.case["actor_data"][d.to_string()]["builds"].as_array().unwrap().iter().map(u).collect())
    }
    fn settlement_98(&mut self, object: u32) -> Result<Option<Settlement<u32>>, Error> {
        let s = &self.object(object)["settlement"];
        if s.is_null() {
            return Ok(None);
        }
        Ok(Some(Settlement {
            parent_14: opt(&s["parent"]),
            children_18: s["children"].as_array().unwrap().iter().map(opt).collect(),
        }))
    }
    fn definition_4(&mut self, object: u32) -> Result<Option<u32>, Error> {
        Ok(opt(&self.object(object)["def4"]))
    }
    fn required_object_4e0(&mut self, definition: &u32) -> Result<Option<u32>, Error> {
        Ok(opt(&self.case["actor_data"][definition.to_string()]["r4e0"]))
    }
}

/// Replays one 4DA570 call; returns the number of candidate lists compared.
fn replay(case: &Value, variables: &Value, native: &Value, context: &str) -> usize {
    let mut lab = Lab { case, variables, events: Vec::new() };
    let command = CommandFields { flags_0c: u(&case["command"]["flags"]), target_10: u(&case["command"]["handle"]) };
    let actor = u(&case["actor"]);
    let result = guess_command_data(&mut lab, command, actor, u(&case["arg2"]) & 0xff != 0);
    let native_events: Vec<Value> =
        native["events"].as_array().unwrap().iter().filter(|e| e[0] != "candidates" && e[0] != "log").cloned().collect();
    let mut compared = 0;
    for e in native["events"].as_array().unwrap().iter().filter(|e| e[0] == "candidates") {
        let mut fresh = Lab { case, variables, events: Vec::new() };
        let list = candidates_4d44e0(&mut fresh, actor).unwrap();
        assert_eq!(json!(list), e[1], "4D44E0 list {context}");
        compared += 1;
    }
    if !native["fault"].is_null() {
        assert!(result.is_err(), "native fault must be a Rust error {context}: {result:?}");
        assert_eq!(json!(lab.events), json!(native_events), "events before fault {context}");
        return compared;
    }
    let outcome = result.unwrap_or_else(|e| panic!("{context}: {e}"));
    let mut events = lab.events.clone();
    if let Some(handle) = outcome.invalid_target_logged {
        events.push(json!(["log", handle]));
        let log = &native["logs"][0];
        assert_eq!(native["logs"].as_array().unwrap().len(), 1);
        assert_eq!(u(&log["handle"]), handle);
        assert_eq!(u(&log["line"]), 0x12a);
        assert_eq!(log["format"], "GuessCommandData - target actor %d is invalid");
        assert_eq!(log["function"], "KKC_GActorCommand::GuessCommandData");
        assert_eq!(log["file"], ".\\Source\\Sim\\GActorCommands.cpp");
    } else {
        assert!(native["logs"].as_array().unwrap().is_empty(), "{context}");
    }
    let native_all: Vec<Value> = native["events"].as_array().unwrap().iter().filter(|e| e[0] != "candidates").cloned().collect();
    assert_eq!(json!(events), json!(native_all), "events {context}");
    let mut flags = command.flags_0c;
    let mut object = None;
    let guessed = outcome.apply(&mut flags, &mut object);
    assert_eq!(json!(guessed.map(|g| g.index)), native["result"], "result {context}");
    assert_eq!(flags, u(&native["flags"]), "flags {context}");
    assert_eq!(json!(object), native["object14"], "object14 {context}");
    if let Some(g) = guessed {
        assert_eq!(g.definition, lab.definition(g.index).unwrap());
    }
    compared
}

#[test]
fn native_4da570_rows_replay() {
    let f = fixture();
    assert_eq!(f["schema"], 1);
    assert_eq!(f["identical_across_runs"], true);
    let runs: Vec<String> = f["runs"].as_array().unwrap().iter()
        .map(|r| format!("{}{}", r["image"].as_str().unwrap(), r["precision"].as_str().unwrap())).collect();
    assert_eq!(runs, ["originalSingle24", "originalDouble53", "originalExtended64", "v14Single24", "v14Double53", "v14Extended64"]);
    let cases = f["cases"].as_array().unwrap();
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(cases.len(), rows.len());
    let (mut faults, mut guessed, mut completed, mut lists, mut seconds) = (0, 0, 0, 0, 0);
    for (i, (case, row)) in cases.iter().zip(rows).enumerate() {
        let first = &row["output"];
        lists += replay(case, &case["variables"], first, &format!("row {i} {}", case["kind"]));
        faults += usize::from(!first["fault"].is_null());
        guessed += usize::from(!first["result"].is_null());
        completed += usize::from(!first["object14"].is_null());
        assert_ne!(first["unknown_return"], true);
        // B3CB9C caches only the two record pointers (registry B39938).
        let lookups = first["lookups"].as_array().unwrap();
        let hotkey_read = first["events"].as_array().unwrap().iter().any(|e| e[0] == "hotkey");
        if first["events"].as_array().unwrap().iter().any(|e| e[0] == "swap") {
            assert_eq!(lookups[0], json!(["SwapRightClickWithAltRightClick", 1, 0xb39938]));
            assert_eq!(lookups.len(), 1 + usize::from(hotkey_read));
            if hotkey_read {
                assert_eq!(lookups[1], json!(["HotkeyAltRightClickCommand", 4, 0xb39938]));
            }
        } else {
            assert!(lookups.is_empty());
        }
        if !row["second"].is_null() {
            let second = &row["second"];
            lists += replay(case, &case["second_variables"], second, &format!("row {i} second call"));
            let second_hotkey = second["events"].as_array().unwrap().iter().any(|e| e[0] == "hotkey");
            let expected: Vec<Value> = if second_hotkey && !hotkey_read {
                vec![json!(["HotkeyAltRightClickCommand", 4, 0xb39938])]
            } else {
                vec![]
            };
            assert_eq!(second["lookups"], json!(expected), "row {i} cached records");
            seconds += 1;
        }
    }
    assert_eq!(rows.len(), 757);
    assert_eq!((faults, seconds), (35, 8));
    assert!(guessed > 250 && completed > 20 && lists > 120, "{guessed} {completed} {lists}");
}

#[test]
fn native_enum_b3cba0_and_497220_lookups() {
    let f = fixture();
    let entries: Vec<Value> = aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS
        .iter()
        .map(|d| json!([d.mind_type, d.id, 0]))
        .collect();
    assert_eq!(f["enum_entries"], json!(entries));
    let lookups = f["enum_lookups"].as_array().unwrap();
    assert!(lookups.len() > 90);
    for row in lookups {
        assert_eq!(json!(hot_command_497220(&units(&row["name"]))), row["value"], "{row}");
    }
}

#[test]
fn actual_definition_fields_match_registration_and_controls() {
    // 4740F0 stores type at +0C, component at +18, flags at +2C (475870
    // argument order); 4B4E60 rewrites +0C with the registration index.
    let f = fixture();
    for (i, d) in f["definitions_actual"].as_array().unwrap().iter().enumerate() {
        let p = aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS[i];
        assert_eq!(d["ids"], p.id);
        assert_eq!(u(&d["type"]), p.mind_type);
        assert_eq!(p.mind_type, i as u32);
        assert_eq!(u(&d["flags"]), p.behavior_flags);
    }
}

struct NoVariables;
impl crate::player_order_world::Globals for NoVariables {
    fn can_process_40eaa0(&mut self) -> crate::player_order_world::Result<bool> {
        panic!("4DA570 never reads the 40EAA0 gate")
    }
    fn session_phase_76ac90(&mut self) -> crate::player_order_world::Result<u32> {
        panic!("4DA570 never reads the session phase")
    }
}

#[test]
fn unbound_uvars_are_required_and_invalid_targets_log_first() {
    use crate::player_order_world::Globals;
    let mut g = NoVariables;
    assert!(g.swap_right_click_with_alt_right_click_4be150().unwrap_err().to_string().contains("4BE130/4BE150"));
    assert!(g.hotkey_alt_right_click_command_4b2e60().unwrap_err().to_string().contains("4B2E60"));
}

/// The live controls binding (48CF0A..48CF97 in the supplied Bank) agrees
/// with the separately produced native controls for all 39 definitions.
#[test]
fn live_definitions_bind_actual_controls() {
    use aa_rules::{actors::ActorCatalog, sources::{SourceLimits, V14Sources}, tgi::Limits};
    use crate::{combat_source_completion as completion, combat_source_receipt::VerifiedStartupReceipt, settings::Settings};
    let Some(game) = std::env::var_os("AA_GAME_ROOT").map(std::path::PathBuf::from) else {
        eprintln!("AA_GAME_ROOT unset: live control binding not exercised");
        return;
    };
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits { max_member_bytes: 64 * 1024 * 1024 },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(settings);
    let receipt = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut sources).unwrap();
    completion::install(&catalog, &mut sources, receipt.context(), receipt.csv(), receipt.globals()).unwrap();
    completion::compute(&catalog).unwrap();
    let mut store = crate::actor_world::ActorStore::new();
    let (_, access) = store.split_minds();
    let mut globals = NoVariables;
    let mut live = Live { catalog: &catalog, banks: &catalog.definition_bank, actors: &access, globals: &mut globals };
    let f = fixture();
    for (i, d) in f["definitions_actual"].as_array().unwrap().iter().enumerate() {
        let got = live.definition(i as u32).unwrap();
        assert_eq!(got, Definition {
            type_0c: u(&d["type"]),
            requirements_2c: u(&d["flags"]),
            non_group_3c: u(&d["b3c"]) != 0,
            hot_actor_3f: u(&d["b3f"]) != 0,
            hot_location_40: u(&d["b40"]) != 0,
        }, "{}", d["ids"]);
    }
    // An unresolved target logs before any UVar read (no owner needed).
    let outcome = guess_command_data(&mut live, CommandFields { flags_0c: 0x11, target_10: 0x0001_0005 }, ActorId::from_raw(1), false).unwrap();
    assert_eq!(outcome, Outcome { guessed: None, completed_object_14: None, invalid_target_logged: Some(0x0001_0005) });
    // Without a target the first reached leaf is the unbound UVar owner.
    let error = guess_command_data(&mut live, CommandFields { flags_0c: 0x14, target_10: 0 }, ActorId::from_raw(1), false).unwrap_err();
    assert!(error.to_string().contains("4BE130/4BE150"), "{error}");
}
