//! User-facing, stopped native restoration diagnostic, shared by CLI and viewer.
//! Calls the existing original/v14 ports; this adds no simulation or fake world.
//! Each run owns fresh source readers, catalogs and a fresh managed-load token.
use aa_assets::MapInfo;
use aa_mapworld::{
    MapArchive,
    restore::{RestoreMode, RestoreReport},
};
use aa_numeric::Precision;
use aa_resources::{MemoryEstimate, MemoryPolicy, count_bytes, preflight};
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use aa_runtime::{
    combat_source_completion,
    combat_source_receipt::VerifiedStartupReceipt,
    localization::Localization,
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
};
use anyhow::{Context, Result, bail};
use eframe::egui;
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    io::Write,
    path::{Component, Path, PathBuf},
    sync::{Arc, mpsc},
    time::Instant,
};

#[derive(Debug, Serialize)]
pub struct MapSource {
    pub requested: String,
    pub id: Option<String>,
    pub name: Option<String>,
    pub path: PathBuf,
    pub sha256: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Counts {
    /// Successful registered record events, not every object left in the arena.
    pub loaded_records: usize,
    pub failed_records: usize,
    pub skipped_records: usize,
    pub pending_conversions: usize,
    /// Includes shells from partial failures and embedded records.
    pub registered_objects: usize,
    pub record_events: usize,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub diagnostic_completed: bool,
    pub battle_ready: bool,
    pub source: MapSource,
    pub context: Value,
    pub counts: Counts,
    pub record_sections_without_failures: bool,
    pub first_unsupported_step: String,
    pub seconds: f64,
    pub native_details: Value,
}

fn resolve(root: &Path, requested: &str) -> Result<MapSource> {
    let catalog: Vec<MapInfo> = serde_json::from_slice(
        &std::fs::read(root.join("game/data/maps/index.json"))
            .context("Read the recovered original map index")?,
    )?;
    let by_id = catalog.iter().find(|map| map.id == requested);
    let path = if let Some(map) = by_id {
        let normalized = map.source.replace('\\', "/");
        let source = Path::new(&normalized);
        if normalized.contains(':')
            || source
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            bail!(
                "Map index contains an invalid relative source path: {}",
                map.source
            );
        }
        let data = root
            .join("assets/original/Data")
            .canonicalize()
            .context("Original map sources are missing from assets/original/Data")?;
        let path = data
            .join(source)
            .canonicalize()
            .with_context(|| format!("Original map source is unavailable: {}", map.source))?;
        if !path.starts_with(&data) {
            bail!("Map index source resolves outside the original Data directory");
        }
        path
    } else {
        let path = Path::new(requested);
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("rmp"))
        {
            bail!(
                "Unknown map ID {requested:?}; select an original map ID or an explicit .RMP path"
            );
        }
        path.canonicalize()
            .with_context(|| format!("Original map path is unavailable: {requested}"))?
    };
    let relative_id = root
        .join("assets/original/Data/Maps")
        .canonicalize()
        .ok()
        .and_then(|maps| path.strip_prefix(maps).ok().map(Path::to_owned))
        .map(|mut relative| {
            relative.set_extension("");
            relative.to_string_lossy().replace('\\', "/")
        });
    let info = by_id.or_else(|| {
        catalog
            .iter()
            .find(|map| Some(&map.id) == relative_id.as_ref())
    });
    Ok(MapSource {
        requested: requested.into(),
        id: info.map(|map| map.id.clone()).or(relative_id),
        name: info.map(|map| map.name.clone()),
        path,
        sha256: String::new(),
    })
}

fn counts(report: &RestoreReport, registered_objects: usize) -> Counts {
    Counts {
        loaded_records: report
            .events
            .iter()
            .filter(|event| event.registered && event.error.is_none())
            .count(),
        failed_records: report
            .events
            .iter()
            .filter(|event| event.error.is_some())
            .count(),
        skipped_records: report.skipped.len(),
        pending_conversions: report.converted.len(),
        registered_objects,
        record_events: report.events.len(),
    }
}

fn first_step(report: &RestoreReport) -> String {
    match report.events.iter().find(|event| event.error.is_some()) {
        Some(event) if event.error.as_deref().is_some_and(|error|
            error.contains("5788B0 combat value over the restore shells is unported")) =>
            "First unsupported step: restoring a unit's combat values. That native service is not connected to map loading yet.".into(),
        Some(event) => format!("Record loading stopped on an unsupported or invalid {:?} record ({}). See native details.",
            event.stage, event.source_ordinal),
        None if report.stopped_before.is_some() => format!(
            "Record loading stopped before {:?}; that section is unavailable. See native details.",
            report.stopped_before.expect("checked above")),
        None => "Record loading finished. Immediate object initialization and battle startup are still unfinished.".into(),
    }
}

pub fn run(root: &Path, requested: &str, policy: MemoryPolicy) -> Result<Report> {
    run_with_progress(root, requested, policy, |_| {})
}

fn run_with_progress(
    root: &Path,
    requested: &str,
    policy: MemoryPolicy,
    progress: impl Fn(&'static str),
) -> Result<Report> {
    let start = Instant::now();
    progress("Reading original map…");
    let root = root
        .canonicalize()
        .context("Recovered asset root is unavailable")?;
    let mut source = resolve(&root, requested)?;
    let file_bytes = std::fs::metadata(&source.path)?.len();
    preflight(
        MemoryEstimate {
            resident_cpu_bytes: file_bytes,
            ..Default::default()
        },
        policy,
    )?;
    let bytes = std::fs::read(&source.path)?;
    source.sha256 = aa_rules::sources::sha256(&bytes);
    let map = MapArchive::from_bytes(bytes).context("Read original map archive")?;
    let decoded_bytes = map.directory().iter().try_fold(0u64, |sum, chunk| {
        sum.checked_add(chunk.decoded_bytes)
            .context("Map decoded-byte estimate overflow")
    })?;
    let initial_budget = preflight(
        MemoryEstimate {
            resident_cpu_bytes: count_bytes(decoded_bytes, 8)?,
            ..Default::default()
        },
        policy,
    )?;
    let install = root
        .parent()
        .context("Original installation parent is unavailable")?;
    let archive_path = install.join("Data.rwd");
    let overrides_path = root.join("native-game/runtime-expanded-v14");
    let mut sources = V14Sources::open(
        &archive_path,
        &overrides_path,
        SourceLimits {
            max_member_bytes: initial_budget.effective_cpu_budget_bytes / 48,
        },
    )
    .context("Open original Data.rwd and v14 rule overrides")?;
    let rule_bytes = sources
        .catalog()
        .iter()
        .filter(|row| row.logical_path.to_ascii_lowercase().ends_with(".tgi"))
        .try_fold(0u64, |sum, row| {
            sum.checked_add(row.size)
                .context("Rule source size overflow")
        })?;
    let estimate = count_bytes(decoded_bytes, 8)?
        .checked_add(count_bytes(rule_bytes, 48)?)
        .context("Map and rule memory estimate overflow")?;
    let budget = preflight(
        MemoryEstimate {
            resident_cpu_bytes: estimate,
            ..Default::default()
        },
        policy,
    )?;
    progress("Reading original rules and resources…");
    let settings = Settings::load_v14(&mut sources, Limits::default())?;
    let text = Localization::load_v14(&mut sources, Limits::default())?;
    let actors = ActorCatalog::load_v14(&mut sources)?;
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources)?);
    progress("Preparing a fresh stopped load scope…");
    // An explicit reproducible diagnostic clock input, not the original app's
    // observed clock. No active game or previous-world lifecycle is adopted.
    let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
        .map_err(anyhow::Error::msg)?
        .begin(TaskKind::Normal)
        .map_err(anyhow::Error::msg)?;
    let initialization = inactive.initialization();
    let mut context = json!({
        "binding":"managed", "native_load_mode":1, "precision":"Double53",
        "restore_mode":"InspectPartial", "elapsed_clock_fixture_millis":0,
        "elapsed_clock_source":"explicit diagnostic fixture; not observed original application time",
        "fresh_application_scope":true, "original_old_world_lifecycle":false, "active":false,
        "uvars_path":initialization.uvars_path, "uvars_sha256":initialization.uvars_sha256,
        "private_constructor_seed":initialization.private_seed,
        "rules_archive":archive_path, "v14_overrides":overrides_path,
        "actor_definitions":actors.actors.len(), "actor_source_files":actors.sources.len(),
        "resource_source":resources.sources, "resource_count":resources.resources.len()
    });
    progress("Computing original shared combat values, then restoring saved records…");
    let attempt = Attempt::load_with_source_setup(
        &map,
        &actors,
        settings,
        text,
        Precision::Double53,
        budget.effective_cpu_budget_bytes / 8,
        RestoreMode::InspectPartial,
        LoadBindings {
            native_load_mode: Some(1),
            inactive_load: Some(inactive),
            resources: Some(resources),
        },
        |catalog| {
            let proof = VerifiedStartupReceipt::verify_embedded_reference(catalog, &mut sources)?;
            combat_source_completion::install(catalog, &mut sources,
                proof.context(), proof.csv(), proof.globals())?;
            combat_source_completion::compute(catalog)
        },
    )
    .context("Run native map record restoration")?;
    let combat_source = attempt.definitions.combat_source().map_err(anyhow::Error::msg)?;
    context["combat_source"] = json!({
        "phase":format!("{:?}",combat_source.phase()),
        "definition_count":combat_source.manager_order().len(),
        "result_sha256":combat_source.result_fingerprint().map_err(anyhow::Error::msg)?,
        "reference":combat_source.context().reference_startup(),
        "scope":"Computed against the sealed Windows/v14 reference; not an observation of this Rust thread or a live battle"
    });
    let counts = counts(&attempt.report, attempt.registry.len());
    let record_sections_without_failures =
        counts.failed_records == 0 && attempt.report.stopped_before.is_none();
    let first_unsupported_step = first_step(&attempt.report);
    let native_details = json!({
        "warning":"InspectPartial continues after record errors; later errors can be cascades. This stopped attempt is discarded after reporting.",
        "counts_definition":"Loaded records are successful registered record events. Registered objects also include embedded records and failed partial shells. Conversions are queued, not executed.",
        "immediate_init":"not executed by this diagnostic", "immediate_init_complete":false,
        "post_load":"not executed by this diagnostic", "conversion_execution":"not executed",
        "runtime_publication_ready":attempt.report.runtime_publication_ready,
        "battle_ready":false, "world_tick":attempt.clock.tick, "world_rng_state":attempt.rng.state,
        "first_record_error":attempt.report.events.iter().find(|event| event.error.is_some()),
        "restore_report":attempt.report
    });
    Ok(Report {
        schema: "axis-allies-native-map-check-v1",
        diagnostic_completed: true,
        battle_ready: false,
        source,
        context,
        counts,
        record_sections_without_failures,
        first_unsupported_step,
        seconds: start.elapsed().as_secs_f64(),
        native_details,
    })
}

/// A new report only: neither source maps nor previous reports are overwritten.
pub fn write_new(path: &Path, report: &Report) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(report)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| {
            format!(
                "Create a new report (existing files are preserved): {}",
                path.display()
            )
        })?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    Ok(())
}

enum Message {
    Progress(&'static str),
    Complete(Box<Result<Report>>),
}

#[derive(Default)]
pub struct Panel {
    pending: Option<mpsc::Receiver<Message>>,
    requested: String,
    progress: &'static str,
    report: Option<Report>,
    error: Option<String>,
}
impl Panel {
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }

    pub fn poll(&mut self) {
        loop {
            let Some(receive) = &self.pending else { break };
            match receive.try_recv() {
                Ok(Message::Progress(message)) => self.progress = message,
                Ok(Message::Complete(result)) => {
                    self.pending = None;
                    match *result {
                        Ok(report) => self.report = Some(report),
                        Err(error) => self.error = Some(format!("{error:#}")),
                    }
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.error = Some("Map loading check stopped before returning a report".into());
                    break;
                }
            }
        }
    }

    fn start(
        &mut self,
        ctx: egui::Context,
        root: PathBuf,
        requested: String,
        policy: MemoryPolicy,
    ) {
        if self.busy() {
            return;
        }
        let (send, receive) = mpsc::channel();
        self.pending = Some(receive);
        self.requested = requested.clone();
        self.report = None;
        self.error = None;
        self.progress = "Starting map loading check…";
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_with_progress(&root, &requested, policy, |message| {
                    let _ = send.send(Message::Progress(message));
                    ctx.request_repaint();
                })
            }))
            .unwrap_or_else(|_| {
                Err(anyhow::anyhow!(
                    "Map loading check failed unexpectedly; no world was published"
                ))
            });
            let _ = send.send(Message::Complete(Box::new(result)));
            ctx.request_repaint();
        });
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        root: &Path,
        selected: &str,
        policy: MemoryPolicy,
        asset_loading: bool,
    ) {
        ui.separator();
        if ui
            .add_enabled(
                !self.busy() && !asset_loading,
                egui::Button::new("Check map loading"),
            )
            .clicked()
        {
            self.start(ui.ctx().clone(), root.to_owned(), selected.into(), policy);
        }
        ui.colored_label(
            egui::Color32::from_rgb(236, 193, 109),
            "Battle startup is unfinished",
        );
        ui.small("Runs the native record loader. This does not start a playable match.");
        if self.busy() {
            ui.label(format!("Checking {}", self.requested));
            ui.horizontal_wrapped(|ui| {
                ui.spinner();
                ui.label(self.progress);
            });
        }
        if let Some(error) = &self.error {
            ui.label(format!("Check failed: {}", self.requested));
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        if let Some(report) = &self.report {
            ui.strong(format!(
                "Report: {}",
                report
                    .source
                    .id
                    .as_deref()
                    .unwrap_or(&report.source.requested)
            ));
            if let Some(name) = &report.source.name {
                ui.small(name);
            }
            if self.requested != selected {
                ui.small("This report belongs to the map named above, not the current selection.");
            }
            egui::Grid::new("native-map-check-counts")
                .num_columns(2)
                .show(ui, |ui| {
                    for (label, count) in [
                        ("Loaded records", report.counts.loaded_records),
                        ("Failed records", report.counts.failed_records),
                        ("Skipped records", report.counts.skipped_records),
                        ("Pending conversions", report.counts.pending_conversions),
                    ] {
                        ui.label(label);
                        ui.label(count.to_string());
                        ui.end_row();
                    }
                });
            ui.label(&report.first_unsupported_step);
            ui.small(format!(
                "Check completed in {:.2} seconds. Partial record diagnostic only.",
                report.seconds
            ));
            ui.collapsing("Native details", |ui| {
                ui.label(format!("Source: {}", report.source.path.display()));
                ui.label(format!("SHA-256: {}", report.source.sha256));
                ui.small("Managed fresh scope · mode 1 · PC53 · elapsed 0 ms fixture");
                ui.small("Immediate Init, PostLoad, conversions and world publication are not executed here. Later errors may follow an earlier partial failure.");
                if let Some(events) = report.native_details["restore_report"]["events"].as_array() {
                    egui::ScrollArea::vertical().id_salt("native-map-errors").max_height(200.0).show(ui, |ui| {
                        for event in events.iter().filter(|e| e["error"].is_string()) {
                            ui.monospace(format!("{} record {}, saved ID {}: {}", event["stage"], event["source_ordinal"], event["saved_id"], event["error"].as_str().unwrap_or_default()));
                        }
                    });
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_mapworld::restore::{ConvertedRecord, RestoreEvent, RestoreStage};

    #[test]
    fn counts_do_not_call_failed_shells_or_conversions_loaded() {
        let mut report = RestoreReport::default();
        for (registered, error) in [
            (true, None),
            (true, Some("unsupported".into())),
            (false, None),
        ] {
            report.events.push(RestoreEvent {
                stage: RestoreStage::DynamicThings,
                source_ordinal: report.events.len(),
                saved_id: None,
                registered,
                completion: None,
                error,
            });
        }
        report.converted.push(ConvertedRecord {
            source_ordinal: 2,
            saved_id: 7,
            queue_index: 0,
        });
        assert_eq!(
            counts(&report, 8),
            Counts {
                loaded_records: 1,
                failed_records: 1,
                skipped_records: 0,
                pending_conversions: 1,
                registered_objects: 8,
                record_events: 3
            }
        );
        assert!(first_step(&report).contains("(1)"));
        assert!(first_step(&RestoreReport::default()).contains("still unfinished"));
    }

    #[test]
    fn disconnected_background_check_remains_failure() {
        let (send, receive) = mpsc::channel();
        let mut panel = Panel {
            pending: Some(receive),
            requested: "captured-map".into(),
            ..Default::default()
        };
        drop(send);
        panel.poll();
        assert!(!panel.busy());
        assert!(panel.report.is_none());
        assert!(panel.error.is_some());
        assert_eq!(panel.requested, "captured-map");
    }

    #[test]
    fn unavailable_section_never_claims_record_loading_finished() {
        for stage in [
            RestoreStage::Teams,
            RestoreStage::Kingdoms,
            RestoreStage::RequiredObjects,
            RestoreStage::DynamicThings,
        ] {
            let report = RestoreReport {
                stopped_before: Some(stage),
                ..Default::default()
            };
            let message = first_step(&report);
            assert!(message.contains("stopped before"));
            assert!(!message.contains("loading finished"));
        }
    }

    fn sample_report() -> Report {
        Report {
            schema: "axis-allies-native-map-check-v1",
            diagnostic_completed: true,
            battle_ready: false,
            source: MapSource {
                requested: "captured-map".into(),
                id: Some("captured-map".into()),
                name: None,
                path: PathBuf::from("sample.RMP"),
                sha256: String::new(),
            },
            context: json!({}),
            counts: counts(&RestoreReport::default(), 0),
            record_sections_without_failures: true,
            first_unsupported_step: first_step(&RestoreReport::default()),
            seconds: 0.0,
            native_details: json!({}),
        }
    }

    #[test]
    fn pending_check_refuses_duplicate_and_retains_captured_report_identity() {
        let (send, receive) = mpsc::channel();
        let mut panel = Panel {
            pending: Some(receive),
            requested: "captured-map".into(),
            ..Default::default()
        };
        panel.start(
            egui::Context::default(),
            PathBuf::from("missing"),
            "different-map".into(),
            MemoryPolicy::default(),
        );
        assert_eq!(panel.requested, "captured-map");
        send.send(Message::Progress("Restoring…")).unwrap();
        send.send(Message::Complete(Box::new(Ok(sample_report()))))
            .unwrap();
        panel.poll();
        assert!(!panel.busy());
        assert!(panel.error.is_none());
        assert_eq!(
            panel.report.unwrap().source.id.as_deref(),
            Some("captured-map")
        );
    }

    #[test]
    fn report_output_never_overwrites_an_existing_file() {
        let path = std::env::temp_dir().join(format!(
            "aa-dev09-report-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        write_new(&path, &sample_report()).unwrap();
        let original = std::fs::read(&path).unwrap();
        let mut different = sample_report();
        different.source.requested = "different-map".into();
        assert!(write_new(&path, &different).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    #[ignore = "requires AA_GAME_ROOT original/v14 sources; real runtime diagnostic"]
    fn actual_sources_report_clean_and_partial_maps_without_claiming_battle_success() {
        let game = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
        let root = game.join("reconstruction");
        for (id, clean) in [
            ("DeveloperMaps/(3v3)Firefight", true),
            ("(2v2)Kursk", false),
        ] {
            let report = run(&root, id, MemoryPolicy::default()).unwrap();
            assert!(report.diagnostic_completed);
            assert!(!report.battle_ready);
            assert_eq!(report.source.id.as_deref(), Some(id));
            assert_eq!(report.record_sections_without_failures, clean);
            assert_eq!(report.context["native_load_mode"], 1);
            assert_eq!(report.context["elapsed_clock_fixture_millis"], 0);
            assert_eq!(report.native_details["immediate_init_complete"], false);
            assert_eq!(report.source.sha256.len(), 64);
            assert!(report.counts.loaded_records > 0);
            if !clean {
                assert!(report.counts.failed_records > 0);
            }
            let explicit = resolve(&root, report.source.path.to_str().unwrap()).unwrap();
            assert_eq!(explicit.id, report.source.id);
            assert_eq!(explicit.path, report.source.path);
        }
        assert!(run(&root, "NoSuchOriginalMap", MemoryPolicy::default()).is_err());
    }
}
