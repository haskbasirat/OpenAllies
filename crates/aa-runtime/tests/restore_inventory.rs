//! Diagnostic census of every original map's restoration stops. This uses
//! `RestoreMode::InspectPartial`, which continues after a failed record, so
//! later failures may be cascades of earlier partial state. Never publishable.
use aa_mapworld::restore::{RestoreMode, RestoreStage};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use aa_runtime::{
    localization::Localization,
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BindingContext {
    Standalone,
    Resources,
    Managed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Config {
    context: BindingContext,
    requested_mode: Option<u32>,
    elapsed_fixture: Option<u32>,
}
impl Config {
    fn parse(
        context: Option<&str>,
        mode: Option<&str>,
        elapsed: Option<&str>,
    ) -> Result<Self, String> {
        let context = match context {
            None | Some("standalone") => BindingContext::Standalone,
            Some("resources") => BindingContext::Resources,
            Some("managed") => BindingContext::Managed,
            Some(value) => {
                return Err(format!(
                    "invalid AA_RESTORE_CONTEXT {value:?}; use standalone, resources or managed"
                ));
            }
        };
        let number = |name, value: Option<&str>| {
            value
                .map(|value| {
                    value
                        .parse::<u32>()
                        .map_err(|_| format!("{name} must be a decimal u32"))
                })
                .transpose()
        };
        let requested_mode = number("AA_RESTORE_NATIVE_LOAD_MODE", mode)?;
        let elapsed_fixture = number("AA_RESTORE_ELAPSED_MILLIS", elapsed)?;
        if context == BindingContext::Managed {
            if requested_mode.is_some_and(|mode| mode != TaskKind::Normal.native_load_mode()) {
                return Err("managed normal-task context conflicts with AA_RESTORE_NATIVE_LOAD_MODE (must be1 or omitted)".into());
            }
            if elapsed_fixture.is_none() {
                return Err(
                    "managed diagnostic requires explicit AA_RESTORE_ELAPSED_MILLIS clock fixture"
                        .into(),
                );
            }
        } else if elapsed_fixture.is_some() {
            return Err("AA_RESTORE_ELAPSED_MILLIS is only meaningful in managed context".into());
        }
        Ok(Self {
            context,
            requested_mode,
            elapsed_fixture,
        })
    }
    fn from_env() -> Self {
        let read = |name| match std::env::var(name) {
            Ok(value) => Some(value),
            Err(std::env::VarError::NotPresent) => None,
            Err(error) => panic!("invalid {name}: {error}"),
        };
        Self::parse(
            read("AA_RESTORE_CONTEXT").as_deref(),
            read("AA_RESTORE_NATIVE_LOAD_MODE").as_deref(),
            read("AA_RESTORE_ELAPSED_MILLIS").as_deref(),
        )
        .unwrap()
    }
    fn mode(self) -> Option<u32> {
        if self.context == BindingContext::Managed {
            Some(TaskKind::Normal.native_load_mode())
        } else {
            self.requested_mode
        }
    }
    fn label(self) -> &'static str {
        match self.context {
            BindingContext::Standalone => "standalone",
            BindingContext::Resources => "resources",
            BindingContext::Managed => "managed",
        }
    }
    fn metadata(self) -> Value {
        json!({"context":self.label(),"requested_native_load_mode":self.requested_mode,
            "effective_native_load_mode":self.mode(),"elapsed_clock_fixture_millis":self.elapsed_fixture,
            "elapsed_clock_source":self.elapsed_fixture.map(|_|"AA_RESTORE_ELAPSED_MILLIS diagnostic fixture; not observed original application time"),
            "managed_scope":(self.context == BindingContext::Managed).then_some(
                "fresh stopped application per map and per StopOnError reload; not an old-world lifecycle or activation")})
    }
}
fn open_sources(game: &std::path::Path) -> V14Sources {
    V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap()
}
struct Context {
    config: Config,
    resources: Option<Arc<ResourceCatalog>>,
}
impl Context {
    fn bindings(&self, sources: &mut Option<V14Sources>) -> Result<(LoadBindings, Value), String> {
        let inactive_load = if self.config.context == BindingContext::Managed {
            Some(
                ScopedManagedLoad::from_sources(
                    sources
                        .as_mut()
                        .ok_or("managed worker source owner missing")?,
                    self.config
                        .elapsed_fixture
                        .ok_or("managed clock fixture missing")?,
                )?
                .begin(TaskKind::Normal)?,
            )
        } else {
            None
        };
        let initialization = inactive_load.as_ref().map(|load| {
            let init = load.initialization();
            json!({"uvars_path":init.uvars_path,"uvars_sha256":init.uvars_sha256,
                "elapsed_clock_fixture_millis":init.elapsed_millis,"private_seed":init.private_seed,
                "task":"Normal","fresh_application_scope":true,"active":false})
        });
        Ok((
            LoadBindings {
                native_load_mode: self.config.requested_mode,
                inactive_load,
                resources: self.resources.as_ref().map(Arc::clone),
            },
            json!(initialization),
        ))
    }
}
/// Record/PostLoad success never certifies omitted immediate Init callbacks.
fn lifecycle_coverage(records_without_failures: bool, publication_ready: bool) -> Value {
    json!({"record_sections_without_failures":records_without_failures,
        "immediate_init":"not_executed_by_this_diagnostic",
        "immediate_init_complete":false,"runtime_publication_ready":publication_ready,
        "battle_ready":false,
        "note":"record codecs and PostLoad do not supply missing immediate Init/PostCreate or world publication"})
}

fn maps(root: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            maps(&path, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("rmp"))
        {
            out.push(path);
        }
    }
}

fn utf16(v: &[u16]) -> String {
    String::from_utf16_lossy(v)
}

/// Strip per-record numbers so identical unsupported operations group together.
fn normalize(error: &str) -> String {
    let mut out = String::new();
    let mut digits = false;
    let mut chars = error.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '0' && chars.peek() == Some(&'x') {
            chars.next();
            while chars.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                chars.next();
            }
            out.push('#');
            digits = false;
            continue;
        }
        if c.is_ascii_digit() {
            if !digits {
                out.push('#');
            }
            digits = true;
        } else {
            digits = false;
            out.push(c);
        }
    }
    out
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets; diagnostic census only"]
fn all_original_maps_inspect_partial_inventory() {
    let config = Config::from_env();
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = open_sources(&game);
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let context = Context {
        config,
        resources: if config.context == BindingContext::Standalone {
            None
        } else {
            Some(Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap()))
        },
    };
    let root = game.join("reconstruction/assets/original/Data/Maps");
    let mut paths = Vec::new();
    maps(&root, &mut paths);
    assert_eq!(paths.len(), 80, "original corpus has 80 maps");
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(4);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = std::sync::Mutex::new(BTreeMap::<usize, Value>::new());
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                // Each worker owns its reader; each map/reload constructs a
                // new non-Clone capability. Neither reader nor token is shared.
                let mut managed_sources =
                    (config.context == BindingContext::Managed).then(|| open_sources(&game));
                loop {
                    let index = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some(path) = paths.get(index) else { break };
                    let value = inspect(
                        path,
                        &root,
                        &actors,
                        &settings,
                        &text,
                        &context,
                        &mut managed_sources,
                    );
                    results.lock().unwrap().insert(index, value);
                }
            });
        }
    });
    let results: Vec<Value> = results.into_inner().unwrap().into_values().collect();
    // Aggregate first-failure and all-failure groups.
    let mut groups: BTreeMap<String, Value> = BTreeMap::new();
    let mut first_stop: BTreeMap<String, Value> = BTreeMap::new();
    for map in &results {
        let name = map["map"].as_str().unwrap().to_owned();
        if let Some(error) = map["load_error"].as_str() {
            let key = format!("[map load] {}", normalize(error));
            let entry = first_stop
                .entry(key.clone())
                .or_insert_with(|| json!({"normalized_error":key,"maps":[]}));
            entry["maps"]
                .as_array_mut()
                .unwrap()
                .push(json!({"map":name}));
            continue;
        }
        let failures = map["failures"].as_array().unwrap();
        for (i, failure) in failures.iter().enumerate() {
            let key = normalize(failure["error"].as_str().unwrap());
            let entry = groups.entry(key.clone()).or_insert_with(|| {
                json!({"normalized_error":key,"count":0,"maps":[],"definitions":{},
                       "first":{"map":name,"stage":failure["stage"],"source_ordinal":failure["source_ordinal"],
                                "saved_id":failure["saved_id"],"definition":failure["definition"],
                                "error":failure["error"]}})
            });
            entry["count"] = json!(entry["count"].as_u64().unwrap() + 1);
            let maps = entry["maps"].as_array_mut().unwrap();
            if !maps.iter().any(|m| m == &json!(name)) {
                maps.push(json!(name));
            }
            if let Some(def) = failure["definition"].as_str() {
                let defs = entry["definitions"].as_object_mut().unwrap();
                let n = defs.get(def).and_then(Value::as_u64).unwrap_or(0) + 1;
                defs.insert(def.to_owned(), json!(n));
            }
            if i == 0 {
                let entry = first_stop
                    .entry(key.clone())
                    .or_insert_with(|| json!({"normalized_error":key,"maps":[]}));
                entry["maps"].as_array_mut().unwrap().push(json!({
                    "map":name,"stage":failure["stage"],"source_ordinal":failure["source_ordinal"],
                    "saved_id":failure["saved_id"],"definition":failure["definition"]}));
            }
        }
    }
    let mut groups: Vec<Value> = groups.into_values().collect();
    groups.sort_by(|a, b| b["count"].as_u64().cmp(&a["count"].as_u64()));
    let mut skipped: BTreeMap<String, u64> = BTreeMap::new();
    for map in &results {
        for record in map["skipped"].as_array().into_iter().flatten() {
            let cause = match &record["cause"] {
                Value::String(v) => v.clone(),
                Value::Object(v) => v.keys().next().cloned().unwrap_or_default(),
                _ => String::new(),
            };
            *skipped.entry(cause).or_default() += 1;
        }
    }
    let clean = results
        .iter()
        .filter(|m| m["load_error"].is_null() && m["failures"].as_array().unwrap().is_empty())
        .count();
    // Maps whose record sections all load reach GWorld::Load's queue Load and
    // PostLoad pass (StopOnError reload, then Staged::post_load).
    let mut post_load: BTreeMap<String, Value> = BTreeMap::new();
    for map in &results {
        let Some(outcome) = map.get("post_load").filter(|v| !v.is_null()) else {
            continue;
        };
        let key = match outcome["error"].as_str() {
            Some(error) => format!("error: {}", normalize(error)),
            None => "ok".to_owned(),
        };
        let entry = post_load
            .entry(key.clone())
            .or_insert_with(|| json!({"outcome":key,"maps":[]}));
        entry["maps"]
            .as_array_mut()
            .unwrap()
            .push(map["map"].clone());
    }
    let summary = json!({
        "mode": "InspectPartial",
        "native_load_mode": config.mode(),
        "binding_context": config.metadata(),
        "resource_source":context.resources.as_ref().map(|r|json!({
            "sources":r.sources,"resource_count":r.resources.len(),
            "resource_ids":r.resources.iter().map(|v|&v.id).collect::<Vec<_>>() })),
        "precision": "Double53",
        "warning": "InspectPartial continues after failures; later errors may be cascades of earlier partial state. Not publishable.",
        "map_count": results.len(),
        "maps_without_failures": clean,
        "lifecycle_coverage":lifecycle_coverage(clean == results.len(),false),
        "maps_with_completed_immediate_init":0,
        "skipped_records_by_cause": skipped,
        "converted_records": results.iter().map(|m| m["converted"].as_u64().unwrap_or(0)).sum::<u64>(),
        "post_load_by_outcome": post_load.into_values().collect::<Vec<_>>(),
        "first_stop_by_error": first_stop.into_values().collect::<Vec<_>>(),
        "groups": groups,
        "maps": results,
    });
    let text = serde_json::to_string_pretty(&summary).unwrap();
    if let Some(out) = std::env::var_os("AA_RESTORE_INVENTORY_OUT") {
        std::fs::write(out, &text).unwrap();
    } else {
        println!("{text}");
    }
}

fn inspect(
    path: &std::path::Path,
    root: &std::path::Path,
    actors: &ActorCatalog,
    settings: &Settings,
    text: &Localization,
    context: &Context,
    sources: &mut Option<V14Sources>,
) -> Value {
    let name = path
        .strip_prefix(root)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let map = match std::fs::read(path)
        .map_err(|e| e.to_string())
        .and_then(|b| aa_mapworld::MapArchive::from_bytes(b).map_err(|e| e.to_string()))
    {
        Ok(map) => map,
        Err(e) => {
            return json!({"map":name,"load_error":e,"failures":[],
            "binding_context":context.config.metadata(),"lifecycle_coverage":lifecycle_coverage(false,false)});
        }
    };
    let (bindings, initialization) = match context.bindings(sources) {
        Ok(value) => value,
        Err(error) => {
            return json!({"map":name,"load_error":format!("bindings: {error}"),"failures":[],
            "binding_context":context.config.metadata(),"lifecycle_coverage":lifecycle_coverage(false,false)});
        }
    };
    let attempt = match Attempt::load_with_bindings(
        &map,
        actors,
        settings.clone(),
        text.clone(),
        Precision::Double53,
        u64::MAX,
        RestoreMode::InspectPartial,
        bindings,
    ) {
        Ok(v) => v,
        Err(e) => {
            let inv = aa_mapworld::WorldInventory::decode(&map, u64::MAX).ok();
            return json!({"map":name,"load_error":e.to_string(),
                "header_error": inv.as_ref().and_then(|i| i.header_error.clone()),
                "teams_error": inv.as_ref().and_then(|i| i.teams_error.clone()),
                "kingdoms_error": inv.as_ref().and_then(|i| i.kingdoms_error.clone()),
                "binding_context":context.config.metadata(),"managed_initialization":initialization,
                "lifecycle_coverage":lifecycle_coverage(false,false),
                "failures":[]});
        }
    };
    let inv = &attempt.inventory;
    let placements: BTreeMap<usize, &aa_mapworld::Placement> = inv
        .dynamic_things
        .placements
        .iter()
        .map(|p| (p.record_ordinal, p))
        .collect();
    let mut failures = Vec::new();
    for event in &attempt.report.events {
        let Some(error) = &event.error else { continue };
        let (definition, class) = match event.stage {
            RestoreStage::Teams => (
                inv.teams
                    .as_ref()
                    .and_then(|t| t.slots.get(event.source_ordinal))
                    .and_then(|s| s.record.as_ref())
                    .map(|r| utf16(&r.definition_id_utf16)),
                None,
            ),
            RestoreStage::Kingdoms => (
                inv.kingdoms
                    .as_ref()
                    .and_then(|t| t.slots.get(event.source_ordinal))
                    .and_then(|s| s.record.as_ref())
                    .map(|r| utf16(&r.definition_id_utf16)),
                None,
            ),
            RestoreStage::DynamicThings => placements
                .get(&event.source_ordinal)
                .map(|p| (Some(utf16(&p.thing_id_utf16)), Some(p.class_code)))
                .unwrap_or((None, None)),
            RestoreStage::Objectives | RestoreStage::ConfigureKingdoms | RestoreStage::RequiredObjects => (None, None),
        };
        failures.push(json!({
            "stage": event.stage,
            "source_ordinal": event.source_ordinal,
            "saved_id": event.saved_id,
            "registered": event.registered,
            "completion": event.completion,
            "definition": definition,
            "class_code": class,
            "error": error,
        }));
    }
    let records_without_failures = failures.is_empty();
    let mut result = json!({
        "map": name,
        "load_error": null,
        "post_load": null,
        "binding_context":context.config.metadata(),"managed_initialization":initialization,
        "lifecycle_coverage":lifecycle_coverage(records_without_failures,attempt.report.runtime_publication_ready),
        "dynamic_records": inv.dynamic_things.placements.len() + inv.dynamic_things.unsupported.len(),
        "decoded_placements": inv.dynamic_things.placements.len(),
        "events": attempt.report.events.len(),
        "registered": attempt.registry.len(),
        "stopped_before": attempt.report.stopped_before,
        "converted": attempt.report.converted.len(),
        "skipped": attempt.report.skipped.iter().map(|r| json!({
            "source_ordinal": r.source_ordinal, "saved_id": r.saved_id,
            "definition": utf16(&r.thing_id_utf16), "class_code": r.class_code,
            "cause": r.cause, "native_warning": r.cause.native_warning_key()})).collect::<Vec<_>>(),
        "failures": failures,
    });
    // Dispose the diagnostic attempt before the independent fail-fast reload.
    // This is not native previous-world teardown or task completion.
    drop(attempt);
    if records_without_failures {
        result["post_load"] = post_load(&map, actors, settings, text, context, sources);
    }
    result
}

/// StopOnError reload of a map whose census found no failure, staged, then
/// the UPDATABLES queue Load and the PostLoad pass.
fn post_load(
    map: &aa_mapworld::MapArchive,
    actors: &ActorCatalog,
    settings: &Settings,
    text: &Localization,
    context: &Context,
    sources: &mut Option<V14Sources>,
) -> Value {
    let (bindings, initialization) = match context.bindings(sources) {
        Ok(value) => value,
        Err(error) => {
            return json!({"error":format!("reload bindings: {error}"),
            "binding_context":context.config.metadata(),"lifecycle_coverage":lifecycle_coverage(false,false)});
        }
    };
    let attempt = match Attempt::load_with_bindings(
        map,
        actors,
        settings.clone(),
        text.clone(),
        Precision::Double53,
        u64::MAX,
        RestoreMode::StopOnError,
        bindings,
    ) {
        Ok(v) => v,
        Err(e) => {
            return json!({"error": format!("reload: {e}"),
            "binding_context":context.config.metadata(),"managed_initialization":initialization,
            "lifecycle_coverage":lifecycle_coverage(false,false)});
        }
    };
    let mut staged = match attempt.into_staged() {
        Ok(v) => v,
        Err(e) => {
            return json!({"error": format!("staging: {e}"),
            "binding_context":context.config.metadata(),"managed_initialization":initialization,
            "lifecycle_coverage":lifecycle_coverage(true,false)});
        }
    };
    let rng_before = staged.rng.state;
    match staged.post_load(actors, Precision::Double53) {
        Ok(report) => json!({
            "error": null,
            "objects": report.objects,
            "discarded_updates": report.discarded_updates,
            "messages": report.messages,
            "rng_before": rng_before,
            "rng_after": staged.rng.state,
            "binding_context":context.config.metadata(),"managed_initialization":initialization,
            "lifecycle_coverage":lifecycle_coverage(true,staged.report.runtime_publication_ready),
            "next": staged.init_for_loaded_map().err().map(|e| e.to_string()),
        }),
        Err(e) => json!({"error": e.to_string(),
            "binding_context":context.config.metadata(),"managed_initialization":initialization,
            "lifecycle_coverage":lifecycle_coverage(true,staged.report.runtime_publication_ready)}),
    }
}

#[test]
fn inventory_context_defaults_and_explicit_bindings_remain_distinct() {
    let default = Config::parse(None, None, None).unwrap();
    assert_eq!(default.context, BindingContext::Standalone);
    assert_eq!(default.mode(), None);
    let context = Context {
        config: default,
        resources: None,
    };
    let (bindings, initialization) = context.bindings(&mut None).unwrap();
    assert!(bindings.resources.is_none() && bindings.inactive_load.is_none());
    assert!(initialization.is_null());
    assert_eq!(
        Config::parse(Some("resources"), Some("1"), None)
            .unwrap()
            .mode(),
        Some(1)
    );
    let managed = Config::parse(Some("managed"), None, Some("0")).unwrap();
    assert_eq!(managed.mode(), Some(1));
    assert_eq!(managed.metadata()["elapsed_clock_fixture_millis"], 0);
    assert!(
        managed.metadata()["managed_scope"]
            .as_str()
            .unwrap()
            .contains("not an old-world")
    );
    assert_eq!(
        Config::parse(Some("managed"), Some("1"), Some("4294967295"))
            .unwrap()
            .elapsed_fixture,
        Some(u32::MAX)
    );
}

#[test]
fn inventory_context_rejects_invalid_environment_values_and_conflicts() {
    for (context, mode, elapsed) in [
        (Some(""), None, None),
        (Some("Managed"), None, None),
        (Some("unknown"), None, None),
        (None, Some(""), None),
        (None, Some("-1"), None),
        (None, Some("4294967296"), None),
        (Some("managed"), Some("2"), Some("0")),
        (Some("managed"), Some("4"), Some("0")),
        (Some("managed"), None, None),
        (Some("managed"), None, Some("")),
        (Some("managed"), None, Some("-1")),
        (Some("managed"), None, Some("clock")),
        (Some("resources"), Some("1"), Some("0")),
        (None, None, Some("0")),
    ] {
        assert!(
            Config::parse(context, mode, elapsed).is_err(),
            "{context:?}/{mode:?}/{elapsed:?}"
        );
    }
}

#[test]
fn inventory_context_record_clean_is_not_immediate_init_or_publication() {
    let coverage = lifecycle_coverage(true, false);
    assert_eq!(coverage["record_sections_without_failures"], true);
    assert_eq!(coverage["immediate_init_complete"], false);
    assert_eq!(coverage["runtime_publication_ready"], false);
    assert_eq!(coverage["battle_ready"], false);
    assert_eq!(
        coverage["immediate_init"],
        "not_executed_by_this_diagnostic"
    );
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 sources; focused context construction only"]
fn inventory_context_each_managed_attempt_owns_a_fresh_source_bound_scope() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = Some(open_sources(&game));
    let resources = Arc::new(ResourceCatalog::load_v14(sources.as_mut().unwrap()).unwrap());
    let context = Context {
        config: Config::parse(Some("managed"), Some("1"), Some("0")).unwrap(),
        resources: Some(Arc::clone(&resources)),
    };
    let (first, first_info) = context.bindings(&mut sources).unwrap();
    let (second, second_info) = context.bindings(&mut sources).unwrap();
    let first_scope = first.inactive_load.as_ref().unwrap();
    let second_scope = second.inactive_load.as_ref().unwrap();
    assert_ne!(
        first_scope.synchronization_owner_address(),
        second_scope.synchronization_owner_address()
    );
    assert!(Arc::ptr_eq(first.resources.as_ref().unwrap(), &resources));
    assert!(Arc::ptr_eq(second.resources.as_ref().unwrap(), &resources));
    assert_eq!(first_info, second_info);
    assert_eq!(first_info["elapsed_clock_fixture_millis"], 0);
    assert_eq!(first_info["active"], false);
    assert_eq!(first_info["uvars_path"], "UVars.tgi");
    assert!(first_scope.observer_present().unwrap() && second_scope.observer_present().unwrap());
    assert_eq!(first_scope.native_load_mode(), 1);
}
