//! Replays research/phase8-metagame/native-save.json: the original metagame
//! save writer 938EE0 and loader 93A3B0 (with every sub-record) and the army
//! component record 93EEA0 / 940740, run under Unicorn on perturbed
//! InitCreate worlds of every shipped campaign (plus two synthetic campaign
//! events, so 939B90 creates event states), against `aa_metagame::save` and
//! the `MgState` model.
//!
//! Per round: the Rust replay of the recorded operations must (1) equal the
//! native source state (including the army snapshots `MgState::enter_territory`
//! captures, 9402E0, and the InitCreate event states), (2) write the native
//! bytes, (3) parse the native bytes into the native loaded state, (4)
//! re-write the bytes the native loaded state writes; every army component
//! record must be written and parsed like the original. Error and edge cases
//! are replayed on the native (patched or synthetic) payloads.
use aa_metagame::battle::{DivisionRef, KingdomActor, KingdomRelations};
use aa_metagame::data::{CampaignEvent, MetaGameData};
use aa_metagame::save::{
    parse_metagame, write_metagame, ArmyComponentImage, EngineIdentities, MetagameImage, ParsedMetagame,
    SaveRegistries, SavedEventState,
};
use aa_metagame::state::{ArmyId, BattleRecord, EventState, InitEngine, MgState, INITIAL_RNG_STATE};
use aa_metagame::Error;
use serde_json::{json, Value};

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../research/phase8-metagame/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

struct Teams(Vec<Option<usize>>);
impl KingdomRelations for Teams {
    fn enemy(&self, s: KingdomActor, o: KingdomActor) -> bool {
        self.0[s.0 as usize] != self.0[o.0 as usize]
    }
    fn relation_138(&self, s: KingdomActor, o: KingdomActor) -> bool {
        self.0[s.0 as usize] == self.0[o.0 as usize]
    }
}

fn u(v: &Value) -> usize {
    v.as_u64().unwrap() as usize
}
fn strs(v: &Value) -> Vec<String> {
    v.as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_owned()).collect()
}

/// GTID handles of the oracle world.
fn identities(row: &Value) -> EngineIdentities {
    let k: Vec<u32> = row["kingdom_actor_handles"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u32).collect();
    let a: Vec<u32> = row["army_handles"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u32).collect();
    EngineIdentities {
        kingdom_actors: k.iter().enumerate().map(|(i, &h)| (KingdomActor(i as u32), h)).collect(),
        armies: a.iter().enumerate().map(|(i, &h)| (ArmyId(i as u32), h)).collect(),
    }
}

/// 4BEDF0 names of the oracle kingdom actors (actor `+0x18`).
fn names(row: &Value) -> Vec<(KingdomActor, String)> {
    strs(&row["kingdom_names"]).into_iter().enumerate().map(|(i, n)| (KingdomActor(i as u32), n)).collect()
}

fn registries(row: &Value) -> SaveRegistries {
    SaveRegistries {
        factions: strs(&row["factions"]),
        shared_properties: strs(&row["shared_properties"]),
        help_popups: strs(&row["help_popups"]),
        things: row["things"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| (t[0].as_str().unwrap().to_owned(), t[1].as_bool().unwrap()))
            .collect(),
    }
}

/// The oracle adds synthetic campaign event data objects (IDS only) to the
/// campaign before 939B90 runs; mirror them in the data.
fn with_events(data: &MetaGameData, ci: usize, events: &[String]) -> MetaGameData {
    let mut d = data.clone();
    assert!(d.campaigns[ci].events.is_empty());
    for ids in events {
        d.campaigns[ci].events.push(CampaignEvent {
            ids: ids.clone(),
            name: String::new(),
            round: 0,
            round_probability_bits: 0,
            message: String::new(),
            kingdom_turn: None,
            effects: Vec::new(),
        });
    }
    d
}

/// The image in the shape of the oracle's `dump_state` (IDS / positions).
fn dump(image: &MetagameImage, data: &MetaGameData) -> Value {
    let c = &data.campaigns[image.campaign.unwrap()];
    let map = &data.maps[c.map.unwrap()];
    let kingdom_ids = |k: usize| c.kingdoms[image.kingdoms[k].data.unwrap()].ids.clone();
    let tdef = |d: Option<usize>| d.map(|d| map.territories[d].ids.clone());
    json!({
        "campaign": c.ids,
        "faction": image.faction,
        "current": image.current,
        "round": image.round,
        "rng": image.rng,
        "bytes_44_45_46_50_51": [image.flags_44_45_46[0], image.flags_44_45_46[1], image.flags_44_45_46[2], image.byte_50, image.byte_51],
        "string_48": image.rts_winner,
        "last_purchased_handle": image.last_purchased,
        "kingdoms": image.kingdoms.iter().enumerate().map(|(i, k)| json!({
            "state_back_pointer": true,
            "data": kingdom_ids(i),
            "actor_handle": k.actor,
            "capital": k.capital,
            "territories": k.territories,
            "technologies": k.technologies,
            "help_popups": k.help_popups,
            "resources": k.resources,
            "battles": k.battles,
        })).collect::<Vec<_>>(),
        "territories": image.territories.iter().map(|t| json!({
            "index": t.index,
            "definition": map.territories[t.definition].ids,
            "visual_handle": t.visual,
            "owner": t.owner,
            "enemy_army_count": t.enemy_army_count,
            "armies": t.armies.iter().map(|r| json!({
                "handle": r.handle,
                "slot": r.slot,
                "snapshot": {
                    "fought": r.snapshot.fought,
                    "territory": tdef(r.snapshot.territory),
                    "path": r.snapshot.path.iter().map(|&p| tdef(p)).collect::<Vec<_>>(),
                    "divisions": r.snapshot.divisions.iter().map(|d| d.map(|d| data.divisions[d.0 as usize].ids.clone())).collect::<Vec<_>>(),
                    "owner_name": r.snapshot.owner_name,
                    "definition": r.snapshot.definition.map(|d| data.armies[d].ids.clone()),
                },
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "events": image.events.iter().map(|e| json!({
            "event": c.events[e.event].ids,
            "flag": u8::from(e.triggered),
            "target": e.kingdom.map(|k| c.kingdoms[k].ids.clone()),
        })).collect::<Vec<_>>(),
        "records": image.records.iter().map(|r| json!({
            "territory": r.territory, "attacker": r.attacker, "defender": r.defender, "attacker_won": r.attacker_won,
        })).collect::<Vec<_>>(),
    })
}

/// Native dump without the fields the image does not carry (`has_record`).
fn strip(native: &Value) -> Value {
    let mut v = native.clone();
    for t in v["territories"].as_array_mut().unwrap() {
        t.as_object_mut().unwrap().remove("has_record");
    }
    v
}

fn assert_same(rust: &Value, native: &Value, what: &str) {
    if rust != native {
        let r = rust.as_object().unwrap();
        for (k, nv) in native.as_object().unwrap() {
            if r.get(k) != Some(nv) {
                if let (Some(ra), Some(na)) = (r.get(k).and_then(Value::as_array), nv.as_array()) {
                    for (i, (x, y)) in ra.iter().zip(na).enumerate() {
                        assert_eq!(x, y, "{what}: field {k}[{i}]");
                    }
                    assert_eq!(ra.len(), na.len(), "{what}: field {k} length");
                }
                assert_eq!(r.get(k), Some(nv), "{what}: field {k}");
            }
        }
        assert_eq!(rust, native, "{what}");
    }
}

fn component_dump(img: &ArmyComponentImage, data: &MetaGameData) -> Value {
    json!({
        "fought": img.fought,
        "territory": img.territory,
        "path": img.path,
        "waypoints": img.waypoints,
        "divisions": img.divisions.iter().map(|d| d.map(|d| data.divisions[d.0 as usize].ids.clone())).collect::<Vec<_>>(),
    })
}

struct Replay<'a> {
    rel: &'a Teams,
    state: MgState,
    registered: Vec<bool>,
}

impl Replay<'_> {
    fn apply(&mut self, op: &Value) {
        let o = op.as_array().unwrap();
        let st = &mut self.state;
        let opt_str = |v: &Value| v.as_str().map(str::to_owned);
        let null_str = |v: &Value| v.as_str().unwrap_or("").to_owned();
        match o[0].as_str().unwrap() {
            "fought" => st.armies[u(&o[1])].as_mut().unwrap().fought = o[2].as_i64().unwrap() as i32,
            "divisions" => {
                st.armies[u(&o[1])].as_mut().unwrap().divisions =
                    o[2].as_array().unwrap().iter().map(|d| DivisionRef(d.as_u64().unwrap() as u32)).collect()
            }
            // 940100, including the 9402E0 snapshot of the new reference
            "move" => {
                st.enter_territory(ArmyId(u(&o[1]) as u32), u(&o[2]), self.rel).unwrap();
            }
            "owner" => st.set_owner(u(&o[1]), u(&o[2]), self.rel),
            "waypoints" => st.armies[u(&o[1])].as_mut().unwrap().waypoints = o[2].as_array().unwrap().iter().map(u).collect(),
            "resources" => st.kingdoms[u(&o[1])].resources = o[2].as_i64().unwrap() as i32,
            "battles" => st.kingdoms[u(&o[1])].battles = o[2].as_i64().unwrap() as i32,
            "technologies" => st.kingdoms[u(&o[1])].technologies = o[2].as_array().unwrap().iter().map(null_str).collect(),
            "help_popups" => st.kingdoms[u(&o[1])].help_popups = o[2].as_array().unwrap().iter().map(null_str).collect(),
            "visual" => st.territories[u(&o[1])].visual = Some(o[2].as_u64().unwrap() as u32),
            "round" => st.round = o[1].as_i64().unwrap() as i32,
            "rng" => st.rng.state = o[1].as_u64().unwrap() as u32,
            "flags" => {
                let b: Vec<u64> = o[1..].iter().map(|v| v.as_u64().unwrap()).collect();
                st.battle_started = b[0] != 0;
                st.battle_play_requested = b[1] != 0;
                st.battle_resolved = b[2] != 0;
                st.game_over = b[3] != 0;
                st.fast_ai = b[4] as u8;
            }
            "string_48" => st.rts_winner = o[1].as_str().unwrap().to_owned(),
            "last_purchased" => st.last_purchased = o[1].as_u64().map(|a| ArmyId(a as u32)),
            "faction" => st.faction = opt_str(&o[1]),
            "current" => st.current = Some(u(&o[1])),
            "records" => {
                st.results = o[1]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| BattleRecord {
                        territory: u(&r[0]),
                        attacker: Some(u(&r[1])),
                        defender: Some(u(&r[2])),
                        attacker_won: r[3].as_u64().unwrap() != 0,
                    })
                    .collect()
            }
            "events" => {
                st.events = o[1]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|e| EventState {
                        event: u(&e[0]),
                        triggered: e[1].as_u64().unwrap() != 0,
                        kingdom: e[2].as_u64().map(|k| k as usize),
                    })
                    .collect()
            }
            "registered" => self.registered[u(&o[1])] = o[2].as_bool().unwrap(),
            other => panic!("unknown op {other}"),
        }
    }
}

fn load_fixture() -> (MetaGameData, Value) {
    let data_bytes = read("metagame-data.json");
    let data: MetaGameData = serde_json::from_slice(&data_bytes).unwrap();
    let fx: Value = serde_json::from_slice(&read("native-save.json")).unwrap();
    assert_eq!(fx["data_sha256"].as_str().unwrap(), aa_rules::sources::sha256(&data_bytes));
    (data, fx)
}

fn teams(data: &MetaGameData, ci: usize) -> Teams {
    let c = &data.campaigns[ci];
    let mut team = vec![None; c.kingdoms.len()];
    for (ti, t) in c.teams.iter().enumerate() {
        for &k in &t.kingdoms {
            team[k] = Some(ti);
        }
    }
    Teams(team)
}

/// 939B90 with the oracle world's engine seams: kingdom actor i = campaign
/// kingdom i, names and territory visual handles as the oracle recorded
/// them, and the oracle's 9397E0 faction object (empty IDS).
fn init_world(data: &MetaGameData, ci: usize, row: &Value, rel: &Teams) -> MgState {
    let names = names(&row["identities"]);
    let visual: Vec<u32> = row["rounds"][0]["source"]["territories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["visual_handle"].as_u64().unwrap() as u32)
        .collect();
    let engine = InitEngine {
        kingdom_actor: &|i| Some(KingdomActor(i as u32)),
        kingdom_name: &|a| names.iter().find(|(x, _)| *x == a).map(|(_, n)| n.clone()),
        territory_visual: &|t| visual.get(t).copied(),
        faction: Some(String::new()),
    };
    MgState::init_create_with(data, ci, &engine, rel).unwrap()
}

#[test]
fn metagame_save_matches_native_938ee0_93a3b0() {
    let (base, fx) = load_fixture();
    let rows = fx["campaigns"].as_array().unwrap();
    assert_eq!(rows.len(), base.campaigns.len());
    let (mut saves, mut components) = (0, 0);
    for (ci, row) in rows.iter().enumerate() {
        let data = with_events(&base, ci, &strs(&row["registries"]["events"]));
        assert_eq!(row["ids"].as_str().unwrap(), data.campaigns[ci].ids);
        let ids = identities(&row["identities"]);
        let reg = registries(&row["registries"]);
        let rel = teams(&data, ci);
        let state = init_world(&data, ci, row, &rel);
        assert_eq!(state.rng.state, INITIAL_RNG_STATE);
        let n_armies = state.armies.len();
        let mut rp = Replay { rel: &rel, state, registered: vec![false; n_armies] };
        for (ri, round) in row["rounds"].as_array().unwrap().iter().enumerate() {
            let what = format!("campaign {ci} round {ri}");
            for op in round["ops"].as_array().unwrap() {
                rp.apply(op);
            }
            // (1) the replayed state is the native source state
            let image = MetagameImage::from_state(&rp.state, &ids).unwrap();
            assert_same(&dump(&image, &data), &strip(&round["source"]), &format!("{what} source"));
            for (t, nt) in rp.state.territories.iter().zip(round["source"]["territories"].as_array().unwrap()) {
                assert_eq!(t.record.is_some(), nt["has_record"].as_bool().unwrap(), "{what} record");
            }
            // (2) writer
            let native = hex(round["bytes"].as_str().unwrap());
            let bytes = write_metagame(&rp.state, &data, &ids).unwrap();
            assert!(bytes == native, "{what}: writer bytes differ (len {} vs {})", bytes.len(), native.len());
            // (3) reader
            assert_eq!(round["load"]["result"], "ok");
            let parsed = parse_metagame(&native, &data, &reg).unwrap();
            assert_eq!(parsed.consumed, u(&round["load"]["consumed"]), "{what} consumed");
            assert_same(&dump(&parsed.image, &data), &strip(&round["loaded"]), &format!("{what} loaded"));
            for nt in round["loaded"]["territories"].as_array().unwrap() {
                assert!(!nt["has_record"].as_bool().unwrap());
            }
            let c = &data.campaigns[ci];
            let written: Vec<SavedEventState> = rp
                .state
                .events
                .iter()
                .map(|e| SavedEventState {
                    event: c.events[e.event].ids.clone(),
                    flag: u8::from(e.triggered),
                    target: e.kingdom.map(|k| c.kingdoms[k].ids.clone()),
                })
                .collect();
            assert_eq!(parsed.discarded_events, written, "{what} events are read and dropped");
            let back = parsed.into_state(&ids, rp.state.armies.clone(), names(&row["identities"])).unwrap();
            assert!(back.state.territories.iter().all(|t| t.record.is_none()));
            assert!(back.state.events.is_empty());
            // (4) the loaded state writes what the native loaded state writes
            let again = write_metagame(&back.state, &data, &ids).unwrap();
            let expect = match round["resave_identical"].as_bool().unwrap() {
                true => native.clone(),
                false => hex(round["resave_bytes"].as_str().unwrap()),
            };
            assert!(again == expect, "{what}: re-save differs");
            // army component records
            for comp in round["components"].as_array().unwrap() {
                let a = u(&comp["army"]);
                let registered = comp["registered"].as_bool().unwrap();
                assert_eq!(registered, rp.registered[a], "{what} army {a} registration");
                let id = if registered { comp["session_counter"].as_i64().unwrap() as i32 } else { -1 };
                let army = rp.state.army(ArmyId(a as u32));
                let img = ArmyComponentImage::from_army(army, id);
                assert_eq!(component_dump(&img, &data), comp["source"], "{what} army {a} component");
                let native = hex(comp["bytes"].as_str().unwrap());
                assert!(img.write(&data).unwrap() == native, "{what} army {a}: component bytes differ");
                let load = &comp["load"];
                assert_eq!(load["result"], "ok");
                let (parsed, consumed) = ArmyComponentImage::parse(&native, &data, rp.state.territories.len()).unwrap();
                assert_eq!(consumed, u(&load["consumed"]));
                assert_eq!(component_dump(&parsed, &data), load["component"], "{what} army {a} component load");
                assert_eq!(parsed.updatable_id, Some(id));
                let slots: Vec<i64> = load["registered_slots"].as_array().unwrap().iter().map(|v| v.as_i64().unwrap()).collect();
                assert_eq!(slots, if id >= 0 { vec![i64::from(id)] } else { vec![] });
                let mut copy = army.clone();
                parsed.apply_to(&mut copy).unwrap();
                assert_eq!(&copy, army);
                components += 1;
            }
            saves += 1;
        }
    }
    assert!(saves >= 35 && components > 2000, "{saves} saves, {components} components");
}

fn check_parse(native: &Value, bytes: &[u8], data: &MetaGameData, reg: &SaveRegistries, what: &str) -> Option<ParsedMetagame> {
    let res = parse_metagame(bytes, data, reg);
    match native["native"]["result"].as_str().unwrap() {
        "throw" => {
            let msg = native["native"]["message"].as_str().unwrap();
            match res {
                Err(Error::NativeData(m)) => assert_eq!(m, msg, "{what}"),
                other => panic!("{what}: expected native error {msg:?}, got {other:?}"),
            }
            None
        }
        "ok" => {
            let parsed = res.unwrap_or_else(|e| panic!("{what}: {e}"));
            assert_eq!(parsed.consumed, u(&native["native"]["consumed"]), "{what} consumed");
            assert_same(&dump(&parsed.image, data), &strip(&native["loaded"]), what);
            Some(parsed)
        }
        other => panic!("{what}: native result {other}"),
    }
}

#[test]
fn metagame_reader_edge_cases_match_native() {
    let (base, fx) = load_fixture();
    let rows = fx["campaigns"].as_array().unwrap();
    let mut n = 0;
    for e in fx["error_cases"].as_array().unwrap() {
        let ci = u(&e["campaign"]);
        let row = &rows[ci];
        let data = with_events(&base, ci, &strs(&row["registries"]["events"]));
        let reg = registries(&row["registries"]);
        let ids = identities(&row["identities"]);
        let bytes0 = hex(row["rounds"][u(&e["base_round"])]["bytes"].as_str().unwrap());
        for case in e["cases"].as_array().unwrap() {
            let p = &case["patch"];
            let o = u(&p["offset"]);
            let mut bytes = bytes0[..o].to_vec();
            bytes.extend(hex(p["insert"].as_str().unwrap()));
            bytes.extend_from_slice(&bytes0[o + u(&p["remove"])..]);
            let what = format!("campaign {ci} {}", case["case"]);
            if let Some(parsed) = check_parse(case, &bytes, &data, &reg, &what) {
                let rel = teams(&data, ci);
                let arena = MgState::init_create(&data, ci, |i| Some(KingdomActor(i as u32)), &rel).unwrap().armies;
                let loaded = parsed.into_state(&ids, arena, names(&row["identities"])).unwrap_or_else(|e| panic!("{what}: {e}"));
                if case["case"] == "current_case_swapped" {
                    assert_eq!(loaded.state.current, None);
                }
            }
            n += 1;
        }
    }
    let row0 = &rows[0];
    let data0 = with_events(&base, 0, &strs(&row0["registries"]["events"]));
    for case in fx["synthetic_cases"].as_array().unwrap() {
        let reg = registries(&row0["registries"]);
        let ids = identities(&case["identities"]);
        let bytes = hex(case["bytes"].as_str().unwrap());
        let name = case["case"].as_str().unwrap();
        let what = format!("synthetic {name}");
        if let Some(parsed) = check_parse(case, &bytes, &data0, &reg, &what) {
            // representability of the native result in MgState
            let rel = teams(&data0, 0);
            let arena = MgState::init_create(&data0, 0, |i| Some(KingdomActor(i as u32)), &rel).unwrap().armies;
            let res = parsed.into_state(&ids, arena, names(&case["identities"]));
            match name {
                "unknown_record_territory" => assert!(matches!(res, Err(Error::Unsupported(_))), "{what}: {res:?}"),
                _ => {
                    res.unwrap_or_else(|e| panic!("{what}: {e}"));
                }
            }
        }
        n += 1;
    }
    for e in fx["component_cases"].as_array().unwrap() {
        for case in e["cases"].as_array().unwrap() {
            let what = format!("campaign {} {}", e["campaign"], case["case"]);
            let bytes = hex(case["bytes"].as_str().unwrap());
            let n_t = rows[u(&e["campaign"])]["rounds"][0]["source"]["territories"].as_array().unwrap().len();
            if case["case"] == "component_null_territory_write" {
                // writer: -1 for a null territory; the reader refuses it (9389F0
                // reads territories[-1])
                let src = &case["source"];
                assert!(src["territory"].is_null());
                let id = i32::from_le_bytes(bytes[bytes.len() - 4..].try_into().unwrap());
                let div = |v: &Value| -> Option<DivisionRef> {
                    v.as_str().map(|s| DivisionRef(base.divisions.iter().position(|d| d.ids == s).unwrap() as u32))
                };
                let img = ArmyComponentImage {
                    fought: src["fought"].as_i64().unwrap() as i32,
                    territory: None,
                    path: src["path"].as_array().unwrap().iter().map(u).collect(),
                    waypoints: src["waypoints"].as_array().unwrap().iter().map(u).collect(),
                    divisions: src["divisions"].as_array().unwrap().iter().map(div).collect(),
                    updatable_id: Some(id),
                };
                assert!(img.write(&base).unwrap() == bytes, "{what}");
                assert!(matches!(ArmyComponentImage::parse(&bytes, &base, n_t), Err(Error::Unsupported(_))));
                n += 1;
                continue;
            }
            let res = ArmyComponentImage::parse(&bytes, &base, n_t);
            let native = &case["native"];
            match native["result"].as_str().unwrap() {
                "throw" => match res {
                    Err(Error::NativeData(m)) => assert_eq!(m, native["message"].as_str().unwrap(), "{what}"),
                    other => panic!("{what}: {other:?}"),
                },
                "ok" => {
                    let (img, consumed) = res.unwrap();
                    assert_eq!(consumed, u(&native["consumed"]), "{what}");
                    assert_eq!(component_dump(&img, &base), native["component"], "{what}");
                    let slots: Vec<i64> = native["registered_slots"].as_array().unwrap().iter().map(|v| v.as_i64().unwrap()).collect();
                    match img.updatable_id {
                        Some(id) if id >= 0 => assert_eq!(slots, vec![i64::from(id)], "{what}"),
                        _ => assert!(slots.is_empty(), "{what}"),
                    }
                    if case["case"] == "updatable_version_0" {
                        assert_eq!(img.updatable_id, None);
                    }
                }
                other => panic!("{what}: {other}"),
            }
            n += 1;
        }
    }
    assert!(n >= 90, "{n} cases");
}

#[test]
fn writer_refuses_native_null_dereferences_and_missing_engine_values() {
    let (base, fx) = load_fixture();
    let row = &fx["campaigns"][0];
    let data = with_events(&base, 0, &strs(&row["registries"]["events"]));
    let ids = identities(&row["identities"]);
    let rel = teams(&data, 0);
    // without the engine seams (names, visual handles) the state cannot be saved
    let bare = MgState::init_create(&data, 0, |i| Some(KingdomActor(i as u32)), &rel).unwrap();
    assert!(matches!(write_metagame(&bare, &data, &ids), Err(Error::Unsupported(_))));
    let mut state = init_world(&data, 0, row, &rel);
    assert!(write_metagame(&state, &data, &ids).is_ok());
    state.current = None;
    assert!(matches!(write_metagame(&state, &data, &ids), Err(Error::Unsupported(_))));
    state.current = Some(0);
    state.results.push(BattleRecord { territory: 0, attacker: None, defender: Some(0), attacker_won: true });
    assert!(matches!(write_metagame(&state, &data, &ids), Err(Error::Unsupported(_))));
    state.results.clear();
    state.kingdoms[0].capital = None;
    assert!(matches!(write_metagame(&state, &data, &ids), Err(Error::Unsupported(_))));
    // truncated payloads: the engine stream's end-of-chunk behaviour is not
    // established, so the reader refuses them
    state.kingdoms[0].capital = Some(0);
    let bytes = write_metagame(&state, &data, &ids).unwrap();
    let reg = registries(&row["registries"]);
    for cut in [0, 3, 10, bytes.len() / 2, bytes.len() - 1] {
        assert!(matches!(parse_metagame(&bytes[..cut], &data, &reg), Err(Error::Unsupported(_))));
    }
}
