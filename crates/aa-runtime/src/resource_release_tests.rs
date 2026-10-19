//!Replays research/development10/resource-release/native-release.json.
use super::*;
use crate::resource_objects::{Entry, Location, SourceFile};
use crate::resource_task::{Services, ThreadHandle};
use std::sync::Mutex;

fn native() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../research/development10/resource-release/native-release.json")).unwrap()
}
fn kind(name: &str) -> Kind {
    match name {
        "Texture" => Kind::Texture,
        "Model" => Kind::Model,
        "Sequence" => Kind::Sequence,
        "Sound" => Kind::Sound,
        other => panic!("{other}"),
    }
}
fn u(v: &serde_json::Value) -> u32 {
    v.as_u64().unwrap_or(0) as u32
}
fn name_of(row: &serde_json::Value, index: usize) -> String {
    row["name"].as_str().map(str::to_owned).unwrap_or_else(|| format!("{}{index}", row["class"].as_str().unwrap()))
}
fn source(name: &str) -> SourceFile {
    SourceFile::from_catalog(&aa_rules::sources::CatalogEntry {
        logical_path: name.into(),
        size: 0,
        origin: aa_rules::sources::Origin::OriginalArchive { member_index: 0 },
    })
}
fn entry(origin: &Arc<()>, priority: &crate::resource_task::DefaultPriority, row: &serde_json::Value, index: usize) -> Entry {
    let name = name_of(row, index);
    let ids: Vec<u16> = name.encode_utf16().collect();
    let mut e = match kind(row["class"].as_str().unwrap()) {
        Kind::Texture => Entry::texture(origin, ids, source(&name), priority),
        Kind::Model => Entry::model(origin, ids, source(&name), priority),
        Kind::Sequence => Entry::sequence(origin, ids, source(&name), priority),
        Kind::Sound => Entry::sound(origin, ids, source(&name), priority),
        Kind::TerrainBlock => unreachable!("the release fixture has no dynamic rows"),
    }
    .unwrap();
    e.flags8 = u(&row["flags"]);
    e.requests4 = u(&row["count"]);
    e.memory0c = u(&row["ram"]);
    e.memory10 = u(&row["vram"]);
    e
}
///Native events that the port represents by host ownership/queue state or
///that only exist under a removed budget/absent B401AC are not backend calls.
fn native_events(events: &serde_json::Value) -> Vec<String> {
    events
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| {
            let a = e.as_array().unwrap();
            match a[0].as_str().unwrap() {
                "lock" | "unlock" if a[1] == "cache" => None,
                "lock" | "unlock" => Some(format!("{}:{}", a[0].as_str().unwrap(), a[1].as_str().unwrap())),
                "caching54" => Some("caching54".into()),
                "detach_73d4b0" => Some("detach_73d4b0".into()),
                "log" => Some(format!(
                    "log{}:{:#x}:{}",
                    a[1],
                    a[4].as_u64().unwrap(),
                    a[7..].iter().map(|v| v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string())).collect::<Vec<_>>().join("|")
                )),
                "remove_queue_5ae0e0" | "ni_entry_teardown_59dab0" | "pool_free_85c2b0" | "deferred_59d430" | "deferred_5a75d0"
                | "other_thread_loaded" | "sleep_6fb190" => None,
                other => panic!("unexpected native event {other}"),
            }
        })
        .collect()
}

#[test]
fn release_log_constants_are_the_native_call_arguments() {
    let native = native();
    let mut seen = 0;
    for row in native["cases"].as_array().unwrap() {
        for e in row["output"]["events"].as_array().unwrap() {
            if e[0] != "log" || e[4] == 0x636 || e[4] == 0x253 {
                continue;
            }
            let log = match e[4].as_u64().unwrap() {
                0x656 => ReleaseLog::AbortRemovedFromQueue { count: 0 },
                0x661 => ReleaseLog::AbortAlreadyPopped { count: 0 },
                0x667 => ReleaseLog::AbortNotRun { count: 0 },
                0x672 => ReleaseLog::AbortTooLate { count: 0 },
                0x69b => ReleaseLog::Unloading,
                0x523 => ReleaseLog::Unloaded,
                0x32c => ReleaseLog::NiUnloading { count: 0 },
                0x268 => ReleaseLog::TextureUnloading,
                other => panic!("{other:#x}"),
            };
            let n = log.native();
            assert_eq!((u(&e[1]), e[2].as_str().unwrap(), e[3].as_str().unwrap(), e[5].as_str().unwrap(), e[6].as_str().unwrap()),
                (n.level, format!("{:#x}", n.channel).as_str(), n.file, n.function, n.format), "{e}");
            seen += 1;
        }
    }
    assert!(seen > 50);
}

#[test]
fn cache_admission_and_removal_accounting_match_native() {
    let native = native();
    let origin = Arc::new(());
    let priority = crate::resource_task::DefaultPriority::from_fresh_application(&origin);
    let mut checked = 0;
    for row in native["cases"].as_array().unwrap() {
        let input = &row["input"];
        let rerequest = input["op"] == "rerequest";
        //Native rows admitted through an unbounded class cap (no eviction).
        let events = row["output"]["events"].as_array().unwrap();
        let admitted = input["op"] == "release"
            && events.iter().any(|e| e[0] == "caching54")
            && !events.iter().any(|e| e[0] == "log" && (e[4] == 0x69b || e[4] == 0x253));
        if !(rerequest || admitted) || input.get("hook_loaded").is_some() {
            continue;
        }
        let mut entries: Vec<Entry> = input["resources"].as_array().unwrap().iter().enumerate().map(|(i, r)| entry(&origin, &priority, r, i)).collect();
        let k = kind(input["resources"][0]["class"].as_str().unwrap());
        let mut cache = super::super::cache::State::default();
        if let Some(c) = input["classes"].get(input["resources"][0]["class"].as_str().unwrap()) {
            let class = cache.class_mut_for_test(k);
            class.ram_used0c = u(&c["ram_used"]);
            class.ram_peak10 = u(&c["ram_peak"]);
            class.vram_used1c = u(&c["vram_used"]);
            class.vram_peak20 = u(&c["vram_peak"]);
        }
        if let Some(lru) = input["lru"].get(input["resources"][0]["class"].as_str().unwrap()) {
            for i in lru.as_array().unwrap() {
                let h = entries[i.as_u64().unwrap() as usize].handle.clone();
                cache.class_mut_for_test(k).lru.push(h);
            }
        }
        cache.ram_used30 = u(&input["ram_total"]);
        cache.vram_used3c = u(&input["vram_total"]);
        if rerequest {
            cache.remove_59bfa0(&mut entries[0]).unwrap();
        } else {
            assert!(cache.admit_59bf30(&mut entries[0]).unwrap());
        }
        let expected = &row["output"]["state"];
        let class = cache.class(k).unwrap();
        let ce = &expected["classes"][input["resources"][0]["class"].as_str().unwrap()];
        assert_eq!([class.ram_used0c, class.ram_peak10, class.vram_used1c, class.vram_peak20],
            [u(&ce["ram_used"]), u(&ce["ram_peak"]), u(&ce["vram_used"]), u(&ce["vram_peak"])], "{row}");
        let lru: Vec<u64> = class.lru.iter().map(|h| entries.iter().position(|e| e.handle.same_identity(h)).unwrap() as u64).collect();
        assert_eq!(serde_json::json!(lru), ce["lru"], "{row}");
        assert_eq!(serde_json::json!([cache.ram_used30, cache.vram_used3c]), expected["totals"], "{row}");
        assert_eq!(entries[0].flags8, u(&expected["resources"][0]["flags"]), "{row}");
        checked += 1;
    }
    assert!(checked >= 3 * 10, "{checked}");
}

struct LabOs(u32);
impl crate::execution_context::OsThreadIds for LabOs {
    fn current_thread_id(&mut self) -> Result<u32> {
        Ok(self.0)
    }
}
///Recording backend over the SAME manager (shared Mutex loan) so a case can
///model the other thread finishing the load at the native hook address.
struct Lab<'m> {
    manager: &'m Mutex<super::super::Manager>,
    events: Vec<String>,
    caching: u8,
    current: Arc<crate::execution_context::ApplicationThread>,
    hook: Option<(&'static str, String)>,
}
impl Lab<'_> {
    fn set_loaded(&mut self, at: &str) {
        if let Some((point, key)) = &self.hook {
            if *point == at {
                let mut m = self.manager.lock().unwrap();
                m.resources.get_mut(key).unwrap().flags8 |= 1;
            }
        }
    }
}
impl Services for Lab<'_> {
    fn current_thread(&mut self) -> Result<ThreadHandle> {
        ThreadHandle::on_application(&self.current, &mut LabOs(0x1234))
    }
    fn enter_request(&mut self, _: &Handle) -> Result<()> {
        self.events.push("lock:20".into());
        Ok(())
    }
    fn leave_request(&mut self, _: &Handle) -> Result<()> {
        self.events.push("unlock:20".into());
        Ok(())
    }
    fn removing_from_queue_diagnostic(&mut self, h: &Handle) -> Result<()> {
        self.events.push(format!("log20:0x636:{}", String::from_utf16_lossy(h.logical_ids())));
        Ok(())
    }
    fn enter_manager_5abb30(&mut self, _: &Handle) -> Result<()> {
        self.events.extend(["lock:04".into(), "lock:20".into(), "lock:3c".into()]);
        Ok(())
    }
    fn enter_load(&mut self, _: &Handle) -> Result<()> {
        self.events.push("lock:load".into());
        Ok(())
    }
    fn leave_load(&mut self, _: &Handle) -> Result<()> {
        self.events.push("unlock:load".into());
        Ok(())
    }
    fn loaded_diagnostic(&mut self, h: &Handle) -> Result<()> {
        self.events.push(format!("loaded:{}", String::from_utf16_lossy(h.logical_ids())));
        Ok(())
    }
    fn request_diagnostic(&mut self, h: &Handle, immediate: bool) -> Result<()> {
        self.request_events(h, immediate);
        Ok(())
    }
    fn leave_manager_5abb50(&mut self, _: &Handle) -> Result<()> {
        self.events.extend(["unlock:3c".into(), "unlock:20".into(), "unlock:04".into()]);
        self.set_loaded("0x5aef1a");
        Ok(())
    }
}
impl Backend for Lab<'_> {
    fn resource_caching_54(&mut self, _: &Handle) -> Result<u8> {
        self.events.push("caching54".into());
        Ok(self.caching)
    }
    fn enter_unload_3c(&mut self, _: &Handle) -> Result<()> {
        self.events.push("lock:3c".into());
        Ok(())
    }
    fn leave_unload_3c(&mut self, _: &Handle) -> Result<()> {
        self.events.push("unlock:3c".into());
        Ok(())
    }
    fn release_log(&mut self, h: &Handle, log: &ReleaseLog) -> Result<()> {
        let n = log.native();
        let name = String::from_utf16_lossy(h.logical_ids());
        let args = match log {
            ReleaseLog::AbortRemovedFromQueue { count } | ReleaseLog::AbortAlreadyPopped { count } | ReleaseLog::AbortNotRun { count } | ReleaseLog::AbortTooLate { count } => format!("{name}|{count}"),
            ReleaseLog::NiUnloading { count } => format!("Main|{name}|{count}"),
            ReleaseLog::Unloading | ReleaseLog::Unloaded | ReleaseLog::TextureUnloading => name,
        };
        self.events.push(format!("log{}:{:#x}:{args}", n.level, n.line));
        if matches!(log, ReleaseLog::AbortAlreadyPopped { .. }) {
            self.set_loaded("0x5aeedf");
        }
        Ok(())
    }
    fn detach_model_root_73d4b0(&mut self, _: &Handle) -> Result<()> {
        self.events.push("detach_73d4b0".into());
        Ok(())
    }
    fn terrain_unload_71b5f0(&mut self, _: &Handle) -> Result<()> {
        self.events.push("vf54_71b5f0".into());
        Ok(())
    }
}
impl Lab<'_> {
    fn request_events(&mut self, h: &Handle, immediate: bool) {
        self.events.push(format!("request:{}:{}", immediate, String::from_utf16_lossy(h.logical_ids())));
    }
}
#[derive(Debug)]
struct TestModel;
impl crate::resource_model_payload::Model for TestModel {
    fn source_sha256(&self) -> &str {
        "fixture"
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT for the Application source backend; Resource fields are fixture inputs"]
fn every_native_release_row_replays_through_the_same_manager() {
    let native = native();
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let files = aa_rules::sources::V14Sources::open(root.join("Data.rwd"), root.join("reconstruction/native-game/runtime-expanded-v14"),
        aa_rules::sources::SourceLimits { max_member_bytes: u64::MAX }).unwrap();
    let mut app = super::super::Application::from_sources(files);
    let work = std::env::temp_dir().join(format!("aa-resource-release-{}", std::process::id()));
    std::fs::create_dir_all(&work).unwrap();
    app.configure_portable_work_depot(&work).unwrap();
    let current = Arc::new(crate::execution_context::ApplicationThread::capture(&mut LabOs(0x1234)).unwrap());
    let other = Arc::new(crate::execution_context::ApplicationThread::capture(&mut LabOs(0x1235)).unwrap());
    let (mut replayed, mut refused) = (0, 0);
    for row in native["cases"].as_array().unwrap() {
        let input = &row["input"];
        let op = input["op"].as_str().unwrap();
        let events = row["output"]["events"].as_array().unwrap();
        //Removed-cap rows (59BC70 eviction/59DA00) and the absent-B401AC
        //renderer path are native evidence only, checked by the other tests.
        if events.iter().any(|e| e[0] == "out_of_memory_59da00" || (e[0] == "log" && e[4] == 0x253)) || input["deferred"] == false {
            continue;
        }
        //Fresh same-Application manager state per row.
        {
            let mut m = app.manager.lock().unwrap();
            m.resources.clear();
            m.cache = super::super::cache::State::default();
            m.ni_cache = crate::resource_ni_cache::State::from_fresh_application(&app.origin);
        }
        let work_id = app.manager.lock().unwrap().work.as_ref().unwrap().id.clone();
        let rows = input["resources"].as_array().unwrap();
        let mut handles = Vec::new();
        let mut keys = Vec::new();
        for (i, r) in rows.iter().enumerate() {
            let mut e = entry(&app.origin, &app.resource_priority, r, i);
            let key = crate::resource_map_cache::normalized_key(e.handle.logical_ids()).unwrap();
            let h = e.handle.clone();
            let mut m = app.manager.lock().unwrap();
            if r["ni3c"] == true {
                let cached = m.ni_cache.append(&h, None, 0).unwrap();
                if let Some(model) = &mut e.model { model.source3c = Some(cached) } else if let Some(s) = &mut e.sequence { s.source3c = Some(cached) }
            }
            if r["root40"] == true {
                let payload = crate::resource_model_payload::Input::new(h.clone(), h.source_file().unwrap().clone(), vec![], "fixture".into(), None)
                    .prepare_with(|_, _| Ok(Box::new(TestModel))).unwrap();
                if let Some(model) = &mut e.model { model.source40 = Some(payload) } else if let Some(s) = &mut e.sequence { s.source40 = Some(()) }
            }
            if let Some(t) = &mut e.texture {
                if r["source3c"] == true {
                    let id = m.texture_objects.source_from_name(&h).unwrap();
                    m.texture_objects.retain_source(id).unwrap();
                    t.source3c = Some(m.texture_objects.reference(id).unwrap());
                }
                if r["child40"] == true {
                    t.writable_child40 = Some(h.clone());
                }
            }
            match r.get("location_writable").and_then(|v| v.as_bool()) {
                Some(true) => e.locations.push(Location::Work(work_id.clone())),
                Some(false) => e.locations.push(Location::Physical(source(&name_of(r, i)))),
                None => {}
            }
            let state = u(&r["state"]);
            if state != 0 {
                let thread = if u(&r["task_thread"]) == 0x1234 { &current } else { &other };
                let id = thread.id();
                e.task.set_state_5b18c0(1, || ThreadHandle::on_application(thread, &mut LabOs(id))).unwrap();
                if state != 1 {
                    e.task.set_state_5b18c0(state, || unreachable!()).unwrap();
                }
            }
            m.resources.insert(key.clone(), e);
            handles.push(h);
            keys.push(key);
        }
        {
            let mut m = app.manager.lock().unwrap();
            for (class, c) in input["classes"].as_object().into_iter().flatten() {
                let a = m.cache.class_mut_for_test(kind(class));
                a.ram_used0c = u(&c["ram_used"]);
                a.ram_peak10 = u(&c["ram_peak"]);
                a.vram_used1c = u(&c["vram_used"]);
                a.vram_peak20 = u(&c["vram_peak"]);
            }
            for (class, lru) in input["lru"].as_object().into_iter().flatten() {
                for i in lru.as_array().unwrap() {
                    let h = handles[i.as_u64().unwrap() as usize].clone();
                    m.cache.class_mut_for_test(kind(class)).lru.push(h);
                }
            }
            m.cache.ram_used30 = u(&input["ram_total"]);
            m.cache.vram_used3c = u(&input["vram_total"]);
        }
        if input["in_queue"] == true {
            let task = app.manager.lock().unwrap().resources[&keys[0]].task.clone();
            app.resource_tasks.enqueue_5aed10(&task).unwrap();
        }
        let hook = input["hook_loaded"].as_object().and_then(|h| h.keys().next().cloned()).map(|at| {
            (if at == "0x5aeedf" { "0x5aeedf" } else { "0x5aef1a" }, keys[0].clone())
        });
        let mut lab = Lab { manager: &app.manager, events: vec![], caching: input.get("caching").map(u).unwrap_or(1) as u8, current: Arc::clone(&current), hook };
        let mut resources = Resources {
            origin: &app.origin, sources: &mut app.sources, maps: &mut app.maps, manager: &app.manager,
            priority: &app.resource_priority, tasks: &app.resource_tasks, cache_variables: &app.cache_variables,
            model_source_context: &app.model_source_context,
        };
        let h = &handles[0];
        let result = match op {
            "release" => resources.release_14(h, u(&input["delta"]), &mut lab),
            "rerequest" => {
                let task = resources.with_entry(h, |e| Ok(Arc::clone(&e.task))).unwrap();
                let tasks = resources.tasks;
                task.request_5b0270(tasks, u(&input["mode"]).max(1), &mut super::super::ResourceCallbacks { resources: &mut resources, backend: &mut lab })
            }
            "unload" => resources.unload_5abff0(h, &mut lab),
            "vf40" => resources.unload_40(h, &mut lab),
            "vf54" => resources.unload_54(h, &mut lab),
            other => panic!("{other}"),
        };
        let native_seq = native_events(&row["output"]["events"]);
        if events.iter().any(|e| e[0] == "sleep_6fb190") {
            //Cross-thread status1 abort: native sleeps until the other
            //thread's dispatcher stores 5; the port refuses explicitly.
            assert!(result.unwrap_err().contains("cross-thread"), "{row}");
            assert!(native_seq.starts_with(&lab.events), "{row}\n{:?}", lab.events);
            refused += 1;
            continue;
        }
        result.unwrap_or_else(|e| panic!("{row}: {e}"));
        assert_eq!(lab.events, native_seq, "{row}");
        let expected = &row["output"]["state"];
        let m = app.manager.lock().unwrap();
        for (i, key) in keys.iter().enumerate() {
            let e = &m.resources[key];
            let x = &expected["resources"][i];
            assert_eq!((e.requests4, e.flags8), (u(&x["count"]), u(&x["flags"])), "{row} resource {i}");
            let field3c = e.model.as_ref().map(|m| m.source3c.is_some()).or(e.sequence.as_ref().map(|s| s.source3c.is_some())).or(e.texture.as_ref().map(|t| t.source3c.is_some())).unwrap_or(false);
            let field40 = e.model.as_ref().map(|m| m.source40.is_some()).or(e.sequence.as_ref().map(|s| s.source40.is_some())).or(e.texture.as_ref().map(|t| t.writable_child40.is_some())).unwrap_or(false);
            assert_eq!((field3c, field40), (x["field3c"].as_bool().unwrap(), x["field40"].as_bool().unwrap()), "{row} resource {i}");
            assert_eq!(e.task.state14().unwrap(), u(&x["task_state"]), "{row}");
        }
        assert_eq!(serde_json::json!([m.cache.ram_used30, m.cache.vram_used3c]), expected["totals"], "{row}");
        for (class, x) in expected["classes"].as_object().unwrap() {
            let Some(c) = m.cache.class(kind(class)) else {
                assert_eq!(x["lru"], serde_json::json!([]), "{row}");
                continue;
            };
            assert_eq!([c.ram_used0c, c.ram_peak10, c.vram_used1c, c.vram_peak20],
                [u(&x["ram_used"]), u(&x["ram_peak"]), u(&x["vram_used"]), u(&x["vram_peak"])], "{row}");
            let lru: Vec<usize> = c.lru.iter().map(|h| handles.iter().position(|v| v.same_identity(h)).unwrap()).collect();
            assert_eq!(serde_json::json!(lru), x["lru"], "{row}");
        }
        //A removed Ni cache entry is unlinked from the SAME list.
        let linked = m.ni_cache.linked();
        for (i, h) in handles.iter().enumerate() {
            if rows[i]["ni3c"] == true {
                assert_eq!(linked.iter().any(|v| v.same_identity(h)), expected["resources"][i]["field3c"].as_bool().unwrap(), "{row}");
            }
        }
        if input["in_queue"] == true {
            assert!(!m.resources[&keys[0]].task.queued10().unwrap());
        }
        replayed += 1;
    }
    println!("replayed {replayed} native release rows, {refused} explicit cross-thread refusals");
    assert!(replayed > 100 && refused == 3, "{replayed} {refused}");
}

#[test]
fn absent_deferred_owner_and_removed_cap_rows_are_documented_native_evidence() {
    let native = native();
    let rows = native["cases"].as_array().unwrap();
    //B401AC==0 (5A5030 / renderer+18 refcount path) leaves exactly the same
    //Resource fields as the live deferred owner the process constructs.
    let mut pairs = 0;
    for row in rows.iter().filter(|r| r["input"]["deferred"] == false) {
        let mut twin = row["input"].clone();
        twin["deferred"] = serde_json::json!(true);
        let other = rows.iter().find(|r| r["input"] == twin && r["precision"] == row["precision"]).unwrap();
        assert_eq!(row["output"]["state"], other["output"]["state"], "{row}");
        pairs += 1;
    }
    assert_eq!(pairs, 9 * 3);
    //Finite 59C040 caps: native evicts the LRU head (0x253 "Decaching") or
    //refuses through 59DA00 and unloads at once. The port's unbounded cache
    //admits instead (removed cap); every other field rule is unchanged.
    let origin = Arc::new(());
    let priority = crate::resource_task::DefaultPriority::from_fresh_application(&origin);
    let mut capped = 0;
    for row in rows {
        let events = row["output"]["events"].as_array().unwrap();
        if !events.iter().any(|e| e[0] == "out_of_memory_59da00" || (e[0] == "log" && e[4] == 0x253)) {
            continue;
        }
        let input = &row["input"];
        let mut entries: Vec<Entry> = input["resources"].as_array().unwrap().iter().enumerate().map(|(i, r)| entry(&origin, &priority, r, i)).collect();
        let mut cache = super::super::cache::State::default();
        for i in input["lru"]["Model"].as_array().into_iter().flatten() {
            let h = entries[i.as_u64().unwrap() as usize].handle.clone();
            cache.class_mut_for_test(Kind::Model).lru.push(h);
        }
        assert!(cache.admit_59bf30(&mut entries[0]).unwrap());
        assert_eq!(cache.class(Kind::Model).unwrap().lru.len(), input["lru"]["Model"].as_array().map_or(0, |v| v.len()) + 1);
        capped += 1;
    }
    assert_eq!(capped, 3 * 3);
}

///Request (5AA3F0) -> release (5AA550) -> re-request (5B0270) round trips over
///the SAME manager, queue and cache, using the ported request side unchanged.
#[test]
#[ignore = "requires AA_GAME_ROOT for the Application source backend; Resource fields are fixture inputs"]
fn request_release_and_cached_rerequest_round_trip() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let files = aa_rules::sources::V14Sources::open(root.join("Data.rwd"), root.join("reconstruction/native-game/runtime-expanded-v14"),
        aa_rules::sources::SourceLimits { max_member_bytes: u64::MAX }).unwrap();
    let mut app = super::super::Application::from_sources(files);
    let current = Arc::new(crate::execution_context::ApplicationThread::capture(&mut LabOs(0x1234)).unwrap());
    let row = serde_json::json!({"class": "Model", "flags": 0x400, "count": 0, "ram": 0x40, "vram": 0x20, "name": "Units/RoundTrip.nif"});
    let e = entry(&app.origin, &app.resource_priority, &row, 0);
    let key = crate::resource_map_cache::normalized_key(e.handle.logical_ids()).unwrap();
    let h = e.handle.clone();
    app.manager.lock().unwrap().resources.insert(key.clone(), e);
    let mut lab = Lab { manager: &app.manager, events: vec![], caching: 1, current: Arc::clone(&current), hook: None };
    let mut resources = Resources {
        origin: &app.origin, sources: &mut app.sources, maps: &mut app.maps, manager: &app.manager,
        priority: &app.resource_priority, tasks: &app.resource_tasks, cache_variables: &app.cache_variables,
        model_source_context: &app.model_source_context,
    };
    //Unloaded and queued: the release removes the queued task, nothing cached.
    resources.request_0c(&h, 1, 1, &mut lab).unwrap();
    assert_eq!(resources.tasks.len().unwrap(), 1);
    lab.events.clear();
    resources.release_14(&h, 1, &mut lab).unwrap();
    assert_eq!(lab.events, ["lock:20", "log20:0x636:Units/RoundTrip.nif", "unlock:20", "log10:0x656:Units/RoundTrip.nif|0"]);
    assert_eq!(resources.tasks.len().unwrap(), 0);
    assert_eq!(resources.requests_04(&h).unwrap(), 0);
    //Requested and loaded (count1, bit0|4|6 as the task body stores them):
    //the release admits it to the cache.
    resources.with_entry(&h, |e| { e.flags8 |= 7; e.requests4 = 1; Ok(()) }).unwrap();
    lab.events.clear();
    resources.release_14(&h, 1, &mut lab).unwrap();
    assert_eq!(lab.events, ["lock:20", "caching54", "unlock:20"]);
    assert_eq!(resources.flags_08(&h).unwrap(), 0x427);
    {
        let m = app.manager.lock().unwrap();
        let c = m.cache.class(Kind::Model).unwrap();
        assert_eq!((c.lru.len(), c.ram_used0c, c.vram_used1c, m.cache.ram_used30), (1, 0x40, 0x20, 0x40));
    }
    //Re-request: 5AA3F0 skips the double-request test (bit5), 5B0270 takes
    //the cached branch: 59BFA0 + vf2C, no queue node, bit0 still set.
    lab.events.clear();
    resources.request_0c(&h, 1, 1, &mut lab).unwrap();
    assert_eq!(lab.events, ["lock:20", "unlock:20", "lock:20", "unlock:20"]);
    assert_eq!((resources.flags_08(&h).unwrap(), resources.requests_04(&h).unwrap(), resources.tasks.len().unwrap()), (0x407, 1, 0));
    let m = app.manager.lock().unwrap();
    let c = m.cache.class(Kind::Model).unwrap();
    assert_eq!((c.lru.len(), c.ram_used0c, c.ram_peak10, m.cache.ram_used30), (0, 0, 0x40, 0));
}

fn terrain() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../research/development10/resource-terrain-block/native-terrain-block.json")).unwrap()
}
///723D30/723960/5ACEC0 facts: class chain, 4F0500, memory words and flags.
#[test]
fn terrain_block_constructor_class_and_memory_match_native_all_pc() {
    let native = terrain();
    assert_eq!(native["format"], "Dynamic/Terrain x%d y%d");
    assert_eq!(native["class_name"], "KKC_RTerrainBlockResource");
    assert_eq!(native["vtable"][19], "0x4f0500");
    assert_eq!(native["vtable"][21], "0x71b5f0");
    assert_eq!(native["vtable"][16], "0x5aa700");
    assert_eq!(native["vtable"][15], "0x5aa6f0");
    let origin = Arc::new(());
    let priority = crate::resource_task::DefaultPriority::from_fresh_application(&origin);
    for row in native["rows"].as_array().unwrap() {
        let out = &row["output"];
        let chain: Vec<u32> = out["class_"]["vf4_chain"].as_array().unwrap().iter().map(|v| u32::from_str_radix(v.as_str().unwrap().trim_start_matches("0x"), 16).unwrap()).collect();
        assert_eq!(crate::resource_model_unload::native_class_chain(Kind::TerrainBlock), chain.as_slice());
        assert_eq!(out["class_"]["vf4c"], 1);
        for (n, m) in [17u32, 33, 0, 0x10000].iter().zip(out["memory"].as_array().unwrap()) {
            let e = Entry::terrain_block(&origin, 3, 7, *n, &priority).unwrap();
            assert_eq!(serde_json::json!([e.memory0c, e.memory10]), m["memory"]);
            assert_eq!(serde_json::json!(e.flags8), m["flags"]);
            assert_eq!(String::from_utf16_lossy(e.handle.logical_ids()), "Dynamic/Terrain x3 y7");
            assert!(e.handle.source_file().is_none() && e.requests4 == 0 && e.references14 == 0);
            assert_eq!(e.task.references4().unwrap(), 1);
        }
        //5ACEC0: bit4 inserts by name, 59CB00 always, both under +20.
        assert_eq!(out["publish"][0], serde_json::json!([["lock20"], ["5AC1C0_insert", 5], ["59CB00_class", 156], ["unlock20"]]));
        assert_eq!(out["publish"][1], serde_json::json!([["lock20"], ["59CB00_class", 156], ["unlock20"]]));
    }
}
///Registration, the immediate (mode 4) request through the generic
///5AA3F0/5B0270/5B1910 path and the release through 5AA550/5AEE70/5AC3D0
///(bit3: never cached) on the SAME manager.
#[test]
#[ignore = "requires AA_GAME_ROOT for the Application source backend"]
fn terrain_block_registration_request_and_release_round_trip() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let files = aa_rules::sources::V14Sources::open(root.join("Data.rwd"), root.join("reconstruction/native-game/runtime-expanded-v14"),
        aa_rules::sources::SourceLimits { max_member_bytes: u64::MAX }).unwrap();
    let mut app = super::super::Application::from_sources(files);
    let current = Arc::new(crate::execution_context::ApplicationThread::capture(&mut LabOs(0x1234)).unwrap());
    let mut lab = Lab { manager: &app.manager, events: vec![], caching: 1, current: Arc::clone(&current), hook: None };
    let mut resources = Resources {
        origin: &app.origin, sources: &mut app.sources, maps: &mut app.maps, manager: &app.manager,
        priority: &app.resource_priority, tasks: &app.resource_tasks, cache_variables: &app.cache_variables,
        model_source_context: &app.model_source_context,
    };
    let h = resources.register_terrain_block_5acec0(3, 7, 17, &mut lab).unwrap();
    assert_eq!(lab.events, ["lock:20", "unlock:20"]);
    assert!(resources.register_terrain_block_5acec0(3, 7, 17, &mut lab).is_err());
    assert_eq!((resources.flags_08(&h).unwrap(), resources.requests_04(&h).unwrap()), (0x18, 0));
    lab.events.clear();
    resources.request_0c(&h, 1, 4, &mut lab).unwrap();
    assert_eq!(lab.events, ["lock:20", "unlock:20", "request:true:Dynamic/Terrain x3 y7", "lock:load", "loaded:Dynamic/Terrain x3 y7", "unlock:load"]);
    assert_eq!((resources.flags_08(&h).unwrap(), resources.requests_04(&h).unwrap()), (0x1f, 1));
    lab.events.clear();
    resources.release_14(&h, 1, &mut lab).unwrap();
    assert_eq!(lab.events, ["lock:20", "log20:0x69b:Dynamic/Terrain x3 y7", "lock:3c", "vf54_71b5f0", "log20:0x523:Dynamic/Terrain x3 y7", "unlock:3c", "unlock:20"]);
    assert_eq!((resources.flags_08(&h).unwrap(), resources.requests_04(&h).unwrap()), (0x1c, 0));
    let m = app.manager.lock().unwrap();
    assert!(m.cache.class(Kind::TerrainBlock).is_none());
}
